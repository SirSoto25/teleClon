//! Vídeo de Parlo (ADR-0010): los «GIF» de Telegram, que son MP4/H.264 sin sonido
//! (`documentAttributeAnimated`), a fotogramas RGBA para animarlos en bucle.
//!
//! * [`mp4`]: demultiplexor MP4 propio, mínimo y acotado (primera pista `avc1`/`avc3`).
//! * [`plan`]: cuántos fotogramas y a qué tamaño para caber en el presupuesto de memoria.
//! * [`decode_mp4`]: H.264 con OpenH264 (compilado desde las fuentes del crate `openh264`).
//!
//! Todo es síncrono y pensado para el pool bloqueante: la UI lo llama fuera de su hilo.
pub mod decode;
pub mod mp4;
pub mod plan;
#[cfg(test)]
mod testmux;

pub use decode::{Clip, Frame, decode_mp4};
pub use mp4::looks_like_mp4;
pub use plan::{Limits, Plan};

/// Error de vídeo. Los textos son para registro (la UI cae a la miniatura estática).
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum VideoError {
    /// El contenedor no es un MP4 que sepamos leer.
    #[error("MP4 no válido: {0}")]
    Container(String),
    /// El MP4 no tiene pista de vídeo.
    #[error("el MP4 no tiene vídeo")]
    NoVideo,
    /// Hay vídeo, pero no H.264 (p. ej. `hvc1`).
    #[error("códec de vídeo no compatible: {0}")]
    Unsupported(String),
    /// OpenH264 no pudo decodificar ningún fotograma.
    #[error("no se pudo decodificar el vídeo: {0}")]
    Decode(String),
}
