# Plan de pruebas Parlo (replica-test, app de escritorio Rust)

Fecha: 2026-11-14. Adaptación del skill `replica-skill:replica-test`: no hay Playwright ni web; la «automatización» son tests de `cargo` (update de iced contra `MockBackend`, tc-telegram con transporte falso, tc-store, tc-secret) y capturas sin ventana. Plan base por flujo: `docs/test/test-plan.md` (T-Fxx-nn); aquí cada caso lleva ID del skill (Fxx-H/E/N) y estado.

Estados: **AUTO** = lo cubre un test existente (se cita); **NUEVO** = test añadido en esta pasada (`crates/tc-ui/src/tests_qa.rs`); **MANUAL** = ejecutable a mano por una persona con la app; **REAL** = no probado contra Telegram real (nunca se ha ejecutado; requiere cuenta y red); **PEND** = depende de una función no construida.

Resultado de la ejecución (2026-11-14): `cargo test --workspace` 411 pasan, 0 fallan, 2 ignorados (live_roundtrip del llavero se ejecutó aparte con `--ignored`: pasa; capturas del panel ejecutadas con `TC_PANEL_SHOTS`: pasan, en `replica/evidence/`). Con los 9 tests nuevos: 420 pasan + 1 ignorado a propósito (B-01). Clippy `-D warnings` limpio.

## F01 Login con teléfono
| ID | Caso | Estado |
| --- | --- | --- |
| F01-H1 | Teléfono, código, 2FA → shell | AUTO `tests_m2::login_telefono_espera_codigo_y_2fa`; mock `tc-mock tests::simulated_login_phone_code_and_2fa`; red falsa `tc-telegram tests::flows::phone_code_and_2fa_login_with_dc_migration` |
| F01-H2 | Segundo arranque salta al shell (sesión persistida) | AUTO `tc-app stores::tests::restart_and_resume_with_memory_vault`, `tests_m2::sin_login_entra_directo_y_conector_asincrono` |
| F01-E1 | Teléfono vacío/mal formado | AUTO `login::tests::telefono_en_formato_internacional`, `tc-telegram auth::tests::phone_validation` |
| F01-E2 | Código con letras/espacios, recorte | AUTO `login::tests::codigo_solo_cifras_y_recortado` |
| F01-E3 | Código incorrecto y luego correcto | AUTO `tests::flows::invalid_code_keeps_waiting` |
| F01-E4 | Cuenta atrás / reenvío | AUTO `login::tests::cuentas_atras` |
| F01-E5 | FloodWait corto se espera, largo se devuelve | AUTO `tests::flows::flood_wait_short_is_slept_long_is_returned`, `net::tests::flood_wait_policy_sleeps_only_short_waits` |
| F01-E6 | Migración de DC | AUTO `phone_code_and_2fa_login_with_dc_migration` |
| F01-E7 | Un `LoggedOut` repetido no borra lo escrito | AUTO `login::tests::un_logged_out_repetido_no_borra_lo_escrito` |
| F01-E8 | Sin red durante el login | AUTO `tests::flows::network_errors_retry_with_backoff_then_report_offline` |
| F01-E9 | Número sin cuenta → registro (S06) | AUTO `tests_m2::login_por_qr_numero_sin_cuenta_y_escape` |
| F01-N1 | 2FA incorrecta | AUTO `tc-mock` (login simulado) y `login::tests::errores_con_texto_propio` |
| F01-N2 | Sesión revocada: vuelve al login y limpia | AUTO `tests::flows::revoked_session_logs_out_and_clears_store`, `net::tests::only_specific_401s_end_the_session` |
| F01-N3 | Secretos nunca en claro en disco | AUTO `stores::tests::session_goes_to_the_vault_not_to_the_file`, `session::tests::debug_never_prints_the_auth_key` |
| F01-R1 | Login real (SMS, 2FA real, DC real) | REAL |

## F02 Login por QR
| F02-H1 | QR aceptado → sesión | AUTO `tests_m2::login_por_qr_numero_sin_cuenta_y_escape`, `tc-mock simulated_qr_login_is_accepted_later`, `tests::flows::qr_login_renews_migrates_and_completes` |
| F02-E1 | QR caduca y se renueva; URL base64url sin relleno | AUTO `qr_login_renews...`, `auth::tests::qr_url_is_unpadded_base64url` |
| F02-E2 | Escape vuelve a teléfono | AUTO `tests_m2::login_por_qr_numero_sin_cuenta_y_escape` |
| F02-R1 | Escanear con móvil real | REAL / MANUAL |

## F03 Abrir chat y leer
| F03-H1 | Abrir chat carga la primera página y marca leído | AUTO `tests_reading::lectura_progresiva_con_espera`, `tests::enviar_optimista_...` |
| F03-H2 | Abre en el primer no leído con contexto | AUTO `tests_reading::abre_en_el_primer_no_leido_con_contexto_y_sin_marcar_todo`, `tc-store cached_tests::ventana_del_primer_no_leido_...` |
| F03-E1 | Sin no leídos abre al final | AUTO `tests_reading::sin_no_leidos_abre_al_final` |
| F03-E2 | Historial de 5000 sin huecos, ventana virtual | AUTO `tests::historial_paginado_carga_los_5000_sin_huecos`, `tc-mock long_history_paging_covers_everything_without_gaps` |
| F03-E3 | Ir al final; lo nuevo espera fuera de la ventana | AUTO `tests_reading::ir_al_final_...`, `pagina_hacia_lo_nuevo_sin_duplicados_y_llega_al_final` |
| F03-E4 | Sin red: caché fría = error, caché tibia sirve | AUTO `cached_tests::cold_cache_offline_is_an_error_and_pagination_works`, `history_is_lossless_paginates_and_serves_offline` |
| F03-E5 | Un chat no aparece dos veces al recargar listas | AUTO `tests_chat_actions::a_chat_never_appears_twice_after_reloading_the_lists` |
| F03-E6 | Fluidez de scroll a 60 fps con 10 000 mensajes (wgpu/tiny-skia) | MANUAL |
| F03-R1 | Historial real con huecos y mensajes de servicio | REAL |

## F04 Enviar texto
| F04-H1 | Enter envía optimista → confirmado, compositor vacío | AUTO `tests::enviar_optimista_confirmar_editar_reaccionar_borrar` |
| F04-E1 | Vacío / solo espacios / solo saltos | NUEVO `tests_qa::f04_texto_vacio_o_en_blanco_no_se_envia` |
| F04-E2 | Acentos, emoji, saltos internos, recorte | NUEVO `f04_texto_con_emoji_acentos_y_saltos_se_envia_integro` |
| F04-E3 | Doble Enter no duplica | NUEVO `f04_doble_enter_no_duplica_el_mensaje` |
| F04-E4 | Más de 4096 caracteres: aviso claro y sin burbuja | NUEVO (ignorado, **falla**: B-01) `f04_texto_de_mas_de_4096_avisa_y_no_deja_burbuja` |
| F04-E5 | Evento duplicado del backend no duplica la fila | AUTO `tests::enviar_optimista_...` |
| F04-E6 | Shift+Enter inserta salto; Enter envía (teclado real) | MANUAL |
| F04-E7 | IME, pegado de texto enorme | MANUAL |
| F04-E8 | Borrador: se guarda al cambiar de chat y se restaura | AUTO parcial: persistencia en `tc-store dialogs::tests::drafts_set_clear_list`, `tc-mock pin_archive_mute_draft`; el ciclo UI (`save_draft_task`, `open`) no tiene test (ver «por comprobar» en bugs.md) |
| F04-E9 | Canal sin permiso: compositor bloqueado | AUTO `tests::chat_secreto_huella_y_canal_de_solo_lectura` |
| F04-N1 | Sin red: mensaje fallido + toast, reintento | AUTO `tests::fallo_de_envio_y_reintento` |
| F04-N2 | Reintento con el mismo `random_id` no duplica | NUEVO `f04_reintento_conserva_random_id_y_no_duplica_en_el_backend`; `tc-telegram flows::send_text_confirms_and_is_idempotent`; `tc-mock send_is_idempotent_by_random_id` |
| F04-N3 | Validación del backend (vacío, peer) | AUTO `tc-mock send_validates_input_and_peer` |
| F04-R1 | Envío real, flood real | REAL |

## F05 Responder
| F05-H1 | Responder: barra, `reply_to` en el pendiente | NUEVO `f05_responder_lleva_reply_to_y_cancelar_lo_quita`; AUTO `tc-mock reply_must_reference_existing_message` |
| F05-E1 | Cancelar con Esc / ✕ no deja cita | NUEVO (idem) + AUTO `tests::enviar_optimista_...` (Esc) |
| F05-E2 | Borrar el citado cancela el modo respuesta | NUEVO `f05_borrar_el_citado_cancela_la_respuesta` |
| F05-E3 | Clic en la cita salta al original no cargado | PEND/MANUAL (sin test; ver bugs «por comprobar») |
| F05-N1 | Respuesta a un mensaje que no existe | AUTO `tc-mock reply_must_reference_existing_message` |

## F06 Reenviar
| F06-H1 | Reenviar crea mensajes con origen | AUTO `tc-mock forward_creates_messages_with_origin`; en UI no hay flujo de reenvío (`clone=no`): PEND |
| F06-N1 | Chat secreto no reenvía | PEND |

## F07 Editar
| F07-H1 | ↑ edita el último propio, «editado» visible | AUTO `tests::enviar_optimista_...`; `tc-mock edit_only_own_messages` |
| F07-E1 | Editar con texto vacío no envía y sigue en edición; Esc limpia | NUEVO `f07_editar_con_texto_vacio_no_envia_y_sigue_editando` |
| F07-E2 | ↑ solo toma mensajes propios | NUEVO `f07_editar_ultimo_solo_toma_mensajes_propios` |
| F07-N1 | Mensaje ajeno no editable (backend) | AUTO `tc-mock edit_only_own_messages` |
| F07-N2 | Texto idéntico (MESSAGE_NOT_MODIFIED) | AUTO parcial: mapeo en `net::tests::errors_map_to_domain`; el aviso al usuario dice «vacío» (ver B-01) |
| F07-R1 | Plazo de edición real | REAL |

## F08 Borrar
| F08-H1 | Borrar para todos / solo para mí (diálogo, casilla) | AUTO `tests::enviar_optimista_...`, `tests_chat_actions::message_menu_selection_and_bulk_delete` |
| F08-E1 | Cancelar el diálogo no cambia nada | NUEVO `f08_cancelar_el_dialogo_de_borrado_no_toca_el_mensaje` |
| F08-E2 | Tecla Supr pregunta, borra y restaura si falla | AUTO `tests_chat_actions::delete_key_asks_then_removes_and_restores_on_error` |
| F08-E3 | Borrado remoto en caché persiste y se anuncia | AUTO `cached_tests::remote_deletions_in_refreshed_range_are_removed_and_announced`, `cached_actions_tests::removal_events_are_persisted` |
| F08-R1 | Revocar en grupo sin ser admin | REAL |

## F09 Reacciones
| F09-H1 | Poner/mover/quitar mi reacción | AUTO `tests_reactions::my_reaction_moves_like_the_server_does`, `tc-mock reactions_toggle_and_replace`, `tc-store reactions::tests::my_reaction_moves_between_emojis` |
| F09-N1 | Premium requerida: revierte con aviso | AUTO `tests_reactions::premium_required_reverts_the_optimistic_reaction_with_a_clear_toast` |
| F09-E1 | Reacciones ofrecidas sin Premium, por chat | AUTO `tests_reactions::the_mock_offers_...`, `tc-telegram reactions::tests::compose_filters_premium_and_chat_and_orders_by_use` |
| F09-E2 | Clic derecho/doble clic abre selector | MANUAL |

## F10 Archivos y medios
| F10-H1 | Descarga con progreso, caché de disco, nunca para secretos | AUTO `tests_media::el_progreso_de_descarga_se_registra_y_se_limpia`, `cached_tests::downloads_are_cached_on_disk_but_never_for_secret_chats`, `media_cache::tests::fetch_cached_goes_to_the_network_once_and_never_for_secrets` |
| F10-H2 | Visor de fotos: cierre, navegación, zoom | AUTO `tests_media::visor_vias_de_cierre_navegacion_y_zoom`, `fotos_de_la_conversacion_y_visor` |
| F10-E1 | Máx. 4 descargas simultáneas; referencias caducadas se refrescan; FILE_MIGRATE | AUTO `tc-telegram tests::media::*` |
| F10-E2 | Subida de archivos desde la UI | PEND (la subida existe en tc-store `uploads::tests::lifecycle_progress_resume_and_purge`, sin UI) |
| F10-E3 | Arrastrar y soltar, pegar imagen | MANUAL / PEND |
| F10-E4 | Avatares solo de lo visible | AUTO `tests_media::la_lista_pide_solo_avatares_visibles_y_los_dibuja_al_llegar` |
| F10-R1 | Fotos/archivos reales de Telegram | REAL |

## F11 Buscar
| F11-H1 | Búsqueda de chats (mayúsculas, sin resultados, limpiar) | AUTO `tests::lista_fijados_primero_busqueda_y_carpetas` |
| F11-E1 | FTS con acentos, prefijo, alcance | AUTO `tc-store messages::tests::fts_search_accents_prefix_scope_and_delete`, `cached_tests::events_are_persisted_and_searchable_locally` |
| F11-E2 | Búsqueda global de mensajes / en chat con filtros | PEND |

## F12 Carpetas
| F12-H1 | Filtros fijos Todos/Sin leer/Personas/Grupos/Canales | AUTO `tests::lista_fijados_primero_busqueda_y_carpetas` |
| F12-E1 | CRUD y orden de carpetas (persistencia) | AUTO `tc-store folders::tests::*`; UI: PEND |

## F13 Info de chat / acciones de chat
| F13-H1 | Menús según tipo de chat; silenciar, fijar, archivar, marcar leído | AUTO `tests_chat_actions::chat_menu_items_depend_on_the_kind_of_chat`, `unread_mark_read_pin_limit_message`, `archive_is_optimistic_reverts_on_error_and_can_be_undone` |
| F13-E1 | Salir, bloquear, vaciar, borrar chat | AUTO `tests_chat_actions::leave_block_info_and_clear`, `tc-telegram tests::dialog_actions::*` |
| F13-E2 | Límite de fijados, errores en español | AUTO `pin_limit_is_an_error`, `tests_chat_actions::errors_and_links_in_spanish` |

## F14 Notificaciones / F15 Sesiones
| F14-H1 | Ajustes de notificación por chat (silenciar hasta) | AUTO `tc-mock mute_for_a_while_then_unmute`, `tc-telegram pin_limit_and_mute_until`, `tc-store dialogs::tests::notify_scopes` |
| F14-R1 | Notificación del SO, bandeja, no molestar | MANUAL (no implementado en UI: PEND) |
| F15-H1 | Listar y cerrar sesiones | PEND (sin función) |

## F16 Tiempo real
| F16-H1 | Entrante en vivo reordena la lista, no leídos +1 | AUTO `tests::entrante_en_vivo_reordena_la_lista`, `tc-mock script_delivers_incoming_messages` |
| F16-E1 | Lectura más arriba: no mueve el scroll; lo nuevo espera fuera de ventana | AUTO `tests_reading::ir_al_final_...`; la compensación de scroll (`row_height_estimate`) MANUAL |
| F16-E2 | Pump de updates mapea eventos y se recupera de errores | AUTO `tc-telegram tests::flows::update_pump_maps_events_and_recovers_from_errors`, `mapping::updates_*` |
| F16-E3 | Reconexión con backoff y huecos (getDifference) | AUTO parcial `network_errors_retry_with_backoff_then_report_offline`; huecos reales: REAL |
| F16-R1 | Latencia y orden con tráfico real | REAL |

## F17 Stickers, GIF, emoji
| F17-H1 | Selector, teclado, envío, recientes | AUTO `tests_m2::selector_de_stickers_teclado_envio_y_recientes`, `stickers::tests::la_rejilla_se_recorre_con_flechas_sin_salirse` |
| F17-E1 | Sticker fallido se reintenta como sticker | AUTO `tests_m2::sticker_fallido_se_reintenta_como_sticker` |
| F17-E2 | WebP/TGS inválido o bomba: error, sin pánico | AUTO `tc-stickers tests::webp_garbage_is_error`, `tgs_bomb_is_rejected`, `anim_tests::tgs_invalido_se_rechaza` |
| F17-E3 | Animación: solo con foco, sin movimiento reducido, solo lo visible | AUTO `tests_sticker_anim::*`, `sticker_anim::tests::sin_foco_o_con_movimiento_reducido_no_hay_trabajo` |
| F17-E4 | Panel emojis (búsqueda, tono, recientes) y GIFs (guardados, búsqueda, envío, MP4) | AUTO `tests_panel::*`, `gifs::tests::*`; capturas `replica/evidence/*.png` |
| F17-E5 | Stickers/reacciones reales de la cuenta; referencias caducadas | AUTO `tests_reactions::account_stickers_load_in_the_picker_and_in_bubbles`, `tc-telegram tests::stickers_reactions::*`, `tests::gifs::*` |
| F17-E6 | Rendimiento con muchas animaciones | MANUAL |
| F17-R1 | GIF/MP4 y TGS reales de Telegram | REAL |

## F18 Chat secreto
| F18-H1 | Crear, aceptar invitación, TTL, cerrar | AUTO `tests_m2::crear_chat_secreto_ttl_y_cerrar`, `invitacion_entrante_se_acepta` |
| F18-H2 | Cripto MTProto 2.0 con vectores independientes; e2e entre dos partes | AUTO `tc-secret` (13 unit + `tests/e2e.rs` 22 + `snapshot.rs` + `tl_visual_ttl.rs`) |
| F18-E1 | Huella visual 8×8 y 4 grupos | AUTO `tests::chat_secreto_huella_y_canal_de_solo_lectura`, `tl_visual_ttl` |
| F18-E2 | Nunca en claro en caché/FTS; sellado | AUTO `cached_tests::secret_chats_never_reach_plaintext_tables`, `secret::tests::not_in_fts_and_cascade_delete`, `seal::tests::*` |
| F18-E3 | Borrar chat secreto lo descarta | AUTO `tc-mock dialog_actions::deleting_a_secret_chat_discards_it` |
| F18-R1 | Interoperar con cliente oficial; auditoría de seguridad | REAL (tc-secret es experimental, sin auditoría) |

## Transversales
| X-1 | Tema claro/oscuro, movimiento reducido | AUTO `tests::tema_y_movimiento_conmutables`, `tc-theme tests::*` |
| X-2 | Contraste AA | AUTO `tc-theme tests::contraste_*` |
| X-3 | Selección de backend sin credenciales no filtra valores | AUTO `select::tests::*`, `config::tests::missing_and_invalid_values_never_echo_the_value` |
| X-4 | Teclado/Tab, Wayland/X11, HiDPI, IME, portal de archivos | MANUAL |
| X-5 | Accesibilidad con lector de pantalla | MANUAL (iced no expone árbol de accesibilidad: ver bugs «por comprobar») |
| X-6 | Capturas headless (wgpu) | AUTO `tests_panel_shots::capturas_del_panel` con `TC_PANEL_SHOTS=<dir>` (8 PNG en `replica/evidence/`) |
