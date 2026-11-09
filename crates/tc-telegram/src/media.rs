//! Envío de fotos/archivos: subida por partes (`upload.saveFilePart` / `saveBigFilePart`)
//! y `messages.sendMedia`. Sin reanudación todavía (la tabla `uploads` de tc-store la hará).
//!
//! Descarga (`upload.getFile` por partes de 512 KiB) de fotos de perfil
//! (`inputPeerPhotoFileLocation`) y de medios (`inputPhotoFileLocation` /
//! `inputDocumentFileLocation`) en el DC del archivo:
//! - DC distinto del de casa sin autorizar (`AUTH_KEY_UNREGISTERED`): se copia la autorización
//!   (`auth.exportAuthorization` en casa + `auth.importAuthorization` allí) y se reintenta.
//! - `FILE_MIGRATE_X`: se sigue al DC indicado.
//! - `FILE_REFERENCE_*` (referencia caducada): se vuelve a pedir el mensaje para obtener una
//!   referencia fresca y se reintenta una vez.
//! - `FLOOD_WAIT` corto se duerme en `Net::invoke_in`; uno largo se devuelve como `FloodWait`.
//! - Como mucho `MAX_DOWNLOADS` descargas a la vez (semáforo).
use async_trait::async_trait;
use grammers_client::InvocationError;
use grammers_session::Session;
use grammers_tl_types as tl;
use tc_core::*;
use tokio::io::AsyncReadExt;

use crate::GrammersBackend;
use crate::backend::{messages_of, random_i64};
use crate::ids::channel_of;
use crate::net::map_error;

/// Partes de 512 KiB (máximo permitido por la API).
pub(crate) const PART_SIZE: usize = 512 * 1024;
/// Por encima de 10 MiB hay que usar `saveBigFilePart`.
pub(crate) const BIG_FILE: u64 = 10 * 1024 * 1024;
/// Límite de tamaño de archivo para cuentas normales.
pub(crate) const MAX_FILE: u64 = 2 * 1024 * 1024 * 1024;

/// Descargas simultáneas como mucho.
pub(crate) const MAX_DOWNLOADS: usize = 4;
/// Tope de lo que se descarga a memoria (los documentos mayores se rechazan).
pub(crate) const MAX_IN_MEMORY: u64 = 128 * 1024 * 1024;

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

    /// Descarga completa de `location` empezando en el DC `dc`. `progress`: `(id, total)` para emitir
    /// `BackendEvent::Download` en archivos de más de una parte.
    async fn download_location(
        &self,
        dc: i32,
        location: tl::enums::InputFileLocation,
        total: Option<u64>,
        progress: Option<i64>,
    ) -> Result<Vec<u8>, DownloadError> {
        let home = self.net.session.home_dc_id().map_err(|_| DownloadError::Backend(BackendError::Unauthorized))?;
        let mut dc = if dc > 0 { dc } else { home };
        let (mut out, mut offset, mut copies, mut migrations) = (Vec::new(), 0i64, 0u8, 0u8);
        let chunk = PART_SIZE as i32;
        loop {
            let seen = self.dc_auth.lock().await.get(&dc).copied().unwrap_or(0);
            let req = tl::functions::upload::GetFile { precise: false, cdn_supported: false, location: location.clone(), offset, limit: chunk };
            match self.net.invoke_in(Some(dc), &req).await {
                Ok(tl::enums::upload::File::File(f)) => {
                    let n = f.bytes.len();
                    out.extend_from_slice(&f.bytes);
                    offset += i64::from(chunk);
                    let done = out.len() as u64;
                    if let (Some(id), Some(t)) = (progress, total.filter(|t| *t > PART_SIZE as u64)) {
                        self.emit(BackendEvent::Download { file_id: id, done: done.min(t), total: t });
                    }
                    if n < PART_SIZE || total.is_some_and(|t| done >= t) {
                        return Ok(out);
                    }
                    if done > MAX_IN_MEMORY {
                        return Err(DownloadError::Backend(BackendError::InvalidInput("media")));
                    }
                }
                // Con `cdn_supported = false` el servidor no redirige a CDN.
                Ok(tl::enums::upload::File::CdnRedirect(_)) => return Err(DownloadError::Backend(BackendError::Internal)),
                Err(InvocationError::Rpc(r)) if r.name == "AUTH_KEY_UNREGISTERED" && dc != home && copies < 2 => {
                    copies += 1;
                    self.copy_auth_to(dc, seen).await.map_err(DownloadError::Backend)?;
                }
                Err(InvocationError::Rpc(r)) if r.code == 303 && migrations < 3 => match r.value {
                    Some(v) => {
                        migrations += 1;
                        dc = v as i32;
                    }
                    None => return Err(DownloadError::Backend(BackendError::Network)),
                },
                Err(e) if is_file_reference_error(&e) => return Err(DownloadError::FileReference),
                Err(e) => return Err(DownloadError::Backend(map_error(&e))),
            }
        }
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
            .filter_map(|m| crate::convert::media_from_tl(m).0.and_then(|md| md.file))
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
            return Err(BackendError::Forbidden("los chats secretos aún no admiten medios".into()));
        }
        let full_doc = file.kind == FileKind::Document && variant == MediaVariant::Full;
        if full_doc && file.size_bytes > MAX_IN_MEMORY {
            return Err(BackendError::InvalidInput("media"));
        }
        let _permit = self.downloads.acquire().await.map_err(|_| BackendError::Internal)?;
        let mut file = file;
        for attempt in 0..2 {
            let location = media_location(&file, variant).ok_or(BackendError::NotFound)?;
            let total = if variant == MediaVariant::Full { Some(file.size_bytes).filter(|t| *t > 0) } else { None };
            match self.download_location(file.dc_id, location, total, Some(file.id)).await {
                Ok(b) if b.is_empty() => return Err(BackendError::NotFound),
                Ok(b) => return Ok(b),
                Err(DownloadError::FileReference) if attempt == 0 => {
                    tracing::debug!(id = file.id, "referencia de archivo caducada: se refresca");
                    file = self.refresh_file(peer, msg, &file).await?;
                }
                Err(DownloadError::FileReference) => return Err(BackendError::NotFound),
                Err(DownloadError::Backend(e)) => return Err(e),
            }
        }
        Err(BackendError::NotFound)
    }

    /// Sube un archivo del disco y devuelve el `InputFile` para `sendMedia`.
    async fn upload(&self, path: &str, name: &str) -> BackendResult<tl::enums::InputFile> {
        let mut f = tokio::fs::File::open(path).await.map_err(|_| BackendError::InvalidInput("media"))?;
        let size = f.metadata().await.map_err(|_| BackendError::InvalidInput("media"))?.len();
        if size == 0 || size > MAX_FILE {
            return Err(BackendError::InvalidInput("media"));
        }
        let big = size > BIG_FILE;
        let total = parts_for(size);
        let file_id = random_i64();
        let mut buf = vec![0u8; PART_SIZE];
        for part in 0..total {
            let mut filled = 0;
            while filled < PART_SIZE {
                let n = f.read(&mut buf[filled..]).await.map_err(|_| BackendError::InvalidInput("media"))?;
                if n == 0 {
                    break;
                }
                filled += n;
            }
            let bytes = buf[..filled].to_vec();
            let ok = if big {
                self.rpc(&tl::functions::upload::SaveBigFilePart { file_id, file_part: part, file_total_parts: total, bytes }).await?
            } else {
                self.rpc(&tl::functions::upload::SaveFilePart { file_id, file_part: part, bytes }).await?
            };
            if !ok {
                return Err(BackendError::Network);
            }
        }
        Ok(if big {
            tl::types::InputFileBig { id: file_id, parts: total, name: name.to_string() }.into()
        } else {
            // El md5 es opcional: el servidor lo acepta vacío.
            tl::types::InputFile { id: file_id, parts: total, name: name.to_string(), md5_checksum: String::new() }.into()
        })
    }
}

/// Fallo de una descarga: la referencia caducó (se puede refrescar) u otro error ya traducido.
#[derive(Debug)]
pub(crate) enum DownloadError {
    FileReference,
    Backend(BackendError),
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
    async fn send_media(&self, peer: PeerRef, media: OutgoingMedia) -> BackendResult<Vec<Message>> {
        if media.items.is_empty() || media.items.len() != media.random_ids.len() {
            return Err(BackendError::InvalidInput("items"));
        }
        let mut out = Vec::new();
        for (i, (item, rid)) in media.items.iter().zip(&media.random_ids).enumerate() {
            let file = self.inner.upload(&item.path, &item.file_name).await?;
            let (caption, reply) = if i == 0 { (media.caption.clone(), media.reply_to) } else { (String::new(), None) };
            let m = self.inner.send_media_item(peer, input_media(item, file), caption, reply, *rid).await?;
            out.push(m);
        }
        Ok(out)
    }

    async fn download_profile_photo(&self, peer: PeerRef, photo: ProfilePhoto, size: PhotoSize) -> BackendResult<Vec<u8>> {
        self.inner.download_profile_photo(peer, photo, size).await
    }

    async fn download_media(&self, peer: PeerRef, msg: MsgId, file: FileRef, variant: MediaVariant) -> BackendResult<Vec<u8>> {
        self.inner.download_media(peer, msg, file, variant).await
    }
}
