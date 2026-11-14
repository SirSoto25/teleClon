# Informe de paridad: Parlo frente al cliente original

> **Actualización 2026-11-14 (tras los 5 puntos del plan):** paridad **58,2 / 100** (antes 45,0). Must-haves hechos: **23 de 54** (antes 16). Sigue **no distribuible**: faltan 31 must-haves (registro de cuenta, crear chat privado, carpetas de chats, editar perfil propio y varios `partial` sin probar contra Telegram real).
> Subieron a `yes` con prueba real del usuario: enviar archivo, pegar imagen, diálogo de envío, progreso/cancelar, descargar/guardar, archivos como tipo de mensaje, GIFs, notificaciones de escritorio, ajustes por tipo y vista previa. `partial` `[sin real]`: búsqueda en el chat (+ filtros), arrastrar y soltar (no en Wayland nativo), álbum, sonido, atajos, descarga (solo carpeta).
> Áreas más débiles ahora: carpetas 20,0; grupos 25,0; ajustes 29,4; perfil 38,9; seguridad 41,7; composición 42,9; búsqueda 44,4. Mejor: attachments 74,0 (antes 12,0), notifications 57,7 (antes 11,5).
> Las cifras y tablas siguientes son del informe anterior (45,0) y se conservan como historial.

Fecha: 2026-11-14. Herramienta: `replica-skill:replica-diff` (`parity.py` sobre `replica/features.csv`).

## Resumen

- **Paridad de funciones: 45,0 / 100** (105 funciones puntuadas; ponderación must 3, should 2, could 1; «partial» cuenta la mitad; se excluyen las 8 `skip`).
- Recuento de la columna `clone`: 21 yes, 44 partial, 40 no, 8 skip (113 filas; se añadieron 3 filas de funciones recientes que no estaban en el recon).
- Must-haves hechos: **16 de 54**. Veredicto: **no distribuible** (faltan 38 must-haves por completar; hay que sumar los fallos abiertos en `replica/bugs.md`, que otro agente escribe y no se ha consultado).
- **Diff de pantallas: no realizado.** No existe captura del original (`replica/screens/` no tiene imágenes y no se usa), así que `imgdiff.py` no se ejecutó y no hay puntuación de diseño. Este informe se limita a la paridad de funciones. Las capturas de Parlo en `docs/design/screens/` son de la propia app.
- Criterio de la columna `clone`: `yes` = hecho y probado (verificado contra Telegram real por el usuario el 2026-11-09, o es local y no depende de la red, con pruebas). `partial` = falta una parte (`[incompleto]`) o está hecho pero **nunca se ha probado contra Telegram real** (`[sin real]`); la evidencia (crate/archivo/prueba) está en `notes`.

## Puntuación por área (la más débil primero)

| Área | Puntos |
| --- | --- |
| ux | 0,0 |
| settings | 8,8 |
| notifications | 11,5 |
| attachments | 12,0 |
| search | 16,7 |
| folders | 20,0 |
| groups | 25,0 |
| profile | 38,9 |
| security | 41,7 |
| composer | 42,9 |
| message types | 43,9 |
| contacts | 50,0 |
| reactions | 50,0 |
| platform | 50,0 |
| auth | 57,5 |
| chats | 66,7 |
| realtime | 75,0 |
| message actions | 80,4 |
| conversation | 81,0 |
| brand | 100,0 |

## Los 10 faltantes más importantes (orden de construcción, solo must-haves en `no`)

1. **Enviar archivo con selector, diálogo de envío con pie de foto, arrastrar y soltar** (attachments, S20/S21): la UI solo envía stickers y GIF; `send_media` ya existe en el backend (tc-telegram `media.rs`).
2. **Progreso de subida/descarga con cancelar y descargar/guardar archivo** (attachments): hoy los documentos no se bajan a disco.
3. **Archivos/documentos como tipo de mensaje** (message types): se mapean como `MediaKind::File` sin acción.
4. **Búsqueda dentro de un chat** (search, S23): FTS5 existe en tc-store (`messages.rs`) pero sin UI; la búsqueda global es solo un filtro de la lista.
5. **Ajustes de notificaciones por tipo (S28) y notificaciones de escritorio del SO (S34)** (settings, notifications).
6. **Editar perfil propio** (settings, S27).
7. **Crear chat privado nuevo** (chats, S36; el grupo básico es should).
8. **Carpetas de chats propias** (folders, S09/S10): solo hay 5 filtros fijos.
9. **Registro de cuenta nueva** (auth, S06): la UI dice que no registra cuentas (decisión posible: pasarlo a `skip`).
10. **Soporte de portal de archivos y selector nativo** (platform): condiciona el punto 1 en Linux.

Además, entre los must-haves en `partial` conviene cerrar: cerrar sesión (falta el botón), vídeos (reproducción), tema persistente y reconexión con recuperación de huecos probada.

## Filas «partial» por no estar probadas contra Telegram real (`[sin real]`): 33

Lo único verificado en real (2026-11-09) es: login por teléfono, recuperar la sesión, lista de chats y enviar/recibir texto en Mensajes guardados y chats normales. Todo lo siguiente está implementado con pruebas contra el mock o un transporte falso, pero **no** contra Telegram:

- auth: verificación con reenvío/cambio de número, 2FA, QR.
- chats: fijar, silenciar, archivar, menús contextuales de chat (leído/no leído, vaciar, borrar, salir).
- lectura: abrir en el primer no leído, marcar como leído progresivo, reconexión/`getDifference`.
- mensajes: fotos y visor, notas de voz, audio/música, stickers (envío, selector, animados TGS), GIFs (incluido MP4/H.264 con openh264), reenviar, fijar mensaje, selección múltiple, reacciones (el arreglo de `PREMIUM_ACCOUNT_REQUIRED` no se reprobó).
- foros (grupos con temas).
- perfil: información de usuario y de grupo, bloquear.
- seguridad: chats secretos (crear/DH, texto E2E, huella, TTL), todos EXPERIMENTALES, probados solo entre instancias propias, sin interoperabilidad verificada ni auditoría criptográfica.
- medios: miniaturas y caché, silenciar por chat (`mute_until`).

Las demás filas `partial` están incompletas por diseño o por falta de piezas (cerrar sesión sin botón, vídeos sin reproducción, tema sin persistencia, spoilers y enlaces sin clic, contactos sin pantalla, carpetas propias, búsqueda solo en la lista, servicio de mensajes parcial, Wayland sin portal de archivos).

## Diferencias de comportamiento (paso 3 de la skill)

No se pudo recorrer el original; no hay comparación de clics ni de errores frente a él. Queda pendiente cuando haya acceso a la cuenta de referencia.

## Siguientes pasos

1. Pasar a `yes` lo `[sin real]` con una segunda prueba real guiada (`replica/test-plan.md`).
2. Construir los puntos 1 a 3 de la lista (adjuntos), que desbloquean 7 must-haves.
3. Reejecutar: `python3 <skill>/parity.py replica/features.csv`.

Listado completo de faltantes por orden de construcción: salida de `parity.py` (reproducible con el comando anterior).
