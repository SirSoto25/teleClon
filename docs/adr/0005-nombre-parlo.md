# ADR-0005: Nombre de producto «Parlo»

- **Fecha:** 2026-11-07 · **Origen:** elección del usuario entre los 8 candidatos de `docs/brand/brand.md`.
- **Decisión:** el nombre visible y de producto es **Parlo** (de «parlar»). Aplica a título de ventana, textos de UI, binario (`parlo`), README, servicio del llavero (`parlo/<cuenta>`) y directorio de datos (`~/.local/share/parlo`).
- **Sin cambio:** el repo `teleClon` y los crates `tc-*` conservan su prefijo interno; las variables `TC_*` son internas. «Telegram» solo aparece como término técnico (protocolo, crate `tc-telegram`, `TC_BACKEND=telegram`, `my.telegram.org`), nunca como nombre de producto.
- **Migración desde `teleclon`:**
  - Datos: si existe `~/.local/share/teleclon` y no `~/.local/share/parlo`, se renombra una vez (`tc_store::resolve_data_dir`, con test); si el renombrado falla se sigue usando el directorio antiguo.
  - Llavero: `KeyringVault::for_account` lee del servicio `parlo/<cuenta>` y, si falta la entrada, la copia desde `teleclon/<cuenta>` y borra la antigua.
  - Se conserva `teleclon/secret-chat-state/v1/` como AAD del cifrado del estado de chats secretos: cambiarlo invalidaría los datos ya cifrados.
- **Riesgo abierto (NO verificado):** no se ha comprobado disponibilidad de marca, dominio, handles ni paquetes. `brand.md` ya avisaba de la cercanía con «Parler». Debe comprobarlo el usuario (marca OEPM/EUIPO/USPTO clases 9, 38, 42; dominios; handles; Flathub/AUR/crates.io) antes de cualquier lanzamiento.
- **Pendiente:** logo (solo hay brief en `brand.md`), `.desktop` e id de Flatpak (no existen aún; usar el id solo tras verificar el dominio).
- **Barrido:** `tools/brand/sweep.py` deja 0 bloqueantes y 0 a revisar; se añadió a su lista de permitidos el selector técnico `TC_BACKEND=telegram` y sus identificadores.
