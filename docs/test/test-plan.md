# Plan de pruebas — generado desde `replica/recon.md`

Fecha: 2026-11-06 · Skill: `replica-skill:replica-test` (adaptada: la app es nativa en iced, no web; no hay Playwright).

## 1. Estrategia

| Nivel | Qué | Herramienta | Notas |
| --- | --- | --- | --- |
| A. Update/estado | Funciones puras `update(state, mensaje)` de tc-ui/tc-app contra `MockBackend` | `cargo test` con mensajes de iced simulados, sin ventana | Automatizable y rápido. Cubre la lógica de cada flujo. |
| B. Vista/estructura | Que `view()` produce los controles esperados en cada estado | tests con el selector de iced (`iced_test`/simulator, si la versión lo permite) o comprobaciones sobre el modelo de vista | Automatizable parcialmente. |
| C. Backend | Contratos de `ChatBackend`/`AuthApi` contra mock; red real con transporte falso | `cargo test -p tc-mock -p tc-telegram -p tc-store -p tc-secret -p tc-stickers` | Ya existe parte (ver log). |
| D. Manual | Teclado real, IME, Wayland/X11, portal de archivos, notificaciones, bandeja, arrastrar y soltar, rendimiento de scroll, red real | Persona + lista de comprobación | Ver columna «Manual». |

Convenciones: ID `T-Fxx-nn`; Tipo HP = happy path, ED = borde. «A» = automatizable (nivel A/B/C), «M» = manual.

## 2. Casos por flujo

### F01 Iniciar sesión con teléfono (S01 → S02 → S04 → S05 → S06 → S07)
| ID | Tipo | Caso | Auto |
| --- | --- | --- | --- |
| T-F01-01 | HP | Teléfono válido, código correcto → `LoggedIn` y shell visible | A (mock: `MOCK_LOGIN_CODE`) |
| T-F01-02 | ED | Teléfono vacío/inválido: botón deshabilitado y error de campo | A |
| T-F01-03 | ED | Código incorrecto, luego correcto: mensaje, campo limpio, sin avanzar | A |
| T-F01-04 | ED | Código expirado/reenvío: contador, reenviar activa nuevo código | A |
| T-F01-05 | ED | Teléfono acabado en 2: pasa por S05 (2FA) con contraseña correcta/incorrecta | A (mock) |
| T-F01-06 | ED | Teléfono acabado en 0: pasa por S06 (registro) con nombre vacío/válido | A (mock) |
| T-F01-07 | ED | Flood-wait: se muestra cuenta atrás y no se puede reintentar antes | A (error simulado) |
| T-F01-08 | ED | Sin red durante el login: error recuperable, se conserva lo escrito | A |
| T-F01-09 | ED | Migración de DC durante el login | A (nivel C, transporte falso) |
| T-F01-10 | HP | Sesión persistida: segundo arranque salta a S07 | A (store de sesión en memoria/fichero) |
| T-F01-11 | ED | Login contra la red real (código SMS real, 2FA real) | M |

### F02 Login con QR (S01 → S02 → S03 → S07)
| T-F02-01 | HP | QR mostrado, aceptado tras `qr_accept_delay` → `LoggedIn` | A (mock) |
| T-F02-02 | ED | QR caduca: se refresca sin intervención | A |
| T-F02-03 | ED | Aceptado y requiere 2FA → S05 | A |
| T-F02-04 | ED | Legibilidad/escaneo del QR con móvil real, contraste en ambos temas | M |

### F03 Abrir chat y leer (S07 → S08 → S11)
| T-F03-01 | HP | Pulsar fila selecciona chat, carga 1.ª página (60), marca leído | A |
| T-F03-02 | ED | Chat con no leídos: separador «Sin leer desde aquí» y contador | A |
| T-F03-03 | ED | Historial largo (BIG_CHAT 5000): solo se instancian filas visibles; paginación al subir | A (ya existe) |
| T-F03-04 | ED | Chat vacío, fin de historial, sin conexión (estado visible) | A |
| T-F03-05 | ED | Saltar automáticamente a la marca de no leídos (hoy parcial) | A (cuando se implemente) |
| T-F03-06 | ED | Fluidez de scroll a 60 fps con 10 000 mensajes en wgpu y tiny-skia | M (medida) |

### F04 Enviar mensaje de texto (S11 → S13)
| T-F04-01 | HP | Enter envía; estado enviando → enviado; el compositor se vacía | A |
| T-F04-02 | ED | Shift+Enter inserta salto de línea, no envía | A (evento de teclado simulado) + M (teclado real) |
| T-F04-03 | ED | Texto vacío o solo espacios: no se envía | A |
| T-F04-04 | ED | Mensaje de más de 4096 caracteres: aviso y no se envía | A |
| T-F04-05 | ED | Fallo de envío: estado error y reintento con el mismo `random_id` (sin duplicar) | A |
| T-F04-06 | ED | Canal sin permiso de escritura: compositor deshabilitado | A |
| T-F04-07 | ED | Borrador: se guarda al cambiar de chat, se restaura, se borra al enviar | A |
| T-F04-08 | ED | IME (entrada compuesta), emoji de sistema, pegado de texto grande | M |

### F05 Responder (S11 → S14 → S13)
| T-F05-01 | HP | Elegir «Responder»: barra de cita en compositor, envío con `reply_to` | A |
| T-F05-02 | ED | Cancelar respuesta con ✕/Esc | A |
| T-F05-03 | ED | Mensaje original borrado: la cita muestra «mensaje no disponible» | A |
| T-F05-04 | ED | Clic en la cita salta al original (aunque no esté cargado) | A |

### F06 Reenviar (S11 → S14/S18 → S16)
| T-F06-01 | HP | Reenviar uno a un chat: aparece con atribución | A (cuando exista) |
| T-F06-02 | ED | Sin selección el botón está deshabilitado; varias selecciones | A |
| T-F06-03 | ED | Chat sin permiso; restricción de reenvío; chat secreto no permite reenviar | A |
| T-F06-04 | ED | Ocultar remitente (could) | A |

### F07 Editar (S11 → S14 → S13)
| T-F07-01 | HP | Editar propio: texto cargado, guardar, aparece «editado» | A |
| T-F07-02 | ED | ↑ en compositor vacío edita el último propio | A |
| T-F07-03 | ED | Mensaje ajeno: no hay acción de editar | A |
| T-F07-04 | ED | Fuera de plazo/no permitido: error visible | A |
| T-F07-05 | ED | Guardar sin cambios o vacío: no se envía/avisa | A |

### F08 Borrar (S11 → S14 → S17)
| T-F08-01 | HP | Borrar para todos: desaparece y se emite evento a los demás | A |
| T-F08-02 | ED | Solo para mí (casilla desmarcada) | A |
| T-F08-03 | ED | Mensaje ajeno en grupo sin permiso de admin: solo «para mí» | A |
| T-F08-04 | ED | Cancelar el diálogo no cambia nada; selección múltiple (cuando exista) | A |

### F09 Reaccionar (S11 → S14/S15)
| T-F09-01 | HP | Poner reacción: chip con contador 1 y estado propio | A |
| T-F09-02 | ED | Quitar la propia con un segundo clic | A |
| T-F09-03 | ED | Varias personas con la misma/distinta reacción; límite por chat | A |
| T-F09-04 | ED | Clic derecho abre el selector (hoy), doble clic (según recon) | M |

### F10 Enviar archivo/foto (S13 → S20 → S21)
| T-F10-01 | HP | Seleccionar archivo, previsualizar, enviar con pie de foto | A (cuando exista) + M (portal de archivos) |
| T-F10-02 | ED | Arrastrar y soltar uno/varios | M |
| T-F10-03 | ED | Archivo > 2 GB rechazado con mensaje | A |
| T-F10-04 | ED | Cancelar subida, reanudar, progreso | A (mock) + M (red real) |
| T-F10-05 | ED | Comprimir foto vs enviar como archivo; álbum | A |
| T-F10-06 | ED | Pegar imagen del portapapeles | M |

### F11 Buscar (S08 → S22 → S11, S23)
| T-F11-01 | HP | Término con resultados en chats, contactos, mensajes; abrir mensaje salta y resalta | A (cuando exista) |
| T-F11-02 | ED | Sin resultados, vacío, cargando; cancelar con Esc | A |
| T-F11-03 | ED | Búsqueda en chat con filtros (medios, enlaces, archivos) y navegar «N de M» | A |
| T-F11-04 | ED | Acentos, mayúsculas, caracteres especiales (FTS5 en tc-store) | A (nivel C) |

### F12 Carpetas (S26 → S10 → S09 → S08)
| T-F12-01 | HP | Crear carpeta con chats incluidos; aparece pestaña con contador | A (cuando exista) |
| T-F12-02 | ED | Carpeta vacía; chat en varias; límite; renombrar/borrar/ordenar | A |
| T-F12-03 | ED | Filtros fijos actuales (Todos/Sin leer/Personas/Grupos/Canales) filtran la lista | A (ya aplicable) |

### F13 Info de chat (S12 → S24/S25)
| T-F13-01 | HP | Abrir info de usuario/grupo | A |
| T-F13-02 | ED | Silenciar/bloquear cambia estado y badge; contacto vs no contacto | A |

### F14 Notificaciones (S26 → S28; S08/S12)
| T-F14-01 | HP | Interruptores por tipo se guardan | A |
| T-F14-02 | ED | Silenciar por chat; «silenciar hasta» | A |
| T-F14-03 | ED | Permiso del SO denegado; modo no molestar | M |

### F15 Sesiones (S26 → S29 → S33)
| T-F15-01 | HP | Listar sesiones, cerrar una remota | A |
| T-F15-02 | ED | Una sola sesión (la actual) no se puede cerrar desde la lista; error de red | A |

### F16 Tiempo real (evento → S08 → S34 → S35)
| T-F16-01 | HP | Mensaje entrante: lista se reordena, no leídos +1, burbuja nueva | A (ya existe, suscripción al mock) |
| T-F16-02 | ED | Leyendo más arriba: no se mueve el scroll (compensación) | A |
| T-F16-03 | ED | Chat silenciado: contador gris, sin notificación | A |
| T-F16-04 | ED | App en foco, chat abierto: marca leído sin notificar | A |
| T-F16-05 | ED | Reconexión con huecos (`getDifference`), sin duplicar ni perder | A (nivel C, transporte falso) + M (red real) |
| T-F16-06 | HP | Notificación del SO con vista previa; icono de bandeja con contador | M |

### F17 Stickers (S13 → S38 → S11; S39)
| T-F17-01 | HP | Abrir selector, elegir sticker → mensaje sticker sin marco, recientes se actualiza | A |
| T-F17-02 | ED | Sin packs instalados: estado vacío con acción de explorar | A |
| T-F17-03 | ED | Instalar/quitar pack desde S39 y reordenar en Ajustes; límite de packs | A |
| T-F17-04 | ED | Marcar/quitar favorito; búsqueda por emoji sin resultados | A |
| T-F17-05 | ED | WebP inválido o muy grande (>512): fallback sin pánico | A (tc-stickers) |
| T-F17-06 | ED | Sticker animado (TGS): reproduce o muestra fallback estático; movimiento reducido lo detiene | A (fallback) + M (animación) |
| T-F17-07 | ED | Sin conexión: solo recientes de caché; canal sin permiso: botón deshabilitado | A |
| T-F17-08 | ED | Rendimiento de rejilla con muchos packs; descarga y caché LRU | M |

### F18 Chat secreto (S24 → S11 → S40 → S41)
| T-F18-01 | HP | Iniciar chat secreto entre dos cuentas del mock: estados Waiting → Active | A (tc-secret/mock) |
| T-F18-02 | HP | Enviar y recibir texto cifrado; el texto aparece descifrado en ambos lados | A (ya hay e2e en tc-secret) |
| T-F18-03 | ED | Rechazo o descarte: chat cerrado, historial según opción | A |
| T-F18-04 | ED | Peticiones cruzadas: se aborta la de `exchange_id` menor | A (ya probado) |
| T-F18-05 | ED | Huella: ambos lados ven la misma rejilla y 4 grupos de cifras (S40); alterar clave muestra aviso | A |
| T-F18-06 | ED | TTL: fijar en S41; arranca al leer; el mensaje se borra al vencer | A (parcial hoy: solo se muestra el TTL) |
| T-F18-07 | ED | Sin reenvío ni multi-dispositivo: acciones ausentes en el menú | A |
| T-F18-08 | ED | Medios cifrados (should); aviso de captura recibido | A (cuando exista) |
| T-F18-09 | ED | Interoperabilidad con un cliente real; revisión de seguridad externa | M (pendiente; no verificado) |

## 3. Pruebas transversales

| ID | Caso | Auto |
| --- | --- | --- |
| T-X-01 | Tema claro/oscuro: conmutador cambia tokens; seguir al portal XDG (hoy no) | A + M |
| T-X-02 | Contraste AA con `tools/design/contrast.py` tras cualquier cambio de paleta | A (script) |
| T-X-03 | Movimiento reducido desactiva animaciones (puntos de «escribiendo», muelles) | A |
| T-X-04 | Atajos de teclado básicos y navegación por Tab con foco visible | M |
| T-X-05 | Wayland y X11: ventana, portal de archivos, portapapeles, bandeja | M |
| T-X-06 | Barrido de marca limpio: `python3 tools/brand/sweep.py --strict` | A |
| T-X-07 | Escalado de texto y HiDPI; cadenas largas no rompen el diseño | M |

## 4. Orden y criterios

- Prioridad de automatización: F04, F16, F03, F05, F07, F08, F09 (ya hay base en el mock), luego F01/F02 (mock de auth), F17, F18.
- Los casos marcados «cuando exista» dependen de funciones con `clone=no` en `features.csv`; se activan al construirlas.
- Criterio de salida de una fase: todos los casos A del flujo pasan, clippy limpio y los M del flujo ejecutados una vez y anotados en `docs/logs/log.md`.
- Informe de errores: título, severidad (bloqueante/alta/media/baja), pasos, esperado vs obtenido, entorno (Wayland/X11, tema, versión), captura.
- Nada de esto se ha ejecutado aún contra la red real de Telegram.
