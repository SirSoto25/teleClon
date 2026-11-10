//! Audio de Parlo (ADR-0007).
//!
//! * [`waveform`] (de tc-core): forma de onda empaquetada a 5 bits de Telegram.
//! * [`decode`]: decodificado en streaming a muestras `f32` intercaladas. OGG/Opus (notas de voz) con
//!   un demultiplexor Ogg propio y `opus-decoder` (Rust puro); MP3, M4A/AAC, FLAC, WAV y OGG/Vorbis con
//!   symphonia.
//! * [`player`]: reproductor con un único audio a la vez. Las órdenes van a un hilo propio que abre la
//!   salida (rodio/cpal) al primer uso; la UI nunca se bloquea y, sin dispositivo de salida, recibe un
//!   error en el estado (sin pánico).
pub mod decode;
mod ogg;
pub mod player;
pub use tc_core::waveform;

pub use decode::{AudioStream, Pcm, decode_all, open};
pub use player::{NullOutput, Output, PlaybackState, Player, SPEEDS, Status};

/// Error de audio. Los textos son para el usuario.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum AudioError {
    /// No se pudo abrir la salida de audio (no hay dispositivo o el sistema la rechazó).
    #[error("No hay salida de audio disponible ({0}).")]
    NoDevice(String),
    /// Formato o códec que no sabemos decodificar.
    #[error("Formato de audio no compatible.")]
    Unsupported,
    /// Archivo dañado o truncado.
    #[error("El audio está dañado: {0}.")]
    Corrupt(String),
}
