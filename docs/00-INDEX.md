# teleClon — Índice maestro (leer primero tras perder contexto)

Clon de Telegram Desktop para Linux, escrito en Rust, con identidad visual moderna y propia.

## Dónde está cada cosa
| Documento | Contenido |
| --- | --- |
| [logs/log.md](logs/log.md) | Bitácora cronológica de TODO lo hecho (append-only) |
| [adr/](adr/) | Decisiones de arquitectura/diseño (una por archivo, con motivo) |
| [agents/registry.md](agents/registry.md) | Qué agente hizo qué, con qué modelo/esfuerzo y por qué |
| [research/design-skills.md](research/design-skills.md) | Skills de diseño poco conocidas encontradas y cómo se usaron |
| [research/graphify.md](research/graphify.md) | Cómo se instaló/usa graphify y cómo regenerar el grafo |
| [research/skills-used.md](research/skills-used.md) | Registro de TODAS las skills usadas (fecha, quién, para qué) |
| `../replica/recon.md`, `../replica/features.csv` | Mapa de reconocimiento y matriz de funciones (skill replica-recon) |
| `../graphify-out/` | Grafo de conocimiento (graph.html, GRAPH_REPORT.md, graph.json) |

## Reglas de trabajo
1. Cada acción relevante se añade a `logs/log.md`; cada decisión nueva = nuevo ADR.
2. Clean-room: se reconstruyen funciones y patrones UX, nunca código, assets, nombre ni marca de Telegram. Marca propia en la fase replica-brand.
3. Credenciales de Telegram (api_id/api_hash) las aporta el usuario; nunca se inventan ni se guardan en el repo.
4. Tras cada fase: actualizar este índice y regenerar el grafo (`./.venv/bin/graphify update .` o `/graphify . --update`).

## Estado
Ver última entrada de [logs/log.md](logs/log.md).
