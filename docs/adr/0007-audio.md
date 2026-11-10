# ADR-0007: Audio (notas de voz y música)

- **Fecha:** 2026-11-09 · **Estado:** aceptado (decodificado probado con ficheros generados; salida real por
  altavoces sin probar en ningún SO).
- **Contexto:** con Telegram real las notas de voz «fallaban»: se dibujaban como una caja de foto «0 × 0», la
  forma de onda, el título/intérprete y el MIME se descartaban al mapear, la música quedaba como «archivo» y no
  había reproductor. Telegram manda las notas de voz en OGG/Opus y la música en MP3, M4A/AAC, FLAC, OGG/Vorbis
  o WAV. Requisito: Linux, Windows y macOS desde la misma CI, sin dependencias del sistema más allá de lo
  inevitable, y sin bloquear la UI.

## Decisiones
1. **Crate nuevo `tc-audio`** (decodificado + reproductor). tc-core solo gana el modelo (`AudioInfo`,
   `MediaKind::{Audio, VideoNote}`) y `tc_core::waveform` (forma de onda de 5 bits, sin dependencias), para que
   tc-telegram no arrastre audio.
2. **Opus en Rust puro:** `opus-decoder` 0.1.1 (port de libopus, sin `unsafe` ni FFI, conforme a RFC 8251) con un
   demultiplexor Ogg propio (≈100 líneas: `OpusHead`, `pre_skip`, ganancia, recorte final por la posición
   granular, salto con 80 ms de pre-roll). Descartados: `audiopus_sys`/`opus` (libopus en C: cmake o libopus del
   sistema, frágil en la CI de Windows/macOS) y `symphonia-adapter-libopus` (lo mismo). symphonia 0.5/0.6 no
   decodifica Opus.
3. **Resto de formatos con symphonia 0.5.5** (MP3, AAC/M4A, FLAC, WAV/PCM, OGG/Vorbis), Rust puro, la misma
   serie que usa rodio. Se elige el decodificador por el contenido (cabecera `OggS`+`OpusHead`), con el MIME de
   Telegram solo como pista.
4. **Salida con rodio 0.23 (`default-features = false, features = ["playback"]`)** sobre cpal 0.18: WASAPI en
   Windows, CoreAudio en macOS (sin bindgen: `cargo check` cruzado pasa), ALSA en Linux. Sin la feature
   `recording` (no se toca el micrófono ni hacen falta permisos en macOS). Nuestra fuente (`Track`) decodifica a
   trozos (streaming, nada entero en memoria) y publica la posición en frames del archivo, así que la posición es
   correcta a cualquier velocidad.
5. **Hilos:** `Player` es un asa barata; las órdenes van por canal a un hilo `parlo-audio` que abre el dispositivo al
   primer uso y lo conserva. La UI lee el estado de atómicos (nunca espera). Sin dispositivo → `AudioError::NoDevice`
   en el estado y «Sin salida de audio» en la burbuja; los pánicos del decodificador se atrapan (`catch_unwind`) y se
   convierten en error o silencio del paquete, porque corren en el hilo de audio del sistema.
6. **Velocidad 1x/1.5x/2x con `Player::set_speed` de rodio:** remuestrea, así que sube el tono (sin estiramiento
   de tiempo). Aceptado por ahora; un time-stretch (WSOLA) sería un paso posterior.
7. **Un solo audio a la vez**, descarga perezosa al pulsar (documento completo por `download_media(Full)`, por
   partes de 512 KiB con eventos `Download` de progreso; caché en disco de tc-store y 48 MiB en memoria en la UI).
8. **Persistencia:** migración 0005 `media_extra` (voz, título, intérprete, forma de onda decodificada, MIME y la
   marca de vídeo redondo, porque el CHECK de `media.kind` de la 0001 no admite otro valor sin reconstruir la tabla).

## Consecuencias
- Linux necesita `libasound2-dev` + `pkg-config` para compilar y `libasound.so.2` para ejecutar (matiza la
  decisión 1 de ADR-0006); la CI lo instala. Windows y macOS siguen sin dependencias.
- `opus-decoder` es joven: en debug hace un desplazamiento de `u8` ≥ 8 bits (`celt/vq.rs`) que entra en pánico;
  `[profile.dev.package.opus-decoder] overflow-checks = false` iguala debug a release. Si da problemas con notas
  reales, la alternativa es `audiopus_sys` con libopus empaquetado (`cc`) solo en el crate de audio.
- No verificado: sonido real por altavoces/auriculares en ningún SO, cambio de dispositivo en caliente, latencia
  y consumo con audios largos.
