//! Envío de fotos/archivos: subida por partes (`upload.saveFilePart` / `saveBigFilePart`)
//! y `messages.sendMedia`. Sin reanudación todavía (la tabla `uploads` de tc-store la hará).
use async_trait::async_trait;
use grammers_tl_types as tl;
use tc_core::*;
use tokio::io::AsyncReadExt;

use crate::GrammersBackend;
use crate::backend::random_i64;

/// Partes de 512 KiB (máximo permitido por la API).
pub(crate) const PART_SIZE: usize = 512 * 1024;
/// Por encima de 10 MiB hay que usar `saveBigFilePart`.
pub(crate) const BIG_FILE: u64 = 10 * 1024 * 1024;
/// Límite de tamaño de archivo para cuentas normales.
pub(crate) const MAX_FILE: u64 = 2 * 1024 * 1024 * 1024;

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

impl crate::Inner {
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
}
