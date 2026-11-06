//! Interfaz iced de teleClon (dirección «Brasa», docs/design/system.md). Solo conoce
//! `tc_core::ChatBackend`. Ver docs/architecture.md §4 y §9.
//!
//! Hitos cubiertos: M1.1 lista de chats (fijados, no leídos, búsqueda local, carpetas por tipo),
//! M1.2 conversación con scroll virtual medido, M1.3 compositor (envío optimista, editar,
//! responder, borrar, reaccionar, reintentar) y chat secreto con huella de clave.
#![forbid(unsafe_code)]

pub mod anim;
pub mod app;
pub mod conv;
pub mod format;
pub mod icon;
pub mod measure;
pub mod style;
pub mod view;
pub mod widgets;

pub use app::{App, DemoStep, Flags, Folder, Msg};

#[cfg(test)]
mod tests;
