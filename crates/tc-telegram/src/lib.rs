//! `GrammersBackend`: implementación de `tc_core::ChatBackend` (y `AuthApi`) sobre la red de
//! Telegram con grammers 0.11 (MTProto en Rust puro) y tokio. Ver docs/architecture.md §3–4.
//!
//! - Credenciales: `TelegramConfig::from_env()` lee `TC_API_ID`/`TC_API_HASH` del entorno.
//! - Sesión: `TcSession` (sesión propia de grammers) serializada a un `SessionStore`
//!   (el keyring lo integra otro paso; aquí hay memoria y archivo 0600).
//! - Llamadas: TL crudo a través de `Transport` (real: `SenderPool` de grammers) con
//!   reintento de `FLOOD_WAIT` corto y reconexión con espera exponencial.
//! - Tiempo real: `UpdatesReceiver` de grammers (gestiona pts/qts y huecos con
//!   `getDifference`) → `events::apply_batch` → `BackendEvent`.
//! - Chats secretos: NO implementados aún contra la red (M2.7, `tc-secret`).
//!
//! Nada de esto se ha probado contra los servidores reales: ver docs/logs/log.md.
#![forbid(unsafe_code)]

mod auth;
mod backend;
pub mod config;
mod convert;
mod events;
mod ids;
mod media;
mod net;
mod peers;
pub mod session;

#[cfg(test)]
mod tests;

use std::sync::{Arc, Mutex, MutexGuard};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use async_trait::async_trait;
use grammers_client::InvocationError;
use grammers_client::sender::{ConnectionParams, SenderPool, SenderPoolHandle, UpdatesConfiguration, UpdatesReceiver};
use grammers_session::storages::ErasedSession;
use grammers_session::updates::UpdatesLike;
use grammers_tl_types as tl;
use tc_core::*;
use tokio::sync::{Notify, broadcast, mpsc, watch};
use tokio::task::JoinHandle;

pub use config::{ApiHash, ConfigError, TelegramConfig};
pub use net::{PoolTransport, Transport, map_error};
pub use session::{FileSessionStore, MemorySessionStore, SessionStore, StoreError, TcSession};

use crate::auth::AuthInner;
use crate::events::Cache;
use crate::net::{ConnTracker, Net, is_network, is_session_gone};

/// Lote de updates con los usuarios y chats que mencionan.
pub type UpdateBatch = (Vec<tl::enums::Update>, Vec<tl::enums::User>, Vec<tl::enums::Chat>);

/// Fuente de lotes de updates `(updates, usuarios, chats)`. La real es el `UpdatesReceiver`
/// de grammers; las pruebas usan una falsa.
#[async_trait]
pub trait UpdateSource: Send {
    async fn next_batch(&mut self) -> Result<UpdateBatch, InvocationError>;
}

#[async_trait]
impl UpdateSource for UpdatesReceiver {
    async fn next_batch(&mut self) -> Result<UpdateBatch, InvocationError> {
        let (ups, users, chats) = self.next().await?;
        Ok((ups.into_iter().map(|(u, _state)| u).collect(), users, chats))
    }
}

/// De dónde salen los updates.
enum Feed {
    /// Canal del `SenderPool`: antes del login solo se vigila `updateLoginToken` (QR);
    /// tras el login se entrega a un `UpdatesReceiver`.
    Pool { rx: mpsc::Receiver<UpdatesLike>, handle: SenderPoolHandle, session: Arc<ErasedSession> },
    Source(Box<dyn UpdateSource>),
    None,
}

pub(crate) fn now_secs() -> i64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map_or(0, |d| d.as_secs() as i64)
}

pub(crate) struct Inner {
    pub(crate) net: Net,
    store: Arc<dyn SessionStore>,
    pub(crate) cache: Mutex<Cache>,
    pub(crate) tx: broadcast::Sender<BackendEvent>,
    pub(crate) auth_tx: broadcast::Sender<AuthState>,
    pub(crate) auth: Mutex<AuthInner>,
    /// Despierta la tarea del QR (llegó `updateLoginToken`).
    pub(crate) qr_wake: Notify,
    pub(crate) logged_in: watch::Sender<bool>,
}

impl Inner {
    pub(crate) fn cache(&self) -> MutexGuard<'_, Cache> {
        self.cache.lock().unwrap_or_else(|e| e.into_inner())
    }

    pub(crate) fn emit(&self, ev: BackendEvent) {
        let _ = self.tx.send(ev);
    }

    pub(crate) fn emit_all(&self, evs: Vec<BackendEvent>) {
        evs.into_iter().for_each(|e| self.emit(e));
    }

    pub(crate) fn me(&self) -> Option<UserId> {
        self.cache().me
    }

    /// Llamada TL con traducción de error; si el servidor invalida la sesión, cierra sesión local.
    pub(crate) async fn rpc<R: tl::RemoteCall>(&self, req: &R) -> BackendResult<R::Return> {
        match self.net.invoke(req).await {
            Ok(r) => Ok(r),
            Err(e) => {
                if is_session_gone(&e) && *self.logged_in.borrow() {
                    self.session_lost().await;
                }
                Err(map_error(&e))
            }
        }
    }

    /// Guarda la sesión ya (tras iniciar sesión o al cerrar).
    pub(crate) async fn persist(&self) {
        let _ = self.net.session.take_dirty();
        let bytes = self.net.session.to_bytes();
        let store = Arc::clone(&self.store);
        let res = tokio::task::spawn_blocking(move || store.save(&bytes)).await;
        if !matches!(res, Ok(Ok(()))) {
            tracing::warn!("no se pudo guardar la sesión");
        }
    }

    pub(crate) async fn forget_session(&self) {
        self.net.session.reset();
        let _ = self.net.session.take_dirty();
        let store = Arc::clone(&self.store);
        let res = tokio::task::spawn_blocking(move || store.clear()).await;
        if !matches!(res, Ok(Ok(()))) {
            tracing::warn!("no se pudo borrar la sesión guardada");
        }
    }

    /// El servidor ya no reconoce la sesión (revocada desde otro dispositivo, etc.).
    pub(crate) async fn session_lost(&self) {
        self.forget_session().await;
        self.cache().me = None;
        self.logged_in.send_replace(false);
        self.set_auth(AuthState::LoggedOut);
    }

    /// Ingresa usuarios/chats de una respuesta y copia sus hashes a la sesión.
    pub(crate) fn ingest(&self, users: &[tl::enums::User], chats: &[tl::enums::Chat]) {
        let infos = {
            let mut c = self.cache();
            let mut infos = c.peers.ingest_users(users);
            infos.extend(c.peers.ingest_chats(chats));
            infos
        };
        self.cache_infos(infos);
    }

    pub(crate) fn cache_infos(&self, infos: Vec<grammers_session::types::PeerInfo>) {
        use grammers_session::Session;
        for i in infos {
            // `cache_peer` de TcSession es síncrono por dentro: el futuro se resuelve al primer poll.
            let _ = futures::executor::block_on(self.net.session.cache_peer(i));
        }
    }

    /// Aplica la respuesta `Updates` de una llamada propia y devuelve los eventos producidos.
    pub(crate) fn apply_response(&self, ups: Vec<tl::enums::Update>, users: Vec<tl::enums::User>, chats: Vec<tl::enums::Chat>) -> events::Applied {
        let (applied, infos) = {
            let mut c = self.cache();
            events::apply_batch(&mut c, &ups, &users, &chats, now_secs())
        };
        self.cache_infos(infos);
        applied
    }
}

/// Backend de la red real. Construir dentro de un runtime tokio (el de iced vale).
pub struct GrammersBackend {
    pub(crate) inner: Arc<Inner>,
    tasks: Mutex<Vec<JoinHandle<()>>>,
    pool: Option<SenderPoolHandle>,
}

impl GrammersBackend {
    /// Conecta con la red usando la sesión de `store` (si la hay). No bloquea en la red: el
    /// estado empieza en `AuthState::Checking` y cambia por `auth_events`.
    pub async fn connect(cfg: TelegramConfig, store: Arc<dyn SessionStore>) -> BackendResult<Self> {
        let loader = Arc::clone(&store);
        let saved = tokio::task::spawn_blocking(move || loader.load())
            .await
            .map_err(|_| BackendError::Internal)?
            .map_err(|e| BackendError::Storage(e.to_string()))?;
        let session = match saved {
            Some(bytes) => match TcSession::from_bytes(&bytes) {
                Ok(s) => Arc::new(s),
                Err(e) => {
                    tracing::warn!(error = %e, "sesión guardada ilegible: se empieza de cero");
                    Arc::new(TcSession::default())
                }
            },
            None => Arc::new(TcSession::default()),
        };
        let params = ConnectionParams { app_version: cfg.app_version.clone(), ..Default::default() };
        let pool = SenderPool::with_configuration(Arc::clone(&session), cfg.api_id, params);
        let handle = pool.handle.thin.clone();
        let erased = Arc::clone(&pool.handle.session);
        let runner = tokio::spawn(pool.runner.run());
        let transport = Arc::new(PoolTransport { handle: handle.clone() });
        let feed = Feed::Pool { rx: pool.updates, handle: handle.clone(), session: erased };
        let b = Self::build(cfg, store, session, transport, feed, Some(handle));
        b.tasks.lock().unwrap_or_else(|e| e.into_inner()).push(runner);
        Ok(b)
    }

    /// Construye el backend sobre un transporte y una fuente de updates cualesquiera (pruebas).
    #[doc(hidden)]
    pub fn with_transport(
        cfg: TelegramConfig,
        store: Arc<dyn SessionStore>,
        session: Arc<TcSession>,
        transport: Arc<dyn Transport>,
        updates: Option<Box<dyn UpdateSource>>,
    ) -> Self {
        let feed = updates.map_or(Feed::None, Feed::Source);
        Self::build(cfg, store, session, transport, feed, None)
    }

    fn build(cfg: TelegramConfig, store: Arc<dyn SessionStore>, session: Arc<TcSession>, transport: Arc<dyn Transport>, feed: Feed, pool: Option<SenderPoolHandle>) -> Self {
        let (tx, _) = broadcast::channel(1024);
        let (auth_tx, _) = broadcast::channel(64);
        let (logged_in, _) = watch::channel(false);
        let inner = Arc::new(Inner {
            net: Net { transport, session, cfg, conn: ConnTracker::new(tx.clone()) },
            store,
            cache: Mutex::new(Cache::default()),
            tx,
            auth_tx,
            auth: Mutex::new(AuthInner::default()),
            qr_wake: Notify::new(),
            logged_in,
        });
        let tasks = vec![
            tokio::spawn(startup(Arc::clone(&inner))),
            tokio::spawn(pump(Arc::clone(&inner), feed)),
            tokio::spawn(persister(Arc::clone(&inner))),
        ];
        Self { inner, tasks: Mutex::new(tasks), pool }
    }

    /// Guarda la sesión ahora mismo (por ejemplo antes de salir).
    pub async fn persist_now(&self) {
        if *self.inner.logged_in.borrow() {
            self.inner.persist().await;
        }
    }
}

impl Drop for GrammersBackend {
    fn drop(&mut self) {
        if *self.inner.logged_in.borrow() && self.inner.net.session.take_dirty() {
            let _ = self.inner.store.save(&self.inner.net.session.to_bytes());
        }
        for t in self.tasks.lock().unwrap_or_else(|e| e.into_inner()).drain(..) {
            t.abort();
        }
        if let Some(p) = &self.pool {
            p.quit();
        }
    }
}

/// Comprueba la sesión guardada al arrancar.
async fn startup(inner: Arc<Inner>) {
    let session = &inner.net.session;
    let Some(self_id) = session.self_user_id().filter(|_| session.has_auth_key()) else {
        inner.set_auth(AuthState::LoggedOut);
        return;
    };
    inner.cache().me = Some(UserId(self_id));
    inner.set_auth(AuthState::Checking);
    let mut wait = Duration::from_secs(2);
    loop {
        match inner.net.invoke(&tl::functions::users::GetUsers { id: vec![tl::enums::InputUser::UserSelf] }).await {
            Ok(users) => {
                inner.ingest(&users, &[]);
                let me = inner.cache().peers.users.get(&UserId(self_id)).cloned();
                match me {
                    Some(u) => {
                        inner.logged_in.send_replace(true);
                        inner.set_auth(AuthState::LoggedIn(u));
                    }
                    None => inner.session_lost().await,
                }
                return;
            }
            Err(e) if is_network(&e) => {
                // Sin red: se sigue en `Checking` y se reintenta.
                tokio::time::sleep(wait).await;
                wait = (wait * 2).min(Duration::from_secs(30));
            }
            Err(e) => {
                if matches!(&e, InvocationError::Rpc(r) if r.code == 401) {
                    inner.session_lost().await;
                } else {
                    tracing::warn!(error = %map_error(&e), "no se pudo comprobar la sesión");
                    inner.set_auth(AuthState::LoggedOut);
                }
                return;
            }
        }
    }
}

fn has_login_token(u: &UpdatesLike) -> bool {
    match u {
        UpdatesLike::Updates(tl::enums::Updates::UpdateShort(s)) => matches!(s.update, tl::enums::Update::LoginToken),
        UpdatesLike::Updates(tl::enums::Updates::Updates(s)) => s.updates.iter().any(|u| matches!(u, tl::enums::Update::LoginToken)),
        _ => false,
    }
}

async fn wait_logged_in(inner: &Inner) {
    let mut rx = inner.logged_in.subscribe();
    let _ = rx.wait_for(|v| *v).await;
}

/// Bombeo de updates: un único lector secuencial (preserva el orden por chat).
async fn pump(inner: Arc<Inner>, feed: Feed) {
    let source: Box<dyn UpdateSource> = match feed {
        Feed::None => return,
        Feed::Source(s) => {
            wait_logged_in(&inner).await;
            s
        }
        Feed::Pool { mut rx, handle, session } => {
            // Antes del login solo interesa saber si escanearon el QR.
            loop {
                tokio::select! {
                    _ = wait_logged_in(&inner) => break,
                    u = rx.recv() => match u {
                        Some(u) if has_login_token(&u) => inner.qr_wake.notify_one(),
                        Some(_) => {}
                        None => return,
                    },
                }
            }
            match UpdatesReceiver::create(handle, session, rx, UpdatesConfiguration { catch_up: true }).await {
                Ok(r) => Box::new(r),
                Err(_) => {
                    tracing::warn!("no se pudo crear el receptor de updates");
                    return;
                }
            }
        }
    };
    run_source(inner, source).await;
}

pub(crate) async fn run_source(inner: Arc<Inner>, mut source: Box<dyn UpdateSource>) {
    let mut wait = inner.net.cfg.network_backoff;
    loop {
        match source.next_batch().await {
            Ok((ups, users, chats)) => {
                wait = inner.net.cfg.network_backoff;
                inner.net.conn.set(ConnectionState::Online);
                let applied = inner.apply_response(ups, users, chats);
                if applied.login_token {
                    inner.qr_wake.notify_one();
                }
                inner.emit_all(applied.events);
            }
            Err(e) if is_session_gone(&e) => {
                inner.session_lost().await;
                return;
            }
            Err(e) => {
                if matches!(e, InvocationError::Dropped("updates channel was closed")) {
                    return;
                }
                inner.net.conn.set(if is_network(&e) { ConnectionState::Offline } else { ConnectionState::Connecting });
                tokio::time::sleep(wait).await;
                wait = (wait * 2).min(Duration::from_secs(30));
                inner.net.conn.set(ConnectionState::Connecting);
            }
        }
    }
}

/// Persiste la sesión cuando cambia (con un pequeño retardo para agrupar cambios).
async fn persister(inner: Arc<Inner>) {
    loop {
        inner.net.session.changed().await;
        tokio::time::sleep(Duration::from_secs(2)).await;
        if *inner.logged_in.borrow() && inner.net.session.take_dirty() {
            inner.persist().await;
        }
    }
}
