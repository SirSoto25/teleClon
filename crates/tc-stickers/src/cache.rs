//! Caché LRU en memoria de fotogramas decodificados, con tope en bytes.

use std::{
    collections::{BTreeMap, HashMap},
    sync::Arc,
};

use crate::Frame;

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
    max_bytes: usize,
    used: usize,
    tick: u64,
    map: HashMap<CacheKey, (Arc<Frame>, u64)>,
    order: BTreeMap<u64, CacheKey>,
}

impl Default for FrameCache {
    fn default() -> Self {
        Self::new(DEFAULT_CACHE_BYTES)
    }
}

impl FrameCache {
    pub fn new(max_bytes: usize) -> Self {
        Self { max_bytes, used: 0, tick: 0, map: HashMap::new(), order: BTreeMap::new() }
    }

    pub fn len(&self) -> usize {
        self.map.len()
    }

    pub fn is_empty(&self) -> bool {
        self.map.is_empty()
    }

    pub fn used_bytes(&self) -> usize {
        self.used
    }

    pub fn max_bytes(&self) -> usize {
        self.max_bytes
    }

    fn touch(&mut self, key: CacheKey) {
        self.tick += 1;
        if let Some((_, t)) = self.map.get_mut(&key) {
            self.order.remove(t);
            *t = self.tick;
            self.order.insert(self.tick, key);
        }
    }

    /// Devuelve el fotograma y lo marca como usado recientemente.
    pub fn get(&mut self, key: &CacheKey) -> Option<Arc<Frame>> {
        let frame = self.map.get(key)?.0.clone();
        self.touch(*key);
        Some(frame)
    }

    pub fn contains(&self, key: &CacheKey) -> bool {
        self.map.contains_key(key)
    }

    /// Inserta (reemplaza si existía) y expulsa los menos recientes hasta caber.
    /// Un fotograma mayor que el tope entero no se guarda (se devuelve igualmente el `Arc`).
    pub fn insert(&mut self, key: CacheKey, frame: Frame) -> Arc<Frame> {
        self.remove(&key);
        let arc = Arc::new(frame);
        let len = arc.byte_len();
        if len > self.max_bytes {
            return arc;
        }
        while self.used + len > self.max_bytes {
            let Some((_, oldest)) = self.order.pop_first() else { break };
            if let Some((f, _)) = self.map.remove(&oldest) {
                self.used -= f.byte_len();
            }
        }
        self.tick += 1;
        self.order.insert(self.tick, key);
        self.map.insert(key, (arc.clone(), self.tick));
        self.used += len;
        arc
    }

    pub fn remove(&mut self, key: &CacheKey) -> Option<Arc<Frame>> {
        let (f, t) = self.map.remove(key)?;
        self.order.remove(&t);
        self.used -= f.byte_len();
        Some(f)
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
        self.map.clear();
        self.order.clear();
        self.used = 0;
    }
}
