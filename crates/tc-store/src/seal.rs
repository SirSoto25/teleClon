//! Sellado AEAD (AES-256-GCM, RustCrypto) de blobs secretos con una clave de 256 bits que vive en la
//! bóveda ([`SecretVault`]), nunca en SQLite. Lo usa el almacén cifrado del estado de los chats secretos
//! (tabla `secret_chat_states`, migración 0003); sirve para cualquier blob que deba ir a disco cifrado.
//!
//! * Clave: aleatoria (`getrandom`), creada la primera vez con [`SealKey::load_or_create`] y guardada
//!   en la bóveda con el nombre que elija el llamador (p. ej. [`SECRET_STATE_KEY`]). En memoria va en
//!   `Zeroizing` (se borra al soltarla) y su `Debug` no la muestra.
//! * Nonce: 96 bits aleatorios nuevos en cada sellado (con clave aleatoria por instalación, el límite
//!   de ~2³² sellados por clave queda muy lejos del uso real).
//! * Datos asociados (`aad`): el llamador liga el blob a su fila (p. ej. el id del chat): un blob
//!   copiado a otra fila, truncado o con un solo bit cambiado se rechaza ([`SealError::Rejected`]).

use aes_gcm::aead::{Aead, KeyInit, Payload};
use aes_gcm::{Aes256Gcm, Nonce};
use zeroize::Zeroizing;

use crate::secret::NONCE_LEN;
use crate::vault::{SecretVault, VaultError};

/// Longitud de la clave de sellado (AES-256).
pub const SEAL_KEY_LEN: usize = 32;
/// Longitud de la etiqueta de autenticación de AES-GCM.
pub const TAG_LEN: usize = 16;
/// Nombre en la bóveda de la clave que cifra el estado de los chats secretos.
pub const SECRET_STATE_KEY: &str = "secret-state-key";

/// Error del sellado. Nunca contiene la clave ni el texto en claro.
#[derive(Debug, thiserror::Error)]
pub enum SealError {
    /// La bóveda falló o no está disponible (`VaultError::Unavailable`).
    #[error(transparent)]
    Vault(#[from] VaultError),
    /// La clave guardada no tiene el tamaño esperado (no se sobrescribe: se perderían los datos).
    #[error("la clave de sellado guardada no mide {SEAL_KEY_LEN} bytes")]
    BadKey,
    /// El sistema no dio entropía.
    #[error("sin entropía del sistema")]
    Entropy,
    /// El blob no se autentica: manipulado, de otra fila o sellado con otra clave.
    #[error("blob cifrado rechazado (manipulado, de otra fila o con otra clave)")]
    Rejected,
}

/// Blob sellado: nonce + texto cifrado con la etiqueta al final.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Sealed {
    /// Nonce aleatorio de esta escritura.
    pub nonce: [u8; NONCE_LEN],
    /// Texto cifrado ‖ etiqueta (16 bytes).
    pub ciphertext: Vec<u8>,
}

/// Clave de sellado de 256 bits (se borra de memoria al soltarse).
pub struct SealKey(Zeroizing<[u8; SEAL_KEY_LEN]>);

impl std::fmt::Debug for SealKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("SealKey(<oculta>)")
    }
}

impl SealKey {
    /// Clave aleatoria nueva.
    pub fn generate() -> Result<Self, SealError> {
        let mut k = Zeroizing::new([0u8; SEAL_KEY_LEN]);
        getrandom::fill(&mut k[..]).map_err(|_| SealError::Entropy)?;
        Ok(Self(k))
    }

    /// Clave desde bytes (deben ser exactamente [`SEAL_KEY_LEN`]).
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, SealError> {
        let mut k = Zeroizing::new([0u8; SEAL_KEY_LEN]);
        if bytes.len() != SEAL_KEY_LEN {
            return Err(SealError::BadKey);
        }
        k.copy_from_slice(bytes);
        Ok(Self(k))
    }

    /// Lee la clave `name` de la bóveda o, si no existe, crea una aleatoria y la guarda. Sin bóveda
    /// devuelve `SealError::Vault(VaultError::Unavailable(_))`: el llamador NO debe escribir en claro.
    pub fn load_or_create(vault: &dyn SecretVault, name: &str) -> Result<Self, SealError> {
        if let Some(saved) = vault.get(name)? {
            return Self::from_bytes(&saved);
        }
        let key = Self::generate()?;
        vault.set(name, &key.0[..])?;
        // Se relee para no cifrar con una clave que el almacén no haya guardado de verdad.
        match vault.get(name)? {
            Some(back) if back[..] == key.0[..] => Ok(key),
            _ => Err(SealError::Vault(VaultError::Backend("la clave de sellado no se guardó correctamente".into()))),
        }
    }

    fn cipher(&self) -> Aes256Gcm {
        Aes256Gcm::new_from_slice(&self.0[..]).expect("la clave mide 32 bytes")
    }

    /// Cifra `plaintext` ligado a `aad` con un nonce aleatorio nuevo.
    pub fn seal(&self, aad: &[u8], plaintext: &[u8]) -> Result<Sealed, SealError> {
        let mut nonce = [0u8; NONCE_LEN];
        getrandom::fill(&mut nonce).map_err(|_| SealError::Entropy)?;
        let ciphertext = self
            .cipher()
            .encrypt(&Nonce::from(nonce), Payload { msg: plaintext, aad })
            // Solo falla con entradas de tamaño absurdo (> 64 GiB).
            .map_err(|_| SealError::Rejected)?;
        Ok(Sealed { nonce, ciphertext })
    }

    /// Descifra y autentica. Cualquier cambio en `nonce`, `ciphertext` o `aad` → [`SealError::Rejected`].
    pub fn open(&self, aad: &[u8], nonce: &[u8], ciphertext: &[u8]) -> Result<Zeroizing<Vec<u8>>, SealError> {
        let nonce: [u8; NONCE_LEN] = nonce.try_into().map_err(|_| SealError::Rejected)?;
        if ciphertext.len() < TAG_LEN {
            return Err(SealError::Rejected);
        }
        self.cipher()
            .decrypt(&Nonce::from(nonce), Payload { msg: ciphertext, aad })
            .map(Zeroizing::new)
            .map_err(|_| SealError::Rejected)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::vault::{MemoryVault, VaultResult};
    use crate::Secret;

    #[test]
    fn roundtrip_and_every_kind_of_tampering_is_rejected() {
        let k = SealKey::generate().unwrap();
        let s = k.seal(b"fila-1", b"texto secreto").unwrap();
        assert_eq!(s.ciphertext.len(), b"texto secreto".len() + TAG_LEN);
        assert!(!s.ciphertext.windows(6).any(|w| w == b"secret"));
        assert_eq!(&**k.open(b"fila-1", &s.nonce, &s.ciphertext).unwrap(), b"texto secreto");
        // Otro aad (blob movido de fila), otro nonce, un bit del cifrado, la etiqueta, truncado.
        assert!(matches!(k.open(b"fila-2", &s.nonce, &s.ciphertext), Err(SealError::Rejected)));
        let mut n = s.nonce;
        n[0] ^= 1;
        assert!(matches!(k.open(b"fila-1", &n, &s.ciphertext), Err(SealError::Rejected)));
        for i in [0, s.ciphertext.len() - 1] {
            let mut c = s.ciphertext.clone();
            c[i] ^= 0x80;
            assert!(matches!(k.open(b"fila-1", &s.nonce, &c), Err(SealError::Rejected)), "byte {i}");
        }
        assert!(matches!(k.open(b"fila-1", &s.nonce, &s.ciphertext[..10]), Err(SealError::Rejected)));
        assert!(matches!(k.open(b"fila-1", &s.nonce[..5], &s.ciphertext), Err(SealError::Rejected)));
        // Otra clave.
        let other = SealKey::generate().unwrap();
        assert!(matches!(other.open(b"fila-1", &s.nonce, &s.ciphertext), Err(SealError::Rejected)));
        // Nonce nuevo en cada sellado.
        assert_ne!(k.seal(b"a", b"x").unwrap().nonce, k.seal(b"a", b"x").unwrap().nonce);
    }

    #[test]
    fn key_is_created_once_in_the_vault_and_reused() {
        let v = MemoryVault::new();
        let a = SealKey::load_or_create(&v, SECRET_STATE_KEY).unwrap();
        assert_eq!(v.keys(), [SECRET_STATE_KEY]);
        assert_eq!(v.get(SECRET_STATE_KEY).unwrap().unwrap().len(), SEAL_KEY_LEN);
        let b = SealKey::load_or_create(&v, SECRET_STATE_KEY).unwrap();
        let s = a.seal(b"", b"hola").unwrap();
        assert_eq!(&**b.open(b"", &s.nonce, &s.ciphertext).unwrap(), b"hola");
        assert_eq!(format!("{a:?}"), "SealKey(<oculta>)");
        // Una clave de tamaño erróneo no se sobrescribe.
        v.set("otra", b"corta").unwrap();
        assert!(matches!(SealKey::load_or_create(&v, "otra"), Err(SealError::BadKey)));
        assert_eq!(&**v.get("otra").unwrap().unwrap(), b"corta");
    }

    struct DeadVault;
    impl SecretVault for DeadVault {
        fn get(&self, _: &str) -> VaultResult<Option<Secret>> {
            Err(VaultError::Unavailable("sin servicio".into()))
        }
        fn set(&self, _: &str, _: &[u8]) -> VaultResult<()> {
            Err(VaultError::Unavailable("sin servicio".into()))
        }
        fn delete(&self, _: &str) -> VaultResult<bool> {
            Err(VaultError::Unavailable("sin servicio".into()))
        }
    }

    #[test]
    fn without_vault_there_is_no_key() {
        assert!(matches!(SealKey::load_or_create(&DeadVault, SECRET_STATE_KEY), Err(SealError::Vault(VaultError::Unavailable(_)))));
    }
}
