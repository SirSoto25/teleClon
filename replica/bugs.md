# Bugs (replica-test, 2026-11-14)

Solo lo reproducido. Entorno: Linux, `cargo test`, `MockBackend`; nada contra Telegram real.

## B-01 (S3) Texto de más de 4096 caracteres: burbuja condenada y aviso equivocado
- Pasos: abrir cualquier chat, pegar 4097 caracteres en el compositor, Enter.
- Esperado: aviso claro («Máximo 4096 caracteres») y no se envía (T-F04-04 del plan).
- Actual: `App::send` (crates/tc-ui/src/app.rs, `fn send`) solo comprueba vacío/permiso; crea la burbuja pendiente. El backend real (`tc-telegram`, `MAX_TEXT` = 4096) lo rechaza con `InvalidInput("text")` y `error_text` responde «No se puede enviar un mensaje vacío.»; la burbuja queda en Failed y «Reintentar» fallará siempre. El mismo texto engañoso sale con `MESSAGE_NOT_MODIFIED` al editar sin cambios y `MESSAGE_TOO_LONG`. El mock tampoco valida el límite, así que la divergencia mock/real oculta el fallo.
- Evidencia: `cargo test -p tc-ui tests_qa -- --ignored` → `f04_texto_de_mas_de_4096_avisa_y_no_deja_burbuja` FALLA («no debe crear burbuja pendiente», 61 ≠ 60). Test en crates/tc-ui/src/tests_qa.rs.
- Arreglo propuesto (no aplicado, se pidió no tocar código): comprobar `chars().count() > 4096` en `send` con toast propio, distinguir `InvalidInput` por campo/causa en `error_text`, y validar el límite también en el mock.
- Estado: abierto.

## Por comprobar (no reproducido)
- P-01 Borrador: no hay test del ciclo UI (`save_draft_task` al cambiar de chat y restauración en `open`); solo la persistencia. Comprobar a mano que el borrador y su cita sobreviven al cambio de chat y al reinicio.
- P-02 `Msg::StartEdit(id)` acepta cualquier id sin comprobar que el mensaje sea propio; la UI quizá solo lo ofrece en los propios. Comprobar que el menú contextual de un mensaje ajeno no ofrece «Editar».
- P-03 Clic en la cita para saltar a un mensaje aún no cargado (F05-E3): sin test.
- P-04 Compensación de scroll con mensaje entrante leyendo más arriba (`row_height_estimate`): solo visual.
- P-05 Captura `conversacion-gifs-wgpu.png`: en la barra lateral, la insignia de «Sin leer» (11) se solapa con el icono y los iconos de «Personas»/«Grupos» quedan recortados por su insignia; posible S4 visual, falta confirmar en vivo.
- P-06 Accesibilidad: sin etiquetas para lector de pantalla verificadas; foco visible con Tab sin comprobar.
- P-07 Todo contra la red real (login, envío, límites, flood, medios, reacciones, GIFs MP4/TGS, secretos interoperables) nunca se ha ejecutado.

## Por severidad
S1: 0 · S2: 0 · S3: 1 (B-01) · S4: 0 (P-05 pendiente de confirmar)
