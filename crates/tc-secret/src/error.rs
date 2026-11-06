//! Errores del crate. Ningún mensaje incluye material de clave.

/// Error de una operación de chat secreto.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum SecretError {
    /// El generador aleatorio del sistema falló.
    #[error("entropía del sistema no disponible")]
    Entropy,
    /// `p` o `g` de `messages.getDhConfig` no cumplen la especificación.
    #[error("parámetros Diffie-Hellman inválidos: {0}")]
    InvalidDhConfig(&'static str),
    /// `g_a`/`g_b` fuera del rango permitido.
    #[error("valor público Diffie-Hellman fuera de rango")]
    InvalidPublicValue,
    /// La huella recibida no coincide con la clave calculada.
    #[error("la huella de la clave no coincide")]
    FingerprintMismatch,
    /// El mensaje viene cifrado con una clave que no conocemos.
    #[error("huella de clave desconocida")]
    UnknownKeyFingerprint,
    /// El `msg_key` recalculado no coincide: mensaje manipulado o clave/dirección errónea.
    #[error("msg_key no coincide: mensaje manipulado o clave errónea")]
    MsgKeyMismatch,
    /// Estructura binaria inválida (longitudes, relleno, alineación).
    #[error("mensaje cifrado mal formado: {0}")]
    Malformed(&'static str),
    /// Error de (de)serialización TL.
    #[error("TL: {0}")]
    Tl(&'static str),
    /// Constructor TL que este crate no implementa.
    #[error("constructor TL no soportado: {0:#010x}")]
    UnsupportedConstructor(u32),
    /// La operación no es válida en el estado actual del chat.
    #[error("operación no válida en el estado actual: {0}")]
    InvalidState(&'static str),
}

/// Alias de resultado del crate.
pub type Result<T> = core::result::Result<T, SecretError>;
