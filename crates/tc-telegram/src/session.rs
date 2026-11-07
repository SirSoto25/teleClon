//! Sesión MTProto propia (implementa `grammers_session::Session`) y su persistencia.
//!
//! La sesión contiene la clave de autorización (equivale a estar logueado): se serializa a
//! bytes (`SessionFile`, JSON) y se entrega a un `SessionStore`. El almacén seguro (llavero del
//! sistema, `VaultSessionStore`) está en tc-app porque necesita tc-store; aquí hay un almacén en
//! memoria (pruebas) y uno en archivo 0600 (respaldo cuando no hay Secret Service, ver
//! docs/architecture.md §7).
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Mutex, MutexGuard};

use grammers_session::types::{ChannelState, DcOption, PeerId, PeerInfo, UpdateState, UpdatesState};
use grammers_session::{BoxFuture, Session, SessionData};
use serde::{Deserialize, Serialize};
use tokio::sync::Notify;

#[derive(Debug, thiserror::Error)]
pub enum StoreError {
    #[error("error de E/S del almacén de sesión: {0}")]
    Io(String),
    #[error("sesión guardada ilegible: {0}")]
    Format(String),
    #[error("almacén de sesión: {0}")]
    Backend(String),
}

/// Dónde se guarda la sesión serializada. Implementaciones síncronas (el keyring lo es);
/// el backend las llama desde `spawn_blocking`.
pub trait SessionStore: Send + Sync + 'static {
    fn load(&self) -> Result<Option<Vec<u8>>, StoreError>;
    fn save(&self, bytes: &[u8]) -> Result<(), StoreError>;
    fn clear(&self) -> Result<(), StoreError>;
}

/// Almacén en memoria (pruebas; se pierde al cerrar).
#[derive(Default)]
pub struct MemorySessionStore(Mutex<Option<Vec<u8>>>);

impl MemorySessionStore {
    pub fn with(bytes: Vec<u8>) -> Self {
        Self(Mutex::new(Some(bytes)))
    }
    pub fn snapshot(&self) -> Option<Vec<u8>> {
        self.0.lock().unwrap_or_else(|e| e.into_inner()).clone()
    }
}

impl SessionStore for MemorySessionStore {
    fn load(&self) -> Result<Option<Vec<u8>>, StoreError> {
        Ok(self.snapshot())
    }
    fn save(&self, bytes: &[u8]) -> Result<(), StoreError> {
        *self.0.lock().unwrap_or_else(|e| e.into_inner()) = Some(bytes.to_vec());
        Ok(())
    }
    fn clear(&self) -> Result<(), StoreError> {
        *self.0.lock().unwrap_or_else(|e| e.into_inner()) = None;
        Ok(())
    }
}

/// Archivo con permisos 0600, escrito de forma atómica (temporal + rename).
pub struct FileSessionStore {
    path: PathBuf,
}

impl FileSessionStore {
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self { path: path.into() }
    }
}

impl SessionStore for FileSessionStore {
    fn load(&self) -> Result<Option<Vec<u8>>, StoreError> {
        match std::fs::read(&self.path) {
            Ok(b) => Ok(Some(b)),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(e) => Err(StoreError::Io(e.kind().to_string())),
        }
    }

    fn save(&self, bytes: &[u8]) -> Result<(), StoreError> {
        use std::io::Write;
        let io = |e: std::io::Error| StoreError::Io(e.kind().to_string());
        if let Some(dir) = self.path.parent() {
            std::fs::create_dir_all(dir).map_err(io)?;
        }
        let tmp = self.path.with_extension("tmp");
        let mut opts = std::fs::OpenOptions::new();
        opts.write(true).create(true).truncate(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            opts.mode(0o600);
        }
        let mut f = opts.open(&tmp).map_err(io)?;
        f.write_all(bytes).map_err(io)?;
        f.sync_all().map_err(io)?;
        drop(f);
        std::fs::rename(&tmp, &self.path).map_err(io)
    }

    fn clear(&self) -> Result<(), StoreError> {
        match std::fs::remove_file(&self.path) {
            Ok(()) => Ok(()),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(e) => Err(StoreError::Io(e.kind().to_string())),
        }
    }
}

/// Formato serializado (versionado). `SessionData` de grammers no deriva serde, pero todos
/// sus campos sí (feature `serde` de grammers-session), así que se copia aquí.
#[derive(Serialize, Deserialize)]
struct SessionFile {
    v: u32,
    home_dc: i32,
    dc_options: Vec<DcOption>,
    peers: Vec<PeerInfo>,
    updates_state: UpdatesState,
}

const SESSION_FORMAT: u32 = 1;

/// Error de la sesión en memoria (no ocurre en la práctica: el mutex se recupera si se envenena).
#[derive(Debug, thiserror::Error)]
#[error("sesión no disponible")]
pub struct SessionError;

/// Sesión en memoria con marca de «cambiada» para que el backend la persista.
pub struct TcSession {
    data: Mutex<SessionData>,
    dirty: AtomicBool,
    changed: Notify,
}

impl Default for TcSession {
    fn default() -> Self {
        Self::from_data(SessionData::default())
    }
}

impl std::fmt::Debug for TcSession {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // Nunca imprimir claves de autorización.
        let d = self.lock();
        f.debug_struct("TcSession")
            .field("home_dc", &d.home_dc)
            .field("peers", &d.peer_infos.len())
            .field("has_auth_key", &d.dc_options.get(&d.home_dc).is_some_and(|o| o.auth_key.is_some()))
            .finish()
    }
}

impl TcSession {
    fn from_data(data: SessionData) -> Self {
        Self { data: Mutex::new(data), dirty: AtomicBool::new(false), changed: Notify::new() }
    }

    fn lock(&self) -> MutexGuard<'_, SessionData> {
        self.data.lock().unwrap_or_else(|e| e.into_inner())
    }

    /// Reconstruye la sesión desde bytes de `to_bytes`.
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, StoreError> {
        let f: SessionFile = serde_json::from_slice(bytes).map_err(|e| StoreError::Format(e.to_string()))?;
        if f.v != SESSION_FORMAT {
            return Err(StoreError::Format(format!("versión {} desconocida", f.v)));
        }
        let mut data = SessionData { home_dc: f.home_dc, updates_state: f.updates_state, ..SessionData::default() };
        for o in f.dc_options {
            data.dc_options.insert(o.id, o);
        }
        for p in f.peers {
            data.peer_infos.insert(p.id(), p);
        }
        Ok(Self::from_data(data))
    }

    /// Serializa la sesión completa (incluye claves de autorización: tratar como secreto).
    pub fn to_bytes(&self) -> Vec<u8> {
        let d = self.lock();
        let mut dc_options: Vec<DcOption> = d.dc_options.values().cloned().collect();
        dc_options.sort_by_key(|o| o.id);
        let f = SessionFile {
            v: SESSION_FORMAT,
            home_dc: d.home_dc,
            dc_options,
            peers: d.peer_infos.values().cloned().collect(),
            updates_state: d.updates_state.clone(),
        };
        serde_json::to_vec(&f).expect("la sesión siempre se puede serializar")
    }

    /// Vuelve al estado de fábrica (sin claves ni pares). Tras cerrar sesión.
    pub fn reset(&self) {
        *self.lock() = SessionData::default();
        self.mark();
    }

    /// Hay clave de autorización para el DC de casa.
    pub fn has_auth_key(&self) -> bool {
        let d = self.lock();
        d.dc_options.get(&d.home_dc).is_some_and(|o| o.auth_key.is_some())
    }

    /// Id de la cuenta propia, si se guardó al iniciar sesión.
    pub fn self_user_id(&self) -> Option<i64> {
        self.lock().peer_infos.values().find_map(|p| match p {
            PeerInfo::User { id, is_self: Some(true), .. } => Some(*id),
            _ => None,
        })
    }

    /// Autorización (access_hash) conocida de un par.
    pub fn peer_info(&self, id: PeerId) -> Option<PeerInfo> {
        self.lock().peer_infos.get(&id).cloned()
    }

    pub fn home_dc(&self) -> i32 {
        self.lock().home_dc
    }

    pub(crate) fn take_dirty(&self) -> bool {
        self.dirty.swap(false, Ordering::AcqRel)
    }

    pub(crate) async fn changed(&self) {
        self.changed.notified().await;
    }

    fn mark(&self) {
        self.dirty.store(true, Ordering::Release);
        self.changed.notify_one();
    }

    fn cache_peer_sync(&self, peer: PeerInfo) {
        let mut d = self.lock();
        let changed = match d.peer_infos.get_mut(&peer.id()) {
            Some(e) => {
                let before = e.clone();
                e.extend_info(&peer);
                *e != before
            }
            None => {
                d.peer_infos.insert(peer.id(), peer);
                true
            }
        };
        drop(d);
        if changed {
            self.mark();
        }
    }
}

impl Session for TcSession {
    type Error = SessionError;

    fn home_dc_id(&self) -> Result<i32, SessionError> {
        Ok(self.lock().home_dc)
    }

    fn set_home_dc_id(&self, dc_id: i32) -> BoxFuture<'_, Result<(), SessionError>> {
        Box::pin(async move {
            self.lock().home_dc = dc_id;
            self.mark();
            Ok(())
        })
    }

    fn dc_option(&self, dc_id: i32) -> Result<Option<DcOption>, SessionError> {
        Ok(self.lock().dc_options.get(&dc_id).cloned())
    }

    fn set_dc_option(&self, dc_option: &DcOption) -> BoxFuture<'_, Result<(), SessionError>> {
        let o = dc_option.clone();
        Box::pin(async move {
            self.lock().dc_options.insert(o.id, o);
            self.mark();
            Ok(())
        })
    }

    fn peer(&self, peer: PeerId) -> BoxFuture<'_, Result<Option<PeerInfo>, SessionError>> {
        Box::pin(async move {
            let d = self.lock();
            if peer == PeerId::self_user() {
                // grammers pide la cuenta propia con un id especial: se busca por `is_self`.
                return Ok(d.peer_infos.values().find(|p| matches!(p, PeerInfo::User { is_self: Some(true), .. })).cloned());
            }
            Ok(d.peer_infos.get(&peer).cloned())
        })
    }

    fn cache_peer(&self, peer: PeerInfo) -> BoxFuture<'_, Result<(), SessionError>> {
        Box::pin(async move {
            self.cache_peer_sync(peer);
            Ok(())
        })
    }

    fn updates_state(&self) -> BoxFuture<'_, Result<UpdatesState, SessionError>> {
        Box::pin(async move { Ok(self.lock().updates_state.clone()) })
    }

    fn set_update_state(&self, update: UpdateState) -> BoxFuture<'_, Result<(), SessionError>> {
        Box::pin(async move {
            {
                let mut d = self.lock();
                let s = &mut d.updates_state;
                match update {
                    UpdateState::All(all) => *s = all,
                    UpdateState::Primary { pts, date, seq } => {
                        s.pts = pts;
                        s.date = date;
                        s.seq = seq;
                    }
                    UpdateState::Secondary { qts } => s.qts = qts,
                    UpdateState::Channel { id, pts } => {
                        s.channels.retain(|c| c.id != id);
                        s.channels.push(ChannelState { id, pts });
                    }
                }
            }
            self.mark();
            Ok(())
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use grammers_session::types::PeerAuth;

    fn sample() -> TcSession {
        let s = TcSession::default();
        futures::executor::block_on(async {
            s.set_home_dc_id(4).await.unwrap();
            let mut o = s.dc_option(4).unwrap().expect("DC 4 conocido de fábrica");
            o.auth_key = Some([7u8; 256]);
            s.set_dc_option(&o).await.unwrap();
            s.cache_peer(PeerInfo::User { id: 77, auth: Some(PeerAuth::from_hash(99)), bot: Some(false), is_self: Some(true) }).await.unwrap();
            s.cache_peer(PeerInfo::Channel { id: 5, auth: Some(PeerAuth::from_hash(-3)), kind: None }).await.unwrap();
            s.set_update_state(UpdateState::Primary { pts: 10, date: 20, seq: 30 }).await.unwrap();
            s.set_update_state(UpdateState::Channel { id: 5, pts: 8 }).await.unwrap();
        });
        s
    }

    #[test]
    fn roundtrip_keeps_keys_peers_and_state() {
        let s = sample();
        let bytes = s.to_bytes();
        let r = TcSession::from_bytes(&bytes).unwrap();
        assert_eq!(r.home_dc(), 4);
        assert!(r.has_auth_key());
        assert_eq!(r.self_user_id(), Some(77));
        let st = futures::executor::block_on(r.updates_state()).unwrap();
        assert_eq!((st.pts, st.date, st.seq), (10, 20, 30));
        assert_eq!(st.channels, vec![ChannelState { id: 5, pts: 8 }]);
        assert_eq!(r.peer_info(PeerId::channel_unchecked(5)).and_then(|p| p.auth()).map(|a| a.hash()), Some(-3));
        // La búsqueda especial de la cuenta propia que usa grammers.
        let me = futures::executor::block_on(r.peer(PeerId::self_user())).unwrap();
        assert!(matches!(me, Some(PeerInfo::User { id: 77, .. })));
    }

    #[test]
    fn changes_mark_dirty_and_reset_forgets_everything() {
        let s = sample();
        assert!(s.take_dirty());
        assert!(!s.take_dirty());
        s.reset();
        assert!(s.take_dirty());
        assert!(!s.has_auth_key());
        assert_eq!(s.self_user_id(), None);
    }

    #[test]
    fn debug_never_prints_the_auth_key() {
        let dbg = format!("{:?}", sample());
        assert!(dbg.contains("has_auth_key: true"));
        assert!(!dbg.contains("7, 7, 7"));
    }

    #[test]
    fn rejects_garbage_and_unknown_versions() {
        assert!(TcSession::from_bytes(b"no es json").is_err());
        let mut v: serde_json::Value = serde_json::from_slice(&sample().to_bytes()).unwrap();
        v["v"] = 99.into();
        assert!(TcSession::from_bytes(&serde_json::to_vec(&v).unwrap()).is_err());
    }

    #[test]
    fn file_store_is_private_and_atomic() {
        let dir = std::env::temp_dir().join(format!("tc-telegram-test-{}", std::process::id()));
        let path = dir.join("cuenta").join("session.json");
        let store = FileSessionStore::new(&path);
        assert!(store.load().unwrap().is_none());
        store.save(b"abc").unwrap();
        assert_eq!(store.load().unwrap().as_deref(), Some(&b"abc"[..]));
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mode = std::fs::metadata(&path).unwrap().permissions().mode() & 0o777;
            assert_eq!(mode, 0o600);
        }
        store.clear().unwrap();
        assert!(store.load().unwrap().is_none());
        let _ = std::fs::remove_dir_all(dir);
    }
}
