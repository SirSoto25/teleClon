//! Fuente de aleatoriedad inyectable (sans-IO: el llamador decide de dónde sale).

use crate::error::{Result, SecretError};

/// Fuente de bytes aleatorios criptográficamente seguros.
pub trait Entropy {
    /// Rellena `buf` con bytes aleatorios.
    fn fill(&mut self, buf: &mut [u8]) -> Result<()>;
}

/// Generador del sistema operativo (`getrandom`). Es el que debe usarse en producción.
#[derive(Debug, Default, Clone, Copy)]
pub struct OsEntropy;

impl Entropy for OsEntropy {
    fn fill(&mut self, buf: &mut [u8]) -> Result<()> {
        getrandom::fill(buf).map_err(|_| SecretError::Entropy)
    }
}

impl<E: Entropy + ?Sized> Entropy for &mut E {
    fn fill(&mut self, buf: &mut [u8]) -> Result<()> {
        (**self).fill(buf)
    }
}

pub(crate) fn random_i64(e: &mut dyn Entropy) -> Result<i64> {
    let mut b = [0u8; 8];
    e.fill(&mut b)?;
    Ok(i64::from_le_bytes(b))
}

/// Entero uniforme en `0..=max` (rechazo, sin sesgo).
pub(crate) fn random_up_to(e: &mut dyn Entropy, max: u32) -> Result<u32> {
    if max == 0 {
        return Ok(0);
    }
    let range = u64::from(max) + 1;
    let zone = u64::MAX - (u64::MAX % range);
    loop {
        let mut b = [0u8; 8];
        e.fill(&mut b)?;
        let v = u64::from_le_bytes(b);
        if v < zone {
            return Ok((v % range) as u32);
        }
    }
}
