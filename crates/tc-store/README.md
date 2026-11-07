# tc-store

Caché local de Parlo: SQLite (WAL) por cuenta, repositorios síncronos, `CachedBackend` y bóveda de secretos.

## Contenido

| Módulo | Qué hace |
|---|---|
| `Store` (`db.rs`) | Abre `<datos>/accounts/<cuenta>/cache.sqlite3` (0600), PRAGMAs (WAL, `foreign_keys`, `secure_delete`) y migraciones con `PRAGMA user_version`. |
| Repositorios | usuarios, grupos, diálogos, borradores, ajustes de notificación, mensajes (paginación por ancla, FTS5), reacciones, carpetas, subidas reanudables, stickers, chats secretos (solo blobs cifrados). |
| `CachedBackend<B>` | Decorador de `ChatBackend` (ver abajo). |
| Bóveda (`vault.rs`) | `SecretVault`, `KeyringVault`, `MemoryVault`. |
| Sellado (`seal.rs`) | `SealKey`: AEAD AES-256-GCM con clave aleatoria de 256 bits guardada en la bóveda (`load_or_create`). |

Migraciones (`migrations/`): `0001_init.sql` (esquema completo), `0002_groups_folders_reactions_uploads.sql`
(`dialogs.title`, `messages.sticker_json`, `uploads.last_error` e índices) y `0003_secret_chat_states.sql` (tabla
`secret_chat_states`: estado de protocolo de los chats secretos, solo cifrado). Una base v1/v2 se actualiza sin perder datos.

## CachedBackend

```rust
let store = Store::open_account(default_data_dir()?, "cuenta-1")?;
let backend = CachedBackend::new(inner_backend, store);   // implementa ChatBackend
```

* `dialogs` / `history`: si la caché basta (página completa, o historial/lista marcados como completos y, para
  «últimos mensajes», al día respecto al último mensaje conocido del diálogo) se devuelve al instante y se refresca
  en segundo plano; lo que cambió sale por `events()` como `DialogUpserted`, `MessageNew`, `MessageEdited` o
  `MessagesDeleted`. Si no basta, se espera al backend; ante `Network`/`FloodWait` se sirve lo cacheado.
* Escrituras: al backend primero; si salen bien, se reflejan en la base (editar, borrar, reacción propia, leído,
  fijar/archivar/silenciar). `send_text` deja antes un **pendiente local** (id negativo derivado del `random_id`)
  que se convierte en el mensaje real al confirmarse y pasa a `Failed` si falla (`unsent_messages` para reintentar).
  Los borradores se guardan en local antes de enviarlos.
* `events()`: cada evento se persiste al entregarse (solo mientras alguien consume el stream), mezclado con los
  eventos de refresco de la propia caché.
* `search_local(query, peer, limit)`: FTS5 (sin acentos ni mayúsculas, prefijo en la última palabra).
* `with_store(|s| ...)`: acceso a los demás repositorios (carpetas, subidas...) desde código async.

**Chats secretos**: nunca se escribe su contenido en `messages`, FTS ni borradores (tablas en claro); sus diálogos
se guardan sin último mensaje. Las llamadas de `SecretChatsApi` pasan íntegras al backend.

## Bóveda de secretos

`SecretVault` (`get`/`set`/`delete` por nombre, valores en `Zeroizing`). Nombres usados: `"session"` (sesión de
tc-telegram: `save_session`/`load_session`/`clear_session`) y `"secret-chat/<id>"` (clave de un chat secreto;
`save_secret_chat_key` devuelve ese nombre para anotarlo en `SecretChatRow.key_ref`). **Las claves nunca van a
SQLite en claro**: la base solo guarda la referencia.

* `KeyringVault` (feature `keyring-vault`, por defecto): `keyring` 4.x → Secret Service (D-Bus, `zbus`, Rust puro),
  Keychain, Credential Manager. Compila sin D-Bus; sin servicio en ejecución devuelve `VaultError::Unavailable`
  (nunca entra en pánico) y el llamador decide el respaldo (p. ej. el `FileSessionStore` 0600 de tc-telegram).
  `KeyringVault::is_available()` lo comprueba. Sin la feature solo existe `MemoryVault` y no se enlaza `keyring`.
* `MemoryVault`: pruebas y modo efímero.

## Sellado y estado cifrado de chats secretos

`SealKey::load_or_create(vault, SECRET_STATE_KEY)` lee (o crea con `getrandom` y guarda) la clave
`"secret-state-key"`; `seal(aad, texto)` devuelve nonce aleatorio de 12 bytes + cifrado‖etiqueta y `open` rechaza
(`SealError::Rejected`) cualquier blob manipulado, truncado, con otra clave o con otros datos asociados. La clave va en
`Zeroizing` y su `Debug` no la muestra. Sin bóveda (`VaultError::Unavailable`) no hay clave: el llamador no persiste.

Repositorio `secret_chat_states` (`put_/get_/list_/delete_secret_chat_state`, fila `SecretChatStateRow`): solo
acepta blobs sellados (nonce de 12 bytes, al menos la etiqueta). Lo usa el adaptador `SealedSecretStateStore` de
tc-app, que sella el `StoredSecretChat` de tc-telegram (metadatos + `SessionSnapshot`) con el id del chat en los datos
asociados (un blob copiado a otra fila no descifra).

## Decisiones y límites

* **Cifrado en reposo de la base: no se hace.** SQLCipher exige `bundled-sqlcipher` de rusqlite, que sustituye a
  `bundled` (el workspace usa `bundled`) y arrastra OpenSSL o su compilación vendorizada; además cambiaría el
  features del workspace y la gestión de la clave maestra en el arranque. A cambio: archivo 0600, `secure_delete`,
  claves solo en el keyring, cuerpos de chats secretos cifrados con AES-256-GCM (clave del keyring) y fuera del FTS.
  Recomendación: cifrado de disco/`$HOME`. Si se quisiera, es un cambio localizado (`Store::init` + feature).
* Medios: la tabla `media` va por id de red; dos medios con el mismo id pisan sus metadatos (en Telegram son únicos;
  el `MockBackend` repite alguno).
* Sin detección exacta de huecos en el historial: la heurística (página completa + al día respecto al último
  mensaje del diálogo) puede servir un tramo con huecos en casos raros; el refresco en segundo plano lo corrige.
* Los diálogos borrados en el servidor no se eliminan de la caché al refrescar una página (solo se actualizan los que llegan).
* Los packs de stickers y el estado de protocolo de los chats secretos no pasan por `CachedBackend`.
* tc-store no depende de tc-telegram (ni al revés): los adaptadores `VaultSessionStore` (sesión en el llavero, clave
  `"session"`, respaldo `FileSessionStore` 0600 con aviso y migración única de `session.json`) y
  `SealedSecretStateStore` viven en `tc-app/src/stores.rs`.
* AEAD: AES-256-GCM (ya era dependencia del crate) en lugar de ChaCha20-Poly1305; nonces aleatorios de 96 bits
  (límite práctico ~2³² sellados por clave, muy por encima del uso).
* Si se pierde la clave del llavero (llavero reiniciado), los blobs existentes ya no descifran: se ignoran con aviso
  (no se borran) y se crea una clave nueva.
