//! Bóveda de secretos: dónde viven las claves que NUNCA van a SQLite en claro.
//!
//! * [`SecretVault`]: trait síncrono mínimo (`get`/`set`/`delete` por nombre).
//! * [`KeyringVault`] (feature `keyring-vault`, activa por defecto): Secret Service en Linux
//!   (D-Bus, vía `keyring` 4.x), Keychain en macOS, Credential Manager en Windows. Compila en
//!   cualquier entorno; si en tiempo de ejecución no hay servicio, **no entra en pánico**: todas las
//!   operaciones devuelven [`VaultError::Unavailable`] y el llamador decide el respaldo.
//! * [`MemoryVault`]: en memoria, para pruebas y modo efímero.
//!
//! Nombres de clave en uso: [`SESSION_KEY`] (`"session"`, sesión serializada de tc-telegram) y
//! [`secret_chat_key_name`] (`"secret-chat/<id>"`, clave de un chat secreto; ese mismo nombre es el
//! `key_ref` que guarda `SecretChatRow`). Los valores se devuelven en `Zeroizing`, que borra la
//! memoria al soltarlos.

use std::collections::HashMap;
use std::sync::Mutex;
use zeroize::Zeroizing;

/// Nombre de la sesión MTProto serializada (tc-telegram).
pub const SESSION_KEY: &str = "session";

/// Bytes secretos que se borran de memoria al soltarse.
pub type Secret = Zeroizing<Vec<u8>>;

/// Error de la bóveda. Nunca contiene el valor del secreto.
#[derive(Debug, thiserror::Error)]
pub enum VaultError {
    /// No hay almacén de secretos disponible (sin servicio D-Bus, plataforma no soportada, bloqueado).
    #[error("almacén de secretos no disponible: {0}")]
    Unavailable(String),
    /// El almacén respondió con un error.
    #[error("error del almacén de secretos: {0}")]
    Backend(String),
    /// Nombre de clave no permitido.
    #[error("nombre de clave no válido: {0:?}")]
    InvalidKey(String),
}

/// Resultado de la bóveda.
pub type VaultResult<T> = std::result::Result<T, VaultError>;

/// Almacén de secretos por nombre. Síncrono (el keyring lo es): llamar desde `spawn_blocking`.
pub trait SecretVault: Send + Sync + 'static {
    /// Valor guardado, o `None` si no existe.
    fn get(&self, key: &str) -> VaultResult<Option<Secret>>;
    /// Guarda (o sustituye) un valor.
    fn set(&self, key: &str, value: &[u8]) -> VaultResult<()>;
    /// Borra; devuelve `false` si no existía.
    fn delete(&self, key: &str) -> VaultResult<bool>;
}

/// Nombre de la clave de un chat secreto (y `key_ref` que se anota en `SecretChatRow`).
pub fn secret_chat_key_name(secret_chat_id: i64) -> String {
    format!("secret-chat/{secret_chat_id}")
}

/// Guarda la sesión serializada de tc-telegram.
pub fn save_session(v: &dyn SecretVault, bytes: &[u8]) -> VaultResult<()> {
    v.set(SESSION_KEY, bytes)
}
/// Sesión guardada, si hay.
pub fn load_session(v: &dyn SecretVault) -> VaultResult<Option<Secret>> {
    v.get(SESSION_KEY)
}
/// Olvida la sesión (cerrar sesión).
pub fn clear_session(v: &dyn SecretVault) -> VaultResult<bool> {
    v.delete(SESSION_KEY)
}
/// Guarda la clave de un chat secreto y devuelve el `key_ref` para la base.
pub fn save_secret_chat_key(v: &dyn SecretVault, secret_chat_id: i64, key: &[u8]) -> VaultResult<String> {
    let name = secret_chat_key_name(secret_chat_id);
    v.set(&name, key)?;
    Ok(name)
}
/// Clave de un chat secreto, si está.
pub fn load_secret_chat_key(v: &dyn SecretVault, secret_chat_id: i64) -> VaultResult<Option<Secret>> {
    v.get(&secret_chat_key_name(secret_chat_id))
}
/// Borra la clave de un chat secreto (al descartarlo).
pub fn delete_secret_chat_key(v: &dyn SecretVault, secret_chat_id: i64) -> VaultResult<bool> {
    v.delete(&secret_chat_key_name(secret_chat_id))
}

fn check_key(key: &str) -> VaultResult<()> {
    let ok = !key.is_empty()
        && key.len() <= 128
        && key.chars().all(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '-' | '.' | '/' | ':'));
    if ok { Ok(()) } else { Err(VaultError::InvalidKey(key.to_owned())) }
}

/// Bóveda en memoria (pruebas, modo efímero): se pierde al cerrar.
#[derive(Default)]
pub struct MemoryVault(Mutex<HashMap<String, Secret>>);

impl MemoryVault {
    /// Bóveda vacía.
    pub fn new() -> Self {
        Self::default()
    }
    /// Nombres guardados (para pruebas).
    pub fn keys(&self) -> Vec<String> {
        let mut k: Vec<_> = self.lock().keys().cloned().collect();
        k.sort();
        k
    }
    fn lock(&self) -> std::sync::MutexGuard<'_, HashMap<String, Secret>> {
        self.0.lock().unwrap_or_else(|e| e.into_inner())
    }
}

impl SecretVault for MemoryVault {
    fn get(&self, key: &str) -> VaultResult<Option<Secret>> {
        check_key(key)?;
        Ok(self.lock().get(key).map(|s| Zeroizing::new(s.to_vec())))
    }
    fn set(&self, key: &str, value: &[u8]) -> VaultResult<()> {
        check_key(key)?;
        self.lock().insert(key.to_owned(), Zeroizing::new(value.to_vec()));
        Ok(())
    }
    fn delete(&self, key: &str) -> VaultResult<bool> {
        check_key(key)?;
        Ok(self.lock().remove(key).is_some())
    }
}

#[cfg(feature = "keyring-vault")]
pub use keyring_vault::KeyringVault;

#[cfg(feature = "keyring-vault")]
mod keyring_vault {
    use super::*;

    /// Bóveda sobre el almacén de credenciales del sistema (crate `keyring` 4.x). Cada secreto es
    /// una entrada `(service, key)`; `service` separa cuentas (`teleclon/<cuenta>`).
    pub struct KeyringVault {
        service: String,
    }

    impl KeyringVault {
        /// Bóveda con un nombre de servicio arbitrario. No toca el sistema hasta la primera operación.
        pub fn new(service: &str) -> VaultResult<Self> {
            if service.is_empty() || service.len() > 128 {
                return Err(VaultError::InvalidKey(service.to_owned()));
            }
            Ok(Self { service: service.to_owned() })
        }

        /// Bóveda de una cuenta: servicio `teleclon/<account_id>` (mismo alfabeto que la ruta de la base).
        pub fn for_account(account_id: &str) -> VaultResult<Self> {
            let ok = !account_id.is_empty()
                && account_id.len() <= 64
                && account_id.chars().all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-');
            if !ok {
                return Err(VaultError::InvalidKey(account_id.to_owned()));
            }
            Self::new(&format!("{}/{account_id}", crate::APP_DIR))
        }

        /// ¿Hay un almacén de credenciales utilizable en este sistema? (Inicializa el almacén la primera vez.)
        pub fn is_available() -> bool {
            keyring::Entry::store_status().is_ok()
        }

        fn entry(&self, key: &str) -> VaultResult<keyring::Entry> {
            check_key(key)?;
            keyring::Entry::new(&self.service, key).map_err(map_err)
        }
    }

    /// Traduce el error sin copiar nunca los bytes que algunas variantes arrastran.
    fn map_err(e: keyring::Error) -> VaultError {
        use keyring::Error as E;
        match e {
            E::NoDefaultStore => VaultError::Unavailable("sin almacén de credenciales (¿servicio Secret Service/D-Bus parado?)".into()),
            E::NoStorageAccess(err) => VaultError::Unavailable(format!("sin acceso al almacén: {err}")),
            E::PlatformFailure(err) => VaultError::Backend(format!("fallo de plataforma: {err}")),
            E::BadEncoding(_) | E::BadDataFormat(..) => VaultError::Backend("dato guardado con formato inesperado".into()),
            E::TooLong(attr, max) => VaultError::Backend(format!("{attr} supera {max} caracteres")),
            E::Invalid(attr, why) => VaultError::Backend(format!("{attr} no válido: {why}")),
            E::NoEntry => VaultError::Backend("sin entrada".into()),
            _ => VaultError::Backend("error desconocido del almacén".into()),
        }
    }

    impl SecretVault for KeyringVault {
        fn get(&self, key: &str) -> VaultResult<Option<Secret>> {
            match self.entry(key)?.get_secret() {
                Ok(v) => Ok(Some(Zeroizing::new(v))),
                Err(keyring::Error::NoEntry) => Ok(None),
                Err(e) => Err(map_err(e)),
            }
        }
        fn set(&self, key: &str, value: &[u8]) -> VaultResult<()> {
            self.entry(key)?.set_secret(value).map_err(map_err)
        }
        fn delete(&self, key: &str) -> VaultResult<bool> {
            match self.entry(key)?.delete_credential() {
                Ok(()) => Ok(true),
                Err(keyring::Error::NoEntry) => Ok(false),
                Err(e) => Err(map_err(e)),
            }
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn construction_and_validation_do_not_touch_the_system() {
            assert!(KeyringVault::new("").is_err());
            assert!(KeyringVault::for_account("../x").is_err());
            let v = KeyringVault::for_account("u1").unwrap();
            assert_eq!(v.service, "teleclon/u1");
            assert!(matches!(v.get("bad key"), Err(VaultError::InvalidKey(_))));
        }

        #[test]
        fn without_dbus_degrades_to_unavailable_instead_of_panicking() {
            let v = KeyringVault::for_account("probe").unwrap();
            match v.get("session") {
                Ok(_) => {} // hay servicio y la entrada falta o existe: ambas valen
                Err(e) => assert!(matches!(e, VaultError::Unavailable(_) | VaultError::Backend(_)), "{e}"),
            }
            let _ = KeyringVault::is_available();
        }

        /// Ronda completa contra el almacén real (puede pedir desbloquear el llavero): `--ignored`.
        #[test]
        #[ignore = "usa el almacén de credenciales real del sistema"]
        fn live_roundtrip() {
            let v = KeyringVault::new(&format!("teleclon-test-{}", std::process::id())).unwrap();
            v.set("session", b"abc").unwrap();
            assert_eq!(&**v.get("session").unwrap().unwrap(), b"abc");
            assert!(v.delete("session").unwrap());
            assert!(v.get("session").unwrap().is_none());
            assert!(!v.delete("session").unwrap());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn memory_vault_roundtrip_and_isolation() {
        let v = MemoryVault::new();
        assert!(v.get("a").unwrap().is_none());
        v.set("a", b"uno").unwrap();
        v.set("a", b"dos").unwrap();
        assert_eq!(&**v.get("a").unwrap().unwrap(), b"dos");
        assert!(v.delete("a").unwrap());
        assert!(!v.delete("a").unwrap());
        for bad in ["", "a b", "ñ", &"x".repeat(129)] {
            assert!(matches!(v.set(bad, b"x"), Err(VaultError::InvalidKey(_))), "{bad}");
        }
    }

    #[test]
    fn session_and_secret_chat_helpers() {
        let v = MemoryVault::new();
        assert!(load_session(&v).unwrap().is_none());
        save_session(&v, b"{\"dc\":2}").unwrap();
        assert_eq!(&**load_session(&v).unwrap().unwrap(), b"{\"dc\":2}");
        let key_ref = save_secret_chat_key(&v, 77, &[7u8; 256]).unwrap();
        assert_eq!(key_ref, "secret-chat/77");
        assert_eq!(load_secret_chat_key(&v, 77).unwrap().unwrap().len(), 256);
        assert!(load_secret_chat_key(&v, 78).unwrap().is_none());
        assert_eq!(v.keys(), ["secret-chat/77", "session"]);
        assert!(delete_secret_chat_key(&v, 77).unwrap());
        assert!(clear_session(&v).unwrap());
        assert!(v.keys().is_empty());
    }

    #[test]
    fn works_as_trait_object_across_threads() {
        let v: std::sync::Arc<dyn SecretVault> = std::sync::Arc::new(MemoryVault::new());
        let v2 = v.clone();
        std::thread::spawn(move || save_session(&*v2, b"x").unwrap()).join().unwrap();
        assert!(load_session(&*v).unwrap().is_some());
    }

    /// El `key_ref` que guarda la base nunca contiene la clave, solo su nombre.
    #[test]
    fn key_ref_is_a_name_not_a_secret() {
        let v = MemoryVault::new();
        let r = save_secret_chat_key(&v, 5, b"super-secreto").unwrap();
        assert!(!r.contains("secreto"));
    }
}
