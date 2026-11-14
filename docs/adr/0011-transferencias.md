# ADR-0011: Transferencias (subidas y descargas a disco con progreso y cancelación)

- **Fecha:** 2026-11-14 · **Estado:** aceptado (fase A0 del plan `docs/plans/archivos-busqueda-notificaciones.md`:
  contrato en `tc-core`, simulación en `tc-mock` y *passthrough* en `tc-store`; tc-telegram aún con la
  implementación por defecto, la hace A1). Probado solo con el mock.
- **Contexto:** `send_media` no daba progreso ni se podía cancelar y `download_media` devuelve el archivo entero
  en memoria (`Vec<u8>`): no sirve para guardar documentos grandes en disco. Las fases A1 (red), A2 (UI de envío)
  y A3 (UI de descarga) necesitan un contrato común antes de empezar en paralelo.

## Decisiones
1. **Tipos** (`tc-core/src/transfer.rs`):
   - `TransferId = Upload(RandomId) | Download(u64)`. Dos espacios separados: una subida y una descarga nunca
     chocan. El de las subidas lo fija la UI (ver 3) y el de las descargas el backend (contador propio).
   - `Transfer { id, kind: Upload|Download, peer, msg: Option<MsgId>, name, done: u64, total: u64, state }` con
     `TransferState = Running | Done | Failed(BackendError) | Cancelled`. `kind` repite `id.kind()` para la UI;
     `msg` (no estaba en el plan) es el mensaje de una descarga, para que A3 encuentre la burbuja aunque el
     evento llegue antes que el id (ver 5); en subidas es `None` porque el mensaje aún no existe. `total = 0` =
     desconocido (`fraction()` → `None`, progreso indeterminado).
   - Evento `BackendEvent::TransferProgress(Transfer)`. Se mantiene `BackendEvent::Download{file_id, done, total}`
     para `download_media` (a memoria: notas de voz, fotos), que ya usa la UI.
   - Error nuevo `BackendError::Cancelled` («Se canceló la transferencia.» en `tc-ui::error_text`).
2. **Métodos nuevos en `MediaApi`** con implementación por defecto `Err(unsupported())`, como se hizo con los GIF
   (ADR-0010), para no romper a quien implemente el trait (tc-telegram compila sin cambios):
   - `download_media_to(peer, msg, file, variant, dest: PathBuf) -> BackendResult<TransferId>`
   - `cancel_transfer(id: TransferId) -> BackendResult<()>`
   No hay implementación por defecto «de respaldo» sobre `download_media` porque `tc-core` no hace E/S.
3. **`send_media` conserva su firma** (`-> BackendResult<Vec<Message>>`). Es lo menos invasivo: no cambia
   `OutgoingMedia` (no hay que tocar sus constructores), ni `ThreadScope`/`ForumApi::send_topic_media`, ni la
   persistencia de `CachedBackend`, y un backend sin progreso sigue cumpliendo el contrato. El id de la
   transferencia se **deriva** del primer `random_id` (`TransferId::upload_of(&media)`), que la UI genera antes
   de llamar: así puede cancelar mientras espera el futuro sin que haga falta una variante «con id» ni un canal
   de vuelta. Descartado: (a) añadir un campo `transfer` a `OutgoingMedia` (rompe constructores en UI, mock y
   tests); (b) un método `send_media_tracked` (dos caminos de envío que mantener y la UI tendría que elegir);
   (c) devolver un `TransferId` y llegar los mensajes solo por evento (rompe a `CachedBackend` y al foro).
   Una llamada = **una** transferencia (todos los elementos): `total` = suma de `size_bytes`, `name` = el del
   primer elemento. Cancelar → `Err(BackendError::Cancelled)`; lo ya confirmado con `MessageSent` (si el
   backend envía elemento a elemento) queda enviado.
4. **Descarga a disco:** escribe por trozos en `partial_path(dest)` (`<dest>.part`, mismo directorio) y al
   terminar renombra a `dest` (atómico; **reemplaza** si existe: quien llama elige `dest` con `unique_path` o
   tras confirmar «Guardar como…»). `Done` se emite con el archivo ya en su sitio (la UI puede «Abrir» al
   momento). Fallo o cancelación: se borra el parcial y `dest` queda como estaba. Vuelve en cuanto arranca;
   errores inmediatos sin eventos: `NotFound`, `InvalidInput("dest")` (directorio inexistente o `dest` es un
   directorio), `Network` sin conexión. No pasa por la caché de medios (el archivo va donde dice el usuario).
5. **Eventos:** cada transferencia emite `Running` (con `done` creciente, empezando en 0) y **exactamente un**
   estado terminal. El primer `Running` puede llegar **antes** de que `download_media_to` devuelva el id: la UI
   debe aceptar eventos de ids que aún no conoce (por eso `Transfer` lleva `peer`, `msg` y `name`). Los
   `Running` son efímeros (se pueden coalescer, gana el último por id); la red real los limita a ≤ 10 Hz; el
   mock, a `MAX_STEPS` = 100 por transferencia. En subidas, `Done` llega después de los `MessageSent`.
   Errores de validación previos (lista vacía, chat de solo lectura) no emiten eventos.
6. **`cancel_transfer` idempotente:** cancelar algo terminado o desconocido devuelve `Ok(())` (la carrera entre
   terminar y pulsar «Cancelar» no es un error). Si la cancelación llega tarde, la transferencia acaba en `Done`.
7. **Nombres de archivo** (`tc-core/src/file_name.rs`): `sanitize_file_name` (sin `/`, `\`, `<>:"|?*` ni
   controles → `_`; quita controles bidi que disfrazan extensiones; puntos seguidos → uno, así nunca hay `..`;
   sin puntos ni espacios al principio o al final; reservados de Windows `CON`/`nul.txt`/`COM1`/`LPT¹`… con `_`
   delante; ≤ 200 bytes conservando una extensión de ≤ 16; vacío → `archivo`; idempotente y siempre un único
   componente normal) y `unique_path(dir, name)` (sufijo « (1)», « (2)»… antes de la extensión;
   `unique_path_with` recibe la comprobación de existencia, para pruebas). Es la única E/S de `tc-core` (una
   consulta de solo lectura, `Path::exists`); entre comprobar y crear hay carrera (TOCTOU), aceptable porque el
   renombrado final de la descarga es atómico. Los 200 bytes dejan sitio al sufijo y a `.part` bajo los 255
   de ext4/NTFS.
8. **Mock** (`tc-mock/src/transfers.rs`): `TransferSim { chunk, delay, fail_at: Option<(bytes, BackendError)> }`
   con `MockBackend::set_transfer_sim`; espera `delay` con el reloj de tokio (en pruebas con
   `start_paused = true` el tiempo es virtual y exacto), cancelación con `tokio::sync::watch`, fallo inyectable
   al llegar a N bytes. Por defecto `delay = 0` (las pruebas existentes no se ralentizan). Las descargas generan
   los bytes en memoria (son de muestra) pero los escriben por trozos en el `.part`.

## Consecuencias
- A1 implementa en tc-telegram `download_media_to` (`upload.getFile` por partes a `.part` + renombrado,
  `FILE_REFERENCE_*`, cambio de DC) y el progreso/cancelación de `send_media` emitiendo los eventos con
  `TransferId::upload_of`. A2 genera los `random_ids`, muestra la burbuja de subida con `Upload(random_ids[0])` y
  trata `Err(Cancelled)` quitando la burbuja (no como fallo). A3 usa `unique_path` + `download_media_to` +
  `TransferState::Done` para «Abrir».
- `tc-ui` solo recibió dos líneas (texto del error y el evento ignorado) para que el `match` exhaustivo compile.
- Pendiente: reanudar subidas tras cerrar la app (fuera de alcance), límite de transferencias simultáneas
  (semáforo de §4 de la arquitectura) en A1.
