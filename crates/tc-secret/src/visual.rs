//! Huella visual de la clave ("key visualization").
//!
//! Según la especificación: los primeros 128 bits de SHA1(clave inicial) seguidos de
//! los primeros 160 bits de SHA256(clave en uso cuando el chat pasó a la capa 46).
//! Si el chat nace en capa ≥ 46 ambas claves son la misma. Total: 288 bits = 36 bytes.
//!
//! Lo que NO fija la especificación y aquí es convención propia de teleClon (no
//! interoperable visualmente con otros clientes; el texto hexadecimal sí lo es):
//! - la rejilla: 12×12 celdas de 2 bits, recorriendo los bytes en orden y cada byte
//!   desde sus 2 bits más altos;
//! - la secuencia de emoji (tabla propia de 64 símbolos).

use sha1::{Digest as _, Sha1};
use sha2::Sha256;

use crate::key::SecretKey;

/// Bytes de la visualización.
pub const VISUAL_LEN: usize = 36;
/// Lado de la rejilla.
pub const GRID_SIDE: usize = 12;

/// Tabla propia (no tomada de ningún cliente) de 64 emoji bien distinguibles.
const EMOJI: [&str; 64] = [
    "🦊", "🐢", "🐙", "🦉", "🐝", "🦋", "🐳", "🦀", "🐌", "🦔", "🦒", "🦓", "🐘", "🦩", "🐧", "🦜",
    "🌵", "🌻", "🍄", "🌲", "🍁", "🌙", "⭐", "🌈", "🔥", "💧", "❄", "⚡", "🌋", "🏔", "🌊", "☂",
    "🍋", "🍉", "🍒", "🍇", "🥕", "🌽", "🥨", "🧀", "🍩", "🎂", "☕", "🍯", "🥥", "🌶", "🍐", "🥑",
    "🎈", "🎸", "🎲", "🧩", "🔑", "⚓", "🚲", "⛵", "🚂", "🛸", "🪁", "🧭", "🔔", "📚", "🧲", "🪐",
];

/// Visualización de la clave de un chat secreto.
#[derive(Clone, PartialEq, Eq)]
pub struct KeyVisualization {
    bytes: [u8; VISUAL_LEN],
}

impl core::fmt::Debug for KeyVisualization {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "KeyVisualization({})", self.hex())
    }
}

impl KeyVisualization {
    /// Desde la clave inicial y la clave vigente al pasar a la capa 46.
    pub fn new(initial_key: &SecretKey, layer46_key: &SecretKey) -> Self {
        let sha1 = Sha1::digest(initial_key.as_bytes());
        let sha256 = Sha256::digest(layer46_key.as_bytes());
        let mut bytes = [0u8; VISUAL_LEN];
        bytes[..16].copy_from_slice(&sha1[..16]);
        bytes[16..].copy_from_slice(&sha256[..20]);
        Self { bytes }
    }

    /// Desde los 36 bytes ya calculados (p. ej. guardados junto al chat).
    pub fn from_bytes(bytes: [u8; VISUAL_LEN]) -> Self {
        Self { bytes }
    }

    /// Los 36 bytes.
    pub fn bytes(&self) -> &[u8; VISUAL_LEN] {
        &self.bytes
    }

    /// Rejilla 12×12 de valores 0..=3 (convención propia, ver módulo).
    pub fn grid(&self) -> [[u8; GRID_SIDE]; GRID_SIDE] {
        let mut g = [[0u8; GRID_SIDE]; GRID_SIDE];
        for i in 0..GRID_SIDE * GRID_SIDE {
            let byte = self.bytes[i / 4];
            let shift = 6 - 2 * (i % 4);
            g[i / GRID_SIDE][i % GRID_SIDE] = (byte >> shift) & 0b11;
        }
        g
    }

    /// Hexadecimal en minúsculas, 9 grupos de 4 bytes separados por espacios.
    pub fn hex(&self) -> String {
        let mut s = String::with_capacity(VISUAL_LEN * 2 + 8);
        for (i, b) in self.bytes.iter().enumerate() {
            if i > 0 && i % 4 == 0 {
                s.push(' ');
            }
            s.push_str(&format!("{b:02x}"));
        }
        s
    }

    /// Cuatro emoji derivados de los bytes 0..32 (8 bytes cada uno, módulo 64).
    pub fn emoji(&self) -> [&'static str; 4] {
        core::array::from_fn(|i| {
            let mut chunk = [0u8; 8];
            chunk.copy_from_slice(&self.bytes[i * 8..i * 8 + 8]);
            EMOJI[(u64::from_be_bytes(chunk) % 64) as usize]
        })
    }
}
