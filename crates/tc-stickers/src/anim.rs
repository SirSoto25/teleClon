//! Animación de stickers TGS (ADR-0008): datos ya descomprimidos y validados, qué fotograma toca en
//! cada instante (a 30 fps como mucho), política de movimiento (reducido, foco, visibilidad) y render
//! por lotes con rlottie (feature `lottie`).
//!
//! `rlottie::Animation` no es `Send`: no se guarda entre fotogramas. Cada lote crea la animación a
//! partir del JSON (rlottie reutiliza el modelo ya analizado por su clave), pinta varios fotogramas
//! seguidos y la suelta, todo dentro del mismo hilo bloqueante.

use std::fmt;
use std::time::Duration;

use crate::{Frame, StickerError, tgs};

/// Lado fijo (px) al que se pintan los fotogramas animados (burbuja y selector).
pub const ANIM_PX: u32 = 256;
/// Tope de fotogramas por segundo que se muestran (y del reloj de la UI).
pub const MAX_FPS: f64 = 30.0;

/// Un TGS listo para animar: JSON Lottie descomprimido, clave de caché de rlottie y metadatos.
#[derive(Clone, PartialEq)]
pub struct Tgs {
    json: Vec<u8>,
    key: String,
    pub info: tgs::TgsInfo,
}

impl fmt::Debug for Tgs {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Tgs").field("key", &self.key).field("json_bytes", &self.json.len()).field("info", &self.info).finish()
    }
}

impl Tgs {
    /// Descomprime y valida un TGS y lee sus metadatos. `key` identifica el documento (rlottie la
    /// usa como clave de su caché de modelos: dos TGS distintos no deben compartirla).
    pub fn load(bytes: &[u8], key: &str) -> Result<Self, StickerError> {
        let json = tgs::decompress(bytes)?;
        if json.contains(&0) || key.is_empty() || key.as_bytes().contains(&0) {
            return Err(StickerError::NotLottie);
        }
        let info = info_of(&json, key)?;
        if info.frames == 0 || !info.fps.is_finite() || info.fps <= 0.0 {
            return Err(StickerError::NotLottie);
        }
        Ok(Self { json, key: key.to_owned(), info })
    }

    /// Bytes que ocupa en memoria (para la caché acotada).
    pub fn byte_len(&self) -> usize {
        self.json.len() + self.key.len()
    }

    /// Pinta un fotograma a `px`×`px` RGBA (sin premultiplicar).
    pub fn render(&self, frame: usize, px: u32) -> Result<Frame, StickerError> {
        self.render_many(&[frame], px).pop().map_or(Err(StickerError::LottieLoad), |(_, f)| f)
    }

    /// Pinta varios fotogramas con una sola instancia de la animación (en el orden dado).
    pub fn render_many(&self, frames: &[usize], px: u32) -> Vec<(usize, Result<Frame, StickerError>)> {
        if px == 0 || px > 2048 {
            return frames.iter().map(|&f| (f, Err(StickerError::BadSize))).collect();
        }
        render_batch(&self.json, &self.key, frames, px)
    }
}

#[cfg(feature = "lottie")]
fn info_of(json: &[u8], key: &str) -> Result<tgs::TgsInfo, StickerError> {
    let anim = rlottie::Animation::from_data(json.to_vec(), key, ".").ok_or(StickerError::LottieLoad)?;
    let size = anim.size();
    Ok(tgs::TgsInfo { frames: anim.totalframe(), fps: anim.framerate(), width: size.width, height: size.height })
}

#[cfg(not(feature = "lottie"))]
fn info_of(_json: &[u8], _key: &str) -> Result<tgs::TgsInfo, StickerError> {
    Err(StickerError::LottieDisabled)
}

#[cfg(feature = "lottie")]
fn render_batch(json: &[u8], key: &str, frames: &[usize], px: u32) -> Vec<(usize, Result<Frame, StickerError>)> {
    let Some(mut anim) = rlottie::Animation::from_data(json.to_vec(), key, ".") else {
        return frames.iter().map(|&f| (f, Err(StickerError::LottieLoad))).collect();
    };
    let total = anim.totalframe();
    let mut surface = rlottie::Surface::new(rlottie::Size::new(px as usize, px as usize));
    frames
        .iter()
        .map(|&f| {
            if f >= total {
                return (f, Err(StickerError::FrameOutOfRange(f)));
            }
            anim.render(f, &mut surface);
            (f, Ok(Frame { width: px, height: px, rgba: tgs::unpremultiply(surface.data()) }))
        })
        .collect()
}

#[cfg(not(feature = "lottie"))]
fn render_batch(_json: &[u8], _key: &str, frames: &[usize], _px: u32) -> Vec<(usize, Result<Frame, StickerError>)> {
    frames.iter().map(|&f| (f, Err(StickerError::LottieDisabled))).collect()
}

/// Cada cuántos fotogramas de la fuente se muestra uno para no pasar de [`MAX_FPS`] (60 fps → 2).
pub fn frame_step(info: &tgs::TgsInfo) -> usize {
    if !info.fps.is_finite() || info.fps <= MAX_FPS {
        1
    } else {
        (info.fps / MAX_FPS).ceil() as usize
    }
}

/// Fotograma que toca a los `elapsed` de reloj, en bucle. Solo devuelve múltiplos de
/// [`frame_step`]: el conjunto de fotogramas de un bucle es siempre el mismo (la caché acierta en
/// la segunda vuelta aunque la duración no sea múltiplo del tic).
pub fn frame_at(info: &tgs::TgsInfo, elapsed: Duration) -> usize {
    if info.frames == 0 || !info.fps.is_finite() || info.fps <= 0.0 {
        return 0;
    }
    let step = frame_step(info);
    let shown = info.frames.div_ceil(step);
    let idx = (elapsed.as_secs_f64() * info.fps / step as f64).floor() as u64;
    (idx % shown as u64) as usize * step
}

/// Los `n` fotogramas que siguen a `frame` en el bucle (sin incluirlo), para pintarlos por adelantado.
pub fn next_frames(info: &tgs::TgsInfo, frame: usize, n: usize) -> Vec<usize> {
    if info.frames == 0 {
        return Vec::new();
    }
    let step = frame_step(info);
    let shown = info.frames.div_ceil(step);
    let idx = frame / step;
    (1..=n.min(shown.saturating_sub(1))).map(|k| (idx + k) % shown * step).collect()
}

/// Estado del que depende si los stickers animados se mueven.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Playback {
    /// Movimiento reducido (`TC_REDUCED_MOTION` o el conmutador de la app): siempre primer fotograma.
    pub reduced_motion: bool,
    /// La ventana tiene el foco: sin él, la animación se congela donde esté.
    pub focused: bool,
}

/// Qué se dibuja para un sticker animado.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Show {
    /// Imagen estática (primer fotograma o miniatura): movimiento reducido o fuera de la vista.
    Still,
    /// Congelado en el último fotograma mostrado (ventana sin foco).
    Paused,
    /// Animándose: este fotograma.
    Frame(usize),
}

impl Playback {
    /// ¿Hace falta el reloj de animación? Solo con foco, sin movimiento reducido y con algo visible.
    pub fn ticks(&self, any_visible: bool) -> bool {
        any_visible && self.focused && !self.reduced_motion
    }

    /// Qué dibujar para un sticker `visible` a los `elapsed` del reloj.
    pub fn show(&self, info: &tgs::TgsInfo, visible: bool, elapsed: Duration) -> Show {
        if self.reduced_motion || !visible {
            Show::Still
        } else if !self.focused {
            Show::Paused
        } else {
            Show::Frame(frame_at(info, elapsed))
        }
    }
}
