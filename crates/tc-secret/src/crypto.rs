//! Cifrado de mensajes de chats secretos con MTProto 2.0.
//!
//! Formato en el cable: `key_fingerprint (8, LE) || msg_key (16) || AES-256-IGE(datos)`,
//! con `datos = longitud (4, LE) || carga TL || relleno aleatorio (12..=1024)` y
//! `|datos| % 16 == 0`.
//!
//! Derivación (x = 0 si el mensaje lo envía quien creó el chat, x = 8 si lo envía el otro):
//! - `msg_key_large = SHA256(key[88+x .. 88+x+32] || datos)`; `msg_key = msg_key_large[8..24]`
//! - `a = SHA256(msg_key || key[x .. x+36])`, `b = SHA256(key[40+x .. 40+x+36] || msg_key)`
//! - `aes_key = a[0..8] || b[8..24] || a[24..32]`, `aes_iv = b[0..8] || a[8..24] || b[24..32]`

use aes::Aes256;
use aes::cipher::{Block, BlockModeDecrypt, BlockModeEncrypt, KeyIvInit};
use sha2::{Digest, Sha256};
use subtle::ConstantTimeEq;
use zeroize::Zeroizing;

use crate::entropy::{Entropy, random_up_to};
use crate::error::{Result, SecretError};
use crate::key::SecretKey;

type IgeEnc = ige::Encryptor<Aes256>;
type IgeDec = ige::Decryptor<Aes256>;

/// Relleno mínimo de MTProto 2.0.
pub const MIN_PADDING: usize = 12;
/// Relleno máximo de MTProto 2.0.
pub const MAX_PADDING: usize = 1024;
/// Bloques de 16 bytes extra (como máximo) que se añaden al azar al relleno mínimo
/// para ocultar la longitud. La especificación permite hasta 1024 bytes; aquí se
/// limita a 7 bloques para no inflar el tráfico.
pub const MAX_EXTRA_PADDING_BLOCKS: u32 = 7;
/// Cabecera: `key_fingerprint` + `msg_key`.
pub const HEADER_LEN: usize = 8 + 16;

/// Quién creó el chat secreto. Determina la `x` de la derivación y la paridad de
/// los números de secuencia.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Role {
    /// Creador del chat (quien llamó a `messages.requestEncryption`).
    Initiator,
    /// Quien aceptó la solicitud.
    Responder,
}

impl Role {
    /// El otro extremo.
    pub fn peer(self) -> Self {
        match self {
            Role::Initiator => Role::Responder,
            Role::Responder => Role::Initiator,
        }
    }

    /// `x` de la derivación para mensajes ENVIADOS por este rol.
    pub fn kdf_x(self) -> usize {
        match self {
            Role::Initiator => 0,
            Role::Responder => 8,
        }
    }
}

fn msg_key_for(key: &SecretKey, x: usize, data: &[u8]) -> [u8; 16] {
    let k = key.as_bytes();
    let mut h = Sha256::new();
    h.update(&k[88 + x..88 + x + 32]);
    h.update(data);
    let large = h.finalize();
    let mut mk = [0u8; 16];
    mk.copy_from_slice(&large[8..24]);
    mk
}

fn derive(key: &SecretKey, msg_key: &[u8; 16], x: usize) -> (Zeroizing<[u8; 32]>, Zeroizing<[u8; 32]>) {
    let k = key.as_bytes();
    let mut ha = Sha256::new();
    ha.update(msg_key);
    ha.update(&k[x..x + 36]);
    let a = Zeroizing::new(<[u8; 32]>::from(ha.finalize()));
    let mut hb = Sha256::new();
    hb.update(&k[40 + x..40 + x + 36]);
    hb.update(msg_key);
    let b = Zeroizing::new(<[u8; 32]>::from(hb.finalize()));
    let mut aes_key = Zeroizing::new([0u8; 32]);
    aes_key[..8].copy_from_slice(&a[..8]);
    aes_key[8..24].copy_from_slice(&b[8..24]);
    aes_key[24..].copy_from_slice(&a[24..]);
    let mut aes_iv = Zeroizing::new([0u8; 32]);
    aes_iv[..8].copy_from_slice(&b[..8]);
    aes_iv[8..24].copy_from_slice(&a[8..24]);
    aes_iv[24..].copy_from_slice(&b[24..]);
    (aes_key, aes_iv)
}

fn ige_encrypt(key: &[u8; 32], iv: &[u8; 32], buf: &mut [u8]) {
    let mut enc = IgeEnc::new(key.into(), iv.into());
    for chunk in buf.as_chunks_mut::<16>().0 {
        let block: &mut Block<IgeEnc> = chunk.into();
        enc.encrypt_block(block);
    }
}

fn ige_decrypt(key: &[u8; 32], iv: &[u8; 32], buf: &mut [u8]) {
    let mut dec = IgeDec::new(key.into(), iv.into());
    for chunk in buf.as_chunks_mut::<16>().0 {
        let block: &mut Block<IgeDec> = chunk.into();
        dec.decrypt_block(block);
    }
}

/// Longitud de relleno válida para una carga de `payload_len` bytes con `extra_blocks`
/// bloques adicionales.
pub fn padding_len(payload_len: usize, extra_blocks: u32) -> usize {
    let mut pad = (16 - (4 + payload_len) % 16) % 16;
    if pad < MIN_PADDING {
        pad += 16;
    }
    pad + 16 * extra_blocks as usize
}

/// Cifra una carga TL ya serializada (normalmente un `decryptedMessageLayer`).
/// `sender` es el rol de quien envía (nosotros).
pub fn encrypt(key: &SecretKey, sender: Role, payload: &[u8], entropy: &mut dyn Entropy) -> Result<Vec<u8>> {
    let extra = random_up_to(entropy, MAX_EXTRA_PADDING_BLOCKS)?;
    let mut padding = Zeroizing::new(vec![0u8; padding_len(payload.len(), extra)]);
    entropy.fill(&mut padding)?;
    encrypt_with_padding(key, sender, payload, &padding)
}

/// Igual que [`encrypt`] pero con un relleno dado (vectores de prueba).
pub fn encrypt_with_padding(key: &SecretKey, sender: Role, payload: &[u8], padding: &[u8]) -> Result<Vec<u8>> {
    if !payload.len().is_multiple_of(4) {
        return Err(SecretError::Malformed("la carga TL no está alineada a 4 bytes"));
    }
    let len = u32::try_from(payload.len()).map_err(|_| SecretError::Malformed("carga demasiado grande"))?;
    if !(MIN_PADDING..=MAX_PADDING).contains(&padding.len()) || !(4 + payload.len() + padding.len()).is_multiple_of(16) {
        return Err(SecretError::Malformed("relleno inválido"));
    }
    let mut data = Zeroizing::new(Vec::with_capacity(4 + payload.len() + padding.len()));
    data.extend_from_slice(&len.to_le_bytes());
    data.extend_from_slice(payload);
    data.extend_from_slice(padding);

    let x = sender.kdf_x();
    let msg_key = msg_key_for(key, x, &data);
    let (aes_key, aes_iv) = derive(key, &msg_key, x);
    ige_encrypt(&aes_key, &aes_iv, &mut data);

    let mut out = Vec::with_capacity(HEADER_LEN + data.len());
    out.extend_from_slice(&key.fingerprint().to_le_bytes());
    out.extend_from_slice(&msg_key);
    out.extend_from_slice(&data);
    Ok(out)
}

/// Lee la `key_fingerprint` de un mensaje cifrado sin descifrarlo.
pub fn peek_fingerprint(data: &[u8]) -> Result<i64> {
    let fp: [u8; 8] = data
        .get(..8)
        .ok_or(SecretError::Malformed("mensaje demasiado corto"))?
        .try_into()
        .map_err(|_| SecretError::Malformed("mensaje demasiado corto"))?;
    Ok(i64::from_le_bytes(fp))
}

/// Descifra y autentica un mensaje. `sender` es el rol de quien lo envió (el otro extremo).
/// Devuelve la carga TL (sin prefijo de longitud ni relleno).
pub fn decrypt(key: &SecretKey, sender: Role, data: &[u8]) -> Result<Zeroizing<Vec<u8>>> {
    if data.len() < HEADER_LEN + 16 || !(data.len() - HEADER_LEN).is_multiple_of(16) {
        return Err(SecretError::Malformed("longitud de mensaje cifrado inválida"));
    }
    if peek_fingerprint(data)? != key.fingerprint() {
        return Err(SecretError::UnknownKeyFingerprint);
    }
    let mut msg_key = [0u8; 16];
    msg_key.copy_from_slice(&data[8..24]);
    let x = sender.kdf_x();
    let (aes_key, aes_iv) = derive(key, &msg_key, x);
    let mut plain = Zeroizing::new(data[HEADER_LEN..].to_vec());
    ige_decrypt(&aes_key, &aes_iv, &mut plain);

    // Autenticación ANTES de mirar ningún campo descifrado.
    let expected = msg_key_for(key, x, &plain);
    if !bool::from(expected.ct_eq(&msg_key)) {
        return Err(SecretError::MsgKeyMismatch);
    }
    let len = u32::from_le_bytes([plain[0], plain[1], plain[2], plain[3]]) as usize;
    if !len.is_multiple_of(4) || len > plain.len() - 4 {
        return Err(SecretError::Malformed("prefijo de longitud inválido"));
    }
    let pad = plain.len() - 4 - len;
    if !(MIN_PADDING..=MAX_PADDING).contains(&pad) {
        return Err(SecretError::Malformed("relleno fuera de 12..=1024"));
    }
    Ok(Zeroizing::new(plain[4..4 + len].to_vec()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::{SeededEntropy, hex};

    fn key() -> SecretKey {
        let mut k = [0u8; 256];
        k.copy_from_slice(&hex(crate::dh::tests::KEY));
        SecretKey::from_bytes(k)
    }

    // Generados con Python + pycryptodome (AES-ECB y un IGE escrito aparte,
    // convenio iv = y0 || x0): carga = bytes 0..32, relleno = (7·i + 3) mod 256, 28 bytes.
    const WIRE_X0: &str = "4f973482894a38d1cbc7bc2cfbba5cb0695922f0764874d73d86c0be2311621b1a59dea2106f7f5c87ec588ebbb6b40bd1224b2a2b93b6e90ba2c37a4cee09964da06e5ecbe79d9172b202849b38e446495b3d889007e64a";
    const WIRE_X8: &str = "4f973482894a38d13b45ac43008a685b27dd2c879741ff0606e4be4cc231f56bc640eac3520819dc0208e6121178188c98f89e0edbc436b8f0802497a88ae083875767bd92133987cced49c78c233427ec183dcfc4ab1aeb";

    fn vector_inputs() -> (Vec<u8>, Vec<u8>) {
        ((0u8..32).collect(), (0..28).map(|i: u32| ((i * 7 + 3) & 0xff) as u8).collect())
    }

    #[test]
    fn matches_independent_vectors_both_directions() {
        let (payload, padding) = vector_inputs();
        let k = key();
        assert_eq!(encrypt_with_padding(&k, Role::Initiator, &payload, &padding).unwrap(), hex(WIRE_X0));
        assert_eq!(encrypt_with_padding(&k, Role::Responder, &payload, &padding).unwrap(), hex(WIRE_X8));
        assert_eq!(decrypt(&k, Role::Initiator, &hex(WIRE_X0)).unwrap().as_slice(), payload.as_slice());
        assert_eq!(decrypt(&k, Role::Responder, &hex(WIRE_X8)).unwrap().as_slice(), payload.as_slice());
    }

    #[test]
    fn wrong_direction_is_rejected() {
        let k = key();
        assert_eq!(decrypt(&k, Role::Responder, &hex(WIRE_X0)).unwrap_err(), SecretError::MsgKeyMismatch);
    }

    #[test]
    fn padding_always_valid() {
        let mut e = SeededEntropy::new(b"pad");
        let k = key();
        for len in (0..400).step_by(4) {
            let payload = vec![0x5a; len];
            let wire = encrypt(&k, Role::Initiator, &payload, &mut e).unwrap();
            let body = wire.len() - HEADER_LEN;
            assert!(body.is_multiple_of(16));
            let pad = body - 4 - len;
            assert!((MIN_PADDING..=MAX_PADDING).contains(&pad), "len={len} pad={pad}");
            assert_eq!(decrypt(&k, Role::Initiator, &wire).unwrap().as_slice(), payload.as_slice());
        }
        for extra in 0..=MAX_EXTRA_PADDING_BLOCKS {
            for len in [0usize, 4, 12, 16, 100] {
                let p = padding_len(len, extra);
                assert!((MIN_PADDING..=MAX_PADDING).contains(&p) && (4 + len + p).is_multiple_of(16));
            }
        }
    }

    #[test]
    fn tampering_is_detected() {
        let (payload, padding) = vector_inputs();
        let k = key();
        let wire = encrypt_with_padding(&k, Role::Initiator, &payload, &padding).unwrap();
        for i in 8..wire.len() {
            let mut t = wire.clone();
            t[i] ^= 0x01;
            assert_eq!(decrypt(&k, Role::Initiator, &t).unwrap_err(), SecretError::MsgKeyMismatch, "byte {i}");
        }
        let mut t = wire.clone();
        t[0] ^= 1;
        assert_eq!(decrypt(&k, Role::Initiator, &t).unwrap_err(), SecretError::UnknownKeyFingerprint);
        assert!(matches!(decrypt(&k, Role::Initiator, &wire[..wire.len() - 16]), Err(SecretError::MsgKeyMismatch)));
        assert!(matches!(decrypt(&k, Role::Initiator, &wire[..wire.len() - 1]), Err(SecretError::Malformed(_))));
        assert!(matches!(decrypt(&k, Role::Initiator, &wire[..30]), Err(SecretError::Malformed(_))));
    }

    #[test]
    fn rejects_bad_inputs() {
        let k = key();
        assert!(encrypt_with_padding(&k, Role::Initiator, &[1, 2, 3], &[0; 13]).is_err(), "no alineado");
        assert!(encrypt_with_padding(&k, Role::Initiator, &[0; 4], &[0; 8]).is_err(), "relleno < 12");
        assert!(encrypt_with_padding(&k, Role::Initiator, &[0; 4], &[0; 1032]).is_err(), "relleno > 1024");
    }

    #[test]
    fn length_prefix_is_authenticated_but_checked() {
        // Un mensaje auténtico cuyo prefijo de longitud deja < 12 bytes de relleno
        // (construido a mano con la misma clave) debe rechazarse tras autenticar.
        let k = key();
        let mut data = Vec::new();
        data.extend_from_slice(&28u32.to_le_bytes());
        data.extend_from_slice(&[0u8; 28]);
        let x = Role::Initiator.kdf_x();
        let mk = msg_key_for(&k, x, &data);
        let (ak, iv) = derive(&k, &mk, x);
        ige_encrypt(&ak, &iv, &mut data);
        let mut wire = k.fingerprint().to_le_bytes().to_vec();
        wire.extend_from_slice(&mk);
        wire.extend_from_slice(&data);
        assert_eq!(decrypt(&k, Role::Initiator, &wire).unwrap_err(), SecretError::Malformed("relleno fuera de 12..=1024"));
    }
}
