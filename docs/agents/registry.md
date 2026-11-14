# Registro de agentes

Criterio: investigación/redacción estructurada → Sonnet (esfuerzo medio); decisiones de diseño/arquitectura y UI compleja → Opus (esfuerzo alto); tareas mecánicas → Haiku/Sonnet bajo. Orquestador: sesión principal.

| # | Agente | Modelo | Esfuerzo | Tarea | Salida | Estado |
| --- | --- | --- | --- | --- | --- | --- |
| 1 | recon | sonnet | medium | replica-recon de Telegram Desktop (fuentes públicas) | replica/recon.md, replica/features.csv | HECHO (37 pantallas, 16 flujos, 99 funciones; sin fetch en vivo, inferido de conocimiento previo; replica/screens vacío) |
| 2 | diseño | opus | high | identidad visual propia, tokens | docs/design/*, ADR-0002 | HECHO: dirección «Brasa» (grafito cálido + acento persimón), 144 pares AA sin fallos; scripts en tools/design |
| 3 | arquitecto | opus | high | stack, esquema, crates (tras recon) | docs/architecture.md, ADR-0003, migrations/0001_init.sql, esqueleto workspace (8 crates) | HECHO (cargo check OK; incluye tc-secret y tc-stickers por ADR-0004) |
| 4 | core+mock | sonnet | medium | crate de dominio + backend simulado | crates/tc-core, crates/tc-mock | HECHO (25 tests OK, clippy -D warnings limpio con toolchain 1.99.1; 29 chats, 5000 msgs, script tokio, secreto simulado sin cripto real) |
| 5 | UI | opus | high | app iced con tema: M1.1 lista, M1.2 conversación con scroll virtual, M1.3 compositor, chat secreto con huella | crates/tc-ui, crates/tc-app, docs/design/screens/ | HECHO (build, 57 tests y clippy -D warnings OK; app ejecutada en Wayland con wgpu y tiny-skia, 7 capturas; iconos Phosphor; sin verificar animaciones en vivo ni teclado real) |
| 6 | QA | sonnet | medium | build, clippy, tests | informe | pendiente |
| 7 | theme | sonnet | medium | crate de tokens: paletas Brasa, tipografía/fuentes, muelles, movimiento reducido, curvas de nivel por chat | crates/tc-theme | HECHO (12 tests, clippy -D warnings OK con toolchain 1.99.1) |
| 8 | stickers | sonnet | medium | tc-stickers: WebP→RGBA, TGS (gunzip + rlottie tras feature `lottie`), caché LRU por bytes, recientes/favoritos, pack original de 12 stickers procedurales | crates/tc-stickers | HECHO (10 tests OK con y sin `lottie`, clippy -D warnings limpio con y sin la feature; sin integrar en mock/UI) |
| 8 | STORE | sonnet | medium | tc-store: apertura+PRAGMAs, migraciones user_version, repos (diálogos, mensajes+FTS5, borradores, ajustes, stickers, chats secretos) | crates/tc-store (+ `directories` en su Cargo.toml) | HECHO (27 tests OK, clippy -D warnings OK con 1.99.1; sin CachedBackend ni keyring) |
| 9 | CRIPTO | opus | default | tc-secret: DH validado, MTProto 2.0 (msg_key SHA-256, AES-256-IGE x=0/8), TL de capa 144, seq_no/huecos/Resend, re-keying PFS, TTL, huella visual; EXPERIMENTAL sin auditoría | crates/tc-secret (+ README; Cargo.toml: crypto-bigint, feature `testing`) | HECHO (40 tests, vectores independientes en Python, clippy -D warnings OK; NO probado contra Telegram real; sin medios, archivos, MTProto 1.0 ni persistencia de sesión) |
| 10 | RED | opus | default | tc-telegram: `GrammersBackend` sobre grammers 0.11 (login teléfono/código/2FA y QR en TL crudo, sesión serializada tras `SessionStore`, diálogos, historial, enviar/editar/borrar/responder/reenviar/reacciones/leer, medios y stickers, updates → `BackendEvent`, FloodWait y reconexión); `AuthApi` en tc-core (sub-trait aparte) e implementado en tc-mock | crates/tc-telegram, crates/tc-core/src/auth.rs, crates/tc-mock (AuthApi), Cargo.toml raíz | HECHO (37 tests con transporte falso + 2 del mock, clippy -D warnings OK; NADA probado contra la red real; sin chats secretos ni keyring) |

## 2026-11-14 — agente DIFF (Sonnet)
| # | Agente | Modelo | Esfuerzo | Tarea | Salida | Estado |
| --- | --- | --- | --- | --- | --- | --- |
| 11 | diff | sonnet | medium | replica-diff: columna `clone` de features.csv + puntuación de paridad ponderada y faltantes | replica/features.csv, replica/diff-report.md | HECHO (paridad 45,0/100; must-haves 16 de 54; sin diff de pantallas por no haber captura del original; no se tocó código ni test-plan.md/bugs.md) |
| 11 | QA | sonnet | medium | pasada replica-test: plan numerado, ejecución de la suite, tests nuevos de F04/F05/F07/F08, bugs | replica/test-plan.md, replica/bugs.md, replica/evidence/, crates/tc-ui/src/tests_qa.rs (+ `mod tests_qa` en lib.rs) | HECHO (cargo test --workspace: 411 OK + 9 nuevos OK + 1 ignorado que reproduce B-01; clippy -D warnings limpio; nada contra red real) |

## 2026-11-14 — fase A0 (archivos)
| # | Agente | Modelo | Esfuerzo | Tarea | Salida | Estado |
| --- | --- | --- | --- | --- | --- | --- |
| 12 | A0 contratos | opus | high | contrato de transferencias (progreso, cancelación, descarga a disco) y saneado de nombres; mock y *passthrough* | crates/tc-core (transfer.rs, file_name.rs, api.rs, event.rs, error.rs, forum.rs), crates/tc-mock (transfers.rs), crates/tc-store (cached.rs, media_cache.rs), 2 líneas en crates/tc-ui/src/app.rs, docs/adr/0011-transferencias.md, docs/architecture.md | HECHO (tests de propiedades + 8 de contrato en mock + 2 de passthrough; cargo test --workspace y clippy -D warnings limpios; tc-telegram con la implementación por defecto, la hace A1; sin commit) |

## 2026-11-14 — fase A1 (archivos)
| # | Agente | Modelo | Esfuerzo | Tarea | Salida | Estado |
| --- | --- | --- | --- | --- | --- | --- |
| 13 | A1 Telegram | opus | high | `download_media_to` real (partes a `.part` + renombrado, `FILE_REFERENCE_*`, `FILE_MIGRATE`, reanudación), progreso ≤ 10 Hz y cancelación en `send_media`, `cancel_transfer`, límite de 2000 MiB con aviso, validación sin eventos, nombres saneados | crates/tc-telegram (media.rs, lib.rs, net.rs, tests/transfers.rs, tests/mod.rs) | HECHO (9 tests nuevos con transporte falso y reloj virtual; cargo test --workspace y clippy -D warnings limpios; NADA probado contra Telegram real; sin commit) |

## 2026-11-14 — fase A2 (archivos)
| # | Agente | Modelo | Esfuerzo | Tarea | Salida | Estado |
| --- | --- | --- | --- | --- | --- | --- |
| 14 | A2 envío UI | opus | medium | clip + Ctrl+O (rfd), soltar, pegar (rutas e imagen con arboard), diálogo de envío (vista previa, comprimir/archivo, pie ≤ 1024, álbum), burbuja con progreso, cancelar y reintentar; foros y `write_block` | crates/tc-ui (files.rs, app/uploads.rs, view/send_dialog.rs, tests_files.rs, tests_send_shots.rs, app.rs, conv.rs, view/conversation.rs, view/mod.rs, icon.rs, lib.rs, Cargo.toml), Cargo.toml, docs/design/system.md §8.11, docs/design/screens/a2-*.png | HECHO (18 tests nuevos con mock y reloj virtual + capturas; cargo test --workspace y clippy -D warnings limpios; NADA contra Telegram real ni en Windows/macOS; sin commit) |


## 2026-11-14 — fase A3 (archivos)
| # | Agente | Modelo | Esfuerzo | Tarea | Salida | Estado |
| --- | --- | --- | --- | --- | --- | --- |
| 15 | A3 descarga UI | sonnet | medium | clic en documento = descargar a Descargas (sin sobrescribir) con progreso y cancelar, Abrir / Mostrar en carpeta, Guardar como…, Guardar en visor y menús, reintentar; foros | crates/tc-ui (app/downloads.rs, view/downloads.rs, tests_downloads.rs, tests_download_shots.rs, app.rs, app/uploads.rs, view/send_dialog.rs, view/context_menu.rs, view/mod.rs, lib.rs, Cargo.toml), Cargo.toml (dirs), docs/design/system.md §8.12, docs/design/screens/a3-*.png | HECHO (13 tests + capturas; NADA contra Telegram real; sin commit) |
