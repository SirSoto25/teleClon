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
- ALCANCE AMPLIADO por el usuario: stickers y chats secretos incluidos. features.csv (+11 filas, 2 skip->must), recon.md y ADR-0004 actualizados; avisado el agente arquitecto.
- [Dirección de diseño] Invocadas `replica-skill:replica-design` y `anthropic-skills:theme-factory` (tema propio). Elegida dirección **«Brasa»** (grafito cálido + acento persimón; descartadas vidrio glacial y neobrutalismo) → [ADR 0002](../adr/0002-direccion-visual.md).
  - Creados [design/system.md](../design/system.md) (paletas oscura/clara en OKLCH + hex, tipografía libre, espaciado, radios, elevación, muelles, specs de lista, burbujas, compositor, cabecera, raíl, diálogos, toasts, primitivas, lista anti-«aspecto IA», preferencias persistidas) y [design/tokens.json](../design/tokens.json).
  - Hex calculados con OKLab→sRGB y reducción de croma a gama; ida y vuelta con error máx. ΔL 0.0018. Contraste con `contrast.py` de replica-design: 144 pares, 0 fallos AA. Muelles (rapido/estandar/suave/rebote) ajustados por integración numérica a 119–221 ms.
  - Componentes cruzados con `replica/recon.md` y ADR-0004 (stickers, nota de voz, chat secreto con huella de clave propia, QR de login). Pendiente: copia de UX, marca (replica-brand), primitivas aisladas en iced. Sin código Rust ni commit.
- Usuario dispone de api_id/api_hash. Plantilla .env.example creada; los valores van en .env local (gitignored), nunca en chat ni en el repo.
- Agente diseño terminado: dirección «Brasa» (docs/design/system.md, tokens.json, ADR-0002). Scripts generadores copiados a tools/design/ (estaban en scratchpad efímero). Pendiente: fuentes OFL (Atkinson, Bricolage, JetBrains Mono) y iconos Phosphor requieren descarga -> pedir permiso al usuario.
- Fuentes OFL descargadas (con permiso del usuario) a assets/fonts + NOTICE.md. Iconos Phosphor NO descargados (no autorizados aún). Usuario confirma .env con credenciales (verificado sin mostrar valores).

## 2026-11-06 — Arquitectura (agente ARQUITECTO, Opus/high)
- Invocada skill `replica-skill:replica-architect`. Leídos 00-INDEX, ADR-0001, ADR-0004 (cambio de alcance: stickers y chats secretos), recon.md, features.csv.
- Versiones comprobadas con `cargo search`/`cargo info`: iced 0.14.0, grammers-* 0.11.0, tokio 1.54.1, rusqlite 0.41.1, keyring 4.2.0, RustCrypto (aes 0.9.4, ige 0.2.1, sha1 0.11, sha2 0.11.1, num-bigint 0.5.1, aes-gcm 0.11.1), image 0.25.10, flate2 1.1.11, rlottie 0.5.4, jiff 0.2.39.
- Hallazgos: `rusqlite_migration` 2.6 exige rusqlite ^0.40 (descartado, migraciones con user_version); `grammers-session` por defecto trae libsql (otro SQLite) → se usa sin `sqlite-storage`; `grammers-tl-types` 0.11 solo trae api.tl/mtproto.tl (capa secreta E2E hay que serializarla a mano); grammers expone `Update::Raw`; no hay login QR de alto nivel (TL crudo); iced lee `ICED_BACKEND` para forzar tiny-skia.
- Escritos: docs/architecture.md, docs/adr/0003-arquitectura.md (aceptado), ADR-0001 → aceptado con correcciones, esquema `crates/tc-store/migrations/0001_init.sql` (17 tablas + FTS5; validado con sqlite3).
- Esqueleto del workspace: Cargo.toml raíz + 8 crates (tc-core, tc-mock, tc-store, tc-theme, tc-secret, tc-stickers, tc-ui, tc-app) sin funcionalidad. `cargo check --workspace` OK (1m39s en frío); `cargo check -p tc-stickers --features lottie` OK; sonda aparte de grammers 0.11 + rusqlite bundled OK.
- .gitignore: añadidos `*.sqlite3`, `*.sqlite3-*`, `/config.toml`, `/config.local.toml`. Sin commit.
- Siguiente: M1.1 (lista de chats contra MockBackend) con `/replica-build`; añadir pantallas S38–S41 al recon.
