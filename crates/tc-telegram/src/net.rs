//! Transporte de llamadas TL y política de reintentos.
//!
//! `Transport` abstrae «mandar bytes de una función TL y recibir bytes de respuesta», así las
//! pruebas usan un transporte falso que decodifica la petición y responde con TL serializado.
//! La implementación real (`PoolTransport`) es el `SenderPool` de grammers-mtsender, que
//! crea las conexiones bajo demanda: si una conexión se cae, la siguiente llamada abre otra
//! (esa es la reconexión); aquí se añade el reintento con espera exponencial y el estado
//! `Connection` hacia la UI.
use std::sync::Arc;
use std::sync::atomic::{AtomicU8, Ordering};
use std::time::Duration;

use async_trait::async_trait;
use grammers_client::InvocationError;
use grammers_client::sender::SenderPoolHandle;
use grammers_session::Session;
use grammers_tl_types::{self as tl, Deserializable};
use tc_core::{BackendError, BackendEvent, ConnectionState};
use tokio::sync::broadcast;

use crate::config::TelegramConfig;
use crate::session::TcSession;

#[async_trait]
pub trait Transport: Send + Sync + 'static {
    /// Envía el cuerpo TL serializado al DC indicado y devuelve el cuerpo de la respuesta.
    async fn invoke_raw(&self, dc_id: i32, body: Vec<u8>) -> Result<Vec<u8>, InvocationError>;
    /// Cierra la conexión con un DC (tras migrar de DC al iniciar sesión).
    fn disconnect_from_dc(&self, _dc_id: i32) {}
}

/// Transporte real sobre el `SenderPool` de grammers.
pub struct PoolTransport {
    pub(crate) handle: SenderPoolHandle,
}

#[async_trait]
impl Transport for PoolTransport {
    async fn invoke_raw(&self, dc_id: i32, body: Vec<u8>) -> Result<Vec<u8>, InvocationError> {
        self.handle.raw_invoke_in_dc(dc_id, body).await
    }
    fn disconnect_from_dc(&self, dc_id: i32) {
        self.handle.disconnect_from_dc(dc_id);
    }
}

/// Qué hacer ante un error de una llamada.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Retry {
    /// Dormir y reintentar (FLOOD_WAIT corto o error interno del servidor).
    After(Duration),
    /// Corte de red: reintentar con espera exponencial.
    Reconnect,
    No,
}

/// Errores que invalidan la sesión guardada (hay que volver a iniciar sesión).
const SESSION_GONE: &[&str] = &[
    "AUTH_KEY_UNREGISTERED",
    "AUTH_KEY_INVALID",
    "AUTH_KEY_PERM_EMPTY",
    "SESSION_REVOKED",
    "SESSION_EXPIRED",
    "USER_DEACTIVATED",
    "USER_DEACTIVATED_BAN",
];

pub(crate) fn is_session_gone(e: &InvocationError) -> bool {
    matches!(e, InvocationError::Rpc(r) if r.code == 401 && SESSION_GONE.iter().any(|n| r.name == *n))
}

pub(crate) fn is_network(e: &InvocationError) -> bool {
    matches!(
        e,
        InvocationError::Io(_) | InvocationError::Dropped(_) | InvocationError::Transport(_) | InvocationError::Authentication(_)
    )
}

/// Decide el reintento. `attempt` empieza en 1.
pub(crate) fn retry_for(e: &InvocationError, attempt: u32, cfg: &TelegramConfig) -> Retry {
    match e {
        InvocationError::Rpc(r) if r.code == 420 => match r.value {
            Some(secs) if attempt <= 2 && Duration::from_secs(secs.into()) <= cfg.flood_sleep_threshold => {
                Retry::After(Duration::from_secs(secs.into()))
            }
            _ => Retry::No,
        },
        InvocationError::Rpc(r) if r.code == 500 || r.code == -500 => {
            if attempt <= 2 {
                Retry::After(Duration::from_secs(1))
            } else {
                Retry::No
            }
        }
        e if is_network(e) => {
            if attempt <= cfg.network_retries {
                Retry::Reconnect
            } else {
                Retry::No
            }
        }
        _ => Retry::No,
    }
}

/// Traduce un error de grammers al dominio. Nunca incluye datos del usuario.
pub fn map_error(e: &InvocationError) -> BackendError {
    match e {
        InvocationError::Rpc(r) => match r.code {
            420 => BackendError::FloodWait(r.value.unwrap_or(0)),
            303 => BackendError::Network,
            401 => BackendError::Unauthorized,
            404 => BackendError::NotFound,
            500 | -500 => BackendError::Internal,
            // Puede llegar como 403 o 400 según el método (p. ej. `messages.sendReaction`).
            _ if r.name == "PREMIUM_ACCOUNT_REQUIRED" => BackendError::PremiumRequired,
            _ if r.name == "REACTION_INVALID" || r.name == "REACTION_EMPTY" => BackendError::ReactionUnavailable,
            _ if r.name.starts_with("FILE_REFERENCE_") => BackendError::InvalidInput("file_reference"),
            403 => BackendError::Forbidden(r.name.clone()),
            _ => match r.name.as_str() {
                "PEER_ID_INVALID" | "MESSAGE_ID_INVALID" | "MSG_ID_INVALID" | "CHANNEL_INVALID" | "CHAT_ID_INVALID"
                | "USER_ID_INVALID" | "STICKERSET_INVALID" => BackendError::NotFound,
                "MESSAGE_EMPTY" | "MESSAGE_TOO_LONG" | "MESSAGE_NOT_MODIFIED" => BackendError::InvalidInput("text"),
                "PHONE_NUMBER_INVALID" | "PHONE_NUMBER_BANNED" | "PHONE_NUMBER_FLOOD" => BackendError::InvalidInput("phone"),
                "PHONE_CODE_INVALID" | "PHONE_CODE_EMPTY" => BackendError::InvalidInput("code"),
                "PHONE_CODE_EXPIRED" => BackendError::InvalidInput("code_expired"),
                "PASSWORD_HASH_INVALID" => BackendError::InvalidInput("password"),
                "RANDOM_ID_DUPLICATE" => BackendError::InvalidInput("random_id"),
                "FILE_PARTS_INVALID" | "FILE_PART_INVALID" | "PHOTO_INVALID_DIMENSIONS" | "MEDIA_EMPTY" => {
                    BackendError::InvalidInput("media")
                }
                "CHAT_WRITE_FORBIDDEN" | "CHAT_ADMIN_REQUIRED" | "MESSAGE_AUTHOR_REQUIRED" | "USER_IS_BLOCKED"
                | "YOU_BLOCKED_USER" | "CHAT_RESTRICTED" | "USER_BANNED_IN_CHANNEL" | "MESSAGE_EDIT_TIME_EXPIRED"
                // Gestión de chats (menú contextual): la UI los explica en español (`chat_actions.rs`).
                | "PINNED_DIALOGS_TOO_MUCH" | "PINNED_TOO_MUCH" | "USER_NOT_PARTICIPANT" | "USER_CREATOR" | "CHANNEL_PRIVATE"
                | "MESSAGE_DELETE_FORBIDDEN" | "PIN_RESTRICTED" | "CHAT_NOT_MODIFIED" => {
                    BackendError::Forbidden(r.name.clone())
                }
                _ => {
                    tracing::debug!(code = r.code, name = %r.name, "error RPC sin traducción específica");
                    BackendError::InvalidInput("request")
                }
            },
        },
        e if is_network(e) => BackendError::Network,
        InvocationError::Session(_) => BackendError::Storage("sesión".into()),
        InvocationError::Deserialize(_) | InvocationError::InvalidDc => BackendError::Internal,
        _ => BackendError::Internal,
    }
}

/// Último `ConnectionState` emitido, para no repetir eventos iguales.
pub(crate) struct ConnTracker {
    state: AtomicU8,
    tx: broadcast::Sender<BackendEvent>,
}

impl ConnTracker {
    pub(crate) fn new(tx: broadcast::Sender<BackendEvent>) -> Self {
        Self { state: AtomicU8::new(u8::MAX), tx }
    }

    pub(crate) fn set(&self, s: ConnectionState) {
        let code = match s {
            ConnectionState::Connecting => 0,
            ConnectionState::Online => 1,
            ConnectionState::Updating => 2,
            ConnectionState::Offline => 3,
        };
        if self.state.swap(code, Ordering::AcqRel) != code {
            let _ = self.tx.send(BackendEvent::Connection(s));
        }
    }

    pub(crate) fn is_online(&self) -> bool {
        self.state.load(Ordering::Acquire) == 1
    }
}

/// Cliente RPC: serializa, envía, reintenta y deserializa.
pub(crate) struct Net {
    pub(crate) transport: Arc<dyn Transport>,
    pub(crate) session: Arc<TcSession>,
    pub(crate) cfg: TelegramConfig,
    pub(crate) conn: ConnTracker,
}

impl Net {
    /// Llamada al DC de casa con la política de reintentos. Error sin traducir.
    pub(crate) async fn invoke<R: tl::RemoteCall>(&self, req: &R) -> Result<R::Return, InvocationError> {
        self.invoke_in(None, req).await
    }

    pub(crate) async fn invoke_in<R: tl::RemoteCall>(&self, dc: Option<i32>, req: &R) -> Result<R::Return, InvocationError> {
        let body = req.to_bytes();
        let mut attempt = 0u32;
        let mut backoff = self.cfg.network_backoff;
        loop {
            attempt += 1;
            let dc_id = match dc {
                Some(d) => d,
                None => self.session.home_dc_id().map_err(|e| InvocationError::Session(Box::new(e)))?,
            };
            match self.transport.invoke_raw(dc_id, body.clone()).await {
                Ok(bytes) => {
                    if !self.conn.is_online() {
                        self.conn.set(ConnectionState::Online);
                    }
                    return R::Return::from_bytes(&bytes).map_err(InvocationError::from);
                }
                Err(e) => match retry_for(&e, attempt, &self.cfg) {
                    Retry::After(d) => {
                        tracing::debug!(secs = d.as_secs(), "espera del servidor antes de reintentar");
                        tokio::time::sleep(d).await;
                    }
                    Retry::Reconnect => {
                        self.conn.set(ConnectionState::Connecting);
                        tokio::time::sleep(backoff).await;
                        backoff = (backoff * 2).min(Duration::from_secs(30));
                    }
                    Retry::No => {
                        if is_network(&e) {
                            self.conn.set(ConnectionState::Offline);
                        }
                        return Err(e);
                    }
                },
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use grammers_client::sender::RpcError;

    fn rpc(code: i32, name: &str, value: Option<u32>) -> InvocationError {
        InvocationError::Rpc(RpcError { code, name: name.into(), value, caused_by: None })
    }

    #[test]
    fn errors_map_to_domain() {
        assert_eq!(map_error(&rpc(420, "FLOOD_WAIT", Some(31))), BackendError::FloodWait(31));
        assert_eq!(map_error(&rpc(400, "PHONE_CODE_INVALID", None)), BackendError::InvalidInput("code"));
        assert_eq!(map_error(&rpc(400, "PHONE_CODE_EXPIRED", None)), BackendError::InvalidInput("code_expired"));
        assert_eq!(map_error(&rpc(400, "PASSWORD_HASH_INVALID", None)), BackendError::InvalidInput("password"));
        assert_eq!(map_error(&rpc(400, "PEER_ID_INVALID", None)), BackendError::NotFound);
        assert_eq!(map_error(&rpc(401, "AUTH_KEY_UNREGISTERED", None)), BackendError::Unauthorized);
        assert_eq!(map_error(&rpc(403, "CHAT_WRITE_FORBIDDEN", None)), BackendError::Forbidden("CHAT_WRITE_FORBIDDEN".into()));
        // Lo que devolvió la red real al reaccionar en «Mensajes guardados» sin Premium.
        assert_eq!(map_error(&rpc(403, "PREMIUM_ACCOUNT_REQUIRED", None)), BackendError::PremiumRequired);
        assert_eq!(map_error(&rpc(400, "PREMIUM_ACCOUNT_REQUIRED", None)), BackendError::PremiumRequired);
        assert_eq!(map_error(&rpc(400, "REACTION_INVALID", None)), BackendError::ReactionUnavailable);
        assert_eq!(map_error(&rpc(400, "FILE_REFERENCE_EXPIRED", None)), BackendError::InvalidInput("file_reference"));
        assert_eq!(map_error(&InvocationError::Dropped("x")), BackendError::Network);
        assert_eq!(map_error(&InvocationError::Io(std::io::Error::other("x"))), BackendError::Network);
    }

    #[test]
    fn flood_wait_policy_sleeps_only_short_waits() {
        let cfg = TelegramConfig::new(1, "0".repeat(32));
        assert_eq!(retry_for(&rpc(420, "FLOOD_WAIT", Some(3)), 1, &cfg), Retry::After(Duration::from_secs(3)));
        assert_eq!(retry_for(&rpc(420, "FLOOD_WAIT", Some(3)), 3, &cfg), Retry::No);
        assert_eq!(retry_for(&rpc(420, "FLOOD_WAIT", Some(600)), 1, &cfg), Retry::No);
        assert_eq!(retry_for(&rpc(420, "SLOWMODE_WAIT", Some(10)), 1, &cfg), Retry::After(Duration::from_secs(10)));
        assert_eq!(retry_for(&rpc(400, "MESSAGE_EMPTY", None), 1, &cfg), Retry::No);
        assert_eq!(retry_for(&InvocationError::Dropped("x"), 1, &cfg), Retry::Reconnect);
        assert_eq!(retry_for(&InvocationError::Dropped("x"), cfg.network_retries + 1, &cfg), Retry::No);
    }

    #[test]
    fn only_specific_401s_end_the_session() {
        assert!(is_session_gone(&rpc(401, "AUTH_KEY_UNREGISTERED", None)));
        assert!(is_session_gone(&rpc(401, "SESSION_REVOKED", None)));
        // SESSION_PASSWORD_NEEDED también es 401, pero es parte del login.
        assert!(!is_session_gone(&rpc(401, "SESSION_PASSWORD_NEEDED", None)));
    }
}
