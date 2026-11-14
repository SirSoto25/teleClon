//! Notificaciones de escritorio de Parlo (ADR-0013).
//!
//! * [`Notifier`]: interfaz propia para mostrar y retirar avisos. Implementaciones: [`NullNotifier`]
//!   (no hace nada), [`RecordingNotifier`] (pruebas) y la del sistema que devuelve [`system`]:
//!   D-Bus `org.freedesktop.Notifications` en Linux/BSD, toasts WinRT en Windows y
//!   `NSUserNotification` en macOS.
//! * [`NotifyEvent`]: lo que vuelve del SO (clic en el aviso o en una acción) por un canal
//!   ([`NotifyEvents`], un `Stream` de `futures`) que la UI puede escuchar como suscripción.
//! * [`rules`]: reglas puras de cuándo avisar (foco, chat abierto, silenciado, propio, archivado,
//!   tipo, «No molestar», ráfagas) y qué texto enseñar (vista previa, chats secretos).
//!
//! Este crate no depende de `tc-core`: la clave de chat ([`ChatKey`]) es una cadena opaca que la
//! app forma a partir de su `PeerRef` y vuelve a interpretar al recibir un [`NotifyEvent`].

use std::path::PathBuf;

use futures::channel::mpsc;

pub mod rules;

mod icon;
mod null;
#[cfg(any(windows, test))]
mod toast_xml;

#[cfg(all(unix, not(target_os = "macos")))]
mod linux;
#[cfg(target_os = "macos")]
mod macos;
#[cfg(windows)]
mod windows;

pub use null::{NullNotifier, RecordingNotifier};

/// Clave opaca de un chat (o tema de foro). La app decide el formato (p. ej. `"u:42"`, `"s:7"`).
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ChatKey(pub String);

impl ChatKey {
    pub fn new(key: impl Into<String>) -> Self {
        Self(key.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl From<&str> for ChatKey {
    fn from(s: &str) -> Self {
        Self(s.to_owned())
    }
}

impl From<String> for ChatKey {
    fn from(s: String) -> Self {
        Self(s)
    }
}

impl std::fmt::Display for ChatKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

/// Imagen del aviso (foto del chat).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Icon {
    /// Archivo de imagen ya en disco (PNG o JPEG).
    Path(PathBuf),
    /// Bytes de una imagen codificada (PNG o JPEG). Se escribe a una caché temporal porque los tres
    /// SO piden una ruta.
    Bytes(Vec<u8>),
}

/// Botón del aviso. `id` vuelve en [`NotifyEvent::Action`]; `label` es el texto visible.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Action {
    pub id: String,
    pub label: String,
}

impl Action {
    pub fn new(id: impl Into<String>, label: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
        }
    }
}

/// Un aviso. Un aviso nuevo del mismo `chat_key` sustituye al anterior donde el SO lo permite
/// (agrupar por chat; ver [`Capabilities::replace`]).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Notification {
    pub chat_key: ChatKey,
    pub title: String,
    pub body: String,
    pub icon: Option<Icon>,
    pub actions: Vec<Action>,
    /// Sin sonido.
    pub silent: bool,
}

/// Lo que vuelve del SO.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NotifyEvent {
    /// Clic en el cuerpo del aviso: enfocar la ventana y abrir ese chat.
    Clicked(ChatKey),
    /// Clic en un botón: `(chat, Action::id)`.
    Action(ChatKey, String),
}

/// Qué admite el backend en este sistema.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Capabilities {
    /// Botones de acción.
    pub actions: bool,
    /// Un aviso nuevo del mismo chat reemplaza al anterior.
    pub replace: bool,
    /// [`Notifier::withdraw`] quita el aviso de la pantalla o del centro de notificaciones.
    pub withdraw: bool,
    /// Se recibe [`NotifyEvent::Clicked`].
    pub click: bool,
}

#[derive(Debug, thiserror::Error)]
pub enum NotifyError {
    /// No hay servicio de notificaciones (sin sesión D-Bus, sin AppUserModelID…).
    #[error("notificaciones no disponibles: {0}")]
    Unavailable(String),
    /// El servicio rechazó o no pudo mostrar el aviso.
    #[error("error del sistema de notificaciones: {0}")]
    Backend(String),
}

pub type Result<T, E = NotifyError> = std::result::Result<T, E>;

/// Muestra y retira avisos. Las implementaciones son baratas de llamar (una llamada IPC como
/// mucho) y seguras entre hilos; los eventos de vuelta llegan por el [`NotifyEvents`] que entrega
/// su constructor.
pub trait Notifier: Send + Sync {
    /// Muestra `n` (o reemplaza el aviso vigente de ese chat).
    fn notify(&self, n: Notification) -> Result<()>;
    /// Retira el aviso de `chat` si sigue visible (p. ej. al leer el chat). Sin aviso: `Ok`.
    fn withdraw(&self, chat: &ChatKey) -> Result<()>;
    fn capabilities(&self) -> Capabilities;
}

/// Receptor de eventos de vuelta (`Stream<Item = NotifyEvent>`).
pub type NotifyEvents = mpsc::UnboundedReceiver<NotifyEvent>;

/// Extremo emisor del canal de eventos; lo usan los backends (y [`RecordingNotifier`]).
#[derive(Debug, Clone)]
pub struct EventSender(mpsc::UnboundedSender<NotifyEvent>);

impl EventSender {
    /// Envía un evento; si la UI ya soltó el receptor, se descarta en silencio.
    pub fn send(&self, ev: NotifyEvent) {
        let _ = self.0.unbounded_send(ev);
    }
}

/// Canal nuevo de eventos.
pub fn event_channel() -> (EventSender, NotifyEvents) {
    let (tx, rx) = mpsc::unbounded();
    (EventSender(tx), rx)
}

/// Identidad de la app ante el SO.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AppInfo {
    /// Nombre visible («Parlo»).
    pub name: String,
    /// Windows: AppUserModelID (el mismo que el acceso directo del instalador). macOS: bundle id.
    pub app_id: String,
    /// Linux: nombre del `.desktop` sin extensión (pista `desktop-entry`, icono y agrupación).
    pub desktop_entry: String,
}

impl AppInfo {
    /// Identidad de Parlo: `app.parlo.Parlo` (AUMID y bundle id, provisional según ADR-0005) y
    /// `parlo.desktop`.
    pub fn parlo() -> Self {
        Self {
            name: "Parlo".into(),
            app_id: "app.parlo.Parlo".into(),
            desktop_entry: "parlo".into(),
        }
    }
}

impl Default for AppInfo {
    fn default() -> Self {
        Self::parlo()
    }
}

/// El notificador del sistema y su canal de eventos. Si el SO no ofrece servicio (Linux sin sesión
/// D-Bus, por ejemplo) devuelve un [`NullNotifier`] y lo registra con `tracing::warn!`: la app
/// sigue funcionando sin avisos.
pub fn system(app: &AppInfo) -> (Box<dyn Notifier>, NotifyEvents) {
    let (tx, rx) = event_channel();
    match platform(app, tx) {
        Ok(n) => (n, rx),
        Err(e) => {
            tracing::warn!("sin notificaciones de escritorio: {e}");
            (Box::new(NullNotifier), rx)
        }
    }
}

#[cfg(all(unix, not(target_os = "macos")))]
fn platform(app: &AppInfo, tx: EventSender) -> Result<Box<dyn Notifier>> {
    Ok(Box::new(linux::DbusNotifier::connect(app, tx)?))
}

#[cfg(windows)]
fn platform(app: &AppInfo, tx: EventSender) -> Result<Box<dyn Notifier>> {
    Ok(Box::new(windows::ToastNotifier::new(app, tx)?))
}

#[cfg(target_os = "macos")]
fn platform(app: &AppInfo, tx: EventSender) -> Result<Box<dyn Notifier>> {
    Ok(Box::new(macos::MacNotifier::new(app, tx)?))
}

#[cfg(not(any(unix, windows)))]
fn platform(_app: &AppInfo, _tx: EventSender) -> Result<Box<dyn Notifier>> {
    Err(NotifyError::Unavailable("SO sin backend".into()))
}
