# ADR-0009: Teclado de emojis

- **Fecha:** 2026-11-10 · **Estado:** aceptado (probado con el mock y capturas en Linux; Windows/macOS solo
  compilados).
- **Contexto:** con Telegram real el botón del compositor solo abría los stickers. Hacía falta un teclado de
  emojis completo (categorías, búsqueda, tono de piel, recientes) dentro de un panel con pestañas
  «Emojis | GIFs | Stickers», sin red ni datos externos y dibujado en color en Linux, Windows y macOS.

## Decisiones
1. **Catálogo: crate `emojis` 0.9.1** (MIT/Apache + Unicode-3.0, sin dependencias): todos los emojis en orden
   CLDR con grupo, nombre inglés, atajos de GitHub y variantes de tono. Se ofrecen los de **Emoji ≤ 15.0**
   (los posteriores salen como «tofu» con fuentes de 2023 en Windows/Linux).
2. **Búsqueda en español con tabla propia** (`crates/tc-ui/src/emoji_es.rs`, ~600 emojis frecuentes escritos a
   mano, sin tildes) + palabras de grupo («bandera», «animal», «comida»…) + nombre inglés y atajos del crate.
   Se descartó empaquetar las anotaciones CLDR en español (más datos y una descarga que no se hizo); queda como
   mejora si la tabla se queda corta. La consulta se pliega (tildes, mayúsculas) y cada palabra debe casar.
3. **Render: texto con la fuente de color del sistema.** iced 0.14 dibuja con cosmic-text + swash, que
   rasteriza glifos en color (CBDT/CBLC de Noto Color Emoji, sbix de Apple Color Emoji, COLR de Segoe UI Emoji)
   y recurre a la fuente de emoji del sistema. El panel pide la familia «Noto Color Emoji» (`ts::EMOJI_ONLY`)
   y se ve en color: comprobado en Linux con capturas sin ventana (`tests_panel_shots.rs`, `iced_test`, la
   prueba cuenta píxeles saturados). No se empaqueta un atlas de imágenes. **Sin verificar** en Windows/macOS.
   **Limitación encontrada:** en texto mezclado (compositor y cuerpo de los mensajes, con la fuente de la UI)
   cosmic-text 0.15 recorre su lista de respaldo de Linux, que pone «DejaVu Sans» antes que «Noto Color
   Emoji»; DejaVu trae algunos emojis (😀, 😂, ☺…) en monocromo y salen así, mientras que los que no tiene (🔥, 👍)
   salen en color. iced no deja cambiar ese orden. Alternativas: (a) en las burbujas, partir el texto en
   `rich_text` con los tramos de emoji en la familia de color (posible, pendiente; el compositor `text_editor`
   no admite fuentes por tramo); (b) subir a un cosmic-text que priorice la fuente de emoji para caracteres
   `Emoji_Presentation`; (c) atlas PNG de Twemoji (CC-BY 4.0) o Noto (OFL) si también falla en Windows/macOS.
4. **Inserción en el cursor:** `text_editor::Action::Edit(Edit::Paste(..))` sobre el `Content` del compositor
   (respeta la posición del cursor y la selección), devolviendo el foco al compositor; el panel sigue abierto.
5. **Recientes y frecuentes persistidos:** `StickersApi::emoji_usage`/`record_emoji_use` (métodos con valor por
   defecto en tc-core). En la app los implementa `MediaCacheLayer` sobre la base de la cuenta (migración 0007,
   tabla `emoji_usage`, 200 filas como mucho); el mock los guarda en memoria. El tono de piel elegido vive en la
   sesión (no se persiste).
6. **Panel:** `App::picker.open` sigue siendo «panel abierto»; `App::panel.tab` guarda la pestaña (Ctrl+E reabre
   en la última). Alt+1/2/3 cambia de pestaña; Esc y un clic en la conversación lo cierran; enviar un sticker o
   un GIF lo cierra. En Emojis las flechas mueven el cursor del texto (no la rejilla).

## Consecuencias
- +1 dependencia pequeña (`emojis`, datos estáticos ≈ 300 KiB en el binario).
- La búsqueda en español cubre bien lo frecuente; emojis raros solo se encuentran por inglés o por grupo.
