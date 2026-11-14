//! Descarga de archivos a disco (fase A3, ADR-0011): clic en el documento = descargar a la carpeta de
//! descargas sin sobrescribir (`unique_path`), con progreso y cancelar en la tarjeta; al terminar,
//! «Abrir» y «Mostrar en carpeta»; «Guardar como…» con el selector nativo; fallo → «Reintentar».
//!
//! Cada descarga se identifica por (chat, mensaje): `BackendEvent::TransferProgress` trae ambos
//! (`Transfer::peer`/`msg`), así los eventos que llegan **antes** de que `download_media_to` devuelva
//! el id (ADR-0011 §5) también encuentran su tarjeta. Se recuerda durante la sesión qué mensajes ya
//! se bajaron y dónde: vuelven a «Abrir» al reabrir el chat (si el archivo sigue ahí).
//!
//! Nada pasa por un intérprete de órdenes: «Abrir» lanza `xdg-open`/`open`/`explorer` con la ruta
//! como un único argumento.

use std::collections::HashMap;
use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use iced::Task;
use tc_core::*;

use super::{App, Msg, error_text};

/// En qué punto está la descarga de un mensaje.
#[derive(Debug, Clone, PartialEq)]
pub enum DlState {
    /// Pedida; aún no se conoce el id ni ha llegado progreso.
    Starting,
    Running { done: u64, total: u64 },
    /// Cancelación pedida (se confirma con `TransferState::Cancelled`).
    Cancelling,
    /// El archivo está en `Dl::path`.
    Done,
    /// Texto del error para la tarjeta.
    Failed(String),
}

/// La descarga de un mensaje (en curso, terminada o fallida). Cancelada = no existe.
#[derive(Debug, Clone)]
pub struct Dl {
    pub id: Option<TransferId>,
    /// Destino final (ya único: nada se sobrescribe sin que el usuario lo elija en «Guardar como…»).
    pub path: PathBuf,
    pub state: DlState,
    /// Se pidió cancelar antes de conocer el id: se cancela en cuanto llegue.
    cancel_early: bool,
    /// Avisar con un aviso al terminar (fotos y vídeos no tienen tarjeta donde verlo).
    pub notify: bool,
}

impl Dl {
    pub fn fraction(&self) -> Option<f32> {
        match self.state {
            DlState::Running { done, total } if total > 0 => Some((done.min(total) as f64 / total as f64) as f32),
            _ => None,
        }
    }

    pub fn is_active(&self) -> bool {
        matches!(self.state, DlState::Starting | DlState::Running { .. } | DlState::Cancelling)
    }
}

/// Ajustes del usuario que sobreviven al cierre (los aporta `tc-app` sobre la tabla de ajustes de
/// `tc-store`; sin ellos el ajuste solo dura la sesión). Nunca se guardan secretos.
pub trait Prefs: Send + Sync {
    fn get(&self, key: &str) -> Option<String>;
    /// `None` borra el ajuste.
    fn set(&self, key: &str, value: Option<&str>);
}

/// Clave del ajuste «carpeta de descargas».
pub const DIR_KEY: &str = "ui.download_dir";

#[derive(Default)]
pub struct Downloads {
    items: HashMap<(PeerRef, MsgId), Dl>,
    /// Carpeta de descargas elegida por el usuario (`None`: la predeterminada).
    pub dir: Option<PathBuf>,
    /// Persistencia del ajuste (si la hay).
    pub prefs: Option<Arc<dyn Prefs>>,
    /// El selector de «Guardar como…» está abierto.
    picking: bool,
    /// Pruebas: lo último que se habría abierto (en pruebas no se lanza ningún programa).
    /// Pruebas: carpeta predeterminada en lugar de la del sistema.
    #[cfg(test)]
    pub default_override: Option<PathBuf>,
    #[cfg(test)]
    pub last_open: Option<(PathBuf, bool)>,
}

impl Downloads {
    pub fn get(&self, peer: PeerRef, msg: MsgId) -> Option<&Dl> {
        self.items.get(&(peer, msg))
    }

    /// Capturas: fija el estado de una descarga sin pasar por el backend.
    #[cfg(test)]
    pub fn force(&mut self, peer: PeerRef, msg: MsgId, path: PathBuf, state: DlState) {
        self.items.insert((peer, msg), Dl { id: None, path, state, cancel_early: false, notify: false });
    }

    /// Carpeta donde caen las descargas directas: la elegida si sigue siendo válida; si no, la
    /// predeterminada (y `invalid` lleva la elegida que ya no sirve, para avisar y olvidarla).
    pub fn resolve(&self, default: Option<PathBuf>) -> Resolved {
        match &self.dir {
            Some(d) if is_writable_dir(d) => Resolved { dir: Some(d.clone()), invalid: None },
            Some(d) => Resolved { dir: default, invalid: Some(d.clone()) },
            None => Resolved { dir: default, invalid: None },
        }
    }

    /// ¿Hay una carpeta elegida (distinta de la predeterminada)?
    pub fn is_custom(&self) -> bool {
        self.dir.is_some()
    }

    /// Guarda (o borra) la carpeta elegida, también en los ajustes persistentes.
    pub fn set_dir(&mut self, dir: Option<PathBuf>) {
        if let Some(p) = &self.prefs {
            p.set(DIR_KEY, dir.as_deref().and_then(Path::to_str));
        }
        self.dir = dir;
    }

    /// Lee el ajuste guardado (al arrancar). No se valida aquí: la carpeta puede no estar montada
    /// todavía; se comprueba al usarla.
    pub fn load(&mut self) {
        self.dir = self.prefs.as_ref().and_then(|p| p.get(DIR_KEY)).filter(|v| !v.trim().is_empty()).map(PathBuf::from);
    }
}

/// Resultado de decidir dónde caen las descargas.
#[derive(Debug, Clone, PartialEq)]
pub struct Resolved {
    pub dir: Option<PathBuf>,
    /// Carpeta elegida que ya no existe o no se puede escribir.
    pub invalid: Option<PathBuf>,
}

/// ¿Existe, es una carpeta y se puede escribir en ella? (se prueba creando y borrando un archivo).
pub fn is_writable_dir(dir: &Path) -> bool {
    if !dir.is_dir() {
        return false;
    }
    let probe = dir.join(format!(".parlo-prueba-{}", std::process::id()));
    match std::fs::OpenOptions::new().write(true).create_new(true).open(&probe) {
        Ok(_) => {
            let _ = std::fs::remove_file(&probe);
            true
        }
        Err(_) => false,
    }
}

#[derive(Debug, Clone)]
pub enum DlMsg {
    /// Clic en la tarjeta o «Guardar»: descargar a la carpeta de descargas; si ya está, abrirla.
    Start(MsgId),
    /// «Guardar como…»: selector nativo de destino.
    SaveAs(MsgId),
    /// Destino elegido (`None`: se cerró el selector).
    SavePicked { peer: PeerRef, msg: MsgId, dest: Option<PathBuf> },
    /// Respuesta de `download_media_to` (los eventos de progreso llegan aparte).
    Started { peer: PeerRef, msg: MsgId, result: BackendResult<TransferId> },
    Cancel(MsgId),
    /// Abrir el archivo descargado con la aplicación del sistema.
    Open(MsgId),
    /// Mostrar el archivo en su carpeta.
    Reveal(MsgId),
    /// Abrir el diálogo de la carpeta de descargas (raíl).
    Settings,
    /// «Elegir carpeta…»: selector nativo de carpetas.
    ChooseDir,
    /// Carpeta elegida (`None`: se cerró el selector).
    DirPicked(Option<PathBuf>),
    /// «Restablecer»: vuelve a la carpeta predeterminada.
    ResetDir,
}

/// Carpeta predeterminada del sistema: `<Descargas>/Telegram Desktop` si ya existe (la de Telegram
/// Desktop); si no, `<Descargas>/Telegram` (se crea al primer uso, no al arrancar). Sin carpeta
/// Descargas, la personal.
pub fn default_dir() -> Option<PathBuf> {
    default_dir_in(dirs::download_dir(), dirs::home_dir())
}

/// `default_dir` con las carpetas del sistema como parámetros (pruebas).
pub fn default_dir_in(downloads: Option<PathBuf>, home: Option<PathBuf>) -> Option<PathBuf> {
    match downloads.filter(|d| d.is_dir()) {
        Some(d) => {
            let desktop = d.join("Telegram Desktop");
            Some(if desktop.is_dir() { desktop } else { d.join("Telegram") })
        }
        None => home,
    }
}

/// Nombre con el que se guarda un medio: el suyo (saneado: nunca trae rutas ni `..`) o uno por tipo.
pub fn download_name(md: &Media) -> String {
    let fallback = match md.kind {
        MediaKind::Photo => "foto.jpg",
        MediaKind::Video | MediaKind::Gif | MediaKind::VideoNote => "video.mp4",
        MediaKind::Voice => "nota-de-voz.ogg",
        MediaKind::Audio | MediaKind::File => "archivo",
    };
    let raw = md.file_name.as_deref().map(str::trim).filter(|n| !n.is_empty()).unwrap_or(fallback);
    sanitize_file_name(raw)
}

/// Destino por defecto: nombre saneado y sin chocar con lo que ya hay en `dir` (sufijo « (1)»).
pub fn dest_in(dir: &Path, md: &Media) -> PathBuf {
    unique_path(dir, &download_name(md))
}

/// Orden que abre `path` (o su carpeta, con `reveal`) en este sistema: programa y argumentos.
pub fn open_command(path: &Path, reveal: bool) -> (&'static str, Vec<OsString>) {
    let p = path.as_os_str().to_os_string();
    let dir = || path.parent().map_or_else(|| p.clone(), |d| d.as_os_str().to_os_string());
    if cfg!(target_os = "macos") {
        ("open", if reveal { vec!["-R".into(), p] } else { vec![p] })
    } else if cfg!(target_os = "windows") {
        // `explorer` abre cualquier archivo con su aplicación y, con `/select,`, lo señala en su carpeta.
        let mut sel = OsString::from("/select,");
        sel.push(&p);
        ("explorer", if reveal { vec![sel] } else { vec![p] })
    } else {
        // Sin API estándar para señalar un archivo: se abre su carpeta.
        ("xdg-open", vec![if reveal { dir() } else { p }])
    }
}

/// Lanza `open_command` sin esperar (un hilo recoge el proceso hijo para no dejar zombis).
fn launch(path: &Path, reveal: bool) -> std::io::Result<()> {
    let (prog, args) = open_command(path, reveal);
    let mut child = std::process::Command::new(prog)
        .args(args)
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()?;
    std::thread::spawn(move || {
        let _ = child.wait();
    });
    Ok(())
}

/// Selector nativo de destino (`None` si se cancela).
pub async fn pick_destination(name: String, dir: Option<PathBuf>) -> Option<PathBuf> {
    let mut d = rfd::AsyncFileDialog::new().set_title("Guardar como").set_file_name(name);
    if let Some(dir) = dir.filter(|d| d.is_dir()) {
        d = d.set_directory(dir);
    }
    d.save_file().await.map(|f| f.path().to_path_buf())
}

/// Crea la carpeta del destino si falta (la primera descarga a `<Descargas>/Telegram`).
pub fn ensure_parent(dest: &Path) -> BackendResult<()> {
    match dest.parent().filter(|d| !d.as_os_str().is_empty()) {
        Some(dir) => std::fs::create_dir_all(dir).map_err(|e| BackendError::Storage(e.to_string())),
        None => Ok(()),
    }
}

/// Selector nativo de carpeta (`None` si se cancela).
pub async fn pick_folder(start: Option<PathBuf>) -> Option<PathBuf> {
    let mut d = rfd::AsyncFileDialog::new().set_title("Carpeta de descargas");
    if let Some(dir) = start.filter(|d| d.is_dir()) {
        d = d.set_directory(dir);
    }
    d.pick_folder().await.map(|f| f.path().to_path_buf())
}

impl App {
    /// ¿Tiene este mensaje un medio que se pueda guardar? (no los envíos locales sin confirmar).
    pub fn can_save(&self, m: &Message) -> bool {
        !crate::conv::is_local(m) && m.media.as_ref().is_some_and(|md| md.file.is_some())
    }

    pub(crate) fn on_dl(&mut self, m: DlMsg) -> Task<Msg> {
        self.chat_actions.menu = None;
        // El ajuste de la carpeta no depende de que haya un chat abierto.
        match m {
            DlMsg::Settings => return self.dl_settings(),
            DlMsg::ChooseDir => return self.dl_choose_dir(),
            DlMsg::DirPicked(dir) => return self.dl_dir_picked(dir),
            DlMsg::ResetDir => {
                self.dl.set_dir(None);
                return Task::none();
            }
            _ => {}
        }
        let Some(peer) = self.conv.as_ref().map(|c| c.peer) else { return Task::none() };
        match m {
            DlMsg::Start(id) => self.dl_start(peer, id),
            DlMsg::SaveAs(id) => self.dl_save_as(peer, id),
            DlMsg::SavePicked { peer, msg, dest } => match dest {
                Some(dest) => self.dl_begin(peer, msg, dest),
                None => {
                    self.dl.picking = false;
                    Task::none()
                }
            },
            DlMsg::Started { peer, msg, result } => self.dl_started(peer, msg, result),
            DlMsg::Cancel(id) => self.dl_cancel(peer, id),
            DlMsg::Open(id) => self.dl_open(peer, id, false),
            DlMsg::Reveal(id) => self.dl_open(peer, id, true),
            DlMsg::Settings | DlMsg::ChooseDir | DlMsg::DirPicked(_) | DlMsg::ResetDir => Task::none(),
        }
    }

    fn dl_settings(&mut self) -> Task<Msg> {
        self.overlay = Some(super::Overlay::DownloadDir);
        self.open_overlay_anim();
        Task::none()
    }

    fn dl_choose_dir(&mut self) -> Task<Msg> {
        if self.dl.picking {
            return Task::none();
        }
        self.dl.picking = true;
        let start = self.dl.dir.clone().or_else(default_dir);
        Task::perform(pick_folder(start), |d| Msg::Dl(DlMsg::DirPicked(d)))
    }

    fn dl_dir_picked(&mut self, dir: Option<PathBuf>) -> Task<Msg> {
        self.dl.picking = false;
        let Some(dir) = dir else { return Task::none() };
        if is_writable_dir(&dir) {
            self.dl.set_dir(Some(dir));
        } else {
            self.toast("No se puede guardar en esa carpeta. Elige otra.".into(), true);
        }
        Task::none()
    }

    fn dl_media(&self, peer: PeerRef, id: MsgId) -> Option<&Media> {
        let m = self.conv.as_ref().filter(|c| c.peer == peer)?.find(id)?;
        self.can_save(m).then_some(())?;
        m.media.as_ref()
    }

    /// Carpeta de las descargas directas. Si la elegida ya no existe o no se puede escribir, avisa,
    /// la olvida (vuelve a la predeterminada) y sigue con esa.
    pub(crate) fn dl_target_dir(&mut self) -> Option<PathBuf> {
        #[cfg(test)]
        let default = self.dl.default_override.clone().or_else(default_dir);
        #[cfg(not(test))]
        let default = default_dir();
        let r = self.dl.resolve(default);
        if let Some(bad) = r.invalid {
            self.dl.set_dir(None);
            let name = bad.display();
            self.toast(format!("No se puede usar la carpeta «{name}». Se usará la predeterminada."), true);
        }
        r.dir
    }

    /// Clic: abrir si ya está; si no, descargar a la carpeta de descargas.
    fn dl_start(&mut self, peer: PeerRef, id: MsgId) -> Task<Msg> {
        if let Some(d) = self.dl.items.get(&(peer, id)) {
            match d.state {
                DlState::Starting | DlState::Running { .. } | DlState::Cancelling => return Task::none(),
                DlState::Done if d.path.exists() => return self.dl_open(peer, id, false),
                // Se borró o se movió: se baja otra vez. Un fallo: «Reintentar».
                DlState::Done | DlState::Failed(_) => {}
            }
        }
        let Some(md) = self.dl_media(peer, id).cloned() else {
            self.toast("Este archivo ya no está disponible.".into(), true);
            return Task::none();
        };
        let Some(dir) = self.dl_target_dir() else {
            self.toast("No se encontró la carpeta de descargas. Usa «Guardar como…».".into(), true);
            return Task::none();
        };
        let dest = dest_in(&dir, &md);
        let notify = md.kind != MediaKind::File;
        self.dl_begin_with(peer, id, dest, notify)
    }

    fn dl_save_as(&mut self, peer: PeerRef, id: MsgId) -> Task<Msg> {
        if self.dl.picking || self.dl.items.get(&(peer, id)).is_some_and(Dl::is_active) {
            return Task::none();
        }
        let Some(md) = self.dl_media(peer, id) else {
            self.toast("Este archivo ya no está disponible.".into(), true);
            return Task::none();
        };
        let name = download_name(md);
        let dir = self.dl_target_dir();
        self.dl.picking = true;
        Task::perform(pick_destination(name, dir), move |dest| Msg::Dl(DlMsg::SavePicked { peer, msg: id, dest }))
    }

    /// Descarga a un destino elegido por el usuario.
    fn dl_begin(&mut self, peer: PeerRef, id: MsgId, dest: PathBuf) -> Task<Msg> {
        self.dl.picking = false;
        let notify = self.dl_media(peer, id).is_some_and(|md| md.kind != MediaKind::File);
        self.dl_begin_with(peer, id, dest, notify)
    }

    fn dl_begin_with(&mut self, peer: PeerRef, id: MsgId, dest: PathBuf, notify: bool) -> Task<Msg> {
        let Some(file) = self.dl_media(peer, id).and_then(|md| md.file.clone()) else {
            self.toast("Este archivo ya no está disponible.".into(), true);
            return Task::none();
        };
        self.dl.items.insert((peer, id), Dl { id: None, path: dest.clone(), state: DlState::Starting, cancel_early: false, notify });
        self.call(
            move |b| async move {
                ensure_parent(&dest)?;
                b.download_media_to(peer, id, file, MediaVariant::Full, dest).await
            },
            move |result| Msg::Dl(DlMsg::Started { peer, msg: id, result }),
        )
    }

    fn dl_started(&mut self, peer: PeerRef, id: MsgId, result: BackendResult<TransferId>) -> Task<Msg> {
        let Some(d) = self.dl.items.get_mut(&(peer, id)) else { return Task::none() };
        match result {
            Ok(tid) => {
                d.id.get_or_insert(tid);
                if d.cancel_early && d.is_active() {
                    return self.call(move |b| async move { b.cancel_transfer(tid).await }, Msg::Op);
                }
            }
            Err(BackendError::Cancelled) => {
                self.dl.items.remove(&(peer, id));
            }
            Err(e) => {
                let text = download_error_text(&e);
                let notify = d.notify;
                d.state = DlState::Failed(text.clone());
                if notify {
                    self.toast(text, true);
                }
            }
        }
        Task::none()
    }

    fn dl_cancel(&mut self, peer: PeerRef, id: MsgId) -> Task<Msg> {
        let Some(d) = self.dl.items.get_mut(&(peer, id)).filter(|d| matches!(d.state, DlState::Starting | DlState::Running { .. })) else { return Task::none() };
        d.state = DlState::Cancelling;
        match d.id {
            Some(tid) => self.call(move |b| async move { b.cancel_transfer(tid).await }, Msg::Op),
            None => {
                d.cancel_early = true;
                Task::none()
            }
        }
    }

    fn dl_open(&mut self, peer: PeerRef, id: MsgId, reveal: bool) -> Task<Msg> {
        let Some(path) = self.dl.items.get(&(peer, id)).filter(|d| d.state == DlState::Done).map(|d| d.path.clone()) else { return Task::none() };
        if !path.exists() {
            // Lo borraron o lo movieron: la tarjeta vuelve a ofrecer la descarga.
            self.dl.items.remove(&(peer, id));
            self.toast("El archivo ya no está en esa carpeta.".into(), true);
            return Task::none();
        }
        if cfg!(test) {
            // En pruebas no se lanza ningún programa.
            #[cfg(test)]
            {
                self.dl.last_open = Some((path, reveal));
            }
        } else if launch(&path, reveal).is_err() {
            self.toast("No se pudo abrir el archivo. Búscalo en la carpeta de descargas.".into(), true);
        }
        Task::none()
    }

    /// Progreso de una descarga a disco (`BackendEvent::TransferProgress` con `TransferId::Download`).
    pub(crate) fn on_download(&mut self, t: Transfer) -> Task<Msg> {
        let Some(msg) = t.msg else { return Task::none() };
        let key = (t.peer, msg);
        let Some(d) = self.dl.items.get_mut(&key) else { return Task::none() };
        // Un evento rezagado de un intento anterior no pisa al actual.
        if d.id.is_some_and(|i| i != t.id) {
            return Task::none();
        }
        d.id = Some(t.id);
        match t.state {
            TransferState::Running => {
                if d.state != DlState::Cancelling {
                    d.state = DlState::Running { done: t.done, total: t.total };
                }
            }
            TransferState::Done => {
                d.state = DlState::Done;
                if d.notify {
                    let place = d.path.parent().map_or_else(String::new, |p| p.display().to_string());
                    self.toast(format!("Guardado en {place}"), false);
                }
            }
            TransferState::Failed(e) => {
                let text = download_error_text(&e);
                d.state = DlState::Failed(text.clone());
                if d.notify {
                    self.toast(text, true);
                }
            }
            // Cancelar deja la tarjeta como estaba antes de pulsar.
            TransferState::Cancelled => {
                self.dl.items.remove(&key);
            }
        }
        Task::none()
    }
}

/// Texto del error de una descarga (los de disco, con más detalle que el genérico).
pub fn download_error_text(e: &BackendError) -> String {
    match e {
        BackendError::Storage(_) => "No se pudo guardar: comprueba el espacio y los permisos de la carpeta.".into(),
        BackendError::InvalidInput("dest") => "La carpeta de destino no existe.".into(),
        BackendError::NotFound => "El archivo ya no está en Telegram.".into(),
        e => error_text(e),
    }
}
