# Registro de skills usadas

Se actualiza cada vez que se invoca una skill. Estados: **usada** (invocada y seguida), **encargada** (indicada a un agente en su prompt; confirmar en su informe), **consultada** (leída como referencia, no instalada), **descartada**.

| Fecha | Skill | Quién | Para qué | Estado |
| --- | --- | --- | --- | --- |
| 2026-11-06 | `replica-skill:replica-recon` | orquestador | Pasos de reconocimiento y plantillas (alcance, fuentes, matriz) | usada |
| 2026-11-06 | `replica-skill:replica-recon` | agente recon (Sonnet/medium) | Generar replica/recon.md y features.csv | usada (informe recibido) |
| 2026-11-06 | `replica-skill:replica-design` | agente diseño (Opus/high) | Sistema de diseño y tokens | usada (informe recibido) |
| 2026-11-06 | `anthropic-skills:theme-factory` | agente diseño | Apoyo para tokens de color/tipo (opcional) | sin confirmar en el informe |
| 2026-11-06 | `SearchSkills` / `ListSkills` (herramienta) | orquestador | Buscar skills de diseño y graphify en el catálogo del usuario; sin resultados | usada |
| 2026-11-06 | graphify (`graphifyy`, CLI `update`) | orquestador | Grafo de código (1161 nodos, 2839 aristas, 58 comunidades) en graphify-out/; sin LLM. La extracción semántica de documentos (/graphify --update) NO se ha ejecutado (requiere `graphify install`, que escribe en el perfil del usuario) | usada (parcial) |
| 2026-11-06 | Hallmark, Interface Design, Emil Kowalski, claudekit, Bencium, Awesome Claude Design, AccessLint | — | Ideas de diseño; ver [design-skills.md](design-skills.md) | consultada (no instaladas) |
| 2026-11-06 | Scroll-Craft, Scroll-World, img2threejs | — | Orientadas a web/3D | descartada |
| 2026-11-06 | `replica-skill:replica-architect` | agente arquitecto (Opus/high) | Stack, esquema SQLite, operaciones por flujo, orden de construcción → docs/architecture.md, ADR-0003 | usada |

## Skills previstas en fases siguientes
`replica-architect`, `replica-build`, `replica-test`, `replica-diff`, `replica-brand`, `design:accessibility-review`, `superpowers:verification-before-completion` (antes de dar nada por terminado).

## Nota de transparencia
No se invocó ninguna skill `superpowers:*` hasta ahora. Las skills de terceros no se han instalado.

## 2026-11-06 — agente CORE (Sonnet)
- `replica-skill:replica-build`: método de rebanada vertical (backend falso con las mismas firmas que el real, datos semilla, estados vacíos/largos) aplicado a `tc-core`/`tc-mock` para M1.1–M1.3.

## 2026-11-06 — agente UI (Opus/high)
| Fecha | Skill | Quién | Para qué | Estado |
| --- | --- | --- | --- | --- |
| 2026-11-06 | `replica-skill:replica-build` | agente UI | Rebanada vertical M1.1 (lista), M1.2 (conversación con scroll virtual), M1.3 (compositor) en tc-ui/tc-app contra el mock; columna `clone` de features.csv actualizada (29 filas yes/partial) | usada |
