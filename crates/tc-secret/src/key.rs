//! Clave compartida de 2048 bits y su huella (`key_fingerprint`).

use core::fmt;

use sha1::{Digest, Sha1};
use subtle::{Choice, ConstantTimeEq};
use zeroize::{Zeroize, ZeroizeOnDrop};

/// Longitud de la clave compartida: siempre 256 bytes (rellenada con ceros a la izquierda).
pub const KEY_LEN: usize = 256;

/// Clave compartida de un chat secreto. Se borra de memoria al soltarse y no se
/// imprime nunca (su `Debug` está censurado).
#[derive(Clone, Zeroize, ZeroizeOnDrop)]
pub struct SecretKey {
    bytes: [u8; KEY_LEN],
}

impl SecretKey {
    /// Construye la clave desde sus 256 bytes (p. ej. al recuperarla del keyring).
    pub fn from_bytes(bytes: [u8; KEY_LEN]) -> Self {
        Self { bytes }
    }

    /// Bytes de la clave. Usar solo para guardarla en almacenamiento protegido.
    pub fn as_bytes(&self) -> &[u8; KEY_LEN] {
        &self.bytes
    }

    /// `key_fingerprint`: los 64 bits de menor orden de SHA1(clave), es decir los
    /// bytes 12..20 del resumen leídos como `long` TL (little-endian, con signo).
    pub fn fingerprint(&self) -> i64 {
        fingerprint_of(&self.bytes)
    }
}

pub(crate) fn fingerprint_of(key: &[u8; KEY_LEN]) -> i64 {
    let h = Sha1::digest(key);
    let mut low = [0u8; 8];
    low.copy_from_slice(&h[12..20]);
    i64::from_le_bytes(low)
}

impl ConstantTimeEq for SecretKey {
    fn ct_eq(&self, other: &Self) -> Choice {
        self.bytes.ct_eq(&other.bytes)
    }
}

impl fmt::Debug for SecretKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("SecretKey(<censurada>)")
    }
}
