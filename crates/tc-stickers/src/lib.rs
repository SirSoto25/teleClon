//! Decodificación de stickers a fotogramas RGBA para la UI: WebP estático (crate `image`),
//! TGS = Lottie comprimido con gzip (`flate2`; render con `rlottie` tras la feature `lottie`),
//! caché LRU por bytes, recientes/favoritos y un pack propio original generado por código.
//! También la animación de TGS (`anim`: fotograma por tiempo a 30 fps como mucho, política de movimiento y
//! render por lotes). WebM (vídeo VP9 con alfa) queda estático: ver ADR-0008 y architecture.md §8.
#![forbid(unsafe_code)]

pub mod anim;
pub mod cache;
pub mod error;
pub mod frame;
pub mod lru;
pub mod pack;
pub mod recents;
pub mod tgs;
pub mod webp;

pub use cache::{CacheKey, FrameCache, DEFAULT_CACHE_BYTES};
pub use error::StickerError;
pub use frame::Frame;
pub use lru::ByteLru;
pub use recents::{Favorites, Recents};

#[cfg(test)]
mod tests;
#[cfg(test)]
mod anim_tests;
