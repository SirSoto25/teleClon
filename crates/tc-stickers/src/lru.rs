//! LRU genérica acotada en bytes (cada entrada declara su peso al insertarse).

use std::collections::{BTreeMap, HashMap};
use std::hash::Hash;

#[derive(Debug, Clone)]
pub struct ByteLru<K, V> {
    max_bytes: usize,
    used: usize,
    tick: u64,
    map: HashMap<K, (V, usize, u64)>,
    order: BTreeMap<u64, K>,
}

impl<K: Copy + Eq + Hash, V> ByteLru<K, V> {
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

    pub fn contains(&self, key: &K) -> bool {
        self.map.contains_key(key)
    }

    /// Valor sin tocar el orden de uso.
    pub fn peek(&self, key: &K) -> Option<&V> {
        self.map.get(key).map(|(v, _, _)| v)
    }

    /// Valor, marcándolo como usado recientemente.
    pub fn get(&mut self, key: &K) -> Option<&V> {
        self.tick += 1;
        let tick = self.tick;
        let (v, _, t) = self.map.get_mut(key)?;
        self.order.remove(t);
        *t = tick;
        self.order.insert(tick, *key);
        Some(v)
    }

    /// Inserta (reemplaza si existía) y expulsa los menos recientes hasta caber. Una entrada mayor
    /// que el tope entero no se guarda: devuelve `false`.
    pub fn insert(&mut self, key: K, value: V, bytes: usize) -> bool {
        self.remove(&key);
        if bytes > self.max_bytes {
            return false;
        }
        while self.used + bytes > self.max_bytes {
            let Some((_, oldest)) = self.order.pop_first() else { break };
            if let Some((_, b, _)) = self.map.remove(&oldest) {
                self.used -= b;
            }
        }
        self.tick += 1;
        self.order.insert(self.tick, key);
        self.map.insert(key, (value, bytes, self.tick));
        self.used += bytes;
        true
    }

    pub fn remove(&mut self, key: &K) -> Option<V> {
        let (v, b, t) = self.map.remove(key)?;
        self.order.remove(&t);
        self.used -= b;
        Some(v)
    }

    /// Quita las entradas para las que `keep` devuelve `false`.
    pub fn retain(&mut self, mut keep: impl FnMut(&K) -> bool) {
        let gone: Vec<K> = self.map.keys().filter(|k| !keep(k)).copied().collect();
        for k in gone {
            self.remove(&k);
        }
    }

    pub fn clear(&mut self) {
        self.map.clear();
        self.order.clear();
        self.used = 0;
    }
}
