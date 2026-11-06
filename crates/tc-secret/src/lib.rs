//! # tc-secret — chats secretos E2E (MTProto 2.0), sans-IO
//!
//! > **EXPERIMENTAL — SIN AUDITORÍA.** Criptografía escrita en este proyecto a
//! > partir de la documentación pública (core.telegram.org/api/end-to-end y sus
//! > artículos `seq_no` y `pfs`) sobre primitivas RustCrypto. No ha pasado ninguna
//! > auditoría externa ni se ha probado contra clientes reales de la red: no debe
//! > presentarse como segura a usuarios finales. Ver `README.md` del crate y ADR-0004.
//!
//! Qué contiene:
//! - [`dh`]: validación de `messages.getDhConfig` (primo seguro de 2048 bits,
//!   condiciones de `g`), exponentes secretos (tiempo constante con `crypto-bigint`),
//!   comprobación de `g_a`/`g_b` (`1 < x < p-1` y cota `2^(2048-64)`).
//! - [`key`]: clave de 256 bytes con borrado al soltarse y `key_fingerprint`.
//! - [`crypto`]: `msg_key` SHA-256 y AES-256-IGE de MTProto 2.0 (x = 0 / 8), relleno 12..1024.
//! - [`tl`]: serializador propio del subconjunto de la capa secreta (capa 144).
//! - [`session`]: máquina de estados solicitud/aceptación, números de secuencia,
//!   huecos y `Resend`, re-keying (PFS), TTL del chat y descarte.
//! - [`SessionSnapshot`]: instantánea del estado (claves incluidas) para guardarlo y
//!   restaurarlo; el almacenamiento lo pone el llamador.
//! - [`ttl`]: temporizador de autodestrucción; [`visual`]: huella visual de la clave.
//!
//! Diseño sans-IO: nada de red, disco ni reloj. El llamador pasa bytes, la hora y una
//! fuente de [`Entropy`]; recibe [`session::Output`] con lo que debe enviar y mostrar.
#![forbid(unsafe_code)]
#![deny(missing_docs)]

pub mod crypto;
pub mod dh;
pub mod entropy;
pub mod error;
pub mod key;
pub mod session;
#[cfg(any(test, feature = "testing"))]
pub mod testing;
pub mod tl;
pub mod ttl;
pub mod visual;

pub use crypto::Role;
pub use dh::{DhParams, DhSecret};
pub use entropy::{Entropy, OsEntropy};
pub use error::{Result, SecretError};
pub use key::SecretKey;
pub use session::{DiscardReason, Event, Outbound, Output, SecretChat, SessionSnapshot, StateKind};
pub use ttl::SelfDestructTimer;
pub use visual::KeyVisualization;
