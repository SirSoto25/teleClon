# graphify

- Paquete PyPI `graphifyy` (CLI `graphify`), autor Graphify Labs, Apache-2.0, fuente https://github.com/Graphify-Labs/graphify, docs https://docs.graphify.com. Verificado en la ficha de PyPI antes de instalar.
- Instalado en un venv local del proyecto: `./.venv` (ignorado por git). Versión: ver `./.venv/bin/pip show graphifyy`.
- Uso: skill `/graphify . --update` en Claude Code (requiere `graphify install`), o CLI (`./.venv/bin/graphify --help`).
- Salida: `graphify-out/{graph.html,GRAPH_REPORT.md,graph.json}`.
- Se regenera al final de cada fase (ver bitácora).

## Extracción semántica de documentos (2026-11-06)
- `graphify install --project` (permiso del usuario) copió la skill a `.claude/skills/graphify/`. Se eliminaron los hooks de `.claude/settings.json` (llaman a `graphify` fuera del PATH y corren en cada herramienta). `CLAUDE.md` apunta a `./.venv/bin/graphify`.
- La skill no se carga a mitad de sesión: se siguió su SKILL.md a mano. 2 subagentes Sonnet/medium extrajeron conceptos de 14 documentos (135 nodos: concept, document, rationale); se fusionaron con el AST de código. Script reutilizable: `tools/graphify_semantic_build.py` (ejecutar con `./.venv/bin/python` desde la raíz tras regenerar chunks).
- Resultado: 1183 nodos, 2926 aristas, 64 comunidades (92 % EXTRACTED). Hubs: Msg, App, MockBackend, Message, Brasa.
- Limitaciones: comunidades sin nombre ("Community N", no hay backend LLM configurado para `graphify label`); el concepto doc `ChatBackend` y el símbolo de código no se unifican (nodos duplicados); `tokens.json` excluido por ser clasificado como sensible; no se validó el JSON de los chunks más allá de que el build lo aceptó.
- Para actualizar solo código: `./.venv/bin/graphify update .` (sin LLM).
