# Plan de acción: archivos, búsqueda en el chat y notificaciones de escritorio

- **Fecha:** 2026-11-14 · **Origen:** faltantes 1–4 del informe de paridad (`replica/diff-report.md`,
  45,0/100; filas F10 y S23/S28/S34 de `replica/features.csv`).
- **Estado de partida (verificado en el código):** `send_media` y `download_media` existen en `MediaApi`
  (`tc-core/src/api.rs`), pero la UI solo envía stickers y GIF; `download_media` devuelve `Vec<u8>` entero
  (sin progreso ni cancelación); los documentos se mapean como `MediaKind::File` sin acción; `tc-store` tiene
  FTS5 de mensajes sin interfaz; no hay ninguna dependencia de notificaciones.

## Orden y dependencias

```
A. Archivos (enviar → descargar/guardar)   ← B y C son independientes entre sí y de A
B. Búsqueda dentro del chat
C. Notificaciones de escritorio
```
A es lo más grande y toca `tc-core`/`tc-telegram`/`tc-ui`; B toca sobre todo `tc-store`/`tc-ui`; C añade un
crate pequeño y un punto de enganche en `tc-app`. B y C pueden ir en paralelo con A en sesiones propias
(worktree) siempre que A termine primero la **fase A0** (contratos de `tc-core`).

## A. Envío, descarga y guardado de archivos

**A0 · Contratos (Opus/high, 1 agente).** Añadir a `tc-core`:
- `TransferId`, `Transfer { id, kind: Upload|Download, peer, name, done: u64, total: u64, state }` y evento
  `BackendEvent::TransferProgress(Transfer)`; `cancel_transfer(TransferId)`.
- `download_media_to(peer, msg, file, variant, dest: PathBuf) -> TransferId` (a disco por trozos, sin cargar
  todo en memoria) y `send_media` con progreso/cancelación (sigue devolviendo los mensajes confirmados).
- Implementación en `MockBackend` (progreso simulado) y *passthrough* en `CachedBackend`/`MediaCacheLayer`.
- Tests de contrato en mock. **Hecho cuando:** `cargo test --workspace` verde y el contrato documentado en
  `docs/architecture.md`.

**A1 · Telegram real (Opus/high).** `tc-telegram/src/media.rs`: subida por partes con callback de progreso
(`upload.saveFilePart`/`saveBigFilePart`, ya hay `.upload()`), descarga por partes (`upload.getFile`) a
fichero temporal + renombrado atómico, cancelación, reintento ante `FILE_REFERENCE_*` y cambio de DC. Límite
de tamaño 2 GiB (4 GiB Premium: avisar, no asumir). Pruebas con el transporte falso.

**A2 · UI de envío (Opus/medium).**
- Botón clip + selector nativo (crate `rfd`; en Linux usa el portal XDG, comprobar Wayland), Ctrl+O.
- Arrastrar y soltar (evento `FileDropped` de iced) y pegar imagen del portapapeles.
- Diálogo de envío (vista previa de imagen / icono y nombre+tamaño de archivo, pie de foto, «enviar como
  archivo» vs «comprimir foto», varios archivos = álbum).
- Burbuja de subida con barra y botón cancelar; fallo → «Reintentar».
- Límite de 1024 caracteres en el pie (aviso como el de 4096 de `send`).

**A3 · UI de descarga y guardado (Sonnet/medium).**
- Burbuja de documento: icono por tipo, nombre, tamaño; clic = descargar (barra + cancelar) y luego «Abrir»
  (`xdg-open`/`open`/`start`) y «Guardar como…» (selector nativo de destino).
- Carpeta de descargas por defecto (`dirs`) configurable; no sobrescribir (sufijo « (1)»).
- Fotos y vídeos: «Guardar» desde el visor y el menú contextual del mensaje.
- Panel «Descargas» mínimo (lista de transferencias activas y recientes) — opcional, solo si cabe.

**Riesgos:** rutas con caracteres raros y nombres hostiles (sanear `file_name`: sin `/`, `..`, nombres
reservados de Windows); archivos enormes (progreso sin bloquear la UI); permisos del portal en Flatpak.
**Pruebas:** mock con progreso, cancelación a mitad, fallo de red y reintento; saneado de nombres
(propiedad); manual contra Telegram real (subida/descarga de 1 MB, 100 MB y un archivo con tilde y emoji).

## B. Búsqueda dentro de un chat

**B1 · Backend (Sonnet/medium).** `MessagesApi::search_in_chat(peer, query, filter, offset)`; en
`tc-store` usar el FTS5 existente (acentos y prefijos ya probados) y completar con `messages.search` de
Telegram cuando el historial local no esté completo (`history_complete`); en foros, limitar por tema
(`top_msg_id`). Mock y tests.

**B2 · UI (Opus/medium).** Lupa en la cabecera del chat y Ctrl+F: barra de búsqueda con contador «3 de 17»,
↑/↓ (o Enter/Shift+Enter) para saltar entre resultados, resaltado del término en las burbujas, filtros por
tipo (fotos, archivos, enlaces) en una segunda pasada. Saltar a un mensaje no cargado reutiliza el anclaje
`Anchor::Around` del historial (ya existe para «abrir en el primer no leído»). Esc cierra y vuelve a la
posición previa.

**Riesgos:** resultados que no están cargados (salto + carga de ventana); orden y paginación mezclando
local y servidor sin duplicar. **Pruebas:** acentos, mayúsculas, prefijo, sin resultados, borrado de un
resultado mientras se navega; manual con un chat largo real.

## C. Notificaciones de escritorio

**C1 · Crate `tc-notify` (Opus/medium).** Interfaz propia `Notifier` con implementación Linux (D-Bus
`org.freedesktop.Notifications` vía `notify-rust`/`zbus`), Windows (toast con `winrt-notification`/
`tauri-winrt-notification`) y macOS (`mac-notification-sys`); compilación condicional por SO y un
`NullNotifier` para tests. Acciones «Responder» y «Marcar como leído» donde el SO las admita.

**C2 · Reglas y ajustes (Sonnet/medium).** En `tc-ui`: notificar `MessageNew` entrantes cuando la ventana no
tiene foco **o** el chat no es el abierto; respetar silenciados (`muted`/`mute_until`), no notificar
mensajes propios ni de chats archivados silenciados; agrupar por chat; vista previa configurable
(nombre+texto / solo nombre / nada, importante para chats secretos: nunca mostrar texto por defecto);
sonido opcional (reutiliza `tc-audio`); ajustes por tipo (privados, grupos, canales) y «No molestar»
(S28). Clic en la notificación → enfocar la ventana y abrir ese chat/tema.

**C3 · Empaquetado (Sonnet/low).** Windows necesita AppUserModelID (en el instalador Inno y en el
manifiesto) para que los toasts salgan con el nombre e icono de Parlo; macOS requiere permiso en el primer
uso; Linux: `parlo.desktop` ya instalado (`packaging/linux/install-user.sh`). Documentar en
`docs/packaging.md`.

**Riesgos:** Wayland (foco y activación de ventana desde la notificación), tormentas de notificaciones al
reconectar con huecos (limitar y agrupar), privacidad (texto en pantalla de bloqueo).
**Pruebas:** `NullNotifier` con reglas (foco, silenciado, abierto, propio, secreto); manual en Linux real y
CI para compilar los tres SO.

## Reparto sugerido

| Fase | Agente / modelo / esfuerzo | Depende de | Ficheros principales |
| --- | --- | --- | --- |
| A0 contratos | Opus / high | — | `tc-core`, `tc-mock`, `tc-store` (passthrough) |
| A1 Telegram | Opus / high | A0 | `tc-telegram/src/media.rs` |
| A2 envío UI | Opus / medium | A0 | `tc-ui` (compositor, diálogo, burbuja de subida) |
| A3 descarga UI | Sonnet / medium | A0, A2 (burbuja) | `tc-ui` (burbuja de documento, visor) |
| B1 + B2 búsqueda | Sonnet / medium + Opus / medium | — | `tc-store`, `tc-ui` (cabecera) |
| C1–C3 notificaciones | Opus / medium + Sonnet | — | `tc-notify`, `tc-app`, `tc-ui`, `packaging` |

Reglas de proceso: un escritor por conjunto de ficheros; cada fase con tests antes del código; clippy
`-D warnings` y `cargo test --workspace` verdes antes de integrar; ADR nuevo si cambia un contrato
(archivos y notificaciones: dos ADR); entradas en `docs/logs/log.md` y `docs/agents/registry.md`; commit y
push por fase. Tras cada bloque, prueba real del usuario contra Telegram y luego se sube la puntuación de
paridad (`replica-diff`).

## Fuera de alcance por ahora
Vídeos con reproductor, vista previa de documentos (PDF), búsqueda global entre chats (F11), subida en
segundo plano reanudable tras cerrar la app, notificaciones push con la app cerrada.
