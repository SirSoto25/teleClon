# ADR-0001: Stack técnico

- **Estado:** propuesto (se confirma tras replica-architect)
- **UI:** `iced` (Rust puro, MIT, wgpu con respaldo software, tema totalmente personalizable, Wayland/X11). Descartados: GTK4/libadwaita (aspecto GNOME difícil de hacer distinto), Slint (licencia dual GPL/comercial), egui (modo inmediato, poco apto para texto largo/chat), Tauri (webview, no es Rust-UI).
- **Protocolo:** MTProto vía el crate `grammers`, detrás de un trait `ChatBackend`. Requiere `api_id`/`api_hash` del usuario (https://my.telegram.org); hasta entonces se desarrolla contra un backend simulado (`MockBackend`).
- **Persistencia local:** SQLite (`rusqlite`).
- **Por qué mock primero:** permite construir y verificar toda la UI sin credenciales ni red.
- **Riesgo:** los términos de la API de Telegram exigen clave propia y prohíben suplantar al cliente oficial; la marca será propia.
