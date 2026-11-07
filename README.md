# teleClon

Cliente de mensajería de escritorio para Linux escrito en Rust, compatible con el protocolo MTProto (clean-room: código, assets y marca propios). Interfaz moderna y propia.

Documentación y memoria del proyecto: [docs/00-INDEX.md](docs/00-INDEX.md).

## Red real: dónde se guardan los secretos

Con `TC_BACKEND=telegram` (credenciales `TC_API_ID`/`TC_API_HASH`, ver `.env.example`):

| Variable | Por defecto | Uso |
|---|---|---|
| `TC_ACCOUNT` | `default` | Cuenta (`[A-Za-z0-9_-]`, hasta 64): servicio `teleclon/<cuenta>` del llavero y base `~/.local/share/teleclon/accounts/<cuenta>/cache.sqlite3`. |
| `TC_SESSION_FILE` | `~/.local/share/teleclon/telegram/session.json` | Respaldo **en claro** (0600) de la sesión solo si no hay llavero; si existe con llavero disponible se migra a él una vez y se borra. |

* La sesión (clave de autorización) va al llavero del sistema (Secret Service/KWallet, entrada `session`).
* El estado de los chats secretos se cifra (AES-256-GCM, clave `secret-state-key` del llavero) y se guarda en SQLite.
  Sin llavero no se guarda (se avisa en el registro): nunca se escribe en claro.
