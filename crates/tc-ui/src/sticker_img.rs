//! Stickers de la red como imagen: qué variante pedir y cómo decodificarla.
//!
//! * WebP estático: el archivo entero en la burbuja; su miniatura (WebP de ~128 px) en el selector.
//! * TGS (Lottie con gzip): sin `lottie` (rlottie; solo se compila en Linux, ADR-0008) se dibuja la
//!   miniatura WebP estática que trae el documento; con `lottie`, el primer fotograma del TGS en la
//!   burbuja. Esa es la imagen estática (movimiento reducido, mientras cargan los fotogramas); la
//!   animación va aparte, en `sticker_anim.rs`.
//! * WebM (vídeo): su miniatura estática. Reproducir el vídeo queda pendiente (sin decodificador de
//!   VP9 con alfa fiable y portable, ver ADR-0008).
//!
//! La descarga va por `MediaApi::download_media` (con la caché de disco de tc-store) y el
//! decodificado fuera del hilo de la UI (`media::load`); los `Handle` viven en `media::Images`.

use ::image::imageops::FilterType;
use iced::widget::image::Handle;
use tc_core::{MediaVariant, MsgId, PeerRef, Sticker, StickerFormat};

use crate::media::{Decoded, ImageKey, ImageRequest};

/// Lado (px físicos) al que se decodifica el sticker de la burbuja (2× su caja de 200 px, sin pasar
/// de los 512 px nativos).
pub const BUBBLE_PX: u32 = 400;
/// Lado de las celdas del selector y las pestañas (2× 60 px).
pub const CELL_PX: u32 = 128;
/// Por debajo de este tamaño en pantalla se usa la miniatura.
pub const SMALL_MAX: f32 = 96.0;

/// Variante que se pide para dibujar `s` en pequeño (selector) o en grande (burbuja). `None`: no
/// hay nada que se pueda dibujar como imagen (p. ej. un WebM sin miniatura) y queda el emoji.
pub fn variant_for(s: &Sticker, small: bool) -> Option<MediaVariant> {
    let thumb = s.file.as_ref()?.thumb.is_some();
    match s.format {
        StickerFormat::Webp => Some(if small && thumb { MediaVariant::Thumb } else { MediaVariant::Full }),
        StickerFormat::Lottie if !small && tc_stickers::tgs::lottie_enabled() => Some(MediaVariant::Full),
        StickerFormat::Lottie | StickerFormat::Webm => thumb.then_some(MediaVariant::Thumb),
    }
}

/// Clave de la imagen de un sticker (global: el mismo documento en dos chats es la misma imagen).
pub fn key(s: &Sticker, variant: MediaVariant) -> Option<ImageKey> {
    s.file.as_ref().map(|f| ImageKey::Sticker { file: f.id, full: variant == MediaVariant::Full })
}

/// Petición para dibujar `s`. `msg = MsgId(0)` si no cuelga de un mensaje real (selector o envío
/// aún pendiente): el backend refresca la referencia por el pack.
pub fn request(peer: PeerRef, msg: MsgId, s: &Sticker, small: bool) -> Option<ImageRequest> {
    let variant = variant_for(s, small)?;
    Some(ImageRequest::Sticker { peer, msg, file: s.file.clone()?, format: s.format, variant })
}

fn fit(img: ::image::RgbaImage, px: u32) -> Decoded {
    let (w, h) = img.dimensions();
    let img = if w.max(h) > px {
        let (nw, nh) = if w >= h { (px, (h * px / w).max(1)) } else { ((w * px / h).max(1), px) };
        ::image::imageops::resize(&img, nw, nh, FilterType::Triangle)
    } else {
        img
    };
    let (width, height) = img.dimensions();
    Decoded { handle: Handle::from_rgba(width, height, img.into_raw()), width, height }
}

/// Decodifica los bytes descargados (en el pool bloqueante).
pub fn decode(bytes: &[u8], format: StickerFormat, variant: MediaVariant, cache_key: i64) -> Option<Decoded> {
    let px = if variant == MediaVariant::Full { BUBBLE_PX } else { CELL_PX };
    match (format, variant) {
        (StickerFormat::Lottie, MediaVariant::Full) => {
            let f = tc_stickers::tgs::render(bytes, &format!("tg{cache_key}"), 0, px, px).ok()?;
            ::image::RgbaImage::from_raw(f.width, f.height, f.rgba).map(|i| fit(i, px))
        }
        (StickerFormat::Webp, MediaVariant::Full) => {
            let f = tc_stickers::webp::decode(bytes).ok()?;
            ::image::RgbaImage::from_raw(f.width, f.height, f.rgba).map(|i| fit(i, px))
        }
        // WebM completo no se decodifica (sin vídeo): nunca se pide.
        (StickerFormat::Webm, MediaVariant::Full) => None,
        // Miniaturas: WebP (a veces JPEG/PNG en documentos antiguos).
        (_, MediaVariant::Thumb) => ::image::load_from_memory(bytes).ok().map(|i| fit(i.to_rgba8(), px)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tc_core::{FileKind, FileRef, StickerId, StickerSetId};

    fn sticker(format: StickerFormat, thumb: bool) -> Sticker {
        Sticker {
            id: StickerId(5),
            set_id: StickerSetId(6),
            emoji: "😺".into(),
            width: 512,
            height: 512,
            format,
            file: Some(FileRef {
                kind: FileKind::Document,
                id: 5,
                access_hash: 1,
                file_reference: vec![],
                dc_id: 2,
                size_bytes: 9,
                thumb: thumb.then(|| "m".into()),
                full: None,
            }),
        }
    }

    #[test]
    fn variants_per_format() {
        use MediaVariant::*;
        assert_eq!(variant_for(&sticker(StickerFormat::Webp, true), true), Some(Thumb));
        assert_eq!(variant_for(&sticker(StickerFormat::Webp, true), false), Some(Full));
        assert_eq!(variant_for(&sticker(StickerFormat::Webp, false), true), Some(Full));
        assert_eq!(variant_for(&sticker(StickerFormat::Webm, true), false), Some(Thumb));
        assert_eq!(variant_for(&sticker(StickerFormat::Webm, false), false), None);
        let tgs = variant_for(&sticker(StickerFormat::Lottie, true), false);
        assert_eq!(tgs, Some(if tc_stickers::tgs::lottie_enabled() { Full } else { Thumb }));
        assert_eq!(variant_for(&Sticker { file: None, ..sticker(StickerFormat::Webp, true) }, false), None);
        assert_eq!(key(&sticker(StickerFormat::Webp, true), Full), Some(ImageKey::Sticker { file: 5, full: true }));
    }

    #[test]
    fn webp_stickers_decode_and_fit() {
        let frame = tc_stickers::Frame { width: 512, height: 256, rgba: vec![200; 512 * 256 * 4] };
        let bytes = tc_stickers::webp::encode_lossless(&frame).unwrap();
        let d = decode(&bytes, StickerFormat::Webp, MediaVariant::Full, 1).unwrap();
        assert_eq!((d.width, d.height), (BUBBLE_PX, BUBBLE_PX / 2));
        let t = decode(&bytes, StickerFormat::Lottie, MediaVariant::Thumb, 1).unwrap();
        assert_eq!((t.width, t.height), (CELL_PX, CELL_PX / 2));
        assert!(decode(b"basura", StickerFormat::Webp, MediaVariant::Full, 1).is_none());
        assert!(decode(&bytes, StickerFormat::Webm, MediaVariant::Full, 1).is_none());
    }
}
