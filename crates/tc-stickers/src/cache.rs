//! Caché LRU en memoria de fotogramas decodificados, con tope en bytes (sobre [`ByteLru`]).

use std::sync::Arc;

use crate::{Frame, lru::ByteLru};

/// 64 MiB por defecto (architecture.md §8).
pub const DEFAULT_CACHE_BYTES: usize = 64 * 1024 * 1024;

/// Clave `(sticker_id, tamaño, fotograma)`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct CacheKey {
    pub sticker_id: i64,
    pub size: u32,
    pub frame: u32,
}

#[derive(Debug)]
pub struct FrameCache {
    lru: ByteLru<CacheKey, Arc<Frame>>,
}

impl Default for FrameCache {
    fn default() -> Self {
        Self::new(DEFAULT_CACHE_BYTES)
    }
}

impl FrameCache {
    pub fn new(max_bytes: usize) -> Self {
        Self { lru: ByteLru::new(max_bytes) }
    }

    pub fn len(&self) -> usize {
        self.lru.len()
    }

    pub fn is_empty(&self) -> bool {
        self.lru.is_empty()
    }

    pub fn used_bytes(&self) -> usize {
        self.lru.used_bytes()
    }

    pub fn max_bytes(&self) -> usize {
        self.lru.max_bytes()
    }

    /// Devuelve el fotograma y lo marca como usado recientemente.
    pub fn get(&mut self, key: &CacheKey) -> Option<Arc<Frame>> {
        self.lru.get(key).cloned()
    }

    pub fn contains(&self, key: &CacheKey) -> bool {
        self.lru.contains(key)
    }

    /// Inserta (reemplaza si existía) y expulsa los menos recientes hasta caber.
    /// Un fotograma mayor que el tope entero no se guarda (se devuelve igualmente el `Arc`).
    pub fn insert(&mut self, key: CacheKey, frame: Frame) -> Arc<Frame> {
        let arc = Arc::new(frame);
        let len = arc.byte_len();
        self.lru.insert(key, arc.clone(), len);
        arc
    }

    pub fn remove(&mut self, key: &CacheKey) -> Option<Arc<Frame>> {
        self.lru.remove(key)
    }

    /// Obtiene de la caché o calcula con `make` y guarda.
    pub fn get_or_insert_with<E>(
        &mut self,
        key: CacheKey,
        make: impl FnOnce() -> Result<Frame, E>,
    ) -> Result<Arc<Frame>, E> {
        if let Some(f) = self.get(&key) {
            return Ok(f);
        }
        Ok(self.insert(key, make()?))
    }

    pub fn clear(&mut self) {
        self.lru.clear();
    }
}
