# ADR-0003: Arquitectura del cliente (workspace, backend tras trait, caché SQLite, E2E sans-IO)

- **Estado:** aceptado
- **Fecha:** 2026-11-06
- **Autor:** agente arquitecto (skill `replica-skill:replica-architect`)
- **Detalle:** [../architecture.md](../architecture.md) · esquema: [`crates/tc-store/migrations/0001_init.sql`](../../crates/tc-store/migrations/0001_init.sql)
- **Relacionados:** ADR-0001 (stack, confirmado con correcciones), ADR-0002 (dirección visual), ADR-0004 (stickers y chats secretos en alcance)

## Contexto
teleClon es un cliente de escritorio Linux en Rust de la red de Telegram, con `api_id`/`api_hash` que aporta el usuario (entorno o config, nunca el repo). Hay que poder construir y probar toda la UI sin credenciales ni red. Tras el recon, el usuario amplió el alcance a stickers y chats secretos E2E, que `grammers` no implementa.

## Decisión
1. **Workspace Cargo de 8 crates** (edición 2024, resolver 3): `tc-core` (dominio + trait), `tc-mock`, `tc-store`, `tc-theme`, `tc-secret`, `tc-stickers`, `tc-ui`, `tc-app` (binario). `tc-telegram` (grammers) se crea en M2. Un solo proceso, sin servidor propio.
2. **`ChatBackend`** en `tc-core`, partido en sub-traits por área (auth, diálogos, mensajes, medios, búsqueda, perfil, ajustes, stickers, chats secretos) + un flujo único `events()`. Se usa como `Arc<dyn ChatBackend>` con `async-trait`. La UI solo conoce este trait.
3. **Mock primero:** `MockBackend` determinista (semilla, latencia y fallos inyectables, dos cuentas con E2E real). Es también el doble de pruebas.
4. **Caché como decorador:** `tc-store::CachedBackend<B>` implementa `ChatBackend`: lee de SQLite primero y refresca de la red; escribe cada evento antes de reenviarlo a la UI.
5. **SQLite por cuenta** (rusqlite 0.41 `bundled`, WAL, `secure_delete`, FTS5), migraciones manuales con `PRAGMA user_version`; ids de la red en lugar de UUID, tiempos `INTEGER` UTC, clave local `chat_key` para unificar usuarios, grupos y chats secretos. 17 tablas + 1 FTS5.
6. **Concurrencia:** un runtime tokio (el ejecutor de iced con feature `tokio`); un hilo dedicado dueño de la conexión SQLite con escrituras por lotes; una tarea de bombeo secuencial que preserva el orden; canal acotado a la UI con coalescencia de eventos efímeros y `Resync` en vez de bloquear la red. `update()` de iced nunca hace E/S.
7. **Scroll virtual propio** en `tc-ui` (iced no lo trae): ventana de ~600 mensajes, alturas medidas con `Paragraph`, suma de prefijos (Fenwick), anclaje inferior; plan B con `Widget` propio (feature `advanced`).
8. **Chats secretos en `tc-secret`**, sans-IO, con primitivas RustCrypto (`aes` + `ige`, `sha1`, `sha2`, `num-bigint`, `getrandom`, `zeroize`, `subtle`), serializador TL propio de la capa secreta (grammers-tl-types 0.11 solo trae `api.tl` y `mtproto.tl`, comprobado). Claves solo en el keyring (`keyring` 4.2, Secret Service); cuerpos de mensajes secretos cifrados en reposo con AES-256-GCM; sin FTS; etiqueta "experimental" hasta pasar revisión de seguridad.
9. **Stickers en `tc-stickers`:** WebP con `image` (Rust puro); TGS con `flate2` + `rlottie` tras la feature `lottie`; WebM aplazado. No se empaquetan stickers de Telegram.
10. **Orden de construcción:** rebanada vertical lista de chats → conversación → compositor contra el mock (M1.1–M1.3), después caché (M1.4), después red real (M2.1) y el resto de must por área; chats secretos al final de los must (M2.7).

## Alternativas descartadas
- **Backend propio en lugar de cliente de Telegram:** descartado por decisión del usuario.
- **`grammers-session` con `sqlite-storage`:** trae `libsql` (otro SQLite empaquetado) con riesgo de choque de símbolos frente a `libsqlite3-sys`; se usa sin esa feature y con `serde` para guardar `SessionData` en el keyring.
- **`rusqlite_migration`:** su 2.6.0 exige `rusqlite ^0.40`; con 0.41 no resuelve. Migraciones a mano (pocas líneas).
- **SQLCipher para toda la base:** exige OpenSSL y complica la compilación; se cifra solo lo de los chats secretos.
- **`velato` (Lottie sobre vello):** solo GPU, no encaja con el respaldo `tiny-skia`.
- **Trait con `async fn` nativo:** no es compatible con `dyn`; se necesita elegir backend en tiempo de ejecución.

## Consecuencias
- La UI se puede desarrollar y probar entera sin red; cambiar a la red real es cambiar `TC_BACKEND`.
- La caché añade complejidad de conciliación (upserts idempotentes por `(chat_key, msg_id)` y `random_id`).
- `tc-secret` es el mayor riesgo: criptografía propia sin auditoría externa.
- La feature `lottie` compila C++ (cmake, clang, g++); `cargo check` del esqueleto no la activa, pero se verificó aparte que compila.
- Verificado el 2026-11-06: `cargo check --workspace` pasa en Rust 1.99.1 con todas las dependencias del esqueleto; grammers 0.11 compila aparte junto a rusqlite empaquetado.
