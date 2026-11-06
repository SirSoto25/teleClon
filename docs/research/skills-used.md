# Registro de skills usadas

Se actualiza cada vez que se invoca una skill. Estados: **usada** (invocada y seguida), **encargada** (indicada a un agente en su prompt; confirmar en su informe), **consultada** (leída como referencia, no instalada), **descartada**.

| Fecha | Skill | Quién | Para qué | Estado |
| --- | --- | --- | --- | --- |
| 2026-11-06 | `replica-skill:replica-recon` | orquestador | Pasos de reconocimiento y plantillas (alcance, fuentes, matriz) | usada |
| 2026-11-06 | `replica-skill:replica-recon` | agente recon (Sonnet/medium) | Generar replica/recon.md y features.csv | usada (informe recibido) |
| 2026-11-06 | `replica-skill:replica-design` | agente diseño (Opus/high) | Sistema de diseño y tokens | usada (informe recibido) |
| 2026-11-06 | `anthropic-skills:theme-factory` | agente diseño | Apoyo para tokens de color/tipo (opcional) | sin confirmar en el informe |
| 2026-11-06 | `SearchSkills` / `ListSkills` (herramienta) | orquestador | Buscar skills de diseño y graphify en el catálogo del usuario; sin resultados | usada |
| 2026-11-06 | graphify (`graphifyy`, CLI/skill) | orquestador | Grafo de conocimiento; instalado en `.venv`, aún sin ejecutar | pendiente |
| 2026-11-06 | Hallmark, Interface Design, Emil Kowalski, claudekit, Bencium, Awesome Claude Design, AccessLint | — | Ideas de diseño; ver [design-skills.md](design-skills.md) | consultada (no instaladas) |
| 2026-11-06 | Scroll-Craft, Scroll-World, img2threejs | — | Orientadas a web/3D | descartada |

## Skills previstas en fases siguientes
`replica-architect`, `replica-build`, `replica-test`, `replica-diff`, `replica-brand`, `design:accessibility-review`, `superpowers:verification-before-completion` (antes de dar nada por terminado).

## Nota de transparencia
No se invocó ninguna skill `superpowers:*` hasta ahora. Las skills de terceros no se han instalado.
