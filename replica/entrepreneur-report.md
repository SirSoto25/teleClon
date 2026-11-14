# Informe de emprendedor: qué molesta, qué falta y qué plan de mejoras para Parlo

Fecha: 2026-11-14. Skill: `replica-skill:replica-entrepreneur`, adaptada.

**Aviso de alcance.** Parlo es un cliente de escritorio de Telegram para uso PERSONAL, sin comercializar. El ángulo de venta (sección 6) se documenta por completitud, pero no se persigue: no hay nombre comercial, tienda, precios ni marketing en marcha. El valor de este informe es decidir qué construir a continuación.

## 0. Muestra y límites (leer primero)

Nada de lo de abajo está inventado. Cada cita es literal y trae su enlace. Pero la muestra es pequeña y desigual:

| Fuente | Qué se leyó | Estado |
| --- | --- | --- |
| GitHub `telegramdesktop/tdesktop` (API pública de búsqueda e issues) | Los 40 issues abiertos con más 👍 y unos 30 cerrados con muchos 👍; del cuerpo se leyeron las primeras líneas de 31 issues. Los 👍 son los del 2026-11-14. | Leído |
| Hacker News (API Algolia) | 25 comentarios recientes sobre Telegram Desktop; 5 se releyeron completos por la API de ítems. | Leído, pero casi todo son elogios |
| Reddit (r/Telegram) | Intento de lectura bloqueado por la herramienta de acceso web. | NO consultado: no hay citas de Reddit |
| Foro oficial de feedback (`bugs.telegram.org`), tiendas de apps, G2/Capterra/Trustpilot | No consultados. Las tiendas no aplican (es una app de escritorio) y el foro no se pudo leer. | NO consultado |
| README de Kotatogram | Leído para ver qué añaden los forks. | Leído |

Lectura honesta: es una muestra de **unas 55 piezas en 2 fuentes** (frente al objetivo de la skill de 100 o más en 3 o más). Un issue con 200 👍 pesa más que un comentario suelto, pero los 👍 miden interés, no quejas. Donde un tema aparece en una sola fuente se marca como delgado.

Los comentarios de GitHub de cada issue (los hilos de cientos de respuestas) no se leyeron: solo título, 👍, número de comentarios y primeras líneas. Faltan por revisar para confirmar el estado de varios issues.

## 1. Lo que MÁS molesta (quejas sobre lo que el original hace)

Ordenado por peso de evidencia (👍 y fuentes). Todas son de GitHub salvo que se diga.

1. **Notificaciones: ruido y poco control** (varios issues, una sola fuente: tema de peso medio, no delgado por número de issues).
   - #964, 251 👍, cerrado: "Should be great to have the same option you got on the Android client to ignore the message '... just joined Telegram' when someone in your contact list creates a Telegram account." https://github.com/telegramdesktop/tdesktop/issues/964
   - #277, 192 👍, cerrado: "when using Telegram Desktop is it possible to silence out the notifications on the phone?" https://github.com/telegramdesktop/tdesktop/issues/277
   - #91, 68 👍, cerrado: "Use the desktop native notification system in Linux to a better user experience." https://github.com/telegramdesktop/tdesktop/issues/91
   - #24556 (24 👍, abierto) "Completely ignore notifications for selected chats" y #7529 (24 👍, abierto) "Mute notification per folder": solo títulos leídos.
2. **Escala, DPI y desplazamiento en Linux/macOS** (una fuente; varios issues).
   - #1121, 244 👍, 251 comentarios, **abierto desde 2015**: "This is an issue in multi-screen environments with different DPI scaling per screen." https://github.com/telegramdesktop/tdesktop/issues/1121
   - #17254, 69 👍, cerrado: "Telegram desktop doesn't respond to any scroll wheel action." https://github.com/telegramdesktop/tdesktop/issues/17254
   - #30795, cerrado, versión 6.9: "Mac M3/M5. On both machines the new update causes lagging scroll in both the contact list and in the chat." https://github.com/telegramdesktop/tdesktop/issues/30795
   - #6746, 45 👍: "Kinetic scroll" (desplazamiento con inercia) como petición. Título leído; es una queja de que no existe.
3. **Consumo de memoria y CPU con uso largo** (una fuente, issues antiguos: delgado como tendencia actual).
   - #3640, 98 👍: "At some point it will start consuming lots of CPU" https://github.com/telegramdesktop/tdesktop/issues/3640
   - #2464, 30 👍: "1.5Gb RSS !!!" https://github.com/telegramdesktop/tdesktop/issues/2464
   - Contrapeso en Hacker News (el mismo tema, dicho a favor): "the only goddamn desktop messenger app that actually feels smooth" https://news.ycombinator.com/item?id=46471505 (Barrin92, 2026-01-03).
4. **Funciones de red social que algunos no quieren: historias e IA** (una fuente).
   - #26539, 100 👍, cerrado: título "[Feature Request] Add a way to hide Stories"; y #26568 (53 👍, abierto) "Add option to disable stories in search bar".
   - #30504, 179 👍, 154 comentarios, 2026: "Ability to turn off AI text editor". Se cerró con un ajuste experimental: "the experimental option in settings > advanced > experimental was added for Ai button hiding and it is released already in 6.7.4." https://github.com/telegramdesktop/tdesktop/issues/30504
5. **Vista previa de enlaces que no se puede apagar** (una fuente, tema repetido en 2015 y 2024).
   - #687, 152 👍, cerrado: "Pls add an option to disable link previews." https://github.com/telegramdesktop/tdesktop/issues/687
   - #28725, 20 👍, abierto: "I don't like the link preview widget in messages (and I believe I'm not the only one)." https://github.com/telegramdesktop/tdesktop/issues/28725
6. **Animaciones que no se pueden parar** (delgado, accesibilidad).
   - #24658, 44 👍, abierto: "Animated avatars are extremely distracting and not accessible." https://github.com/telegramdesktop/tdesktop/issues/24658
   - #6234, 63 👍, abierto: "Set 'Autoplay GIFs' to Off ... Animated stickers should respect the Autoplay GIFs setting" https://github.com/telegramdesktop/tdesktop/issues/6234
7. **Seguridad del cliente** (HN, un hilo): título del hilo "Telegram Desktop vulnerability allowed any user's file to be stolen" (2026-10-10) https://news.ycombinator.com/item?id=50044382 ; el comentario https://news.ycombinator.com/item?id=50029124 critica una "internal tool which uploads any file". Solo se leyó el título del hilo y un extracto, no el aviso técnico.

## 2. Lo que FALTA (peticiones por nombre)

1. **Atajos de teclado y navegación con teclado** (2 issues).
   - #1060, 127 👍, abierto desde 2015, título "Shortcuts": "The goal of this issue is to add the following shortcuts" https://github.com/telegramdesktop/tdesktop/issues/1060
   - #16300, 88 👍, abierto: "is it possible to add some shortcuts to navigate Telegram Desktop like in Vim?" https://github.com/telegramdesktop/tdesktop/issues/16300
2. **Chat secreto en escritorio** (HN y GitHub, 2 fuentes).
   - #871, 184 👍, 220 comentarios, abierto desde 2015: "I would be wonderful to create and send secret chats on telegram desktop." https://github.com/telegramdesktop/tdesktop/issues/871
   - HN, 2025-10-14, fsflover: "There is no option to start a secret chat for me on a right click." https://news.ycombinator.com/item?id=45583996
3. **Organizar la lista: categorías, vista compacta, tipografía, burbujas** (4 issues).
   - #1107, 138 👍, cerrado (hoy existen carpetas): "When user have a large number of chats, the list looks dirty, it is inconvenient to navigate." https://github.com/telegramdesktop/tdesktop/issues/1107
   - #5634, 18 👍, abierto: "It would make Telegram so much more friendly to use, if we had an option to turn off the chat bubbles, so that its more in the style of Discord." https://github.com/telegramdesktop/tdesktop/issues/5634
   - #90, 59 👍, abierto: "It would be cool if user could choose fonts for conversations" https://github.com/telegramdesktop/tdesktop/issues/90
   - #2235, 107 👍, cerrado: título "Make "round" message bubbles toggleable".
   - El README de Kotatogram (fork con unas 1384 estrellas a 2026-11-14, última subida 2026-10-06) lista como funciones propias: "Local folders", "Forward to multiple chats and forward without author", "Custom font", "Compact chat list", "Custom text replaces". https://github.com/Kotatogram/kotatogram-desktop
4. **Gestión de descargas y archivos** (3 issues).
   - #809, 37 👍, abierto: "I suggest that you add a page or tab or menu or something like that for managing downloads." https://github.com/telegramdesktop/tdesktop/issues/809
   - #1484, 98 👍, abierto: "I really wished I could still save the original .gif though..." https://github.com/telegramdesktop/tdesktop/issues/1484
   - #1382, 100 👍, cerrado: "Please provide an option to download all of them together, not one by one." https://github.com/telegramdesktop/tdesktop/issues/1382
5. **Varias cuentas** (#4261, 232 👍, cerrado: hoy existe en el original): "There is menu to login into (three?) different accounts" https://github.com/telegramdesktop/tdesktop/issues/4261
6. **Exportación incremental y archivo personal** (GitHub y HN, 2 fuentes, delgado).
   - #10497, 37 👍, abierto: "I periodically download all my Telegram data on my hard disk to keep a personal archive off-cloud. However, I need to redownload all the data each time" https://github.com/telegramdesktop/tdesktop/issues/10497
   - HN 2026-05-28, sixhobbits, sobre la exportación: "they removed the export functionality from the main app" https://news.ycombinator.com/item?id=48305868 (afirmación de un usuario, no verificada contra el cliente).
7. **Citas con sintaxis Markdown** (#27772, 104 👍, abierto): título "Support Quotes using Telegram's Markdown Syntax".

## 3. Lo NO resuelto (trabajos o grupos que el original ignora)

- **Escritorio sin iPhone/Android de por medio con chat secreto** (el punto 2 anterior): tdesktop no lo implementa desde 2015.
- **Accesibilidad con lector de pantalla**: #476, 32 👍, 239 comentarios, abierto desde 2015: "i wanted to introduce telegram to a blind user but had to realize, the (windows) client is not good to use with a screen reader" https://github.com/telegramdesktop/tdesktop/issues/476 . Riesgo propio: iced (la GUI de Parlo) tiene accesibilidad limitada; no se ha medido.
- **Dock oculto con icono de bandeja** (#206, 42 👍, abierto desde 2014): "There should be an option where the dock icon can be hidden when the tray is enabled." https://github.com/telegramdesktop/tdesktop/issues/206
- **Proxy y bloqueos de red**: #30733, 577 👍, 2026: "In Russia, they are once again blocking connections to MTPROTO proxies." https://github.com/telegramdesktop/tdesktop/issues/30733 . Es la issue con más 👍 de la muestra, pero depende de la región; no consta que aplique al uso del usuario.
- **Pantalla compartida/llamadas** (#16487, 66 👍): fuera del alcance de Parlo (llamadas en `skip`).

## 4. Cruce con replica/diff-report.md y replica/features.csv

Aviso de frescura: `diff-report.md` (45,0/100) es anterior a las fases A1 a A3 de archivos (envío con selector, soltar, pegar, álbum, progreso y cancelar; descarga, abrir y guardar). Las filas de `attachments` en `features.csv` siguen en `no`; en el código ya están hechas, sin verificar contra Telegram real (ver `docs/logs/log.md`, fases A1 a A3). Este cruce usa el CSV tal cual y lo corrige donde el log lo contradice.

| Queja o petición | Evidencia | Estado en Parlo | Veredicto |
| --- | --- | --- | --- |
| Chat secreto en escritorio | #871 (184 👍), HN | `partial` EXPERIMENTAL, sin interoperabilidad real verificada | Cubierto en parte; es un diferenciador si se verifica |
| Sin IA ni historias | #30504, #26539 | Parlo no las tiene (historias en `skip`) | Cubierto por ausencia |
| Vista previa de enlaces apagable | #687, #28725 | "Vista previa de enlaces" = `no` (no se genera) | Cubierto por ausencia; si se añade, ponerlo apagable |
| Rendimiento y memoria | #3640, #2464, HN | Lista con scroll virtual (páginas de 60, probada con 5000); sin medidas de RAM/CPU | Probable fortaleza, no medida |
| Notificaciones nativas, por tipo, tray, sonido | #91, #964, #277, #24556 | `notifications` 11,5/100 y `settings` 8,8/100; todo en `no` salvo silenciar por plazo | HUECO |
| Atajos / navegación por teclado | #1060, #16300 | `ux` 0,0/100; "Atajos de teclado básicos" = `no` (existen Ctrl+O y alguno suelto) | HUECO |
| Carpetas propias, lista compacta, fuente, burbujas | #1107, #5634, #90, #2235 | Carpetas propias `no` (5 filtros fijos); "Tamaño de texto y fondo" `no` | HUECO |
| Panel de descargas, GIF como .gif, bajar todo | #809, #1484, #1382 | Descarga individual hecha (A3); sin panel "Descargas" (lo dice el log) | HUECO |
| Varias cuentas | #4261 | No existe fila en `features.csv`; la sesión es una sola | HUECO (no listado) |
| Exportación/archivo incremental | #10497, HN | Parlo tiene `tc-store` (SQLite/FTS5) local; sin exportador | HUECO / oportunidad |
| Búsqueda en el chat | (no es queja de GitHub en la muestra, es must del original) | `no`; FTS5 existe sin UI | HUECO de paridad |
| Escala DPI, scroll con inercia, rueda | #1121, #6746, #17254 | Se ejecuta en Wayland y X11; sin pruebas de DPI mixto ni inercia | NO MEDIDO |
| Arrastrar y soltar en Wayland | (propio) | No llega en Wayland nativo (winit 0.30), según el log de A2 | HUECO propio |
| Reducir movimiento / parar animaciones | #24658, #6234 | Puntos de "escribiendo" respetan movimiento reducido; stickers con política de foco/movimiento (ADR-0008); sin ajuste global | Cubierto en parte |
| Accesibilidad con lector de pantalla | #476 | Sin evaluar | RIESGO desconocido |
| Proxy / FakeTLS | #30733 | Sin soporte de proxy en el CSV | HUECO opcional (solo si el usuario lo necesita) |

Resumen del cruce (16 filas): 2 cubiertas por ausencia (IA/historias y vista previa de enlaces), 3 cubiertas en parte o fortaleza sin medir (chat secreto, rendimiento, reducir movimiento), 11 huecos reales o no medidos. Los huecos coinciden con las áreas más débiles del informe de paridad (`ux`, `settings`, `notifications`, `folders`).

## 5. Plan de mejoras priorizado

Prioridad = evidencia por barato de arreglar. Tamaños: S (días), M, L. Para cada una se propone la fila de `features.csv` (`original = no`; no se han añadido al CSV porque este encargo solo escribe el informe y no debe alterar la puntuación de paridad).

| # | Mejora | Tamaño | Evidencia | Fila propuesta para features.csv |
| --- | --- | --- | --- | --- |
| 1 | **Notificaciones de escritorio reales con ajustes**: portal/libnotify, vista previa on/off, sonido, ignorar "se unió a Telegram", silenciar por carpeta, contador en bandeja. Cierra 4 filas must/should de `notifications`. | L | #91, #964, #277, #24556, #7529 (5 issues, 1 fuente; en el CSV son must) | `Ignorar aviso "contacto se unió" y silenciar por carpeta,notifications,should,no,no,` |
| 2 | **Teclado primero**: atajos completos (cambiar de chat, buscar, Ctrl+K), navegación tipo Vim opcional, y búsqueda dentro del chat con la FTS5 ya existente. | M | #1060, #16300 (2 issues); búsqueda es must del CSV | `Navegación con teclado estilo Vim (opcional),ux,could,no,no,` |
| 3 | **Apariencia a gusto**: carpetas locales, lista compacta, tipografía y tamaño, burbujas sin redondear, persistir el tema y seguir al portal XDG. | M | #1107, #5634, #90, #2235 (4 issues); Kotatogram las ofrece | `Lista de chats compacta y sin burbujas,settings,could,no,no,` |
| 4 | **Panel de Descargas** con todas las transferencias, "bajar todo" en álbumes, "guardar GIF como .gif" y reanudar. Reutiliza A1 a A3. | S a M | #809, #1382, #1484 (3 issues) | `Panel de descargas y descarga de álbum completo,attachments,should,no,no,` |
| 5 | **Archivo personal incremental**: exportar a disco desde `tc-store` solo lo nuevo (JSON/HTML + medios), y varias cuentas locales por perfil. | L | #10497 + HN 48305868; #4261 (232 👍) | `Exportación incremental del chat a disco,settings,could,no,no,` y `Varias cuentas,auth,should,no,no,` |

Complementos baratos fuera del top 5: opción para apagar vista previa de enlaces desde el primer día (#687), interruptor global de reducir movimiento (#24658), ocultar el icono de dock/bandeja (#206), spoilers y enlaces con clic (ya listados como `partial`). Antes del top 5, lo que el informe de paridad ya marca como cierre de `[sin real]` sigue siendo la prioridad de calidad, porque ninguna mejora vale si el envío real falla.

Qué no hacer: las llamadas, pantalla compartida (#16487) y Premium quedan fuera (siguen en `skip`). Proxy FakeTLS (#30733) solo si el usuario sufre bloqueos.

## 6. Ángulo de venta (documentado, NO perseguido: uso personal)

Tres opciones, cada una con su evidencia. Se recomienda la A solo como brújula de diseño.

- **A. Cliente de escritorio rápido y sin ruido** (recomendada como criterio interno). "Para quien usa Telegram en el escritorio y quiere un cliente ligero con notificaciones y teclado a su medida, Parlo hace eso en vez de ...". Evidencia: notificaciones (5 issues, 1 fuente) más teclado (2 issues) más elogios de rendimiento en HN (3 comentarios, 1 fuente): tema con 2 fuentes en lo que respecta al rendimiento, pero delgado.
- **B. Sin IA, sin historias, sin vista previa que no pediste.** Evidencia: #30504 (179 👍), #26539 (100 👍), #687 (152 👍): 3 issues, 1 fuente. Nota: el propio original ya añadió el interruptor de IA en 6.7.4, así que el argumento se debilita.
- **C. Tus mensajes en tu disco** (archivo local y exportación incremental, chat secreto de escritorio). Evidencia: #871 (184 👍), #10497, HN: 3 temas, 2 fuentes, pero el cifrado propio está sin auditar y sin verificar contra Telegram real, así que no se puede prometer.

Condicionantes no resueltos: nombre sin la marca del original (ver `docs/brand/`), uso de la API de Telegram bajo sus términos y licencias de los assets son asuntos que debe revisar quien decida comercializar, no esta skill. Las citas de esta sección son investigación, no testimonios; no se usarían en ninguna página.

## 7. Siguientes pasos

1. Si se aprueba el plan, añadir las 6 filas propuestas a `replica/features.csv` y volver a ejecutar `parity.py`.
2. Leer los hilos de #871, #1060, #1121 y #476 completos para confirmar qué está realmente resuelto.
3. Intentar Reddit y `bugs.telegram.org` con navegador, leyendo como una persona, para ampliar la muestra por encima de 100 piezas.
