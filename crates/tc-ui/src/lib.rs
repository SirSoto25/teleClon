//! Interfaz iced de Parlo (dirección «Brasa», docs/design/system.md). Solo conoce
//! `tc_core::ChatBackend`. Ver docs/architecture.md §4 y §9.
//!
//! Hitos cubiertos: M1.1 lista de chats (fijados, no leídos, búsqueda local, carpetas por tipo),
//! M1.2 conversación con scroll virtual medido, M1.3 compositor (envío optimista, editar,
//! responder, borrar, reaccionar, reintentar), inicio de sesión (teléfono, código, 2FA y QR sobre
//! `AuthApi`), selector y render de stickers (`tc-stickers`, estáticos y animados) y chats
//! secretos (crear, aceptar, TTL, cerrar, huella y aviso EXPERIMENTAL) solo vía `SecretChatsApi`.
//! Fotos de perfil y de mensajes cargadas de forma perezosa por `MediaApi` (ver `media.rs`), con visor.
#![forbid(unsafe_code)]

pub mod anim;
pub mod app;
pub mod audio;
pub mod conv;
pub mod emoji;
mod emoji_es;
pub mod format;
pub mod gifs;
pub mod icon;
pub mod login;
pub mod measure;
pub mod media;
pub mod panel;
pub mod reading;
pub mod sticker_anim;
pub mod sticker_img;
pub mod stickers;
pub mod style;
pub mod view;
pub mod widgets;

pub use app::{App, Auth, Backend, Connector, DemoStep, Flags, Folder, Msg, Phase, Session};
pub use login::LoginMsg;

#[cfg(test)]
mod tests;
#[cfg(test)]
mod tests_m2;
#[cfg(test)]
mod tests_panel;
#[cfg(test)]
mod tests_panel_shots;
#[cfg(test)]
mod tests_media;
#[cfg(test)]
mod tests_audio;
#[cfg(test)]
mod tests_reactions;
#[cfg(test)]
mod tests_chat_actions;
#[cfg(test)]
mod tests_reading;
#[cfg(test)]
mod tests_forum;
#[cfg(test)]
mod tests_sticker_anim;
