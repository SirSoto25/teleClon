# ADR-0004: Stickers y chats secretos entran en el alcance

- **Fecha:** 2026-11-06 · **Origen:** petición explícita del usuario (revierte el `skip` del recon).
- **Stickers:** se renderizan los packs que sirve la red (WebP estático; Lottie/TGS animado como "should"; WebM "could"). NO se empaquetan assets de Telegram en el repo; se crea un pack original propio para el mock y la marca.
- **Chats secretos:** cifrado E2E de MTProto 2.0 (intercambio Diffie-Hellman, `auth_key` por chat, mensajes cifrados, temporizador de autodestrucción, huella visual de la clave). Está ligado a un dispositivo, sin multi-dispositivo ni reenvío.
- **Riesgo principal:** `grammers` no implementa chats secretos; hay que escribir la capa a partir de la documentación pública (core.telegram.org/api/end-to-end) usando primitivas auditadas (RustCrypto: `aes`, `sha2`, `num-bigint`...). Criptografía propia sin auditoría externa: se etiqueta como experimental, con vectores de prueba y revisión de seguridad antes de recomendarla.
- **Consecuencias:** nuevo crate `tc-secret`; el trait `ChatBackend` gana operaciones de chat secreto; el MockBackend simula el cifrado con claves reales entre dos cuentas simuladas; el esquema SQLite guarda claves en almacenamiento protegido (keyring del sistema), nunca en claro; tamaño del proyecto sube.
