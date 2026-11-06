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
