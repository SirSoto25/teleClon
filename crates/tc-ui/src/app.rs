//! Estado de la aplicación, mensajes y `update` (arquitectura Elm de iced). `update` nunca
//! bloquea: toda llamada al backend es una `Task` y los eventos en vivo llegan por `Subscription`.

use std::collections::{HashMap, HashSet};
use std::future::Future;
use std::hash::{Hash, Hasher};
use std::sync::Arc;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use futures::future::BoxFuture;
use futures::stream::BoxStream;
use iced::keyboard::{self, Key, key::Named};
use iced::widget::operation::{self, AbsoluteOffset};
use iced::widget::{Id, canvas, qr_code, text_editor};
use iced::{Subscription, Task, Theme, window};
use jiff::tz::TimeZone;
use tc_core::*;
use tc_theme::motion::{Easing, Property};
use tc_theme::{MotionPreference, Spring, ThemeKind};

use crate::anim::{Animator, Curve};
use crate::conv::{self, Conversation, LayoutInput, MsgKey, PAGE};
use crate::format;
use crate::login::{self, Login, LoginMsg};
use crate::measure::IcedMeasurer;
use crate::media::{self, ImageKey, ImageRequest, Images};
use crate::stickers::{self, Art, Nav, Picker};
use crate::style::Brasa;

pub mod chat_actions;
/// Grupos con temas: lista de temas y conversación de un tema.
pub mod forum;
mod net_stickers;
mod panel;
mod sticker_anims;
mod reacts;
pub use reacts::{apply_mine, reaction_error_text};

pub type Backend = Arc<dyn ChatBackend>;
pub type Auth = Arc<dyn AuthApi>;

/// Backend + autenticación ya creados (p. ej. por un `Connector`).
#[derive(Clone)]
pub struct Session {
    pub chat: Backend,
    /// `None`: el backend no necesita inicio de sesión.
    pub auth: Option<Auth>,
}

impl std::fmt::Debug for Session {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Session").field("auth", &self.auth.is_some()).finish_non_exhaustive()
    }
}

/// Crea el backend de forma asíncrona DENTRO del runtime tokio de iced (la red real lanza sus
/// tareas al construirse). El error es un texto para el usuario, sin secretos.
pub type Connector = Arc<dyn Fn() -> BoxFuture<'static, Result<Session, String>> + Send + Sync>;

/// Configuración de arranque que aporta `tc-app`.
#[derive(Clone)]
pub struct Flags {
    /// Backend ya creado (el mock). Si es `None`, se usa `connect`.
    pub backend: Option<Backend>,
    /// Autenticación del mismo backend. `None`: se entra directamente (sin pantalla de login).
    pub auth: Option<Auth>,
    pub connect: Option<Connector>,
    /// Se ejecuta una vez dentro del runtime tokio de iced (p. ej. arrancar el guion del mock).
    pub on_start: Option<Arc<dyn Fn() + Send + Sync>>,
    pub theme: ThemeKind,
    pub reduced_motion: bool,
    /// Guion opcional (demostración / capturas): pasos con su instante desde el arranque.
    pub demo: Vec<(Duration, DemoStep)>,
    /// Guarda una captura RGBA (ruta, ancho, alto, bytes). La aporta `tc-app` (codifica PNG).
    pub on_screenshot: Option<SaveShot>,
    /// Backend simulado: la UI avisa de que la huella de clave y los datos son de prueba.
    pub simulated: bool,
    /// Pista bajo la tarjeta de login (p. ej. los valores de prueba del mock).
    pub login_hint: Option<String>,
    /// Aviso que se muestra al arrancar (p. ej. «faltan credenciales, se usa el mock»).
    pub notice: Option<String>,
}

impl Flags {
    /// Flags mínimos sobre un backend ya creado, sin login.
    pub fn new(backend: Backend) -> Self {
        Self { backend: Some(backend), ..Self::empty() }
    }

    /// Flags para un backend que se crea de forma asíncrona (`Connector`).
    pub fn with_connector(connect: Connector) -> Self {
        Self { connect: Some(connect), ..Self::empty() }
    }

    fn empty() -> Self {
        Self {
            backend: None,
            auth: None,
            connect: None,
            on_start: None,
            theme: ThemeKind::Dark,
            reduced_motion: false,
            demo: Vec::new(),
            on_screenshot: None,
            simulated: false,
            login_hint: None,
            notice: None,
        }
    }
}

/// Fase de la app: creando el backend, iniciando sesión o dentro.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Phase {
    Connecting,
    Auth,
    Main,
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

#[derive(Clone)]
struct AuthHandle(Auth);

impl Hash for AuthHandle {
    fn hash<H: Hasher>(&self, state: &mut H) {
        "tc-auth-events".hash(state);
    }
}

fn auth_events(h: &AuthHandle) -> BoxStream<'static, AuthState> {
    h.0.auth_events()
}

/// Opciones de autodestrucción de un chat secreto (segundos; `None` = desactivada).
pub const TTL_OPTIONS: [Option<u32>; 7] = [None, Some(5), Some(30), Some(60), Some(3600), Some(86_400), Some(604_800)];

pub const SEARCH_ID: &str = "chat-search";
pub const COMPOSER_ID: &str = "composer";
pub const CONV_SCROLL_ID: &str = "conversation";
pub const LIST_SCROLL_ID: &str = "chat-list";

/// Reacciones por defecto (el conjunto gratuito de la red). Lo que se ofrece en cada chat sale de
/// `App::reaction_choices` (ver `app/reacts.rs`).
pub const REACTIONS: [&str; 6] = DEFAULT_REACTIONS;
/// Cuántas reacciones caben en la barra del selector.
pub const MAX_REACTION_CHOICES: usize = 12;

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
    /// Crear un chat secreto: filtro de contactos, contacto elegido y si la petición está en curso.
    NewSecret { query: String, user: Option<UserId>, busy: bool },
    /// Autodestrucción del chat secreto abierto (y cerrarlo, con confirmación).
    Ttl { confirm_discard: bool },
    /// Visor de una foto a pantalla completa.
    Photo { peer: PeerRef, msg: MsgId },
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
    secs: u64,
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
    Connected(Result<Session, String>),
    RetryConnect,
    Login(LoginMsg),
    StickerSets(BackendResult<Vec<StickerSet>>),
    TogglePicker,
    PickerTab(usize),
    PickerNav(Nav),
    SendSticker(StickerId),
    NewSecret(Option<UserId>),
    SecretQuery(String),
    SecretPick(UserId),
    ConfirmNewSecret,
    SecretCreated(BackendResult<SecretChat>),
    ShowTtl,
    SetTtl(Option<u32>),
    AskDiscardSecret,
    DiscardSecret,
    AcceptSecret,
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
    /// Reacciones que admite un chat (`MessagesApi::available_reactions`).
    ReactionsLoaded(PeerRef, BackendResult<Vec<String>>),
    /// Respuesta de la red a una reacción ya aplicada en pantalla (`prev` para deshacerla).
    Reacted { peer: PeerRef, id: MsgId, prev: Vec<ReactionCount>, result: BackendResult<()> },
    /// Recientes y favoritos de stickers del servidor.
    NetStickers { recent: BackendResult<Vec<Sticker>>, faved: BackendResult<Vec<Sticker>> },
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
    /// Terminó la carga de una foto (`None`: falló la descarga o el decodificado).
    ImageLoaded(ImageKey, Option<media::Decoded>),
    /// Abrir el visor de la foto de un mensaje.
    OpenPhoto { peer: PeerRef, msg: MsgId },
    /// Visor: pasar a la foto anterior (`-1`) o siguiente (`1`) del chat.
    PhotoStep(i32),
    /// Visor: alternar ajuste/100 % (`None`), o fijarlo (`Some(true)` = 100 %).
    PhotoZoom(Option<bool>),
    /// Notas de voz y audios (ver `audio.rs`).
    Audio(crate::audio::AudioMsg),
    /// Menús contextuales de chats y mensajes (ver `app/chat_actions.rs`).
    Chat(chat_actions::ChatMsg),
    /// Lectura de la conversación: paginación hacia lo nuevo y marcado progresivo (`reading.rs`).
    Reading(crate::reading::ReadMsg),
    /// Stickers TGS animados: reloj, cargas, fotogramas, foco y hover (ver `sticker_anim.rs`).
    StickerAnim(crate::sticker_anim::AnimMsg),
    /// Grupos con temas (ver `app/forum.rs`).
    Forum(forum::ForumMsg),
    /// Panel del compositor: pestañas, emojis y GIFs (ver `panel.rs`, `app/panel.rs`).
    Panel(crate::panel::PanelMsg),
}

pub struct App {
    backend: Option<Backend>,
    auth: Option<Auth>,
    connect: Option<Connector>,
    pub phase: Phase,
    pub connect_error: Option<String>,
    main_started: bool,
    pub login: Login,
    /// Código QR del token vigente (y su URL, para no regenerarlo si no cambia).
    pub qr: Option<qr_code::Data>,
    pub(crate) qr_url: String,
    pub simulated: bool,
    pub login_hint: Option<String>,
    pub sticker_sets: Vec<StickerSet>,
    pub recents: tc_stickers::Recents,
    pub picker: Picker,
    /// Columnas de la rejilla del selector en la última vista (navegación con flechas).
    pub picker_cols: std::cell::Cell<usize>,
    pub art: Art,
    sticker_t0: Instant,
    /// Milisegundos del reloj de animación de los stickers (avanza con los frames).
    pub sticker_ms: f32,
    on_start: Option<Arc<dyn Fn() + Send + Sync>>,
    pub brasa: Brasa,
    /// Visor de fotos: `true` = tamaño real (100 %), `false` = ajustado a la ventana.
    pub photo_zoom: bool,
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
    /// Reacciones que se ofrecen por chat (`None` = pidiéndose).
    pub reaction_choices: HashMap<PeerRef, Option<Vec<String>>>,
    /// Recientes y favoritos de stickers de la cuenta, y si el pack propio es solo local.
    pub sticker_lists: stickers::NetLists,
    pub overlay: Option<Overlay>,
    pub toasts: Vec<Toast>,
    next_toast: u64,
    local_seq: i32,
    pub tz: TimeZone,
    pub bg_cache: canvas::Cache,
    /// Fotos decodificadas (avatares y medios) y cargas en curso.
    pub images: Images,
    /// Progreso de las descargas largas (id de archivo → hechos, total).
    pub downloads: HashMap<i64, (u64, u64)>,
    /// Reproductor de notas de voz y audios, y sus descargas.
    pub audio: crate::audio::AudioState,
    /// Menús contextuales, confirmaciones y selección de mensajes (`app/chat_actions.rs`).
    pub chat_actions: chat_actions::ChatActions,
    /// Animación de los stickers TGS (fotogramas en caché, foco de la ventana, hover del selector).
    pub sticker_anims: crate::sticker_anim::StickerAnims,
    /// Grupos con temas: lista abierta, tema abierto y temas cargados (`app/forum.rs`).
    pub forum: forum::ForumUi,
    /// Pestaña del panel del compositor y estado de emojis y GIFs (`picker.open` = panel abierto).
    pub panel: crate::panel::Panel,
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
        BackendError::PremiumRequired => "Eso necesita una cuenta de Telegram Premium.".into(),
        BackendError::ReactionUnavailable => "Esa reacción no está disponible en este chat.".into(),
    }
}

impl App {
    pub fn new(flags: Flags) -> (Self, Task<Msg>) {
        let now = Instant::now();
        let mut app = Self {
            backend: flags.backend,
            auth: flags.auth,
            connect: flags.connect,
            phase: Phase::Connecting,
            connect_error: None,
            main_started: false,
            login: Login::new(now),
            qr: None,
            qr_url: String::new(),
            simulated: flags.simulated,
            login_hint: flags.login_hint,
            sticker_sets: Vec::new(),
            recents: tc_stickers::Recents::default(),
            picker: Picker::default(),
            picker_cols: std::cell::Cell::new(6),
            art: Art::default(),
            sticker_t0: now,
            sticker_ms: 0.0,
            on_start: flags.on_start,
            brasa: Brasa::new(flags.theme),
            photo_zoom: false,
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
            reaction_choices: HashMap::new(),
            sticker_lists: stickers::NetLists::default(),
            overlay: None,
            toasts: Vec::new(),
            next_toast: 0,
            local_seq: 0,
            tz: TimeZone::system(),
            bg_cache: canvas::Cache::new(),
            images: Images::default(),
            downloads: HashMap::new(),
            audio: crate::audio::AudioState::default(),
            chat_actions: chat_actions::ChatActions::default(),
            sticker_anims: crate::sticker_anim::StickerAnims::default(),
            forum: forum::ForumUi::default(),
            panel: crate::panel::Panel::default(),
            dots: 0,
            demo: flags.demo.into_iter().collect(),
            demo_start: None,
            on_screenshot: flags.on_screenshot,
        };
        if let Some(n) = flags.notice {
            app.toast_for(n, true, 12);
        }
        let mut tasks = vec![app.boot()];
        if let Some(f) = app.on_start.clone() {
            tasks.push(Task::perform(async move { f() }, |()| Msg::Started));
        }
        (app, Task::batch(tasks))
    }

    /// Crea el backend si hace falta y decide entre login y app.
    fn boot(&mut self) -> Task<Msg> {
        if self.backend.is_none() {
            self.phase = Phase::Connecting;
            self.connect_error = None;
            return match self.connect.clone() {
                Some(c) => Task::perform(c(), Msg::Connected),
                None => {
                    self.connect_error = Some("No hay ningún backend configurado.".into());
                    Task::none()
                }
            };
        }
        if self.auth.is_some() {
            self.phase = Phase::Auth;
            self.login = Login::new(Instant::now());
            return self.call_auth(|a| async move { a.auth_state().await }, |r| Msg::Login(LoginMsg::State(r)));
        }
        self.enter_main()
    }

    /// Entra en la app (lista de chats) y lanza la carga inicial una sola vez por sesión.
    pub(crate) fn enter_main(&mut self) -> Task<Msg> {
        self.phase = Phase::Main;
        if self.main_started {
            return Task::none();
        }
        self.main_started = true;
        Task::batch([
            self.call(|b| async move { b.me().await }, Msg::Me),
            self.load_dialogs(),
            self.call(|b| async move { b.contacts().await }, Msg::Contacts),
            self.call(|b| async move { b.secret_chats().await }, Msg::Secrets),
            self.call(|b| async move { b.installed_sticker_sets().await }, Msg::StickerSets),
            self.load_net_stickers(),
            self.load_archived(),
        ])
    }

    /// La sesión se cerró o se revocó: vuelta a la pantalla de login con el estado limpio.
    pub(crate) fn back_to_login(&mut self) -> Task<Msg> {
        self.phase = Phase::Auth;
        self.main_started = false;
        self.login = Login::new(Instant::now());
        self.login.step = login::Step::Phone;
        self.conv = None;
        self.overlay = None;
        self.picker.close();
        self.dialogs.clear();
        self.dialogs_state = LoadState::Loading;
        self.me = None;
        self.images.clear();
        self.downloads.clear();
        self.toast("La sesión se ha cerrado. Vuelve a entrar.".into(), true);
        operation::focus(Id::new(login::PHONE_ID))
    }

    pub fn title(&self) -> String {
        if self.phase != Phase::Main {
            return "Iniciar sesión · Parlo".into();
        }
        match self.conv.as_ref().and_then(|c| self.dialog(c.peer)) {
            Some(d) => format!("{} · Parlo", d.title),
            None => "Parlo".into(),
        }
    }

    pub fn theme(&self) -> Theme {
        self.brasa.iced_theme()
    }

    pub fn style(&self, _theme: &Theme) -> iced::theme::Style {
        crate::style::app_style(self.brasa)
    }

    pub fn subscription(&self) -> Subscription<Msg> {
        let mut subs = vec![keyboard::listen().map(Msg::Key)];
        if let Some(b) = &self.backend {
            subs.push(Subscription::run_with(BackendHandle(b.clone()), backend_events).map(Msg::Backend));
        }
        if let Some(a) = &self.auth {
            subs.push(Subscription::run_with(AuthHandle(a.clone()), auth_events).map(|s| Msg::Login(LoginMsg::Event(s))));
        }
        if self.phase == Phase::Auth && self.login.needs_tick() {
            subs.push(iced::time::every(Duration::from_secs(1)).map(|t| Msg::Login(LoginMsg::Tick(t))));
        }
        if self.anim.is_active() {
            subs.push(window::frames().map(Msg::Frame));
        }
        // Reloj único de los stickers animados (pack propio y TGS): 30 fps como mucho y solo con algo
        // animado a la vista, la ventana con foco y sin movimiento reducido.
        if self.stickers_animating() || self.gifs_animating() {
            subs.push(iced::time::every(crate::sticker_anim::TICK).map(|t| Msg::StickerAnim(crate::sticker_anim::AnimMsg::Tick(t))));
        }
        subs.push(iced::event::listen_with(crate::sticker_anim::focus_event));
        if !self.demo.is_empty() {
            subs.push(iced::time::every(Duration::from_millis(250)).map(Msg::DemoTick));
        }
        if !self.typing.is_empty() || !self.toasts.is_empty() || self.chat_actions.needs_tick() {
            subs.push(iced::time::every(Duration::from_millis(400)).map(Msg::Tick));
        }
        if self.read_flush_due() {
            subs.push(iced::time::every(crate::reading::FLUSH_EVERY).map(|t| Msg::Reading(crate::reading::ReadMsg::Flush(t))));
        }
        if self.audio.needs_tick() {
            subs.push(iced::time::every(Duration::from_millis(100)).map(|_| Msg::Audio(crate::audio::AudioMsg::Tick)));
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
        // Con un tema abierto, las llamadas del grupo van a ese tema (`app/forum.rs`).
        match self.scoped_backend() {
            Some(b) => Task::perform(f(b), map),
            None => Task::none(),
        }
    }

    pub(crate) fn call_auth<T, F, Fut>(&self, f: F, map: impl FnOnce(T) -> Msg + Send + 'static) -> Task<Msg>
    where
        F: FnOnce(Auth) -> Fut,
        Fut: Future<Output = T> + Send + 'static,
        T: Send + 'static,
    {
        match &self.auth {
            Some(a) => Task::perform(f(a.clone()), map),
            None => Task::none(),
        }
    }

    /// ¿Hay stickers animados a la vista (con la ventana enfocada y sin movimiento reducido)?
    pub fn stickers_animating(&self) -> bool {
        if self.anim.pref.is_reduced() || self.phase != Phase::Main || !self.sticker_anims.focused {
            return false;
        }
        // En el selector: la animación procedural del pack propio o el TGS activo (hover/cursor).
        let in_picker = self.picker.open && self.picker_tab_stickers().iter().any(|s| stickers::is_animated(s) && stickers::art_index(s).is_some());
        in_picker || self.conv.as_ref().is_some_and(|c| c.anim_visible.get()) || self.tgs_visible()
    }

    /// Fase (0..1) del ciclo de animación para el sticker `seed` (desfasados entre sí).
    pub fn sticker_phase(&self, seed: usize) -> f32 {
        if self.anim.pref.is_reduced() {
            return 0.0;
        }
        (self.sticker_ms / stickers::LOOP_MS + seed as f32 * 0.137).fract()
    }

    pub fn picker_tabs(&self) -> Vec<stickers::Tab> {
        stickers::tabs_with(&self.sticker_sets, &self.recents, &self.sticker_lists)
    }

    pub fn picker_tab_stickers(&self) -> Vec<&Sticker> {
        // Con el panel en Emojis o GIFs no hay stickers a la vista (ni que cargar ni que animar).
        if self.panel.tab != crate::panel::PanelTab::Stickers {
            return Vec::new();
        }
        match self.picker_tabs().get(self.picker.tab) {
            Some(t) => stickers::tab_stickers_with(*t, &self.sticker_sets, &self.recents, &self.sticker_lists),
            None => Vec::new(),
        }
    }

    /// Contactos que pueden recibir un chat secreto, filtrados y por nombre.
    pub fn secret_candidates(&self, query: &str) -> Vec<&User> {
        let q = format::fold(query.trim());
        let mut v: Vec<&User> =
            self.users.values().filter(|u| u.is_contact && !u.is_bot && Some(u.id) != self.me_id() && format::matches(&u.display_name(), &q)).collect();
        v.sort_by_key(|u| format::fold(&u.display_name()));
        v
    }

    /// Invitación a un chat secreto pendiente de aceptar por mí.
    pub fn secret_invite(&self, peer: PeerRef) -> Option<&SecretChat> {
        match peer {
            PeerRef::Secret(s) => self.secrets.get(&s).filter(|c| c.state == SecretChatState::Waiting && !c.outgoing),
            _ => None,
        }
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
        if let Some(r) = self.forum_write_block(peer) {
            return Some(r);
        }
        if self.is_channel(peer) {
            return Some("En este canal solo publica quien lo administra.".into());
        }
        if let PeerRef::Secret(s) = peer {
            let who = self.secrets.get(&s).map(|c| self.user_name(c.user)).unwrap_or_default();
            return match self.secrets.get(&s).map(|c| c.state) {
                Some(SecretChatState::Accepted) => None,
                Some(SecretChatState::Waiting) if self.secret_invite(peer).is_some() => Some(format!("{who} te invita a un chat secreto.")),
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
        // Archivados aparte: solo con «Archivados» abierto (ver `app/chat_actions.rs`).
        let archived = self.chat_actions.show_archived;
        self.dialogs.iter().filter(|d| d.archived == archived && self.in_folder(d, self.folder) && format::matches(&d.title, &q)).collect()
    }

    pub fn in_folder(&self, d: &Dialog, f: Folder) -> bool {
        match f {
            Folder::All => true,
            Folder::Unread => d.looks_unread() || self.conv.as_ref().is_some_and(|c| c.peer == d.peer),
            Folder::Personal => matches!(d.peer, PeerRef::User(_) | PeerRef::Secret(_)),
            Folder::Groups => matches!(d.peer, PeerRef::Group(_)) && !self.is_channel(d.peer),
            Folder::Channels => self.is_channel(d.peer),
        }
    }

    pub fn folder_unread(&self, f: Folder) -> usize {
        self.dialogs.iter().filter(|d| !d.archived && d.looks_unread() && !d.muted && self.in_folder(d, f)).count()
    }

    fn toast(&mut self, text: String, error: bool) {
        self.toast_for(text, error, 4);
    }

    fn toast_for(&mut self, text: String, error: bool, secs: u64) {
        self.next_toast += 1;
        self.toasts.push(Toast { id: self.next_toast, text, error, born: Instant::now(), secs });
        if self.toasts.len() > 3 {
            self.toasts.remove(0);
        }
    }

    pub(crate) fn want_user(&mut self, id: Option<UserId>) -> Task<Msg> {
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
        let d = self.estimate_unread(d);
        // Diagnóstico de duplicados: mismo título con otra identidad (solo ids, nunca contenido).
        if let Some(x) = self.dialogs.iter().find(|x| x.peer != d.peer && x.title == d.title) {
            tracing::warn!(nuevo = ?d.peer, existente = ?x.peer, fijado = d.pinned_order.is_some(), "dos chats con el mismo título y distinto id");
        }
        match self.dialogs.iter_mut().find(|x| x.peer == d.peer) {
            Some(x) => *x = d,
            None => self.dialogs.push(d),
        }
        sort_dialogs(&mut self.dialogs);
    }

    fn mark_read_task(&self) -> Task<Msg> {
        let Some(c) = &self.conv else { return Task::none() };
        let peer = c.peer;
        // Con la ventana lejos del final, «lo más nuevo cargado» no es el último del chat.
        let unread = self.reading_dialog(peer).is_some_and(|d| d.unread_count > 0) && c.reading.reached_end;
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

    /// Backend de chat ya conectado (para módulos hermanos como `audio.rs`).
    pub(crate) fn chat_backend(&self) -> Option<Backend> {
        self.scoped_backend()
    }

    // ------------------------------------------------------------ fotos

    /// Avatar cargado de un chat (o `None`: se dibuja el generado).
    pub fn avatar_handle(&self, peer: PeerRef, photo: Option<ProfilePhoto>) -> Option<iced::widget::image::Handle> {
        photo.and_then(|p| self.images.handle(&ImageKey::Avatar { peer, photo: p.id }))
    }

    /// Avatar cargado de un usuario (remitentes en grupos, contactos).
    pub fn user_avatar(&self, u: UserId) -> Option<iced::widget::image::Handle> {
        self.avatar_handle(PeerRef::User(u), self.users.get(&u).and_then(|u| u.photo))
    }

    fn avatar_request(peer: PeerRef, photo: Option<ProfilePhoto>) -> Option<ImageRequest> {
        photo.map(|photo| ImageRequest::Avatar { peer, photo })
    }

    /// Id de la foto vecina (`dir < 0` anterior, `dir > 0` siguiente) de la conversación abierta.
    pub fn photo_neighbor(&self, peer: PeerRef, msg: MsgId, dir: i32) -> Option<MsgId> {
        let cv = self.conv.as_ref().filter(|c| c.peer == peer)?;
        let is_photo = |m: &Message| m.media.as_ref().is_some_and(|md| md.kind == MediaKind::Photo && md.file.is_some()) && !conv::is_local(m);
        let at = cv.messages.iter().position(|m| m.id == msg)?;
        if dir < 0 {
            cv.messages[..at].iter().rev().find(|m| is_photo(m)).map(|m| m.id)
        } else {
            cv.messages[at + 1..].iter().find(|m| is_photo(m)).map(|m| m.id)
        }
    }

    /// Petición de la foto (o miniatura de vídeo) de un mensaje, si tiene algo que mostrar.
    pub fn media_request(peer: PeerRef, m: &Message, variant: MediaVariant) -> Option<ImageRequest> {
        let md = m.media.as_ref()?;
        let file = md.file.as_ref()?;
        let visual = matches!(md.kind, MediaKind::Photo | MediaKind::Video | MediaKind::Gif | MediaKind::VideoNote);
        if !visual || conv::is_local(m) || !file.has_image(variant) {
            return None;
        }
        Some(ImageRequest::Media { peer, msg: m.id, file: file.clone(), variant, aspect: media::bubble_aspect(md.width, md.height) })
    }

    /// Lo que se ve ahora, por prioridad: visor, conversación (cabecera, burbujas visibles y avatares de
    /// remitentes), filas visibles de la lista y contactos del panel de chat secreto.
    fn wanted_images(&self) -> Vec<ImageRequest> {
        let mut out = Vec::new();
        if let Some(Overlay::Photo { peer, msg }) = &self.overlay
            && let Some(m) = self.conv.as_ref().filter(|c| c.peer == *peer).and_then(|c| c.find(*msg))
        {
            out.extend(Self::media_request(*peer, m, MediaVariant::Full));
            out.extend(Self::media_request(*peer, m, MediaVariant::Thumb));
        }
        if let Some(cv) = &self.conv {
            out.extend(Self::avatar_request(cv.peer, self.dialog(cv.peer).and_then(|d| d.photo)));
            let (a, z) = cv.image_range();
            let group = self.is_group_chat(cv.peer);
            for m in &cv.messages[a..z] {
                out.extend(Self::media_request(cv.peer, m, MediaVariant::Thumb));
                if group && !m.outgoing && let Some(s) = m.sender {
                    out.extend(Self::avatar_request(PeerRef::User(s), self.users.get(&s).and_then(|u| u.photo)));
                }
            }
        }
        // Stickers de la conversación y del selector antes que los avatares de la lista.
        out.extend(self.sticker_requests());
        out.extend(self.gif_requests());
        let list = self.visible_dialogs();
        let (first, last) = crate::view::sidebar::visible_rows(self.list_from_top, self.list_viewport, list.len());
        for d in &list[first..last] {
            out.extend(Self::avatar_request(d.peer, d.photo));
        }
        if let Some(Overlay::NewSecret { query, .. }) = &self.overlay {
            for u in self.secret_candidates(query).into_iter().take(12) {
                out.extend(Self::avatar_request(PeerRef::User(u.id), u.photo));
            }
        }
        out
    }

    /// Lanza la carga de lo visible que aún no está (como mucho `media::MAX_INFLIGHT` a la vez, salvo
    /// la foto del visor).
    pub(crate) fn pump_images(&mut self) -> Task<Msg> {
        let Some(backend) = self.backend.clone().filter(|_| self.phase == Phase::Main) else { return Task::none() };
        let now = Instant::now();
        let mut seen = HashSet::new();
        let mut tasks = Vec::new();
        for req in self.wanted_images() {
            let key = req.key();
            // La foto del visor la ha pedido el usuario: no espera a que acaben los avatares.
            if !self.images.can_start() && !matches!(key, ImageKey::Full { .. }) {
                break;
            }
            if !seen.insert(key) || !self.images.wants(&key, now) {
                continue;
            }
            self.images.start(key);
            tasks.push(Task::perform(media::load(backend.clone(), req), move |img| Msg::ImageLoaded(key, img)));
        }
        Task::batch(tasks)
    }

    // ------------------------------------------------------------ update

    pub fn update(&mut self, msg: Msg) -> Task<Msg> {
        // Los mensajes de cada frame, reloj, ratón y tecleo no cambian lo que hay que cargar.
        let pump = !matches!(msg, Msg::Frame(_) | Msg::Tick(_) | Msg::Hover(_) | Msg::Edit(_) | Msg::DemoTick(_) | Msg::Audio(crate::audio::AudioMsg::Tick) | Msg::StickerAnim(_))
            && !matches!(msg, Msg::Chat(chat_actions::ChatMsg::HoverChat(_)));
        let task = self.update_inner(msg);
        if pump { Task::batch([task, self.pump_images(), self.pump_gif_anims()]) } else { task }
    }

    fn update_inner(&mut self, msg: Msg) -> Task<Msg> {
        match msg {
            Msg::Started => Task::none(),
            Msg::Connected(Ok(s)) => {
                self.backend = Some(s.chat);
                self.auth = s.auth;
                self.boot()
            }
            Msg::Connected(Err(e)) => {
                self.connect_error = Some(e);
                Task::none()
            }
            Msg::RetryConnect => self.boot(),
            Msg::Login(m) => self.on_login(m),
            Msg::StickerSets(r) => self.on_sticker_sets(r),
            Msg::NetStickers { recent, faved } => self.on_net_stickers(recent, faved),
            Msg::ReactionsLoaded(peer, r) => self.on_reactions_loaded(peer, r),
            Msg::Reacted { peer, id, prev, result } => self.on_reacted(peer, id, prev, result),
            Msg::TogglePicker => {
                if self.picker.open {
                    self.picker.close();
                    return operation::focus(Id::new(COMPOSER_ID));
                }
                match self.conv.as_ref().map(|c| c.peer) {
                    Some(p) if self.write_block(p).is_none() && !matches!(self.compose, Compose::Edit(_)) => {
                        self.picker.open();
                        let opened = self.panel_opened();
                        if self.sticker_sets.is_empty() {
                            return Task::batch([opened, self.call(|b| async move { b.installed_sticker_sets().await }, Msg::StickerSets)]);
                        }
                        opened
                    }
                    _ => Task::none(),
                }
            }
            Msg::PickerTab(i) => {
                let n = self.picker_tabs().len();
                self.picker.select_tab(i, n);
                Task::none()
            }
            Msg::PickerNav(nav) => {
                if !self.picker.open {
                    return Task::none();
                }
                if let Some(t) = self.panel_nav(nav) {
                    return t;
                }
                let (len, n) = (self.picker_tab_stickers().len(), self.picker_tabs().len());
                match self.picker.nav(nav, len, self.picker_cols.get(), n) {
                    Some(i) => match self.picker_tab_stickers().get(i).map(|s| s.id) {
                        Some(id) => self.send_sticker(id),
                        None => Task::none(),
                    },
                    None => Task::none(),
                }
            }
            Msg::SendSticker(id) => self.send_sticker(id),
            Msg::NewSecret(user) => {
                self.overlay = Some(Overlay::NewSecret { query: String::new(), user, busy: false });
                self.open_overlay_anim();
                Task::none()
            }
            Msg::SecretQuery(q) => {
                if let Some(Overlay::NewSecret { query, .. }) = &mut self.overlay {
                    *query = q;
                }
                Task::none()
            }
            Msg::SecretPick(u) => {
                if let Some(Overlay::NewSecret { user, .. }) = &mut self.overlay {
                    *user = Some(u);
                }
                Task::none()
            }
            Msg::ConfirmNewSecret => {
                let Some(Overlay::NewSecret { query, user, busy }) = &self.overlay else { return Task::none() };
                // Con el filtro dejando un único contacto, Enter lo elige.
                let only = match self.secret_candidates(query).as_slice() {
                    [u] => Some(u.id),
                    _ => None,
                };
                let Some(target) = user.or(only).filter(|_| !*busy) else { return Task::none() };
                if let Some(Overlay::NewSecret { user, busy, .. }) = &mut self.overlay {
                    *user = Some(target);
                    *busy = true;
                }
                self.call(move |b| async move { b.request_secret_chat(target).await }, Msg::SecretCreated)
            }
            Msg::SecretCreated(r) => match r {
                Ok(chat) => {
                    if matches!(self.overlay, Some(Overlay::NewSecret { .. })) {
                        self.overlay = None;
                    }
                    let (id, user) = (chat.id, chat.user);
                    self.secrets.insert(id, chat);
                    let want = self.want_user(Some(user));
                    Task::batch([want, self.open(PeerRef::Secret(id))])
                }
                Err(e) => {
                    if let Some(Overlay::NewSecret { busy, .. }) = &mut self.overlay {
                        *busy = false;
                    }
                    self.toast(error_text(&e), true);
                    Task::none()
                }
            },
            Msg::ShowTtl => {
                if !matches!(self.conv.as_ref().map(|c| c.peer), Some(PeerRef::Secret(_))) {
                    return Task::none();
                }
                self.overlay = Some(Overlay::Ttl { confirm_discard: false });
                self.open_overlay_anim();
                Task::none()
            }
            Msg::SetTtl(ttl) => {
                let Some(PeerRef::Secret(id)) = self.conv.as_ref().map(|c| c.peer) else { return Task::none() };
                self.overlay = None;
                // Optimista: el evento `SecretChat` del backend lo confirma.
                if let Some(c) = self.secrets.get_mut(&id) {
                    c.ttl_secs = ttl;
                }
                self.call(move |b| async move { b.set_secret_ttl(id, ttl).await }, Msg::Op)
            }
            Msg::AskDiscardSecret => {
                if let Some(Overlay::Ttl { confirm_discard }) = &mut self.overlay {
                    *confirm_discard = true;
                }
                Task::none()
            }
            Msg::DiscardSecret => {
                let Some(PeerRef::Secret(id)) = self.conv.as_ref().map(|c| c.peer) else { return Task::none() };
                self.overlay = None;
                self.picker.close();
                self.call(move |b| async move { b.discard_secret_chat(id).await }, Msg::Op)
            }
            Msg::AcceptSecret => {
                let Some(PeerRef::Secret(id)) = self.conv.as_ref().map(|c| c.peer) else { return Task::none() };
                self.call(move |b| async move { b.accept_secret_chat(id).await.map(|_| ()) }, Msg::Op)
            }
            Msg::Me(Ok(u)) => {
                self.me = Some(u);
                Task::none()
            }
            Msg::Me(Err(e)) | Msg::Contacts(Err(e)) | Msg::Secrets(Err(e)) => {
                self.toast(error_text(&e), true);
                Task::none()
            }
            Msg::DialogsLoaded(Ok(list)) => {
                // Los archivados se cargan aparte (`load_archived`): se conservan.
                // Un chat solo puede estar en una carpeta: si la lista principal lo trae, gana sobre
                // una copia archivada obsoleta; y se descartan repetidos dentro de la propia lista.
                let fresh: std::collections::HashSet<PeerRef> = list.iter().map(|d| d.peer).collect();
                self.dialogs.retain(|d| d.archived && !fresh.contains(&d.peer));
                let mut seen = std::collections::HashSet::new();
                self.dialogs.extend(list.into_iter().filter(|d| seen.insert(d.peer)));
                let mut titles: std::collections::HashMap<&str, PeerRef> = std::collections::HashMap::new();
                for d in &self.dialogs {
                    if let Some(o) = titles.insert(d.title.as_str(), d.peer).filter(|o| *o != d.peer) {
                        tracing::warn!(a = ?o, b = ?d.peer, fijado = d.pinned_order.is_some(), "dos chats con el mismo título y distinto id");
                    }
                }
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
                if let Some(t) = self.jump_to_latest() {
                    return t;
                }
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
                match self.conv.as_ref().map(|c| c.peer) {
                    Some(peer) => self.load_reactions(peer),
                    None => Task::none(),
                }
            }
            Msg::React(id, emoji) => self.react(id, emoji),
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
                self.sticker_ms = now.saturating_duration_since(self.sticker_t0).as_secs_f32() * 1000.0;
                Task::none()
            }
            Msg::Tick(now) => {
                self.typing.retain(|_, (_, until)| *until > now);
                self.toasts.retain(|t| now.saturating_duration_since(t.born) < Duration::from_secs(t.secs));
                self.chat_actions.tick(now);
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
            Msg::ImageLoaded(key, img) => {
                if let ImageKey::Full { file, .. } = key {
                    self.downloads.remove(&file);
                }
                self.images.finish(key, img, Instant::now());
                Task::none()
            }
            Msg::OpenPhoto { peer, msg } => {
                if self.conv.as_ref().is_none_or(|c| c.peer != peer) {
                    return Task::none();
                }
                self.overlay = Some(Overlay::Photo { peer, msg });
                self.photo_zoom = false;
                self.open_overlay_anim();
                Task::none()
            }
            Msg::PhotoStep(dir) => {
                if let Some(Overlay::Photo { peer, msg }) = self.overlay
                    && let Some(next) = self.photo_neighbor(peer, msg, dir)
                {
                    self.overlay = Some(Overlay::Photo { peer, msg: next });
                    self.photo_zoom = false;
                }
                Task::none()
            }
            Msg::PhotoZoom(z) => {
                if matches!(self.overlay, Some(Overlay::Photo { .. })) {
                    self.photo_zoom = z.unwrap_or(!self.photo_zoom);
                }
                Task::none()
            }
            Msg::Audio(m) => self.on_audio(m),
            Msg::Chat(m) => self.on_chat(m),
            Msg::Reading(m) => self.on_reading(m),
            Msg::StickerAnim(m) => self.on_sticker_anim(m),
            Msg::Forum(m) => self.on_forum(m),
            Msg::Panel(m) => self.on_panel(m),
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
        // Un grupo con temas abre su lista de temas (`app/forum.rs`).
        if let Some(t) = self.forum_intercept_open(peer) {
            return t;
        }
        if self.conv.as_ref().is_some_and(|c| c.peer == peer) {
            return Task::none();
        }
        let save = self.save_draft_task();
        let carry = self.conv.as_ref().map(crate::reading::Carry::of);
        self.conv = Some(Conversation::new(peer));
        // Con no leídos: ventana alrededor del último leído (abre en «Sin leer desde aquí»).
        let anchor = self.start_reading(carry);
        self.compose = Compose::Normal;
        self.hover = None;
        self.reactions_for = None;
        self.picker.close();
        let draft = self.dialog(peer).and_then(|d| d.draft.clone());
        if let Some(r) = draft.as_ref().and_then(|d| d.reply_to) {
            self.compose = Compose::Reply(r);
        }
        self.composer = draft.map_or_else(text_editor::Content::new, |d| text_editor::Content::with_text(&d.text));
        self.bg_cache.clear();
        let now = Instant::now();
        self.anim.start(AnimKey::ChatSwitch, 0.0, 1.0, Curve::new(Spring::ESTANDAR, Property::Opacity, Self::anim_ms(Easing::ESTANDAR)), now);
        let load = self.call(
            move |b| async move { b.history(peer, anchor, PAGE).await },
            move |result| Msg::History { peer, older: false, result },
        );
        let user = match peer {
            PeerRef::Secret(s) => self.secrets.get(&s).map(|c| c.user),
            PeerRef::User(u) => Some(u),
            PeerRef::Group(_) => None,
        };
        let want = self.want_user(user);
        let reactions = self.load_reactions(peer);
        Task::batch([save, load, want, reactions, Self::scroll_bottom()])
    }

    fn on_history(&mut self, peer: PeerRef, older: bool, result: BackendResult<Vec<Message>>) -> Task<Msg> {
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
            // Hasta la primera maquetación: ¿hay algún sticker animado entre los últimos?
            c.anim_visible.set(c.messages.iter().rev().take(16).any(|m| m.sticker.as_ref().is_some_and(stickers::is_animated)));
        }
        let mut tasks: Vec<Task<Msg>> = senders.into_iter().collect::<HashSet<_>>().into_iter().map(|s| self.want_user(s)).collect();
        if !older {
            // Marca «Sin leer desde aquí», salto a ella y lectura progresiva (no todo de golpe).
            tasks.push(self.after_first_page(peer));
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
        tasks.push(self.reading_on_scroll(vh));
        Task::batch(tasks)
    }

    fn on_event(&mut self, ev: BackendEvent) -> Task<Msg> {
        // Temas de los foros (y filtro de la conversación de un tema): `app/forum.rs`.
        let ev = match self.forum_event(ev) {
            Ok(t) => return t,
            Err(ev) => *ev,
        };
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
                let in_conv = self.conv.as_ref().is_some_and(|c| c.peer == m.peer) && !self.defer_new_message(&m);
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
            BackendEvent::Download { file_id, done, total } => {
                self.downloads.insert(file_id, (done, total));
                Task::none()
            }
            BackendEvent::DialogRemoved { peer } => self.on_dialog_removed(peer),
            BackendEvent::HistoryCleared { peer } => self.on_history_cleared(peer),
            // Ya tratados en `forum_event`.
            BackendEvent::TopicMessageNew { .. } | BackendEvent::ForumTopicUpserted(_) | BackendEvent::ForumTopicsChanged { .. } => Task::none(),
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

    /// Envío optimista de un sticker (pendiente con id local → confirmado por `random_id`).
    fn send_sticker(&mut self, id: StickerId) -> Task<Msg> {
        let Some(peer) = self.conv.as_ref().map(|c| c.peer) else { return Task::none() };
        if self.write_block(peer).is_some() || matches!(self.compose, Compose::Edit(_)) {
            return Task::none();
        }
        let Some(sticker) = self.find_sticker(id).cloned() else { return Task::none() };
        let to_send = sticker.clone();
        self.recents.record(id);
        self.picker.close();
        let reply_to = match self.compose {
            Compose::Reply(r) => Some(r),
            _ => None,
        };
        let random_id = self.new_random_id();
        let ttl = match peer {
            PeerRef::Secret(s) => self.secrets.get(&s).and_then(|c| c.ttl_secs),
            _ => None,
        };
        let pending = Message {
            id: MsgId(-self.local_seq),
            peer,
            sender: self.me_id(),
            date: self::now(),
            edit_date: None,
            outgoing: true,
            text: String::new(),
            entities: vec![],
            reply_to,
            forwarded_from: None,
            media: None,
            sticker: Some(sticker),
            reactions: vec![],
            random_id: Some(random_id),
            state: SendState::Pending,
            ttl_secs: ttl,
        };
        let now = Instant::now();
        let key = MsgKey::Random(random_id);
        self.anim.start(AnimKey::BubbleOpacity(key), 0.0, 1.0, Curve::new(Spring::ESTANDAR, Property::Opacity, 180), now);
        self.anim.start(AnimKey::BubbleY(key), 8.0, 0.0, Curve::new(Spring::ESTANDAR, Property::Position, 180), now);
        if let Some(c) = self.conv.as_mut() {
            c.push_pending(pending);
            c.from_bottom = 0.0;
            c.new_below = 0;
        }
        if reply_to.is_some() {
            self.compose = Compose::Normal;
        }
        let send = self.sticker_send_task(peer, to_send, reply_to, random_id);
        Task::batch([send, Self::scroll_bottom(), operation::focus(Id::new(COMPOSER_ID))])
    }

    fn retry(&mut self, rid: RandomId) -> Task<Msg> {
        if let Some(t) = self.retry_gif(rid) {
            return t;
        }
        let Some(c) = self.conv.as_mut() else { return Task::none() };
        let Some(m) = c.messages.iter_mut().find(|m| m.random_id == Some(rid) && conv::is_local(m)) else {
            return Task::none();
        };
        m.state = SendState::Pending;
        if let Some(st) = m.sticker.clone() {
            let (peer, reply_to) = (m.peer, m.reply_to);
            return self.sticker_send_task(peer, st, reply_to, rid);
        }
        let (peer, out) = (m.peer, OutgoingText { text: m.text.clone(), entities: vec![], reply_to: m.reply_to, random_id: rid });
        self.call(
            move |b| async move { b.send_text(peer, out).await },
            move |result| Msg::Sent { peer, random_id: rid, result },
        )
    }

    fn escape(&mut self) -> Task<Msg> {
        if self.phase == Phase::Auth {
            return self.login_escape();
        }
        if self.chat_actions_escape() {
            return Task::none();
        }
        if self.overlay.take().is_some() {
            return Task::none();
        }
        if self.picker.open {
            self.picker.close();
            return operation::focus(Id::new(COMPOSER_ID));
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
        // Atajos de los menús contextuales (Supr, Shift+F10 / Menú, Enter en confirmaciones).
        if self.phase == Phase::Main
            && let Some(t) = self.chat_actions_key(key.as_ref(), modifiers)
        {
            return t;
        }
        match key.as_ref() {
            Key::Named(Named::Escape) => self.escape(),
            Key::Named(Named::ArrowLeft) if matches!(self.overlay, Some(Overlay::Photo { .. })) => self.update(Msg::PhotoStep(-1)),
            Key::Named(Named::ArrowRight) if matches!(self.overlay, Some(Overlay::Photo { .. })) => self.update(Msg::PhotoStep(1)),
            // Navegación con Tab entre campos y botones enfocables (login, diálogos).
            Key::Named(Named::Tab) if self.picker.open && self.panel.tab == crate::panel::PanelTab::Stickers && self.overlay.is_none() => {
                self.update(Msg::PickerNav(if modifiers.shift() { Nav::PrevTab } else { Nav::NextTab }))
            }
            Key::Named(Named::Tab) if modifiers.shift() => operation::focus_previous(),
            Key::Named(Named::Tab) => operation::focus_next(),
            _ if self.phase != Phase::Main => Task::none(),
            Key::Character("e") if modifiers.command() => self.update(Msg::TogglePicker),
            // Alt+1/2/3: pestañas Emojis | GIFs | Stickers del panel.
            Key::Character(d) if self.picker.open && modifiers.alt() && crate::panel::PanelTab::from_digit(d).is_some() => {
                let tab = crate::panel::PanelTab::from_digit(d).unwrap_or_default();
                self.update(Msg::Panel(crate::panel::PanelMsg::Tab(tab)))
            }
            Key::Named(n @ (Named::ArrowLeft | Named::ArrowRight | Named::ArrowUp | Named::ArrowDown | Named::Enter))
                if self.picker.open && self.panel.tab.grid_keys() && self.overlay.is_none() && !modifiers.alt() =>
            {
                let nav = match n {
                    Named::ArrowLeft => Nav::Left,
                    Named::ArrowRight => Nav::Right,
                    Named::ArrowUp => Nav::Up,
                    Named::ArrowDown => Nav::Down,
                    _ => Nav::Activate,
                };
                self.update(Msg::PickerNav(nav))
            }
            Key::Named(Named::Enter) if matches!(self.overlay, Some(Overlay::NewSecret { .. })) => self.update(Msg::ConfirmNewSecret),
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
