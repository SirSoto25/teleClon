# Informe de barrido de marca

Generado: 2026-11-06 por `tools/brand/sweep.py` (raíz `.`).

Busca: telegram, tdesktop, durov, t.me, tg://. Excluye .git, target, .venv, graphify-out, .claude.

## Resumen

| Clase | Coincidencias |
| --- | ---: |
| BLOQUEANTE | 2 |
| REVISAR | 4 |
| PERMITIDA | 147 |

## Bloqueantes (texto visible o assets)

| Archivo | Línea | Texto |
| --- | ---: | --- |
| `README.md` | 3 | `Cliente de mensajería de escritorio para Linux escrito en Rust, inspirado en los patrones de UX de Telegram Desktop (clean-room: sin código, assets ni marca de ` |
| `crates/tc-app/src/main.rs` | 29 | `Ok("telegram") => tracing::warn!("TC_BACKEND=telegram aún no está disponible (hito M2.1); se usa el backend de pruebas"),` |

## A revisar (código que podría llegar a pantalla)

| Archivo | Línea | Texto |
| --- | ---: | --- |
| `Cargo.toml` | 71 | `# Red de Telegram (crate tc-telegram, hito M2.1). grammers-crypto se usa directamente para el SRP de 2FA` |
| `crates/tc-stickers/src/webp.rs` | 9 | `/// Lado máximo aceptado (los stickers de Telegram miden 512).` |
| `crates/tc-app/src/main.rs` | 2 | `//! (por ahora siempre `MockBackend`; `TC_BACKEND=telegram` llegará en M2.1), registra las fuentes` |
| `crates/tc-telegram/Cargo.toml` | 3 | `description = "GrammersBackend: ChatBackend sobre la red de Telegram (MTProto vía grammers 0.11)."` |

## Permitidas (resumen por archivo)

Nombres de crate, protocolo, URLs de documentación y docs de clean-room.

| Archivo | Coincidencias |
| --- | ---: |
| `docs/logs/log.md` | 20 |
| `replica/recon.md` | 20 |
| `docs/architecture.md` | 13 |
| `crates/tc-telegram/src/config.rs` | 9 |
| `crates/tc-telegram/src/auth.rs` | 8 |
| `crates/tc-telegram/src/lib.rs` | 7 |
| `tools/brand/sweep.py` | 6 |
| `crates/tc-telegram/src/net.rs` | 5 |
| `docs/design/system.md` | 5 |
| `crates/tc-telegram/src/tests/mod.rs` | 4 |
| `docs/adr/0003-arquitectura.md` | 4 |
| `replica/features.csv` | 4 |
| `crates/tc-secret/README.md` | 3 |
| `docs/00-INDEX.md` | 3 |
| `docs/adr/0004-stickers-y-chats-secretos.md` | 3 |
| `docs/agents/registry.md` | 3 |
| `Cargo.toml` | 2 |
| `crates/tc-secret/src/dh.rs` | 2 |
| `crates/tc-telegram/Cargo.toml` | 2 |
| `crates/tc-telegram/src/backend.rs` | 2 |
| `crates/tc-telegram/src/convert.rs` | 2 |
| `crates/tc-telegram/src/events.rs` | 2 |
| `crates/tc-telegram/src/ids.rs` | 2 |
| `crates/tc-telegram/src/session.rs` | 2 |
| `crates/tc-telegram/src/tests/flows.rs` | 2 |
| `docs/adr/0001-stack.md` | 2 |
| `Cargo.lock` | 1 |
| `crates/tc-mock/src/lib.rs` | 1 |
| `crates/tc-mock/src/tests.rs` | 1 |
| `crates/tc-secret/src/lib.rs` | 1 |
| `crates/tc-telegram/src/media.rs` | 1 |
| `crates/tc-telegram/src/peers.rs` | 1 |
| `crates/tc-telegram/src/tests/fixtures.rs` | 1 |
| `crates/tc-telegram/src/tests/mapping.rs` | 1 |
| `docs/adr/0002-direccion-visual.md` | 1 |
| `docs/research/skills-used.md` | 1 |
