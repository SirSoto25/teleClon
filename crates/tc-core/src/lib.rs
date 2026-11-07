//! Dominio de Parlo: identificadores, entidades (usuario, chat, diálogo, mensaje, medio,
//! sticker, chat secreto), errores, eventos (`BackendEvent`) y el trait `ChatBackend`.
//! Sin E/S ni dependencias de UI. Ver docs/architecture.md §3.
//!
//! Alcance actual: lo necesario para M1.1 (lista de chats), M1.2 (conversación),
//! M1.3 (compositor) y la autenticación de M2.1 (`AuthApi`, sub-trait aparte). Search,
//! Settings, carpetas y perfil completo se añadirán en hitos posteriores como nuevos sub-traits.
#![forbid(unsafe_code)]

pub mod api;
pub mod auth;
pub mod error;
pub mod event;
pub mod ids;
pub mod model;

pub use api::*;
pub use auth::{AuthApi, AuthState, CodeKind, CodeRequest, QrToken, SignInResult};
pub use error::{BackendError, BackendResult};
pub use event::{BackendEvent, ConnectionState};
pub use ids::*;
pub use model::*;
