# Sistema de diseño de teleClon — dirección «Brasa»

> Archivo de **decisiones persistidas** (idea de *Interface Design*): cualquier agente o persona que toque la UI
> lo lee antes y lo actualiza en el mismo cambio. Si una pantalla contradice este documento, gana el documento
> o se abre un ADR nuevo. Tokens máquina-legibles: [tokens.json](tokens.json). Decisión: [ADR 0002](../adr/0002-direccion-visual.md).

Versión 1.0.0 · 2026-11-06 · skills aplicadas: `replica-skill:replica-design` (roles, escala, componentes, contraste),
`anthropic-skills:theme-factory` (tema con nombre propio y pareja tipográfica), ideas de Hallmark, Interface Design,
Emil Kowalski y Bencium (ver [research/design-skills.md](../research/design-skills.md)).

---

## 1. Dirección estética

### Elegida: «Brasa» — grafito cálido con acento persimón
Neutros de grafito con un leve tinte cálido (H≈65–80, C≤0.012) y **un único acento** persimón/brasa
(H≈40–45). Chrome sobrio y estricto; la personalidad vive en las burbujas y en el fondo del chat.

Justificación:
1. **Distancia de marca.** Telegram se identifica por azul cian, burbujas con cola, avatares circulares y fondo de
   garabatos. Brasa cambia los cuatro: acento naranja-persimón, burbujas sin cola con esquina de unión,
   avatares en cuadrado redondeado y fondo de curvas de nivel generativo. Además evita los tonos de la
   competencia directa: azul (Telegram, Signal, Messenger), verde (WhatsApp), violeta (Discord, Viber).
2. **Lectura prolongada.** Un chat se mira horas: neutros cálidos de baja croma cansan menos que el gris
   azulado por defecto y que los fondos negros puros (el oscuro arranca en L=0.17, no en 0).
3. **Anti-«aspecto IA».** Nada de degradado violeta-azul, Inter + slate por defecto, cristal esmerilado
   ni tarjetas con la misma sombra en todo (lista completa en §9).
4. **Viable en iced.** Todo se expresa con fondos sólidos, degradados lineales, radios por esquina,
   sombras de contenedor y `canvas`; no requiere desenfoque de fondo (iced no lo ofrece).

Modo **controlled** (Bencium) para el chrome: barra lateral, lista, cabecera, compositor, diálogos y ajustes usan
solo tokens, sin degradados ni variaciones. Modo **innovative** solo en burbujas (degradado vertical sutil,
geometría de unión) y fondo del chat (patrón generativo por chat).

### Alternativas descartadas
| Alternativa | Idea | Por qué se descarta |
| --- | --- | --- |
| **Vidrio glacial** (glassmorphism) | Paneles translúcidos con desenfoque sobre fondo cian/azul hielo | Cae en el azul de Telegram y en el cliché «IA 2024»; el desenfoque de fondo no existe en iced (habría que simularlo con shaders y costaría GPU/batería); contraste inestable sobre fondos variables. |
| **Neobrutalismo** | Bordes negros de 2–3 px, sombras duras desplazadas, amarillo/rosa planos | Fuerte identidad pero fatiga en uso de horas; la densidad de una lista de chats se vuelve ruidosa; envejece rápido como tendencia; choca con la integración GNOME/KDE. |

---

## 2. Color

Fuente de verdad: **OKLCH**. Los hex se calcularon con la matriz OKLab→sRGB lineal de Björn Ottosson,
con mapeo a gama por **reducción de croma** (búsqueda binaria manteniendo L y H). Los valores con croma
recortada ya aparecen con la C final. Contraste verificado con `contrast.py` de replica-design:
**144 pares, 0 fallos AA** (47 alcanzan AAA). Ida y vuelta hex→OKLCH: error máximo ΔL 0.0018, ΔC 0.0009,
ΔH 1.4° (cuantización a 8 bits). En código se usa el hex; el OKLCH sirve para derivar nuevos tonos.

### 2.1 Reglas de uso
- Los componentes solo consumen **roles**; nunca hex.
- Elevación en oscuro = **subir L** de la superficie (`bg` 0.17 → `surface` 0.205 → `surface-raised` 0.25); la sombra es secundaria.
- `accent` es relleno (botones primarios, badge de no leídos, burbuja propia en claro como tinte). Para texto
  sobre superficies se usa `accent-text`, que está ajustado para AA.
- En oscuro el texto sobre `accent` es **oscuro** (`on-accent` L=0.20): el persimón con texto blanco no llega a AA.
- `danger` (H=22–25) se separa del acento (H=40–45) por tono y luminosidad, y **siempre** se acompaña de icono o texto.
- `name-1…8`: colores de nombre de remitente en grupos, L y C constantes y H girando 45° (la uniformidad
  perceptual de OKLCH garantiza que ninguno destaque más). Índice = `hash(user_id) % 8`.
- `pattern-line` es decorativo y deliberadamente de bajo contraste (ΔL≈0.045); nunca lleva información.

### 2.2 Paleta (oscuro y claro)
| Rol | Oscuro OKLCH | Oscuro hex | Claro OKLCH | Claro hex |
| --- | --- | --- | --- | --- |
| `bg` | oklch(0.17 0.008 65) | `#120f0c` | oklch(0.972 0.007 80) | `#f8f5f1` |
| `surface` | oklch(0.205 0.009 65) | `#1a1613` | oklch(0.995 0.003 80) | `#fefdfb` |
| `surface-raised` | oklch(0.25 0.01 65) | `#25211d` | oklch(1 0 0) | `#ffffff` |
| `surface-sunken` | oklch(0.15 0.007 65) | `#0d0b08` | oklch(0.952 0.008 78) | `#f2efe9` |
| `surface-hover` | oklch(0.24 0.01 65) | `#231e1a` | oklch(0.957 0.008 75) | `#f4f0eb` |
| `surface-selected` | oklch(0.3 0.05 45) | `#422518` | oklch(0.935 0.037 52) | `#ffe3d3` |
| `border` | oklch(0.3 0.01 65) | `#312d29` | oklch(0.905 0.008 75) | `#e3dfda` |
| `border-strong` | oklch(0.4 0.012 65) | `#4c4641` | oklch(0.82 0.01 70) | `#c8c3bd` |
| `border-input` | oklch(0.56 0.012 65) | `#7a736d` | oklch(0.6 0.012 65) | `#857f79` |
| `text` | oklch(0.95 0.008 75) | `#f2eee9` | oklch(0.23 0.012 60) | `#211c17` |
| `text-muted` | oklch(0.76 0.012 70) | `#b6b0a9` | oklch(0.48 0.014 65) | `#635c56` |
| `text-placeholder` | oklch(0.68 0.012 70) | `#9d9790` | oklch(0.53 0.012 65) | `#716a65` |
| `accent` | oklch(0.72 0.165 45) | `#f67e42` | oklch(0.57 0.17 40) | `#c64913` |
| `accent-hover` | oklch(0.77 0.148 48) | `#ff955b` | oklch(0.535 0.165 38) | `#b83f12` |
| `accent-pressed` | oklch(0.67 0.16 43) | `#e46f3a` | oklch(0.5 0.15 36) | `#a73a1b` |
| `on-accent` | oklch(0.2 0.03 45) | `#21110a` | oklch(0.99 0.005 60) | `#fefbf8` |
| `accent-text` | oklch(0.79 0.13 52) | `#fca169` | oklch(0.53 0.16 40) | `#b4410d` |
| `bubble-out` | oklch(0.37 0.075 42) | `#60311e` | oklch(0.93 0.043 58) | `#ffe2cc` |
| `bubble-out-end` | oklch(0.335 0.072 36) | `#55271b` | oklch(0.91 0.053 52) | `#ffd8c2` |
| `on-bubble-out` | oklch(0.965 0.015 60) | `#fbf1ea` | oklch(0.23 0.02 50) | `#251a15` |
| `bubble-out-meta` | oklch(0.84 0.04 55) | `#e0c4b2` | oklch(0.46 0.06 48) | `#744e3a` |
| `tick-read` | oklch(0.88 0.092 70) | `#ffcd94` | oklch(0.5 0.15 38) | `#a73b15` |
| `bubble-in` | oklch(0.25 0.01 65) | `#25211d` | oklch(1 0 0) | `#ffffff` |
| `on-bubble-in` | oklch(0.95 0.008 75) | `#f2eee9` | oklch(0.23 0.012 60) | `#211c17` |
| `bubble-in-meta` | oklch(0.74 0.012 70) | `#b0aaa3` | oklch(0.48 0.014 65) | `#635c56` |
| `danger` | oklch(0.7 0.18 22) | `#fa686a` | oklch(0.53 0.2 25) | `#c51d28` |
| `danger-solid` | oklch(0.58 0.2 22) | `#d73240` | oklch(0.54 0.21 25) | `#cd1526` |
| `on-danger` | oklch(0.985 0.005 22) | `#fdf9f8` | oklch(0.99 0.005 25) | `#fffbfa` |
| `success` | oklch(0.76 0.15 155) | `#52cd86` | oklch(0.5 0.124 155) | `#007742` |
| `warning` | oklch(0.83 0.14 85) | `#f1bf4e` | oklch(0.54 0.116 72) | `#976200` |
| `focus-ring` | oklch(0.82 0.121 58) | `#ffb072` | oklch(0.56 0.169 42) | `#c14800` |
| `pattern-line` | oklch(0.215 0.012 65) | `#1d1814` | oklch(0.935 0.012 78) | `#eee9e1` |
| `name-1` | oklch(0.79 0.12 25) | `#fe9b93` | oklch(0.5 0.14 25) | `#a43b38` |
| `name-2` | oklch(0.79 0.12 60) | `#f3a768` | oklch(0.5 0.117 60) | `#925000` |
| `name-3` | oklch(0.79 0.12 100) | `#ccbc5b` | oklch(0.5 0.104 100) | `#716400` |
| `name-4` | oklch(0.79 0.12 150) | `#7fd091` | oklch(0.5 0.138 150) | `#007835` |
| `name-5` | oklch(0.79 0.12 195) | `#40d2d2` | oklch(0.5 0.085 195) | `#007272` |
| `name-6` | oklch(0.79 0.119 240) | `#6dc4ff` | oklch(0.5 0.115 240) | `#006a9d` |
| `name-7` | oklch(0.79 0.111 285) | `#b3b0ff` | oklch(0.5 0.14 285) | `#5d53ae` |
| `name-8` | oklch(0.79 0.12 330) | `#e69edf` | oklch(0.5 0.14 330) | `#8e4088` |

`surface-hover` y `surface-selected` son estados de fila; `bubble-out-end` es el extremo inferior del degradado de la burbuja propia;
`tick-read` es el doble check de leído sobre burbuja propia.

### 2.3 Transparencias (tokens `alpha`)
| Token | Oscuro | Claro | Uso |
| --- | --- | --- | --- |
| `scrim` | `#060403` @ 62 % | `#211c17` @ 40 % | Fondo tras diálogos |
| `shadow` | `#000000` @ 45 % | `#3a2a1c` @ 14 % | Base de sombras (× `alpha_scale` del nivel) |
| `highlight` | acento @ 18 % | acento @ 14 % | Mensaje al que se salta (respuesta, búsqueda) |
| `selection` | acento @ 30 % | acento @ 22 % | Selección de texto y multiselección de mensajes |
| `pressed` | blanco @ 6 % | tinta @ 5 % | Capa de pulsación sobre cualquier superficie |

### 2.4 Contraste verificado (extracto)
| Par | Oscuro | Claro | Requisito |
| --- | --- | --- | --- |
| `text` / `bg` | 16.54 | 15.54 | 4.5 |
| `text-muted` / `surface` | 8.37 | 6.46 | 4.5 |
| `text-placeholder` / `surface-sunken` | 6.80 | 4.63 | 4.5 |
| `accent-text` / `surface` | 8.94 | 5.58 | 4.5 |
| `on-accent` / `accent` | 6.95 | 4.67 | 4.5 |
| `on-bubble-out` / `bubble-out` | 9.65 | 13.74 | 4.5 |
| `bubble-out-meta` / `bubble-out` | 6.50 | 5.88 | 4.5 |
| `bubble-in-meta` / `bubble-in` | 6.94 | 6.57 | 4.5 |
| `on-danger` / `danger-solid` | 4.54 | 5.48 | 4.5 |
| `warning` / `bg` | 11.19 | 4.76 | 4.5 |
| `border-input` / `surface-sunken` | 4.21 | 3.45 | 3 (UI) |
| `focus-ring` / `bubble-out` | 5.99 | 4.05 | 3 (UI) |
| `tick-read` / `bubble-out` | 7.37 | 5.18 | 3 (UI) |
| `name-*` / `bubble-in` y `bubble-out` (peor caso) | > 4.5 | 4.55 | 4.5 |
| Iniciales de avatar `#21110a` / `name-*` oscuro | 8.98–9.89 | igual | 3 (texto grande) |

Regenerar: `python3 ~/.claude/plugins/cache/replica-skill/replica-skill/1.0.0/replica-design/contrast.py docs/design/tokens.json`
(sale con código 1 si algún par falla). Cualquier cambio de paleta, incluido el de replica-brand, pasa por aquí.

---

## 3. Tipografía

Todas libres (OFL‑1.1) y **empaquetadas en el binario** (`include_bytes!` + carga de fuentes de iced) para que el
render sea idéntico en cualquier distro; se usan instancias estáticas (Regular 400, Medium 500, SemiBold 600, Bold 700).

| Papel | Familia | Disponibilidad Linux | Motivo |
| --- | --- | --- | --- |
| UI y mensajes | **Atkinson Hyperlegible Next** | Google Fonts / Braille Institute; AUR | Diseñada para legibilidad (glifos 0/O, l/I/1 diferenciados), carácter propio sin ser «la Inter de siempre» |
| Respaldo UI | IBM Plex Sans → Noto Sans → Cantarell | Repos oficiales de Arch, Debian, Fedora (Noto y Cantarell ya instaladas aquí) | Cobertura de escrituras no latinas vía Noto |
| Display | **Bricolage Grotesque** | Google Fonts | Solo onboarding, estados vacíos y nombre en el perfil: el único guiño expresivo del texto |
| Mono | **JetBrains Mono** | Repos oficiales | Bloques de código y `inline code` |
| Emoji | Noto Color Emoji (del sistema) | Instalada | No se empaqueta (peso); respaldo al del sistema |

### Escala (px lógicos; la escala de usuario multiplica todo)
| Token | Tamaño / interlínea | Peso | Uso |
| --- | --- | --- | --- |
| `caption` | 12 / 16 | 500, tracking +0.2 | Badges, etiquetas de sección, «editado» |
| `meta` | 13 / 18 | 400 | Hora, vista previa del último mensaje, estado en cabecera |
| `ui` | 14 / 20 | 400 | Menús, ajustes, campos |
| `ui-strong` | 14 / 20 | 600 | Nombre en lista de chats, botones |
| `body` | 15 / 22 | 400 | Texto de mensaje y compositor |
| `title` | 16 / 22 | 600 | Nombre en cabecera, título de diálogo |
| `heading` | 20 / 26 | 700 | Secciones de ajustes y perfil |
| `display` | 28 / 34 | 700 (Bricolage) | Onboarding, vacíos |
| `emoji-only` | 40 / 46 | — | Mensaje compuesto solo de 1–3 emojis |
| `code` | 13 / 20 | 400 (mono) | Código |

Escala de usuario persistida: 0.875 – 1.25 en pasos de 0.125 (por defecto 1.0). Longitud de línea de mensaje ≤ 72 caracteres
gracias a `bubble_max_width` 560.

---

## 4. Espaciado, radios, bordes

**Espaciado** base 4: `0, 2, 4, 6, 8, 12, 16, 20, 24, 32, 40, 48, 64` (claves `0, 0.5, 1, 1.5, 2, 3, 4, 5, 6, 8, 10, 12, 16`).
Relleno de fila de lista 12×8, de burbuja 12×8 (texto) y 4 (media), de diálogo 24, separación de grupos de mensajes 12,
dentro de un grupo 2.

**Radios**: `xs 4` (chips de código), `sm 8` (botones pequeños, campos), `md 12` (botones, menús, tarjetas de media),
`lg 18` (burbujas, compositor), `xl 24` (diálogos), `pill 999` (badges, separador de fecha).
**Avatar**: cuadrado redondeado con radio = 0.32 × tamaño (no círculo — rasgo de marca).
**Burbuja**: radio 18 en las cuatro esquinas, salvo la esquina del lado del remitente que «une» con la burbuja
vecina del mismo grupo, que baja a 6 (iced admite radio por esquina). Sin cola.

**Bordes**: hairline 1, campo 1, anillo de foco 2 con separación 2, barra de seleccionado 3.

---

## 5. Elevación

| Nivel | Superficie | Sombra (y / blur / opacidad × `alpha.shadow`) | Uso |
| --- | --- | --- | --- |
| 0 | `bg` | — | Fondo del chat |
| 1 | `surface` | 1 / 2 / ×0.5 | Barra lateral, lista, cabecera, compositor |
| 2 | `surface-raised` | 4 / 12 / ×0.8 | Menús contextuales, popovers, toasts, burbuja flotante de fecha |
| 3 | `surface-raised` | 12 / 32 / ×1.0 | Diálogos modales |

Las sombras nunca son iguales en todo (anti-IA): solo niveles 2 y 3 llevan sombra perceptible; el nivel 1 se separa por
borde `border` de 1 px en el lado que toca al chat.

---

## 6. Movimiento

Principios (Emil Kowalski): **120–220 ms**, muelles en vez de curvas fijas, animar **solo posición, escala y opacidad**,
animaciones interrumpibles (un muelle nuevo hereda posición y velocidad), nada se anima al cargar la app por primera vez.

### Muelles (masa 1; tiempos de asentamiento al 1 % calculados por integración numérica)
| Token | Rigidez | Amortiguación | Asentamiento | Rebase | Uso |
| --- | --- | --- | --- | --- | --- |
| `rapido` | 1840 | 77 | 119 ms | 0.2 % | Hover, pulsación, toggles, checks |
| `estandar` | 720 | 47 | 179 ms | 0.3 % | Cambio de chat, menús, entrada de burbuja, panel lateral |
| `suave` | 600 | 45 | 221 ms | 0.1 % | Diálogos, toasts, panel de información |
| `rebote` | 900 | 43 | 220 ms | 3.9 % | **Solo** botón enviar, reacción añadida, badge que aparece |

Respaldo con curva (si un widget no admite muelle): `rapido` 120 ms `(0.2,0,0,1)`, `estandar` 180 ms `(0.2,0.8,0.2,1)`,
`suave` 220 ms `(0.3,0,0,1)`, `salida` 140 ms `(0.4,0,1,1)` — las salidas siempre más cortas que las entradas.

Excepciones documentadas (no son respuesta a una acción): desvanecido del resaltado de mensaje 900 ms,
puntos de «escribiendo» periodo 1200 ms, pulso de esqueleto 1400 ms.

Coreografías clave:
- **Mensaje enviado**: la burbuja entra desde y+8 y opacidad 0 → 1 con `estandar`; el botón enviar hace 0.92 → 1 con `rebote`.
- **Mensaje recibido**: solo opacidad 0 → 1 en 120 ms (sin desplazamiento, para no mover lo que el usuario lee).
- **Cambio de chat**: el contenido nuevo hace opacidad + x 12 → 0 con `estandar`; la cabecera no se mueve.
- **Menú contextual**: escala 0.96 → 1 desde el punto de clic + opacidad, `estandar`; salida 140 ms.
- **Diálogo**: scrim opacidad con `suave`; panel escala 0.97 → 1 + opacidad; salida 140 ms.

**Movimiento reducido** (preferencia persistida, valor inicial leído del escritorio: `enable-animations` de GNOME o el
factor de duración de KDE cuando estén disponibles): duración 0 en todo salvo opacidades ≤ 100 ms; los puntos de
«escribiendo» pasan a texto estático.

Implementación orientativa en iced: integrador de muelle propio por propiedad animada, alimentado por suscripción a
frames de ventana solo mientras haya animaciones activas (CPU en reposo = 0).

---

## 7. Disposición

- Tres zonas: **raíl de carpetas** 72 px · **lista de chats** 340 px (redimensionable 280–440, persistido) · **conversación**
  fluida (columna de mensajes centrada, máx. 820 px) · **panel de info** opcional 360 px.
- Puntos de corte: < 720 px una sola columna con navegación; 720–1179 lista + chat; ≥ 1180 se permite el panel de info fijo.
- Cabecera 56 px. Compositor mínimo 52 px, crece hasta 10 líneas y luego hace scroll.
- Burbuja: máx. 560 px o 72 % del ancho de la columna, lo menor.
- Densidad persistida: **cómoda** (fila 68 px, avatar 44) o **compacta** (fila 52 px, avatar 36).
- Iconos **Phosphor** (MIT) peso *regular*, *fill* para el estado activo; 16 / 20 / 24 px. Área de clic mínima 32 px.

### Fondo del chat (innovative)
Patrón generativo de **curvas de nivel**: ruido de baja frecuencia (escala 0.004) muestreado en rejilla, isolíneas cada 28 px
de valor, trazo 1 px en `pattern-line`. **Semilla = id del chat**, así cada conversación tiene su propio «mapa» reconocible
sin usar imágenes. Se dibuja en `canvas` con caché y solo se regenera al redimensionar o cambiar de tema. Opción en
ajustes: patrón / liso. Es un asset propio, no un fondo de Telegram.

---

## 8. Componentes

Formato: variantes · medidas · estados · tokens · accesibilidad/teclado. Todo estado no listado = `default`.
Foco visible en **todos** los interactivos: anillo 2 px `focus-ring`, separación 2 px (dentro de filas, anillo interior).

### 8.1 Barra lateral (raíl de carpetas)
- **Contenido**: menú principal arriba, carpetas (Todos, No leídos, Personales, Grupos, Canales, personalizadas) como
  icono 24 + etiqueta `caption`, ajustes abajo.
- **Medidas**: 72 px de ancho, celda 64 px de alto, fondo `surface`, borde derecho `border`.
- **Estados**: default (icono `text-muted`), hover (`surface-hover`, `rapido`), activo (icono fill `accent-text`, etiqueta `text`,
  píldora 4×24 `accent` en el borde izquierdo que se desliza entre carpetas con `estandar`), foco, con no leídos
  (badge `accent`/`on-accent` arriba a la derecha; si la carpeta está silenciada, badge `border-strong`/`text`),
  arrastrando para reordenar (elevación 2, el resto se aparta con `estandar`).
- **Teclado**: `Ctrl+1…9` saltan a carpeta; flechas arriba/abajo dentro del raíl.

### 8.2 Lista de chats
- **Fila**: avatar (cuadrado redondeado) · nombre `ui-strong` · vista previa `meta` `text-muted` (1 línea, elipsis) ·
  hora `caption` `text-muted` · badge / iconos de estado a la derecha. 68 px cómoda, 52 px compacta.
- **Cabecera de lista**: campo de búsqueda (`surface-sunken`, radio `sm`, icono lupa, `Ctrl+K` lo enfoca).
- **Estados de fila**:
  - default: fondo `surface`.
  - hover: `surface-hover` (`rapido`).
  - seleccionado: `surface-selected` + barra 3 px `accent` en el borde izquierdo; nombre en `text`. (Distinto del relleno
    sólido de Telegram.)
  - foco visible: anillo interior 2 px `focus-ring`.
  - no leído: nombre 600, badge pill `accent`/`on-accent` `caption` (mín. 20 px, «99+» por encima de 99).
  - silenciado: badge `border-strong`/`text` + icono campana tachada 16 `text-muted`.
  - mención: badge «@» separado, `accent`.
  - fijado: icono chincheta 16 `text-muted` donde iría el badge si no hay no leídos.
  - escribiendo: la vista previa se sustituye por «escribiendo…» en `accent-text` con tres puntos (excepción 1200 ms).
  - borrador: prefijo «Borrador:» en `danger`, resto de la vista previa en `text-muted`.
  - último mensaje propio: icono de check (enviado / leído en `accent-text`), reloj si pendiente, `danger` si falló.
  - en línea: punto 10 px `success` con aro 2 px del color de fondo de la fila, esquina inferior derecha del avatar.
  - arrastrando (reordenar fijados): elevación 2.
  - cargando: 8 filas esqueleto (bloques `surface-hover`, pulso de opacidad 0.6 ↔ 1, 1400 ms; sin brillo deslizante).
  - vacío: ilustración propia simple + texto `display` corto + botón «Nuevo chat» (copia propia, ver ux-copy).
  - sin resultados de búsqueda: texto `ui` `text-muted` + sugerencia de buscar en mensajes.
- **Avatar sin foto**: fondo `name-n` del tema oscuro (ambos temas), iniciales `#21110a` 600 (contraste 9:1).
- **Teclado**: flechas recorren, `Enter` abre, `Alt+↑/↓` chat anterior/siguiente con no leídos, menú contextual con
  `Shift+F10` o tecla Menú (fijar, silenciar, marcar leído, archivar, eliminar → diálogo).

### 8.3 Cabecera de conversación
- **Contenido**: avatar 36 · nombre `title` · estado `meta` (en línea `accent-text`, «visto hace…» `text-muted`,
  «escribiendo…», n.º de miembros) · acciones icono 20: buscar, información, menú.
- **Medidas**: 56 px, fondo `surface`, borde inferior `border`, botones icono 36×36 radio `sm`.
- **Estados**: normal; hover/press de acciones (`surface-hover`, capa `pressed`); **modo selección** (sustituye contenido por
  «N seleccionados» + reenviar / copiar / eliminar, transición de opacidad 120 ms); **modo búsqueda** (campo en línea con
  contador «3 de 12» y flechas); **conexión degradada** («Conectando…» o «Actualizando…» en `warning` en lugar del estado,
  con spinner 16); chat silenciado (icono tras el nombre).
- **Teclado**: `Ctrl+F` busca en el chat, `Esc` sale de selección/búsqueda.

### 8.4 Burbujas de mensaje (innovative)
- **Propia** (derecha): degradado vertical `bubble-out` → `bubble-out-end`, texto `on-bubble-out`, meta `bubble-out-meta`.
- **Ajena** (izquierda): `bubble-in` plano, texto `on-bubble-in`, meta `bubble-in-meta`; en grupos, nombre del
  remitente `title`-13/600 en `name-n` en la primera burbuja del grupo y avatar 28 alineado a la última.
- **Geometría**: radio 18; la esquina del lado del remitente baja a 6 en las uniones de un grupo
  (primera: abajo; intermedias: arriba y abajo; última: arriba). Agrupación: mismo remitente y < 5 min.
  Separación 2 px dentro del grupo, 12 px entre grupos. Relleno 12×8; la hora y los checks flotan abajo a la
  derecha dentro de la última línea si cabe, si no en línea propia.
- **Contenido**: texto con enlaces (`accent-text` en ajena, `on-bubble-out` subrayado en propia), `code` en chip
  `xs` con fondo `pressed`, bloque de código `surface-sunken` radio `md`; cita de respuesta = bloque con barra
  izquierda 3 px `name-n` + nombre + 1 línea, radio `sm`; media con radio `md` interior y pie de foto; reenviado =
  línea «Reenviado de …» `meta`; solo emoji (1–3) = sin burbuja, `emoji-only`; reacciones = píldoras bajo la burbuja
  (`surface-raised`, borde `border`, la propia con borde `accent`).
- **Estados**:
  - enviando: icono reloj 14 en meta; opacidad 1 (no se atenúa el texto).
  - enviado: un check; entregado/leído: doble check, leído en `tick-read`.
  - fallido: icono alerta `danger` fuera de la burbuja + acción «Reintentar» (botón ghost), la burbuja no cambia de color.
  - editado: «editado» `caption` antes de la hora.
  - hover: aparecen tras 150 ms las acciones rápidas (reaccionar, responder, más) en una barra flotante de elevación 2
    junto a la burbuja; entrada `rapido`.
  - foco (navegación por teclado entre mensajes con `Alt+↑/↓`): anillo 2 px `focus-ring` en la burbuja.
  - seleccionado (multiselección): capa `alpha.selection` en todo el ancho de la fila + casilla a la izquierda.
  - resaltado (salto a respuesta o resultado): capa `alpha.highlight` que se desvanece en 900 ms.
  - menú contextual abierto: la burbuja recibe borde 1 px `border-strong`.
  - mensaje eliminado para todos: texto `meta` en cursiva «Mensaje eliminado», sin acciones.
- **Elementos de servicio**: separador de fecha = píldora `caption` `surface-raised` con elevación 2, pegajosa arriba al
  hacer scroll; mensajes de servicio (alguien se unió) = píldora centrada `meta` `text-muted` sobre `surface`.
  Marca «Mensajes no leídos» = línea `accent` 1 px con etiqueta `caption` `accent-text`.
- **Botón «ir al final»**: círculo 40 elevación 2 abajo a la derecha, con contador de no leídos; aparece con `estandar`.
- **Accesibilidad**: cada mensaje expone autor, hora y estado como texto («Tú, 14:02, leído»); el color nunca es el único
  indicador (checks se cuentan, fallo lleva icono).

### 8.5 Compositor
- **Estructura**: botón adjuntar (icono 20) · campo multilínea `body` · emoji · botón enviar. Fondo de la barra `surface`
  con borde superior `border`; el campo es `surface-sunken` radio `lg`, borde 1 `border` → `border-input` en foco +
  anillo `focus-ring`. Placeholder `text-placeholder`.
- **Estados**:
  - vacío: botón enviar oculto, en su lugar micrófono (cuando exista nota de voz) o enviar deshabilitado
    (`border-strong`, sin anillo).
  - con texto: enviar visible, círculo 36 `accent` con icono `on-accent`, entrada con `rebote` 0.92 → 1.
  - foco: borde `border-input` + anillo 2 px.
  - multilínea: crece por líneas sin animar el alto del texto; a partir de 10 líneas, scroll interno.
  - respondiendo / editando: franja encima del campo con barra 3 px `accent`, icono, nombre y 1 línea citada, botón cerrar;
    entra con opacidad + y 8 → 0 `estandar`. Editando cambia el icono de enviar por check.
  - adjuntos pendientes: fila de miniaturas 56 radio `md` con botón quitar.
  - deshabilitado (canal sin permiso de escritura, chat bloqueado): campo sustituido por botón ancho ghost
    («Silenciar» / «Desbloquear») o texto `text-muted`.
  - error (mensaje demasiado largo, fallo al subir): contador o texto `caption` `danger` bajo el campo.
  - sin conexión: se permite escribir y enviar; los mensajes quedan en estado «enviando».
- **Teclado**: `Enter` envía, `Shift+Enter` salto de línea (configurable y persistido), `↑` en campo vacío edita el último
  mensaje propio, `Esc` cancela respuesta/edición, `Ctrl+V` pega imágenes.

### 8.6 Diálogos
- **Tamaños**: sm 360, md 480, lg 640 de ancho; radio `xl`; relleno 24; elevación 3 sobre `alpha.scrim`.
- **Estructura**: título `title`, cuerpo `ui` `text-muted`, acciones a la derecha (secundaria ghost, primaria `accent`;
  destructiva = botón `danger-solid`/`on-danger` y nunca es la acción por defecto de `Enter`).
- **Estados**: entrando/saliendo (§6), cargando (botón primario con spinner, conserva etiqueta para lectores de pantalla,
  resto deshabilitado), error (mensaje `danger` sobre las acciones), deshabilitado.
- **Teclado**: foco atrapado dentro, primer control enfocado al abrir, `Esc` cierra (salvo operaciones en curso),
  el foco vuelve al elemento que lo abrió.

### 8.7 Toasts
- **Posición**: abajo al centro de la columna de conversación, 16 px sobre el compositor; máximo 3 apilados (el más nuevo
  abajo), ancho 280–420.
- **Estilo**: `surface-raised`, elevación 2, radio `md`, texto `ui` `text`, icono 20 de variante a la izquierda
  (info `accent-text`, éxito `success`, error `danger`), acción opcional como botón de texto `accent-text` («Deshacer»).
- **Comportamiento**: entra con `suave` (y 12 → 0 + opacidad), sale 140 ms; se cierra a los 4 s (6 s si tiene acción);
  el temporizador se pausa con hover o foco; se puede descartar con `Esc` o deslizando.
- **Accesibilidad**: anuncio *polite* (errores *assertive*) cuando iced exponga el árbol de accesibilidad; nunca contienen
  la única vía para una acción importante.

### 8.8 Primitivas compartidas
| Componente | Variantes / tamaños | Estados | Tokens |
| --- | --- | --- | --- |
| Botón | primary, secondary, ghost, danger · sm 32, md 36, lg 44 | default, hover (`accent-hover`), active (`accent-pressed`, escala 0.98 `rapido`), focus-visible, disabled (opacidad 0.45, sin hover), loading | radio `md`, `ui-strong` |
| Botón icono | 32 / 36 / 40 | igual que botón; tooltip a los 500 ms | radio `sm`, icono 20 |
| Campo de texto | sm 32, md 40 | default, hover (`border-strong`), focus, error (borde `danger` + texto), disabled | `surface-sunken`, borde `border-input` |
| Interruptor | 36×20 | off (`border-strong`), on (`accent`, perilla desliza con `rapido`), focus, disabled | — |
| Casilla | 18 | off, on, indeterminada, focus, disabled | radio `xs` |
| Menú contextual | ítem 32 | hover/teclado (`surface-hover`), destructivo (`danger`), deshabilitado, con atajo (`caption` `text-muted`) | `surface-raised`, elevación 2, radio `md` |
| Tooltip | — | aparece 500 ms, desaparece 100 ms | `text` sobre `surface-raised`, `caption` |
| Badge | contador, punto, @ | normal, silenciado | pill |
| Spinner | 16 / 24 | — | arco `accent`, 900 ms por vuelta (excepción de bucle) |

### 8.9 Componentes del recon y de ADR-0004
**Burbuja de medios** (foto, vídeo, archivo, nota de voz): media con radio `md` dentro de la burbuja (relleno 4).
Estados: miniatura borrosa propia (bloque `surface-hover`) mientras carga · descargando/subiendo = anillo de progreso 48
(`accent` sobre círculo `alpha.scrim`) con botón cancelar · completa · error = icono reintentar `danger` en el centro.
Archivo = fila icono 40 `surface-sunken` + nombre `ui-strong` + tamaño `meta`. Barra lineal de progreso 3 px `accent`
sobre `border` (determinada / indeterminada con desplazamiento de 1400 ms).

**Nota de voz**: botón reproducir círculo 40 `accent` / `on-accent`; forma de onda en barras de 3 px con separación 2;
parte reproducida `accent` en burbuja ajena y `on-bubble-out` en propia, parte pendiente `bubble-in-meta` /
`bubble-out-meta` (≥ 3:1 en ambos temas); duración `meta`. Compositor grabando: punto `danger` 10 px con pulso de
opacidad, contador `meta` en `text`, «Esc cancela» `caption` `text-muted`, enviar visible.

**Sticker** (ADR-0004): sin burbuja; máx. 200×200; hora y checks en una píldora `surface-raised` al 85 % de opacidad en la
esquina inferior. Animado (Lottie/WebM) solo mientras está visible; con movimiento reducido se muestra el primer frame.
Pack propio de muestra para el mock; nunca assets del original. Panel de stickers = rejilla 72 px con pestañas de pack
(iconos 28), hover `surface-hover`, foco anillo.

**Chat secreto** (ADR-0004), señalado sin depender solo del color:
- Fila de lista: icono escudo-candado 16 `success` antes del nombre; el nombre no cambia de color.
- Cabecera: el mismo icono + estado «cifrado de extremo a extremo» en `meta`.
- Fondo: el patrón de curvas de nivel pasa a **trama de isolíneas al doble de densidad** (espaciado 14) para que el modo se
  reconozca de un vistazo.
- Mensaje con autodestrucción: anillo de cuenta atrás 14 px en la zona meta (`bubble-*-meta`); al expirar, la burbuja
  se desvanece 180 ms y los mensajes inferiores se desplazan (translate) con `estandar` para cerrar el hueco.
- **Huella de clave**: cuadrícula 4×4 de celdas de 20 px con radio `xs`, color de cada celda = `name-(n)` según 3 bits del
  hash de la clave, más 4 grupos de 5 dígitos en `code`. Diseño propio (el original usa otra representación); se muestra en
  la info del chat y en un diálogo md de verificación.
- Estados del chat: esperando a que el otro acepte (compositor deshabilitado con texto `text-muted`), activo, cerrado/rechazado
  (servicio `danger`).

**Selector de reacciones / menú de mensaje** (S14–S15): fila de 6 reacciones de 32 px sobre el menú contextual
(`surface-raised`, elevación 2, radio `lg`); hover escala 1.15 con `rapido`; la reacción elegida vuela a la burbuja con
`rebote`. Ítems del menú según §8.8, «Eliminar» en `danger`.

**Diálogos de reenviar, borrar, envío de archivos** (S16, S17, S21): diálogo md/lg de §8.6. Reenviar = buscador + lista de
chats con casillas + campo de comentario. Borrar = casilla «eliminar también para …» + botón `danger-solid`.

**Inicio de sesión** (S01–S06, controlled): tarjeta de 400 px centrada sobre `bg` con el patrón de curvas de nivel; título
`display`, campos md 40, botón primario lg ancho completo. Estados: vacío, inválido (borde y texto `danger`), espera por
límite de intentos (texto `warning` con cuenta atrás), error de red (toast error). **QR**: siempre módulos `#211c17` sobre
`#ffffff` con zona tranquila de 16 px y radio `md`, en ambos temas, para que lo lea cualquier cámara; se refresca con
fundido de 120 ms.

**Lista virtual** (chats, mensajes): fila «cargando más» = spinner 16 centrado; fin del historial = píldora de servicio.

---

## 9. Anti-«aspecto IA» (prohibido)
1. Degradados violeta→azul o arcoíris; texto con degradado.
2. Glassmorphism por defecto, brillos, «sparkles» ✨ como icono de nada.
3. Inter + grises slate azulados sin decisión; negro puro `#000` como fondo oscuro.
4. Misma sombra suave en todas las tarjetas; tarjetas dentro de tarjetas.
5. Todo con radio 16+ (los radios aquí dependen del tamaño: 4/8/12/18/24).
6. Emojis usados como iconos de interfaz.
7. `ease-in-out 300ms` en todo; animaciones de entrada al arrancar la app; esqueletos con brillo deslizante.
8. Gris sobre gris de bajo contraste para «elegancia» (todos los textos ≥ 4.5:1).
9. Copias genéricas («¡Bienvenido a tu experiencia de mensajería!»); el texto lo define `design:ux-copy` en la fase de marca.
10. Centrar todo; las pantallas de app se alinean a la izquierda salvo vacíos y diálogos.

## 10. Preferencias de usuario persistidas
Guardadas en la configuración local (XDG config dir): tema (`system` | `dark` | `light`; `system` sigue `color-scheme`
del portal XDG), densidad (cómoda | compacta), escala de texto, fondo del chat (patrón | liso), movimiento reducido,
`Enter` envía, ancho de la lista. Los cambios se aplican en vivo sin reiniciar.

## 11. Qué falta / siguiente fase
- replica-brand: nombre, logo e ilustraciones propias; puede mover el tono de `accent` pero no los roles.
- design:ux-copy: textos de vacíos, errores y diálogos.
- Construir las primitivas de §8.8 aisladas (pantalla «/design» interna) antes de cualquier pantalla y capturarla.
- Componentes cruzados con `replica/recon.md` (S01–S24 y tabla de componentes) y con ADR-0004 (stickers, chats secretos).
  Pantallas de ajustes y editor de carpetas reutilizan las primitivas de §8.8; sin spec propia hasta la fase build.

## Registro de cambios de este documento
| Fecha | Versión | Cambio |
| --- | --- | --- |
| 2026-11-06 | 1.0.0 | Creación: dirección Brasa, paletas OKLCH, escalas, muelles, componentes. |
