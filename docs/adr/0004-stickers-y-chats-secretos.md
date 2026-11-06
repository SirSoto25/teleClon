# ADR-0004: Stickers y chats secretos entran en el alcance

- **Fecha:** 2026-11-06 · **Origen:** petición explícita del usuario (revierte el `skip` del recon).
- **Stickers:** se renderizan los packs que sirve la red (WebP estático; Lottie/TGS animado como "should"; WebM "could"). NO se empaquetan assets de Telegram en el repo; se crea un pack original propio para el mock y la marca.
- **Chats secretos:** cifrado E2E de MTProto 2.0 (intercambio Diffie-Hellman, `auth_key` por chat, mensajes cifrados, temporizador de autodestrucción, huella visual de la clave). Está ligado a un dispositivo, sin multi-dispositivo ni reenvío.
- **Riesgo principal:** `grammers` no implementa chats secretos; hay que escribir la capa a partir de la documentación pública (core.telegram.org/api/end-to-end) usando primitivas auditadas (RustCrypto: `aes`, `sha2`, `num-bigint`...). Criptografía propia sin auditoría externa: se etiqueta como experimental, con vectores de prueba y revisión de seguridad antes de recomendarla.
- **Consecuencias:** nuevo crate `tc-secret`; el trait `ChatBackend` gana operaciones de chat secreto; el MockBackend simula el cifrado con claves reales entre dos cuentas simuladas; el esquema SQLite guarda claves en almacenamiento protegido (keyring del sistema), nunca en claro; tamaño del proyecto sube.

## Estado de la implementación (2026-11-06, agente CRIPTO)
- **`tc-secret` es EXPERIMENTAL y está SIN AUDITORÍA.** No debe presentarse a los usuarios como cifrado seguro. Antes de quitar la etiqueta hacen falta tres cosas: `security-review`, una auditoría externa y vectores de caja negra obtenidos de un cliente real.
- Implementado y probado (40 tests):
  - DH con validación de p y g.
  - Comprobación de rangos de g_a y g_b.
  - Exponenciación de tiempo constante (`crypto-bigint`).
  - KDF y AES-256-IGE de MTProto 2.0 con x = 0/8 y relleno de 12 a 1024 bytes.
  - TL de la capa secreta 144 (sin medios).
  - seq_no con paridad, huecos y `Resend`.
  - Re-keying PFS, incluidas peticiones cruzadas.
  - TTL del chat y temporizador.
  - Huella visual.
- Desviación respecto a la lista de primitivas: se añade `crypto-bigint` 0.7.10 (RustCrypto) solo en `crates/tc-secret/Cargo.toml`, para las exponenciaciones con exponente secreto. `num-bigint` no es de tiempo constante y queda solo para valores públicos.
- No verificado contra Telegram real y pendiente: lista completa en `crates/tc-secret/README.md` ("Lo que NO está hecho o NO está verificado").
