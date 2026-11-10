//! Pruebas sin red: mapeo TL → dominio y flujos completos contra un transporte falso
//! que decodifica las peticiones TL y responde con TL serializado.
mod audio;
mod fixtures;
mod flows;
mod mapping;
mod media;
mod secret;
mod stickers_reactions;

use std::collections::VecDeque;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use async_trait::async_trait;
use grammers_client::InvocationError;
use grammers_client::sender::RpcError;
use grammers_session::Session;
use grammers_session::types::{PeerAuth, PeerInfo};
use grammers_tl_types as tl;
use tc_core::*;

use crate::{GrammersBackend, MemorySessionStore, TcSession, TelegramConfig, Transport, UpdateSource};

pub(crate) type Reply = Result<Vec<u8>, InvocationError>;
type Handler = Box<dyn FnMut(i32, u32, &[u8]) -> Reply + Send>;

/// Transporte falso: registra `(dc, nombre de la función)` y responde con un manejador.
pub(crate) struct Fake {
    handler: Mutex<Handler>,
    pub calls: Mutex<Vec<(i32, &'static str)>>,
    pub disconnected: Mutex<Vec<i32>>,
}

impl Fake {
    pub fn new(h: impl FnMut(i32, u32, &[u8]) -> Reply + Send + 'static) -> Arc<Self> {
        Arc::new(Self { handler: Mutex::new(Box::new(h)), calls: Mutex::new(vec![]), disconnected: Mutex::new(vec![]) })
    }
    pub fn names(&self) -> Vec<&'static str> {
        self.calls.lock().unwrap().iter().map(|c| c.1).collect()
    }
    pub fn count(&self, name: &str) -> usize {
        self.calls.lock().unwrap().iter().filter(|c| c.1 == name).count()
    }
}

#[async_trait]
impl Transport for Fake {
    async fn invoke_raw(&self, dc_id: i32, body: Vec<u8>) -> Result<Vec<u8>, InvocationError> {
        let id = u32::from_le_bytes(body[..4].try_into().unwrap());
        self.calls.lock().unwrap().push((dc_id, tl::name_for_id(id)));
        (self.handler.lock().unwrap())(dc_id, id, &body)
    }
    fn disconnect_from_dc(&self, dc_id: i32) {
        self.disconnected.lock().unwrap().push(dc_id);
    }
}

pub(crate) fn cid<T: tl::Identifiable>() -> u32 {
    T::CONSTRUCTOR_ID
}

pub(crate) fn ok<T: tl::Serializable>(v: T) -> Reply {
    Ok(v.to_bytes())
}

pub(crate) fn rpc_err(code: i32, name: &str, value: Option<u32>) -> Reply {
    Err(InvocationError::Rpc(RpcError { code, name: name.into(), value, caused_by: None }))
}

/// Decodifica una petición (las funciones TL se deserializan sin su id de constructor).
pub(crate) fn decode<T: tl::Deserializable>(body: &[u8]) -> T {
    T::from_bytes(&body[4..]).expect("petición TL válida")
}

pub(crate) const ME_ID: i64 = 77;

pub(crate) fn cfg() -> TelegramConfig {
    let mut c = TelegramConfig::new(12345, "0123456789abcdef0123456789abcdef".into());
    c.network_backoff = Duration::from_millis(100);
    c
}

/// Sesión ya iniciada (clave en el DC de casa y cuenta propia guardada).
pub(crate) fn logged_session() -> Arc<TcSession> {
    logged_session_as(ME_ID)
}

/// Igual que `logged_session` pero con otra cuenta propia.
pub(crate) fn logged_session_as(me: i64) -> Arc<TcSession> {
    let s = TcSession::default();
    futures::executor::block_on(async {
        let dc = s.home_dc_id().unwrap();
        let mut o = s.dc_option(dc).unwrap().unwrap();
        o.auth_key = Some([1; 256]);
        s.set_dc_option(&o).await.unwrap();
        s.cache_peer(PeerInfo::User { id: me, auth: Some(PeerAuth::from_hash(1)), bot: Some(false), is_self: Some(true) }).await.unwrap();
    });
    Arc::new(s)
}

pub(crate) fn backend(session: Arc<TcSession>, fake: Arc<Fake>, src: Option<Box<dyn UpdateSource>>) -> (GrammersBackend, Arc<MemorySessionStore>) {
    let store = Arc::new(MemorySessionStore::default());
    let b = GrammersBackend::with_transport(cfg(), store.clone(), session, fake, src);
    (b, store)
}

/// Espera (con tiempo real o pausado) a que el estado cumpla `pred`.
pub(crate) async fn wait_auth(b: &GrammersBackend, pred: impl Fn(&AuthState) -> bool) -> AuthState {
    for _ in 0..2000 {
        let s = b.auth_state().await.unwrap();
        if pred(&s) {
            return s;
        }
        tokio::time::sleep(Duration::from_millis(5)).await;
    }
    panic!("estado de auth no alcanzado: {:?}", b.auth_state().await);
}

/// Fuente de updates guionizada.
pub(crate) struct ScriptSource(pub VecDeque<Result<crate::UpdateBatch, InvocationError>>);

#[async_trait]
impl UpdateSource for ScriptSource {
    async fn next_batch(&mut self) -> Result<crate::UpdateBatch, InvocationError> {
        match self.0.pop_front() {
            Some(r) => r,
            None => futures::future::pending().await,
        }
    }
}
