# Registro de skills usadas

Se actualiza cada vez que se invoca una skill. Estados: **usada** (invocada y seguida), **encargada** (indicada a un agente en su prompt; confirmar en su informe), **consultada** (leída como referencia, no instalada), **descartada**.

| Fecha | Skill | Quién | Para qué | Estado |
| --- | --- | --- | --- | --- |
| 2026-11-06 | `replica-skill:replica-recon` | orquestador | Pasos de reconocimiento y plantillas (alcance, fuentes, matriz) | usada |
| 2026-11-06 | `replica-skill:replica-recon` | agente recon (Sonnet/medium) | Generar replica/recon.md y features.csv | usada (informe recibido) |
| 2026-11-06 | `replica-skill:replica-design` | agente diseño (Opus/high) | Sistema de diseño y tokens | usada (informe recibido) |
| 2026-11-06 | `anthropic-skills:theme-factory` | agente diseño | Apoyo para tokens de color/tipo (opcional) | sin confirmar en el informe |
| 2026-11-06 | `SearchSkills` / `ListSkills` (herramienta) | orquestador | Buscar skills de diseño y graphify en el catálogo del usuario; sin resultados | usada |
| 2026-11-06 | graphify (`graphifyy`, skill de proyecto + CLI) | orquestador + 2 agentes Sonnet/medium | Grafo de código + documentos (1183 nodos, 2926 aristas, 64 comunidades); ver research/graphify.md | usada |
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
| 2026-11-06 | `replica-skill:replica-build` | agente STICKERS | Implementar tc-stickers (WebP, TGS/Lottie, caché LRU, recientes/favoritos, pack propio procedural) como pieza de la fase build | usada |
| 2026-11-06 | `replica-skill:replica-backend` | agente STORE | Capa SQLite de tc-store (migraciones por user_version, repositorios, FTS5, chats secretos sin claves en la base); auth/pagos/email de la skill no aplican a una caché local | usada |

## 2026-11-06 — agente CRIPTO (Opus)
| Fecha | Skill | Quién | Para qué | Estado |
| --- | --- | --- | --- | --- |
| 2026-11-06 | `replica-skill:replica-build` | agente CRIPTO | Reglas de sala limpia (todo escrito desde la especificación pública, sin código de otros clientes) aplicadas a `tc-secret`; las partes de pantallas, capturas y features.csv no aplican a un crate de cripto | usada |

## 2026-11-06 — agente RED (Opus)
| Fecha | Skill | Quién | Para qué | Estado |
| --- | --- | --- | --- | --- |
| 2026-11-06 | `replica-skill:replica-backend` | agente RED | Reglas: solo API pública con las claves del usuario, secretos solo en variables de entorno, nada de cuentas ni credenciales tecleadas por el agente; aplicado a `tc-telegram` (cliente, sin servidor propio: pagos/correo/webhooks no aplican) | usada |
| 2026-11-06 | `replica-skill:replica-build` | agente RED | Mismas firmas que el backend falso (tc-mock) para que cambiar a la red real no toque la UI; `AuthApi` también implementado en el mock | usada |

## 2026-11-06 — agente DOCS (Sonnet)
| Fecha | Skill | Quién | Para qué | Estado |
| --- | --- | --- | --- | --- |
| 2026-11-06 | `replica-skill:replica-brand` | agente DOCS | Candidatos de nombre, criterios, chequeos para el usuario, voz y tono, brief de logo y `tools/brand/sweep.py`; sin consultas externas (disponibilidad NO verificada) | usada |
| 2026-11-06 | `replica-skill:replica-test` | agente DOCS | Plan de pruebas generado desde los flujos de recon.md; no se escribieron tests Playwright (app nativa) ni se ejecutó nada | usada (solo plan) |

## 2026-11-14 — agente DIFF (Sonnet)
| Fecha | Skill | Quién | Para qué | Estado |
| --- | --- | --- | --- | --- |
| 2026-11-14 | `replica-skill:replica-diff` | agente DIFF | Rellenar `clone` de replica/features.csv con evidencia del código y docs/ y calcular la paridad ponderada con `parity.py` (45,0/100). El diff de pantallas (`imgdiff.py`) NO se usó: no hay captura del original | usada (solo paridad de funciones) |

## 2026-11-14 — agente QA (Sonnet)
| Fecha | Skill | Quién | Para qué | Estado |
| --- | --- | --- | --- | --- |
| 2026-11-14 | `replica-skill:replica-test` | agente QA | `replica/test-plan.md` (casos Fxx-H/E/N con estado AUTO/NUEVO/MANUAL/REAL), `replica/bugs.md`, 9 tests nuevos en tc-ui y capturas headless; adaptado a app nativa (sin Playwright) | usada |

## 2026-11-14 — agente EMPRENDEDOR (Sonnet)
| Fecha | Skill | Quién | Para qué | Estado |
| --- | --- | --- | --- | --- |
| 2026-11-14 | `replica-skill:replica-entrepreneur` | agente EMPRENDEDOR | `replica/entrepreneur-report.md`: quejas y peticiones públicas de Telegram Desktop (API de GitHub de tdesktop y API Algolia de Hacker News), cruce con diff-report/features.csv y plan de 5 mejoras. Muestra de unas 55 piezas en 2 fuentes; Reddit NO se pudo leer; no se usó `reviews.py` ni se creó `reviews.csv`. Uso personal: el ángulo de venta se documenta, no se persigue | usada (muestra parcial) |
