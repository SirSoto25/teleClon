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

## 2026-11-14 — informe de emprendedor
| # | Agente | Modelo | Esfuerzo | Tarea | Salida | Estado |
| --- | --- | --- | --- | --- | --- | --- |
| 16 | EMPRENDEDOR | sonnet | medium | Leer quejas públicas de Telegram Desktop (issues de tdesktop, Hacker News), clasificar lo que molesta, falta y no está resuelto, cruzar con diff-report y features.csv y proponer plan de mejoras | replica/entrepreneur-report.md, docs/research/skills-used.md, docs/agents/registry.md, docs/logs/log.md | HECHO (solo documentación; sin tocar código; Reddit no accesible; sin commit) |
| 16 | A3b carpeta de descargas | sonnet | medium | carpeta predeterminada Telegram Desktop/Telegram (creada al primer uso), ajuste persistente con selector y «Restablecer», «Guardar como…» parte de ella, validación y vuelta a la predeterminada | crates/tc-ui (app/downloads.rs, view/downloads.rs, view/sidebar.rs, view/mod.rs, app.rs, lib.rs, tests_downloads.rs), crates/tc-app (stores.rs, main.rs), docs/design/system.md §8.12 | HECHO (10 tests; test --workspace y clippy limpios; selector real y otros SO sin verificar; sin commit) |

## 2026-11-14 — icono definitivo
| # | Agente | Modelo | Esfuerzo | Tarea | Salida | Estado |
| --- | --- | --- | --- | --- | --- | --- |
| 15 | logo | opus | low | icono definitivo «Brasa» según el brief (brand.md §5): símbolo, logotipo horizontal, claro/oscuro/mono, bandeja, formatos de paquete | assets/brand/*.svg, tools/packaging/make_logo.py, tools/packaging/make_icons.py, packaging/icons/*, packaging/windows/parlo.ico, packaging/macos/Parlo.icns, packaging/linux/{build-linux,install-user}.sh, docs/brand/brand.md, docs/packaging.md | HECHO (miniaturas 16-1024 revisadas a ojo; .ico/.icns decodificados de vuelta; contraste 7,26 y 4,43; sin búsqueda inversa de imágenes; crates/ sin tocar; sin commit) |

## 2026-11-14 — fase B1 (búsqueda en el chat)
| # | Agente | Modelo | Esfuerzo | Tarea | Salida | Estado |
| --- | --- | --- | --- | --- | --- | --- |
| 17 | B1 búsqueda backend | sonnet | medium | `search_in_chat` (FTS5 local + `messages.search`, filtros por tipo, temas de foro, paginación, fusión sin duplicados en CachedBackend) con mock y pruebas | crates/tc-core (search.rs, api.rs, forum.rs), crates/tc-store (messages.rs, cached.rs, cached/forum.rs, cached_tests.rs), crates/tc-telegram (search.rs, tests/search.rs), crates/tc-mock (search.rs), docs/adr/0012-busqueda-en-chat.md, docs/architecture.md | HECHO (ver log; Telegram real y UI sin hacer; sin commit) |

## 2026-11-14 — fases C1 + C3 (notificaciones)
| # | Agente | Modelo | Esfuerzo | Tarea | Salida | Estado |
| --- | --- | --- | --- | --- | --- | --- |
| 18 | C1 tc-notify + C3 empaquetado | opus | medium | crate `tc-notify`: trait `Notifier`, eventos de clic/acción, `NullNotifier`/`RecordingNotifier`, backends D-Bus (zbus), WinRT (windows) y macOS (mac-notification-sys); reglas puras en `rules` (foco, silenciado, tipo, archivado, vista previa, secretos, No molestar, ráfagas, agrupar); AUMID/Info.plist/.desktop | crates/tc-notify/*, Cargo.toml, packaging/windows/{parlo.iss,parlo.manifest}, packaging/macos/Info.plist.in, packaging/linux/parlo.desktop, docs/adr/0013-notificaciones.md, docs/packaging.md, docs/architecture.md | HECHO (43 pruebas + aviso real por D-Bus; Windows/macOS solo `cargo check`; clic real sin probar; sin integrar en UI; sin commit) |

## 2026-11-14 — fase B2 (UI de búsqueda en el chat)
| # | Agente | Modelo | Esfuerzo | Tarea | Salida | Estado |
| --- | --- | --- | --- | --- | --- | --- |
| 19 | B2 búsqueda UI | opus | medium | barra de búsqueda en el chat: lupa/Ctrl+F, contador, ↑/↓/Enter, chips de tipo, resaltado, espera, salto con `Anchor::Around`, Esc restaura, borrados, temas, secretos desactivados | crates/tc-ui (app/find.rs, view/find_bar.rs, view/conversation.rs, view/topics.rs, app.rs, style.rs, icon.rs, tests_find.rs, tests_find_shots.rs), docs/design/system.md §8.13, docs/design/screens/b2-*.png | HECHO (tests+clippy tc-ui; sin Telegram real; sin commit) |

