//! TGS = Lottie JSON comprimido con gzip. La descompresión lleva tope contra bombas; el render
//! solo existe con la feature `lottie` (sin ella devuelve [`StickerError::LottieDisabled`]).

use std::io::Read;

use flate2::read::GzDecoder;

use crate::{Frame, StickerError};

/// Tope de bytes descomprimidos (4 MiB, ver architecture.md §8).
pub const MAX_UNCOMPRESSED: usize = 4 * 1024 * 1024;

/// Descomprime un TGS y comprueba que parece JSON Lottie (objeto con `v`, `fr`, `layers`...).
pub fn decompress(bytes: &[u8]) -> Result<Vec<u8>, StickerError> {
    let mut out = Vec::new();
    GzDecoder::new(bytes)
        .take(MAX_UNCOMPRESSED as u64 + 1)
        .read_to_end(&mut out)
        .map_err(StickerError::Gzip)?;
    if out.len() > MAX_UNCOMPRESSED {
        return Err(StickerError::TooLarge(MAX_UNCOMPRESSED));
    }
    let head = out.iter().find(|b| !b.is_ascii_whitespace());
    if head != Some(&b'{') || out.last().is_none_or(|b| !b.is_ascii_whitespace() && *b != b'}') {
        return Err(StickerError::NotLottie);
    }
    Ok(out)
}

/// Comprime JSON Lottie a TGS (para pruebas y empaquetado propio).
pub fn compress(json: &[u8]) -> Vec<u8> {
    use std::io::Write;
    let mut enc = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::default());
    // Escribir en un Vec no falla.
    let _ = enc.write_all(json);
    enc.finish().unwrap_or_default()
}

/// Metadatos de una animación.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TgsInfo {
    pub frames: usize,
    pub fps: f64,
    pub width: usize,
    pub height: usize,
}

/// Renderiza el fotograma `frame` de un TGS a `width`×`height` RGBA (sin premultiplicar).
#[cfg(feature = "lottie")]
pub fn render(
    tgs: &[u8],
    cache_key: &str,
    frame: usize,
    width: u32,
    height: u32,
) -> Result<Frame, StickerError> {
    if width == 0 || height == 0 || width > 2048 || height > 2048 {
        return Err(StickerError::BadSize);
    }
    let json = decompress(tgs)?;
    if json.contains(&0) || cache_key.as_bytes().contains(&0) {
        return Err(StickerError::NotLottie);
    }
    let mut anim = rlottie::Animation::from_data(json, cache_key, ".").ok_or(StickerError::LottieLoad)?;
    if frame >= anim.totalframe() {
        return Err(StickerError::FrameOutOfRange(frame));
    }
    let mut surface = rlottie::Surface::new(rlottie::Size::new(width as usize, height as usize));
    anim.render(frame, &mut surface);
    Ok(Frame { width, height, rgba: unpremultiply(surface.data()) })
}

/// rlottie entrega ARGB premultiplicado (en memoria BGRA): se pasa a RGBA recto.
#[cfg(feature = "lottie")]
pub(crate) fn unpremultiply(data: &[rlottie::Bgra]) -> Vec<u8> {
    let mut rgba = Vec::with_capacity(data.len() * 4);
    for p in data {
        let a = u32::from(p.a);
        let un = |c: u8| (u32::from(c) * 255 + a / 2).checked_div(a).map_or(0, |v| v.min(255) as u8);
        rgba.extend_from_slice(&[un(p.r), un(p.g), un(p.b), p.a]);
    }
    rgba
}

#[cfg(not(feature = "lottie"))]
pub fn render(
    _tgs: &[u8],
    _cache_key: &str,
    _frame: usize,
    _width: u32,
    _height: u32,
) -> Result<Frame, StickerError> {
    Err(StickerError::LottieDisabled)
}

/// Lee número de fotogramas y fps.
#[cfg(feature = "lottie")]
pub fn info(tgs: &[u8], cache_key: &str) -> Result<TgsInfo, StickerError> {
    let json = decompress(tgs)?;
    if json.contains(&0) || cache_key.as_bytes().contains(&0) {
        return Err(StickerError::NotLottie);
    }
    let anim = rlottie::Animation::from_data(json, cache_key, ".").ok_or(StickerError::LottieLoad)?;
    let size = anim.size();
    Ok(TgsInfo { frames: anim.totalframe(), fps: anim.framerate(), width: size.width, height: size.height })
}

#[cfg(not(feature = "lottie"))]
pub fn info(_tgs: &[u8], _cache_key: &str) -> Result<TgsInfo, StickerError> {
    Err(StickerError::LottieDisabled)
}

/// `true` si esta compilación puede renderizar Lottie.
pub const fn lottie_enabled() -> bool {
    cfg!(feature = "lottie")
}
