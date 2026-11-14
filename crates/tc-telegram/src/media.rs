//! Envío de fotos/archivos: subida por partes (`upload.saveFilePart` / `saveBigFilePart`)
//! y `messages.sendMedia`, con progreso y cancelación (ADR-0011). Sin reanudación entre sesiones
//! todavía (la tabla `uploads` de tc-store la hará).
//!
//! Descarga (`upload.getFile` por partes de 512 KiB) de fotos de perfil
//! (`inputPeerPhotoFileLocation`) y de medios (`inputPhotoFileLocation` /
//! `inputDocumentFileLocation`) en el DC del archivo, a memoria (`download_media`) o a disco
//! (`download_media_to`: `<dest>.part` + renombrado atómico):
//! - DC distinto del de casa sin autorizar (`AUTH_KEY_UNREGISTERED`): se copia la autorización
//!   (`auth.exportAuthorization` en casa + `auth.importAuthorization` allí) y se reintenta.
//! - `FILE_MIGRATE_X`: se sigue al DC indicado.
//! - `FILE_REFERENCE_*` (referencia caducada): se vuelve a pedir el mensaje para obtener una
//!   referencia fresca y se reintenta (a disco, desde la parte en que iba).
//! - `FLOOD_WAIT` corto se duerme en `Net::invoke_in`; uno largo se devuelve como `FloodWait`.
//! - Cortes de red: `Net::invoke_in` reintenta con espera exponencial; las transferencias a disco y
//!   las subidas, además, reanudan la misma parte hasta `MAX_RESUMES` veces.
//! - Como mucho `MAX_DOWNLOADS` descargas a la vez (semáforo).
//!
//! Transferencias (ADR-0011): `BackendEvent::TransferProgress` con `Running` (el primero con 0 al
//! momento; después como mucho uno cada `PROGRESS_INTERVAL`, ≤ 10 Hz) y un único estado terminal.
//! `cancel_transfer` corta la petición en vuelo (las partes ya subidas se pierden; los mensajes ya
//! confirmados con `MessageSent` quedan enviados).
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex, MutexGuard};
use std::time::Duration;

use async_trait::async_trait;
use grammers_client::InvocationError;
use grammers_session::Session;
use grammers_tl_types as tl;
use tc_core::*;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::sync::watch;
use tokio::time::Instant;

use crate::GrammersBackend;
use crate::backend::{messages_of, random_i64};
use crate::ids::channel_of;
use crate::net::map_error;

/// Partes de 512 KiB (máximo permitido por la API).
pub(crate) const PART_SIZE: usize = 512 * 1024;
/// Por encima de 10 MiB hay que usar `saveBigFilePart`.
pub(crate) const BIG_FILE: u64 = 10 * 1024 * 1024;
/// Partes como mucho de un archivo en una cuenta normal (Premium admite el doble, pero no se asume:
/// la cuenta puede no serlo y el servidor lo rechazaría tras subirlo todo).
pub(crate) const MAX_PARTS: u64 = 4000;
/// Límite de tamaño de archivo para cuentas normales: 4000 partes de 512 KiB = 2000 MiB (el «2 GB»
/// de Telegram, algo menos de 2 GiB).
pub(crate) const MAX_FILE: u64 = MAX_PARTS * PART_SIZE as u64;

/// Descargas simultáneas como mucho.
pub(crate) const MAX_DOWNLOADS: usize = 4;
/// Tope de lo que se descarga a memoria (los documentos mayores se rechazan).
pub(crate) const MAX_IN_MEMORY: u64 = 128 * 1024 * 1024;
/// Separación mínima entre dos eventos `Running` de una transferencia (≤ 10 por segundo).
pub(crate) const PROGRESS_INTERVAL: Duration = Duration::from_millis(100);
/// Reanudaciones de una misma parte tras agotar los reintentos de red de `Net::invoke_in`.
pub(crate) const MAX_RESUMES: u32 = 3;

pub(crate) fn parts_for(size: u64) -> i32 {
    size.div_ceil(PART_SIZE as u64).max(1) as i32
}

pub(crate) fn mime_for(name: &str) -> &'static str {
    let ext = name.rsplit('.').next().unwrap_or("").to_ascii_lowercase();
    match ext.as_str() {
        "jpg" | "jpeg" => "image/jpeg",
        "png" => "image/png",
        "webp" => "image/webp",
        "gif" => "image/gif",
        "mp4" => "video/mp4",
        "webm" => "video/webm",
        "mov" => "video/quicktime",
        "ogg" | "oga" => "audio/ogg",
        "mp3" => "audio/mpeg",
        "pdf" => "application/pdf",
        "txt" => "text/plain",
        "zip" => "application/zip",
        _ => "application/octet-stream",
    }
}

fn is_file_reference_error(e: &InvocationError) -> bool {
    matches!(e, InvocationError::Rpc(r) if r.code == 400 && r.name.starts_with("FILE_REFERENCE_"))
}

/// Ubicación TL de una variante de un medio. `None` si esa variante no existe.
pub(crate) fn media_location(f: &FileRef, v: MediaVariant) -> Option<tl::enums::InputFileLocation> {
    Some(match f.kind {
        FileKind::Photo => {
            let ty = match v {
                MediaVariant::Thumb => f.thumb.clone().or_else(|| f.full.clone()),
                MediaVariant::Full => f.full.clone().or_else(|| f.thumb.clone()),
            }?;
            tl::types::InputPhotoFileLocation { id: f.id, access_hash: f.access_hash, file_reference: f.file_reference.clone(), thumb_size: ty }.into()
        }
        FileKind::Document => {
            let ty = match v {
                MediaVariant::Thumb => f.thumb.clone()?,
                MediaVariant::Full => String::new(),
            };
            tl::types::InputDocumentFileLocation { id: f.id, access_hash: f.access_hash, file_reference: f.file_reference.clone(), thumb_size: ty }.into()
        }
    })
}

/// Aviso claro de archivo demasiado grande (la UI muestra el texto de `Forbidden` tal cual).
fn too_large(name: &str, size: u64) -> BackendError {
    let gib = size as f64 / (1u64 << 30) as f64;
    BackendError::Forbidden(format!(
        "el archivo «{name}» ocupa {gib:.2} GiB y Telegram admite como máximo 2000 MiB por archivo (4000 MiB con Premium)"
    ))
}

fn storage(e: std::io::Error) -> BackendError {
    BackendError::Storage(e.to_string())
}

/// El directorio de `dest` debe existir y `dest` ha de ser un nombre de archivo (no un directorio,
/// ni `..`).
fn check_dest(dest: &Path) -> BackendResult<()> {
    let parent = match dest.parent() {
        Some(p) if p.as_os_str().is_empty() => Path::new("."),
        Some(p) => p,
        None => return Err(BackendError::InvalidInput("dest")),
    };
    if dest.file_name().is_none() || !parent.is_dir() || dest.is_dir() {
        return Err(BackendError::InvalidInput("dest"));
    }
    Ok(())
}

// ------------------------------------------------------------------ transferencias

/// Transferencias en curso con su señal de cancelación.
#[derive(Default)]
pub(crate) struct Transfers {
    next: AtomicU64,
    active: Mutex<HashMap<TransferId, watch::Sender<bool>>>,
}

impl Transfers {
    fn active(&self) -> MutexGuard<'_, HashMap<TransferId, watch::Sender<bool>>> {
        self.active.lock().unwrap_or_else(|e| e.into_inner())
    }

    fn next_download(&self) -> TransferId {
        TransferId::Download(self.next.fetch_add(1, Ordering::Relaxed) + 1)
    }

    /// Da de alta `id` (sustituye a otra con el mismo id, que ya no se podrá cancelar).
    fn register(&self, id: TransferId) -> Cancel {
        let (tx, rx) = watch::channel(false);
        self.active().insert(id, tx);
        Cancel { id, rx }
    }

    /// Da de baja la transferencia de `c` si sigue registrada con ese id.
    fn unregister(&self, c: &Cancel) {
        let mut active = self.active();
        if active.get(&c.id).is_some_and(|tx| tx.subscribe().same_channel(&c.rx)) {
            active.remove(&c.id);
        }
    }

    /// Pide cancelar `id`; sin efecto si ya terminó o no se conoce.
    pub(crate) fn cancel(&self, id: TransferId) {
        if let Some(tx) = self.active().get(&id) {
            tx.send_replace(true);
        }
    }
}

/// Señal de cancelación de una transferencia.
struct Cancel {
    id: TransferId,
    rx: watch::Receiver<bool>,
}

impl Cancel {
    fn requested(&self) -> bool {
        *self.rx.borrow()
    }

    /// Ejecuta `f` salvo que se cancele antes de que termine (entonces se suelta y da `Cancelled`).
    async fn race<T>(&mut self, f: impl std::future::Future<Output = T>) -> BackendResult<T> {
        if self.requested() {
            return Err(BackendError::Cancelled);
        }
        let mut rx = self.rx.clone();
        tokio::pin!(f);
        tokio::select! {
            biased;
            // Si se sustituyó el registro (`Err`), esta ya no se puede cancelar: se deja terminar.
            Ok(_) = rx.wait_for(|c| *c) => Err(BackendError::Cancelled),
            v = &mut f => Ok(v),
        }
    }

    /// Espera `d` salvo que se cancele antes.
    async fn sleep(&mut self, d: Duration) -> BackendResult<()> {
        self.race(tokio::time::sleep(d)).await
    }
}

/// Estado publicado de una transferencia con el límite de frecuencia de los `Running`.
struct Progress {
    t: Transfer,
    last: Instant,
}

impl Progress {
    /// Emite el primer `Running` (con `done = 0`) al momento.
    fn start(inner: &crate::Inner, t: Transfer) -> Self {
        inner.emit(BackendEvent::TransferProgress(t.clone()));
        Self { t, last: Instant::now() }
    }

    /// Nuevo total transferido; emite `Running` si ha pasado `PROGRESS_INTERVAL` desde el último.
    fn advance(&mut self, inner: &crate::Inner, done: u64) {
        let done = if self.t.total > 0 { done.min(self.t.total) } else { done };
        self.t.done = self.t.done.max(done);
        if self.last.elapsed() >= PROGRESS_INTERVAL {
            self.last = Instant::now();
            inner.emit(BackendEvent::TransferProgress(self.t.clone()));
        }
    }

    /// Estado terminal (siempre se emite). En `Done`, `done = total = bytes`.
    fn finish(mut self, inner: &crate::Inner, r: &BackendResult<()>, bytes: u64) {
        self.t.state = match r {
            Ok(()) => {
                (self.t.done, self.t.total) = (bytes, bytes);
                TransferState::Done
            }
            Err(BackendError::Cancelled) => TransferState::Cancelled,
            Err(e) => TransferState::Failed(e.clone()),
        };
        inner.emit(BackendEvent::TransferProgress(self.t));
    }
}

/// Lector de un archivo de Telegram parte a parte (`upload.getFile`), siguiendo `FILE_MIGRATE` y
/// copiando la autorización a DCs ajenos. Recuerda el desplazamiento: tras un fallo (referencia
/// caducada, red) se puede reintentar `next` y sigue por la misma parte.
pub(crate) struct PartReader {
    dc: i32,
    home: i32,
    location: tl::enums::InputFileLocation,
    offset: i64,
    total: Option<u64>,
    copies: u8,
    migrations: u8,
    finished: bool,
}

impl PartReader {
    fn new(inner: &crate::Inner, dc: i32, location: tl::enums::InputFileLocation, total: Option<u64>) -> Result<Self, DownloadError> {
        let home = inner.net.session.home_dc_id().map_err(|_| DownloadError::Backend(BackendError::Unauthorized))?;
        let dc = if dc > 0 { dc } else { home };
        Ok(Self { dc, home, location, offset: 0, total, copies: 0, migrations: 0, finished: false })
    }

    /// Cambia la ubicación (referencia fresca) sin perder el desplazamiento.
    fn relocate(&mut self, dc: i32, location: tl::enums::InputFileLocation) {
        self.dc = if dc > 0 { dc } else { self.home };
        self.location = location;
        self.migrations = 0;
    }

    /// Siguiente parte, o `None` si ya se leyó la última.
    async fn next(&mut self, inner: &crate::Inner) -> Result<Option<Vec<u8>>, DownloadError> {
        if self.finished {
            return Ok(None);
        }
        let chunk = PART_SIZE as i32;
        loop {
            let seen = inner.dc_auth.lock().await.get(&self.dc).copied().unwrap_or(0);
            let req = tl::functions::upload::GetFile { precise: false, cdn_supported: false, location: self.location.clone(), offset: self.offset, limit: chunk };
            match inner.net.invoke_in(Some(self.dc), &req).await {
                Ok(tl::enums::upload::File::File(f)) => {
                    self.offset += i64::from(chunk);
                    let done = self.offset as u64;
                    self.finished = f.bytes.len() < PART_SIZE || self.total.is_some_and(|t| done >= t);
                    return Ok(Some(f.bytes));
                }
                // Con `cdn_supported = false` el servidor no redirige a CDN.
                Ok(tl::enums::upload::File::CdnRedirect(_)) => return Err(DownloadError::Backend(BackendError::Internal)),
                Err(InvocationError::Rpc(r)) if r.name == "AUTH_KEY_UNREGISTERED" && self.dc != self.home && self.copies < 2 => {
                    self.copies += 1;
                    inner.copy_auth_to(self.dc, seen).await.map_err(DownloadError::Backend)?;
                }
                Err(InvocationError::Rpc(r)) if r.code == 303 && self.migrations < 3 => match r.value {
                    Some(v) => {
                        self.migrations += 1;
                        self.dc = v as i32;
                    }
                    None => return Err(DownloadError::Backend(BackendError::Network)),
                },
                Err(e) if is_file_reference_error(&e) => return Err(DownloadError::FileReference),
                Err(e) => return Err(DownloadError::Backend(map_error(&e))),
            }
        }
    }
}

/// Archivo local ya comprobado para subir.
struct LocalFile {
    file: tokio::fs::File,
    size: u64,
    /// Nombre saneado que verá Telegram (y la UI en los eventos).
    name: String,
}

impl crate::Inner {
    /// Copia la autorización de la cuenta al DC `dc` (una sola vez aunque varias descargas lo pidan
    /// a la vez: `seen` es la generación que vio quien llama).
    async fn copy_auth_to(&self, dc: i32, seen: u64) -> BackendResult<()> {
        let mut gens = self.dc_auth.lock().await;
        if gens.get(&dc).copied().unwrap_or(0) != seen {
            return Ok(()); // otra descarga ya lo hizo mientras esperábamos
        }
        let tl::enums::auth::ExportedAuthorization::Authorization(x) = self.rpc(&tl::functions::auth::ExportAuthorization { dc_id: dc }).await?;
        self.net
            .invoke_in(Some(dc), &tl::functions::auth::ImportAuthorization { id: x.id, bytes: x.bytes })
            .await
            .map_err(|e| map_error(&e))?;
        gens.insert(dc, seen + 1);
        tracing::debug!(dc, "autorización copiada a otro DC para descargar");
        Ok(())
    }

    /// Descarga completa a memoria de `location` empezando en el DC `dc`. `progress`: id para emitir
    /// `BackendEvent::Download` en archivos de más de una parte (con `total` conocido).
    async fn download_location(
        &self,
        dc: i32,
        location: tl::enums::InputFileLocation,
        total: Option<u64>,
        progress: Option<i64>,
    ) -> Result<Vec<u8>, DownloadError> {
        let mut reader = PartReader::new(self, dc, location, total)?;
        let mut out = Vec::new();
        while let Some(bytes) = reader.next(self).await? {
            out.extend_from_slice(&bytes);
            let done = out.len() as u64;
            if let (Some(id), Some(t)) = (progress, total.filter(|t| *t > PART_SIZE as u64)) {
                self.emit(BackendEvent::Download { file_id: id, done: done.min(t), total: t });
            }
            if !reader.finished && done > MAX_IN_MEMORY {
                return Err(DownloadError::Backend(BackendError::InvalidInput("media")));
            }
        }
        Ok(out)
    }

    /// Referencia fresca de `old`: del mensaje `msg`, o del pack/GIF guardado si `msg = MsgId(0)`.
    async fn fresh_file(&self, peer: PeerRef, msg: MsgId, old: &FileRef) -> BackendResult<FileRef> {
        if msg.0 == 0 { self.refresh_loose_file(old).await } else { self.refresh_file(peer, msg, old).await }
    }

    /// Vuelve a pedir el mensaje para obtener una referencia de archivo fresca.
    async fn refresh_file(&self, peer: PeerRef, msg: MsgId, old: &FileRef) -> BackendResult<FileRef> {
        let id = vec![tl::types::InputMessageId { id: msg.0 }.into()];
        let r = match channel_of(peer) {
            Some(ch) => {
                let channel = self.cache().peers.input_channel(ch, &self.net.session)?;
                self.rpc(&tl::functions::channels::GetMessages { channel, id }).await?
            }
            None => self.rpc(&tl::functions::messages::GetMessages { id }).await?,
        };
        let (msgs, users, chats) = messages_of(r);
        self.ingest(&users, &chats);
        msgs.iter()
            .filter_map(|m| match m {
                tl::enums::Message::Message(m) => m.media.as_ref(),
                _ => None,
            })
            .filter_map(|m| {
                // Los stickers no van en `Media`: su referencia está en `Sticker.file`.
                let (md, st) = crate::convert::media_from_tl(m);
                md.and_then(|md| md.file).or_else(|| st.and_then(|s| s.file))
            })
            .find(|f| f.id == old.id && f.kind == old.kind)
            .ok_or(BackendError::NotFound)
    }

    pub(crate) async fn download_profile_photo(&self, peer: PeerRef, photo: ProfilePhoto, size: PhotoSize) -> BackendResult<Vec<u8>> {
        self.ensure_logged_in()?;
        let peer = match peer {
            PeerRef::Secret(id) => PeerRef::User(self.secret_partner(id).await?),
            p => p,
        };
        let input = self.input_peer(peer)?;
        let location = tl::types::InputPeerPhotoFileLocation { big: size == PhotoSize::Big, peer: input, photo_id: photo.id };
        let _permit = self.downloads.acquire().await.map_err(|_| BackendError::Internal)?;
        match self.download_location(photo.dc_id, location.into(), None, None).await {
            Ok(b) if b.is_empty() => Err(BackendError::NotFound),
            Ok(b) => Ok(b),
            // Las fotos de perfil no llevan referencia: si «caduca», la foto ya no es la vigente.
            Err(DownloadError::FileReference) => Err(BackendError::NotFound),
            Err(DownloadError::Backend(e)) => Err(e),
        }
    }

    pub(crate) async fn download_media(&self, peer: PeerRef, msg: MsgId, file: FileRef, variant: MediaVariant) -> BackendResult<Vec<u8>> {
        self.ensure_logged_in()?;
        if matches!(peer, PeerRef::Secret(_)) {
            return Err(secret_media());
        }
        let full_doc = file.kind == FileKind::Document && variant == MediaVariant::Full;
        if full_doc && file.size_bytes > MAX_IN_MEMORY {
            return Err(BackendError::InvalidInput("media"));
        }
        let _permit = self.downloads.acquire().await.map_err(|_| BackendError::Internal)?;
        let mut file = file;
        for attempt in 0..2 {
            let location = media_location(&file, variant).ok_or(BackendError::NotFound)?;
            match self.download_location(file.dc_id, location, known_total(&file, variant), Some(file.id)).await {
                Ok(b) if b.is_empty() => return Err(BackendError::NotFound),
                Ok(b) => return Ok(b),
                Err(DownloadError::FileReference) if attempt == 0 => {
                    tracing::debug!(id = file.id, "referencia de archivo caducada: se refresca");
                    file = self.fresh_file(peer, msg, &file).await?;
                }
                Err(DownloadError::FileReference) => return Err(BackendError::NotFound),
                Err(DownloadError::Backend(e)) => return Err(e),
            }
        }
        Err(BackendError::NotFound)
    }

    /// Arranca la descarga a disco (ADR-0011): valida, emite el primer `Running` y sigue en otra tarea.
    pub(crate) fn start_download_to(self: &Arc<Self>, peer: PeerRef, msg: MsgId, file: FileRef, variant: MediaVariant, dest: PathBuf) -> BackendResult<TransferId> {
        self.ensure_logged_in()?;
        if matches!(peer, PeerRef::Secret(_)) {
            return Err(secret_media());
        }
        check_dest(&dest)?;
        media_location(&file, variant).ok_or(BackendError::NotFound)?;
        if self.net.conn.is_offline() {
            return Err(BackendError::Network);
        }
        let id = self.transfers.next_download();
        let mut cancel = self.transfers.register(id);
        let t = Transfer {
            id,
            kind: TransferKind::Download,
            peer,
            msg: Some(msg),
            name: dest.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default(),
            done: 0,
            total: known_total(&file, variant).unwrap_or(0),
            state: TransferState::Running,
        };
        let mut progress = Progress::start(self, t);
        let inner = Arc::clone(self);
        tokio::spawn(async move {
            let part = partial_path(&dest);
            let r = inner.download_to_disk(peer, msg, file, variant, &dest, &part, &mut progress, &mut cancel).await;
            let bytes = match &r {
                Ok(n) => *n,
                Err(_) => {
                    // El parcial puede no existir (falló antes de crearlo).
                    let _ = tokio::fs::remove_file(&part).await;
                    0
                }
            };
            inner.transfers.unregister(&cancel);
            progress.finish(&inner, &r.map(|_| ()), bytes);
        });
        Ok(id)
    }

    /// Descarga por partes a `part` y la renombra a `dest`. Devuelve los bytes escritos.
    #[allow(clippy::too_many_arguments)]
    async fn download_to_disk(
        &self,
        peer: PeerRef,
        msg: MsgId,
        mut file: FileRef,
        variant: MediaVariant,
        dest: &Path,
        part: &Path,
        progress: &mut Progress,
        cancel: &mut Cancel,
    ) -> BackendResult<u64> {
        let _permit = cancel.race(self.downloads.acquire()).await?.map_err(|_| BackendError::Internal)?;
        let location = media_location(&file, variant).ok_or(BackendError::NotFound)?;
        let mut reader = PartReader::new(self, file.dc_id, location, known_total(&file, variant)).map_err(DownloadError::into_backend)?;
        let mut out = tokio::fs::File::create(part).await.map_err(storage)?;
        let (mut written, mut refreshed, mut resumes) = (0u64, false, 0u32);
        loop {
            match cancel.race(reader.next(self)).await? {
                Ok(Some(bytes)) => {
                    (refreshed, resumes) = (false, 0);
                    out.write_all(&bytes).await.map_err(storage)?;
                    written += bytes.len() as u64;
                    progress.advance(self, written);
                }
                Ok(None) => break,
                // Una vez por parte: si sigue caducada tras refrescar, el archivo ya no está.
                Err(DownloadError::FileReference) if !refreshed => {
                    refreshed = true;
                    tracing::debug!(id = file.id, "referencia de archivo caducada a mitad de descarga: se refresca");
                    file = cancel.race(self.fresh_file(peer, msg, &file)).await??;
                    let location = media_location(&file, variant).ok_or(BackendError::NotFound)?;
                    reader.relocate(file.dc_id, location);
                }
                Err(DownloadError::FileReference) => return Err(BackendError::NotFound),
                Err(DownloadError::Backend(BackendError::Network)) if resumes < MAX_RESUMES => {
                    resumes += 1;
                    tracing::debug!(resumes, "corte de red en una descarga: se reanuda la misma parte");
                    cancel.sleep(self.resume_wait(resumes)).await?;
                }
                Err(DownloadError::Backend(e)) => return Err(e),
            }
        }
        if written == 0 {
            return Err(BackendError::NotFound);
        }
        out.flush().await.map_err(storage)?;
        out.sync_all().await.map_err(storage)?;
        drop(out);
        // Última oportunidad de cancelar; después, el archivo ya está en su sitio y acaba en `Done`.
        if cancel.requested() {
            return Err(BackendError::Cancelled);
        }
        tokio::fs::rename(part, dest).await.map_err(storage)?;
        Ok(written)
    }

    /// Espera antes de la reanudación número `n` (1, 2, …) de una parte.
    fn resume_wait(&self, n: u32) -> Duration {
        (self.net.cfg.network_backoff * 2u32.pow(n.min(6))).min(Duration::from_secs(30))
    }

    /// Abre y comprueba los archivos de `media` antes de subir nada (sin eventos si falla).
    async fn open_items(media: &OutgoingMedia) -> BackendResult<Vec<LocalFile>> {
        let mut out = Vec::with_capacity(media.items.len());
        for item in &media.items {
            let file = tokio::fs::File::open(&item.path).await.map_err(|_| BackendError::InvalidInput("media"))?;
            let meta = file.metadata().await.map_err(|_| BackendError::InvalidInput("media"))?;
            let name = sanitize_file_name(&item.file_name);
            if !meta.is_file() || meta.len() == 0 {
                return Err(BackendError::InvalidInput("media"));
            }
            if meta.len() > MAX_FILE {
                return Err(too_large(&name, meta.len()));
            }
            out.push(LocalFile { file, size: meta.len(), name });
        }
        Ok(out)
    }

    /// Sube cada archivo y lo envía con `sendMedia` (uno a uno: lo confirmado queda enviado).
    async fn upload_and_send(
        &self,
        peer: PeerRef,
        media: &OutgoingMedia,
        files: Vec<LocalFile>,
        progress: &mut Progress,
        cancel: &mut Cancel,
    ) -> BackendResult<Vec<Message>> {
        let mut out = Vec::new();
        let mut base = 0u64;
        for (i, ((item, rid), local)) in media.items.iter().zip(&media.random_ids).zip(files).enumerate() {
            let size = local.size;
            let name = local.name.clone();
            let file = self.upload(local, base, progress, cancel).await?;
            base += size;
            // No se corta un `sendMedia` en vuelo (el mensaje podría quedar enviado sin saberlo).
            if cancel.requested() {
                return Err(BackendError::Cancelled);
            }
            let (caption, reply) = if i == 0 { (media.caption.clone(), media.reply_to) } else { (String::new(), None) };
            let item = OutgoingMediaItem { file_name: name, ..item.clone() };
            out.push(self.send_media_item(peer, input_media(&item, file), caption, reply, *rid).await?);
        }
        Ok(out)
    }

    /// Sube un archivo ya abierto por partes y devuelve el `InputFile` para `sendMedia`. `base`:
    /// bytes de la transferencia ya subidos antes de este archivo.
    async fn upload(&self, local: LocalFile, base: u64, progress: &mut Progress, cancel: &mut Cancel) -> BackendResult<tl::enums::InputFile> {
        let LocalFile { mut file, size, name } = local;
        let big = size > BIG_FILE;
        let total = parts_for(size);
        let file_id = random_i64();
        let mut buf = vec![0u8; PART_SIZE];
        let mut sent = 0u64;
        for part in 0..total {
            let mut filled = 0;
            while filled < PART_SIZE {
                let n = file.read(&mut buf[filled..]).await.map_err(|_| BackendError::InvalidInput("media"))?;
                if n == 0 {
                    break;
                }
                filled += n;
            }
            let mut resumes = 0;
            loop {
                let bytes = buf[..filled].to_vec();
                let r = if big {
                    cancel.race(self.rpc(&tl::functions::upload::SaveBigFilePart { file_id, file_part: part, file_total_parts: total, bytes })).await?
                } else {
                    cancel.race(self.rpc(&tl::functions::upload::SaveFilePart { file_id, file_part: part, bytes })).await?
                };
                match r {
                    Ok(true) => break,
                    Ok(false) => return Err(BackendError::Network),
                    // Guardar una parte es idempotente: se reintenta la misma.
                    Err(BackendError::Network) if resumes < MAX_RESUMES => {
                        resumes += 1;
                        cancel.sleep(self.resume_wait(resumes)).await?;
                    }
                    Err(e) => return Err(e),
                }
            }
            sent += filled as u64;
            progress.advance(self, base + sent);
        }
        Ok(if big {
            tl::types::InputFileBig { id: file_id, parts: total, name }.into()
        } else {
            // El md5 es opcional: el servidor lo acepta vacío.
            tl::types::InputFile { id: file_id, parts: total, name, md5_checksum: String::new() }.into()
        })
    }
}

/// Total conocido de una variante: el tamaño del archivo completo (las miniaturas no lo traen).
fn known_total(file: &FileRef, variant: MediaVariant) -> Option<u64> {
    if variant == MediaVariant::Full { Some(file.size_bytes).filter(|t| *t > 0) } else { None }
}

fn secret_media() -> BackendError {
    BackendError::Forbidden("los chats secretos aún no admiten medios".into())
}

/// Fallo de una descarga: la referencia caducó (se puede refrescar) u otro error ya traducido.
#[derive(Debug)]
pub(crate) enum DownloadError {
    FileReference,
    Backend(BackendError),
}

impl DownloadError {
    fn into_backend(self) -> BackendError {
        match self {
            Self::FileReference => BackendError::NotFound,
            Self::Backend(e) => e,
        }
    }
}

pub(crate) fn input_media(item: &OutgoingMediaItem, file: tl::enums::InputFile) -> tl::enums::InputMedia {
    if item.kind == MediaKind::Photo {
        return tl::types::InputMediaUploadedPhoto { spoiler: false, live_photo: false, file, stickers: None, ttl_seconds: None, video: None }.into();
    }
    let mut attributes: Vec<tl::enums::DocumentAttribute> = vec![tl::types::DocumentAttributeFilename { file_name: item.file_name.clone() }.into()];
    match item.kind {
        MediaKind::Video => attributes.push(
            tl::types::DocumentAttributeVideo {
                round_message: false,
                supports_streaming: true,
                nosound: false,
                duration: 0.0,
                w: item.width as i32,
                h: item.height as i32,
                preload_prefix_size: None,
                video_start_ts: None,
                video_codec: None,
            }
            .into(),
        ),
        MediaKind::Voice => {
            attributes.push(tl::types::DocumentAttributeAudio { voice: true, duration: 0, title: None, performer: None, waveform: None }.into())
        }
        MediaKind::Gif => attributes.push(tl::enums::DocumentAttribute::Animated),
        _ => {}
    }
    tl::types::InputMediaUploadedDocument {
        nosound_video: false,
        force_file: item.kind == MediaKind::File,
        spoiler: false,
        file,
        thumb: None,
        mime_type: mime_for(&item.file_name).to_string(),
        attributes,
        stickers: None,
        video_cover: None,
        video_timestamp: None,
        ttl_seconds: None,
    }
    .into()
}

#[async_trait]
impl MediaApi for GrammersBackend {
    /// Subida con progreso y cancelación (ADR-0011 §3): una llamada = una transferencia con id
    /// `TransferId::upload_of(&media)`. Los errores de validación (lista vacía, chat desconocido,
    /// archivo ilegible, vacío o de más de 2000 MiB) se devuelven sin eventos.
    async fn send_media(&self, peer: PeerRef, media: OutgoingMedia) -> BackendResult<Vec<Message>> {
        let inner = &self.inner;
        let id = match TransferId::upload_of(&media) {
            Some(id) if media.items.len() == media.random_ids.len() => id,
            _ => return Err(BackendError::InvalidInput("items")),
        };
        inner.ensure_logged_in()?;
        inner.input_peer(peer)?;
        let files = crate::Inner::open_items(&media).await?;
        let t = Transfer {
            id,
            kind: TransferKind::Upload,
            peer,
            msg: None,
            name: files[0].name.clone(),
            done: 0,
            total: files.iter().map(|f| f.size).sum(),
            state: TransferState::Running,
        };
        let total = t.total;
        let mut cancel = inner.transfers.register(id);
        let mut progress = Progress::start(inner, t);
        let r = inner.upload_and_send(peer, &media, files, &mut progress, &mut cancel).await;
        inner.transfers.unregister(&cancel);
        progress.finish(inner, &r.as_ref().map(|_| ()).map_err(Clone::clone), total);
        r
    }

    async fn download_media_to(&self, peer: PeerRef, msg: MsgId, file: FileRef, variant: MediaVariant, dest: PathBuf) -> BackendResult<TransferId> {
        self.inner.start_download_to(peer, msg, file, variant, dest)
    }

    async fn cancel_transfer(&self, id: TransferId) -> BackendResult<()> {
        self.inner.transfers.cancel(id);
        Ok(())
    }

    async fn download_profile_photo(&self, peer: PeerRef, photo: ProfilePhoto, size: PhotoSize) -> BackendResult<Vec<u8>> {
        self.inner.download_profile_photo(peer, photo, size).await
    }

    async fn download_media(&self, peer: PeerRef, msg: MsgId, file: FileRef, variant: MediaVariant) -> BackendResult<Vec<u8>> {
        self.inner.download_media(peer, msg, file, variant).await
    }
}
