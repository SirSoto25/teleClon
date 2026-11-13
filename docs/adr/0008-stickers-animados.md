# ADR-0008: Stickers animados (TGS con rlottie; WebM estático)

- **Fecha:** 2026-11-10 · **Origen:** primera prueba real: los stickers animados se veían estáticos (solo la miniatura o el primer fotograma).
- **Estado:** aceptado.

## Decisión

### TGS (Lottie con gzip): animado con rlottie, por defecto solo en Linux
- **Render:** `rlottie` 0.5.4 / `rlottie-sys` 0.2.12 (Samsung rlottie v0.2). `tc_stickers::anim::Tgs` guarda el JSON ya descomprimido y validado (tope de 4 MiB). Los fotogramas se pintan por lotes a un lado fijo de **256 px** (`ANIM_PX`): el que toca y 6 más por delante. Todo va en `spawn_blocking`. `rlottie::Animation` no es `Send`, así que cada lote crea la animación desde el JSON y la suelta. rlottie reutiliza el modelo ya analizado gracias a su caché por clave (`tg<id del documento>`).
- **Tiempo:** hay un único reloj en la UI, `iced::time::every(33 ms)`, de **30 fps como mucho**. El reloj del pack propio también usa ahora ese tic, en lugar de `window::frames`. Solo se suscribe si hay algo animado a la vista, la ventana tiene el foco y el movimiento no está reducido. Con una fuente de 60 fps se muestra un fotograma de cada dos (`frame_step`). El conjunto de fotogramas de cada vuelta es siempre el mismo, así que la caché acierta.
- **Caché:** `ByteLru` (LRU acotada en bytes) de `Handle` de iced, con tope de 48 MiB, unos 190 fotogramas de 256 px. Hay otra LRU de 16 MiB para los TGS preparados. La GPU no vuelve a subir un `Handle` que ya tiene.
- **Política (`Playback`):**
  - Con `TC_REDUCED_MOTION` o el conmutador de la app se dibuja la imagen estática: el primer fotograma en la burbuja y la miniatura en el selector.
  - Sin foco (`window::Event::Unfocused`) la animación se congela en el último fotograma.
  - Lo que queda fuera de la vista no se pide y se olvida su fotograma actual.
  - En las burbujas se anima la ventana visible en bucle.
  - En el selector solo se anima el sticker bajo el ratón o el del cursor de teclado. Animar la rejilla entera (~40 TGS) costaría ~10 MiB por sticker y mucha CPU.
- **Por qué solo en Linux:** `rlottie-sys` hace tres cosas al compilar: clona rlottie con `git` desde GitHub (necesita red), lo compila con `cmake` y C++, y genera los enlaces con bindgen (necesita libclang). Comprobado aquí:
  - **Windows:** `cargo check --target x86_64-pc-windows-msvc -p tc-stickers --features lottie` falla en el propio crate `rlottie`, que usa `std::os::unix::ffi::OsStrExt`. Además, su build.rs enlaza `stdc++`, que no existe con MSVC.
  - **macOS:** `cargo check --target aarch64-apple-darwin` pasa, pero no enlaza. El build.rs fija `-lstdc++`, y los SDK actuales de Xcode solo traen libc++, así que es muy probable que falle al enlazar. No hay Mac para probarlo.
  - **Linux:** se activa siempre con una dependencia por destino en `tc-ui` (`[target.'cfg(target_os = "linux")'.dependencies] tc-stickers = { features = ["lottie"] }`). Los trabajos Linux del CI y de release instalan `cmake g++ git libclang-dev`. La feature `tc-ui/lottie` queda para probar a mano en otros SO.
- **Para Windows y macOS hace falta una de dos cosas:**
  - Parchear o bifurcar `rlottie`/`rlottie-sys`: rutas sin `OsStrExt`, `c++` en macOS, sin `stdc++` en MSVC y fuentes empaquetadas en lugar de `git clone`.
  - Un renderizador Lottie en Rust puro que cubra lo que usan los TGS de Telegram (formas, trazos, degradados, máscaras, recortes).

### WebM (vídeo VP9 con alfa): queda estático (miniatura)
- **Qué haría falta:**
  - Demultiplexar Matroska/WebM, incluido el **alfa**. En los stickers de vídeo el alfa va como un segundo flujo VP9 en `BlockGroup/BlockAdditions/BlockMore/BlockAdditional` (BlockAddID 1).
  - Decodificar los dos flujos VP9.
  - Componer RGBA: YUV420 → RGB, y el plano Y del flujo alfa como canal A.
- **Opciones evaluadas (crates.io, 2026-11-10):**
  - **libvpx** (`env-libvpx-sys` 5.1.3, `vpx-rs`, `shiguredo_libvpx`): es el decodificador de referencia y fiable. Pero necesita libvpx del sistema, vcpkg o brew, o compilarla empaquetada; en x86 eso exige nasm/yasm. Rompe el «sin dependencias del sistema» en Windows y macOS.
  - **VP9 en Rust puro** (`rusty_vp9` 0.1.3, que dice ser bit-exacto con los 315 vectores de conformidad de libvpx; `vp9dec` 0.1.1; `filmcraft-vp9` 0.9.0): son todos muy recientes, de versión 0.x, con un solo autor y sin historial de uso. No hay garantías de robustez ante entradas hostiles; un sticker es contenido remoto no confiable.
  - **Contenedor:** `matroska-demuxer` 0.8.1 expone `alpha_mode`, pero su `Frame` no da los `BlockAdditional`, así que el alfa se perdería. Habría que escribir un lector EBML propio, pequeño, para `SimpleBlock` y `BlockGroup`.
- **Decisión:** no hay una vía que sea fiable y portable a la vez. WebM sigue con su miniatura estática, como antes.
- **Cómo se haría:**
  1. Lector EBML propio en `tc-stickers::webm`, con topes de tamaño y de número de bloques, que saque el color y el alfa por fotograma.
  2. `rusty_vp9` (u otro) tras una feature `webm`, primero con fuzzing (`cargo fuzz`) y con una batería de stickers reales.
  3. Composición YUV+alfa → RGBA a 256 px y los mismos `ByteLru`, reloj y política que los TGS (`Playback`). Los fotogramas por tiempo saldrían de las marcas de tiempo del bloque, no de `fr`.

## Consecuencias
- Los paquetes de Linux animan los TGS. Los de Windows y macOS siguen como antes, con la imagen estática.
- Compilar en Linux necesita red la primera vez (el `git clone` de rlottie-sys), además de cmake, g++ y libclang.
- Nuevos módulos:
  - `tc_stickers::{anim, lru}`.
  - En `tc-ui`: `sticker_anim.rs` (estado y política) y `app/sticker_anims.rs` (tareas).
  - Mensaje `Msg::StickerAnim`.
- No verificado: rendimiento con muchos TGS reales a la vista, fidelidad de rlottie con los TGS de Telegram y comportamiento en Windows y macOS reales.
