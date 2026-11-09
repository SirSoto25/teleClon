# Arquitectura de teleClon (cliente propio de la red de Telegram)

- **Fecha:** 2026-11-06 · **Autor:** agente arquitecto (skill `replica-skill:replica-architect`)
- **Entradas:** [`replica/recon.md`](../replica/recon.md), [`replica/features.csv`](../replica/features.csv), [ADR-0001](adr/0001-stack.md), [ADR-0002](adr/0002-direccion-visual.md), [ADR-0004](adr/0004-stickers-y-chats-secretos.md)
- **Decisión:** [ADR-0003](adr/0003-arquitectura.md)
- **Nota:** la skill escribe por defecto `replica/architecture.md`; en este proyecto la arquitectura vive en `docs/` por la regla del índice. El SQL está en [`crates/tc-store/migrations/0001_init.sql`](../crates/tc-store/migrations/0001_init.sql).

## 0. Resumen

teleClon es una app de escritorio Linux en Rust que actúa como **cliente** de la red de Telegram (MTProto vía `grammers`), con las credenciales `api_id`/`api_hash` que aporta el usuario. Toda la UI se construye primero contra un `MockBackend` detrás del trait `ChatBackend`. Un solo proceso, un runtime tokio, una base SQLite por cuenta que actúa como caché. Nada de servidores propios.

```
            ┌──────────────── tc-app (binario) ────────────────┐
            │ config (env/archivo usuario) · tracing · cableado │
            └───────┬───────────────┬───────────────┬──────────┘
                    │               │               │
                 tc-ui ──────► tc-core ◄────── tc-store (CachedBackend + SQLite + keyring)
              (iced 0.14)     (dominio +            │ envuelve a ↓
                 │  │          ChatBackend)    ┌────┴───────────────┐
                 │  └─► tc-theme               │ tc-mock | tc-telegram (M2) │
                 └────► tc-stickers            └────┬───────────────┘
                                                    └─► tc-secret (E2E sans-IO)
```

## 1. Stack (versiones verificadas el 2026-11-06 con `cargo search`/`cargo info`, Rust 1.99.1)

| capa | elección | versión | por qué |
| --- | --- | --- | --- |
| lenguaje | Rust, edición 2024, resolver 3 | 1.99.1 | exigido por el proyecto |
| UI | `iced` con features `tokio, image, svg, lazy, advanced, qr_code` | 0.14.0 | Elm-like, tema propio total; por defecto trae `wgpu` **y** `tiny-skia` (respaldo software), `wayland` y `x11`; `qr_code` sirve para F02; `advanced` para la lista virtual |
| async | `tokio` | 1.54.1 | iced lo usa como ejecutor (feature `tokio`) y grammers lo exige |
| protocolo | `grammers-client`, `grammers-session` (sin `sqlite-storage`, con `serde`), `grammers-tl-types` | 0.11.0 | MTProto en Rust puro; compilado aparte con éxito en Rust 1.99 (sonda en scratchpad) |
| persistencia | `rusqlite` con `bundled` (SQLite con FTS5) | 0.41.1 | SQLite empaquetado; FTS5 activado en el build de `libsqlite3-sys` 0.39 |
| migraciones | a mano con `PRAGMA user_version` | — | `rusqlite_migration` 2.6.0 exige `rusqlite ^0.40`, incompatible con 0.41 |
| secretos | `keyring` (backend Secret Service por zbus) | 4.2.0 | claves fuera del disco en claro |
| cifrado en reposo | `aes-gcm` | 0.11.1 | cuerpo de mensajes de chats secretos en SQLite |
| E2E | `aes` 0.9.4 + `ige` 0.2.1 + `sha1` 0.11.0 + `sha2` 0.11.1 + `num-bigint` 0.5.1 + `getrandom` 0.4.3 + `zeroize` 1.9.1 + `subtle` 2.6.1 | — | RustCrypto, mismas series que `grammers-crypto` 0.11 (sin duplicados en el árbol) |
| stickers | `image` (solo `webp`,`png`) 0.25.10, `flate2` 1.1.11, `rlottie` 0.5.4 (opcional, feature `lottie`) | — | WebP en Rust puro; TGS = Lottie gzip; `cargo check -p tc-stickers --features lottie` pasa (rlottie-sys 0.2.12 compila su C++ con cmake/clang/g++) |
| tiempo | `jiff` | 0.2.39 | zona horaria local y DST correctos (ya lo usa grammers) |
| config/rutas | `directories` | 6.0.0 | rutas XDG |
| logs | `tracing` 0.1.44 + `tracing-subscriber` 0.3.23 | — | estándar |
| futuras (no añadidas aún) | `notify-rust` 4.18.2, `ksni` 0.3.7, `ashpd` 0.14.0 o `rfd` 0.17.2, `toml` 1.2.1 | — | notificaciones D-Bus, bandeja SNI, portal de archivos, archivo de config |

Sobre Vulkan: no se necesitan cabeceras de desarrollo de Vulkan para compilar; `wgpu` carga Vulkan/GL en tiempo de ejecución y, si no hay GPU usable, iced cae a `tiny-skia` (CPU). Forzar CPU: `ICED_BACKEND=tiny-skia` (variable leída por `iced_renderer` 0.14 en `fallback.rs`; admite lista separada por comas).

Sobre `grammers-session`: su feature por defecto `sqlite-storage` arrastra `libsql`, que empaqueta otro SQLite; convivir con `libsqlite3-sys` empaquetado arriesga choques de símbolos al enlazar. Se desactiva y la sesión se persiste como `SessionData` serializada (feature `serde`) en el keyring.

## 2. Workspace

| crate | tipo | responsabilidad | depende de |
| --- | --- | --- | --- |
| `tc-core` | lib | dominio puro: ids, entidades, `BackendEvent`, errores, trait `ChatBackend` (sub-traits por área) | `async-trait`, `futures`, `thiserror` |
| `tc-mock` | lib | `MockBackend` determinista (semilla fija), latencia y fallos inyectables, 2 cuentas simuladas con E2E real | `tc-core`, `tc-secret`, `tokio` |
| `tc-store` | lib | SQLite (migraciones, actor de escritura), `CachedBackend<B>` (decorador), bóveda de claves (keyring), cifrado en reposo | `tc-core`, `rusqlite`, `keyring`, `aes-gcm` |
| `tc-theme` | lib | tokens de `docs/design/tokens.json` como datos (sin iced) | — |
| `tc-secret` | lib | chats secretos MTProto 2.0, sans-IO (sin red ni disco), EXPERIMENTAL | RustCrypto |
| `tc-stickers` | lib | decodificar WebP/TGS a RGBA; caché de fotogramas | `image`, `flate2`, `rlottie`? |
| `tc-ui` | lib | app iced: estado, mensajes, pantallas, widgets, estilos | `tc-core`, `tc-theme`, `tc-stickers`, `iced` |
| `tc-app` | bin | config, elección de backend, arranque | todos |
| `tc-telegram` | lib (M2.1, creado; sin probar contra la red) | `GrammersBackend: ChatBackend + AuthApi`; chats secretos pendientes (M2.7, `tc-secret`) | `grammers-*`, `tc-core` |

Reglas: `tc-ui` solo conoce `tc-core` (nunca `tc-store`, `tc-mock` ni grammers). `tc-core` no hace E/S. `tc-secret` no hace E/S. Un único binario.

## 3. Dominio y trait `ChatBackend` (tc-core)

Identidad de chat: `PeerRef = User(UserId) | Group(GroupId) | Secret(SecretChatId)`. `MsgId(i32)` por chat; `RandomId(i64)` para idempotencia de envíos. Tiempos como `Timestamp(i64)` UTC.

El trait se parte en sub-traits por área para que cada uno sea legible y el mock pueda crecer por partes. Todos son `#[async_trait] … : Send + Sync` y se usan como `Arc<dyn ChatBackend>`.

```rust
pub trait ChatBackend:
    AuthApi + DialogsApi + MessagesApi + MediaApi + SearchApi
    + ProfileApi + SettingsApi + StickersApi + SecretChatsApi + Send + Sync + 'static
{
    /// Flujo único de eventos en tiempo real (orden por llegada; ver §4).
    fn events(&self) -> BoxStream<'static, BackendEvent>;
}
```

Operaciones ("rutas" del cliente). Cada fila es una operación del backend; en el cliente de Telegram mapea a funciones TL públicas vía grammers.

| área | operación | hace | entrada → salida | flujo |
| --- | --- | --- | --- | --- |
| Auth | `auth_state` | ¿hay sesión? | () → `AuthState` | F01 |
| Auth | `request_login_code` | pide código | phone → `CodeRequest` (tipo, timeout) | F01 |
| Auth | `resend_code` | reenvía | `CodeRequest` → `CodeRequest` | F01 |
| Auth | `sign_in` | verifica código | code → `SignIn{Ok(User) \| PasswordNeeded(hint) \| SignUpNeeded}` | F01 |
| Auth | `check_password` | 2FA (SRP) | password → `User` | F01/F02 |
| Auth | `sign_up` | cuenta nueva | nombre, apellido → `User` | F01 |
| Auth | `qr_login_start` | token QR (`auth.exportLoginToken` en crudo) | () → `QrToken{url, expires}`; aceptación llega como evento | F02 |
| Auth | `log_out` | cierra sesión y borra datos locales | () → () | — |
| Dialogs | `dialogs` | página de la lista | `DialogsPage{offset, limit, archived}` → `Vec<Dialog>` | F03 |
| Dialogs | `folders` / `save_folder` / `delete_folder` / `reorder_folders` | carpetas | `Folder` → () | F12 |
| Dialogs | `set_pinned` / `set_archived` / `set_muted` | estado de la fila | peer, valor → () | F14 |
| Dialogs | `save_draft` | borrador | peer, `Draft` → () | — |
| Dialogs | `contacts` / `resolve_username` | personas | () / `@user` → `Vec<User>` / `User` | S37 |
| Dialogs | `create_private_chat` / `create_group` | nuevo chat | user / título+users → `Dialog` | S36 |
| Messages | `history` | página de historial | peer, `Anchor{before\|after\|around}`, limit → `Vec<Message>` | F03 |
| Messages | `send_text` | envía | peer, `OutgoingText{text, entities, reply_to, random_id}` → `Message` | F04/F05 |
| Messages | `edit_message` | edita | peer, id, texto → `Message` | F07 |
| Messages | `delete_messages` | borra | peer, ids, `revoke` → () | F08 |
| Messages | `forward_messages` | reenvía | from, ids, `Vec<PeerRef>`, `hide_sender` → () | F06 |
| Messages | `send_reaction` | pone/quita | peer, id, `Option<Reaction>` → () | F09 |
| Messages | `mark_read` | lectura | peer, max_id → () | F03 |
| Messages | `set_typing` / `pin_message` | estado | peer, … → () | — |
| Media | `upload` | sube por partes, reanudable | path, kind → `UploadHandle` (progreso por evento) | F10 |
| Media | `send_media` | envía álbum/archivo | peer, `OutgoingMedia{items, caption, compress}` → `Vec<Message>` | F10 |
| Media | `download_profile_photo` / `download_media` (implementado 2026-11-09) | descarga a memoria | peer, `ProfilePhoto`, `PhotoSize` / peer, msg, `FileRef`, `MediaVariant{Thumb,Full}` → bytes | F10/S19 |
| Search | `search_global` / `search_in_chat` | búsqueda servidor | query, filtro, offset → `SearchPage` | F11 |
| Profile | `user_full` / `group_full` | info | id → `UserFull` / `GroupFull` | F13 |
| Profile | `set_blocked` / `update_profile` / `set_username` / `set_profile_photo` | perfil | … → () | F13/S27 |
| Settings | `notify_settings` / `set_notify_settings` | avisos | scope → `NotifySettings` | F14 |
| Settings | `sessions` / `terminate_session` | dispositivos | () → `Vec<Session>` / hash → () | F15 |
| Settings | `privacy` / `set_privacy` | privacidad básica | key → reglas | S29 |
| Stickers | `installed_sticker_sets` / `sticker_set` | packs | () / `SetRef` → `StickerSet` | F17 |
| Stickers | `install_sticker_set` / `uninstall_sticker_set` / `reorder_sticker_sets` | gestión | id → () | F17 |
| Stickers | `recent_stickers` / `faved_stickers` / `set_sticker_faved` / `stickers_for_emoji` | selector | … → `Vec<Sticker>` | F17 |
| Stickers | `send_sticker` | envía | peer, sticker, reply_to → `Message` | F17 |
| Secret | `request_secret_chat` | inicia DH | user → `SecretChat{state: Waiting}` | F18 |
| Secret | `accept_secret_chat` / `discard_secret_chat` | acepta / cierra | id (, borrar historial) → () | F18 |
| Secret | `send_secret_text` / `send_secret_media` | envía cifrado | id, `OutgoingSecret{text, ttl, media}` → `SecretMessage` | F18 |
| Secret | `set_secret_ttl` / `mark_secret_read` | TTL / lectura (arranca TTL) | id, segundos / max_date → () | F18 |
| Secret | `secret_key_visual` | huella | id → `KeyVisual{fingerprint, grid, emoji}` | F18 |

Total: 66 operaciones (contando por separado las agrupadas con `/`).

`BackendEvent` (entrada a la UI): `Connection(Connecting|Online|Updating|Offline)`, `Auth(QrAccepted|LoggedOut|PasswordNeeded)`, `DialogUpserted`, `MessageNew`, `MessageEdited`, `MessagesDeleted`, `MessageSent{random_id, msg}`, `MessageFailed{random_id, err}`, `ReadInbox`, `ReadOutbox`, `Typing`, `UserStatus`, `Reactions`, `FoldersChanged`, `NotifyChanged`, `StickerSetsChanged`, `SecretChat(Requested|Accepted|KeyChanged|TtlChanged|Discarded|Screenshot)`, `SecretMessage`, `SecretMessagesDeleted`, `Transfer{id, done, total}`, `Resync{reason}`.

Errores: `BackendError{ Network, FloodWait(secs), Unauthorized, NotFound, Forbidden(reason), InvalidInput(field), Crypto(..), Storage(..), Internal }`; la UI tiene un mensaje para cada uno.

Webhooks: no aplica (cliente). Entradas push: el flujo de updates de MTProto. Salidas: ninguna.

## 4. Flujo de datos y concurrencia

Un solo runtime: iced con la feature `tokio` usa un runtime tokio multihilo como ejecutor; todo `Task` y `Subscription` corre ahí. `tc-app` no crea un segundo runtime. Los backends se construyen dentro de la `Task` de arranque (ya dentro del runtime), así `tokio::spawn` funciona.

| actor | hilo / tarea | posee | habla con |
| --- | --- | --- | --- |
| UI | hilo principal (winit/iced) | estado de pantalla, ventana de mensajes en memoria | `Task::perform(backend.op())`, `Subscription` de eventos |
| Red | tareas tokio de grammers (runner del sender + bombeo de updates) | conexión, sesión, estado de updates (pts/qts) | emite `BackendEvent` |
| Bombeo de sincronización | 1 tarea tokio en `CachedBackend` | orden de aplicación | lee eventos del backend → escribe en store → reemite a la UI |
| Store | 1 hilo `std` dedicado | `rusqlite::Connection` (escritura) + 1 conexión de lectura | `mpsc` + `oneshot`; agrupa escrituras en una transacción cada ≤16 ms o 256 operaciones |
| Transferencias | tareas tokio limitadas por `Semaphore` (4 descargas, 2 subidas; partes de 512 KiB) | progreso | eventos `Transfer` a ≤10 Hz |
| Decodificación | `spawn_blocking` | WebP/fotos/fotogramas Lottie | devuelve RGBA a la UI |
| Temporizador TTL | 1 tarea tokio | próximo `destroy_at` de `secret_messages` | borra y emite `SecretMessagesDeleted` |

Camino de lectura (abrir chat): UI pide `history` → `CachedBackend` responde **primero desde SQLite** (pintado inmediato, funciona offline) y en paralelo pide al backend; las diferencias llegan como eventos y se aplican por `(chat_key, msg_id)`.

Camino de escritura (enviar): la UI genera `random_id`, inserta optimista `send_state='pending'` (id local negativo) y llama `send_text`. El backend confirma con `MessageSent{random_id, msg}` → se sustituye la fila pendiente por la real (mismo `random_id`, índice único). Fallo → `failed` + botón reintentar (mismo `random_id`, el servidor deduplica).

Orden y contrapresión: el bombeo es secuencial, así que el orden por chat se mantiene. El canal hacia la UI es acotado (1024). Los eventos efímeros (`Typing`, `UserStatus`, `Transfer`) se coalescen (gana el último por clave). Si el canal se llena, el bombeo no bloquea la red: descarta efímeros y, si aún falta sitio, marca `Resync` y la UI recarga lo visible desde el store.

Cancelación: `Task::abortable` en iced; la UI guarda el handle de cada subida/descarga. Soltar el futuro cancela la transferencia; la subida queda `paused` en `uploads` para reanudar.

Regla de oro: `update()` de iced nunca bloquea ni hace E/S.

## 5. MockBackend (tc-mock)

- Datos deterministas con semilla fija (LCG propio, sin dependencia `rand`): 200 diálogos (privados, grupos, canales, 2 chats secretos), un chat con 10 000 mensajes para estresar el scroll, medios simulados con dimensiones reales, el pack de stickers original (fase brand) marcado `is_local`.
- Latencia configurable (`TC_MOCK_LATENCY_MS`, por defecto 80 ms con jitter), tasa de fallos (`TC_MOCK_FAIL_RATE`), modo offline conmutable, `FloodWait` simulado.
- Guion de tiempo real: mensaje entrante cada N s, "escribiendo…", reacciones ajenas, ediciones, borrados.
- Login simulado: el código de prueba vive como constante en `tc-mock` (valor de test, nunca una credencial real).
- E2E: dos cuentas simuladas en el mismo proceso ejecutan `tc-secret` de verdad (DH, cifrado, secuencias, re-keying, TTL), con un primo de prueba de 2048 bits fijo en el código del mock.
- Es también el doble de pruebas de `tc-ui` y `tc-store`.

## 6. Chats secretos (tc-secret) — EXPERIMENTAL

Según la documentación pública de cifrado extremo a extremo (core.telegram.org/api/end-to-end). `grammers` no lo trae: comprobado que `grammers-tl-types` 0.11 solo genera `api.tl` y `mtproto.tl` (no hay tipos `decryptedMessage*`), aunque sí las funciones de transporte (`messages.requestEncryption`, `acceptEncryption`, `sendEncrypted*`, `updateNewEncryptedMessage`…) y grammers entrega updates sin interpretar como `Update::Raw`.

Diseño sans-IO: `tc-secret` recibe bytes y eventos, devuelve bytes y "acciones a enviar". `tc-mock` y `tc-telegram` hacen la E/S. Módulos:

| módulo | contenido |
| --- | --- |
| `dh` | validar `messages.getDhConfig` (p primo seguro de 2048 bits, g y sus condiciones; cachear el p validado), generar `a` mezclando aleatorio local (`getrandom`) con el del servidor, `g_a`, comprobar rangos `1 < g_a < p-1` y la cota de 2^(2048-64) |
| `key` | `SecretKey([u8; 256])` con `Zeroize` al soltarse; `key_fingerprint` = 64 bits bajos de SHA1(clave); visualización (bits de SHA1 y SHA256 de la clave → rejilla + emoji). Contrastar el detalle exacto con la doc |
| `crypto` | MTProto 2.0: `msg_key` desde SHA256 de una parte de la clave + texto + relleno; derivación AES-256-IGE con x = 0 (iniciador) u 8 (receptor); relleno aleatorio 12–1024 bytes; comparación de `msg_key` en tiempo constante (`subtle`) |
| `tl` | serializador propio del subconjunto de la capa secreta: `DecryptedMessageLayer`, `DecryptedMessage`, `DecryptedMessageService` y acciones (`NotifyLayer`, `SetMessageTTL`, `ReadMessages`, `DeleteMessages`, `ScreenshotMessages`, `FlushHistory`, `Resend`, `RequestKey/AcceptKey/CommitKey/AbortKey`, `Noop`), medios básicos |
| `session` | máquina de estados por chat: `requested → waiting → accepted → discarded`; números de secuencia in/out con paridad por iniciador, detección de huecos (`Resend`), descarte de duplicados, negociación de capa, re-keying con PFS cada 100 mensajes o 1 semana |
| `files` | archivos cifrados con clave/iv por archivo y su huella; en disco quedan siempre cifrados |

Garantías locales: `#![forbid(unsafe_code)]`; la clave de cada chat solo existe en el keyring (`acct:<id>:secret:<chat>`) y en memoria con `Zeroize`; nunca en logs (`tracing` sin campos de claves); cuerpos en SQLite cifrados con AES-256-GCM (clave local en el keyring); sin FTS; `PRAGMA secure_delete=ON`; ligado al dispositivo (no se reenvía ni se sincroniza). Sin keyring disponible no se pueden crear chats secretos (no hay modo en claro).

Verificación: pruebas de ida y vuelta entre las dos cuentas del mock, pruebas de propiedades sobre secuencias/huecos, vectores de prueba obtenidos de caja negra con otro cliente (nunca copiando su código), y la skill `security-review` antes de quitar la etiqueta "experimental".

## 7. Persistencia (tc-store)

Ubicación (XDG vía `directories`): `$XDG_DATA_HOME/<app>/accounts/<account_id>/cache.sqlite3` (0600), `…/media/` (caché de archivos `MediaCache`: directorio 0700, archivos 0600, índice `file_cache` en SQLite, LRU con tope de 512 MiB; nunca guarda nada de chats secretos; migración 0004). Configuración en `$XDG_CONFIG_HOME/<app>/config.toml`.

Al abrir: `journal_mode=WAL`, `synchronous=NORMAL`, `foreign_keys=ON`, `secure_delete=ON`, `busy_timeout=5000`; luego se aplican las migraciones pendientes comparando `PRAGMA user_version` con la lista `migrations/NNNN_*.sql` (`include_str!`), cada una en su transacción.

Tablas (17 + 1 virtual FTS5), detalle en [`0001_init.sql`](../crates/tc-store/migrations/0001_init.sql), validado con `sqlite3` (carga sin errores, `foreign_key_check` limpio):

| tabla | clave | índices (además de PK/UNIQUE) | notas |
| --- | --- | --- | --- |
| `meta` | key | — | cuenta propia, versión de datos |
| `users` | id | username, contactos (parcial), foto | absorbe `Contact` (is_contact, saved_name) |
| `groups` | id | foto | grupo, supergrupo, canal |
| `secret_chats` | id | peer_user_id, state | estado E2E; `key_ref` al keyring, nunca la clave |
| `dialogs` | `chat_key` local | (archived, pinned_order, last_message_at DESC), no leídos (parcial) | UNIQUE(peer_kind, peer_id): espacios de ids distintos |
| `media` | id | (download_state, last_used_at) | LRU; `is_secret` → siempre cifrado en disco |
| `messages` | rowid; UNIQUE(chat_key, msg_id) | random_id (único parcial), (chat_key, date), álbum, media, sticker, pendientes, remitente | la UNIQUE sirve la paginación `msg_id < ? ORDER BY msg_id DESC` |
| `messages_fts` | FTS5 contenido externo | — | triggers de insert/update/delete; `unicode61 remove_diacritics 2` |
| `reactions` | (chat_key, msg_id, emoji) | — | FK compuesta a messages, cascada |
| `secret_messages` | rowid; UNIQUE(secret_chat_id, random_id) | (secret_chat_id, date), destroy_at (parcial), media | cuerpo AES-GCM, TTL |
| `sticker_sets` | id; UNIQUE(short_name) | instalados (parcial), thumb | `is_local` para el pack propio |
| `stickers` | id | (set_id, position), emoji, media | formato webp/tgs/webm |
| `sticker_recent` / `sticker_faves` | sticker_id | por fecha DESC | selector |
| `folders` / `folder_peers` | id / (folder_id, chat_key, mode) | sort_order / chat_key | filtros por tipo como columnas |
| `notify_settings` | (scope, chat_key) | — | CHECK: chat_key solo con scope='chat' |
| `uploads` | local_id | state, chat_key, secret_chat_id | subida reanudable por partes |

Adaptaciones a las reglas de la skill (pensadas para Postgres en servidor): ids de la red (enteros) en lugar de UUID; tiempos `INTEGER` UTC en vez de `timestamptz`; sin columna de propietario porque cada archivo es de una cuenta; sin RLS: el acceso real lo decide el servidor y localmente lo protegen permisos 0600 + keyring; no hay dinero. Cada `ON DELETE` está decidido explícitamente (cascada en hijos, `SET NULL` en medios opcionales, `RESTRICT` del usuario de un chat secreto).

Bóveda (keyring, servicio con el nombre provisional de la app; se migra al renombrar en replica-brand): `acct:<id>:mtproto-session` (SessionData de grammers), `acct:<id>:at-rest` (clave AES-GCM local), `acct:<id>:secret:<chat_id>`, `acct:<id>:secret-file:<media_id>`. Si no hay Secret Service: la sesión normal cae a archivo 0600 con aviso visible; los chats secretos se desactivan.

`CachedBackend<B: ChatBackend>` implementa `ChatBackend`: lecturas desde caché + refresco, eventos escritos antes de reenviarse. La UI no sabe que existe.

## 8. Stickers (tc-stickers + widget en tc-ui)

| formato | cómo | prioridad |
| --- | --- | --- |
| WebP estático | `image` (decodificador `image-webp`, Rust puro) → RGBA8 → `iced::widget::image::Handle::from_rgba` | must |
| TGS animado | `flate2` (gunzip con tope de 4 MiB descomprimidos contra bombas) → JSON Lottie → `rlottie` (feature `lottie`) renderiza el fotograma al tamaño pedido → RGBA | should |
| WebM vídeo | aplazado (decodificador de vídeo); se muestra la miniatura | could |

Caché de fotogramas LRU con tope (64 MiB por defecto) clave `(sticker_id, tamaño, fotograma)`. Solo se animan los stickers visibles; pausa con la ventana sin foco o con "reducir movimiento" (ADR-0002). Si `lottie` no está compilada, TGS se muestra como miniatura estática. No se empaquetan stickers de Telegram; el pack propio original vive en `assets/stickers/` con licencia propia (fase brand).

## 9. UI (tc-ui)

Estructura de módulos:

```
tc-ui/src/
  app.rs              State, Message, update, view, subscription (eventos del backend)
  route.rs            enum Screen / Overlay
  style/              tc-theme → iced Theme + estilos por widget
  widget/             virtual_list, bubble, rich_text (entidades), avatar, badge, reaction_chips,
                      context_menu, toast, sticker_view, key_visual
  screens/auth/       welcome, phone, qr, code, password, signup                 (S01–S06)
  screens/main/       shell, chat_list, folder_bar, conversation, header, composer (S07–S09, S11–S13)
  screens/dialogs/    message_menu, reactions, forward, delete, multiselect, attach, send_files,
                      new_chat, folder_editor                                      (S10, S14–S18, S20, S21, S36)
  screens/search/     global, in_chat                                              (S22, S23)
  screens/info/       user, group, shared_media                                    (S24, S25)
  screens/settings/   menu, profile, notifications, privacy, sessions, appearance,
                      storage, language, stickers                                  (S26–S33)
  screens/media_viewer.rs                                                          (S19)
  screens/stickers/   picker, set_preview                                          (S38, S39 propuestas)
  screens/secret/     key_fingerprint, ttl_picker, secret_banner                   (S40, S41 propuestas)
  screens/contacts.rs                                                              (S37)
  platform/           notificaciones (S34), bandeja (S35), portal de archivos — M2
```

Pantallas nuevas propuestas por ADR-0004 (el recon no las tiene; añadirlas en `recon.md`): S38 selector de stickers, S39 vista de pack (instalar/quitar), S40 huella de clave del chat secreto, S41 selector de temporizador de autodestrucción.

**Scroll virtual de mensajes.** iced 0.14 no trae lista virtual, así que se construye en `widget/virtual_list.rs`:

1. Modelo: ventana en memoria de hasta ~600 mensajes del chat abierto (páginas de 50), cada uno con altura medida o estimada. Suma de prefijos en un árbol de Fenwick para pasar de desplazamiento a índice en O(log n).
2. Medición: texto con `iced::advanced::text::Paragraph` (mismo motor que el render) para el ancho actual de burbuja; medios con dimensiones conocidas; caché de alturas por `(msg_id, ancho, escala de texto)`; se invalida al cambiar ancho o escala.
3. Render: solo los elementos visibles más un margen de una pantalla, con espaciadores arriba y abajo. Primera versión con `scrollable` + `on_scroll(Viewport)`; si no da 60 fps con 10 000 mensajes en `wgpu` y `tiny-skia`, se pasa a un `Widget` propio (feature `advanced`) que solo hace layout de los hijos visibles.
4. Anclaje abajo: la conversación se ancla al final; añadir historial antiguo arriba no mueve lo visible. Si el usuario está lejos del final, los nuevos suben el contador del botón "ir al final".
5. Precarga: a 1,5 pantallas del borde superior se pide la página anterior (store primero, red después). Se descartan páginas lejanas del lado contrario.
6. Separadores de fecha calculados en hora local (`jiff`) al construir la ventana; salto al primer no leído con `Anchor::around(read_inbox_max_id)`.

La lista de chats tiene filas de altura fija: virtualización por índice directo.

Texto: `rich_text` + `span` de iced para entidades (negrita, cursiva, código, enlaces); spoiler como widget propio; emoji vía fuentes del sistema (fallback de cosmic-text a Noto Color Emoji). Compositor: `text_editor` con `key_binding` (Enter envía, Shift+Enter nueva línea). IME en Wayland: probar en M1.3.

## 10. Arranque y configuración (tc-app)

| variable | uso |
| --- | --- |
| `TC_BACKEND` | `mock` (por defecto) o `telegram` |
| `TC_API_ID`, `TC_API_HASH` | obligatorias solo con `telegram`; las obtiene el usuario en my.telegram.org |
| `TC_DATA_DIR` | sustituye la ruta XDG de datos |
| `TC_MOCK_LATENCY_MS`, `TC_MOCK_FAIL_RATE` | ajustes del mock |
| `RUST_LOG` | filtro de `tracing` |

Plantilla: [`.env.example`](../.env.example) (mismos nombres `TC_API_ID`/`TC_API_HASH`); el `.env` local del usuario (ignorado por git) se cargará en M2.1 con `dotenvy` 0.15.7 (versión comprobada, aún no añadida), sin sobrescribir variables ya definidas.

Precedencia: variables de entorno > `.env` > `config.toml` del usuario > valores por defecto. Las credenciales nunca se escriben en el repo ni se registran en logs; `.gitignore` excluye `.env`, `*.session`, `*.sqlite3` y `/config.toml`, `/config.local.toml` en la raíz. Si `TC_BACKEND=telegram` y faltan, la app arranca en una pantalla que explica qué falta, sin inventar valores.

## 11. Módulos y operaciones por flujo

| flujo | pantallas | módulos tc-ui | operaciones `ChatBackend` | tablas | eventos |
| --- | --- | --- | --- | --- | --- |
| F01 login teléfono | S01, S02, S04, S05, S06 → S07 | `auth::{welcome,phone,code,password,signup}` | `auth_state`, `request_login_code`, `resend_code`, `sign_in`, `check_password`, `sign_up` | `meta`, `users` | `Auth`, `Connection` |
| F02 login QR | S02, S03 → S07 | `auth::qr` (widget `qr_code`) | `qr_login_start`, `check_password` | `meta` | `Auth(QrAccepted)` |
| F03 abrir y leer | S07, S08, S11, S12 | `main::{shell,chat_list,conversation,header}`, `widget::virtual_list` | `dialogs`, `history`, `mark_read` | `dialogs`, `messages`, `users`, `groups`, `media` | `DialogUpserted`, `ReadInbox` |
| F04 enviar texto | S11, S13 | `main::composer` | `send_text`, `save_draft`, `set_typing` | `messages`, `dialogs` | `MessageSent`, `MessageFailed` |
| F05 responder | S14, S13 | `dialogs::message_menu`, `main::composer` | `send_text{reply_to}` | `messages` | `MessageNew` |
| F06 reenviar | S14, S18, S16 | `dialogs::{forward,multiselect}` | `forward_messages` | `messages` | `MessageNew` |
| F07 editar | S14, S13 | `main::composer` (modo edición) | `edit_message` | `messages`, `messages_fts` | `MessageEdited` |
| F08 borrar | S14, S17, S18 | `dialogs::{delete,multiselect}` | `delete_messages` | `messages` | `MessagesDeleted` |
| F09 reaccionar | S14, S15 | `dialogs::reactions`, `widget::reaction_chips` | `send_reaction` | `reactions` | `Reactions` |
| F10 archivo/foto | S13, S20, S21, S19 | `dialogs::{attach,send_files}`, `media_viewer`, `platform` (portal) | `upload`, `send_media`, `download`, `thumbnail` | `media`, `uploads`, `messages` | `Transfer`, `MessageSent` |
| F11 buscar | S08, S22, S23 → S11 | `search::{global,in_chat}` | `search_global`, `search_in_chat`, `history{around}` | `messages_fts`, `dialogs`, `users` | — |
| F12 carpetas | S26, S10, S09, S08 | `dialogs::folder_editor`, `main::folder_bar` | `folders`, `save_folder`, `delete_folder`, `reorder_folders` | `folders`, `folder_peers` | `FoldersChanged` |
| F13 info de chat | S12, S24, S25 | `info::{user,group,shared_media}` | `user_full`, `group_full`, `set_blocked`, `search_in_chat{media}` | `users`, `groups`, `media` | `UserStatus` |
| F14 notificaciones | S26, S28, S08, S12 | `settings::notifications`, `platform` | `notify_settings`, `set_notify_settings`, `set_muted` | `notify_settings`, `dialogs` | `NotifyChanged` |
| F15 sesiones | S26, S29, S33 | `settings::{privacy,sessions}` | `sessions`, `terminate_session`, `privacy`, `set_privacy` | — (no se cachea) | — |
| F16 tiempo real | S08, S34, S35 | `app::subscription`, `platform` | `events()` | todas las de mensajes | todos; `Resync` tras reconexión |
| F17 stickers (nuevo, ADR-0004) | S13, S38, S39, S11 | `stickers::{picker,set_preview}`, `widget::sticker_view`, `settings::stickers` | `installed_sticker_sets`, `sticker_set`, `install_sticker_set`, `uninstall_sticker_set`, `reorder_sticker_sets`, `recent_stickers`, `faved_stickers`, `set_sticker_faved`, `stickers_for_emoji`, `send_sticker`, `download` | `sticker_sets`, `stickers`, `sticker_recent`, `sticker_faves`, `media` | `StickerSetsChanged` |
| F18 chat secreto (nuevo, ADR-0004) | S24 → S11 (variante secreta), S40, S41 | `secret::{key_fingerprint,ttl_picker,secret_banner}`, `main::conversation` | `request_secret_chat`, `accept_secret_chat`, `discard_secret_chat`, `send_secret_text`, `send_secret_media`, `set_secret_ttl`, `mark_secret_read`, `secret_key_visual` | `secret_chats`, `secret_messages`, `media`, `uploads` + keyring | `SecretChat(..)`, `SecretMessage`, `SecretMessagesDeleted` |

Trabajos en segundo plano: recolector LRU de la caché de medios (al arrancar y cada hora), temporizador TTL de chats secretos (por evento), re-keying de chats secretos (al cumplir 100 mensajes o 1 semana), reintento de pendientes al reconectar, reanudación de `uploads` al arrancar.

## 12. Las partes que muerden

- **Zonas horarias y DST:** todo se guarda en UTC; separadores de fecha y "hoy/ayer" se calculan en hora local con `jiff` (no con desplazamiento fijo) y se recalculan a medianoche.
- **Idempotencia:** `random_id` en cada envío (índice único); los eventos pueden repetirse tras `getDifference`: todo se aplica como upsert por `(chat_key, msg_id)`.
- **Carreras:** edición o borrado que llega antes que el mensaje (se ignora si no existe; el refresco de historial lo corrige); confirmación de envío que llega antes que la respuesta RPC (se casa por `random_id`); dos ventanas no existen (una instancia por cuenta, bloqueo de archivo).
- **Límites de velocidad:** `FLOOD_WAIT` se muestra con cuenta atrás (S02) y nunca se reintenta en bucle; grammers duerme automáticamente por debajo de un umbral configurable.
- **Tamaños:** texto 4096 caracteres; archivos hasta 2 GB; partes de 512 KiB; validación antes de subir.
- **Búsqueda:** FTS5 local para lo cacheado + servidor para lo demás; los chats secretos solo se buscan en memoria.
- **Tiempo real y huecos:** el estado pts/qts lo lleva grammers en su sesión; al reconectar se emite `Resync` y se refrescan los diálogos visibles. Confirmar en M2 que grammers 0.11 sigue `qts` (necesario para mensajes cifrados).
- **Offline:** la caché pinta todo; envíos quedan `pending` y salen al reconectar.
- **Multicuenta:** un directorio y una base por cuenta desde el día uno (aunque la UI tenga una sola cuenta al principio).
- **Borrado/RGPD:** cerrar sesión borra el directorio de la cuenta y sus entradas del keyring; "vaciar caché" en S31.
- **Criptografía propia (E2E):** la parte de mayor riesgo; aislada en `tc-secret`, sans-IO, etiquetada experimental, con revisión de seguridad.
- **Términos de la API:** `api_id` propio del usuario, marca propia, sin suplantar al cliente oficial; solo API pública.
- **Render:** `tiny-skia` es mucho más lento que `wgpu`; el scroll virtual y el número de stickers animados deben medirse en ambos.
- **Wayland/IME/emoji:** probar entrada CJK y emoji en el compositor desde M1.3.

## 13. Orden de construcción (rebanada vertical delgada primero)

| hito | qué | pantallas | tablas | operaciones | listo cuando |
| --- | --- | --- | --- | --- | --- |
| **M0** (hecho) | esqueleto del workspace | — | — | — | `cargo check` pasa |
| **M1.1** lista de chats | tipos mínimos de `tc-core`, `MockBackend` con datos semilla, tokens de `tc-theme`, shell + lista | S07, S08 | — (en memoria) | `dialogs`, `events` | la app abre y muestra 200 chats ordenados que se reordenan al llegar un mensaje simulado |
| **M1.2** conversación | historial paginado, scroll virtual, separadores, salto a no leídos, cabecera | S11, S12 | — | `history`, `mark_read` | 10 000 mensajes se desplazan fluidos en wgpu y tiny-skia |
| **M1.3** compositor | Enter/Shift+Enter, envío optimista, pendiente→enviado→error→reintento, entrante en vivo | S13 | — | `send_text`, `set_typing` | bucle completo enviar/recibir contra el mock |
| **M1.4** caché | `tc-store`, migración 0001, `CachedBackend`, arranque en frío desde SQLite, banner offline | — | `dialogs`, `messages`, `users`, `groups`, `media` | — | relanzar la app pinta al instante sin backend |
| **M2.1** red real | `tc-telegram` con grammers: login teléfono/código/2FA, QR, sesión en keyring; la rebanada M1 contra Telegram | S01–S05 | `meta` | Auth completo | con `api_id`/`api_hash` del usuario se ve la lista real y se envía un mensaje |
| **M2.2** acciones | responder, reenviar, editar, borrar, copiar, reacciones, menú contextual | S14–S17 | `reactions` | F05–F09 | — |
| **M2.3** adjuntos | selector, arrastrar, envío con pie, progreso/cancelar, visor, descarga | S19–S21 | `uploads` | F10 | — |
| **M2.4** stickers WebP + selector | decodificación WebP, selector con recientes/favoritos, envío | S38 | `sticker_*` | F17 (must) | — |
| **M2.5** búsqueda | global y en chat | S22, S23 | `messages_fts` | F11 | — |
| **M2.6** resto de must | carpetas, info, perfil, notificaciones del SO, tema, registro, crear chat | S09, S10, S24, S26–S28, S30, S34, S36 | `folders`, `notify_settings` | F12–F14, F16 | todas las filas `must` de `features.csv` marcadas |
| **M2.7** chats secretos (must) | `tc-secret` con vectores, E2E en el mock entre 2 cuentas, luego Telegram; huella, TTL, texto cifrado | S40, S41 | `secret_*` | F18 | revisión de seguridad hecha; sigue tras bandera "experimental" |
| **M3** should | fijados, escribiendo, borradores, filtros de carpetas, contactos, grupos, formato, notas de voz, álbumes, servicio, Lottie, instalar packs, pack propio, medios E2E, sesiones, bandeja, idioma, atajos | — | — | — | — |
| **M4** could | archivar, vista previa de enlaces, GIF, ubicación, encuestas, WebM, búsqueda por fecha, responder desde notificación, aviso de captura | — | — | — | — |
| **M5** mejoras | lo que encuentre `replica-entrepreneur` | — | — | — | — |

## 14. Riesgos principales y siguiente paso

1. **E2E propio** (`tc-secret`): criptografía sin auditoría externa. Mitigación: sans-IO, vectores, revisión de seguridad, bandera experimental.
2. **Scroll virtual + texto enriquecido en iced** con miles de mensajes y respaldo `tiny-skia`. Mitigación: spike de rendimiento al inicio de M1.2, con plan B de widget propio.
3. **Tiempo real con huecos/reconexión y `qts`** sobre grammers 0.11 (y QR login en crudo). Mitigación: probar pronto en M2.1 con `Update::Raw` y funciones TL crudas.

Siguiente paso: `/replica-design` (ya en curso por el agente de diseño, ADR-0002) y después M1.1 con `/replica-build`.
