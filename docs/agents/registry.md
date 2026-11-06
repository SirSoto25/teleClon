# Registro de agentes

Criterio: investigación/redacción estructurada → Sonnet (esfuerzo medio); decisiones de diseño/arquitectura y UI compleja → Opus (esfuerzo alto); tareas mecánicas → Haiku/Sonnet bajo. Orquestador: sesión principal.

| # | Agente | Modelo | Esfuerzo | Tarea | Salida | Estado |
| --- | --- | --- | --- | --- | --- | --- |
| 1 | recon | sonnet | medium | replica-recon de Telegram Desktop (fuentes públicas) | replica/recon.md, replica/features.csv | lanzado |
| 2 | diseño | opus | high | identidad visual propia, tokens | docs/design/*, ADR-0002 | lanzado |
| 3 | arquitecto | opus | high | stack, esquema, crates (tras recon) | docs/architecture.md | pendiente |
| 4 | core+mock | sonnet | medium | crate de dominio + backend simulado | crates/tc-core | pendiente |
| 5 | UI | opus | high | app iced con tema | crates/tc-ui | pendiente |
| 6 | QA | sonnet | medium | build, clippy, tests | informe | pendiente |
