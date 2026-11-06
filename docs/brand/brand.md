# Marca propia — teleClon

Fecha: 2026-11-06 · Skill: `replica-skill:replica-brand` (sin consultas externas) · Estado: **propuesta, pendiente de elección del usuario**.

> **Aviso importante.** Esta sesión no ha consultado registros de marcas, dominios ni redes. Ninguno de los nombres de abajo está verificado como libre. Los chequeos de la sección 3 los tiene que hacer el usuario (no son asesoría legal). `teleClon` es solo el nombre de trabajo del repo y debe desaparecer de la cara pública: contiene «tele» y «clon».

## 1. Criterios de nombre

1. Nada de «Telegram», «tele-», «gram», «tg», ni nada que suene a la marca original ni a otras mensajerías (Signal, WhatsApp, Viber, Line, Threema, Element).
2. Pronunciable en español e inglés, 2-3 sílabas, sin tildes imprescindibles para escribirlo.
3. Coherente con el sistema visual **Brasa** (ADR-0002): calidez, persimón, curvas de nivel, cuadrados redondeados.
4. Corto para el handle (`@nombre`), el binario (`nombre`) y el paquete (`com.nombre.app`, `nombre-desktop`).
5. Sin significado negativo conocido en español, inglés, portugués ni francés (chequeo rápido, no exhaustivo).
6. Registrable: preferible palabra evocadora antes que genérica (los nombres genéricos son difíciles de proteger).

## 2. Candidatos (6-8)

| # | Nombre | Idea | Pros | Riesgos a mirar |
| --- | --- | --- | --- | --- |
| 1 | **Cota** | «Cota»: altura marcada en un mapa de curvas de nivel; también «estar a cota», nivel. | Enlaza con el fondo de isolíneas; 4 letras; fácil de decir. | Palabra muy común (cota de malla, cotas bursátiles); dominios y handles probablemente ocupados. |
| 2 | **Isolínea** (`isolinea`) | La línea de igual valor en un mapa. | Evoca directamente la marca visual; único. | 4 sílabas; la tilde se pierde en dominios. |
| 3 | **Ascua** | Brasa encendida. | Conecta con Brasa y el persimón; cálido y corto. | Existe como nombre en otros ámbitos; revisar clases de software. |
| 4 | **Rescoldo** | Brasas que quedan: conversación que sigue viva. | Metáfora bonita de continuidad. | Suena a «lo que queda»; 3 sílabas con grupo «sc». |
| 5 | **Parlo** | De «parlar»/«parlotear»: charlar. | Muy corto, amistoso, internacional. | Cercano a nombres de apps existentes (revisar «Parler» y similares). |
| 6 | **Kaki** (`kaki` / «caqui») | El fruto del persimón (en inglés persimmon/kaki). | Enlaza con el acento; sonido simpático. | Es también un color y una tela; muy usado; revisar. |
| 7 | **Tertulia** | Reunión de conversación. | Significado perfecto, muy hispano. | Largo (3 sílabas, 8 letras); difícil para público no hispano. |
| 8 | **Cuadra** | Cuadrado/manzana y «cuadrar» de acuerdo. | Alude al cuadrado redondeado. | Ambiguo (caballeriza); poco distintivo. |

Preferencia provisional (criterio del agente, no vinculante): **Ascua** o **Isolínea** por distintividad y coherencia con Brasa; **Cota** si prima la brevedad. Decide el usuario tras los chequeos.

## 3. Chequeos que debe hacer el usuario

Para cada nombre finalista (máximo 3), anotar fecha y resultado:

- **Marca**: buscar en la OEPM (España) / EUIPO / USPTO / WIPO Global Brand Database, clases Nice 9 (software), 38 (telecomunicaciones/mensajería), 42 (servicios de software). Buscar también variantes fonéticas y escritura sin tilde. Si va a haber comercialización, consultar a un agente de propiedad industrial.
- **Dominio**: `.com`, `.app`, `.io`, `.es`; comprobar en un registrador que no estén ocupados ni en venta a precio absurdo; mirar si hay un sitio activo con el mismo nombre.
- **Handles**: GitHub (organización y repo), Mastodon, Bluesky, X, Instagram, YouTube; y nombre de usuario en la propia app.
- **Tiendas y paquetes**: Flathub (id de aplicación tipo `app.nombre.Client`), Snap Store, AUR, crates.io (nombre del binario), App Store y Google Play si hubiera versión móvil.
- **Búsqueda general**: buscar «nombre + chat/mensajería/app» y ver qué aparece los primeros resultados; confusión con otros productos es motivo de descarte.
- **Significados**: preguntar a hablantes nativos (es-ES, es-MX, es-AR, pt, en) si suena mal.

## 4. Voz y tono (español)

Principios: **cálida, clara, breve, sin jerga ni mascotas.** Tuteo. Español neutro con ortografía correcta. Nunca «¡Ups!» ni exclamaciones de relleno.

| Situación | Evitar | Preferir |
| --- | --- | --- |
| Error de red | «¡Ups! Algo salió mal» | «No hay conexión. Reintentaremos en unos segundos.» |
| Código incorrecto | «Código inválido» | «Ese código no coincide. Revisa los 5 dígitos.» |
| Borrar mensaje | «¿Estás seguro?» | «¿Borrar este mensaje? También lo verá {nombre} sin él.» (con casilla) |
| Chat vacío | «No hay nada aquí» | «Aún no hay mensajes. Escribe el primero.» |
| Chat secreto | «Mensajería ultrasegura» | «Cifrado de extremo a extremo. Solo en este dispositivo.» |
| Estado experimental | promesas de seguridad absolutas | «Función experimental, sin auditar. No la uses para nada crítico.» |

Reglas de redacción: frases de máximo ~14 palabras; verbo en el botón («Enviar», «Borrar», «Instalar»); sin mayúsculas sostenidas; sin emojis en la interfaz salvo los del usuario; los números con formato local; honestidad antes que marketing (no afirmar seguridad que no se haya auditado).

## 5. Brief de logo

**Concepto**: un cuadrado redondeado (la forma de los avatares de Brasa, radio ≈ 0.32 × lado) que contiene líneas de nivel concéntricas desplazadas, y en el centro un punto/«brasa» persimón que a la vez funciona como burbuja de mensaje sin cola.

- **Forma base**: cuadrado redondeado, radio 0.32 × lado, proporción 1:1. Sin cola de burbuja, sin avión ni flecha de papel (evoca la marca original).
- **Motivo interior**: 2-3 curvas cerradas irregulares tipo isolínea, grosor uniforme, que se estrechan hacia el centro; una de ellas se abre para sugerir una «boca» de diálogo.
- **Color**: persimón del acento (`accent` oscuro `#f67e42`, claro `#c64913`, ver `docs/design/tokens.json`) sobre grafito cálido (`#120f0c`) o sobre blanco cálido (`#f8f5f1`). Versión monocroma (una tinta) obligatoria. Comprobar contraste ≥ 3:1 del símbolo contra el fondo.
- **Tipografía del logotipo**: Bricolage Grotesque (display, ya empaquetada, OFL), en minúsculas, peso medio; sin ligaduras raras.
- **Entregables**: SVG de símbolo y de símbolo + logotipo (horizontal), icono de app 512 px y 256/128/64/48/32/16 (legible a 16 px: simplificar a una sola curva + punto), versión monocroma, versión para tema claro y oscuro, icono de bandeja simbólico (una tinta, sin degradado), área de respeto = 1/4 del lado.
- **Prohibido**: azul/cian como color dominante, avión o papel doblado, círculo con cola, degradados cian-azul, cualquier parecido con logos de mensajería existentes. Revisar el resultado final con una búsqueda inversa de imágenes.
- **Producción**: dibujo vectorial original (a mano o con herramienta con licencia clara); si se usa un generador de imágenes, conservar prompts y licencia del resultado en `docs/brand/`.

## 6. Palabras prohibidas y restos

La comprobación automática es `python3 tools/brand/sweep.py --root . --out docs/brand/sweep-report.md` (añadir `--strict` en CI cuando esté todo limpio). Usos técnicos permitidos: crate `tc-telegram`, nombre de protocolo/servicio en comentarios de los crates de red, URLs de documentación, y toda la carpeta `docs/` y `replica/` (clean-room). Informe actual: `docs/brand/sweep-report.md`.

## 7. Siguientes pasos

1. El usuario elige finalistas y hace los chequeos de la sección 3.
2. Con el nombre elegido: renombrar crates `tc-*` (o mantener el prefijo interno, decisión de ADR), `Cargo.toml`, título de ventana, `.desktop`, id de Flatpak.
3. Cambiar la línea de «inspirado en…» del README por una nota de atribución sin marca, si procede.
4. Repetir el barrido y dejarlo en limpio antes de `replica-launch`.
