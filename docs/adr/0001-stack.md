# ADR-0001: Stack técnico

- **Estado:** aceptado con correcciones el 2026-11-06 (ver [ADR-0003](0003-arquitectura.md)). Estado previo: propuesto.
- **UI:** `iced` (Rust puro, MIT, wgpu con respaldo software, tema totalmente personalizable, Wayland/X11). Descartados: GTK4/libadwaita (aspecto GNOME difícil de hacer distinto), Slint (licencia dual GPL/comercial), egui (modo inmediato, poco apto para texto largo/chat), Tauri (webview, no es Rust-UI).
- **Protocolo:** MTProto vía el crate `grammers`, detrás de un trait `ChatBackend`. Requiere `api_id`/`api_hash` del usuario (https://my.telegram.org); hasta entonces se desarrolla contra un backend simulado (`MockBackend`).
- **Persistencia local:** SQLite (`rusqlite`).
- **Por qué mock primero:** permite construir y verificar toda la UI sin credenciales ni red.
- **Riesgo:** los términos de la API de Telegram exigen clave propia y prohíben suplantar al cliente oficial; la marca será propia.

## Confirmación y correcciones (2026-11-06, agente arquitecto)
Versiones comprobadas con `cargo search`/`cargo info` y `cargo check` en Rust 1.99.1 (Wayland, sin cabeceras de Vulkan):
- **iced 0.14.0: confirmado.** Por defecto ya trae `wgpu` + `tiny-skia` (respaldo software), `wayland` y `x11`. Se añaden `tokio`, `image`, `svg`, `lazy`, `advanced`, `qr_code`. No hacen falta cabeceras de Vulkan para compilar; wgpu carga el driver en ejecución y se puede forzar CPU con `ICED_BACKEND=tiny-skia`.
- **grammers 0.11.0: confirmado** (`grammers-client`, `grammers-session`, `grammers-tl-types`). Corrección: `grammers-session` sin su feature por defecto `sqlite-storage` (trae `libsql`, otro SQLite empaquetado que puede chocar con `libsqlite3-sys`), con `serde` para persistir la sesión en el keyring. No implementa chats secretos (ADR-0004): crate propio `tc-secret`.
- **rusqlite 0.41.1 con `bundled`: confirmado.** Corrección: sin `rusqlite_migration` (2.6.0 exige `rusqlite ^0.40`); migraciones con `PRAGMA user_version`.
- **Añadidos:** `keyring` 4.2.0 (claves), RustCrypto para E2E, `image` 0.25 + `flate2` + `rlottie` 0.5 (opcional) para stickers, `jiff` para hora local.
