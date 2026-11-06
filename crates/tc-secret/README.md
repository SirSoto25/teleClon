# tc-secret

> **EXPERIMENTAL. SIN AUDITORÍA.** Este crate contiene criptografía escrita en este proyecto a partir de la documentación pública de Telegram. Nadie de fuera la ha auditado y nunca se ha probado contra clientes reales de Telegram. No debe presentarse a los usuarios como segura. Antes de quitar esta etiqueta: `security-review`, una auditoría externa y vectores de caja negra obtenidos de un cliente real (ADR-0004).

Chats secretos E2E (MTProto 2.0) con diseño sans-IO. El crate no hace red ni disco, y tampoco lee el reloj. El llamador le pasa los bytes recibidos, la hora (`now`, en segundos Unix) y una fuente de entropía (`Entropy`). A cambio recibe un `Output` con dos listas: `outbound` (lo que hay que enviar, en orden) y `events` (lo que hay que mostrar o guardar).

Fuentes, leídas el 2026-11-06: core.telegram.org/api/end-to-end, `/api/end-to-end/seq_no`, `/api/end-to-end/pfs`, `/schema/end-to-end` y las páginas de los constructores `decryptedMessageLayer` y `decryptedMessage`.

## Módulos
| módulo | qué hace |
| --- | --- |
| `dh` | Valida `getDhConfig`: p es un primo seguro de 2048 bits, comprobado con Miller-Rabin de 32 rondas con bases aleatorias, tanto p como (p-1)/2. También comprueba la condición de residuo de g para g = 2..7. Genera a y b con XOR de la aleatoriedad local y la del servidor. Comprueba `1 < g_x < p-1` y la cota `2^(2048-64) ≤ g_x ≤ p - 2^(2048-64)` |
| `key` | `SecretKey` de 256 bytes. Se borra al soltarse (`ZeroizeOnDrop`) y su `Debug` está censurado. `fingerprint()` devuelve los 64 bits bajos de SHA1, es decir los bytes 12..20 en little-endian |
| `crypto` | `msg_key = SHA256(key[88+x..+32] ‖ datos)[8..24]` y la KDF sha256_a/sha256_b, con AES-256-IGE. x = 0 si envía el creador del chat y x = 8 si envía el otro. Relleno de 12 a 1024 bytes. `msg_key` se compara en tiempo constante (`subtle`) antes de leer ningún campo descifrado |
| `tl` | Serializador propio. Cubre `decryptedMessageLayer`, `decryptedMessage#91cc4674`, `decryptedMessageService` (capas 17 y 8), las 13 acciones (TTL, leídos, borrar, captura, vaciar, Resend, NotifyLayer, Typing, Request/Accept/Commit/AbortKey, Noop) y las entidades de la capa 144 |
| `session` | Máquina de estados `Requested`/`Incoming` → `Ready` → `Discarded`. Lleva los seq_no in/out con paridad según quién creó el chat. Detecta huecos, los guarda en cola y pide `Resend`, uno por hueco. Descarta duplicados. Si hay violaciones de secuencia, aborta. También gestiona la capa del otro extremo, el TTL del chat y el re-keying (PFS) a los 100 mensajes o a la semana, incluidas peticiones cruzadas (se aborta la de `exchange_id` menor) y mensajes en vuelo con la clave anterior |
| `ttl` | `SelfDestructTimer`: plazos por `random_id`. Releer un mensaje no alarga su plazo |
| `visual` | 36 bytes: SHA1(clave inicial)[..16] ‖ SHA256(clave de la capa 46)[..20]. Da también una rejilla de 12×12 y una cadena hexadecimal |
| `testing` | Solo con la feature `testing`: `SeededEntropy` determinista y el primo del RFC 3526 (grupo 14). **No es para producción** |

## Garantías locales
- `#![forbid(unsafe_code)]`.
- Las exponenciaciones con exponente secreto usan `crypto-bigint` (`FixedMontyForm::pow`, de tiempo constante). `num-bigint` se usa solo con valores públicos.
- Las claves, los exponentes, los buffers intermedios de la KDF y los textos en claro se guardan en tipos que se borran al soltarse (`Zeroizing`/`ZeroizeOnDrop`). Al descartar el chat se sueltan la clave y el registro de enviados.
- Si un mensaje manipulado o con una clave desconocida devuelve `Err`, el estado del chat no cambia.

## Lo que NO está hecho o NO está verificado
- **No se ha probado contra Telegram real.** Los vectores de las pruebas salen de una implementación independiente en Python (pycryptodome + `pow`) escrita a partir de la misma especificación. Demuestran que el código hace lo que dice el texto, pero no que interopere: la especificación no publica vectores oficiales.
- Interpretaciones que no se han podido comprobar:
  - "64 bits bajos" del fingerprint = SHA1[12..20] en little-endian.
  - `NotifyLayer` se envía suelto en `decryptedMessageService#aa48327d` y no consume seq_no.
  - Capa de envío = `min(144, capa del otro)`, con mínimo 73.
  - Cota de `in_seq_no`: se interpreta en crudo (`≤ mensajes enviados`). La página dice "D + 1" en forma de cable.
  - Re-cifrado de los `Resend` con la clave vigente y el mismo `random_id`.
  - Envío automático de `Noop` tras instalar una clave.
- No está implementado:
  - MTProto 1.0 para otros extremos con capa < 73.
  - Medios: solo `decryptedMessageMediaEmpty`.
  - Archivos cifrados (clave e iv por archivo, huella md5).
  - `SendMessageAction` distintas de escribir y cancelar.
  - Persistencia del estado de sesión (contadores, cola, registro). Por ahora solo se exporta la clave.
- La rejilla y los emoji de `visual` siguen una convención propia, así que no se pueden comparar visualmente con otros clientes. El texto hexadecimal sí se puede comparar.
- El registro de enviados para atender `Resend` guarda en memoria los textos en claro de los últimos 1024 mensajes.
- Si la entropía falla a mitad de procesar un mensaje, puede perderse lo que ya había en `Output`. Con `OsEntropy` no se espera que ocurra.
