//! Utilidades de prueba (feature `testing`): entropía determinista y un primo
//! seguro público de 2048 bits. **Nada de esto es para producción.**

use sha2::{Digest, Sha256};

use crate::dh::DhParams;
use crate::entropy::Entropy;
use crate::error::Result;

/// Primo seguro MODP de 2048 bits del RFC 3526 (grupo 14), en hexadecimal.
/// `p ≡ 7 (mod 8)`, así que `g = 2` es válido. Es un valor público y estándar; los
/// parámetros reales los da siempre el servidor con `messages.getDhConfig`.
pub const RFC3526_2048_P_HEX: &str = concat!(
    "FFFFFFFFFFFFFFFFC90FDAA22168C234C4C6628B80DC1CD129024E088A67CC74020BBEA63B139B22514A08798E3404DD",
    "EF9519B3CD3A431B302B0A6DF25F14374FE1356D6D51C245E485B576625E7EC6F44C42E9A637ED6B0BFF5CB6F406B7ED",
    "EE386BFB5A899FA5AE9F24117C4B1FE649286651ECE45B3DC2007CB8A163BF0598DA48361C55D39A69163FA8FD24CF5F",
    "83655D23DCA3AD961C62F356208552BB9ED529077096966D670C354E4ABC9804F1746C08CA18217C32905E462E36CE3B",
    "E39E772C180E86039B2783A2EC07A28FB5C55DF06F4C52C9DE2BCBF6955817183995497CEA956AE515D2261898FA0510",
    "15728E5A8AACAA68FFFFFFFFFFFFFFFF",
);

/// Decodifica hexadecimal (pruebas).
pub fn hex(s: &str) -> Vec<u8> {
    let s: Vec<u8> = s.bytes().filter(|c| !c.is_ascii_whitespace()).collect();
    s.chunks(2)
        .map(|p| u8::from_str_radix(core::str::from_utf8(p).expect("ascii"), 16).expect("hex válido"))
        .collect()
}

/// `p` del RFC 3526 en bytes.
pub fn rfc3526_p() -> Vec<u8> {
    hex(RFC3526_2048_P_HEX)
}

/// Valida y devuelve los parámetros RFC 3526 con `g = 2` (tarda: cachear).
pub fn rfc3526_params() -> DhParams {
    DhParams::validate(2, &rfc3526_p(), 1, &mut SeededEntropy::new(b"mr")).expect("RFC 3526 es un primo seguro")
}

/// Generador determinista: SHA-256 en modo contador sobre una semilla. **No es
/// seguro**: quien conozca la semilla reproduce todas las claves.
#[derive(Debug, Clone)]
pub struct SeededEntropy {
    seed: [u8; 32],
    counter: u64,
}

impl SeededEntropy {
    /// Crea el generador a partir de una semilla arbitraria.
    pub fn new(seed: &[u8]) -> Self {
        Self { seed: Sha256::digest(seed).into(), counter: 0 }
    }
}

impl Entropy for SeededEntropy {
    fn fill(&mut self, buf: &mut [u8]) -> Result<()> {
        for chunk in buf.chunks_mut(32) {
            let mut h = Sha256::new();
            h.update(self.seed);
            h.update(self.counter.to_le_bytes());
            self.counter += 1;
            let out = h.finalize();
            chunk.copy_from_slice(&out[..chunk.len()]);
        }
        Ok(())
    }
}
