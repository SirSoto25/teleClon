# Bitácora (append-only)

## 2026-11-06
- Petición: clon de Telegram Desktop para Linux en Rust; skills de diseño poco conocidas; multiagente con modelo/esfuerzo por tarea; documentar todo; graphify.
- Entorno: Linux (CachyOS, Wayland), cargo 1.99.1, python3, node. 12 CPU, 15 GB RAM. Directorio vacío al inicio, sin git.
- Invocada skill `replica-skill:replica-recon`. Alcance asumido (el usuario dio plataforma=Linux desktop, app=Telegram Desktop): slice = núcleo de mensajería (lista de chats, conversación, compositor, perfil/ajustes básicos); público = producto propio de marca nueva.
- Búsqueda de skills de diseño: catálogo del usuario sin resultados; investigación web → [research/design-skills.md](../research/design-skills.md). No se instalan skills de terceros (riesgo de inyección); se adoptan ideas.
- graphify (`graphifyy`) verificado en PyPI e instalado en `.venv` → [research/graphify.md](../research/graphify.md).
- Creada estructura `docs/`, `replica/`.
- Creado repo git local (rama main), .gitignore, README. Repo remoto GitHub privado 'teleClon' (cuenta SirSoto25) a petición del usuario; se sube tras cada fase.
