//! Gestión de chats y mensajes desde menús contextuales (clic derecho o «⋯»): leer / no leído,
//! fijar, silenciar con plazo, archivar, información, vaciar, borrar, salir, bloquear; y en
//! mensajes: reaccionar, responder, copiar, enlace, reenviar, editar, fijar, seleccionar varios y
//! borrar en bloque. Estado optimista con reversión ante error, confirmación en lo irreversible y
//! avisos con «Deshacer» cuando se puede. La vista vive en `view/context_menu.rs`.

use std::cell::Cell;
use std::collections::{BTreeSet, HashMap};
use std::time::{Duration, Instant};

use iced::Point;
use iced::Task;
use iced::keyboard::{Key, Modifiers, key::Named};
use tc_core::*;

use super::{App, Compose, Msg, error_text, now};

/// Segundos que se ofrece «Deshacer» (§8.7: 6 s si el aviso tiene acción).
pub const UNDO_SECS: u64 = 6;
/// Plazo para editar un mensaje propio en Telegram (48 h; sin plazo en «Mensajes guardados»).
pub const EDIT_WINDOW_SECS: i64 = 48 * 3600;

/// Plazos de «Silenciar».
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MuteFor {
    Hour,
    EightHours,
    TwoDays,
    Forever,
}

impl MuteFor {
    pub const ALL: [MuteFor; 4] = [MuteFor::Hour, MuteFor::EightHours, MuteFor::TwoDays, MuteFor::Forever];

    pub fn label(self) -> &'static str {
        match self {
            MuteFor::Hour => "Durante 1 hora",
            MuteFor::EightHours => "Durante 8 horas",
            MuteFor::TwoDays => "Durante 2 días",
            MuteFor::Forever => "Para siempre",
        }
    }

    fn short(self) -> &'static str {
        match self {
            MuteFor::Hour => "1 hora",
            MuteFor::EightHours => "8 horas",
            MuteFor::TwoDays => "2 días",
            MuteFor::Forever => "siempre",
        }
    }

    /// Fin del silencio desde `now` (`None` = para siempre).
    pub fn until(self, now: Timestamp) -> Option<Timestamp> {
        let secs = match self {
            MuteFor::Hour => 3600,
            MuteFor::EightHours => 8 * 3600,
            MuteFor::TwoDays => 2 * 86_400,
            MuteFor::Forever => return None,
        };
        Some(Timestamp(now.0 + secs))
    }
}

/// Sobre qué se abrió el menú.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MenuTarget {
    Chat(PeerRef),
    Message(MsgId),
}

/// Menú contextual abierto en `at` (coordenadas de la ventana).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Menu {
    pub target: MenuTarget,
    pub at: Point,
    /// Mostrando los plazos de «Silenciar».
    pub mute_sub: bool,
}

/// Diálogos de confirmación (lo irreversible).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Confirm {
    /// `can_revoke`: chat privado con otra persona (no bots ni «Mensajes guardados»).
    DeleteChat { peer: PeerRef, revoke: bool, can_revoke: bool },
    ClearHistory { peer: PeerRef },
    Leave { peer: PeerRef },
    Block { peer: PeerRef, user: UserId },
    /// Borrado en bloque de la selección; `can_revoke` si todos son míos.
    DeleteMessages { peer: PeerRef, ids: Vec<MsgId>, revoke: bool, can_revoke: bool },
}

/// «Ver información»: `None` mientras se carga.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InfoView {
    pub peer: PeerRef,
    pub info: Option<Result<ChatInfo, String>>,
}

/// Selector de destino de «Reenviar».
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ForwardPicker {
    pub from: PeerRef,
    pub ids: Vec<MsgId>,
    pub query: String,
    pub busy: bool,
}

/// Aviso con «Deshacer».
#[derive(Debug, Clone)]
pub struct Undo {
    pub text: String,
    pub action: Box<ChatMsg>,
    born: Instant,
}

/// Estado de los menús y acciones (un solo campo en `App`).
#[derive(Debug, Default)]
pub struct ChatActions {
    /// Último clic en la ventana (lo anota `view::context_menu::probe`): donde se abre el menú.
    pub cursor: Cell<Point>,
    pub menu: Option<Menu>,
    /// Fila de la lista bajo el ratón (muestra «⋯»).
    pub hover_chat: Option<PeerRef>,
    pub confirm: Option<Confirm>,
    pub info: Option<InfoView>,
    pub forward: Option<ForwardPicker>,
    pub undo: Option<Undo>,
    /// La lista muestra los archivados.
    pub show_archived: bool,
    /// Mensajes seleccionados en la conversación abierta (`None` = sin modo selección).
    pub selection: Option<BTreeSet<MsgId>>,
    /// Bloqueo conocido por usuario (de «Ver información» o de lo hecho aquí).
    pub blocked: HashMap<UserId, bool>,
    /// Mensajes fijados desde aquí (la red no dice cuáles lo están en cada mensaje).
    pub pinned_msgs: HashMap<PeerRef, BTreeSet<MsgId>>,
}

impl ChatActions {
    /// ¿Hace falta el reloj para retirar el aviso con «Deshacer»?
    pub fn needs_tick(&self) -> bool {
        self.undo.is_some()
    }

    pub fn tick(&mut self, now: Instant) {
        if self.undo.as_ref().is_some_and(|u| now.saturating_duration_since(u.born) >= Duration::from_secs(UNDO_SECS)) {
            self.undo = None;
        }
    }

    /// ¿Hay un diálogo propio abierto (bloquea atajos y otros menús)?
    pub fn modal_open(&self) -> bool {
        self.confirm.is_some() || self.info.is_some() || self.forward.is_some()
    }

    pub fn is_selected(&self, id: MsgId) -> bool {
        self.selection.as_ref().is_some_and(|s| s.contains(&id))
    }

    pub fn msg_pinned(&self, peer: PeerRef, id: MsgId) -> bool {
        self.pinned_msgs.get(&peer).is_some_and(|s| s.contains(&id))
    }
}

/// Mensajes de los menús contextuales y sus acciones.
#[derive(Debug, Clone)]
pub enum ChatMsg {
    /// Fija el punto donde se abrirá el siguiente menú (guiones de capturas y atajos de teclado).
    SetCursor(f32, f32),
    HoverChat(Option<PeerRef>),
    OpenChatMenu(PeerRef),
    OpenMessageMenu(MsgId),
    CloseMenu,
    MuteSubmenu,
    // ---- chat
    MarkRead(PeerRef),
    MarkUnread(PeerRef),
    Pin(PeerRef, bool),
    /// `None` = reactivar el sonido.
    Mute(PeerRef, Option<MuteFor>),
    Archive(PeerRef, bool),
    ShowInfo(PeerRef),
    InfoLoaded(PeerRef, BackendResult<ChatInfo>),
    CloseInfo,
    AskClear(PeerRef),
    AskDeleteChat(PeerRef),
    AskLeave(PeerRef),
    AskBlock(PeerRef),
    Unblock(PeerRef),
    SetRevoke(bool),
    Confirm,
    CancelConfirm,
    ToggleArchived,
    ArchivedLoaded(BackendResult<Vec<Dialog>>),
    Undo,
    /// Respuesta a una acción ya aplicada en pantalla (`prev` para revertir).
    Done { peer: PeerRef, prev: Option<Box<Dialog>>, ok: Option<(String, Option<Box<ChatMsg>>)>, result: BackendResult<()> },
    /// Respuesta a un borrado / salida ya aplicado (la fila se quitó; `prev` para devolverla).
    Removed { prev: Option<Box<Dialog>>, ok: String, result: BackendResult<()> },
    Cleared { peer: PeerRef, result: BackendResult<()> },
    BlockDone { peer: PeerRef, user: UserId, blocked: bool, result: BackendResult<()> },
    // ---- mensajes
    CopyLink(MsgId),
    Forward(Vec<MsgId>),
    ForwardSelected,
    ForwardQuery(String),
    ForwardTo(PeerRef),
    Forwarded { to: PeerRef, n: usize, result: BackendResult<()> },
    CloseForward,
    PinMessage(MsgId, bool),
    PinnedMessage { peer: PeerRef, id: MsgId, pinned: bool, result: BackendResult<()> },
    StartSelect(MsgId),
    ToggleSelect(MsgId),
    CancelSelect,
    AskDeleteSelected,
}

/// Texto en español para los errores de estas acciones (códigos de Telegram incluidos).
pub fn action_error_text(e: &BackendError) -> String {
    match e {
        BackendError::Forbidden(code) => match code.as_str() {
            "PINNED_DIALOGS_TOO_MUCH" | "PINNED_TOO_MUCH" => {
                "Ya tienes el máximo de chats fijados. Desfija otro para fijar este.".into()
            }
            "CHAT_ADMIN_REQUIRED" => "Hace falta ser administrador para hacer eso en este chat.".into(),
            "USER_CREATOR" => "Eres quien creó este grupo: transfiere la propiedad antes de salir.".into(),
            "USER_NOT_PARTICIPANT" => "Ya no formas parte de este chat.".into(),
            "CHANNEL_PRIVATE" => "Este chat es privado o te han expulsado.".into(),
            "MESSAGE_DELETE_FORBIDDEN" => "No puedes borrar ese mensaje.".into(),
            "MESSAGE_EDIT_TIME_EXPIRED" => "Ya ha pasado el plazo para editar ese mensaje.".into(),
            "PIN_RESTRICTED" | "CHAT_NOT_SUPPORTED" => "En este chat no se pueden fijar mensajes ni marcarlo.".into(),
            "CHAT_WRITE_FORBIDDEN" => "No puedes escribir en ese chat.".into(),
            "USER_IS_BLOCKED" | "YOU_BLOCKED_USER" => "Hay un bloqueo entre vosotros: desbloquea para continuar.".into(),
            "CHAT_NOT_MODIFIED" => "No había nada que cambiar.".into(),
            UNSUPPORTED => "Esta acción aún no está disponible con esta conexión.".into(),
            _ => error_text(e),
        },
        BackendError::NotFound => "Ese chat ya no existe.".into(),
        BackendError::InvalidInput("user") => "No puedes bloquearte a ti.".into(),
        BackendError::InvalidInput("mute_until") => "Ese plazo ya ha pasado.".into(),
        e => error_text(e),
    }
}

/// Enlace público a un mensaje (`t.me/usuario/id`, o `t.me/c/canal/id` en supergrupos y canales
/// privados). `None` en chats privados, grupos básicos y secretos.
pub fn message_link(group: &Group, id: MsgId) -> Option<String> {
    if group.kind == GroupKind::Basic {
        return None;
    }
    if let Some(u) = group.username.as_deref().filter(|u| !u.is_empty()) {
        return Some(format!("https://t.me/{u}/{}", id.0));
    }
    // Id marcado de canal (convención de la Bot API): -(10^12 + id).
    const CHANNEL_OFFSET: i64 = 1_000_000_000_000;
    (group.id.0 < -CHANNEL_OFFSET).then(|| format!("https://t.me/c/{}/{}", -group.id.0 - CHANNEL_OFFSET, id.0))
}

impl App {
    // ------------------------------------------------------------ consultas para la vista

    /// Usuario al otro lado de un chat privado o secreto.
    pub fn chat_user(&self, peer: PeerRef) -> Option<UserId> {
        match peer {
            PeerRef::User(u) => Some(u),
            PeerRef::Secret(s) => self.secrets.get(&s).map(|c| c.user),
            PeerRef::Group(_) => None,
        }
    }

    /// «Mensajes guardados» (chat conmigo).
    pub fn is_saved(&self, peer: PeerRef) -> bool {
        matches!(peer, PeerRef::User(u) if Some(u) == self.me_id())
    }

    /// ¿Se puede borrar también para el otro? (privado con otra persona que no es un bot).
    pub fn can_revoke_chat(&self, peer: PeerRef) -> bool {
        matches!(peer, PeerRef::User(u) if !self.is_saved(peer) && !self.users.get(&u).is_some_and(|u| u.is_bot))
    }

    /// ¿Se puede editar este mensaje? (propio, con texto, dentro de plazo y con permiso de escribir).
    pub fn can_edit(&self, m: &Message) -> bool {
        let fresh = self.is_saved(m.peer) || now().0 - m.date.0 < EDIT_WINDOW_SECS;
        m.outgoing && !m.text.is_empty() && m.sticker.is_none() && fresh && !crate::conv::is_local(m) && self.write_block(m.peer).is_none()
    }

    /// Cuántos chats archivados hay (y cuántos con algo sin leer).
    pub fn archived_counts(&self) -> (usize, usize) {
        let a = self.dialogs.iter().filter(|d| d.archived);
        let (mut n, mut unread) = (0, 0);
        for d in a {
            n += 1;
            unread += usize::from(d.looks_unread() && !d.muted);
        }
        (n, unread)
    }

    /// Destinos posibles de «Reenviar», filtrados.
    pub fn forward_targets(&self, query: &str) -> Vec<&Dialog> {
        let q = crate::format::fold(query.trim());
        self.dialogs
            .iter()
            .filter(|d| !matches!(d.peer, PeerRef::Secret(_)) && !self.is_channel(d.peer) && crate::format::matches(&d.title, &q))
            .collect()
    }

    // ------------------------------------------------------------ carga

    /// Pide los archivados (para la fila «Archivados» y su lista).
    pub(crate) fn load_archived(&self) -> Task<Msg> {
        self.call(
            |b| async move { b.dialogs(DialogsPage { offset: 0, limit: 500, archived: true }).await },
            |r| Msg::Chat(ChatMsg::ArchivedLoaded(r)),
        )
    }

    // ------------------------------------------------------------ eventos del backend

    pub(crate) fn on_dialog_removed(&mut self, peer: PeerRef) -> Task<Msg> {
        self.dialogs.retain(|d| d.peer != peer);
        if self.chat_actions.menu.is_some_and(|m| m.target == MenuTarget::Chat(peer)) {
            self.chat_actions.menu = None;
        }
        self.close_conv_of(peer);
        Task::none()
    }

    pub(crate) fn on_history_cleared(&mut self, peer: PeerRef) -> Task<Msg> {
        if let Some(c) = self.conv.as_mut().filter(|c| c.peer == peer) {
            c.messages.clear();
            c.reached_start = true;
            c.unread_from = None;
            c.new_below = 0;
            self.compose = Compose::Normal;
            self.hover = None;
            self.reactions_for = None;
            self.chat_actions.selection = None;
        }
        self.chat_actions.pinned_msgs.remove(&peer);
        Task::none()
    }

    fn close_conv_of(&mut self, peer: PeerRef) {
        if self.conv.as_ref().is_some_and(|c| c.peer == peer) {
            self.conv = None;
            self.compose = Compose::Normal;
            self.composer = iced::widget::text_editor::Content::new();
            self.hover = None;
            self.reactions_for = None;
            self.chat_actions.selection = None;
            self.picker.close();
        }
    }

    // ------------------------------------------------------------ teclado y Escape

    /// Escape: cierra lo nuestro en orden (menú, diálogos, selección). `true` si cerró algo.
    pub(crate) fn chat_actions_escape(&mut self) -> bool {
        let a = &mut self.chat_actions;
        if a.menu.take().is_some() || a.confirm.take().is_some() || a.info.take().is_some() || a.forward.take().is_some() {
            return true;
        }
        a.selection.take().is_some()
    }

    /// Atajos: Supr borra el chat abierto (o la selección) con confirmación; Shift+F10 o la tecla
    /// Menú abren el menú del chat abierto; Enter confirma el diálogo abierto.
    pub(crate) fn chat_actions_key(&mut self, key: Key<&str>, modifiers: Modifiers) -> Option<Task<Msg>> {
        let a = &self.chat_actions;
        match key {
            Key::Named(Named::Enter) if a.confirm.is_some() => Some(self.on_chat(ChatMsg::Confirm)),
            Key::Named(Named::Delete) if self.overlay.is_none() && a.menu.is_none() && !a.modal_open() => {
                if a.selection.as_ref().is_some_and(|s| !s.is_empty()) {
                    return Some(self.on_chat(ChatMsg::AskDeleteSelected));
                }
                let peer = self.conv.as_ref().map(|c| c.peer)?;
                let msg = if matches!(peer, PeerRef::Group(_)) { ChatMsg::AskLeave(peer) } else { ChatMsg::AskDeleteChat(peer) };
                Some(self.on_chat(msg))
            }
            Key::Named(Named::ContextMenu) | Key::Named(Named::F10) if self.overlay.is_none() && !a.modal_open() => {
                if matches!(key, Key::Named(Named::F10)) && !modifiers.shift() {
                    return None;
                }
                let peer = self.conv.as_ref().map(|c| c.peer)?;
                // Sin clic de referencia: junto a la lista de chats.
                self.chat_actions.cursor.set(Point::new(tc_theme::tokens::layout::RAIL_WIDTH + 48.0, 96.0));
                Some(self.on_chat(ChatMsg::OpenChatMenu(peer)))
            }
            _ => None,
        }
    }

    // ------------------------------------------------------------ update

    /// Aplica `f` al diálogo ya en pantalla y devuelve el anterior (para revertir).
    fn optimistic(&mut self, peer: PeerRef, f: impl FnOnce(&mut Dialog)) -> Option<Box<Dialog>> {
        let prev = self.dialog(peer).cloned()?;
        let mut d = prev.clone();
        f(&mut d);
        self.upsert_dialog(d);
        Some(Box::new(prev))
    }

    fn show_undo(&mut self, text: String, action: ChatMsg) {
        self.chat_actions.undo = Some(Undo { text, action: Box::new(action), born: Instant::now() });
    }

    fn title_of(&self, peer: PeerRef) -> String {
        self.dialog(peer).map(|d| d.title.clone()).unwrap_or_default()
    }

    /// Lanza una acción sobre la fila ya cambiada en pantalla.
    fn dialog_op<F, Fut>(&self, peer: PeerRef, prev: Option<Box<Dialog>>, ok: Option<(String, Option<Box<ChatMsg>>)>, f: F) -> Task<Msg>
    where
        F: FnOnce(super::Backend) -> Fut,
        Fut: std::future::Future<Output = BackendResult<()>> + Send + 'static,
    {
        self.call(f, move |result| Msg::Chat(ChatMsg::Done { peer, prev, ok, result }))
    }

    /// Quita la fila al momento y lanza el borrado / salida.
    fn remove_op<F, Fut>(&mut self, peer: PeerRef, ok: String, f: F) -> Task<Msg>
    where
        F: FnOnce(super::Backend) -> Fut,
        Fut: std::future::Future<Output = BackendResult<()>> + Send + 'static,
    {
        let prev = self.dialog(peer).cloned().map(Box::new);
        self.dialogs.retain(|d| d.peer != peer);
        self.close_conv_of(peer);
        self.call(f, move |result| Msg::Chat(ChatMsg::Removed { prev, ok, result }))
    }

    fn open_menu(&mut self, target: MenuTarget) {
        self.chat_actions.menu = Some(Menu { target, at: self.chat_actions.cursor.get(), mute_sub: false });
        self.open_overlay_anim();
    }

    pub(crate) fn on_chat(&mut self, msg: ChatMsg) -> Task<Msg> {
        // Toda acción elegida en el menú lo cierra.
        if !matches!(
            msg,
            ChatMsg::HoverChat(_)
                | ChatMsg::SetCursor(..)
                | ChatMsg::MuteSubmenu
                | ChatMsg::InfoLoaded(..)
                | ChatMsg::Done { .. }
                | ChatMsg::Removed { .. }
                | ChatMsg::Cleared { .. }
                | ChatMsg::BlockDone { .. }
                | ChatMsg::Forwarded { .. }
                | ChatMsg::PinnedMessage { .. }
                | ChatMsg::ArchivedLoaded(_)
        ) {
            self.chat_actions.menu = None;
        }
        match msg {
            ChatMsg::SetCursor(x, y) => {
                self.chat_actions.cursor.set(Point::new(x, y));
                Task::none()
            }
            ChatMsg::HoverChat(p) => {
                self.chat_actions.hover_chat = p;
                Task::none()
            }
            ChatMsg::OpenChatMenu(peer) => {
                if self.dialog(peer).is_none() {
                    return Task::none();
                }
                self.open_menu(MenuTarget::Chat(peer));
                // Para ofrecer «Bloquear» o «Desbloquear» hace falta saber el estado.
                match self.chat_user(peer).filter(|u| !self.chat_actions.blocked.contains_key(u) && !self.is_saved(PeerRef::User(*u))) {
                    Some(_) if matches!(peer, PeerRef::User(_)) => {
                        self.call(move |b| async move { b.chat_info(peer).await }, move |r| Msg::Chat(ChatMsg::InfoLoaded(peer, r)))
                    }
                    _ => Task::none(),
                }
            }
            ChatMsg::OpenMessageMenu(id) => {
                let Some(m) = self.conv.as_ref().and_then(|c| c.find(id)) else { return Task::none() };
                if crate::conv::is_local(m) {
                    return Task::none();
                }
                let peer = m.peer;
                self.reactions_for = None;
                self.hover = Some(id);
                self.open_menu(MenuTarget::Message(id));
                self.load_reactions(peer)
            }
            ChatMsg::CloseMenu => Task::none(),
            ChatMsg::MuteSubmenu => {
                if let Some(m) = &mut self.chat_actions.menu {
                    m.mute_sub = !m.mute_sub;
                }
                Task::none()
            }
            ChatMsg::MarkRead(peer) => {
                let prev = self.optimistic(peer, |d| {
                    d.unread_count = 0;
                    d.unread_mentions = 0;
                    d.unread_mark = false;
                });
                self.dialog_op(peer, prev, None, move |b| async move { b.mark_dialog_read(peer).await })
            }
            ChatMsg::MarkUnread(peer) => {
                let prev = self.optimistic(peer, |d| d.unread_mark = true);
                self.dialog_op(peer, prev, None, move |b| async move { b.set_unread_mark(peer, true).await })
            }
            ChatMsg::Pin(peer, pin) => {
                let next = self.dialogs.iter().filter_map(|d| d.pinned_order).max().map_or(0, |m| m + 1);
                let prev = self.optimistic(peer, |d| d.pinned_order = if pin { d.pinned_order.or(Some(next)) } else { None });
                self.dialog_op(peer, prev, None, move |b| async move { b.set_pinned(peer, pin).await })
            }
            ChatMsg::Mute(peer, None) => {
                let prev = self.optimistic(peer, |d| d.muted = false);
                let ok = Some(("Sonido activado.".to_string(), None));
                self.dialog_op(peer, prev, ok, move |b| async move { b.set_muted(peer, false).await })
            }
            ChatMsg::Mute(peer, Some(f)) => {
                let prev = self.optimistic(peer, |d| d.muted = true);
                let until = f.until(now());
                let text = if f == MuteFor::Forever { "Chat silenciado.".to_string() } else { format!("Chat silenciado durante {}.", f.short()) };
                let ok = Some((text, Some(Box::new(ChatMsg::Mute(peer, None)))));
                self.dialog_op(peer, prev, ok, move |b| async move { b.mute_until(peer, until).await })
            }
            ChatMsg::Archive(peer, archive) => {
                let prev = self.optimistic(peer, |d| {
                    d.archived = archive;
                    if archive {
                        d.pinned_order = None;
                    }
                });
                let text = if archive { "Chat archivado." } else { "Chat devuelto a la lista." };
                let ok = Some((text.to_string(), Some(Box::new(ChatMsg::Archive(peer, !archive)))));
                self.dialog_op(peer, prev, ok, move |b| async move { b.set_archived(peer, archive).await })
            }
            ChatMsg::ShowInfo(peer) => {
                self.chat_actions.info = Some(InfoView { peer, info: None });
                self.open_overlay_anim();
                self.call(move |b| async move { b.chat_info(peer).await }, move |r| Msg::Chat(ChatMsg::InfoLoaded(peer, r)))
            }
            ChatMsg::InfoLoaded(peer, r) => {
                if let (Ok(i), Some(u)) = (&r, self.chat_user(peer)) {
                    self.chat_actions.blocked.insert(u, i.blocked);
                }
                if let Some(v) = self.chat_actions.info.as_mut().filter(|v| v.peer == peer) {
                    v.info = Some(r.map_err(|e| match e {
                        BackendError::Forbidden(ref c) if c == UNSUPPORTED => String::new(),
                        e => action_error_text(&e),
                    }));
                }
                Task::none()
            }
            ChatMsg::CloseInfo => {
                self.chat_actions.info = None;
                Task::none()
            }
            ChatMsg::AskClear(peer) => self.ask(Confirm::ClearHistory { peer }),
            ChatMsg::AskDeleteChat(peer) => {
                let can_revoke = self.can_revoke_chat(peer);
                self.ask(Confirm::DeleteChat { peer, revoke: false, can_revoke })
            }
            ChatMsg::AskLeave(peer) => self.ask(Confirm::Leave { peer }),
            ChatMsg::AskBlock(peer) => match self.chat_user(peer) {
                Some(user) => self.ask(Confirm::Block { peer, user }),
                None => Task::none(),
            },
            ChatMsg::Unblock(peer) => {
                let Some(user) = self.chat_user(peer) else { return Task::none() };
                self.chat_actions.blocked.insert(user, false);
                self.call(move |b| async move { b.set_blocked(user, false).await }, move |result| {
                    Msg::Chat(ChatMsg::BlockDone { peer, user, blocked: false, result })
                })
            }
            ChatMsg::SetRevoke(v) => {
                if let Some(Confirm::DeleteChat { revoke, can_revoke: true, .. } | Confirm::DeleteMessages { revoke, can_revoke: true, .. }) =
                    &mut self.chat_actions.confirm
                {
                    *revoke = v;
                }
                Task::none()
            }
            ChatMsg::CancelConfirm => {
                self.chat_actions.confirm = None;
                Task::none()
            }
            ChatMsg::Confirm => match self.chat_actions.confirm.take() {
                Some(c) => self.run_confirmed(c),
                None => Task::none(),
            },
            ChatMsg::ToggleArchived => {
                self.chat_actions.show_archived = !self.chat_actions.show_archived;
                self.list_from_top = 0.0;
                let load = if self.chat_actions.show_archived { self.load_archived() } else { Task::none() };
                Task::batch([
                    load,
                    iced::widget::operation::scroll_to(
                        iced::widget::Id::new(super::LIST_SCROLL_ID),
                        iced::widget::operation::AbsoluteOffset { x: None, y: Some(0.0) },
                    ),
                ])
            }
            ChatMsg::ArchivedLoaded(Ok(list)) => {
                let groups: Vec<PeerRef> = list.iter().map(|d| d.peer).collect();
                // Los que Telegram ya no da como archivados dejan de estar en la carpeta.
                self.dialogs.retain(|d| !d.archived || groups.contains(&d.peer));
                for d in list {
                    self.upsert_dialog(d);
                }
                Task::batch(groups.into_iter().map(|p| self.want_group(p)).collect::<Vec<_>>())
            }
            ChatMsg::ArchivedLoaded(Err(_)) => Task::none(),
            ChatMsg::Undo => match self.chat_actions.undo.take() {
                Some(u) => self.on_chat(*u.action),
                None => Task::none(),
            },
            ChatMsg::Done { peer, prev, ok, result } => {
                match result {
                    Ok(()) => match ok {
                        Some((text, Some(undo))) => self.show_undo(text, *undo),
                        Some((text, None)) => self.toast(text, false),
                        None => {}
                    },
                    Err(e) => {
                        // Revertir lo que se pintó por adelantado.
                        if let Some(p) = prev.filter(|_| self.dialog(peer).is_some()) {
                            self.upsert_dialog(*p);
                        }
                        self.toast(action_error_text(&e), true);
                    }
                }
                Task::none()
            }
            ChatMsg::Removed { prev, ok, result } => {
                match result {
                    Ok(()) => self.toast(ok, false),
                    Err(e) => {
                        if let Some(p) = prev {
                            self.upsert_dialog(*p);
                        }
                        self.toast(action_error_text(&e), true);
                    }
                }
                Task::none()
            }
            ChatMsg::Cleared { peer, result } => {
                match result {
                    Ok(()) => {
                        let _ = self.on_history_cleared(peer);
                        self.toast("Historial vaciado.".into(), false);
                    }
                    Err(e) => self.toast(action_error_text(&e), true),
                }
                Task::none()
            }
            ChatMsg::BlockDone { peer, user, blocked, result } => {
                match result {
                    Ok(()) if blocked => {
                        let who = self.users.get(&user).map_or_else(|| self.title_of(peer), User::display_name);
                        self.show_undo(format!("{who} está bloqueado."), ChatMsg::Unblock(peer));
                    }
                    Ok(()) => self.toast("Usuario desbloqueado.".into(), false),
                    Err(e) => {
                        self.chat_actions.blocked.insert(user, !blocked);
                        self.toast(action_error_text(&e), true);
                    }
                }
                Task::none()
            }
            // ---- mensajes
            ChatMsg::CopyLink(id) => {
                let link = match self.conv.as_ref().map(|c| c.peer) {
                    Some(PeerRef::Group(g)) => self.groups.get(&g).and_then(|g| message_link(g, id)),
                    _ => None,
                };
                match link {
                    Some(l) => {
                        self.toast("Enlace copiado.".into(), false);
                        iced::clipboard::write(l)
                    }
                    None => Task::none(),
                }
            }
            ChatMsg::Forward(ids) => {
                let Some(from) = self.conv.as_ref().map(|c| c.peer) else { return Task::none() };
                if ids.is_empty() || matches!(from, PeerRef::Secret(_)) {
                    return Task::none();
                }
                self.chat_actions.forward = Some(ForwardPicker { from, ids, query: String::new(), busy: false });
                self.open_overlay_anim();
                Task::none()
            }
            ChatMsg::ForwardSelected => {
                let ids: Vec<MsgId> = self.chat_actions.selection.as_ref().map(|s| s.iter().copied().collect()).unwrap_or_default();
                self.on_chat(ChatMsg::Forward(ids))
            }
            ChatMsg::ForwardQuery(q) => {
                if let Some(f) = &mut self.chat_actions.forward {
                    f.query = q;
                }
                Task::none()
            }
            ChatMsg::ForwardTo(to) => {
                let Some(f) = self.chat_actions.forward.as_mut().filter(|f| !f.busy) else { return Task::none() };
                f.busy = true;
                let (from, ids) = (f.from, f.ids.clone());
                let n = ids.len();
                self.call(move |b| async move { b.forward_messages(from, ids, vec![to], false).await }, move |result| {
                    Msg::Chat(ChatMsg::Forwarded { to, n, result })
                })
            }
            ChatMsg::Forwarded { to, n, result } => {
                match result {
                    Ok(()) => {
                        self.chat_actions.forward = None;
                        self.chat_actions.selection = None;
                        let what = if n == 1 { "Mensaje reenviado".to_string() } else { format!("{n} mensajes reenviados") };
                        let title = self.title_of(to);
                        self.toast(format!("{what} a {title}."), false);
                    }
                    Err(e) => {
                        if let Some(f) = &mut self.chat_actions.forward {
                            f.busy = false;
                        }
                        self.toast(action_error_text(&e), true);
                    }
                }
                Task::none()
            }
            ChatMsg::CloseForward => {
                self.chat_actions.forward = None;
                Task::none()
            }
            ChatMsg::PinMessage(id, pinned) => {
                let Some(peer) = self.conv.as_ref().map(|c| c.peer) else { return Task::none() };
                let set = self.chat_actions.pinned_msgs.entry(peer).or_default();
                if pinned {
                    set.insert(id);
                } else {
                    set.remove(&id);
                }
                self.call(move |b| async move { b.pin_message(peer, id, pinned).await }, move |result| {
                    Msg::Chat(ChatMsg::PinnedMessage { peer, id, pinned, result })
                })
            }
            ChatMsg::PinnedMessage { peer, id, pinned, result } => {
                match result {
                    Ok(()) => self.toast(if pinned { "Mensaje fijado." } else { "Mensaje desfijado." }.into(), false),
                    Err(e) => {
                        let set = self.chat_actions.pinned_msgs.entry(peer).or_default();
                        if pinned {
                            set.remove(&id);
                        } else {
                            set.insert(id);
                        }
                        self.toast(action_error_text(&e), true);
                    }
                }
                Task::none()
            }
            ChatMsg::StartSelect(id) => {
                self.reactions_for = None;
                self.chat_actions.selection = Some(BTreeSet::from([id]));
                Task::none()
            }
            ChatMsg::ToggleSelect(id) => {
                if let Some(s) = &mut self.chat_actions.selection {
                    if !s.remove(&id) {
                        s.insert(id);
                    }
                    if s.is_empty() {
                        self.chat_actions.selection = None;
                    }
                }
                Task::none()
            }
            ChatMsg::CancelSelect => {
                self.chat_actions.selection = None;
                Task::none()
            }
            ChatMsg::AskDeleteSelected => {
                let (Some(c), Some(sel)) = (self.conv.as_ref(), self.chat_actions.selection.as_ref()) else { return Task::none() };
                let ids: Vec<MsgId> = sel.iter().copied().collect();
                let can_revoke = !matches!(c.peer, PeerRef::Secret(_)) && ids.iter().all(|i| c.find(*i).is_some_and(|m| m.outgoing));
                let peer = c.peer;
                self.ask(Confirm::DeleteMessages { peer, ids, revoke: can_revoke, can_revoke })
            }
        }
    }

    fn ask(&mut self, c: Confirm) -> Task<Msg> {
        self.chat_actions.confirm = Some(c);
        self.open_overlay_anim();
        Task::none()
    }

    fn run_confirmed(&mut self, c: Confirm) -> Task<Msg> {
        match c {
            Confirm::DeleteChat { peer, revoke, .. } => {
                let ok = if revoke { format!("Chat con {} borrado para ambos.", self.title_of(peer)) } else { "Chat borrado.".into() };
                self.remove_op(peer, ok, move |b| async move { b.delete_dialog(peer, revoke).await })
            }
            Confirm::Leave { peer } => {
                let ok = format!("Has salido de «{}».", self.title_of(peer));
                self.remove_op(peer, ok, move |b| async move { b.leave_chat(peer).await })
            }
            Confirm::ClearHistory { peer } => {
                self.call(move |b| async move { b.clear_history(peer).await }, move |result| Msg::Chat(ChatMsg::Cleared { peer, result }))
            }
            Confirm::Block { peer, user } => {
                self.chat_actions.blocked.insert(user, true);
                self.call(move |b| async move { b.set_blocked(user, true).await }, move |result| {
                    Msg::Chat(ChatMsg::BlockDone { peer, user, blocked: true, result })
                })
            }
            Confirm::DeleteMessages { peer, ids, revoke, .. } => {
                self.chat_actions.selection = None;
                if matches!(self.compose, Compose::Edit(x) | Compose::Reply(x) if ids.contains(&x)) {
                    self.compose = Compose::Normal;
                }
                self.call(move |b| async move { b.delete_messages(peer, ids, revoke).await }, |r| {
                    Msg::Op(r.map_err(|e| BackendError::Forbidden(action_error_text(&e))))
                })
            }
        }
    }
}
