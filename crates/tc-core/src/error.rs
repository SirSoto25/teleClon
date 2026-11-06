//! Errores del backend; la UI tiene un mensaje para cada variante.
use thiserror::Error;

pub type BackendResult<T> = Result<T, BackendError>;

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum BackendError {
    #[error("sin conexión con la red")]
    Network,
    #[error("demasiadas peticiones: espera {0} s")]
    FloodWait(u32),
    #[error("sesión no autorizada")]
    Unauthorized,
    #[error("no encontrado")]
    NotFound,
    #[error("acción no permitida: {0}")]
    Forbidden(String),
    #[error("dato no válido: {0}")]
    InvalidInput(&'static str),
    #[error("error criptográfico: {0}")]
    Crypto(String),
    #[error("error de almacenamiento: {0}")]
    Storage(String),
    #[error("error interno")]
    Internal,
}
