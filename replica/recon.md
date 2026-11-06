# Recon map: Telegram Desktop (Linux)

Scope: núcleo de mensajería — login por teléfono/QR, lista de chats y carpetas, conversación con tipos de mensaje, compositor, responder/reenviar/editar/borrar/reacciones, búsqueda, perfil/info de chat, ajustes básicos, notificaciones, archivos adjuntos.
For: producto propio con marca nueva (clon clean-room en Rust, identidad visual propia). Telegram es solo referencia funcional.
Date: 2026-11-06

Nota de método: reconocimiento hecho solo desde documentación pública (FAQ, docs de la API/MTProto, help center, changelog de la web). No se usó cuenta, ni código fuente, ni tráfico de red. Las URLs son las páginas públicas de referencia; el contenido es conocimiento consolidado de ellas, y algunos detalles de UI (estados, atajos) son **inferencias a validar** con capturas del usuario en `replica/screens/` (vacío por ahora).

## Sources

| # | source | URL | notes |
| --- | --- | --- | --- |
| 1 | FAQ oficial | https://telegram.org/faq | funciones, límites, carpetas, chats secretos |
| 2 | Páginas de apps / Desktop | https://desktop.telegram.org | descripción de la app de escritorio |
| 3 | Changelog | https://telegram.org/blog | novedades (reacciones, carpetas, temas, etc.) |
| 4 | Docs API (MTProto, métodos, tipos) | https://core.telegram.org/api | modelo de datos casi completo |
| 5 | Login / autorización | https://core.telegram.org/api/auth | flujo teléfono + código + 2FA |
| 6 | Login por QR | https://core.telegram.org/api/qr-login | token QR, aceptación desde otro dispositivo |
| 7 | Mensajes y entidades | https://core.telegram.org/api/entities | formato de texto (negrita, spoiler, enlaces) |
| 8 | Reacciones | https://core.telegram.org/api/reactions | emoji, premium, estados |
| 9 | Carpetas | https://core.telegram.org/api/folders | filtros de chats |
| 10 | Notificaciones | https://core.telegram.org/api/notify | ajustes por chat/tipo, silenciar |
| 11 | Búsqueda | https://core.telegram.org/api/search | global, en chat, por filtros de medios |
| 12 | Archivos | https://core.telegram.org/api/files | subida por partes, límites |
| 13 | Edición/borrado/reenvío | https://core.telegram.org/api/forwards | reenvío y atribución |
| 14 | Help center / soporte | https://telegram.org/support | ajustes, privacidad |
| 15 | Tu cuenta (opcional) | n/a | el usuario puede aportar capturas; no se pidió |

## Core loop

Abrir la app, ver tus chats ordenados por actividad reciente, entrar a uno y enviar/recibir mensajes (texto, medios, archivos) al instante y sincronizados entre dispositivos.

## Screens

| ID | screen | route / how to reach | purpose | key components | states seen |
| --- | --- | --- | --- | --- | --- |
| S01 | Bienvenida | primer arranque | introducir y empezar | logo, botón "Empezar", selector idioma | inicial |
| S02 | Login: teléfono | desde S01 | pedir número | selector país, campo teléfono, "Iniciar con QR" | vacío, inválido, flood-wait, error red |
| S03 | Login: QR | enlace en S02 | iniciar sesión escaneando desde móvil | código QR que se refresca, instrucciones | cargando, expirado/refrescando, aceptado |
| S04 | Login: código | tras S02 | verificar código SMS/app | campo código, reenviar, cambiar número | vacío, incorrecto, expirado, reenvío |
| S05 | Login: contraseña 2FA | si hay 2FA | verificar contraseña en la nube | campo contraseña, pista, "olvidé" | incorrecto, bloqueo temporal |
| S06 | Registro (cuenta nueva) | número nuevo | nombre y foto | nombre, apellido, avatar | validación |
| S07 | Ventana principal / shell | tras login | contenedor 2 paneles | barra lateral, panel de chat, menú hamburguesa | sin chat seleccionado, offline/reconectando |
| S08 | Lista de chats | panel izquierdo | ver y abrir chats | fila de chat (avatar, título, último mensaje, hora, no leídos, silenciado, fijado, estado) | vacío, cargando, filtrada por carpeta, con borrador, con escribiendo… |
| S09 | Barra de carpetas | sobre/lado de S08 | filtrar chats | pestañas de carpeta, contadores | una carpeta, varias, vacía |
| S10 | Editor de carpetas | Ajustes > Carpetas | crear/editar carpetas | nombre, chats incluidos/excluidos, tipos | nueva, edición |
| S11 | Conversación | clic en chat | leer y escribir | cabecera, lista de mensajes, burbujas, separadores de fecha, indicador no leído, salto al final | vacía, cargando historial, con no leídos, fin de historial, sin conexión |
| S12 | Cabecera de chat | parte de S11 | contexto y acciones | avatar, título, estado (en línea/escribiendo/miembros), búsqueda, menú | usuario, grupo, canal, bot |
| S13 | Compositor | parte de S11 | escribir y enviar | campo multilinea, adjuntar, emoji/sticker, enviar/voz, barra de respuesta/edición, formato | vacío, con texto, respondiendo, editando, grabando voz, canal sin permiso |
| S14 | Menú contextual de mensaje | clic derecho / mantener | acciones | responder, reenviar, editar, copiar, fijar, borrar, seleccionar, reacciones | propio, ajeno, sin permiso de borrar |
| S15 | Selector de reacciones | desde S14 / doble clic | reaccionar | fila de emojis, ampliar | con/sin reacción propia, premium (fuera) |
| S16 | Diálogo reenviar | desde S14 | elegir destino | buscador, lista de chats multi-selección, comentario | sin selección, varias, enviado |
| S17 | Diálogo borrar | desde S14 | confirmar borrado | casilla "borrar para todos", confirmar | propio (con ventana), ajeno |
| S18 | Selección múltiple | desde S14 | acciones en lote | barra con reenviar/borrar/copiar, contador | N seleccionados |
| S19 | Visor de medios | clic en foto/vídeo | ver en grande | zoom, siguiente/anterior, descargar, reenviar | imagen, vídeo, cargando, error |
| S20 | Selector de adjuntos | clip del compositor | elegir qué enviar | foto/vídeo, archivo, ubicación(could) | menú, arrastrar y soltar |
| S21 | Diálogo de envío de archivos | tras elegir | previsualizar y enviar | miniaturas, pie de foto, comprimir o enviar como archivo, grupo (álbum) | uno, varios, grande, error |
| S22 | Búsqueda global | cuadro superior de S08 | encontrar chats/mensajes | campo, resultados por chats, contactos, mensajes | vacío, resultados, sin resultados, cargando |
| S23 | Búsqueda en chat | lupa de S12 | buscar dentro | campo, filtros (medios, enlaces, archivos), calendario(could), navegar coincidencias | sin resultados, N de M |
| S24 | Info de usuario | clic en cabecera | perfil | avatar, nombre, usuario, teléfono, bio, medios compartidos, silenciar, bloquear | contacto, no contacto, bloqueado |
| S25 | Info de grupo/canal | clic en cabecera | detalles | título, descripción, miembros, medios, permisos | grupo, canal, admin vs miembro |
| S26 | Ajustes (menú) | hamburguesa > Ajustes | entrada a ajustes | lista de secciones, perfil propio | — |
| S27 | Ajustes: cuenta/perfil | S26 | editar nombre, foto, bio, usuario | campos, subida de avatar | guardado, usuario ocupado |
| S28 | Ajustes: notificaciones | S26 | controlar avisos | interruptores por tipo (privados/grupos/canales), vista previa, sonido | activado/desactivado |
| S29 | Ajustes: privacidad y seguridad | S26 | privacidad básica | visto por última vez, sesiones activas, contraseña 2FA, bloqueados | —; (resto fuera) |
| S30 | Ajustes: chat (apariencia) | S26 | tema, tamaño de texto, fondo | claro/oscuro, escala, fondo | — |
| S31 | Ajustes: datos y almacenamiento | S26 | descarga automática, caché | límites por tipo, ruta de descargas | — |
| S32 | Ajustes: idioma | S26 | idioma de la app | lista de idiomas | — |
| S33 | Sesiones activas | S29 | gestionar dispositivos | lista, cerrar sesión remota | una, varias |
| S34 | Notificación del sistema | evento entrante | avisar mensaje nuevo | título, vista previa, responder rápido(could) | normal, silenciada, sin permiso |
| S35 | Icono de bandeja / badge | siempre | contador no leídos | icono, menú (abrir/salir) | 0, N |
| S36 | Diálogo crear chat/grupo | menú > nuevo | iniciar conversación | buscador de contactos, selección | nuevo privado, nuevo grupo |
| S37 | Lista de contactos | menú > Contactos | encontrar personas | filas, búsqueda | vacía, con contactos |
| S38 | Selector de stickers | botón emoji/sticker de S13 | elegir y enviar un sticker | pestañas (recientes, favoritos, packs instalados), rejilla de stickers, búsqueda por emoji, pack propio original | vacío (sin packs), cargando, con recientes, sin resultados, error de red |
| S39 | Vista de pack de stickers | clic en un sticker de S11 o en un pack de S38; Ajustes > Stickers | previsualizar, instalar o quitar un pack | título, rejilla de vista previa, botón instalar/quitar, reordenar packs (en Ajustes) | no instalado, instalado, instalando, límite de packs, error |
| S40 | Huella de clave (chat secreto) | cabecera/info de un chat secreto (S12/S24) | verificar visualmente la clave E2E con el interlocutor | rejilla 4x4 de colores, 4 grupos de 5 cifras, texto explicativo | clave establecida, esperando al otro lado, clave distinta (aviso) |
| S41 | Selector de temporizador de autodestrucción | cabecera/menú de un chat secreto | fijar el TTL de los mensajes | lista de plazos (apagado, segundos, minutos, horas, días, semana), confirmar | apagado, activo (TTL actual), sin permiso hasta que el chat esté activo |

## Flows

```
F01 Iniciar sesión con teléfono
    S01 -> S02 -> S04 -> (S05 si 2FA) -> (S06 si cuenta nueva) -> S07
    happy path clicks: ~5 (número, siguiente, código)
    edge: código erróneo/expirado, flood-wait, 2FA, número nuevo, sin red, cambio de DC (migración)

F02 Iniciar sesión con QR
    S01 -> S02 -> S03 -> S07
    happy path clicks: 2 en escritorio + aceptar en móvil
    edge: QR expirado (se refresca), 2FA tras aceptar, migración de DC

F03 Abrir chat y leer
    S07 -> S08 -> S11
    happy path clicks: 1
    edge: no leídos (salto al primero), historial largo (paginación), sin conexión

F04 Enviar mensaje de texto
    S11 -> S13 -> enviar
    happy path clicks: 1 (Enter)
    edge: envío pendiente/fallido (reintento), mensaje largo (límite 4096), sin permiso en canal, formato

F05 Responder a un mensaje
    S11 -> S14 -> S13 (barra de respuesta) -> enviar
    happy path clicks: 2
    edge: mensaje original borrado, responder desde otro chat (cita)

F06 Reenviar mensajes
    S11 -> S14/S18 -> S16 -> enviar
    happy path clicks: 3
    edge: chat sin permiso, ocultar remitente, multi-selección, restricción de reenvío

F07 Editar mensaje
    S11 -> S14 -> S13 (modo edición) -> guardar
    happy path clicks: 2
    edge: mensaje ajeno, fuera del plazo, marca "editado"

F08 Borrar mensaje
    S11 -> S14 -> S17 -> confirmar
    happy path clicks: 3
    edge: borrar solo para mí vs para todos, mensaje ajeno en grupo, selección múltiple

F09 Reaccionar
    S11 -> S14/S15
    happy path clicks: 1-2
    edge: quitar reacción, reacciones limitadas por chat, varios usuarios

F10 Enviar archivo/foto
    S13 -> S20 -> S21 -> enviar
    happy path clicks: 3 (o arrastrar y soltar)
    edge: archivo grande (límite 2 GB; 4 GB premium fuera), subida cancelada/reanudada, progreso, comprimir foto o como archivo

F11 Buscar
    S08 -> S22 -> abrir resultado -> S11 (salto al mensaje)
    happy path clicks: 2
    edge: sin resultados, búsqueda en chat S23 con filtros

F12 Organizar con carpetas
    S26 -> S10 -> S09 -> S08
    happy path clicks: 4
    edge: carpeta vacía, chat en varias carpetas, límite de carpetas

F13 Ver info de chat
    S12 -> S24/S25
    happy path clicks: 1
    edge: silenciar, bloquear, medios compartidos

F14 Ajustar notificaciones
    S26 -> S28 ; también S08/S12 (silenciar por chat)
    happy path clicks: 3
    edge: permiso del SO denegado, modo no molestar

F15 Gestionar sesiones
    S26 -> S29 -> S33
    happy path clicks: 3

F16 Recibir mensaje en tiempo real
    (evento) -> S08 actualiza -> S34 -> S35
    edge: chat silenciado, app en foco, reconexión con huecos (difference)

F17 Enviar y gestionar stickers (ADR-0004)
    S13 -> S38 -> elegir sticker -> S11 (burbuja sin marco)
    gestión: S11 (clic en sticker) -> S39 -> instalar/quitar ; Ajustes > Stickers -> S39 (reordenar)
    happy path clicks: 2 (abrir selector, elegir)
    edge: sin packs instalados, pack no disponible/borrado, sticker animado (TGS) con fallback estático, límite de packs,
          sin conexión (solo recientes en caché), canal sin permiso, marcar/quitar favorito

F18 Chat secreto (ADR-0004)
    S24 -> "Iniciar chat secreto" -> (espera al otro lado) -> S11 (variante secreta) -> S40 (verificar huella) -> S41 (TTL) -> enviar
    happy path clicks: ~5 (iniciar, esperar aceptación, abrir huella, fijar TTL, enviar)
    edge: el otro lado no acepta o rechaza, peticiones cruzadas simultáneas (se aborta la de exchange_id menor),
          huella distinta, TTL que arranca al leer, dispositivo distinto (sin multi-dispositivo), sin reenvío,
          descartar chat y borrar historial, aviso de captura recibido
```

## Components

| component | variants | states | used on |
| --- | --- | --- | --- |
| Button | primario, secundario, texto, icono, peligro | default, hover, focus, disabled, loading | todas |
| Campo de texto | simple, con etiqueta, contraseña, multilinea | default, focus, error, disabled | S02-S06, S13, S27 |
| Selector de país/teléfono | — | vacío, filtrando | S02 |
| Avatar | foto, iniciales, icono (guardados), con indicador en línea | cargando, fallback | S08, S11, S24 |
| Fila de chat | privado, grupo, canal, bot | normal, seleccionada, hover, no leída, silenciada, fijada | S08 |
| Insignia de no leídos | normal, silenciada, mención | 0, N, 99+ | S08, S09, S35 |
| Burbuja de mensaje | entrante, saliente, servicio | enviando, enviado, entregado, leído, error, editado | S11 |
| Cita de respuesta | en burbuja, en compositor | original disponible, borrado | S11, S13 |
| Reacciones (chips) | propia, ajena | con/sin selección, contador | S11, S15 |
| Burbuja de medios | foto, vídeo, archivo, nota de voz, sticker | descargando, subiendo, error, completa | S11, S19 |
| Compositor | texto, respuesta, edición, voz | vacío, con texto, deshabilitado | S13 |
| Menú contextual | mensaje, chat, adjuntar | abierto, ítem hover, ítem deshabilitado | S14, S20 |
| Diálogo / modal | confirmación, formulario, selector | abierto, cargando | S16, S17, S21 |
| Pestañas de carpeta | horizontal/vertical | activa, contador | S09 |
| Interruptor | — | on/off, disabled | S28, S31 |
| Toast / snackbar | info, error, deshacer | visible, auto-cierre | global |
| Cuadro de búsqueda | global, en chat | vacío, escribiendo, resultados | S22, S23 |
| Separador de fecha | — | — | S11 |
| Indicador "escribiendo…" | — | visible | S08, S12 |
| Barra de progreso | circular en medios, lineal | determinado, indeterminado | S11, S21 |
| Lista con scroll virtual | chats, mensajes | cargando más, fin | S08, S11 |
| Tooltip | — | — | global |

## Inferred data model

```
User           id, access_hash(*), first_name, last_name, username, phone, bio, photo_id,
               status (online | offline(last_seen) | recently | ...), is_bot, is_contact, is_self
               evidence: docs API User, S24, S27   confidence: high
Chat           id, kind (private | group | supergroup | channel), title, photo_id, about,
               participants_count, default_permissions, created_at
               evidence: docs API Chat/Channel, S25   confidence: high
Dialog         peer, top_message_id, unread_count, unread_mentions_count, read_inbox_max_id,
               read_outbox_max_id, notify_settings, pinned, folder_id, draft
               evidence: S08, docs API Dialog   confidence: high
Message        id (por chat), peer, from_id, date, text, entities[], reply_to(msg_id, quote),
               fwd_from(from, date, original_id), media, edit_date, out, post, pinned, grouped_id,
               reactions, views(canal), via_bot
               evidence: S11, S13, docs Message   confidence: high
Entity         type (bold | italic | code | pre | spoiler | text_url | mention | hashtag | url...),
               offset, length, url?
               evidence: docs entities   confidence: high
Media          kind (photo | video | document | voice | audio | sticker | geo...), file_id,
               size, mime, dimensions, duration, thumbs[], file_name
               evidence: S19, S21, docs files   confidence: high
Reaction       message_id, user_id, emoji, date   evidence: S15, docs reactions   confidence: high
Folder         id, title, include_peers[], exclude_peers[], flags (contactos, no contactos, grupos,
               canales, bots, no silenciados, no leídos, archivados), order
               evidence: S09, S10, docs folders   confidence: high
NotifySettings peer_scope (users | groups | channels | peer), mute_until, show_previews, sound
               evidence: S28, docs notify   confidence: high
Session        id (hash), device, app, platform, ip_country, date_created, date_active, current
               evidence: S33   confidence: high
Draft          peer, text, entities, reply_to, date   evidence: S08 (borrador)   confidence: medium
Contact        user_id, mutual, saved_name   evidence: S37   confidence: high
UpdateState    pts, qts, date, seq (por cuenta) y pts por canal
               evidence: docs updates   confidence: high (para sincronización)
Privacy rule   key (last_seen | phone | photo | forwards...), allow[], disallow[]
               evidence: S29   confidence: medium (alcance reducido)
```

(*) Detalle de protocolo; el clon propio lo define su arquitectura.

Relationships: User 1-n Dialog; Dialog n-1 Chat|User(peer); Chat 1-n Message; Message 1-n Reaction; Message 0-1 Media; Message 0-n Entity; User 1-n Folder; Folder n-n Dialog; User 1-n Session; User n-n Contact; Chat n-n User (participantes con rol).

## Feature matrix

See `features.csv`. 110 filas. Must: 54, should: 35, could: 13 (sin contar skip), skip: 8 (las filas skip tienen priority could y clone=skip). Las filas de stickers y chats secretos (ADR-0004) llevan ahora su pantalla (S38-S41) y flujo (F17/F18) en `notes`.

## Out of scope (cannot or should not be cloned)

- La **red y los servidores** de Telegram (MTProto cloud, centros de datos, almacenamiento de archivos y mensajes): el clon necesita su propio backend o ser cliente; no se replica infraestructura.
- Los **usuarios, contactos y chats existentes**: pertenecen a Telegram; un clon con backend propio parte de cero.
- **Marca**: nombre, logo, iconos, stickers, animaciones, sonidos, textos exactos. Se sustituye en replica-brand.
- (ACTUALIZADO 2026-11-06: stickers y chats secretos E2E entran en el alcance por petición del usuario; ver ADR-0004.) Fuera del slice: llamadas y videollamadas, historias, bots/Mini Apps, pagos, Premium, canales y grandes comunidades avanzadas, traducción, StarS/regalos, verificación de cuentas: fuera del slice o no clonables.
- **Registro de números/SMS**: depende de operadores y de la red de Telegram; en clon propio se sustituye.
- **Credenciales api_id/api_hash**: las aporta el usuario si se decide ser cliente alternativo (ver ADR pendiente). Nunca se inventan. Nota: usar clientes de terceros contra la red de Telegram está sujeto a sus términos para apps (https://core.telegram.org/api/terms); decisión de arquitectura pendiente.

## Size

Screens 41, flows 18, entities 14. Hard parts: (1) sincronización en tiempo real con actualizaciones/reconexión y orden consistente, (2) subida/descarga de archivos grandes por partes con reanudación y progreso, (3) lista de mensajes con scroll virtual y render de texto con entidades en Rust (GUI nativa Linux, IME, emoji, Wayland), además de login/seguridad y notificaciones del SO. Size: L (XL si se implementa el protocolo MTProto completo; M si se usa un backend propio simple).
