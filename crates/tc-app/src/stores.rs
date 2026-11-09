//! Adaptadores de persistencia de la red real: unen los almacenes de `tc-telegram` (`SessionStore`,
//! `SecretStateStore`) con la bóveda y la base de `tc-store`. Viven aquí porque `tc-app` es el único
//! crate que depende de ambos (`tc-telegram` y `tc-store` no se conocen entre sí).
//!
//! * [`VaultSessionStore`]: la sesión MTProto (clave de autorización) va al llavero del sistema con el
//!   nombre `"session"`. Si el llavero no está disponible (`VaultError::Unavailable`) se usa el
//!   `FileSessionStore` 0600 de siempre y se avisa (sin valores). Un `session.json` existente se migra
//!   al llavero la primera vez que se lee con llavero disponible y después se borra.
//! * [`SealedSecretStateStore`]: el estado de cada chat secreto (metadatos + `SessionSnapshot`, con la
//!   clave del chat y textos en claro) se cifra con AES-256-GCM con una clave aleatoria de 256 bits que
//!   vive en el llavero (`"secret-state-key"`) y el blob va a la tabla `secret_chat_states` de SQLite.
//!   Sin llavero **no se persiste** (falla cerrado: nunca en claro) y se avisa.
//!
//! * Caché de archivos descargados (`tc_store::MediaCache`, `<datos>/accounts/<cuenta>/media/`, 0700/0600,
//!   LRU con tope de 512 MiB): fotos de perfil y de mensajes de chats NO secretos. No depende del llavero.
//!
//! Todo es síncrono: `tc-telegram` llama a estos almacenes desde `spawn_blocking`.

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use tc_core::{SecretChatId, SecretChatState, UserId};
use tc_store::{MediaCache, SECRET_STATE_KEY, SESSION_KEY, SealError, SealKey, SecretChatStateRow, SecretVault, Store, VaultError};
use tc_telegram::{FileSessionStore, SecretStateStore, SessionStore, StoreError, StoredSecretChat};
use zeroize::Zeroizing;

// ---------------------------------------------------------------------------
// Sesión MTProto en el llavero

/// `SessionStore` sobre una `SecretVault` (clave `"session"`), con respaldo en archivo 0600.
pub struct VaultSessionStore {
    vault: Arc<dyn SecretVault>,
    file: FileSessionStore,
    path: PathBuf,
    warned: AtomicBool,
}

impl VaultSessionStore {
    /// `fallback` es la ruta del `session.json` antiguo: se migra desde ahí y se usa si no hay llavero.
    pub fn new(vault: Arc<dyn SecretVault>, fallback: impl Into<PathBuf>) -> Self {
        let path = fallback.into();
        Self { vault, file: FileSessionStore::new(&path), path, warned: AtomicBool::new(false) }
    }

    /// Aviso (una vez por proceso) de que la sesión va al archivo. Nunca incluye la sesión.
    fn warn_fallback(&self, e: &VaultError) {
        if !self.warned.swap(true, Ordering::Relaxed) {
            tracing::warn!(
                motivo = %e,
                "llavero del sistema no disponible: la sesión se guarda EN CLARO en {} (permisos 0600). \
                 Arranca el Secret Service (gnome-keyring, KWallet…) para protegerla",
                self.path.display()
            );
        }
    }

    /// Mueve un `session.json` existente al llavero y lo borra. Devuelve la sesión migrada.
    fn migrate_file(&self) -> Option<Vec<u8>> {
        let legacy = match self.file.load() {
            Ok(Some(b)) => b,
            Ok(None) => return None,
            Err(e) => {
                tracing::warn!(error = %e, "no se pudo leer {} para migrarla al llavero", self.path.display());
                return None;
            }
        };
        // El archivo solo se escribe cuando no hay llavero, así que es siempre lo más reciente.
        match self.vault.set(SESSION_KEY, &legacy) {
            Ok(()) => match self.file.clear() {
                Ok(()) => tracing::info!("sesión de {} migrada al llavero del sistema; archivo borrado", self.path.display()),
                Err(e) => tracing::warn!(error = %e, "sesión migrada al llavero, pero no se pudo borrar {}", self.path.display()),
            },
            Err(e) => tracing::warn!(motivo = %e, "no se pudo migrar {} al llavero; se sigue usando el archivo", self.path.display()),
        }
        Some(legacy)
    }
}

fn backend(e: VaultError) -> StoreError {
    StoreError::Backend(e.to_string())
}

impl SessionStore for VaultSessionStore {
    fn load(&self) -> Result<Option<Vec<u8>>, StoreError> {
        match self.vault.get(SESSION_KEY) {
            Ok(saved) => Ok(self.migrate_file().or_else(|| saved.map(|s| s.to_vec()))),
            Err(e @ VaultError::Unavailable(_)) => {
                self.warn_fallback(&e);
                self.file.load()
            }
            Err(e) => Err(backend(e)),
        }
    }

    fn save(&self, bytes: &[u8]) -> Result<(), StoreError> {
        match self.vault.set(SESSION_KEY, bytes) {
            // Si quedaba una copia en claro de un arranque sin llavero, fuera.
            Ok(()) => self.file.clear(),
            Err(e @ VaultError::Unavailable(_)) => {
                self.warn_fallback(&e);
                self.file.save(bytes)
            }
            // Otros errores del llavero no caen al archivo: no se degrada a claro sin motivo.
            Err(e) => Err(backend(e)),
        }
    }

    fn clear(&self) -> Result<(), StoreError> {
        match self.vault.delete(SESSION_KEY) {
            Ok(_) => {}
            Err(e @ VaultError::Unavailable(_)) => self.warn_fallback(&e),
            Err(e) => return Err(backend(e)),
        }
        self.file.clear()
    }
}

// ---------------------------------------------------------------------------
// Estado de chats secretos: cifrado (clave en el llavero) en SQLite

/// Versión del formato del texto en claro de `StoredSecretChat` (columna `format`).
const STATE_FORMAT: i64 = 1;
/// Prefijo de los datos asociados del AEAD; se completa con el id del chat (liga el blob a su fila).
// Se conserva «teleclon» a propósito: es el contexto autenticado (AAD) del cifrado; cambiarlo invalidaría los datos ya guardados.
const AAD_PREFIX: &[u8] = b"teleclon/secret-chat-state/v1/";

fn aad(id: i32) -> Vec<u8> {
    [AAD_PREFIX, &id.to_le_bytes()].concat()
}

/// `SecretStateStore` cifrado: AES-256-GCM con clave del llavero, blobs en `secret_chat_states`.
pub struct SealedSecretStateStore {
    vault: Arc<dyn SecretVault>,
    db: Arc<Mutex<Store>>,
    key: Mutex<Option<Arc<SealKey>>>,
    warned: AtomicBool,
}

impl SealedSecretStateStore {
    pub fn new(vault: Arc<dyn SecretVault>, db: Arc<Mutex<Store>>) -> Self {
        Self { vault, db, key: Mutex::new(None), warned: AtomicBool::new(false) }
    }

    fn db(&self) -> std::sync::MutexGuard<'_, Store> {
        self.db.lock().unwrap_or_else(|e| e.into_inner())
    }

    /// Clave de sellado (de la bóveda, creada la primera vez; luego en memoria, en `Zeroizing`).
    fn key(&self) -> Result<Arc<SealKey>, SealError> {
        let mut slot = self.key.lock().unwrap_or_else(|e| e.into_inner());
        if let Some(k) = slot.as_ref() {
            return Ok(Arc::clone(k));
        }
        let k = Arc::new(SealKey::load_or_create(&*self.vault, SECRET_STATE_KEY)?);
        *slot = Some(Arc::clone(&k));
        Ok(k)
    }

    fn warn_unavailable(&self, e: &SealError) {
        if !self.warned.swap(true, Ordering::Relaxed) {
            tracing::warn!(
                motivo = %e,
                "llavero del sistema no disponible: el estado de los chats secretos NO se guarda entre reinicios \
                 (nunca se escribe en claro)"
            );
        }
    }
}

impl SecretStateStore for SealedSecretStateStore {
    fn load_all(&self) -> Result<Vec<StoredSecretChat>, StoreError> {
        let key = match self.key() {
            Ok(k) => k,
            Err(e @ SealError::Vault(VaultError::Unavailable(_))) => {
                self.warn_unavailable(&e);
                return Ok(Vec::new());
            }
            Err(e) => return Err(StoreError::Backend(e.to_string())),
        };
        let rows = self.db().list_secret_chat_states().map_err(|e| StoreError::Backend(e.to_string()))?;
        let mut out = Vec::with_capacity(rows.len());
        for row in rows {
            let Ok(id) = i32::try_from(row.secret_chat_id) else {
                tracing::warn!(fila = row.secret_chat_id, "estado de chat secreto con id fuera de rango: se ignora");
                continue;
            };
            if row.format != STATE_FORMAT {
                tracing::warn!(chat = id, formato = row.format, "estado de chat secreto con formato desconocido: se ignora");
                continue;
            }
            let decoded = key
                .open(&aad(id), &row.nonce, &row.ciphertext)
                .map_err(|e| e.to_string())
                .and_then(|plain| decode(&plain).map_err(|e| e.to_string()))
                .and_then(|c| if c.id.0 == id { Ok(c) } else { Err("el id cifrado no coincide con la fila".into()) });
            match decoded {
                Ok(c) => out.push(c),
                // No se borra la fila: podría descifrarse con la clave correcta (p. ej. otro llavero).
                Err(e) => tracing::warn!(chat = id, motivo = %e, "estado cifrado de chat secreto rechazado: se ignora"),
            }
        }
        Ok(out)
    }

    fn save(&self, chat: &StoredSecretChat) -> Result<(), StoreError> {
        let key = match self.key() {
            Ok(k) => k,
            Err(e @ SealError::Vault(VaultError::Unavailable(_))) => {
                self.warn_unavailable(&e);
                return Err(StoreError::Backend("sin llavero: el estado del chat secreto no se guarda".into()));
            }
            Err(e) => return Err(StoreError::Backend(e.to_string())),
        };
        let plain = encode(chat);
        let sealed = key.seal(&aad(chat.id.0), &plain).map_err(|e| StoreError::Backend(e.to_string()))?;
        let row = SecretChatStateRow {
            secret_chat_id: i64::from(chat.id.0),
            format: STATE_FORMAT,
            nonce: sealed.nonce.to_vec(),
            ciphertext: sealed.ciphertext,
            updated_at: now_secs(),
        };
        self.db().put_secret_chat_state(&row).map_err(|e| StoreError::Backend(e.to_string()))
    }

    fn remove(&self, id: SecretChatId) -> Result<(), StoreError> {
        self.db().delete_secret_chat_state(i64::from(id.0)).map(drop).map_err(|e| StoreError::Backend(e.to_string()))
    }
}

fn now_secs() -> i64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0, |d| d.as_secs() as i64)
}

// ---- formato del texto en claro (v1, little-endian) ----
// id i32 ‖ access_hash i64 ‖ user i64 ‖ outgoing u8 ‖ date i64 ‖ state u8 ‖ ttl? u8 (+ u32) ‖
// next_msg_id i32 ‖ session? u8 (+ len u32 ‖ bytes)

const FIXED_LEN: usize = 4 + 8 + 8 + 1 + 8 + 1 + 1 + 4 + 4 + 1 + 4;

fn encode(c: &StoredSecretChat) -> Zeroizing<Vec<u8>> {
    let session = c.session.as_deref().map(Vec::as_slice);
    // Capacidad exacta: el búfer no se realoja y no deja copias de la instantánea sin borrar.
    let mut w = Zeroizing::new(Vec::with_capacity(FIXED_LEN + session.map_or(0, <[u8]>::len)));
    w.extend_from_slice(&c.id.0.to_le_bytes());
    w.extend_from_slice(&c.access_hash.to_le_bytes());
    w.extend_from_slice(&c.user.0.to_le_bytes());
    w.push(u8::from(c.outgoing));
    w.extend_from_slice(&c.date.to_le_bytes());
    w.push(match c.state {
        SecretChatState::Waiting => 0,
        SecretChatState::Accepted => 1,
        SecretChatState::Discarded => 2,
    });
    w.push(u8::from(c.ttl_secs.is_some()));
    w.extend_from_slice(&c.ttl_secs.unwrap_or(0).to_le_bytes());
    w.extend_from_slice(&c.next_msg_id.to_le_bytes());
    w.push(u8::from(session.is_some()));
    let s = session.unwrap_or_default();
    w.extend_from_slice(&u32::try_from(s.len()).unwrap_or(u32::MAX).to_le_bytes());
    w.extend_from_slice(s);
    w
}

struct Rd<'a>(&'a [u8]);

impl<'a> Rd<'a> {
    fn take(&mut self, n: usize) -> Result<&'a [u8], StoreError> {
        if self.0.len() < n {
            return Err(StoreError::Format("estado de chat secreto truncado".into()));
        }
        let (a, b) = self.0.split_at(n);
        self.0 = b;
        Ok(a)
    }
    fn arr<const N: usize>(&mut self) -> Result<[u8; N], StoreError> {
        Ok(self.take(N)?.try_into().expect("longitud comprobada"))
    }
    fn u8(&mut self) -> Result<u8, StoreError> {
        Ok(self.arr::<1>()?[0])
    }
    fn flag(&mut self) -> Result<bool, StoreError> {
        match self.u8()? {
            0 => Ok(false),
            1 => Ok(true),
            _ => Err(StoreError::Format("estado de chat secreto: marca no válida".into())),
        }
    }
}

fn decode(b: &[u8]) -> Result<StoredSecretChat, StoreError> {
    let mut r = Rd(b);
    let id = SecretChatId(i32::from_le_bytes(r.arr()?));
    let access_hash = i64::from_le_bytes(r.arr()?);
    let user = UserId(i64::from_le_bytes(r.arr()?));
    let outgoing = r.flag()?;
    let date = i64::from_le_bytes(r.arr()?);
    let state = match r.u8()? {
        0 => SecretChatState::Waiting,
        1 => SecretChatState::Accepted,
        2 => SecretChatState::Discarded,
        _ => return Err(StoreError::Format("estado de chat secreto: estado desconocido".into())),
    };
    let has_ttl = r.flag()?;
    let ttl = u32::from_le_bytes(r.arr()?);
    let next_msg_id = i32::from_le_bytes(r.arr()?);
    let has_session = r.flag()?;
    let len = u32::from_le_bytes(r.arr()?) as usize;
    let session = r.take(len)?;
    if !r.0.is_empty() {
        return Err(StoreError::Format("estado de chat secreto: bytes de más".into()));
    }
    Ok(StoredSecretChat {
        id,
        access_hash,
        user,
        outgoing,
        date,
        state,
        ttl_secs: has_ttl.then_some(ttl),
        next_msg_id,
        session: has_session.then(|| Zeroizing::new(session.to_vec())),
    })
}

// ---------------------------------------------------------------------------
// Cableado

/// Almacenes de la red real para una cuenta.
pub struct TelegramStores {
    pub session: Arc<dyn SessionStore>,
    /// `None` = sin persistencia de chats secretos (el backend usa su almacén en memoria).
    pub secrets: Option<Arc<dyn SecretStateStore>>,
    /// Caché de archivos descargados (`None`: sin directorio de datos; se descarga siempre de la red).
    pub media: Option<Arc<MediaCache>>,
}

/// Abre la caché de archivos de la cuenta (no necesita llavero: nunca guarda nada de chats secretos).
fn media_cache(account: &str, data_dir: Option<&Path>) -> Option<Arc<MediaCache>> {
    let dir = data_dir?;
    match MediaCache::open_account(dir, account, tc_store::DEFAULT_MEDIA_CACHE_LIMIT) {
        Ok(c) => Some(Arc::new(c)),
        Err(e) => {
            tracing::warn!(motivo = %e, "no se pudo abrir la caché de archivos: las fotos se descargarán cada vez");
            None
        }
    }
}

/// Construye los almacenes con el llavero del sistema (`KeyringVault` de la cuenta `account`).
/// Síncrono (toca el llavero y abre SQLite): llamar desde `spawn_blocking`.
pub fn telegram_stores(account: &str, session_file: &Path, data_dir: Option<&Path>) -> TelegramStores {
    let vault: Option<Arc<dyn SecretVault>> = match tc_store::KeyringVault::for_account(account) {
        // Windows limita cada credencial a 2560 bytes: la sesión se reparte en varias entradas.
        Ok(v) if tc_store::KeyringVault::is_available() => Some(Arc::new(tc_store::ChunkedVault::new(v, tc_store::KEYRING_MAX_ENTRY))),
        Ok(_) => None,
        Err(e) => {
            tracing::warn!(motivo = %e, "TC_ACCOUNT no válido para el llavero (solo [A-Za-z0-9_-], hasta 64)");
            None
        }
    };
    stores_with_vault(vault, account, session_file, data_dir)
}

/// Igual que [`telegram_stores`] con una bóveda cualquiera (`None` = no hay llavero).
pub fn stores_with_vault(vault: Option<Arc<dyn SecretVault>>, account: &str, session_file: &Path, data_dir: Option<&Path>) -> TelegramStores {
    let Some(vault) = vault else {
        tracing::warn!(
            "llavero del sistema no disponible: la sesión se guarda EN CLARO en {} (permisos 0600) y el \
             estado de los chats secretos NO se guarda entre reinicios",
            session_file.display()
        );
        return TelegramStores { session: Arc::new(FileSessionStore::new(session_file)), secrets: None, media: media_cache(account, data_dir) };
    };
    let session: Arc<dyn SessionStore> = Arc::new(VaultSessionStore::new(Arc::clone(&vault), session_file));
    let db = data_dir.ok_or_else(|| "sin directorio de datos".to_string()).and_then(|d| Store::open_account(d, account).map_err(|e| e.to_string()));
    let secrets = match db {
        Ok(db) => Some(Arc::new(SealedSecretStateStore::new(vault, Arc::new(Mutex::new(db)))) as Arc<dyn SecretStateStore>),
        Err(e) => {
            tracing::warn!(motivo = %e, "no se pudo abrir la base local: el estado de los chats secretos no se guarda entre reinicios");
            None
        }
    };
    TelegramStores { session, secrets, media: media_cache(account, data_dir) }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::OnceLock;
    use tc_secret::testing::{SeededEntropy, rfc3526_params};
    use tc_secret::{DhParams, Event, Outbound, Output, SecretChat, SessionSnapshot};
    use tc_store::{MemoryVault, Secret, VaultResult};

    /// Directorio temporal único que se borra al terminar.
    struct TmpDir(PathBuf);
    impl TmpDir {
        fn new(tag: &str) -> Self {
            let p = std::env::temp_dir().join(format!("tc-app-{tag}-{}-{:?}", std::process::id(), std::thread::current().id()));
            let _ = std::fs::remove_dir_all(&p);
            std::fs::create_dir_all(&p).unwrap();
            Self(p)
        }
    }
    impl Drop for TmpDir {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    /// Bóveda que responde `Unavailable` (sin Secret Service).
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

    // ---- sesión ----

    #[test]
    fn session_goes_to_the_vault_not_to_the_file() {
        let dir = TmpDir::new("sess-vault");
        let path = dir.0.join("telegram/session.json");
        let vault = Arc::new(MemoryVault::new());
        let st = VaultSessionStore::new(vault.clone(), &path);
        assert!(st.load().unwrap().is_none());
        st.save(b"{\"auth\":\"k\"}").unwrap();
        assert!(!path.exists(), "nada en claro en disco");
        assert_eq!(&**vault.get(SESSION_KEY).unwrap().unwrap(), b"{\"auth\":\"k\"}");
        assert_eq!(st.load().unwrap().as_deref(), Some(&b"{\"auth\":\"k\"}"[..]));
        st.clear().unwrap();
        assert!(vault.keys().is_empty());
        assert!(st.load().unwrap().is_none());
    }

    #[test]
    fn existing_session_json_is_migrated_once_and_deleted() {
        let dir = TmpDir::new("sess-migrate");
        let path = dir.0.join("session.json");
        FileSessionStore::new(&path).save(b"antigua").unwrap();
        let vault = Arc::new(MemoryVault::new());
        let st = VaultSessionStore::new(vault.clone(), &path);
        assert_eq!(st.load().unwrap().as_deref(), Some(&b"antigua"[..]));
        assert!(!path.exists(), "el archivo en claro se borra tras migrar");
        assert_eq!(&**vault.get(SESSION_KEY).unwrap().unwrap(), b"antigua");
        // Segunda lectura (o siguiente arranque): sale del llavero, no hay nada que migrar.
        st.save(b"nueva").unwrap();
        let again = VaultSessionStore::new(vault.clone(), &path);
        assert_eq!(again.load().unwrap().as_deref(), Some(&b"nueva"[..]));
    }

    #[test]
    fn without_vault_falls_back_to_a_private_file() {
        let dir = TmpDir::new("sess-file");
        let path = dir.0.join("t/session.json");
        let st = VaultSessionStore::new(Arc::new(DeadVault), &path);
        assert!(st.load().unwrap().is_none());
        st.save(b"respaldo").unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            assert_eq!(std::fs::metadata(&path).unwrap().permissions().mode() & 0o777, 0o600);
        }
        assert_eq!(st.load().unwrap().as_deref(), Some(&b"respaldo"[..]));
        // Al volver el llavero, el archivo se migra y desaparece.
        let vault = Arc::new(MemoryVault::new());
        let st = VaultSessionStore::new(vault.clone(), &path);
        assert_eq!(st.load().unwrap().as_deref(), Some(&b"respaldo"[..]));
        assert!(!path.exists());
        // `stores_with_vault(None)` usa directamente el archivo y no persiste chats secretos.
        let s = stores_with_vault(None, "u1", &path, Some(&dir.0));
        assert!(s.secrets.is_none());
        s.session.save(b"x").unwrap();
        assert!(path.exists());
        // La caché de archivos no depende del llavero.
        let media = s.media.expect("caché de archivos");
        assert_eq!(media.dir(), dir.0.join("accounts/u1/media"));
        assert!(stores_with_vault(None, "u1", &path, None).media.is_none());
    }

    // ---- estado de chats secretos ----

    fn dh() -> DhParams {
        static DH: OnceLock<DhParams> = OnceLock::new();
        DH.get_or_init(rfc3526_params).clone()
    }

    fn sends(out: &Output) -> Vec<Vec<u8>> {
        out.outbound
            .iter()
            .filter_map(|o| match o {
                Outbound::SendEncrypted { data, .. } => Some(data.clone()),
                _ => None,
            })
            .collect()
    }

    fn texts(out: &Output) -> Vec<String> {
        out.events
            .iter()
            .filter_map(|e| match e {
                Event::Message(m) => Some(m.message.clone()),
                _ => None,
            })
            .collect()
    }

    const NOW: i64 = 1_800_000_000;

    /// Dos extremos con la clave ya establecida.
    fn ready_pair(ea: &mut SeededEntropy, eb: &mut SeededEntropy) -> (SecretChat, SecretChat) {
        let (mut a, out) = SecretChat::request(dh(), ea, &[]).unwrap();
        let Outbound::RequestEncryption { g_a } = &out.outbound[0] else { panic!() };
        let mut b = SecretChat::from_request(dh(), g_a).unwrap();
        let ob = b.accept(eb, &[], NOW).unwrap();
        let Outbound::AcceptEncryption { g_b, key_fingerprint } = &ob.outbound[0] else { panic!() };
        let oa = a.on_accepted(g_b, *key_fingerprint, ea, NOW).unwrap();
        for d in sends(&ob) {
            a.receive(&d, ea, NOW).unwrap();
        }
        for d in sends(&oa) {
            b.receive(&d, eb, NOW).unwrap();
        }
        (a, b)
    }

    fn record(id: i32, chat: &SecretChat) -> StoredSecretChat {
        StoredSecretChat {
            id: SecretChatId(id),
            access_hash: -42,
            user: UserId(777),
            outgoing: true,
            date: NOW,
            state: SecretChatState::Accepted,
            ttl_secs: Some(30),
            next_msg_id: 6,
            session: Some(Zeroizing::new(chat.snapshot().as_bytes().to_vec())),
        }
    }

    fn sealed_store(vault: Arc<dyn SecretVault>, db_path: &Path) -> SealedSecretStateStore {
        SealedSecretStateStore::new(vault, Arc::new(Mutex::new(Store::open(db_path).unwrap())))
    }

    /// Bytes de la base y su WAL (para buscar texto en claro).
    fn disk_bytes(db_path: &Path) -> Vec<u8> {
        let mut all = std::fs::read(db_path).unwrap_or_default();
        all.extend(std::fs::read(db_path.with_extension("sqlite3-wal")).unwrap_or_default());
        all
    }

    #[test]
    fn restart_and_resume_with_memory_vault() {
        let dir = TmpDir::new("secret-restart");
        let db_path = dir.0.join("cache.sqlite3");
        let vault: Arc<dyn SecretVault> = Arc::new(MemoryVault::new());
        let (mut ea, mut eb) = (SeededEntropy::new(b"app/a"), SeededEntropy::new(b"app/b"));
        let (mut a, mut b) = ready_pair(&mut ea, &mut eb);
        for i in 0..3 {
            let (_, out) = a.send_text(&format!("texto-en-claro-{i}"), &mut ea).unwrap();
            for d in sends(&out) {
                assert_eq!(texts(&b.receive(&d, &mut eb, NOW).unwrap()), [format!("texto-en-claro-{i}")]);
            }
        }
        let seq_before = a.seq_counters();
        let fingerprint = a.key_fingerprint();
        {
            let st = sealed_store(vault.clone(), &db_path);
            let rec = record(5, &a);
            // La instantánea lleva los textos enviados en claro (para `Resend`): la prueba de abajo no es vacía.
            assert!(rec.session.as_ref().unwrap().windows(14).any(|w| w == b"texto-en-claro"));
            st.save(&rec).unwrap();
            assert_eq!(st.load_all().unwrap().len(), 1);
        }
        drop(a); // «cierre» de la app
        // Nada en claro en disco: ni los textos del registro de enviados ni la clave del chat.
        let disk = disk_bytes(&db_path);
        assert!(!disk.windows(14).any(|w| w == b"texto-en-claro"), "texto en claro en la base");
        assert_eq!(vault.get(SECRET_STATE_KEY).unwrap().unwrap().len(), 32);

        // «Reinicio»: base reabierta y bóveda (la misma) con la clave.
        let st = sealed_store(vault.clone(), &db_path);
        let recs = st.load_all().unwrap();
        assert_eq!(recs.len(), 1);
        let rec = &recs[0];
        assert_eq!((rec.id, rec.access_hash, rec.user, rec.outgoing), (SecretChatId(5), -42, UserId(777), true));
        assert_eq!((rec.state, rec.ttl_secs, rec.next_msg_id, rec.date), (SecretChatState::Accepted, Some(30), 6, NOW));
        let snap = SessionSnapshot::from_bytes(rec.session.as_ref().unwrap()).unwrap();
        let mut a = SecretChat::restore(dh(), &snap).unwrap();
        assert_eq!(a.seq_counters(), seq_before);
        assert_eq!(a.key_fingerprint(), fingerprint);
        // Se sigue hablando en ambos sentidos sin repetir secuencias.
        let (_, out) = a.send_text("tras reiniciar", &mut ea).unwrap();
        for d in sends(&out) {
            assert_eq!(texts(&b.receive(&d, &mut eb, NOW).unwrap()), ["tras reiniciar"]);
        }
        let (_, out) = b.send_text("respuesta", &mut eb).unwrap();
        for d in sends(&out) {
            assert_eq!(texts(&a.receive(&d, &mut ea, NOW).unwrap()), ["respuesta"]);
        }
        // Un chat descartado (sin instantánea) también sobrevive; `remove` lo borra.
        let mut closed = record(6, &a);
        closed.session = None;
        closed.state = SecretChatState::Discarded;
        closed.ttl_secs = None;
        st.save(&closed).unwrap();
        let recs = st.load_all().unwrap();
        assert_eq!(recs.len(), 2);
        assert!(recs[1].session.is_none() && recs[1].ttl_secs.is_none());
        st.remove(SecretChatId(6)).unwrap();
        assert_eq!(st.load_all().unwrap().len(), 1);
    }

    #[test]
    fn tampered_or_moved_blobs_are_rejected() {
        let dir = TmpDir::new("secret-tamper");
        let db_path = dir.0.join("cache.sqlite3");
        let vault: Arc<dyn SecretVault> = Arc::new(MemoryVault::new());
        let (mut ea, mut eb) = (SeededEntropy::new(b"t/a"), SeededEntropy::new(b"t/b"));
        let (a, _b) = ready_pair(&mut ea, &mut eb);
        let st = sealed_store(vault.clone(), &db_path);
        for id in [1, 2, 3] {
            st.save(&record(id, &a)).unwrap();
        }
        let db = Store::open(&db_path).unwrap();
        // 1: un bit del cifrado cambiado.
        let mut r1 = db.get_secret_chat_state(1).unwrap().unwrap();
        r1.ciphertext[40] ^= 0x01;
        db.put_secret_chat_state(&r1).unwrap();
        // 2: blob del chat 3 copiado en la fila 2 (los datos asociados lo atan a su id).
        let mut r2 = db.get_secret_chat_state(3).unwrap().unwrap();
        r2.secret_chat_id = 2;
        db.put_secret_chat_state(&r2).unwrap();
        let ids: Vec<_> = st.load_all().unwrap().iter().map(|c| c.id.0).collect();
        assert_eq!(ids, [3], "solo el blob intacto se acepta");
        // Con otra clave (llavero distinto) no se descifra nada.
        let other = sealed_store(Arc::new(MemoryVault::new()), &db_path);
        assert!(other.load_all().unwrap().is_empty());
        // El texto en claro tampoco se decodifica si llega truncado o con bytes de más.
        let plain = encode(&record(9, &a));
        assert!(decode(&plain[..plain.len() - 1]).is_err());
        assert!(decode(&[&plain[..], &[0]].concat()).is_err());
        assert_eq!(decode(&plain).unwrap().id, SecretChatId(9));
    }

    #[test]
    fn without_vault_secret_state_is_never_written() {
        let dir = TmpDir::new("secret-novault");
        let db_path = dir.0.join("cache.sqlite3");
        let (mut ea, mut eb) = (SeededEntropy::new(b"n/a"), SeededEntropy::new(b"n/b"));
        let (a, _b) = ready_pair(&mut ea, &mut eb);
        let st = sealed_store(Arc::new(DeadVault), &db_path);
        assert!(st.save(&record(1, &a)).is_err(), "falla cerrado");
        assert!(st.load_all().unwrap().is_empty());
        assert!(Store::open(&db_path).unwrap().list_secret_chat_states().unwrap().is_empty());
    }
}
