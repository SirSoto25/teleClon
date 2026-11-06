# Skills de diseño poco conocidas (investigación 2026-11-06)

Búsqueda en el catálogo del usuario (`SearchSkills`): sin resultados. Búsqueda web: fuentes agregadoras
([composio](https://composio.dev/content/top-design-skills), [pasqualepillitteri](https://pasqualepillitteri.it/en/news/576/claude-code-skills-design-uiux-guide),
[snyk](https://snyk.io/articles/top-claude-skills-ui-ux-engineers/)). Los rankings de popularidad no son fiables.

## Decisión de seguridad
NO se instalan skills de terceros automáticamente: son instrucciones que se inyectan en el agente (riesgo de prompt injection,
lo advierten las propias fuentes). Se registran como candidatas y **se adoptan sus ideas** en el sistema de diseño propio.
Si el usuario quiere instalarlas, debe revisar su SKILL.md primero.

## Candidatas poco conocidas
| Skill | Repo | Idea adoptada para teleClon |
| --- | --- | --- |
| Hallmark | `nutlope/hallmark` | Colores en **OKLCH**, lista de patrones "aspecto IA" prohibidos (degradado morado genérico, tarjetas con sombra idéntica...), modo Study (extraer spec de una captura) |
| Interface Design | `Dammyjay93/interface-design` | Decisiones de diseño persistidas en un archivo (`docs/design/system.md`) para evitar deriva entre sesiones/agentes |
| Emil Kowalski skills | `emilkowalski/skills` | Micro-interacciones: muelles/easing, duraciones cortas (120–220 ms), animar solo transform/opacity |
| Frontend Design Pro (claudekit) | `claudekit/frontend-design-pro-demo` | Catálogo de 11 estéticas (glass, neumorfismo, brutalismo, claymorphism...) para elegir dirección |
| Bencium UX Designer | `bencium/bencium-marketplace` | Variante "controlled" (consistencia estricta) frente a "innovative"; usamos controlled para chrome y innovative solo en burbujas/fondos |
| Awesome Claude Design | `rohitg00/awesome-claude-design` | Formato DESIGN.md de tokens por familia estética |
| theme-factory (ya instalada: anthropic-skills:theme-factory) | local | Tokens de color/tipo reutilizables |
| AccessLint | `accesslint/claude-marketplace` | Auditoría WCAG de contraste (objetivo AA) |
| Scroll-Craft / Scroll-World / img2threejs | varios | Descartadas: orientadas a web/landing, no a app de escritorio |

## Skills ya disponibles localmente que sí se usan
`replica-skill:*` (pipeline completo), `anthropic-skills:theme-factory`, `anthropic-skills:canvas-design` (assets estáticos),
`design:accessibility-review`, `design:design-system`, `dataviz` (no aplica).
