# Graph Report - teleClon  (2026-11-14)

## Corpus Check
- 276 files · ~312,695 words
- Verdict: corpus is large enough that graph structure adds value.
- Unclassified: 13 file(s) not represented in the graph (top: (none) 2, .example 1, .opus 1)

## Summary
- 7087 nodes · 19406 edges · 373 communities (203 shown, 170 thin omitted)
- Extraction: 91% EXTRACTED · 9% INFERRED · 0% AMBIGUOUS · INFERRED: 1684 edges (avg confidence: 0.75)
- Token cost: 0 input · 0 output

## Graph Freshness
- Built from commit: `087b5c85`
- Run `git rev-parse HEAD` and compare to check if the graph is stale.
- Run `graphify update .` after code changes (no API cost).

## Community Hubs (Navigation)
- style.rs
- find.rs
- tokens.rs
- Msg
- Msg
- Inner
- net.rs
- tc-telegram/src/secret.rs
- Sistema de diseño de teleClon — dirección «Brasa»
- conv.rs
- format.rs
- model.rs
- tests_files.rs
- ci.yml
- ChatMsg
- player.rs
- BackendResult
- .update
- topics.rs
- api.rs
- make_icons.py
- mp4.rs
- super
- transfers/tests.rs
- tests/transfers.rs
- .search_with
- ForumMsg
- tests_notify.rs
- icon
- view/mod.rs
- tc-ui/src/tests.rs
- EntityKind
- tc-mock/src/forum.rs
- TcSession
- MsgLayout
- send_dialog.rs
- dh.rs
- tc-ui/src/gifs.rs
- app.rs
- with
- App
- tc-ui/src/stickers.rs
- tc-telegram/src/auth.rs
- Entropy
- e2e.rs
- build_items
- TelegramConfig
- ADR-0003: Arquitectura del cliente (workspace, backend tras trait, caché SQLite, E2E sans-IO)
- tc-core/src/lib.rs
- crypto.rs
- tc-telegram/src/lib.rs
- Cache
- dialogs.rs
- tc-stickers/src/lib.rs
- tc-store/src/lib.rs
- Compositor (componente)
- GrammersBackend
- Frame
- tc-store/src/lib.rs
- Cabecera de conversación
- Tipografía (Atkinson Hyperlegible, Bricolage, JetBrains Mono)
- crate::Inner
- pack.rs
- ChatKey
- conversation.rs
- restored
- mapping.rs
- media_cache.rs
- view/audio.rs
- secret.rs
- messages.rs
- FilesMsg
- tl.rs
- rules.rs
- decode.rs
- tests_audio.rs
- tc-ui/src/audio.rs
- tc-telegram/src/media.rs
- CachedBackend<B>
- Store
- reading.rs
- tc-secret/src/session.rs
- EntityKind
- SessionSnapshot
- view/login.rs
- LoginMsg
- tests_reading.rs
- tc-mock/src/lib.rs
- main.rs
- make_logo.py
- MediaCacheLayer<B>
- super
- cached_tests.rs
- tc-mock/src/dialog_actions.rs
- 2. Casos por flujo
- tc-secret
- uploads.rs
- CLAUDE.md
- .claude/CLAUDE.md
- NoDevice
- StoreError
- BackendEvent
- sidebar.rs
- tests/secret.rs
- StickerError
- sidebar.rs
- file_name.rs
- UserId
- .input_peer
- folders.rs
- files.rs
- sticker_anim.rs
- tl_visual_ttl.rs
- Message
- GifAnims
- jobs
- reactions.rs
- SearchQuery
- MockBackend
- Marca propia — Parlo
- ThreadRef
- tc-store
- Informe de barrido de marca
- env
- release
- tests_reactions.rs
- transfer.rs
- .remember_docs
- AudioError
- ImageKey
- Track
- Tone
- stores.rs
- PanelMsg
- windows
- .sticker_send_task
- Plan de pruebas Parlo (replica-test, app de escritorio Rust)
- emoji.rs
- meta
- 0005-nombre-parlo.md
- MediaVariant
- linux.rs
- tc-store/src/convert.rs
- instant
- MuteFor
- vault.rs
- Store
- src/login.rs
- State
- .on_chat
- find_bar.rs
- UserId
- apply_media_extra
- App
- waveform.rs
- Event
- .update_inner
- app/notify.rs
- f
- tc-telegram/src/forum.rs
- Suppress
- tests_qa.rs
- Step
- tc-telegram/src/auth.rs
- tests/media.rs
- tests/search.rs
- Item
- tests_downloads.rs
- palette.py
- sweep.py
- TextStyle
- tests_chat_actions.rs
- StickerId
- Informe de emprendedor: qué molesta, qué falta y qué plan de mejoras para Parlo
- BackendResult
- pattern.rs
- StorePrefs
- tc-mock/src/gifs.rs
- Path
- chat_actions.rs
- ADR-0007: Audio (notas de voz y música)
- make_fixtures.sh
- on
- build.rs
- install-user.sh
- env
- build-macos.sh
- tests_find.rs
- build-linux.sh
- MsgLayout
- Category
- windows.rs
- anim_tests.rs
- cached_actions_tests.rs
- tc_core
- sticker_img.rs
- Login
- CodeKind
- tests_panel_shots.rs
- context_menu.rs
- PanelTab
- tc-ui/src/tests.rs
- ADR-0012: Búsqueda dentro de un chat
- tc-mock/src/audio.rs
- math
- ADR-0008: Stickers animados (TGS con rlottie; WebM estático)
- tests/mod.rs
- ADR-0009: Teclado de emojis
- ADR-0010: GIFs (guardados, búsqueda, envío y animación)
- emoji_es.rs
- media_key
- tests_forum.rs
- VaultError
- Plan de acción: los 5 primeros faltantes del informe de paridad
- .chat_actions_key
- .sticker_send_task
- SignInResult
- Informe de paridad: Parlo frente al cliente original
- ADR-0011: Transferencias (subidas y descargas a disco con progreso y cancelación)
- tc-ui/src/media.rs
- Nav
- Bugs (replica-test, 2026-11-14)
- all
- sorted
- Backend
- tc-ui/src/anim.rs
- Decoded
- ADR-0013: Notificaciones de escritorio (crate `tc-notify`)

## God Nodes (most connected - your core abstractions)
1. `Msg` - 441 edges
2. `App` - 295 edges
3. `BackendResult` - 226 edges
4. `PeerRef` - 145 edges
5. `c()` - 136 edges
6. `MockBackend` - 120 edges
7. `Message` - 102 edges
8. `MsgId` - 92 edges
9. `t1()` - 81 edges
10. `Brasa` - 80 edges

## Surprising Connections (you probably didn't know these)
- `Clean-room (sin código, assets ni marca de Telegram)` --semantically_similar_to--> `Reglas de trabajo (log, ADR, clean-room, credenciales, regenerar grafo)`  [INFERRED] [semantically similar]
  README.md → docs/00-INDEX.md
- `Dirección Brasa (grafito cálido + acento persimón)` --rationale_for--> `Clean-room (sin código, assets ni marca de Telegram)`  [INFERRED]
  docs/adr/0002-direccion-visual.md → README.md
- `Cache` --references--> `Topics`  [INFERRED]
  crates/tc-telegram/src/events.rs → crates/tc-telegram/src/forum.rs
- `radios_de_union()` --calls--> `bubble_radius()`  [INFERRED]
  crates/tc-ui/src/conv.rs → crates/tc-ui/src/style.rs
- `Fuera de alcance (red MTProto, marca, llamadas, historias, bots, pagos)` --conceptually_related_to--> `grammers (MTProto en Rust)`  [INFERRED]
  replica/recon.md → docs/adr/0001-stack.md

## Import Cycles
- 1-file cycle: `crates/tc-stickers/src/anim.rs -> crates/tc-stickers/src/anim.rs`
- 1-file cycle: `crates/tc-store/src/vault.rs -> crates/tc-store/src/vault.rs`
- 1-file cycle: `crates/tc-theme/src/tokens.rs -> crates/tc-theme/src/tokens.rs`
- 2-file cycle: `crates/tc-secret/src/session.rs -> crates/tc-secret/src/session/snapshot.rs -> crates/tc-secret/src/session.rs`
- 2-file cycle: `crates/tc-audio/src/decode.rs -> crates/tc-audio/src/lib.rs -> crates/tc-audio/src/decode.rs`
- 2-file cycle: `crates/tc-audio/src/lib.rs -> crates/tc-audio/src/player.rs -> crates/tc-audio/src/lib.rs`
- 2-file cycle: `crates/tc-notify/src/lib.rs -> crates/tc-notify/src/null.rs -> crates/tc-notify/src/lib.rs`
- 2-file cycle: `crates/tc-video/src/decode.rs -> crates/tc-video/src/lib.rs -> crates/tc-video/src/decode.rs`
- 2-file cycle: `crates/tc-video/src/lib.rs -> crates/tc-video/src/mp4.rs -> crates/tc-video/src/lib.rs`
- 2-file cycle: `crates/tc-mock/src/forum.rs -> crates/tc-mock/src/lib.rs -> crates/tc-mock/src/forum.rs`
- 2-file cycle: `crates/tc-mock/src/lib.rs -> crates/tc-mock/src/transfers.rs -> crates/tc-mock/src/lib.rs`
- 2-file cycle: `crates/tc-telegram/src/auth.rs -> crates/tc-telegram/src/lib.rs -> crates/tc-telegram/src/auth.rs`
- 2-file cycle: `crates/tc-telegram/src/lib.rs -> crates/tc-telegram/src/secret.rs -> crates/tc-telegram/src/lib.rs`
- 2-file cycle: `crates/tc-secret/src/dh.rs -> crates/tc-secret/src/testing.rs -> crates/tc-secret/src/dh.rs`
- 2-file cycle: `crates/tc-ui/src/app.rs -> crates/tc-ui/src/login.rs -> crates/tc-ui/src/app.rs`
- 2-file cycle: `crates/tc-ui/src/app.rs -> crates/tc-ui/src/media.rs -> crates/tc-ui/src/app.rs`
- 3-file cycle: `crates/tc-audio/src/decode.rs -> crates/tc-audio/src/ogg.rs -> crates/tc-audio/src/lib.rs -> crates/tc-audio/src/decode.rs`
- 3-file cycle: `crates/tc-audio/src/decode.rs -> crates/tc-audio/src/lib.rs -> crates/tc-audio/src/player.rs -> crates/tc-audio/src/decode.rs`
- 3-file cycle: `crates/tc-video/src/decode.rs -> crates/tc-video/src/mp4.rs -> crates/tc-video/src/lib.rs -> crates/tc-video/src/decode.rs`
- 4-file cycle: `crates/tc-audio/src/decode.rs -> crates/tc-audio/src/ogg.rs -> crates/tc-audio/src/lib.rs -> crates/tc-audio/src/player.rs -> crates/tc-audio/src/decode.rs`

## Hyperedges (group relationships)
- **Implementaciones y decoradores de ChatBackend** — docs_architecture_mockbackend, docs_architecture_cachedbackend, docs_architecture_tc_telegram, docs_architecture_chatbackend [EXTRACTED 0.95]
- **Fundamentos visuales de Brasa** — docs_design_system_paleta_oklch, docs_design_system_tipografia, docs_design_system_espaciado_radios, docs_design_system_elevacion, docs_design_system_movimiento [EXTRACTED 0.95]
- **Decisiones de arquitectura ADR-0001 a ADR-0004** — docs_adr_0001_stack, docs_adr_0002_direccion_visual, docs_adr_0003_arquitectura, docs_adr_0004_stickers_y_chats_secretos [EXTRACTED 1.00]
- **Pila de chats secretos (cripto, claves y UI)** — docs_architecture_tc_secret, docs_architecture_keyring_boveda, docs_design_system_chat_secreto_ui [INFERRED 0.75]
- **Implementaciones de ChatBackend (Mock, Cached, Telegram)** — docs_adr_0003_arquitectura_chatbackend, docs_adr_0003_arquitectura_mockbackend, docs_adr_0003_arquitectura_cachedbackend, docs_adr_0001_stack_grammers [INFERRED 0.85]
- **Pila de chats secretos E2E** — docs_adr_0004_stickers_y_chats_secretos_secret_chats, docs_adr_0003_arquitectura_tc_secret, docs_adr_0001_stack_keyring, docs_adr_0004_stickers_y_chats_secretos_crypto_risk [INFERRED 0.85]

## Communities (373 total, 170 thin omitted)

### Community 0 - "style.rs"
Cohesion: 0.13
Nodes (54): app_style(), avatar_ink(), avatar_radius(), badge(), bare_input(), border_fill(), Brasa, bubble() (+46 more)

### Community 1 - "find.rs"
Cohesion: 0.06
Nodes (31): CONV_SCROLL_ID, App, ChatSearch, DEBOUNCE, DEBOUNCE_TICK, esc_captured(), FIELD_ID, filter_from_digit() (+23 more)

### Community 2 - "tokens.rs"
Cohesion: 0.03
Nodes (58): AVATAR_LG, AVATAR_MD, AVATAR_RATIO, AVATAR_SM, AVATAR_XL, AVATAR_XS, BREAKPOINT_SPLIT, BREAKPOINT_TRIPLE (+50 more)

### Community 3 - "Msg"
Cohesion: 0.02
Nodes (79): Msg, AcceptSecret, AskDelete, AskDiscardSecret, Audio, Backend, CancelCompose, Chat (+71 more)

### Community 4 - "Msg"
Cohesion: 0.06
Nodes (38): Arquitectura de teleClon, ADR-0001 Stack, ADR-0002 Dirección visual, ADR-0003 Arquitectura, ADR-0004 Stickers y chats secretos, BackendEvent, ChatBackend (trait), Modelo de concurrencia (runtime tokio único, actores) (+30 more)

### Community 6 - "net.rs"
Cohesion: 0.09
Nodes (21): ConnectionState, Connecting, Offline, Online, Updating, ConnTracker, errors_map_to_domain(), flood_wait_policy_sleeps_only_short_waits() (+13 more)

### Community 7 - "tc-telegram/src/secret.rs"
Cohesion: 0.06
Nodes (23): utf16_starts(), utf16_to_chars(), now_secs(), Chat, crypto(), EARLY_CAP, entities_from_secret(), entities_round_trip_through_utf16() (+15 more)

### Community 8 - "Sistema de diseño de teleClon — dirección «Brasa»"
Cohesion: 0.15
Nodes (26): about_thirty_chats_sorted_with_pinned_first(), data_is_deterministic(), delete_updates_last_message_and_emits(), edit_only_own_messages(), forward_creates_messages_with_origin(), grupo_con_muchos_no_leidos_y_lectura_parcial(), history_after_and_around(), long_history_paging_covers_everything_without_gaps() (+18 more)

### Community 9 - "conv.rs"
Cohesion: 0.06
Nodes (43): DATE_H, EDGE_H, emoji_only(), FILE_CARD_H, FILE_CARD_W, FWD_H, GAP_GROUP, GAP_IN_GROUP (+35 more)

### Community 10 - "format.rs"
Cohesion: 0.18
Nodes (22): todas_las_palabras_en_espanol_apuntan_a_emojis_reales(), date_separator(), day_key(), ellipsize(), fold(), hhmm(), horas_y_separadores_en_utc(), huella_determinista_y_en_rango() (+14 more)

### Community 11 - "model.rs"
Cohesion: 0.14
Nodes (13): INK, WHITE, AA_NORMAL, AA_UI, Alpha, Alphas, Rgb, bubble_out_gradient() (+5 more)

### Community 12 - "tests_files.rs"
Cohesion: 0.07
Nodes (52): ALBUM_MAX, CAPTION_COUNTER_FROM, CAPTION_ID, caption_len(), clases_por_extension(), classify(), clipboard_image_to_png(), FILE_WARN_BYTES (+44 more)

### Community 13 - "ci.yml"
Cohesion: 0.07
Nodes (29): name, runs-on, steps, strategy, concurrency, cancel-in-progress, group, env (+21 more)

### Community 14 - "ChatMsg"
Cohesion: 0.06
Nodes (34): ChatMsg, AskBlock, AskClear, AskDeleteSelected, BlockDone, CancelConfirm, CancelSelect, Cleared (+26 more)

### Community 15 - "player.rs"
Cohesion: 0.13
Nodes (16): Cmd, Paused, Play, Speed, Stop, Current, fake(), guarded() (+8 more)

### Community 16 - "BackendResult"
Cohesion: 0.06
Nodes (15): ChatBackend, DialogsApi, MediaApi, MessagesApi, ProfileApi, StickersApi, UNSUPPORTED, BackendResult (+7 more)

### Community 17 - ".update"
Cohesion: 0.11
Nodes (18): BUNDLED, DISPLAY, DISPLAY_FAMILY, EMOJI_FAMILY, family(), MONO, MONO_FALLBACK, MONO_FAMILY (+10 more)

### Community 18 - "topics.rs"
Cohesion: 0.08
Nodes (31): topic_initial(), base(), ellipsize_to(), Font_, Paragraph, SHAPING, text_width(), to_spans() (+23 more)

### Community 19 - "api.rs"
Cohesion: 0.16
Nodes (15): ADR-0002: Dirección visual Brasa, Dirección Brasa (grafito cálido + acento persimón), Controlled para chrome, innovative para burbujas y fondo, Movimiento con muelles 119-221 ms y movimiento reducido, OKLCH como fuente de verdad y contraste WCAG AA (144 pares), docs/design/system.md y tokens.json, Tipografía libre (Atkinson Hyperlegible Next, Bricolage Grotesque, JetBrains Mono) e iconos Phosphor, Agente diseño (opus/high) (+7 more)

### Community 20 - "make_icons.py"
Cohesion: 0.21
Nodes (8): app_png(), decode_png(), icns(), ico(), ico_dib(), _inner(), main(), rasterize()

### Community 21 - "mp4.rs"
Cohesion: 0.06
Nodes (68): basura_o_vacio_es_error(), center(), Clip, Collector, decode_mp4(), decode_track(), decodifica_un_mp4_codificado_con_openh264(), dominant() (+60 more)

### Community 22 - "super"
Cohesion: 0.10
Nodes (14): DeadVault, key_is_created_once_in_the_vault_and_reused(), roundtrip_and_every_kind_of_tampering_is_rejected(), SEAL_KEY_LEN, Sealed, SealError, BadKey, Entropy (+6 more)

### Community 23 - "transfers/tests.rs"
Cohesion: 0.06
Nodes (30): partial_path(), partial_path_is_a_sibling(), check_dest(), Inner, MAX_STEPS, MockBackend, storage(), terminal() (+22 more)

### Community 24 - "tests/transfers.rs"
Cohesion: 0.12
Nodes (31): ANA, assert_well_formed(), content(), Delayed, DOC, doc_location(), doc_media(), doc_ref() (+23 more)

### Community 25 - ".search_with"
Cohesion: 0.13
Nodes (4): MessagesApiSearch, filter_to_tl(), GrammersBackend, total_of()

### Community 26 - "ForumMsg"
Cohesion: 0.05
Nodes (33): App, ForumMsg, BackToTopics, ClearQuery, Close, CloseList, CloseMenu, Create (+25 more)

### Community 27 - "tests_notify.rs"
Cohesion: 0.12
Nodes (29): Prefs, chat_key(), claves_ida_y_vuelta(), topic_key(), accion_marcar_como_leido(), ajustes_persisten_y_se_cargan_al_arrancar(), ajustes_por_tipo(), archivados_siguen_el_ajuste() (+21 more)

### Community 28 - "icon"
Cohesion: 0.19
Nodes (12): ADR-0001: Stack técnico, grammers (MTProto en Rust), iced (UI en Rust, wgpu + tiny-skia), keyring (claves y sesión), rusqlite bundled con migraciones por PRAGMA user_version, tc-secret: chats secretos sans-IO con RustCrypto, tc-stickers: WebP, TGS con rlottie, WebM aplazado, ADR-0004: Stickers y chats secretos en el alcance (+4 more)

### Community 29 - "view/mod.rs"
Cohesion: 0.12
Nodes (26): Preview, Full, Hidden, NameOnly, NotifyMsg, Dnd, Preview, Set (+18 more)

### Community 30 - "tc-ui/src/tests.rs"
Cohesion: 0.24
Nodes (12): ADR-0003: Arquitectura del cliente, Orden de construcción M1.1-M1.4, M2.1, M2.7, CachedBackend (caché como decorador, tc-store), Trait ChatBackend (sub-traits + events()), Modelo de concurrencia (tokio, hilo SQLite, canal acotado, Resync), MockBackend determinista, SQLite por cuenta (17 tablas + FTS5, WAL, chat_key), Scroll virtual propio (Fenwick, ventana ~600 mensajes) (+4 more)

### Community 31 - "EntityKind"
Cohesion: 0.24
Nodes (13): Registro de agentes, Agente arquitecto (opus/high), Agente core+mock (sonnet/medium), Agente QA (pendiente), Agente theme (sonnet/medium), Agente UI (opus/high), Bitácora (append-only), Bug iced 0.14: scrollable.direction antes de anchor_bottom (+5 more)

### Community 32 - "tc-mock/src/forum.rs"
Cohesion: 0.08
Nodes (27): el_foro_tiene_temas_con_general_y_no_leidos_que_suman(), entrante_en_tema_crear_y_fijar(), FORUM_CHAT, FORUM_GROUP, Forums, historial_envio_y_lectura_por_tema(), Inner, mock() (+19 more)

### Community 34 - "TcSession"
Cohesion: 0.09
Nodes (11): changes_mark_dirty_and_reset_forgets_everything(), debug_never_prints_the_auth_key(), MemorySessionStore, rejects_garbage_and_unknown_versions(), roundtrip_keeps_keys_peers_and_state(), sample(), SESSION_FORMAT, SessionError (+3 more)

### Community 35 - "MsgLayout"
Cohesion: 0.21
Nodes (11): Índice maestro de teleClon, architecture.md (crates, ChatBackend, SQLite, F01-F18), graphify-out (graph.html, GRAPH_REPORT.md, graph.json), Reglas de trabajo (log, ADR, clean-room, credenciales, regenerar grafo), Credenciales api_id/api_hash en .env local gitignored, graphify: instalación y uso, graphifyy en ./.venv local, Registro de skills usadas (+3 more)

### Community 36 - "send_dialog.rs"
Cohesion: 0.19
Nodes (21): Upload, BAR_H, big_preview(), cancel_button(), compress_choice(), dialog(), dialog_notes(), DIALOG_W (+13 more)

### Community 37 - "dh.rs"
Cohesion: 0.07
Nodes (27): dh_matches_independent_vector(), DhParams, DhSecret, exp(), FP, GA, GB, generate_mixes_server_random_and_rejects_bad_peer() (+19 more)

### Community 38 - "tc-ui/src/gifs.rs"
Cohesion: 0.08
Nodes (27): demo_script(), ANIM_BUDGET, ANIM_PX, CACHE_BYTES, CELL_GAP, CELL_H, CELL_MIN_W, decode() (+19 more)

### Community 39 - "app.rs"
Cohesion: 0.05
Nodes (45): Auth, auth_events(), AuthHandle, Backend, backend_events(), BackendHandle, Connector, DemoStep (+37 more)

### Community 40 - "with"
Cohesion: 0.20
Nodes (17): steps, steps, steps, steps, id, if, name, run (+9 more)

### Community 41 - "App"
Cohesion: 0.13
Nodes (7): App, focus_window(), Notifs, parse_key(), Target, Chat, Topic

### Community 42 - "tc-ui/src/stickers.rs"
Cohesion: 0.12
Nodes (32): StickerSetId, Sticker, StickerSet, ANIMATED_SET_ID, Art, art_index(), ART_PX, CELL_ART (+24 more)

### Community 43 - "tc-telegram/src/auth.rs"
Cohesion: 0.15
Nodes (4): AuthInner, GrammersBackend, Inner, map_error()

### Community 45 - "Entropy"
Cohesion: 0.09
Nodes (22): Role, Initiator, Responder, Entropy, Message, Abort, nonzero_random_id(), Output (+14 more)

### Community 46 - "e2e.rs"
Cohesion: 0.17
Nodes (34): chat_n(), concurrent_rekey_converges_on_one_instance(), decreasing_in_seq_aborts_chat(), dh(), discard_local_and_remote(), duplicates_are_dropped(), fingerprint_mismatch_discards_chat(), forge() (+26 more)

### Community 47 - "build_items"
Cohesion: 0.31
Nodes (9): replica/features.csv, Rasgos de marca (avatares cuadrado redondeado, burbujas sin cola, fondo de isolíneas), Agente recon (sonnet/medium), Recon map: Telegram Desktop (Linux), Componentes de UI (Burbuja de mensaje, Fila de chat, Compositor...), Core loop (chats, enviar/recibir, sincronizado), Matriz de funciones (features.csv, must/should/could/skip), Flujos F01-F16 (+1 more)

### Community 48 - "TelegramConfig"
Cohesion: 0.12
Nodes (22): Choice, Mock, Telegram, choose(), FAKE_HASH, missing_credentials(), por_defecto_el_mock_sin_mirar_credenciales(), sin_credenciales_vuelve_al_mock_con_aviso_que_no_ensena_valores() (+14 more)

### Community 49 - "ADR-0003: Arquitectura del cliente (workspace, backend tras trait, caché SQLite, E2E sans-IO)"
Cohesion: 0.35
Nodes (12): tc-app, tc-audio, tc-core, tc-mock, tc-notify, tc-secret, tc-stickers, tc-store (+4 more)

### Community 50 - "tc-core/src/lib.rs"
Cohesion: 0.07
Nodes (33): BackendError, Cancelled, Crypto, FloodWait, Forbidden, Internal, InvalidInput, Network (+25 more)

### Community 52 - "crypto.rs"
Cohesion: 0.07
Nodes (31): decrypt(), derive(), encrypt(), encrypt_with_padding(), HEADER_LEN, ige_decrypt(), ige_encrypt(), IgeDec (+23 more)

### Community 53 - "tc-telegram/src/lib.rs"
Cohesion: 0.08
Nodes (13): Feed, None, Pool, Source, GrammersBackend, has_login_token(), Inner, persister() (+5 more)

### Community 54 - "Cache"
Cohesion: 0.08
Nodes (23): dialog_from_tl(), dialog_stub(), DialogCtx, draft_date(), draft_from_tl(), forward_name(), is_muted(), message_from_tl() (+15 more)

### Community 55 - "dialogs.rs"
Cohesion: 0.06
Nodes (24): DIALOG_COLS, dialog_from_row(), draft_from_row(), drafts_set_clear_list(), list_orders_pinned_then_activity_and_filters_archived(), mark_read_and_missing_dialog_errors(), notify_scopes(), scope_parts() (+16 more)

### Community 56 - "tc-stickers/src/lib.rs"
Cohesion: 0.67
Nodes (3): Disposición (raíl, lista, conversación, panel info), Lista de chats (componente), Barra lateral (raíl de carpetas)

### Community 59 - "GrammersBackend"
Cohesion: 0.11
Nodes (4): GrammersBackend, in_topic(), Inner, topic_from_tl()

### Community 60 - "Frame"
Cohesion: 0.08
Nodes (20): CacheKey, DEFAULT_CACHE_BYTES, FrameCache, Frame, ByteLru, cache_evicts_least_recently_used_by_bytes(), cache_get_or_insert_computes_once(), gradient() (+12 more)

### Community 61 - "tc-store/src/lib.rs"
Cohesion: 0.09
Nodes (22): account_db_path(), APP_DIR, DATA_DIR_NAME, default_data_dir(), LEGACY_APP_DIR, MIGRATIONS, resolve_data_dir(), SCHEMA_VERSION (+14 more)

### Community 64 - "crate::Inner"
Cohesion: 0.19
Nodes (5): crate::Inner, GIF_BOT, gif_from_document(), GifCatalog, input_doc()

### Community 65 - "pack.rs"
Cohesion: 0.09
Nodes (27): Def, DEFS, dots(), index_of(), Layer, len(), NATIVE_SIZE, P (+19 more)

### Community 66 - "ChatKey"
Cohesion: 0.07
Nodes (26): Action, AppInfo, Capabilities, ChatKey, event_channel(), EventSender, Notification, Notifier (+18 more)

### Community 67 - "conversation.rs"
Cohesion: 0.19
Nodes (23): now(), body(), bubble_max(), build_items(), date_pill(), edge(), empty(), experimental_banner() (+15 more)

### Community 68 - "restored"
Cohesion: 0.10
Nodes (42): chat_group(), affected_history(), affected_messages(), block_pin_message_and_info_errors(), clear_and_delete_private_chat_repeat_until_done(), is_user(), leave_and_delete_groups_and_channels(), pin_limit_and_mute_until() (+34 more)

### Community 69 - "mapping.rs"
Cohesion: 0.09
Nodes (41): channel_group(), channel(), chat(), dialog(), message(), msg_enum(), notify(), peer_channel() (+33 more)

### Community 70 - "media_cache.rs"
Cohesion: 0.10
Nodes (15): account_cache_lives_next_to_the_database_and_survives_reopen(), check_key(), create_private_dir(), fetch_cached(), fetch_cached_goes_to_the_network_once_and_never_for_secrets(), io(), keys_depend_on_id_and_size_and_skip_secrets(), MediaCache (+7 more)

### Community 71 - "view/audio.rs"
Cohesion: 0.10
Nodes (13): Bar, BAR_GAP, BAR_W, bubble(), BUTTON, fraction_at(), Ink, ink() (+5 more)

### Community 72 - "secret.rs"
Cohesion: 0.11
Nodes (18): chat(), CHAT_COLS, chat_from_row(), chat_roundtrip_and_fk(), messages_idempotent_blob_roundtrip_and_page(), msg(), MSG_COLS, msg_from_row() (+10 more)

### Community 73 - "messages.rs"
Cohesion: 0.16
Nodes (15): COLS, fill(), from_row(), fts_query(), fts_search_accents_prefix_scope_and_delete(), ids(), media_msg(), pagination_by_anchor() (+7 more)

### Community 74 - "FilesMsg"
Cohesion: 0.08
Nodes (22): COMPOSER_ID, App, drop_event(), FilesMsg, Cancel, Caption, Close, Compress (+14 more)

### Community 75 - "tl.rs"
Cohesion: 0.05
Nodes (56): Queued, Action, AbortKey, AcceptKey, CommitKey, DeleteMessages, FlushHistory, Noop (+48 more)

### Community 76 - "rules.rs"
Cohesion: 0.10
Nodes (59): ACTION_MARK_READ, ajustes_por_tipo(), APP_NAME, archivado_no_silenciado_depende_del_ajuste(), archivado_y_silenciado_nunca_avisa(), avisa_con_foco_si_el_chat_no_es_el_abierto(), avisa_sin_foco_aunque_el_chat_este_abierto(), away() (+51 more)

### Community 77 - "decode.rs"
Cohesion: 0.10
Nodes (16): check_tone(), copy_out(), decode_all(), decodes_music_formats(), decodes_stereo_opus(), decodes_telegram_style_voice_note(), fixture(), freq() (+8 more)

### Community 78 - "tests_audio.rs"
Cohesion: 0.20
Nodes (17): AudioBytes, TrackState, Failed, Idle, Loading, Paused, Playing, boot() (+9 more)

### Community 79 - "tc-ui/src/audio.rs"
Cohesion: 0.06
Nodes (32): PlaybackState, Ended, Failed, Paused, Playing, Starting, SPEEDS, App (+24 more)

### Community 80 - "tc-telegram/src/media.rs"
Cohesion: 0.06
Nodes (27): BIG_FILE, Cancel, check_dest(), crate::Inner, DownloadError, Backend, FileReference, GrammersBackend (+19 more)

### Community 81 - "CachedBackend<B>"
Cohesion: 0.05
Nodes (14): SecretChatsApi, SecretChatId, Anchor, After, Around, Before, Latest, Dialog (+6 more)

### Community 82 - "Store"
Cohesion: 0.13
Nodes (12): favorites(), recents_order_dedupe_cap_and_cascade(), seed(), SET_COLS, set_from_row(), sets_and_stickers(), STICKER_COLS, sticker_from_row() (+4 more)

### Community 83 - "reading.rs"
Cohesion: 0.10
Nodes (21): Message, App, Carry, desplazamiento_deja_la_marca_arriba_con_contexto(), first_unread(), FLUSH_EVERY, from_bottom_for(), height_after() (+13 more)

### Community 84 - "tc-secret/src/session.rs"
Cohesion: 0.04
Nodes (51): &mut E, OsEntropy, random_i64(), random_up_to(), Result, SecretError, Entropy, FingerprintMismatch (+43 more)

### Community 85 - "EntityKind"
Cohesion: 0.11
Nodes (17): EntityKind, Blockquote, Bold, BotCommand, Code, CustomEmoji, Email, Hashtag (+9 more)

### Community 86 - "SessionSnapshot"
Cohesion: 0.09
Nodes (18): MAGIC, Rd, Rd<'a>, read_ready(), RK_ACCEPTED, RK_COMMITTED, RK_IDLE, RK_REQUESTED (+10 more)

### Community 87 - "view/login.rs"
Cohesion: 0.29
Nodes (22): card_screen(), CARD_W, code(), connecting(), divider(), field(), field_error(), ghost() (+14 more)

### Community 88 - "LoginMsg"
Cohesion: 0.12
Nodes (19): LoginMsg, CodeChanged, CodeSent, PasswordChanged, PasswordChecked, PhoneChanged, QrReady, Resend (+11 more)

### Community 89 - "tests_reading.rs"
Cohesion: 0.12
Nodes (18): MockConfig, abre_en_el_primer_no_leido_con_contexto_y_sin_marcar_todo(), boot(), ids(), ir_al_final_carga_los_ultimos_y_lo_nuevo_espera_fuera_de_la_ventana(), lectura_progresiva_con_espera(), NOW, open() (+10 more)

### Community 90 - "tc-mock/src/lib.rs"
Cohesion: 0.06
Nodes (27): ANIMATED_SET_ID, animated_sticker_set(), BIG_CHAT, BIG_CHAT_LEN, build(), CHANNEL_POSTS, gen_channel(), gen_history() (+19 more)

### Community 91 - "main.rs"
Cohesion: 0.09
Nodes (13): env_flag(), env_u64(), load_env_file(), main(), mock_notifier(), session_path(), system_notifier(), telegram_config() (+5 more)

### Community 92 - "make_logo.py"
Cohesion: 0.22
Nodes (11): catmull(), dot_center(), f(), horizontal(), main(), motif(), svg(), symbol() (+3 more)

### Community 94 - "super"
Cohesion: 0.06
Nodes (19): Store, unread_mark_and_clear_history(), Error, Invalid, Io, NoDataDir, NotFound, SchemaTooNew (+11 more)

### Community 95 - "cached_tests.rs"
Cohesion: 0.12
Nodes (30): cache_persists_across_restarts_on_disk(), cached(), chats_secretos_se_buscan_en_el_backend_sin_tocar_la_base(), cold_cache_offline_is_an_error_and_pagination_works(), con_el_historial_completo_se_responde_en_local_sin_preguntar_al_servidor(), dialogs_roundtrip_exactly_and_survive_offline(), downloads_are_cached_on_disk_but_never_for_secret_chats(), edits_deletes_reactions_read_and_failures_write_through() (+22 more)

### Community 96 - "tc-mock/src/dialog_actions.rs"
Cohesion: 0.13
Nodes (14): Acts, all(), block_info_and_pinned_messages(), check_pin_limit(), clear_history_keeps_the_row_delete_removes_it(), deleting_a_secret_chat_discards_it(), leave_only_groups_and_channels_cannot_be_cleared(), mock() (+6 more)

### Community 97 - "2. Casos por flujo"
Cohesion: 0.08
Nodes (23): 1. Estrategia, 2. Casos por flujo, 3. Pruebas transversales, 4. Orden y criterios, F01 Iniciar sesión con teléfono (S01 → S02 → S04 → S05 → S06 → S07), F02 Login con QR (S01 → S02 → S03 → S07), F03 Abrir chat y leer (S07 → S08 → S11), F04 Enviar mensaje de texto (S11 → S13) (+15 more)

### Community 98 - "tc-secret"
Cohesion: 0.40
Nodes (4): Garantías locales, Lo que NO está hecho o NO está verificado, Módulos, tc-secret

### Community 99 - "uploads.rs"
Cohesion: 0.16
Nodes (7): UploadRow, COLS, destination_rules_and_cascade(), from_row(), lifecycle_progress_resume_and_purge(), new_upload(), Store

### Community 103 - "StoreError"
Cohesion: 0.13
Nodes (11): backend(), DeadVault, decode(), Rd, Rd<'a>, VaultSessionStore, file_store_is_private_and_atomic(), FileSessionStore (+3 more)

### Community 104 - "BackendEvent"
Cohesion: 0.07
Nodes (22): BackendEvent, Connection, DialogRemoved, DialogUpserted, Download, ForumTopicsChanged, ForumTopicUpserted, HistoryCleared (+14 more)

### Community 105 - "sidebar.rs"
Cohesion: 0.14
Nodes (19): plain_letters(), preview(), badge_at(), chat_row(), folder_icon(), LINE_H, list(), rail() (+11 more)

### Community 106 - "tests/secret.rs"
Cohesion: 0.10
Nodes (23): UpdateBatch, UpdateSource, UpdatesReceiver, ANA, BETO, ChanSource, CHAT, fingerprint() (+15 more)

### Community 107 - "StickerError"
Cohesion: 0.08
Nodes (28): ANIM_PX, frame_at(), frame_step(), info_of(), MAX_FPS, next_frames(), render_batch(), Show (+20 more)

### Community 108 - "sidebar.rs"
Cohesion: 0.06
Nodes (30): PANEL_H, with_weight(), card_body(), dir_panel(), dl(), icon_button(), link(), link_color() (+22 more)

### Community 109 - "file_name.rs"
Cohesion: 0.15
Nodes (23): assert_safe(), bidi_control(), DEFAULT_FILE_NAME, examples(), forbidden(), is_reserved(), long_names_keep_their_extension_and_char_boundaries(), MAX_EXT_BYTES (+15 more)

### Community 110 - "UserId"
Cohesion: 0.13
Nodes (11): AuthApi, AuthState, Checking, LoggedIn, LoggedOut, SignUpRequired, WaitingCode, WaitingPassword (+3 more)

### Community 111 - ".input_peer"
Cohesion: 0.14
Nodes (3): Inner, reply_to(), flatten_updates()

### Community 112 - "folders.rs"
Cohesion: 0.19
Nodes (8): COLS, folder(), folder_peers_replace_and_cascade(), folders_crud_order_and_reorder(), from_row(), Store, FolderPeerRow, FolderRow

### Community 113 - "files.rs"
Cohesion: 0.13
Nodes (10): decode_sticker_file(), file_ref_from(), file_ref_row(), cache_index_orders_by_last_use(), CacheEntry, media_refs_follow_their_media(), MediaRefRow, peer_photos_roundtrip_and_never_for_secrets() (+2 more)

### Community 115 - "sticker_anim.rs"
Cohesion: 0.11
Nodes (21): Playback, carga_una_vez_y_reintenta_tras_fallo(), DATA_CACHE_BYTES, FRAME_CACHE_BYTES, FULL, Job, Load, Render (+13 more)

### Community 116 - "tl_visual_ttl.rs"
Cohesion: 0.07
Nodes (8): TypingAction, Cancel, Typing, SelfDestructTimer, KeyVisualization, layer(), tl_rejects_unknown_and_trailing(), tl_round_trip_all_supported_constructors()

### Community 117 - "Message"
Cohesion: 0.10
Nodes (18): MsgId, RandomId, Message, OutgoingMedia, OutgoingText, SendState, Failed, Pending (+10 more)

### Community 118 - "GifAnims"
Cohesion: 0.22
Nodes (6): GifAnims, la_cache_de_animaciones_respeta_el_tope(), Slot, Failed, Loading, Ready

### Community 119 - "jobs"
Cohesion: 0.18
Nodes (10): env, CARGO_INCREMENTAL, CARGO_TERM_COLOR, RUST_TOOLCHAIN, jobs, linux, name, needs (+2 more)

### Community 120 - "reactions.rs"
Cohesion: 0.29
Nodes (7): message_exists(), my_reaction_moves_between_emojis(), r(), set_get_replace_and_unknown_message(), setup(), Store, ReactionRow

### Community 121 - "SearchQuery"
Cohesion: 0.12
Nodes (14): fold(), has_link(), SearchFilter, All, Files, Links, Photos, Videos (+6 more)

### Community 123 - "Marca propia — Parlo"
Cohesion: 0.07
Nodes (26): ADR-0006: Empaquetado multiplataforma (Windows, macOS, Linux), Consecuencias, Decisiones, 1. Criterios de nombre, 2. Candidatos (6-8), 3. Chequeos que debe hacer el usuario, 4. Voz y tono (español), 5. Brief de logo (+18 more)

### Community 124 - "ThreadRef"
Cohesion: 0.07
Nodes (26): unsupported(), el_tema_como_dialogo_conserva_lectura_y_no_leidos(), fijados_primero_y_luego_por_actividad(), ForumApi, ForumTopic, GENERAL_TOPIC, msg(), sort_topics() (+18 more)

### Community 125 - "tc-store"
Cohesion: 0.29
Nodes (6): Bóveda de secretos, CachedBackend, Contenido, Decisiones y límites, Sellado y estado cifrado de chats secretos, tc-store

### Community 126 - "Informe de barrido de marca"
Cohesion: 0.33
Nodes (5): A revisar (código que podría llegar a pantalla), Bloqueantes (texto visible o assets), Informe de barrido de marca, Permitidas (resumen por archivo), Resumen

### Community 127 - "env"
Cohesion: 0.18
Nodes (11): APPLE_APP_PASSWORD, APPLE_ID, APPLE_TEAM_ID, MACOS_CERT_P12_BASE64, MACOS_CERT_PASSWORD, MACOS_SIGN_IDENTITY_SECRET, macos, env (+3 more)

### Community 128 - "release"
Cohesion: 0.25
Nodes (8): release, permissions, contents, if, name, needs, permissions, runs-on

### Community 130 - "tests_reactions.rs"
Cohesion: 0.14
Nodes (11): App, apply_mine(), reaction_error_text(), account_stickers_load_in_the_picker_and_in_bubbles(), boot(), decoded(), my_reaction_moves_like_the_server_does(), NOW (+3 more)

### Community 131 - "transfer.rs"
Cohesion: 0.12
Nodes (15): default_methods_are_unsupported(), fraction_and_finished(), Old, Transfer, TransferId, Download, Upload, TransferKind (+7 more)

### Community 133 - ".remember_docs"
Cohesion: 0.25
Nodes (5): CachedSet, crate::Inner, MAX_SETS, set_of(), StickerCatalog

### Community 134 - "AudioError"
Cohesion: 0.10
Nodes (18): AudioStream, open(), OpusStream, sym_err(), AudioError, Corrupt, NoDevice, Unsupported (+10 more)

### Community 135 - "ImageKey"
Cohesion: 0.14
Nodes (8): cache_tracks_loads_failures_and_evicts_least_recently_used(), ImageKey, Avatar, Full, Sticker, Thumb, Images, img()

### Community 136 - "Track"
Cohesion: 0.07
Nodes (6): FakeOutput, NullOutput, Output, RodioOutput, run(), Track

### Community 138 - "Tone"
Cohesion: 0.16
Nodes (10): apply(), Tone, .ALL, Dark, Default, Light, Medium, MediumDark (+2 more)

### Community 139 - "stores.rs"
Cohesion: 0.10
Nodes (27): aad(), AAD_PREFIX, dh(), disk_bytes(), encode(), existing_session_json_is_migrated_once_and_deleted(), FIXED_LEN, media_cache() (+19 more)

### Community 140 - "PanelMsg"
Cohesion: 0.10
Nodes (30): GifMsg, AnimLoaded, More, Query, Results, Retry, Saved, Scrolled (+22 more)

### Community 141 - "windows"
Cohesion: 0.29
Nodes (7): WINDOWS_CERT_PASSWORD, WINDOWS_CERT_PFX_BASE64, windows, env, name, needs, runs-on

### Community 142 - ".sticker_send_task"
Cohesion: 0.10
Nodes (10): ActiveTgs, App, AnimMsg, Focus, Frames, Hover, Loaded, Tick (+2 more)

### Community 143 - "Plan de pruebas Parlo (replica-test, app de escritorio Rust)"
Cohesion: 0.10
Nodes (19): F01 Login con teléfono, F02 Login por QR, F03 Abrir chat y leer, F04 Enviar texto, F05 Responder, F06 Reenviar, F07 Editar, F08 Borrar (+11 more)

### Community 144 - "emoji.rs"
Cohesion: 0.13
Nodes (17): busqueda_en_espanol_ingles_y_por_grupo(), catalog_len(), display_name(), el_catalogo_es_completo_y_por_categorias(), es_index(), FREQUENT_SHOWN, list(), lists() (+9 more)

### Community 145 - "meta"
Cohesion: 0.40
Nodes (5): meta, name, outputs, runs-on, version

### Community 147 - "MediaVariant"
Cohesion: 0.03
Nodes (59): AudioInfo, DEFAULT_REACTIONS, EntityKind, Bold, Code, Hashtag, Italic, Mention (+51 more)

### Community 149 - "linux.rs"
Cohesion: 0.17
Nodes (13): NotifyError, Backend, Unavailable, aviso_real(), backend(), DbusNotifier, DEFAULT_ACTION, DEST (+5 more)

### Community 150 - "tc-store/src/convert.rs"
Cohesion: 0.07
Nodes (45): voice_notes_and_music_survive_the_cache(), apply_dialogs(), apply_history(), cached_dialogs(), cached_history(), decode_entities(), decode_sticker(), dialogs_complete_key() (+37 more)

### Community 151 - "instant"
Cohesion: 0.14
Nodes (14): av(), Available, chat_allowed(), CHAT_TTL, ChatAllowed, All, None, Some (+6 more)

### Community 153 - "MuteFor"
Cohesion: 0.20
Nodes (6): MuteFor, .ALL, EightHours, Forever, Hour, TwoDays

### Community 154 - "vault.rs"
Cohesion: 0.13
Nodes (24): check_key(), CHUNK_HEADER_LEN, CHUNK_MAGIC, chunked_vault_reports_missing_parts_and_keeps_magic_values_intact(), chunked_vault_splits_large_values_and_cleans_up(), clear_session(), construction_and_validation_do_not_touch_the_system(), delete_secret_chat_key() (+16 more)

### Community 156 - "src/login.rs"
Cohesion: 0.17
Nodes (12): clean_code(), CODE_ID, code_where(), codigo_solo_cifras_y_recortado(), countdown(), cuentas_atras(), errores_con_texto_propio(), login_error() (+4 more)

### Community 158 - ".on_chat"
Cohesion: 0.13
Nodes (8): App, Archive, ArchivedLoaded, Forward, InfoLoaded, Mute, Unblock, Chat

### Community 161 - "find_bar.rs"
Cohesion: 0.18
Nodes (15): handle(), HANDLES, icon(), icon_fill(), icon_turned(), icons!, TURNED, view() (+7 more)

### Community 162 - "UserId"
Cohesion: 0.08
Nodes (13): UserId, display_name_joins_parts(), User, UserStatus, LastSeen, Offline, Online, Recently (+5 more)

### Community 164 - "apply_media_extra"
Cohesion: 0.23
Nodes (4): apply_media_extra(), MediaExtraRow, Store, voice()

### Community 165 - "App"
Cohesion: 0.17
Nodes (3): AnimTarget, App, Gif

### Community 166 - "waveform.rs"
Cohesion: 0.39
Nodes (8): decode(), decodes_known_bit_layout(), encode(), from_samples(), MAX, resample(), resample_and_from_samples(), roundtrip_any_length()

### Community 167 - "Event"
Cohesion: 0.10
Nodes (23): Event, Deleted, Discarded, Duplicate, HistoryFlushed, KeyChanged, PeerLayer, PeerLayerNewer (+15 more)

### Community 169 - ".update_inner"
Cohesion: 0.06
Nodes (17): AnimKey, BubbleOpacity, BubbleY, ChatSwitch, Overlay, SendButton, App, Compose (+9 more)

### Community 170 - "app/notify.rs"
Cohesion: 0.05
Nodes (33): DoNotDisturb, Off, On, Until, ajustes_ida_y_vuelta(), b(), CONTACT_JOINED_TEXT, decode() (+25 more)

### Community 171 - "f"
Cohesion: 0.24
Nodes (11): colores_coinciden_con_tokens_json(), contraste_todos_los_pares_declarados_en_tokens_json(), espaciado_radios_bordes_y_elevacion_coinciden(), f(), layout_coincide(), movimiento_coincide(), muelles_convergen_y_rebote_se_pasa(), patron_tokens_y_determinismo() (+3 more)

### Community 172 - "tc-telegram/src/forum.rs"
Cohesion: 0.17
Nodes (9): history_params(), messages_of(), apply_update(), current_topic(), is_forum(), route_new_message(), topic_of(), topic_reply_to() (+1 more)

### Community 173 - "Suppress"
Cohesion: 0.12
Nodes (15): ChatKind, Channel, Group, Private, Secret, Suppress, Archived, ChatOpen (+7 more)

### Community 174 - "tests_qa.rs"
Cohesion: 0.24
Nodes (14): error_text(), boot_open(), f04_doble_enter_no_duplica_el_mensaje(), f04_reintento_conserva_random_id_y_no_duplica_en_el_backend(), f04_texto_con_emoji_acentos_y_saltos_se_envia_integro(), f04_texto_de_mas_de_4096_avisa_y_no_deja_burbuja(), f04_texto_vacio_o_en_blanco_no_se_envia(), f05_borrar_el_citado_cancela_la_respuesta() (+6 more)

### Community 175 - "Step"
Cohesion: 0.22
Nodes (8): App, focus_for(), Step, Checking, Code, Password, Phone, SignUp

### Community 179 - "tc-telegram/src/auth.rs"
Cohesion: 0.13
Nodes (10): code_kind(), code_types_map(), migrate_dc(), next_kind(), phone_validation(), qr_url(), qr_url_is_unpadded_base64url(), Srp (+2 more)

### Community 183 - "tests/media.rs"
Cohesion: 0.09
Nodes (29): document_file_ref(), entities_from_tl(), entities_to_tl(), entity_parts(), media_from_document(), media_from_tl(), photo_dims(), photo_file_ref() (+21 more)

### Community 184 - "tests/search.rs"
Cohesion: 0.20
Nodes (13): busca_en_el_chat_con_texto_offset_y_limite_e_ingiere_usuarios(), cada_filtro_usa_su_input_messages_filter(), CH, chat_secreto_no_soportado_y_error_de_red_se_propaga(), en_un_tema_va_top_msg_id_y_el_scope_lo_aplica(), filter_id(), hit(), pagina_llena_da_next_y_la_ultima_no() (+5 more)

### Community 185 - "Item"
Cohesion: 0.29
Nodes (5): Item, Date, Edge, Msg, UnreadMark

### Community 186 - "tests_downloads.rs"
Cohesion: 0.06
Nodes (66): App, default_dir(), default_dir_in(), dest_in(), DIR_KEY, Dl, DlMsg, Cancel (+58 more)

### Community 187 - "palette.py"
Cohesion: 0.24
Nodes (7): build(), enc(), in_gamut(), lum(), pairs_for(), ratio(), to_hex()

### Community 188 - "sweep.py"
Cohesion: 0.27
Nodes (4): classify(), main(), report(), scan()

### Community 189 - "TextStyle"
Cohesion: 0.13
Nodes (16): BODY, CAPTION, CODE, DISPLAY, EMOJI_ONLY, HEADING, META, TextStyle (+8 more)

### Community 191 - "tests_chat_actions.rs"
Cohesion: 0.25
Nodes (15): action_error_text(), a_chat_never_appears_twice_after_reloading_the_lists(), a_user(), archive_is_optimistic_reverts_on_error_and_can_be_undone(), boot(), chat_menu_items_depend_on_the_kind_of_chat(), chat_msg(), delete_key_asks_then_removes_and_restores_on_error() (+7 more)

### Community 192 - "StickerId"
Cohesion: 0.25
Nodes (4): StickerId, DEFAULT_RECENTS, Favorites, Recents

### Community 194 - "Informe de emprendedor: qué molesta, qué falta y qué plan de mejoras para Parlo"
Cohesion: 0.20
Nodes (9): 0. Muestra y límites (leer primero), 1. Lo que MÁS molesta (quejas sobre lo que el original hace), 2. Lo que FALTA (peticiones por nombre), 3. Lo NO resuelto (trabajos o grupos que el original ignora), 4. Cruce con replica/diff-report.md y replica/features.csv, 5. Plan de mejoras priorizado, 6. Ángulo de venta (documentado, NO perseguido: uso personal), 7. Siguientes pasos (+1 more)

### Community 199 - "pattern.rs"
Cohesion: 0.26
Nodes (13): AMPLITUDE, cell_segments(), ContourParams, .DEFAULT, .SECRET, contours(), field(), lattice() (+5 more)

### Community 202 - "tc-mock/src/gifs.rs"
Cohesion: 0.12
Nodes (17): by_id(), download(), encode_gif(), encode_png(), frame(), FRAMES, GIF_BASE, guardados_busqueda_y_bytes() (+9 more)

### Community 203 - "Path"
Cohesion: 0.24
Nodes (6): asignaciones_comillas_y_comentarios(), el_entorno_manda_sobre_el_archivo_salvo_si_esta_vacio(), EnvFile, lookup(), unquote(), account_id()

### Community 205 - "chat_actions.rs"
Cohesion: 0.14
Nodes (14): ChatActions, Confirm, Block, ClearHistory, DeleteChat, DeleteMessages, Leave, EDIT_WINDOW_SECS (+6 more)

### Community 207 - "ADR-0007: Audio (notas de voz y música)"
Cohesion: 0.50
Nodes (3): ADR-0007: Audio (notas de voz y música), Consecuencias, Decisiones

### Community 211 - "on"
Cohesion: 0.50
Nodes (4): on, push, workflow_dispatch, tags

### Community 215 - "env"
Cohesion: 0.67
Nodes (3): GH_TOKEN, VERSION, env

### Community 217 - "tests_find.rs"
Cohesion: 0.23
Nodes (19): highlight_ranges(), boot(), counter(), deliver(), deliver_window(), en_un_chat_secreto_no_se_busca(), en_un_tema_de_foro_busca_solo_en_el_tema(), esc_cierra_y_restaura_la_ventana_y_la_posicion() (+11 more)

### Community 220 - "MsgLayout"
Cohesion: 0.22
Nodes (6): Body, Bubble, Emoji, Sticker, LayoutCache, MsgLayout

### Community 221 - "Category"
Cohesion: 0.14
Nodes (12): Category, Activities, .ALL, Animals, Flags, Food, Objects, People (+4 more)

### Community 222 - "windows.rs"
Cohesion: 0.09
Nodes (18): bytes_a_cache_estable(), escapa_marcado(), escape_xml(), extension(), icon_path(), ruta_tal_cual(), Icon, Bytes (+10 more)

### Community 223 - "anim_tests.rs"
Cohesion: 0.26
Nodes (10): fotograma_por_tiempo_a_30_fps_como_mucho(), gzip(), info(), movimiento_reducido_foco_y_visibilidad(), MOVING, ms(), siguientes_fotogramas_en_bucle(), sin_lottie_no_hay_animacion() (+2 more)

### Community 225 - "cached_actions_tests.rs"
Cohesion: 0.33
Nodes (8): a_user_chat(), all(), cached(), clear_and_delete_reach_the_cache(), offline_list(), removal_events_are_persisted(), unread_mark_read_mute_and_pin_persist(), CachedBackend

### Community 226 - "tc_core"
Cohesion: 0.19
Nodes (12): MAX_TEXT, MAX_DELETE_ROUNDS, channel_of(), CHANNEL_OFFSET, decode_group(), group_ids_roundtrip_and_never_collide(), GroupTarget, Channel (+4 more)

### Community 227 - "sticker_img.rs"
Cohesion: 0.12
Nodes (12): StickerFormat, Lottie, Webm, Webp, LOCAL_SEND_PX, BUBBLE_PX, CELL_PX, key() (+4 more)

### Community 234 - "Login"
Cohesion: 0.18
Nodes (8): Field, Code, Password, Phone, left(), Login, Qr, un_logged_out_repetido_no_borra_lo_escrito()

### Community 235 - "CodeKind"
Cohesion: 0.22
Nodes (9): CodeKind, App, Call, Email, FlashCall, MissedCall, Other, Sms (+1 more)

### Community 236 - "tests_panel_shots.rs"
Cohesion: 0.33
Nodes (5): App, capturas_del_panel(), colorful(), settings(), shot()

### Community 237 - "context_menu.rs"
Cohesion: 0.07
Nodes (32): message_link(), act(), archive_row(), buttons(), chat(), chat_items(), confirm_panel(), CursorProbe (+24 more)

### Community 238 - "PanelTab"
Cohesion: 0.12
Nodes (11): GifPanel, Search, EMOJI_SEARCH_ID, EmojiPanel, GIF_SEARCH_ID, Panel, PanelTab, .ALL (+3 more)

### Community 239 - "tc-ui/src/tests.rs"
Cohesion: 0.07
Nodes (30): ThemeKind, Dark, Light, boot(), chat_secreto_huella_y_canal_de_solo_lectura(), capturas_de_la_descarga(), settings(), shot() (+22 more)

### Community 240 - "ADR-0012: Búsqueda dentro de un chat"
Cohesion: 0.50
Nodes (3): ADR-0012: Búsqueda dentro de un chat, Consecuencias, Decisiones

### Community 241 - "tc-mock/src/audio.rs"
Cohesion: 0.46
Nodes (7): BARS, deterministic_and_well_formed(), mix(), RATE, voice_wav(), wav_len(), waveform()

### Community 242 - "math"
Cohesion: 0.32
Nodes (4): oklch_to_linear(), blob(), open_blob(), wobble()

### Community 243 - "ADR-0008: Stickers animados (TGS con rlottie; WebM estático)"
Cohesion: 0.33
Nodes (5): ADR-0008: Stickers animados (TGS con rlottie; WebM estático), Consecuencias, Decisión, TGS (Lottie con gzip): animado con rlottie, por defecto solo en Linux, WebM (vídeo VP9 con alfa): queda estático (miniatura)

### Community 244 - "tests/mod.rs"
Cohesion: 0.09
Nodes (24): key_visualization_matches_independent_vector(), tl_known_encoding(), authorization(), hex(), password(), state(), user(), invalid_code_keeps_waiting() (+16 more)

### Community 247 - "ADR-0009: Teclado de emojis"
Cohesion: 0.50
Nodes (3): ADR-0009: Teclado de emojis, Consecuencias, Decisiones

### Community 248 - "ADR-0010: GIFs (guardados, búsqueda, envío y animación)"
Cohesion: 0.50
Nodes (3): ADR-0010: GIFs (guardados, búsqueda, envío y animación), Consecuencias, Decisiones

### Community 275 - "tests_forum.rs"
Cohesion: 0.22
Nodes (11): abrir_un_foro_muestra_la_lista_de_temas(), abrir_un_tema_en_su_primer_no_leido_y_volver(), boot(), busy(), crear_un_tema_lo_abre(), enviar_dentro_del_tema_y_eventos_de_otros_temas(), marcar_leido_desde_el_menu_y_tema_cerrado(), NOW (+3 more)

### Community 285 - "VaultError"
Cohesion: 0.36
Nodes (5): ChunkedVault<V>, map_err(), VaultError, Backend, Unavailable

### Community 288 - "Plan de acción: los 5 primeros faltantes del informe de paridad"
Cohesion: 0.22
Nodes (8): A. Envío, descarga y guardado de archivos, B. Búsqueda dentro de un chat, C. Notificaciones de escritorio, Fuera de alcance por ahora, Los 5 puntos y dónde se resuelven, Orden y dependencias, Plan de acción: los 5 primeros faltantes del informe de paridad, Reparto sugerido

### Community 290 - ".chat_actions_key"
Cohesion: 0.33
Nodes (3): AskDeleteChat, AskLeave, OpenChatMenu

### Community 298 - "SignInResult"
Cohesion: 0.50
Nodes (4): SignInResult, LoggedIn, PasswordNeeded, SignUpNeeded

### Community 304 - "Informe de paridad: Parlo frente al cliente original"
Cohesion: 0.25
Nodes (7): Diferencias de comportamiento (paso 3 de la skill), Filas «partial» por no estar probadas contra Telegram real (`[sin real]`): 33, Informe de paridad: Parlo frente al cliente original, Los 10 faltantes más importantes (orden de construcción, solo must-haves en `no`), Puntuación por área (la más débil primero), Resumen, Siguientes pasos

### Community 305 - "ADR-0011: Transferencias (subidas y descargas a disco con progreso y cancelación)"
Cohesion: 0.50
Nodes (3): ADR-0011: Transferencias (subidas y descargas a disco con progreso y cancelación), Consecuencias, Decisiones

### Community 306 - "tc-ui/src/media.rs"
Cohesion: 0.13
Nodes (17): AVATAR_PX, avatars_are_square_and_downscaled(), bubble_aspect(), BUBBLE_ASPECT, bubble_photos_are_cropped_to_the_layout_ratio(), BUBBLE_PX, CACHE_BYTES, jpeg() (+9 more)

### Community 309 - "Nav"
Cohesion: 0.15
Nodes (10): la_rejilla_se_recorre_con_flechas_sin_salirse(), Nav, Activate, Down, Left, NextTab, PrevTab, Right (+2 more)

### Community 310 - "Bugs (replica-test, 2026-11-14)"
Cohesion: 0.40
Nodes (4): B-01 (S3) Texto de más de 4096 caracteres: burbuja condenada y aviso equivocado, Bugs (replica-test, 2026-11-14), Por comprobar (no reproducido), Por severidad

### Community 326 - "Backend"
Cohesion: 0.15
Nodes (19): Overlay, Delete, DownloadDir, Key, NewSecret, Notifications, Photo, Ttl (+11 more)

### Community 335 - "tc-ui/src/anim.rs"
Cohesion: 0.06
Nodes (33): ByteLru<K, V>, Easing, .ESTANDAR, .RAPIDO, .SALIDA, .SUAVE, HIGHLIGHT_FADE_MS, MotionPreference (+25 more)

### Community 356 - "Decoded"
Cohesion: 0.29
Nodes (8): decode_avatar(), decode_cover(), decode_fit(), Decoded, load(), off_thread(), decode(), fit()

### Community 368 - "ADR-0013: Notificaciones de escritorio (crate `tc-notify`)"
Cohesion: 0.40
Nodes (4): ADR-0013: Notificaciones de escritorio (crate `tc-notify`), Consecuencias, Decisiones, Verificación (2026-11-14)

## Knowledge Gaps
- **631 isolated node(s):** `Connecting`, `Auth`, `Main`, `Do`, `Shot` (+626 more)
  These have ≤1 connection - possible missing edges or undocumented components. (Counts symbols only; 2102 node(s) total have ≤1 connection when file, concept and rationale nodes are included.)
- **170 thin communities (<3 nodes) omitted from report** — run `graphify query` to explore isolated nodes.

## Suggested Questions
_Questions this graph is uniquely positioned to answer:_

- **Why does `Msg` connect `Msg` to `style.rs`, `find.rs`, `tests_reactions.rs`, `tc-telegram/src/secret.rs`, `ImageKey`, `PanelMsg`, `tests_files.rs`, `ChatMsg`, `.sticker_send_task`, `topics.rs`, `tests_forum.rs`, `MediaVariant`, `ForumMsg`, `tests_notify.rs`, `src/login.rs`, `view/mod.rs`, `.on_chat`, `find_bar.rs`, `.chat_actions_key`, `send_dialog.rs`, `App`, `app.rs`, `.sticker_send_task`, `.update_inner`, `app/notify.rs`, `App`, `tests_qa.rs`, `Step`, `Nav`, `tests_downloads.rs`, `tests_chat_actions.rs`, `conversation.rs`, `Backend`, `view/audio.rs`, `FilesMsg`, `chat_actions.rs`, `tests_audio.rs`, `tc-ui/src/audio.rs`, `reading.rs`, `view/login.rs`, `LoginMsg`, `tests_find.rs`, `tests_reading.rs`, `main.rs`, `Decoded`, `sidebar.rs`, `tests_panel_shots.rs`, `context_menu.rs`, `sidebar.rs`, `tc-ui/src/tests.rs`, `sticker_anim.rs`?**
  _High betweenness centrality (0.136) - this node is a cross-community bridge._
- **Are the 32 inferred relationships involving `Msg` (e.g. with `Message` and `DlMsg`) actually correct?**
  _`Msg` has 32 INFERRED edges - model-reasoned connections that need verification._
- **What connects `Connecting`, `Auth`, `Main` to the rest of the system?**
  _631 weakly-connected nodes found - possible documentation gaps or missing edges._
- **Should `style.rs` be split into smaller, more focused modules?**
  _Cohesion score 0.12510974539069358 - nodes in this community are weakly interconnected._
- **Why does `App` connect `.update_inner` to `style.rs`, `find.rs`, `tests_reactions.rs`, `Msg`, `ImageKey`, `tests_files.rs`, `PanelMsg`, `.sticker_send_task`, `topics.rs`, `tests_forum.rs`, `MediaVariant`, `tc-store/src/convert.rs`, `ForumMsg`, `tests_notify.rs`, `src/login.rs`, `view/mod.rs`, `.on_chat`, `tc-mock/src/forum.rs`, `find_bar.rs`, `send_dialog.rs`, `app.rs`, `App`, `tc-ui/src/stickers.rs`, `app/notify.rs`, `tests_qa.rs`, `Nav`, `tests_downloads.rs`, `tests_chat_actions.rs`, `conversation.rs`, `Backend`, `view/audio.rs`, `FilesMsg`, `chat_actions.rs`, `tests_audio.rs`, `tc-ui/src/anim.rs`, `tc-ui/src/audio.rs`, `reading.rs`, `view/login.rs`, `tests_find.rs`, `tests_reading.rs`, `sidebar.rs`, `Login`, `sidebar.rs`, `tests_panel_shots.rs`, `PanelTab`, `tc-ui/src/tests.rs`, `context_menu.rs`, `sticker_anim.rs`, `Message`, `ThreadRef`?**
  _High betweenness centrality (0.063) - this node is a cross-community bridge._
- **Are the 10 inferred relationships involving `App` (e.g. with `ChatActions` and `Downloads`) actually correct?**
  _`App` has 10 INFERRED edges - model-reasoned connections that need verification._
- **Should `find.rs` be split into smaller, more focused modules?**
  _Cohesion score 0.05714285714285714 - nodes in this community are weakly interconnected._