//! WebP estático a RGBA8 y codificación sin pérdida (útil para pruebas y miniaturas).

use std::io::Cursor;

use image::{ImageEncoder, ImageFormat, codecs::webp::WebPEncoder};

use crate::{Frame, StickerError};

/// Lado máximo aceptado (los stickers de la red miden 512).
pub const MAX_SIDE: u32 = 4096;

/// Decodifica un WebP (estático) a RGBA8.
pub fn decode(bytes: &[u8]) -> Result<Frame, StickerError> {
    let mut reader = image::ImageReader::with_format(Cursor::new(bytes), ImageFormat::WebP);
    let mut limits = image::Limits::default();
    limits.max_image_width = Some(MAX_SIDE);
    limits.max_image_height = Some(MAX_SIDE);
    reader.limits(limits);
    let rgba = reader.decode()?.into_rgba8();
    let (width, height) = rgba.dimensions();
    Ok(Frame { width, height, rgba: rgba.into_raw() })
}

/// Codifica un fotograma como WebP sin pérdida.
pub fn encode_lossless(frame: &Frame) -> Result<Vec<u8>, StickerError> {
    let mut out = Vec::new();
    WebPEncoder::new_lossless(&mut out).write_image(
        &frame.rgba,
        frame.width,
        frame.height,
        image::ExtendedColorType::Rgba8,
    )?;
    Ok(out)
}
