# Bitácora (append-only)

## 2026-11-06
- Petición: clon de Telegram Desktop para Linux en Rust; skills de diseño poco conocidas; multiagente con modelo/esfuerzo por tarea; documentar todo; graphify.
- Entorno: Linux (CachyOS, Wayland), cargo 1.99.1, python3, node. 12 CPU, 15 GB RAM. Directorio vacío al inicio, sin git.
- Invocada skill `replica-skill:replica-recon`. Alcance asumido (el usuario dio plataforma=Linux desktop, app=Telegram Desktop): slice = núcleo de mensajería (lista de chats, conversación, compositor, perfil/ajustes básicos); público = producto propio de marca nueva.
- Búsqueda de skills de diseño: catálogo del usuario sin resultados; investigación web → [research/design-skills.md](../research/design-skills.md). No se instalan skills de terceros (riesgo de inyección); se adoptan ideas.
- graphify (`graphifyy`) verificado en PyPI e instalado en `.venv` → [research/graphify.md](../research/graphify.md).
- Creada estructura `docs/`, `replica/`.
- Creado repo git local (rama main), .gitignore, README. Repo remoto GitHub privado 'teleClon' (cuenta SirSoto25) a petición del usuario; se sube tras cada fase.
- Creado registro de skills usadas: research/skills-used.md (regla: actualizar en cada invocación de skill).

## 2026-11-06 — Recon (agente RECON)
- Ejecutada skill `replica-skill:replica-recon` sobre Telegram Desktop (Linux), slice: núcleo de mensajería.
- Escritos `replica/recon.md` (15 fuentes públicas, 37 pantallas S01-S37, 16 flujos F01-F16, 22 componentes, 14 entidades inferidas, fuera de alcance, tamaño L) y `replica/features.csv` (99 funciones: 47 must / 31 should / 11 could, 10 skip; clone=no salvo skip).
- Método: solo documentación pública (FAQ, core.telegram.org, changelog); sin cuenta, sin código fuente ni tráfico de red. `replica/screens/` sigue vacío: detalles de UI son inferencias a validar con capturas del usuario.
- Pendiente de decisión (ADR): cliente sobre la red de Telegram (requiere api_id/api_hash del usuario y respetar términos de API) vs backend propio.
- Siguiente paso: `/replica-architect`.
- Agente recon terminado. Limitación: no hizo fetch en vivo; contenido de memoria del modelo -> validar con capturas. Lanzado agente arquitecto (Opus/high) con replica-architect.
