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

#[derive(Default)]
pub struct Downloads {
    items: HashMap<(PeerRef, MsgId), Dl>,
    /// Carpeta de descargas elegida (`None`: la del sistema).
    pub dir: Option<PathBuf>,
    /// El selector de «Guardar como…» está abierto.
    picking: bool,
    /// Pruebas: lo último que se habría abierto (en pruebas no se lanza ningún programa).
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

    /// Carpeta donde caen las descargas directas.
    pub fn target_dir(&self) -> Option<PathBuf> {
        self.dir.clone().or_else(default_dir)
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
}

/// Carpeta de descargas del sistema, o la personal si el sistema no la define.
pub fn default_dir() -> Option<PathBuf> {
    dirs::download_dir().or_else(|| dirs::home_dir().map(|h| h.join("Downloads")))
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

impl App {
    /// ¿Tiene este mensaje un medio que se pueda guardar? (no los envíos locales sin confirmar).
    pub fn can_save(&self, m: &Message) -> bool {
        !crate::conv::is_local(m) && m.media.as_ref().is_some_and(|md| md.file.is_some())
    }

    pub(crate) fn on_dl(&mut self, m: DlMsg) -> Task<Msg> {
        self.chat_actions.menu = None;
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
        }
    }

    fn dl_media(&self, peer: PeerRef, id: MsgId) -> Option<&Media> {
        let m = self.conv.as_ref().filter(|c| c.peer == peer)?.find(id)?;
        self.can_save(m).then_some(())?;
        m.media.as_ref()
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
        let Some(md) = self.dl_media(peer, id) else {
            self.toast("Este archivo ya no está disponible.".into(), true);
            return Task::none();
        };
        let Some(dir) = self.dl.target_dir() else {
            self.toast("No se encontró la carpeta de descargas. Usa «Guardar como…».".into(), true);
            return Task::none();
        };
        let dest = dest_in(&dir, md);
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
        let (name, dir) = (download_name(md), self.dl.target_dir());
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
                if let Some(dir) = dest.parent().filter(|d| !d.as_os_str().is_empty()) {
                    std::fs::create_dir_all(dir).map_err(|e| BackendError::Storage(e.to_string()))?;
                }
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
