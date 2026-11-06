# ADR 0002 — Dirección visual «Brasa»

- **Estado**: aceptada
- **Fecha**: 2026-11-06
- **Autor**: agente de dirección de diseño (skills `replica-skill:replica-design`, `anthropic-skills:theme-factory`)
- **Detalle**: [design/system.md](../design/system.md) · tokens: [design/tokens.json](../design/tokens.json)

## Contexto
teleClon reconstruye los patrones de UX de un cliente de mensajería de escritorio, pero debe tener marca propia
(regla clean-room del índice) y verse moderno sin caer en el «aspecto IA». La UI se hace en Rust con iced, que ofrece
fondos sólidos y degradados lineales, radios por esquina, sombras de contenedor, SVG y `canvas`, pero no desenfoque de fondo.
Varios agentes trabajarán sobre la UI, así que las decisiones tienen que quedar persistidas y ser legibles por máquina.

## Decisión
1. Dirección **Brasa**: neutros de grafito cálido (OKLCH H 65–80, C ≤ 0.012) y un único acento persimón (H 40–45).
   Temas oscuro y claro; por defecto sigue el `color-scheme` del sistema.
2. **OKLCH como fuente de verdad**, con hex sRGB derivados por reducción de croma. Contraste WCAG AA verificado con
   `contrast.py` (144 pares, 0 fallos). Ningún cambio de paleta se acepta sin volver a pasar el script.
3. **Controlled** para el chrome (solo tokens, sin degradados) e **innovative** solo para burbujas (degradado vertical
   sutil, sin cola, esquina de unión de 6 px) y fondo del chat (curvas de nivel generativas con semilla = id del chat).
4. Rasgos de marca que nos separan del original: avatares en cuadrado redondeado (radio 0.32 × tamaño), burbujas sin cola,
   fila seleccionada con tinte y barra en vez de relleno sólido, fondo de isolíneas en vez de garabatos, acento cálido.
5. Tipografía libre empaquetada: Atkinson Hyperlegible Next (UI/mensajes), Bricolage Grotesque (solo display),
   JetBrains Mono (código), Noto Color Emoji del sistema. Iconos Phosphor (MIT).
6. Movimiento con muelles de 119–221 ms (rigidez/amortiguación calculadas), solo posición, escala y opacidad; movimiento
   reducido como preferencia persistida.
7. `docs/design/system.md` es el registro de decisiones persistidas: se lee antes de tocar UI y se actualiza en el mismo cambio.

## Alternativas consideradas
- **Vidrio glacial** (glassmorphism azul-cian): descartada por cercanía al azul del original, cliché visual y falta de
  desenfoque de fondo en iced (coste de GPU si se simula).
- **Neobrutalismo** (bordes gruesos, sombras duras, colores planos): descartada por fatiga en uso prolongado, ruido en
  listas densas y envejecimiento rápido.
- Otros tonos de acento: azul/cian (Telegram, Signal), verde/jade (WhatsApp), violeta (Discord, Viber) descartados por
  asociación con la competencia.

## Consecuencias
- Positivas: identidad reconocible y distinta; contraste garantizado por proceso; tokens que un crate Rust puede
  deserializar (`serde`) o copiar como constantes; sin dependencias de efectos que iced no soporta.
- Negativas / riesgos: el acento naranja está cerca en tono de `danger`; se mitiga con separación de H y L y con icono
  obligatorio en estados de error. Atkinson Hyperlegible Next y Bricolage no están en todos los repos: se empaquetan en el
  binario (OFL lo permite; incluir licencia). El fondo generativo necesita caché para no costar CPU.
- replica-brand puede ajustar el **valor** de `accent` (y derivados) pero no los roles; cualquier cambio de dirección exige ADR nuevo.
