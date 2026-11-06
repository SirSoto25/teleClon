//! Recientes (más nuevo primero, con tope) y favoritos (orden de adición) de stickers.

use tc_core::StickerId;

pub const DEFAULT_RECENTS: usize = 30;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Recents {
    cap: usize,
    ids: Vec<StickerId>,
}

impl Default for Recents {
    fn default() -> Self {
        Self::new(DEFAULT_RECENTS)
    }
}

impl Recents {
    pub fn new(cap: usize) -> Self {
        Self { cap, ids: Vec::new() }
    }

    /// Restaura desde una lista persistida (más nuevo primero), sin duplicados y recortada.
    pub fn from_ids(cap: usize, ids: impl IntoIterator<Item = StickerId>) -> Self {
        let mut r = Self::new(cap);
        for id in ids {
            if !r.ids.contains(&id) && r.ids.len() < cap {
                r.ids.push(id);
            }
        }
        r
    }

    /// Registra un uso: sube al principio; expulsa el más viejo si excede el tope.
    pub fn record(&mut self, id: StickerId) {
        self.ids.retain(|i| *i != id);
        self.ids.insert(0, id);
        self.ids.truncate(self.cap);
    }

    pub fn remove(&mut self, id: StickerId) {
        self.ids.retain(|i| *i != id);
    }

    pub fn clear(&mut self) {
        self.ids.clear();
    }

    pub fn ids(&self) -> &[StickerId] {
        &self.ids
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Favorites {
    ids: Vec<StickerId>,
}

impl Favorites {
    pub fn from_ids(ids: impl IntoIterator<Item = StickerId>) -> Self {
        let mut f = Self::default();
        for id in ids {
            if !f.ids.contains(&id) {
                f.ids.push(id);
            }
        }
        f
    }

    pub fn contains(&self, id: StickerId) -> bool {
        self.ids.contains(&id)
    }

    /// Alterna; devuelve `true` si queda como favorito.
    pub fn toggle(&mut self, id: StickerId) -> bool {
        if let Some(pos) = self.ids.iter().position(|i| *i == id) {
            self.ids.remove(pos);
            false
        } else {
            self.ids.push(id);
            true
        }
    }

    pub fn ids(&self) -> &[StickerId] {
        &self.ids
    }
}
