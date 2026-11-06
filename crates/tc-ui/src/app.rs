//! Estado de la aplicación, mensajes y `update` (arquitectura Elm de iced). `update` nunca
//! bloquea: toda llamada al backend es una `Task` y los eventos en vivo llegan por `Subscription`.

use std::collections::{HashMap, HashSet};
use std::future::Future;
use std::hash::{Hash, Hasher};
use std::sync::Arc;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use futures::stream::BoxStream;
use iced::keyboard::{self, Key, key::Named};
use iced::widget::operation::{self, AbsoluteOffset};
use iced::widget::{Id, canvas, text_editor};
use iced::{Subscription, Task, Theme, window};
use jiff::tz::TimeZone;
use tc_core::*;
use tc_theme::motion::{Easing, Property};
use tc_theme::{MotionPreference, Spring, ThemeKind};

use crate::anim::{Animator, Curve};
use crate::conv::{self, Conversation, LayoutInput, MsgKey, PAGE};
use crate::format;
use crate::measure::IcedMeasurer;
use crate::style::Brasa;

pub type Backend = Arc<dyn ChatBackend>;

/// Configuración de arranque que aporta `tc-app`.
#[derive(Clone)]
pub struct Flags {
    pub backend: Backend,
    /// Se ejecuta una vez dentro del runtime tokio de iced (p. ej. arrancar el guion del mock).
    pub on_start: Option<Arc<dyn Fn() + Send + Sync>>,
    pub theme: ThemeKind,
    pub reduced_motion: bool,
    /// Guion opcional (demostración / capturas): pasos con su instante desde el arranque.
    pub demo: Vec<(Duration, DemoStep)>,
    /// Guarda una captura RGBA (ruta, ancho, alto, bytes). La aporta `tc-app` (codifica PNG).
    pub on_screenshot: Option<SaveShot>,
}

/// Guarda una captura RGBA: ruta, ancho, alto, bytes.
pub type SaveShot = Arc<dyn Fn(&str, u32, u32, &[u8]) + Send + Sync>;

/// Paso de un guion de demostración.
#[derive(Debug, Clone)]
pub enum DemoStep {
    /// Entrega un mensaje como si viniera de la interfaz.
    Do(Box<Msg>),
    /// Captura la ventana en esa ruta.
    Shot(String),
    /// Desplaza la conversación hacia arriba esos px (prueba del scroll virtual y la paginación).
    ScrollUp(f32),
    Exit,
}

/// Envoltura con `Hash` estable para identificar la suscripción de eventos del backend.
#[derive(Clone)]
struct BackendHandle(Backend);

impl Hash for BackendHandle {
    fn hash<H: Hasher>(&self, state: &mut H) {
        "tc-backend-events".hash(state);
    }
}

fn backend_events(h: &BackendHandle) -> BoxStream<'static, BackendEvent> {
    h.0.events()
}

pub const SEARCH_ID: &str = "chat-search";
pub const COMPOSER_ID: &str = "composer";
pub const CONV_SCROLL_ID: &str = "conversation";
pub const LIST_SCROLL_ID: &str = "chat-list";

pub const REACTIONS: [&str; 6] = ["👍", "❤️", "😂", "🎉", "🙏", "🔥"];

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
pub enum Folder {
    #[default]
    All,
    Unread,
    Personal,
    Groups,
    Channels,
}

impl Folder {
    pub const ALL: [Folder; 5] = [Folder::All, Folder::Unread, Folder::Personal, Folder::Groups, Folder::Channels];

    pub fn label(self) -> &'static str {
        match self {
            Folder::All => "Todos",
            Folder::Unread => "Sin leer",
            Folder::Personal => "Personas",
            Folder::Groups => "Grupos",
            Folder::Channels => "Canales",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum Compose {
    #[default]
    Normal,
    Reply(MsgId),
    Edit(MsgId),
}

#[derive(Clone, Debug, PartialEq)]
pub enum Overlay {
    Delete { id: MsgId, revoke: bool, outgoing: bool },
    Key(Option<KeyVisual>),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum AnimKey {
    ChatSwitch,
    BubbleOpacity(MsgKey),
    BubbleY(MsgKey),
    SendButton,
    Overlay,
}

#[derive(Clone, Debug)]
pub struct Toast {
    pub id: u64,
    pub text: String,
    pub error: bool,
    born: Instant,
}

#[derive(Clone, Debug, PartialEq)]
pub enum LoadState {
    Loading,
    Ready,
    Failed(String),
}

#[derive(Debug, Clone)]
pub enum Msg {
    Started,
    Me(BackendResult<User>),
    DialogsLoaded(BackendResult<Vec<Dialog>>),
    RetryDialogs,
    Contacts(BackendResult<Vec<User>>),
    UserLoaded(BackendResult<User>),
    GroupLoaded(BackendResult<Group>),
    Secrets(BackendResult<Vec<SecretChat>>),
    Backend(BackendEvent),
    SearchChanged(String),
    ClearSearch,
    SetFolder(Folder),
    ListScrolled(iced::widget::scrollable::Viewport),
    Open(PeerRef),
    CloseChat,
    History { peer: PeerRef, older: bool, result: BackendResult<Vec<Message>> },
    ConvScrolled(iced::widget::scrollable::Viewport),
    JumpToBottom,
    Edit(text_editor::Action),
    Send,
    EditLast,
    Escape,
    Sent { peer: PeerRef, random_id: RandomId, result: BackendResult<Message> },
    Retry(RandomId),
    Hover(Option<MsgId>),
    ToggleReactions(MsgId),
    React(MsgId, Option<String>),
    ReplyTo(MsgId),
    StartEdit(MsgId),
    CancelCompose,
    Copy(MsgId),
    AskDelete(MsgId),
    SetRevoke(bool),
    ConfirmDelete,
    CloseOverlay,
    ShowKey,
    KeyLoaded(BackendResult<KeyVisual>),
    TogglePin,
    ToggleMute,
    ToggleTheme,
    ToggleMotion,
    Frame(Instant),
    Tick(Instant),
    Key(keyboard::Event),
    Op(BackendResult<()>),
    DemoTick(Instant),
    Shot(String, window::Screenshot),
}

pub struct App {
    backend: Backend,
    on_start: Option<Arc<dyn Fn() + Send + Sync>>,
    pub brasa: Brasa,
    pub anim: Animator<AnimKey>,
    pub me: Option<User>,
    pub users: HashMap<UserId, User>,
    pub groups: HashMap<GroupId, Group>,
    pub secrets: HashMap<SecretChatId, SecretChat>,
    requested_users: HashSet<UserId>,
    requested_groups: HashSet<GroupId>,
    pub dialogs: Vec<Dialog>,
    pub dialogs_state: LoadState,
    pub search: String,
    pub folder: Folder,
    pub list_from_top: f32,
    pub list_viewport: f32,
    pub conv: Option<Conversation>,
    pub connection: ConnectionState,
    pub typing: HashMap<PeerRef, (UserId, Instant)>,
    pub composer: text_editor::Content,
    pub compose: Compose,
    pub hover: Option<MsgId>,
    pub reactions_for: Option<MsgId>,
    pub overlay: Option<Overlay>,
    pub toasts: Vec<Toast>,
    next_toast: u64,
    local_seq: i32,
    pub tz: TimeZone,
    pub bg_cache: canvas::Cache,
    pub dots: u8,
    demo: std::collections::VecDeque<(Duration, DemoStep)>,
    demo_start: Option<Instant>,
    on_screenshot: Option<SaveShot>,
}

/// Instante actual en segundos Unix.
pub fn now() -> Timestamp {
    Timestamp(SystemTime::now().duration_since(UNIX_EPOCH).map_or(0, |d| d.as_secs() as i64))
}

/// Orden de la lista: fijados primero (por su posición), luego por actividad descendente.
pub fn sort_dialogs(list: &mut [Dialog]) {
    list.sort_by(|a, b| {
        let pa = a.pinned_order.unwrap_or(u32::MAX);
        let pb = b.pinned_order.unwrap_or(u32::MAX);
        pa.cmp(&pb).then(b.activity.cmp(&a.activity))
    });
}

/// Mensaje propio para cada error del backend (copia propia, §8.7).
pub fn error_text(e: &BackendError) -> String {
    match e {
        BackendError::Network => "Sin conexión. Se reintentará al volver la red.".into(),
        BackendError::FloodWait(s) => format!("Demasiadas peticiones seguidas; espera {s} s."),
        BackendError::Unauthorized => "La sesión ha caducado; vuelve a entrar.".into(),
        BackendError::NotFound => "Ese elemento ya no existe.".into(),
        BackendError::Forbidden(r) => {
            let mut c = r.chars();
            c.next().map_or_else(String::new, |f| f.to_uppercase().collect::<String>() + c.as_str())
        }
        BackendError::InvalidInput(_) => "No se puede enviar un mensaje vacío.".into(),
        BackendError::Crypto(_) => "Fallo de cifrado en el chat secreto.".into(),
        BackendError::Storage(_) => "No se pudo guardar en el disco.".into(),
        BackendError::Internal => "Algo ha fallado. Inténtalo otra vez.".into(),
    }
}

impl App {
    pub fn new(flags: Flags) -> (Self, Task<Msg>) {
        let app = Self {
            backend: flags.backend,
            on_start: flags.on_start,
            brasa: Brasa::new(flags.theme),
            anim: Animator::new(MotionPreference::from_reduced(flags.reduced_motion)),
            me: None,
            users: HashMap::new(),
            groups: HashMap::new(),
            secrets: HashMap::new(),
            requested_users: HashSet::new(),
            requested_groups: HashSet::new(),
            dialogs: Vec::new(),
            dialogs_state: LoadState::Loading,
            search: String::new(),
            folder: Folder::All,
            list_from_top: 0.0,
            list_viewport: 800.0,
            conv: None,
            connection: ConnectionState::Online,
            typing: HashMap::new(),
            composer: text_editor::Content::new(),
            compose: Compose::Normal,
            hover: None,
            reactions_for: None,
            overlay: None,
            toasts: Vec::new(),
            next_toast: 0,
            local_seq: 0,
            tz: TimeZone::system(),
            bg_cache: canvas::Cache::new(),
            dots: 0,
            demo: flags.demo.into_iter().collect(),
            demo_start: None,
            on_screenshot: flags.on_screenshot,
        };
        let mut tasks = vec![
            app.call(|b| async move { b.me().await }, Msg::Me),
            app.load_dialogs(),
            app.call(|b| async move { b.contacts().await }, Msg::Contacts),
            app.call(|b| async move { b.secret_chats().await }, Msg::Secrets),
        ];
        if let Some(f) = app.on_start.clone() {
            tasks.push(Task::perform(async move { f() }, |()| Msg::Started));
        }
        (app, Task::batch(tasks))
    }

    pub fn title(&self) -> String {
        match self.conv.as_ref().and_then(|c| self.dialog(c.peer)) {
            Some(d) => format!("{} · teleClon", d.title),
            None => "teleClon".into(),
        }
    }

    pub fn theme(&self) -> Theme {
        self.brasa.iced_theme()
    }

    pub fn style(&self, _theme: &Theme) -> iced::theme::Style {
        crate::style::app_style(self.brasa)
    }

    pub fn subscription(&self) -> Subscription<Msg> {
        let mut subs = vec![
            Subscription::run_with(BackendHandle(self.backend.clone()), backend_events).map(Msg::Backend),
            keyboard::listen().map(Msg::Key),
        ];
        if self.anim.is_active() {
            subs.push(window::frames().map(Msg::Frame));
        }
        if !self.demo.is_empty() {
            subs.push(iced::time::every(Duration::from_millis(250)).map(Msg::DemoTick));
        }
        if !self.typing.is_empty() || !self.toasts.is_empty() {
            subs.push(iced::time::every(Duration::from_millis(400)).map(Msg::Tick));
        }
        Subscription::batch(subs)
    }

    // ------------------------------------------------------------ utilidades

    fn call<T, F, Fut>(&self, f: F, map: impl FnOnce(T) -> Msg + Send + 'static) -> Task<Msg>
    where
        F: FnOnce(Backend) -> Fut,
        Fut: Future<Output = T> + Send + 'static,
        T: Send + 'static,
    {
        Task::perform(f(self.backend.clone()), map)
    }

    fn load_dialogs(&self) -> Task<Msg> {
        self.call(
            |b| async move { b.dialogs(DialogsPage { offset: 0, limit: 500, archived: false }).await },
            Msg::DialogsLoaded,
        )
    }

    pub fn dialog(&self, peer: PeerRef) -> Option<&Dialog> {
        self.dialogs.iter().find(|d| d.peer == peer)
    }

    pub fn me_id(&self) -> Option<UserId> {
        self.me.as_ref().map(|u| u.id)
    }

    pub fn user_name(&self, id: UserId) -> String {
        if Some(id) == self.me_id() {
            return "Tú".into();
        }
        self.users.get(&id).map_or_else(|| "…".into(), User::display_name)
    }

    pub fn group_kind(&self, peer: PeerRef) -> Option<GroupKind> {
        match peer {
            PeerRef::Group(g) => self.groups.get(&g).map(|g| g.kind),
            _ => None,
        }
    }

    pub fn is_group_chat(&self, peer: PeerRef) -> bool {
        matches!(self.group_kind(peer), Some(GroupKind::Basic | GroupKind::Super))
            || matches!(peer, PeerRef::Group(_)) && self.group_kind(peer).is_none()
    }

    pub fn is_channel(&self, peer: PeerRef) -> bool {
        self.group_kind(peer) == Some(GroupKind::Channel)
    }

    /// ¿Se puede escribir en el chat? Devuelve el motivo si no.
    pub fn write_block(&self, peer: PeerRef) -> Option<String> {
        if self.is_channel(peer) {
            return Some("En este canal solo publica quien lo administra.".into());
        }
        if let PeerRef::Secret(s) = peer {
            let who = self.secrets.get(&s).map(|c| self.user_name(c.user)).unwrap_or_default();
            return match self.secrets.get(&s).map(|c| c.state) {
                Some(SecretChatState::Accepted) => None,
                Some(SecretChatState::Waiting) => Some(format!("Esperando a que {who} acepte el chat secreto…")),
                Some(SecretChatState::Discarded) => Some("Este chat secreto se cerró.".into()),
                None => Some("Cargando el estado del chat secreto…".into()),
            };
        }
        None
    }

    /// Id numérico estable para semillas de patrón y colores.
    pub fn peer_seed(peer: PeerRef) -> i64 {
        match peer {
            PeerRef::User(u) => u.0,
            PeerRef::Group(g) => g.0.wrapping_mul(31).wrapping_add(7),
            PeerRef::Secret(s) => i64::from(s.0).wrapping_mul(97).wrapping_add(13),
        }
    }

    /// Diálogos visibles según carpeta y búsqueda local.
    pub fn visible_dialogs(&self) -> Vec<&Dialog> {
        let q = format::fold(self.search.trim());
        self.dialogs.iter().filter(|d| self.in_folder(d, self.folder) && format::matches(&d.title, &q)).collect()
    }

    pub fn in_folder(&self, d: &Dialog, f: Folder) -> bool {
        match f {
            Folder::All => true,
            Folder::Unread => d.unread_count > 0 || self.conv.as_ref().is_some_and(|c| c.peer == d.peer),
            Folder::Personal => matches!(d.peer, PeerRef::User(_) | PeerRef::Secret(_)),
            Folder::Groups => matches!(d.peer, PeerRef::Group(_)) && !self.is_channel(d.peer),
            Folder::Channels => self.is_channel(d.peer),
        }
    }

    pub fn folder_unread(&self, f: Folder) -> usize {
        self.dialogs.iter().filter(|d| d.unread_count > 0 && !d.muted && self.in_folder(d, f)).count()
    }

    fn toast(&mut self, text: String, error: bool) {
        self.next_toast += 1;
        self.toasts.push(Toast { id: self.next_toast, text, error, born: Instant::now() });
        if self.toasts.len() > 3 {
            self.toasts.remove(0);
        }
    }

    fn want_user(&mut self, id: Option<UserId>) -> Task<Msg> {
        match id {
            Some(u) if !self.users.contains_key(&u) && Some(u) != self.me_id() && self.requested_users.insert(u) => {
                self.call(move |b| async move { b.user(u).await }, Msg::UserLoaded)
            }
            _ => Task::none(),
        }
    }

    fn want_group(&mut self, peer: PeerRef) -> Task<Msg> {
        match peer {
            PeerRef::Group(g) if !self.groups.contains_key(&g) && self.requested_groups.insert(g) => {
                self.call(move |b| async move { b.group(g).await }, Msg::GroupLoaded)
            }
            _ => Task::none(),
        }
    }

    fn upsert_dialog(&mut self, d: Dialog) {
        match self.dialogs.iter_mut().find(|x| x.peer == d.peer) {
            Some(x) => *x = d,
            None => self.dialogs.push(d),
        }
        sort_dialogs(&mut self.dialogs);
    }

    fn mark_read_task(&self) -> Task<Msg> {
        let Some(c) = &self.conv else { return Task::none() };
        let peer = c.peer;
        let unread = self.dialog(peer).is_some_and(|d| d.unread_count > 0);
        match c.newest_real() {
            Some(max) if unread => self.call(move |b| async move { b.mark_read(peer, max).await }, Msg::Op),
            _ => Task::none(),
        }
    }

    fn save_draft_task(&self) -> Task<Msg> {
        let Some(c) = &self.conv else { return Task::none() };
        if matches!(self.compose, Compose::Edit(_)) || self.write_block(c.peer).is_some() {
            return Task::none();
        }
        let peer = c.peer;
        let text = self.composer.text().trim().to_string();
        let old = self.dialog(peer).and_then(|d| d.draft.as_ref()).map(|d| d.text.clone()).unwrap_or_default();
        if text == old {
            return Task::none();
        }
        let reply_to = match self.compose {
            Compose::Reply(id) => Some(id),
            _ => None,
        };
        let draft = (!text.is_empty()).then_some(Draft { text, reply_to });
        self.call(move |b| async move { b.save_draft(peer, draft).await }, Msg::Op)
    }

    fn anim_ms(e: Easing) -> u32 {
        e.ms
    }

    /// Altura que ocupará un mensaje recién llegado (para compensar el scroll si se lee más arriba).
    fn row_height_estimate(&self, m: &Message) -> f32 {
        let Some(c) = &self.conv else { return 0.0 };
        let col = c.last_col.get();
        if col <= 0.0 {
            return 0.0;
        }
        let meta = format::hhmm(m.date, &self.tz);
        let bubble_max = crate::view::conversation::bubble_max(col, self.is_group_chat(c.peer));
        let input = LayoutInput { show_name: false, name: "", forwarded: m.forwarded_from.as_deref(), meta: &meta, bubble_max, day: 0 };
        let l = conv::layout_message(m, &input, &IcedMeasurer);
        l.bubble.height + conv::GAP_GROUP + if m.reactions.is_empty() { 0.0 } else { conv::REACT_GAP + conv::REACT_H }
    }

    fn scroll_bottom() -> Task<Msg> {
        operation::scroll_to(Id::new(CONV_SCROLL_ID), AbsoluteOffset { x: None, y: Some(0.0) })
    }

    fn new_random_id(&mut self) -> RandomId {
        self.local_seq += 1;
        let nanos = SystemTime::now().duration_since(UNIX_EPOCH).map_or(0, |d| d.as_nanos() as i64);
        RandomId(nanos ^ (i64::from(self.local_seq) << 48) ^ 0x5bd1_e995)
    }

    // ------------------------------------------------------------ update

    pub fn update(&mut self, msg: Msg) -> Task<Msg> {
        match msg {
            Msg::Started => Task::none(),
            Msg::Me(Ok(u)) => {
                self.me = Some(u);
                Task::none()
            }
            Msg::Me(Err(e)) | Msg::Contacts(Err(e)) | Msg::Secrets(Err(e)) => {
                self.toast(error_text(&e), true);
                Task::none()
            }
            Msg::DialogsLoaded(Ok(list)) => {
                self.dialogs = list;
                sort_dialogs(&mut self.dialogs);
                self.dialogs_state = LoadState::Ready;
                let peers: Vec<PeerRef> = self.dialogs.iter().map(|d| d.peer).collect();
                Task::batch(peers.into_iter().map(|p| self.want_group(p)).collect::<Vec<_>>())
            }
            Msg::DialogsLoaded(Err(e)) => {
                self.dialogs_state = LoadState::Failed(error_text(&e));
                Task::none()
            }
            Msg::RetryDialogs => {
                self.dialogs_state = LoadState::Loading;
                self.load_dialogs()
            }
            Msg::Contacts(Ok(list)) => {
                for u in list {
                    self.users.insert(u.id, u);
                }
                Task::none()
            }
            Msg::UserLoaded(Ok(u)) => {
                self.users.insert(u.id, u);
                Task::none()
            }
            Msg::GroupLoaded(Ok(g)) => {
                self.groups.insert(g.id, g);
                Task::none()
            }
            Msg::UserLoaded(Err(_)) | Msg::GroupLoaded(Err(_)) => Task::none(),
            Msg::Secrets(Ok(list)) => {
                let users: Vec<UserId> = list.iter().map(|s| s.user).collect();
                for s in list {
                    self.secrets.insert(s.id, s);
                }
                Task::batch(users.into_iter().map(|u| self.want_user(Some(u))).collect::<Vec<_>>())
            }
            Msg::Backend(ev) => self.on_event(ev),
            Msg::SearchChanged(s) => {
                self.search = s;
                self.list_from_top = 0.0;
                operation::scroll_to(Id::new(LIST_SCROLL_ID), AbsoluteOffset { x: None, y: Some(0.0) })
            }
            Msg::ClearSearch => {
                self.search.clear();
                Task::none()
            }
            Msg::SetFolder(f) => {
                self.folder = f;
                self.list_from_top = 0.0;
                operation::scroll_to(Id::new(LIST_SCROLL_ID), AbsoluteOffset { x: None, y: Some(0.0) })
            }
            Msg::ListScrolled(v) => {
                self.list_from_top = v.absolute_offset().y;
                self.list_viewport = v.bounds().height;
                Task::none()
            }
            Msg::Open(peer) => self.open(peer),
            Msg::CloseChat => {
                let t = self.save_draft_task();
                self.conv = None;
                self.compose = Compose::Normal;
                self.composer = text_editor::Content::new();
                t
            }
            Msg::History { peer, older, result } => self.on_history(peer, older, result),
            Msg::ConvScrolled(v) => self.on_conv_scrolled(v),
            Msg::JumpToBottom => {
                if let Some(c) = &mut self.conv {
                    c.new_below = 0;
                    c.from_bottom = 0.0;
                }
                Task::batch([Self::scroll_bottom(), self.mark_read_task()])
            }
            Msg::Edit(action) => {
                self.composer.perform(action);
                Task::none()
            }
            Msg::Send => self.send(),
            Msg::EditLast => {
                let me = self.me_id();
                match self.conv.as_ref().and_then(|c| c.last_own(me)).map(|m| m.id) {
                    Some(id) => self.update(Msg::StartEdit(id)),
                    None => Task::none(),
                }
            }
            Msg::Escape => self.escape(),
            Msg::Sent { peer, random_id, result } => {
                if let Some(c) = self.conv.as_mut().filter(|c| c.peer == peer) {
                    match result {
                        Ok(m) => {
                            c.upsert(m);
                        }
                        Err(e) => {
                            c.set_state(random_id, SendState::Failed);
                            self.toast(error_text(&e), true);
                        }
                    }
                }
                Task::none()
            }
            Msg::Retry(rid) => self.retry(rid),
            Msg::Hover(h) => {
                if h.is_some() || self.reactions_for.is_none() {
                    self.hover = h;
                }
                Task::none()
            }
            Msg::ToggleReactions(id) => {
                self.reactions_for = if self.reactions_for == Some(id) { None } else { Some(id) };
                self.hover = Some(id);
                Task::none()
            }
            Msg::React(id, emoji) => {
                self.reactions_for = None;
                let Some(peer) = self.conv.as_ref().map(|c| c.peer) else { return Task::none() };
                self.call(move |b| async move { b.send_reaction(peer, id, emoji).await }, Msg::Op)
            }
            Msg::ReplyTo(id) => {
                self.compose = Compose::Reply(id);
                operation::focus(Id::new(COMPOSER_ID))
            }
            Msg::StartEdit(id) => {
                let Some(m) = self.conv.as_ref().and_then(|c| c.find(id)) else { return Task::none() };
                self.composer = text_editor::Content::with_text(&m.text);
                self.compose = Compose::Edit(id);
                Task::batch([operation::focus(Id::new(COMPOSER_ID)), operation::move_cursor_to_end(Id::new(COMPOSER_ID))])
            }
            Msg::CancelCompose => {
                if matches!(self.compose, Compose::Edit(_)) {
                    self.composer = text_editor::Content::new();
                }
                self.compose = Compose::Normal;
                Task::none()
            }
            Msg::Copy(id) => match self.conv.as_ref().and_then(|c| c.find(id)) {
                Some(m) => {
                    let t = m.text.clone();
                    self.toast("Texto copiado.".into(), false);
                    iced::clipboard::write(t)
                }
                None => Task::none(),
            },
            Msg::AskDelete(id) => {
                let outgoing = self.conv.as_ref().and_then(|c| c.find(id)).is_some_and(|m| m.outgoing);
                self.overlay = Some(Overlay::Delete { id, revoke: outgoing, outgoing });
                self.open_overlay_anim();
                Task::none()
            }
            Msg::SetRevoke(v) => {
                if let Some(Overlay::Delete { revoke, .. }) = &mut self.overlay {
                    *revoke = v;
                }
                Task::none()
            }
            Msg::ConfirmDelete => {
                let Some(Overlay::Delete { id, revoke, .. }) = self.overlay.take() else { return Task::none() };
                let Some(peer) = self.conv.as_ref().map(|c| c.peer) else { return Task::none() };
                if matches!(self.compose, Compose::Edit(x) | Compose::Reply(x) if x == id) {
                    self.compose = Compose::Normal;
                }
                self.call(move |b| async move { b.delete_messages(peer, vec![id], revoke).await }, Msg::Op)
            }
            Msg::CloseOverlay => {
                self.overlay = None;
                Task::none()
            }
            Msg::ShowKey => {
                let Some(PeerRef::Secret(id)) = self.conv.as_ref().map(|c| c.peer) else { return Task::none() };
                self.overlay = Some(Overlay::Key(None));
                self.open_overlay_anim();
                self.call(move |b| async move { b.secret_key_visual(id).await }, Msg::KeyLoaded)
            }
            Msg::KeyLoaded(r) => {
                match r {
                    Ok(k) if matches!(self.overlay, Some(Overlay::Key(_))) => self.overlay = Some(Overlay::Key(Some(k))),
                    Ok(_) => {}
                    Err(e) => {
                        self.overlay = None;
                        self.toast(error_text(&e), true);
                    }
                }
                Task::none()
            }
            Msg::TogglePin => {
                let Some(d) = self.conv.as_ref().and_then(|c| self.dialog(c.peer)) else { return Task::none() };
                let (peer, pin) = (d.peer, d.pinned_order.is_none());
                self.call(move |b| async move { b.set_pinned(peer, pin).await }, Msg::Op)
            }
            Msg::ToggleMute => {
                let Some(d) = self.conv.as_ref().and_then(|c| self.dialog(c.peer)) else { return Task::none() };
                let (peer, mute) = (d.peer, !d.muted);
                self.call(move |b| async move { b.set_muted(peer, mute).await }, Msg::Op)
            }
            Msg::ToggleTheme => {
                let kind = match self.brasa.kind {
                    ThemeKind::Dark => ThemeKind::Light,
                    ThemeKind::Light => ThemeKind::Dark,
                };
                self.brasa = Brasa::new(kind);
                self.bg_cache.clear();
                Task::none()
            }
            Msg::ToggleMotion => {
                let reduced = !self.anim.pref.is_reduced();
                self.anim = Animator::new(MotionPreference::from_reduced(reduced));
                self.toast(
                    if reduced { "Movimiento reducido activado." } else { "Animaciones completas activadas." }.into(),
                    false,
                );
                Task::none()
            }
            Msg::Frame(now) => {
                self.anim.tick(now);
                Task::none()
            }
            Msg::Tick(now) => {
                self.typing.retain(|_, (_, until)| *until > now);
                self.toasts.retain(|t| now.saturating_duration_since(t.born) < Duration::from_secs(4));
                self.dots = (self.dots + 1) % 3;
                Task::none()
            }
            Msg::Key(ev) => self.on_key(ev),
            Msg::DemoTick(now) => self.demo_tick(now),
            Msg::Shot(path, shot) => {
                if let Some(c) = &self.conv {
                    tracing::info!(
                        "estado al capturar: {} mensajes en memoria, {} alturas en caché, a {:.0} px del final",
                        c.messages.len(),
                        c.cache.borrow().len(),
                        c.from_bottom
                    );
                }
                match &self.on_screenshot {
                    Some(save) => save(&path, shot.size.width, shot.size.height, &shot.rgba),
                    None => tracing::warn!("captura pedida sin `on_screenshot`"),
                }
                Task::none()
            }
            Msg::Op(Ok(())) => Task::none(),
            Msg::Op(Err(e)) => {
                self.toast(error_text(&e), true);
                Task::none()
            }
        }
    }

    fn demo_tick(&mut self, now: Instant) -> Task<Msg> {
        let start = *self.demo_start.get_or_insert(now);
        let mut tasks = Vec::new();
        while self.demo.front().is_some_and(|(at, _)| now.saturating_duration_since(start) >= *at) {
            let Some((_, step)) = self.demo.pop_front() else { break };
            tasks.push(match step {
                DemoStep::Do(m) => self.update(*m),
                DemoStep::Shot(path) => window::latest()
                    .and_then(window::screenshot)
                    .map(move |shot| Msg::Shot(path.clone(), shot)),
                DemoStep::ScrollUp(px) => operation::scroll_by(Id::new(CONV_SCROLL_ID), AbsoluteOffset { x: 0.0, y: px }),
                DemoStep::Exit => iced::exit(),
            });
        }
        Task::batch(tasks)
    }

    fn open_overlay_anim(&mut self) {
        self.anim.start(AnimKey::Overlay, 0.0, 1.0, Curve::new(Spring::SUAVE, Property::Opacity, Self::anim_ms(Easing::SUAVE)), Instant::now());
    }

    fn open(&mut self, peer: PeerRef) -> Task<Msg> {
        if self.conv.as_ref().is_some_and(|c| c.peer == peer) {
            return Task::none();
        }
        let save = self.save_draft_task();
        self.conv = Some(Conversation::new(peer));
        self.compose = Compose::Normal;
        self.hover = None;
        self.reactions_for = None;
        let draft = self.dialog(peer).and_then(|d| d.draft.clone());
        if let Some(r) = draft.as_ref().and_then(|d| d.reply_to) {
            self.compose = Compose::Reply(r);
        }
        self.composer = draft.map_or_else(text_editor::Content::new, |d| text_editor::Content::with_text(&d.text));
        self.bg_cache.clear();
        let now = Instant::now();
        self.anim.start(AnimKey::ChatSwitch, 0.0, 1.0, Curve::new(Spring::ESTANDAR, Property::Opacity, Self::anim_ms(Easing::ESTANDAR)), now);
        let load = self.call(
            move |b| async move { b.history(peer, Anchor::Latest, PAGE).await },
            move |result| Msg::History { peer, older: false, result },
        );
        let user = match peer {
            PeerRef::Secret(s) => self.secrets.get(&s).map(|c| c.user),
            PeerRef::User(u) => Some(u),
            PeerRef::Group(_) => None,
        };
        let want = self.want_user(user);
        Task::batch([save, load, want, Self::scroll_bottom()])
    }

    fn on_history(&mut self, peer: PeerRef, older: bool, result: BackendResult<Vec<Message>>) -> Task<Msg> {
        let unread = self.dialog(peer).map_or(0, |d| d.unread_count) as usize;
        let Some(c) = self.conv.as_mut().filter(|c| c.peer == peer) else { return Task::none() };
        let page = match result {
            Ok(p) => p,
            Err(e) => {
                c.loading = false;
                c.loading_older = false;
                c.error = Some(error_text(&e));
                return Task::none();
            }
        };
        let senders: Vec<Option<UserId>> = page.iter().map(|m| m.sender).collect();
        c.reached_start = page.len() < PAGE;
        if older {
            c.loading_older = false;
            c.prepend(page);
        } else {
            c.loading = false;
            c.messages = page;
            if unread > 0 {
                c.unread_from = c.messages.iter().rev().filter(|m| !m.outgoing).nth(unread - 1).map(|m| m.id);
            }
        }
        let mut tasks: Vec<Task<Msg>> = senders.into_iter().collect::<HashSet<_>>().into_iter().map(|s| self.want_user(s)).collect();
        if !older {
            tasks.push(self.mark_read_task());
        }
        Task::batch(tasks)
    }

    fn on_conv_scrolled(&mut self, v: iced::widget::scrollable::Viewport) -> Task<Msg> {
        let Some(c) = self.conv.as_mut() else { return Task::none() };
        // Anclado abajo: el desplazamiento interno es la distancia al final.
        c.from_bottom = v.absolute_offset().y;
        c.from_top = v.absolute_offset_reversed().y;
        let vh = v.bounds().height;
        let mut tasks = Vec::new();
        if c.near_bottom() && c.new_below > 0 {
            c.new_below = 0;
        }
        if c.from_top < 1.5 * vh
            && !c.loading
            && !c.loading_older
            && !c.reached_start
            && let Some(oldest) = c.oldest_real()
        {
            {
                c.loading_older = true;
                let peer = c.peer;
                tasks.push(self.call(
                    move |b| async move { b.history(peer, Anchor::Before(oldest), PAGE).await },
                    move |result| Msg::History { peer, older: true, result },
                ));
            }
        }
        if self.conv.as_ref().is_some_and(Conversation::near_bottom) {
            tasks.push(self.mark_read_task());
        }
        Task::batch(tasks)
    }

    fn on_event(&mut self, ev: BackendEvent) -> Task<Msg> {
        match ev {
            BackendEvent::Connection(s) => {
                self.connection = s;
                Task::none()
            }
            BackendEvent::DialogUpserted(d) => {
                let want = self.want_group(d.peer);
                self.upsert_dialog(d);
                want
            }
            BackendEvent::MessageNew(m) => {
                self.typing.remove(&m.peer);
                let want = self.want_user(m.sender);
                let mut tasks = vec![want];
                let in_conv = self.conv.as_ref().is_some_and(|c| c.peer == m.peer);
                if in_conv {
                    let h = self.row_height_estimate(&m);
                    let key = MsgKey::of(&m);
                    let incoming = !m.outgoing;
                    let c = self.conv.as_mut().expect("comprobado");
                    let near = c.near_bottom();
                    if c.upsert(m) {
                        if incoming {
                            self.anim.start(AnimKey::BubbleOpacity(key), 0.0, 1.0, Curve::new(Spring::RAPIDO, Property::Opacity, 120), Instant::now());
                        }
                        let c = self.conv.as_mut().expect("comprobado");
                        if near {
                            tasks.push(self.mark_read_task());
                        } else {
                            // Se lee más arriba: no mover lo visible y avisar en «ir al final».
                            c.new_below += 1;
                            c.from_bottom += h;
                            let y = c.from_bottom;
                            tasks.push(operation::scroll_to(Id::new(CONV_SCROLL_ID), AbsoluteOffset { x: None, y: Some(y) }));
                        }
                    }
                }
                Task::batch(tasks)
            }
            BackendEvent::MessageEdited(m) | BackendEvent::MessageSent { msg: m, .. } => {
                if let Some(c) = self.conv.as_mut().filter(|c| c.peer == m.peer) {
                    c.upsert(m);
                }
                Task::none()
            }
            BackendEvent::MessagesDeleted { peer, ids } => {
                if let Some(c) = self.conv.as_mut().filter(|c| c.peer == peer) {
                    c.remove(&ids);
                    if self.hover.is_some_and(|h| ids.contains(&h)) {
                        self.hover = None;
                    }
                    if matches!(self.compose, Compose::Edit(x) | Compose::Reply(x) if ids.contains(&x)) {
                        self.compose = Compose::Normal;
                    }
                }
                Task::none()
            }
            BackendEvent::MessageFailed { random_id, error } => {
                if let Some(c) = self.conv.as_mut() {
                    c.set_state(random_id, SendState::Failed);
                }
                self.toast(error_text(&error), true);
                Task::none()
            }
            BackendEvent::ReadInbox { .. } => Task::none(),
            BackendEvent::ReadOutbox { peer, max_id } => {
                if let Some(d) = self.dialogs.iter_mut().find(|d| d.peer == peer) {
                    d.read_outbox_max_id = d.read_outbox_max_id.max(max_id);
                }
                Task::none()
            }
            BackendEvent::Typing { peer, user, typing } => {
                if typing {
                    self.typing.insert(peer, (user, Instant::now() + Duration::from_secs(6)));
                } else {
                    self.typing.remove(&peer);
                }
                Task::none()
            }
            BackendEvent::UserStatus { user, status } => {
                if let Some(u) = self.users.get_mut(&user) {
                    u.status = status;
                }
                Task::none()
            }
            BackendEvent::Reactions { peer, msg_id, reactions } => {
                if let Some(m) = self.conv.as_mut().filter(|c| c.peer == peer).and_then(|c| c.find_mut(msg_id)) {
                    m.reactions = reactions;
                }
                Task::none()
            }
            BackendEvent::SecretChat(s) => {
                let want = self.want_user(Some(s.user));
                self.secrets.insert(s.id, s);
                want
            }
        }
    }

    fn send(&mut self) -> Task<Msg> {
        let text = self.composer.text().trim().to_string();
        let Some(peer) = self.conv.as_ref().map(|c| c.peer) else { return Task::none() };
        if text.is_empty() || self.write_block(peer).is_some() {
            return Task::none();
        }
        let now = Instant::now();
        self.anim.start(AnimKey::SendButton, 0.92, 1.0, Curve::new(Spring::REBOTE, Property::Scale, 220), now);
        match self.compose {
            Compose::Edit(id) => {
                self.composer = text_editor::Content::new();
                self.compose = Compose::Normal;
                self.call(move |b| async move { b.edit_message(peer, id, text, vec![]).await.map(|_| ()) }, Msg::Op)
            }
            mode => {
                let reply_to = match mode {
                    Compose::Reply(id) => Some(id),
                    _ => None,
                };
                let random_id = self.new_random_id();
                let me = self.me_id();
                let ttl = match peer {
                    PeerRef::Secret(s) => self.secrets.get(&s).and_then(|c| c.ttl_secs),
                    _ => None,
                };
                let pending = Message {
                    id: MsgId(-self.local_seq),
                    peer,
                    sender: me,
                    date: self::now(),
                    edit_date: None,
                    outgoing: true,
                    text: text.clone(),
                    entities: vec![],
                    reply_to,
                    forwarded_from: None,
                    media: None,
                    sticker: None,
                    reactions: vec![],
                    random_id: Some(random_id),
                    state: SendState::Pending,
                    ttl_secs: ttl,
                };
                let key = MsgKey::Random(random_id);
                self.anim.start(AnimKey::BubbleOpacity(key), 0.0, 1.0, Curve::new(Spring::ESTANDAR, Property::Opacity, 180), now);
                self.anim.start(AnimKey::BubbleY(key), 8.0, 0.0, Curve::new(Spring::ESTANDAR, Property::Position, 180), now);
                if let Some(c) = self.conv.as_mut() {
                    c.push_pending(pending);
                    c.from_bottom = 0.0;
                    c.new_below = 0;
                }
                self.composer = text_editor::Content::new();
                self.compose = Compose::Normal;
                let had_draft = self.dialog(peer).is_some_and(|d| d.draft.is_some());
                let out = OutgoingText { text, entities: vec![], reply_to, random_id };
                let send = self.call(
                    move |b| async move { b.send_text(peer, out).await },
                    move |result| Msg::Sent { peer, random_id, result },
                );
                let clear = if had_draft {
                    self.call(move |b| async move { b.save_draft(peer, None).await }, Msg::Op)
                } else {
                    Task::none()
                };
                Task::batch([send, clear, Self::scroll_bottom()])
            }
        }
    }

    fn retry(&mut self, rid: RandomId) -> Task<Msg> {
        let Some(c) = self.conv.as_mut() else { return Task::none() };
        let Some(m) = c.messages.iter_mut().find(|m| m.random_id == Some(rid) && conv::is_local(m)) else {
            return Task::none();
        };
        m.state = SendState::Pending;
        let (peer, out) = (m.peer, OutgoingText { text: m.text.clone(), entities: vec![], reply_to: m.reply_to, random_id: rid });
        self.call(
            move |b| async move { b.send_text(peer, out).await },
            move |result| Msg::Sent { peer, random_id: rid, result },
        )
    }

    fn escape(&mut self) -> Task<Msg> {
        if self.overlay.take().is_some() {
            return Task::none();
        }
        if self.reactions_for.take().is_some() {
            return Task::none();
        }
        if self.compose != Compose::Normal {
            return self.update(Msg::CancelCompose);
        }
        if !self.search.is_empty() {
            self.search.clear();
        }
        Task::none()
    }

    fn on_key(&mut self, ev: keyboard::Event) -> Task<Msg> {
        let keyboard::Event::KeyPressed { key, modifiers, .. } = ev else { return Task::none() };
        match key.as_ref() {
            Key::Named(Named::Escape) => self.escape(),
            Key::Character("k") if modifiers.command() => operation::focus(Id::new(SEARCH_ID)),
            Key::Character(c) if modifiers.command() && c.len() == 1 && ("1"..="5").contains(&c) => {
                let i = c.parse::<usize>().unwrap_or(1) - 1;
                self.update(Msg::SetFolder(Folder::ALL[i]))
            }
            Key::Named(Named::ArrowUp | Named::ArrowDown) if modifiers.alt() => {
                let list = self.visible_dialogs();
                let cur = self.conv.as_ref().and_then(|c| list.iter().position(|d| d.peer == c.peer));
                let next = match (key.as_ref(), cur) {
                    (Key::Named(Named::ArrowUp), Some(i)) => i.checked_sub(1),
                    (Key::Named(Named::ArrowDown), Some(i)) => Some(i + 1),
                    (_, None) => Some(0),
                    _ => None,
                };
                match next.and_then(|i| list.get(i)).map(|d| d.peer) {
                    Some(p) => self.open(p),
                    None => Task::none(),
                }
            }
            _ => Task::none(),
        }
    }
}
