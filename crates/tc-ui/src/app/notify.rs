//! Notificaciones de escritorio en la UI (fase C2, ADR-0013): traduce el modelo de `tc-core` a las
//! reglas de `tc_notify::rules`, avisa con el `Notifier` que aporta `tc-app`, retira el aviso al leer
//! el chat y atiende los clics que vuelven del SO. Los ajustes (S28) se guardan con [`Prefs`].
//!
//! Cuándo **no** se avisa, además de las reglas del `Engine`:
//! * antes de terminar la carga inicial (lista de chats) y mientras la conexión no está `Online`;
//! * mensajes con fecha anterior al arranque de los avisos (la puesta al día del primer arranque).
//!
//! Las ráfagas al reconectar las limita el `Engine` (`BurstLimit`). Claves de chat: `u:<id>`,
//! `g:<id>`, `s:<id>` y `g:<id>/t:<tema>` para un tema de foro.

use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex};

use futures::stream::BoxStream;
use futures::StreamExt;
use iced::{Subscription, Task, window};
use tc_core::*;
use tc_notify::rules::{
    self, ACTION_MARK_READ, ChatInfo, ChatKind, DoNotDisturb, Engine, IncomingMessage, NotifySettings, Outcome, Preview, ServiceKind,
    WindowState,
};
use tc_notify::{ChatKey, Icon, Notifier, NotifyEvent, NotifyEvents};

use super::chat_actions::ChatMsg;
use super::downloads::Prefs;
use super::forum::ForumMsg;
use super::{App, Msg, Overlay, Phase};

/// Texto con el que `tc-telegram` resume el servicio «X se unió a Telegram» (`ContactSignUp`). El
/// modelo de `tc-core` no distingue los mensajes de servicio: se reconoce por el texto exacto.
pub const CONTACT_JOINED_TEXT: &str = "Se ha unido a la red";
/// Tolerancia (s) entre el instante en que se arman los avisos y la fecha de un mensaje antiguo.
pub const STARTUP_GRACE_SECS: i64 = 120;

/// Notificador y canal de eventos que aporta `tc-app`. Clonable (como `Flags`); el canal se entrega
/// una sola vez a la suscripción.
#[derive(Clone)]
pub struct NotifyHandle {
    pub notifier: Arc<dyn Notifier>,
    events: Arc<Mutex<Option<NotifyEvents>>>,
}

impl NotifyHandle {
    pub fn new(notifier: Arc<dyn Notifier>, events: NotifyEvents) -> Self {
        Self { notifier, events: Arc::new(Mutex::new(Some(events))) }
    }

    /// Sin avisos (pruebas, entornos sin escritorio).
    pub fn null() -> Self {
        let (_tx, rx) = tc_notify::event_channel();
        Self::new(Arc::new(tc_notify::NullNotifier), rx)
    }
}

impl std::fmt::Debug for NotifyHandle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("NotifyHandle").field("capabilities", &self.notifier.capabilities()).finish_non_exhaustive()
    }
}

/// Identidad estable de la suscripción a los eventos del notificador.
#[derive(Clone)]
struct EventsHandle(Arc<Mutex<Option<NotifyEvents>>>);

impl std::hash::Hash for EventsHandle {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        "tc-notify-events".hash(state);
    }
}

fn notify_events(h: &EventsHandle) -> BoxStream<'static, NotifyEvent> {
    match h.0.lock().ok().and_then(|mut g| g.take()) {
        Some(rx) => rx.boxed(),
        None => futures::stream::pending().boxed(),
    }
}

/// Dónde apunta una clave de aviso.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Target {
    Chat(PeerRef),
    Topic(ThreadRef),
}

pub fn chat_key(peer: PeerRef) -> ChatKey {
    ChatKey(match peer {
        PeerRef::User(u) => format!("u:{}", u.0),
        PeerRef::Group(g) => format!("g:{}", g.0),
        PeerRef::Secret(s) => format!("s:{}", s.0),
    })
}

pub fn topic_key(th: ThreadRef) -> ChatKey {
    ChatKey(format!("g:{}/t:{}", th.group.0, th.topic.0))
}

/// Lee una clave formada por [`chat_key`] o [`topic_key`] (`None`: no es nuestra).
pub fn parse_key(key: &ChatKey) -> Option<Target> {
    let s = key.as_str();
    if let Some((g, t)) = s.split_once("/t:") {
        let g = g.strip_prefix("g:")?.parse().ok()?;
        return Some(Target::Topic(ThreadRef { group: GroupId(g), topic: MsgId(t.parse().ok()?) }));
    }
    let (kind, id) = s.split_once(':')?;
    Some(Target::Chat(match kind {
        "u" => PeerRef::User(UserId(id.parse().ok()?)),
        "g" => PeerRef::Group(GroupId(id.parse().ok()?)),
        "s" => PeerRef::Secret(SecretChatId(id.parse().ok()?)),
        _ => return None,
    }))
}

/// Interruptores del diálogo de ajustes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Toggle {
    Enabled,
    Private,
    Groups,
    Channels,
    /// Texto también en chats secretos (por defecto solo el nombre).
    SecretText,
    Sound,
    Archived,
    IgnoreJoined,
}

/// Opciones de «No molestar».
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DndChoice {
    Off,
    Hour,
    EightHours,
    /// Hasta que se quite.
    On,
}

impl DndChoice {
    pub const ALL: [DndChoice; 4] = [DndChoice::Off, DndChoice::Hour, DndChoice::EightHours, DndChoice::On];

    pub fn label(self) -> &'static str {
        match self {
            DndChoice::Off => "No",
            DndChoice::Hour => "1 hora",
            DndChoice::EightHours => "8 horas",
            DndChoice::On => "Hasta que lo quite",
        }
    }

    pub fn apply(self, now: i64) -> DoNotDisturb {
        match self {
            DndChoice::Off => DoNotDisturb::Off,
            DndChoice::Hour => DoNotDisturb::Until(now + 3600),
            DndChoice::EightHours => DoNotDisturb::Until(now + 8 * 3600),
            DndChoice::On => DoNotDisturb::On,
        }
    }
}

#[derive(Debug, Clone)]
pub enum NotifyMsg {
    /// Clic en un aviso o en uno de sus botones.
    Event(NotifyEvent),
    /// Abrir el diálogo de ajustes (raíl).
    Settings,
    Set(Toggle, bool),
    Preview(Preview),
    Dnd(DndChoice),
}

// ---------------------------------------------------------------- persistencia

/// Prefijo de las claves de ajustes de avisos en [`Prefs`].
pub const PREFS_PREFIX: &str = "notify.";

fn preview_str(p: Preview) -> &'static str {
    match p {
        Preview::Full => "full",
        Preview::NameOnly => "name",
        Preview::Hidden => "hidden",
    }
}

fn parse_preview(s: &str) -> Option<Preview> {
    match s {
        "full" => Some(Preview::Full),
        "name" => Some(Preview::NameOnly),
        "hidden" => Some(Preview::Hidden),
        _ => None,
    }
}

fn dnd_str(d: DoNotDisturb) -> String {
    match d {
        DoNotDisturb::Off => "off".into(),
        DoNotDisturb::On => "on".into(),
        DoNotDisturb::Until(t) => format!("until:{t}"),
    }
}

fn parse_dnd(s: &str) -> Option<DoNotDisturb> {
    match s {
        "off" => Some(DoNotDisturb::Off),
        "on" => Some(DoNotDisturb::On),
        _ => s.strip_prefix("until:")?.parse().ok().map(DoNotDisturb::Until),
    }
}

fn b(v: bool) -> String {
    if v { "1" } else { "0" }.into()
}

/// Ajustes como pares (clave, valor) para [`Prefs`]. El límite de ráfagas no se guarda.
pub fn encode(s: &NotifySettings) -> Vec<(&'static str, String)> {
    vec![
        ("notify.enabled", b(s.enabled)),
        ("notify.private", b(s.private)),
        ("notify.groups", b(s.groups)),
        ("notify.channels", b(s.channels)),
        ("notify.archived", b(s.archived)),
        ("notify.ignore_joined", b(s.ignore_contact_joined)),
        ("notify.preview", preview_str(s.preview).into()),
        ("notify.secret_preview", preview_str(s.secret_preview).into()),
        ("notify.sound", b(s.sound)),
        ("notify.dnd", dnd_str(s.dnd)),
    ]
}

/// Lee los ajustes guardados; lo que falte o no se entienda queda con su valor por defecto.
pub fn decode(get: impl Fn(&str) -> Option<String>) -> NotifySettings {
    let mut s = NotifySettings::default();
    let flag = |k: &str, d: &mut bool| {
        if let Some(v) = get(k) {
            match v.as_str() {
                "1" => *d = true,
                "0" => *d = false,
                _ => {}
            }
        }
    };
    flag("notify.enabled", &mut s.enabled);
    flag("notify.private", &mut s.private);
    flag("notify.groups", &mut s.groups);
    flag("notify.channels", &mut s.channels);
    flag("notify.archived", &mut s.archived);
    flag("notify.ignore_joined", &mut s.ignore_contact_joined);
    flag("notify.sound", &mut s.sound);
    if let Some(p) = get("notify.preview").as_deref().and_then(parse_preview) {
        s.preview = p;
    }
    if let Some(p) = get("notify.secret_preview").as_deref().and_then(parse_preview) {
        s.secret_preview = p;
    }
    if let Some(d) = get("notify.dnd").as_deref().and_then(parse_dnd) {
        s.dnd = d;
    }
    s
}

// ---------------------------------------------------------------- estado

/// Estado de los avisos de la sesión.
pub struct Notifs {
    pub engine: Engine,
    notifier: Arc<dyn Notifier>,
    events: Option<EventsHandle>,
    prefs: Option<Arc<dyn Prefs>>,
    /// Instante (s Unix) en que se armaron los avisos tras la carga inicial; `None`: aún no.
    pub armed_at: Option<i64>,
    /// Último mensaje avisable por clave (para no retirar con una lectura más antigua).
    last: HashMap<ChatKey, MsgId>,
    /// Claves con aviso a la vista.
    shown: HashSet<ChatKey>,
    /// Fin del silencio de los chats silenciados desde esta sesión con plazo.
    mute_until: HashMap<PeerRef, i64>,
    /// Pruebas: reloj fijo (s Unix) en lugar del del sistema.
    pub clock: Option<i64>,
}

impl Default for Notifs {
    fn default() -> Self {
        Self::new(None, None)
    }
}

impl Notifs {
    pub fn new(handle: Option<NotifyHandle>, prefs: Option<Arc<dyn Prefs>>) -> Self {
        let handle = handle.unwrap_or_else(NotifyHandle::null);
        let settings = prefs.as_ref().map_or_else(NotifySettings::default, |p| decode(|k| p.get(k)));
        Self {
            engine: Engine::new(settings),
            notifier: handle.notifier,
            events: Some(EventsHandle(handle.events)),
            prefs,
            armed_at: None,
            last: HashMap::new(),
            shown: HashSet::new(),
            mute_until: HashMap::new(),
            clock: None,
        }
    }

    pub fn settings(&self) -> &NotifySettings {
        self.engine.settings()
    }

    /// Cambia los ajustes y guarda en `Prefs` solo lo que cambió.
    pub fn set_settings(&mut self, new: NotifySettings) {
        if let Some(p) = &self.prefs {
            let old = encode(self.engine.settings());
            for ((k, v), (_, was)) in encode(&new).into_iter().zip(old) {
                if v != was {
                    p.set(k, Some(&v));
                }
            }
        }
        self.engine.set_settings(new);
    }

    pub fn now(&self) -> i64 {
        self.clock.unwrap_or_else(|| super::now().0)
    }

    /// El chat se silenció con plazo (`Some`) o sin él / se reactivó (`None`).
    pub fn set_mute_until(&mut self, peer: PeerRef, until: Option<i64>) {
        match until {
            Some(t) => self.mute_until.insert(peer, t),
            None => self.mute_until.remove(&peer),
        };
    }

    /// Se leyó `key`: reinicia su grupo y retira el aviso si lo había.
    fn read(&mut self, key: &ChatKey) {
        let had = self.engine.on_chat_read(key);
        if (had || self.shown.remove(key))
            && let Err(e) = self.notifier.withdraw(key)
        {
            tracing::debug!("no se pudo retirar el aviso: {e}");
        }
        self.shown.remove(key);
        self.last.remove(key);
    }

    /// Claves con aviso que pertenecen a `group` (el chat y sus temas).
    fn keys_of_group(&self, group: GroupId) -> Vec<ChatKey> {
        let base = chat_key(PeerRef::Group(group));
        let topic_prefix = format!("{}/t:", base.as_str());
        self.shown.iter().chain(self.last.keys()).filter(|k| **k == base || k.as_str().starts_with(&topic_prefix)).cloned().collect::<HashSet<_>>().into_iter().collect()
    }
}

// ---------------------------------------------------------------- App

impl App {
    /// Suscripción a los clics que vuelven del SO.
    pub(crate) fn notify_subscription(&self) -> Option<Subscription<Msg>> {
        let h = self.notifs.events.clone()?;
        Some(Subscription::run_with(h, notify_events).map(|e| Msg::Notify(NotifyMsg::Event(e))))
    }

    /// ¿Se puede avisar ya? (carga inicial hecha y conexión al día).
    fn notify_ready(&self) -> bool {
        self.phase == Phase::Main && self.notifs.armed_at.is_some() && self.connection == ConnectionState::Online
    }

    /// Terminó la carga inicial de la lista: a partir de aquí se avisa.
    pub(crate) fn notify_arm(&mut self) {
        if self.notifs.armed_at.is_none() {
            self.notifs.armed_at = Some(self.notifs.now());
        }
    }

    /// Clave del chat (o tema) abierto en el panel de mensajes.
    pub fn open_key(&self) -> Option<ChatKey> {
        if let Some(th) = self.open_thread() {
            return Some(topic_key(th));
        }
        self.conv.as_ref().map(|c| chat_key(c.peer))
    }

    pub fn window_state(&self) -> WindowState {
        WindowState { focused: self.sticker_anims.focused, open_chat: self.open_key() }
    }

    fn chat_kind(&self, peer: PeerRef) -> ChatKind {
        match peer {
            PeerRef::User(_) => ChatKind::Private,
            PeerRef::Secret(_) => ChatKind::Secret,
            PeerRef::Group(g) if self.groups.get(&g).is_some_and(|g| g.kind == GroupKind::Channel) => ChatKind::Channel,
            PeerRef::Group(_) => ChatKind::Group,
        }
    }

    fn chat_title(&self, peer: PeerRef) -> String {
        if let Some(d) = self.dialog(peer) {
            return d.title.clone();
        }
        match peer {
            PeerRef::User(u) => self.users.get(&u).map(User::display_name),
            PeerRef::Group(g) => self.groups.get(&g).map(|g| g.title.clone()),
            PeerRef::Secret(s) => self.secrets.get(&s).and_then(|c| self.users.get(&c.user)).map(User::display_name),
        }
        .unwrap_or_else(|| "Chat".into())
    }

    /// Lo que las reglas necesitan del chat (o tema) de `m`.
    pub fn notify_chat_info(&self, peer: PeerRef, topic: Option<MsgId>) -> ChatInfo {
        let d = self.dialog(peer);
        let muted = d.is_some_and(|d| d.muted);
        let title = self.chat_title(peer);
        let (key, title) = match (peer, topic) {
            (PeerRef::Group(group), Some(topic)) => {
                let th = ThreadRef { group, topic };
                let t = self.topic(th).map_or_else(|| "Tema".to_string(), |t| t.title.clone());
                (topic_key(th), format!("{t} · {title}"))
            }
            _ => (chat_key(peer), title),
        };
        ChatInfo {
            key,
            title,
            kind: self.chat_kind(peer),
            muted,
            // El plazo solo cuenta si el chat sigue silenciado en la lista.
            mute_until: if muted { self.notifs.mute_until.get(&peer).copied() } else { None },
            archived: d.is_some_and(|d| d.archived),
        }
    }

    /// Lo que las reglas necesitan del mensaje.
    pub fn notify_message(&self, m: &Message, kind: ChatKind) -> IncomingMessage {
        let me = self.me.as_ref();
        let own = m.outgoing || (m.sender.is_some() && m.sender == me.map(|u| u.id));
        let text = if m.media.is_none() && m.sticker.is_none() { m.text.trim().to_string() } else { crate::format::preview(m) };
        let joined = kind != ChatKind::Group && kind != ChatKind::Channel && m.media.is_none() && m.text.trim() == CONTACT_JOINED_TEXT;
        let mention = me.and_then(|u| u.username.as_deref()).is_some_and(|un| {
            let needle = format!("@{}", un.to_lowercase());
            m.text.to_lowercase().split(|c: char| !(c.is_alphanumeric() || c == '_' || c == '@')).any(|w| w == needle)
        });
        let reply_to_me = m.reply_to.is_some_and(|r| {
            self.conv.as_ref().filter(|c| c.peer == m.peer).and_then(|c| c.find(r)).is_some_and(|x| x.outgoing)
        });
        let sender = m.sender.and_then(|u| self.users.get(&u)).map(User::display_name);
        IncomingMessage {
            sender,
            text,
            outgoing: own,
            service: joined.then_some(ServiceKind::ContactJoined),
            mentions_me: mention || reply_to_me,
            silent: false,
        }
    }

    /// Foto del chat ya en caché, como PNG para el aviso.
    fn notify_icon(&self, peer: PeerRef) -> Option<Icon> {
        let photo = self.dialog(peer).and_then(|d| d.photo).or_else(|| match peer {
            PeerRef::User(u) => self.users.get(&u).and_then(|u| u.photo),
            PeerRef::Group(g) => self.groups.get(&g).and_then(|g| g.photo),
            PeerRef::Secret(_) => None,
        });
        let handle = self.avatar_handle(peer, photo)?;
        png_of(&handle).map(Icon::Bytes)
    }

    /// Un mensaje nuevo (de un chat o de un tema): decide y avisa.
    pub(crate) fn notify_on_message(&mut self, m: &Message, topic: Option<MsgId>) {
        if !self.notify_ready() || crate::conv::is_local(m) {
            return;
        }
        // Puesta al día del arranque: lo anterior a armar los avisos no se anuncia.
        if self.notifs.armed_at.is_some_and(|t| m.date.0 < t - STARTUP_GRACE_SECS) {
            return;
        }
        // Un foro abierto como chat plano (backend sin temas): su clave es la del chat.
        let flat = self.open_thread().is_none() && self.conv.as_ref().is_some_and(|c| c.peer == m.peer);
        let topic = topic.filter(|_| !flat);
        let chat = self.notify_chat_info(m.peer, topic);
        let msg = self.notify_message(m, chat.kind);
        let window = self.window_state();
        let now = self.notifs.now();
        match self.notifs.engine.on_message(&msg, &chat, &window, now) {
            Outcome::Show(mut n) => {
                n.icon = self.notify_icon(m.peer);
                self.notifs.last.insert(chat.key.clone(), m.id);
                self.notifs.shown.insert(chat.key.clone());
                if let Err(e) = self.notifs.notifier.notify(n) {
                    tracing::debug!("no se pudo mostrar el aviso: {e}");
                }
            }
            Outcome::Suppressed(rules::Suppress::RateLimited) => {
                self.notifs.last.insert(chat.key, m.id);
            }
            Outcome::Suppressed(_) => {}
        }
    }

    /// Se leyó un chat o tema (abrirlo, «Marcar como leído», foco con el chat abierto).
    pub(crate) fn notify_read(&mut self, key: &ChatKey) {
        self.notifs.read(key);
    }

    /// «Marcar como leído» de un chat entero: también sus temas.
    pub(crate) fn notify_read_peer(&mut self, peer: PeerRef) {
        let keys = match peer {
            PeerRef::Group(g) => self.notifs.keys_of_group(g),
            _ => vec![],
        };
        self.notify_read(&chat_key(peer));
        for k in keys {
            self.notify_read(&k);
        }
    }

    /// `ReadInbox` de la red (leído en otro dispositivo o eco de nuestra lectura): retira el aviso
    /// solo si la lectura llega hasta el último mensaje avisado.
    pub(crate) fn notify_read_inbox(&mut self, peer: PeerRef, max_id: MsgId) {
        let key = chat_key(peer);
        if self.notifs.last.get(&key).is_some_and(|last| *last <= max_id) {
            self.notify_read(&key);
        }
    }

    /// Cambió el foco de la ventana: con el foco, el chat abierto cuenta como leído.
    pub(crate) fn notify_focus(&mut self, focused: bool) {
        if focused && let Some(k) = self.open_key() {
            self.notify_read(&k);
        }
    }

    pub(crate) fn on_notify(&mut self, m: NotifyMsg) -> Task<Msg> {
        match m {
            NotifyMsg::Event(ev) => self.on_notify_event(ev),
            NotifyMsg::Settings => {
                self.chat_actions.menu = None;
                self.overlay = Some(Overlay::Notifications);
                self.open_overlay_anim();
                Task::none()
            }
            NotifyMsg::Set(t, v) => {
                let mut s = self.notifs.settings().clone();
                match t {
                    Toggle::Enabled => s.enabled = v,
                    Toggle::Private => s.private = v,
                    Toggle::Groups => s.groups = v,
                    Toggle::Channels => s.channels = v,
                    Toggle::SecretText => s.secret_preview = if v { Preview::Full } else { Preview::NameOnly },
                    Toggle::Sound => s.sound = v,
                    Toggle::Archived => s.archived = v,
                    Toggle::IgnoreJoined => s.ignore_contact_joined = v,
                }
                self.notifs.set_settings(s);
                Task::none()
            }
            NotifyMsg::Preview(p) => {
                let mut s = self.notifs.settings().clone();
                s.preview = p;
                self.notifs.set_settings(s);
                Task::none()
            }
            NotifyMsg::Dnd(choice) => {
                let mut s = self.notifs.settings().clone();
                s.dnd = choice.apply(self.notifs.now());
                self.notifs.set_settings(s);
                Task::none()
            }
        }
    }

    fn on_notify_event(&mut self, ev: NotifyEvent) -> Task<Msg> {
        if self.phase != Phase::Main {
            return Task::none();
        }
        match ev {
            NotifyEvent::Clicked(key) => {
                let Some(target) = parse_key(&key) else { return Task::none() };
                // Fuera lo que tape la conversación (visor, diálogos, menús).
                self.overlay = None;
                self.chat_actions.menu = None;
                // Un foro abre su lista de temas. Ya abierto, el aviso se da por leído igualmente.
                let open = match target {
                    Target::Chat(peer) => self.update(Msg::Open(peer)),
                    Target::Topic(th) => self.update(Msg::Forum(ForumMsg::OpenTopic(th))),
                };
                self.notify_read(&key);
                Task::batch([focus_window(), open])
            }
            NotifyEvent::Action(key, id) if id == ACTION_MARK_READ => match parse_key(&key) {
                Some(Target::Chat(peer)) => self.update(Msg::Chat(ChatMsg::MarkRead(peer))),
                Some(Target::Topic(th)) => self.mark_topic_read_from_notice(th, &key),
                None => Task::none(),
            },
            NotifyEvent::Action(..) => Task::none(),
        }
    }

    /// «Marcar como leído» de un tema desde el aviso: con el tema cargado, como el menú del tema; si
    /// no, hasta el último mensaje avisado.
    fn mark_topic_read_from_notice(&mut self, th: ThreadRef, key: &ChatKey) -> Task<Msg> {
        let known = self.topic(th).and_then(|t| t.last_message.as_ref()).is_some();
        if known {
            return self.update(Msg::Forum(ForumMsg::MarkRead(th)));
        }
        let max = self.notifs.last.get(key).copied();
        self.notify_read(key);
        match max {
            Some(max) => self.call(move |b| async move { b.mark_topic_read(th, max).await }, |r| Msg::Forum(ForumMsg::Done(r))),
            None => Task::none(),
        }
    }
}

/// Enfoca la ventana de la app (en Wayland el compositor puede limitarse a resaltarla).
fn focus_window() -> Task<Msg> {
    window::latest().and_then(window::gain_focus)
}

/// Codifica un avatar decodificado como PNG (los SO piden un archivo de imagen).
fn png_of(handle: &iced::widget::image::Handle) -> Option<Vec<u8>> {
    use iced::widget::image::Handle;
    match handle {
        Handle::Rgba { width, height, pixels, .. } => {
            let img = image::RgbaImage::from_raw(*width, *height, pixels.to_vec())?;
            let mut out = std::io::Cursor::new(Vec::new());
            img.write_to(&mut out, image::ImageFormat::Png).ok()?;
            Some(out.into_inner())
        }
        Handle::Bytes(_, bytes) => Some(bytes.to_vec()),
        Handle::Path(_, p) => std::fs::read(p).ok(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn claves_ida_y_vuelta() {
        for p in [PeerRef::User(UserId(42)), PeerRef::Group(GroupId(-7)), PeerRef::Secret(SecretChatId(3))] {
            assert_eq!(parse_key(&chat_key(p)), Some(Target::Chat(p)));
        }
        let th = ThreadRef { group: GroupId(5), topic: MsgId(9) };
        assert_eq!(parse_key(&topic_key(th)), Some(Target::Topic(th)));
        assert_eq!(parse_key(&ChatKey::from("x:1")), None);
        assert_eq!(parse_key(&ChatKey::from("u:no")), None);
    }

    #[test]
    fn ajustes_ida_y_vuelta() {
        let s = NotifySettings {
            enabled: false,
            groups: false,
            preview: Preview::Hidden,
            secret_preview: Preview::Full,
            sound: false,
            dnd: DoNotDisturb::Until(1234),
            ..NotifySettings::default()
        };
        let map: HashMap<&str, String> = encode(&s).into_iter().collect();
        assert_eq!(decode(|k| map.get(k).cloned()), s);
        assert_eq!(decode(|_| Some("basura".into())), NotifySettings::default());
    }
}
