//! Errores de la caché local.

use std::path::PathBuf;

/// Error de `tc-store`.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// Error de SQLite.
    #[error("sqlite: {0}")]
    Sqlite(#[from] rusqlite::Error),
    /// Error de E/S (crear directorios, permisos del archivo).
    #[error("e/s en {path}: {source}")]
    Io {
        /// Ruta implicada.
        path: PathBuf,
        /// Error original.
        source: std::io::Error,
    },
    /// La base la creó una versión más nueva de la aplicación.
    #[error("la base tiene el esquema v{found} pero esta versión solo entiende hasta v{supported}")]
    SchemaTooNew {
        /// `user_version` encontrado.
        found: i64,
        /// Última migración conocida.
        supported: i64,
    },
    /// No se pudo determinar el directorio de datos (sin HOME/XDG ni perfil de usuario de Windows).
    #[error("no se pudo determinar el directorio de datos del usuario")]
    NoDataDir,
    /// Argumento inválido (id de cuenta, longitud de nonce...).
    #[error("argumento inválido: {0}")]
    Invalid(String),
    /// La fila pedida no existe.
    #[error("no encontrado: {0}")]
    NotFound(String),
}

/// Resultado de `tc-store`.
pub type Result<T> = std::result::Result<T, Error>;
