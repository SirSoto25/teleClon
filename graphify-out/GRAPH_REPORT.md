# Graph Report - teleClon  (2026-11-14)

## Corpus Check
- 276 files · ~313,149 words
- Verdict: corpus is large enough that graph structure adds value.
- Unclassified: 13 file(s) not represented in the graph (top: (none) 2, .example 1, .opus 1)

## Summary
- 7094 nodes · 19422 edges · 390 communities (198 shown, 192 thin omitted)
- Extraction: 91% EXTRACTED · 9% INFERRED · 0% AMBIGUOUS · INFERRED: 1684 edges (avg confidence: 0.75)
- Token cost: 0 input · 0 output

## Graph Freshness
- Built from commit: `2ceea4fe`
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
- tc-ui/src/files.rs
- ci.yml
- ChatMsg
- player.rs
- BackendResult
- .update
- topics.rs
- api.rs
- app.rs
- tc-video/src/decode.rs
- super
- transfers/tests.rs
- tests/transfers.rs
- .search_with
- ForumMsg
- tests_notify.rs
- icon
- NotifyMsg
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
- mp4.rs
- BackendResult
- crypto.rs
- tc-telegram/src/lib.rs
- Cache
- dialogs.rs
- tc-stickers/src/lib.rs
- tc-store/src/lib.rs
- Compositor (componente)
- GrammersBackend
- tc-telegram/src/media.rs
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
- tests_files.rs
- tc-mock/src/lib.rs
- main.rs
- make_logo.py
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
- tests/snapshot.rs
- folders.rs
- files.rs
- tc-mock/src/search.rs
- sticker_anim.rs
- tl_visual_ttl.rs
- Message
- GifAnims
- jobs
- reactions.rs
- SearchQuery
- Inner
- Marca propia — Parlo
- tc-core/src/lib.rs
- tc-store
- Informe de barrido de marca
- env
- release
- tests_reactions.rs
- app/downloads.rs
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
- sticker_img.rs
- linux.rs
- tc-store/src/convert.rs
- DlMsg
- DownloadError
- GifMsg
- vault.rs
- Store
- src/login.rs
- .download_profile_photo
- tests_panel.rs
- mux
- cached.rs
- apply_media_extra
- App
- waveform.rs
- Event
- MockBackend
- .update_inner
- app/notify.rs
- f
- reply_to
- Suppress
- tests_qa.rs
- Step
- B
- tests/media.rs
- tests/search.rs
- Item
- .toast
- Field
- Media
- TextStyle
- tests_chat_actions.rs
- StickerId
- Informe de emprendedor: qué molesta, qué falta y qué plan de mejoras para Parlo
- BackendResult
- Usage
- StorePrefs
- tc-mock/src/gifs.rs
- FileClass
- tests_downloads.rs
- .apply_auth
- ADR-0007: Audio (notas de voz y música)
- make_fixtures.sh
- on
- build.rs
- install-user.sh
- env
- build-macos.sh
- tests_find.rs
- build-linux.sh
- layout_message
- Category
- windows.rs
- anim_tests.rs
- Body
- cached_actions_tests.rs
- tc_core
- .next
- Login
- CodeKind
- context_menu.rs
- PanelTab
- arc
- ADR-0012: Búsqueda dentro de un chat
- tc-mock/src/audio.rs
- ADR-0008: Stickers animados (TGS con rlottie; WebM estático)
- tests/mod.rs
- ADR-0009: Teclado de emojis
- ADR-0010: GIFs (guardados, búsqueda, envío y animación)
- emoji_es.rs
- User
- tests_forum.rs
- VaultError
- Prefs
- Plan de acción: los 5 primeros faltantes del informe de paridad
- .sticker_send_task
- Informe de paridad: Parlo frente al cliente original
- ADR-0011: Transferencias (subidas y descargas a disco con progreso y cancelación)
- tc-ui/src/media.rs
- Nav
- Bugs (replica-test, 2026-11-14)
- Backend
- tc-ui/src/anim.rs
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
- `radios_de_union()` --calls--> `bubble_radius()`  [INFERRED]
  crates/tc-ui/src/conv.rs → crates/tc-ui/src/style.rs
- `Fuera de alcance (red MTProto, marca, llamadas, historias, bots, pagos)` --conceptually_related_to--> `grammers (MTProto en Rust)`  [INFERRED]
  replica/recon.md → docs/adr/0001-stack.md
- `Crates tc-ui y tc-app (M1.1 lista, M1.2 conversación, M1.3 compositor)` --implements--> `iced (UI en Rust, wgpu + tiny-skia)`  [INFERRED]
  docs/logs/log.md → docs/adr/0001-stack.md

## Import Cycles
- 1-file cycle: `crates/tc-stickers/src/anim.rs -> crates/tc-stickers/src/anim.rs`
- 1-file cycle: `crates/tc-store/src/vault.rs -> crates/tc-store/src/vault.rs`
- 1-file cycle: `crates/tc-theme/src/tokens.rs -> crates/tc-theme/src/tokens.rs`
- 2-file cycle: `crates/tc-notify/src/lib.rs -> crates/tc-notify/src/null.rs -> crates/tc-notify/src/lib.rs`
- 2-file cycle: `crates/tc-audio/src/decode.rs -> crates/tc-audio/src/lib.rs -> crates/tc-audio/src/decode.rs`
- 2-file cycle: `crates/tc-audio/src/lib.rs -> crates/tc-audio/src/player.rs -> crates/tc-audio/src/lib.rs`
- 2-file cycle: `crates/tc-video/src/decode.rs -> crates/tc-video/src/lib.rs -> crates/tc-video/src/decode.rs`
- 2-file cycle: `crates/tc-video/src/lib.rs -> crates/tc-video/src/mp4.rs -> crates/tc-video/src/lib.rs`
- 2-file cycle: `crates/tc-mock/src/forum.rs -> crates/tc-mock/src/lib.rs -> crates/tc-mock/src/forum.rs`
- 2-file cycle: `crates/tc-mock/src/lib.rs -> crates/tc-mock/src/transfers.rs -> crates/tc-mock/src/lib.rs`
- 2-file cycle: `crates/tc-telegram/src/auth.rs -> crates/tc-telegram/src/lib.rs -> crates/tc-telegram/src/auth.rs`
- 2-file cycle: `crates/tc-telegram/src/lib.rs -> crates/tc-telegram/src/secret.rs -> crates/tc-telegram/src/lib.rs`
- 2-file cycle: `crates/tc-ui/src/app.rs -> crates/tc-ui/src/media.rs -> crates/tc-ui/src/app.rs`
- 2-file cycle: `crates/tc-ui/src/app.rs -> crates/tc-ui/src/login.rs -> crates/tc-ui/src/app.rs`
- 2-file cycle: `crates/tc-secret/src/session.rs -> crates/tc-secret/src/session/snapshot.rs -> crates/tc-secret/src/session.rs`
- 2-file cycle: `crates/tc-secret/src/dh.rs -> crates/tc-secret/src/testing.rs -> crates/tc-secret/src/dh.rs`
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

## Communities (390 total, 192 thin omitted)

### Community 0 - "style.rs"
Cohesion: 0.13
Nodes (55): filter_label(), app_style(), avatar_ink(), avatar_radius(), badge(), bare_input(), border_fill(), Brasa (+47 more)

### Community 1 - "find.rs"
Cohesion: 0.06
Nodes (29): App, ChatSearch, DEBOUNCE, DEBOUNCE_TICK, FIELD_ID, filter_from_digit(), FILTERS, find_error_text() (+21 more)

### Community 2 - "tokens.rs"
Cohesion: 0.03
Nodes (58): AVATAR_LG, AVATAR_MD, AVATAR_RATIO, AVATAR_SM, AVATAR_XL, AVATAR_XS, BREAKPOINT_SPLIT, BREAKPOINT_TRIPLE (+50 more)

### Community 3 - "Msg"
Cohesion: 0.03
Nodes (86): Msg, AcceptSecret, AskDelete, AskDiscardSecret, Backend, CancelCompose, Chat, ClearSearch (+78 more)

### Community 4 - "Msg"
Cohesion: 0.06
Nodes (38): Arquitectura de teleClon, ADR-0001 Stack, ADR-0002 Dirección visual, ADR-0003 Arquitectura, ADR-0004 Stickers y chats secretos, BackendEvent, ChatBackend (trait), Modelo de concurrencia (runtime tokio único, actores) (+30 more)

### Community 6 - "net.rs"
Cohesion: 0.09
Nodes (20): ConnectionState, Connecting, Offline, Online, Updating, ConnTracker, errors_map_to_domain(), flood_wait_policy_sleeps_only_short_waits() (+12 more)

### Community 7 - "tc-telegram/src/secret.rs"
Cohesion: 0.06
Nodes (23): Entity, utf16_to_chars(), now_secs(), Chat, crypto(), EARLY_CAP, entities_from_secret(), entities_round_trip_through_utf16() (+15 more)

### Community 8 - "Sistema de diseño de teleClon — dirección «Brasa»"
Cohesion: 0.15
Nodes (26): about_thirty_chats_sorted_with_pinned_first(), data_is_deterministic(), delete_updates_last_message_and_emits(), edit_only_own_messages(), forward_creates_messages_with_origin(), grupo_con_muchos_no_leidos_y_lectura_parcial(), history_after_and_around(), long_history_paging_covers_everything_without_gaps() (+18 more)

### Community 9 - "conv.rs"
Cohesion: 0.06
Nodes (35): DATE_H, EDGE_H, FILE_CARD_H, FILE_CARD_W, FWD_H, GAP_GROUP, GAP_IN_GROUP, GROUP_AVATAR (+27 more)

### Community 10 - "format.rs"
Cohesion: 0.18
Nodes (22): todas_las_palabras_en_espanol_apuntan_a_emojis_reales(), date_separator(), day_key(), ellipsize(), fold(), hhmm(), horas_y_separadores_en_utc(), huella_determinista_y_en_rango() (+14 more)

### Community 11 - "model.rs"
Cohesion: 0.15
Nodes (13): INK, WHITE, AA_NORMAL, AA_UI, Alpha, Alphas, Rgb, bubble_out_gradient() (+5 more)

### Community 12 - "tc-ui/src/files.rs"
Cohesion: 0.11
Nodes (26): ALBUM_MAX, CAPTION_COUNTER_FROM, caption_len(), clases_por_extension(), classify(), clipboard_image_to_png(), FILE_WARN_BYTES, foto_comprimida_o_archivo() (+18 more)

### Community 13 - "ci.yml"
Cohesion: 0.07
Nodes (29): name, runs-on, steps, strategy, concurrency, cancel-in-progress, group, env (+21 more)

### Community 14 - "ChatMsg"
Cohesion: 0.04
Nodes (57): App, ChatMsg, Archive, ArchivedLoaded, AskBlock, AskClear, AskDeleteChat, AskDeleteSelected (+49 more)

### Community 15 - "player.rs"
Cohesion: 0.13
Nodes (16): Cmd, Paused, Play, Speed, Stop, Current, fake(), guarded() (+8 more)

### Community 16 - "BackendResult"
Cohesion: 0.04
Nodes (15): ChatBackend, DialogsApi, MediaApi, MessagesApi, StickersApi, UNSUPPORTED, unsupported(), BackendResult (+7 more)

### Community 17 - ".update"
Cohesion: 0.11
Nodes (18): BUNDLED, DISPLAY, DISPLAY_FAMILY, EMOJI_FAMILY, family(), MONO, MONO_FALLBACK, MONO_FAMILY (+10 more)

### Community 18 - "topics.rs"
Cohesion: 0.07
Nodes (38): now(), Measurer, Run, plain_letters(), preview(), base(), ellipsize_to(), Font_ (+30 more)

### Community 19 - "api.rs"
Cohesion: 0.16
Nodes (15): ADR-0002: Dirección visual Brasa, Dirección Brasa (grafito cálido + acento persimón), Controlled para chrome, innovative para burbujas y fondo, Movimiento con muelles 119-221 ms y movimiento reducido, OKLCH como fuente de verdad y contraste WCAG AA (144 pares), docs/design/system.md y tokens.json, Tipografía libre (Atkinson Hyperlegible Next, Bricolage Grotesque, JetBrains Mono) e iconos Phosphor, Agente diseño (opus/high) (+7 more)

### Community 20 - "app.rs"
Cohesion: 0.04
Nodes (42): Easing, .ESTANDAR, .RAPIDO, .SALIDA, .SUAVE, AnimKey, ChatSwitch, Overlay (+34 more)

### Community 21 - "tc-video/src/decode.rs"
Cohesion: 0.10
Nodes (29): basura_o_vacio_es_error(), center(), Clip, Collector, decode_mp4(), decode_track(), decodifica_un_mp4_codificado_con_openh264(), dominant() (+21 more)

### Community 22 - "super"
Cohesion: 0.11
Nodes (13): key_is_created_once_in_the_vault_and_reused(), roundtrip_and_every_kind_of_tampering_is_rejected(), SEAL_KEY_LEN, Sealed, SealError, BadKey, Entropy, Rejected (+5 more)

### Community 23 - "transfers/tests.rs"
Cohesion: 0.06
Nodes (30): partial_path(), partial_path_is_a_sibling(), check_dest(), Inner, MAX_STEPS, MockBackend, storage(), terminal() (+22 more)

### Community 24 - "tests/transfers.rs"
Cohesion: 0.12
Nodes (31): ANA, assert_well_formed(), content(), Delayed, DOC, doc_location(), doc_media(), doc_ref() (+23 more)

### Community 25 - ".search_with"
Cohesion: 0.16
Nodes (3): messages_of(), MessagesApiSearch, GrammersBackend

### Community 26 - "ForumMsg"
Cohesion: 0.08
Nodes (21): ForumMsg, BackToTopics, ClearQuery, Close, CloseList, CloseMenu, Create, Created (+13 more)

### Community 27 - "tests_notify.rs"
Cohesion: 0.13
Nodes (28): chat_key(), claves_ida_y_vuelta(), topic_key(), accion_marcar_como_leido(), ajustes_persisten_y_se_cargan_al_arrancar(), ajustes_por_tipo(), archivados_siguen_el_ajuste(), boot() (+20 more)

### Community 28 - "icon"
Cohesion: 0.19
Nodes (12): ADR-0001: Stack técnico, grammers (MTProto en Rust), iced (UI en Rust, wgpu + tiny-skia), keyring (claves y sesión), rusqlite bundled con migraciones por PRAGMA user_version, tc-secret: chats secretos sans-IO con RustCrypto, tc-stickers: WebP, TGS con rlottie, WebM aplazado, ADR-0004: Stickers y chats secretos en el alcance (+4 more)

### Community 29 - "NotifyMsg"
Cohesion: 0.08
Nodes (21): DndChoice, .ALL, EightHours, Hour, Off, On, NotifyMsg, Dnd (+13 more)

### Community 30 - "tc-ui/src/tests.rs"
Cohesion: 0.24
Nodes (12): ADR-0003: Arquitectura del cliente, Orden de construcción M1.1-M1.4, M2.1, M2.7, CachedBackend (caché como decorador, tc-store), Trait ChatBackend (sub-traits + events()), Modelo de concurrencia (tokio, hilo SQLite, canal acotado, Resync), MockBackend determinista, SQLite por cuenta (17 tablas + FTS5, WAL, chat_key), Scroll virtual propio (Fenwick, ventana ~600 mensajes) (+4 more)

### Community 31 - "EntityKind"
Cohesion: 0.24
Nodes (13): Registro de agentes, Agente arquitecto (opus/high), Agente core+mock (sonnet/medium), Agente QA (pendiente), Agente theme (sonnet/medium), Agente UI (opus/high), Bitácora (append-only), Bug iced 0.14: scrollable.direction antes de anchor_bottom (+5 more)

### Community 32 - "tc-mock/src/forum.rs"
Cohesion: 0.10
Nodes (17): el_foro_tiene_temas_con_general_y_no_leidos_que_suman(), entrante_en_tema_crear_y_fijar(), FORUM_CHAT, FORUM_GROUP, Forums, historial_envio_y_lectura_por_tema(), Inner, mock() (+9 more)

### Community 34 - "TcSession"
Cohesion: 0.07
Nodes (13): changes_mark_dirty_and_reset_forgets_everything(), debug_never_prints_the_auth_key(), MemorySessionStore, rejects_garbage_and_unknown_versions(), roundtrip_keeps_keys_peers_and_state(), sample(), SESSION_FORMAT, SessionError (+5 more)

### Community 35 - "MsgLayout"
Cohesion: 0.21
Nodes (11): Índice maestro de teleClon, architecture.md (crates, ChatBackend, SQLite, F01-F18), graphify-out (graph.html, GRAPH_REPORT.md, graph.json), Reglas de trabajo (log, ADR, clean-room, credenciales, regenerar grafo), Credenciales api_id/api_hash en .env local gitignored, graphify: instalación y uso, graphifyy en ./.venv local, Registro de skills usadas (+3 more)

### Community 36 - "send_dialog.rs"
Cohesion: 0.15
Nodes (21): Upload, BAR_H, big_preview(), cancel_button(), compress_choice(), dialog(), dialog_notes(), DIALOG_W (+13 more)

### Community 37 - "dh.rs"
Cohesion: 0.07
Nodes (26): dh_matches_independent_vector(), DhParams, DhSecret, exp(), FP, GA, GB, generate_mixes_server_random_and_rejects_bad_peer() (+18 more)

### Community 38 - "tc-ui/src/gifs.rs"
Cohesion: 0.09
Nodes (25): demo_script(), ANIM_BUDGET, ANIM_PX, CACHE_BYTES, CELL_GAP, decode(), decode_gif(), decodifica_escala_y_recorre_los_fotogramas() (+17 more)

### Community 39 - "app.rs"
Cohesion: 0.04
Nodes (56): ThemeKind, Dark, Light, Auth, Backend, Connector, DemoStep, Do (+48 more)

### Community 40 - "with"
Cohesion: 0.20
Nodes (17): steps, steps, steps, steps, id, if, name, run (+9 more)

### Community 41 - "App"
Cohesion: 0.13
Nodes (7): App, focus_window(), parse_key(), png_of(), Target, Chat, Topic

### Community 42 - "tc-ui/src/stickers.rs"
Cohesion: 0.12
Nodes (32): StickerSetId, Sticker, StickerSet, SET_ID, plays(), ANIMATED_SET_ID, art_index(), ART_PX (+24 more)

### Community 43 - "tc-telegram/src/auth.rs"
Cohesion: 0.08
Nodes (14): AuthInner, code_kind(), code_types_map(), GrammersBackend, Inner, migrate_dc(), next_kind(), phone_validation() (+6 more)

### Community 45 - "Entropy"
Cohesion: 0.10
Nodes (21): Role, Initiator, Responder, Entropy, Message, Abort, nonzero_random_id(), Output (+13 more)

### Community 46 - "e2e.rs"
Cohesion: 0.17
Nodes (35): DecryptedMessageLayer, chat_n(), concurrent_rekey_converges_on_one_instance(), decreasing_in_seq_aborts_chat(), dh(), discard_local_and_remote(), duplicates_are_dropped(), fingerprint_mismatch_discards_chat() (+27 more)

### Community 47 - "build_items"
Cohesion: 0.31
Nodes (9): replica/features.csv, Rasgos de marca (avatares cuadrado redondeado, burbujas sin cola, fondo de isolíneas), Agente recon (sonnet/medium), Recon map: Telegram Desktop (Linux), Componentes de UI (Burbuja de mensaje, Fila de chat, Compositor...), Core loop (chats, enviar/recibir, sincronizado), Matriz de funciones (features.csv, must/should/could/skip), Flujos F01-F16 (+1 more)

### Community 48 - "TelegramConfig"
Cohesion: 0.12
Nodes (22): Choice, Mock, Telegram, choose(), FAKE_HASH, missing_credentials(), por_defecto_el_mock_sin_mirar_credenciales(), sin_credenciales_vuelve_al_mock_con_aviso_que_no_ensena_valores() (+14 more)

### Community 49 - "ADR-0003: Arquitectura del cliente (workspace, backend tras trait, caché SQLite, E2E sans-IO)"
Cohesion: 0.35
Nodes (12): tc-app, tc-audio, tc-core, tc-mock, tc-notify, tc-secret, tc-stickers, tc-store (+4 more)

### Community 50 - "mp4.rs"
Cohesion: 0.18
Nodes (29): VideoError, Container, Decode, NoVideo, Unsupported, annex_b(), annex_b_pone_codigos_de_inicio(), bad() (+21 more)

### Community 52 - "crypto.rs"
Cohesion: 0.08
Nodes (28): decrypt(), derive(), encrypt(), encrypt_with_padding(), HEADER_LEN, ige_decrypt(), ige_encrypt(), IgeDec (+20 more)

### Community 53 - "tc-telegram/src/lib.rs"
Cohesion: 0.06
Nodes (15): Feed, None, Pool, Source, GrammersBackend, has_login_token(), Inner, persister() (+7 more)

### Community 54 - "Cache"
Cohesion: 0.06
Nodes (27): dialog_from_tl(), dialog_stub(), DialogCtx, draft_date(), draft_from_tl(), is_muted(), message_from_tl(), reactions_from_tl() (+19 more)

### Community 55 - "dialogs.rs"
Cohesion: 0.06
Nodes (24): DIALOG_COLS, dialog_from_row(), draft_from_row(), drafts_set_clear_list(), list_orders_pinned_then_activity_and_filters_archived(), mark_read_and_missing_dialog_errors(), notify_scopes(), scope_parts() (+16 more)

### Community 56 - "tc-stickers/src/lib.rs"
Cohesion: 0.67
Nodes (3): Disposición (raíl, lista, conversación, panel info), Lista de chats (componente), Barra lateral (raíl de carpetas)

### Community 59 - "GrammersBackend"
Cohesion: 0.11
Nodes (5): GrammersBackend, in_topic(), Inner, topic_from_tl(), topic_of()

### Community 60 - "tc-telegram/src/media.rs"
Cohesion: 0.10
Nodes (12): BIG_FILE, input_media(), is_file_reference_error(), MAX_DOWNLOADS, MAX_FILE, MAX_IN_MEMORY, MAX_PARTS, MAX_RESUMES (+4 more)

### Community 61 - "tc-store/src/lib.rs"
Cohesion: 0.10
Nodes (21): account_db_path(), APP_DIR, DATA_DIR_NAME, default_data_dir(), LEGACY_APP_DIR, MIGRATIONS, resolve_data_dir(), SCHEMA_VERSION (+13 more)

### Community 64 - "crate::Inner"
Cohesion: 0.15
Nodes (5): crate::Inner, GIF_BOT, gif_from_document(), GifCatalog, input_doc()

### Community 65 - "pack.rs"
Cohesion: 0.11
Nodes (26): Def, DEFS, dots(), index_of(), Layer, len(), NATIVE_SIZE, P (+18 more)

### Community 66 - "ChatKey"
Cohesion: 0.05
Nodes (33): Action, AppInfo, Capabilities, ChatKey, event_channel(), EventSender, Notification, Notifier (+25 more)

### Community 67 - "conversation.rs"
Cohesion: 0.15
Nodes (30): MsgLayout, GroupPos, First, Last, Middle, Single, body(), bubble() (+22 more)

### Community 68 - "restored"
Cohesion: 0.10
Nodes (45): authorization(), state(), updates(), user(), user_enum(), dialogs_page(), dialogs_paginate_with_server_offsets(), edit_delete_react_and_read() (+37 more)

### Community 69 - "mapping.rs"
Cohesion: 0.09
Nodes (42): key_visualization_matches_independent_vector(), apply_batch(), channel(), chat(), dialog(), hex(), message(), msg_enum() (+34 more)

### Community 70 - "media_cache.rs"
Cohesion: 0.12
Nodes (15): account_cache_lives_next_to_the_database_and_survives_reopen(), check_key(), create_private_dir(), fetch_cached(), fetch_cached_goes_to_the_network_once_and_never_for_secrets(), io(), keys_depend_on_id_and_size_and_skip_secrets(), media_key() (+7 more)

### Community 71 - "view/audio.rs"
Cohesion: 0.08
Nodes (22): TrackView, handle(), HANDLES, icon(), icon_fill(), icon_turned(), icons!, TURNED (+14 more)

### Community 72 - "secret.rs"
Cohesion: 0.11
Nodes (18): chat(), CHAT_COLS, chat_from_row(), chat_roundtrip_and_fk(), messages_idempotent_blob_roundtrip_and_page(), msg(), MSG_COLS, msg_from_row() (+10 more)

### Community 73 - "messages.rs"
Cohesion: 0.16
Nodes (14): COLS, fill(), fts_query(), fts_search_accents_prefix_scope_and_delete(), ids(), media_msg(), pagination_by_anchor(), pending_becomes_confirmed_by_random_id() (+6 more)

### Community 74 - "FilesMsg"
Cohesion: 0.09
Nodes (18): App, FilesMsg, Cancel, Caption, Close, Compress, Inspected, Paste (+10 more)

### Community 75 - "tl.rs"
Cohesion: 0.05
Nodes (53): Action, AbortKey, AcceptKey, CommitKey, DeleteMessages, FlushHistory, Noop, NotifyLayer (+45 more)

### Community 76 - "rules.rs"
Cohesion: 0.09
Nodes (62): ACTION_MARK_READ, ajustes_por_tipo(), APP_NAME, archivado_no_silenciado_depende_del_ajuste(), archivado_y_silenciado_nunca_avisa(), avisa_con_foco_si_el_chat_no_es_el_abierto(), avisa_sin_foco_aunque_el_chat_este_abierto(), away() (+54 more)

### Community 77 - "decode.rs"
Cohesion: 0.10
Nodes (16): check_tone(), copy_out(), decode_all(), decodes_music_formats(), decodes_stereo_opus(), decodes_telegram_style_voice_note(), fixture(), freq() (+8 more)

### Community 78 - "tests_audio.rs"
Cohesion: 0.19
Nodes (17): AudioBytes, TrackState, Failed, Idle, Loading, Paused, Playing, boot() (+9 more)

### Community 79 - "tc-ui/src/audio.rs"
Cohesion: 0.06
Nodes (31): PlaybackState, Ended, Failed, Paused, Playing, Starting, SPEEDS, App (+23 more)

### Community 80 - "tc-telegram/src/media.rs"
Cohesion: 0.13
Nodes (9): Cancel, check_dest(), crate::Inner, GrammersBackend, known_total(), LocalFile, media_location(), Progress (+1 more)

### Community 81 - "CachedBackend<B>"
Cohesion: 0.08
Nodes (3): CachedBackend<B>, is_transient(), Shared<B>

### Community 82 - "Store"
Cohesion: 0.13
Nodes (13): favorites(), RECENT_CAP, recents_order_dedupe_cap_and_cascade(), seed(), SET_COLS, set_from_row(), sets_and_stickers(), STICKER_COLS (+5 more)

### Community 83 - "reading.rs"
Cohesion: 0.11
Nodes (20): Message, CONV_SCROLL_ID, App, desplazamiento_deja_la_marca_arriba_con_contexto(), first_unread(), FLUSH_EVERY, from_bottom_for(), height_after() (+12 more)

### Community 84 - "tc-secret/src/session.rs"
Cohesion: 0.04
Nodes (46): &mut E, OsEntropy, random_i64(), random_up_to(), Result, SecretError, Entropy, FingerprintMismatch (+38 more)

### Community 85 - "EntityKind"
Cohesion: 0.11
Nodes (17): EntityKind, Blockquote, Bold, BotCommand, Code, CustomEmoji, Email, Hashtag (+9 more)

### Community 86 - "SessionSnapshot"
Cohesion: 0.08
Nodes (19): Queued, MAGIC, Rd, Rd<'a>, read_ready(), RK_ACCEPTED, RK_COMMITTED, RK_IDLE (+11 more)

### Community 87 - "view/login.rs"
Cohesion: 0.27
Nodes (22): card_screen(), CARD_W, code(), connecting(), divider(), field(), field_error(), ghost() (+14 more)

### Community 88 - "LoginMsg"
Cohesion: 0.12
Nodes (19): LoginMsg, CodeChanged, CodeSent, PasswordChanged, PasswordChecked, PhoneChanged, QrReady, Resend (+11 more)

### Community 89 - "tests_files.rs"
Cohesion: 0.25
Nodes (19): boot(), cancelar_a_mitad_quita_la_burbuja_sin_marcar_fallo(), elegir_archivos_abre_el_dialogo_y_cerrar_devuelve_el_texto(), en_un_tema_del_foro_va_al_tema_y_respeta_el_tema_cerrado(), enter_envia_y_los_atajos_esperan_con_el_dialogo_abierto(), envio_con_progreso_y_confirmacion(), fallo_y_reintento(), file() (+11 more)

### Community 90 - "tc-mock/src/lib.rs"
Cohesion: 0.06
Nodes (27): ANIMATED_SET_ID, animated_sticker_set(), BIG_CHAT, BIG_CHAT_LEN, build(), CHANNEL_POSTS, gen_channel(), gen_history() (+19 more)

### Community 91 - "main.rs"
Cohesion: 0.06
Nodes (25): asignaciones_comillas_y_comentarios(), el_entorno_manda_sobre_el_archivo_salvo_si_esta_vacio(), EnvFile, lookup(), unquote(), account_id(), env_flag(), env_u64() (+17 more)

### Community 92 - "make_logo.py"
Cohesion: 0.06
Nodes (26): classify(), main(), report(), scan(), build(), enc(), in_gamut(), lum() (+18 more)

### Community 94 - "super"
Cohesion: 0.06
Nodes (19): Store, unread_mark_and_clear_history(), Error, Invalid, Io, NoDataDir, NotFound, SchemaTooNew (+11 more)

### Community 95 - "cached_tests.rs"
Cohesion: 0.10
Nodes (32): all(), cache_persists_across_restarts_on_disk(), cached(), chats_secretos_se_buscan_en_el_backend_sin_tocar_la_base(), cold_cache_offline_is_an_error_and_pagination_works(), con_el_historial_completo_se_responde_en_local_sin_preguntar_al_servidor(), dialogs_roundtrip_exactly_and_survive_offline(), downloads_are_cached_on_disk_but_never_for_secret_chats() (+24 more)

### Community 96 - "tc-mock/src/dialog_actions.rs"
Cohesion: 0.14
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
Cohesion: 0.20
Nodes (10): backend(), decode(), Rd, Rd<'a>, VaultSessionStore, file_store_is_private_and_atomic(), FileSessionStore, StoreError (+2 more)

### Community 104 - "BackendEvent"
Cohesion: 0.04
Nodes (50): BackendError, Cancelled, Crypto, FloodWait, Forbidden, Internal, InvalidInput, Network (+42 more)

### Community 105 - "sidebar.rs"
Cohesion: 0.15
Nodes (16): LIST_SCROLL_ID, SEARCH_ID, badge_at(), folder_icon(), LINE_H, list(), rail(), rail_button() (+8 more)

### Community 106 - "tests/secret.rs"
Cohesion: 0.12
Nodes (21): UpdateBatch, ANA, BETO, ChanSource, CHAT, fingerprint(), handler(), handshake() (+13 more)

### Community 107 - "StickerError"
Cohesion: 0.07
Nodes (31): render_batch(), StickerError, BadSize, FrameOutOfRange, Gzip, Image, LottieDisabled, LottieLoad (+23 more)

### Community 108 - "sidebar.rs"
Cohesion: 0.05
Nodes (40): AudioInfo, Media, from_row(), from_row(), animates(), CELL_H, CELL_MIN_W, CELL_ART (+32 more)

### Community 109 - "file_name.rs"
Cohesion: 0.07
Nodes (37): assert_safe(), bidi_control(), DEFAULT_FILE_NAME, examples(), forbidden(), is_reserved(), long_names_keep_their_extension_and_char_boundaries(), MAX_EXT_BYTES (+29 more)

### Community 110 - "UserId"
Cohesion: 0.10
Nodes (15): AuthApi, AuthState, Checking, LoggedIn, LoggedOut, SignUpRequired, WaitingCode, WaitingPassword (+7 more)

### Community 111 - "tests/snapshot.rs"
Cohesion: 0.16
Nodes (14): StateKind, Discarded, Incoming, Ready, Requested, SeededEntropy, corrupt_or_foreign_snapshots_are_rejected(), dh() (+6 more)

### Community 112 - "folders.rs"
Cohesion: 0.19
Nodes (8): COLS, folder(), folder_peers_replace_and_cascade(), folders_crud_order_and_reorder(), from_row(), Store, FolderPeerRow, FolderRow

### Community 113 - "files.rs"
Cohesion: 0.17
Nodes (7): cache_index_orders_by_last_use(), CacheEntry, media_refs_follow_their_media(), MediaRefRow, peer_photos_roundtrip_and_never_for_secrets(), photo_kind(), Store

### Community 114 - "tc-mock/src/search.rs"
Cohesion: 0.24
Nodes (10): filtros_por_tipo(), mock(), paginacion_sin_repetir_y_borrado(), peer(), q(), say(), search(), sin_red_falla() (+2 more)

### Community 115 - "sticker_anim.rs"
Cohesion: 0.10
Nodes (21): Playback, Tgs, carga_una_vez_y_reintenta_tras_fallo(), DATA_CACHE_BYTES, FRAME_CACHE_BYTES, FULL, Job, Load (+13 more)

### Community 116 - "tl_visual_ttl.rs"
Cohesion: 0.07
Nodes (9): SelfDestructTimer, EMOJI, GRID_SIDE, KeyVisualization, VISUAL_LEN, layer(), tl_known_encoding(), tl_rejects_unknown_and_trailing() (+1 more)

### Community 117 - "Message"
Cohesion: 0.08
Nodes (22): ForumApi, ThreadRef, MsgId, RandomId, Anchor, After, Around, Before (+14 more)

### Community 118 - "GifAnims"
Cohesion: 0.20
Nodes (6): GifAnims, la_cache_de_animaciones_respeta_el_tope(), Slot, Failed, Loading, Ready

### Community 119 - "jobs"
Cohesion: 0.18
Nodes (10): env, CARGO_INCREMENTAL, CARGO_TERM_COLOR, RUST_TOOLCHAIN, jobs, linux, name, needs (+2 more)

### Community 120 - "reactions.rs"
Cohesion: 0.29
Nodes (7): message_exists(), my_reaction_moves_between_emojis(), r(), set_get_replace_and_unknown_message(), setup(), Store, ReactionRow

### Community 121 - "SearchQuery"
Cohesion: 0.08
Nodes (24): EntityKind, Bold, Code, Hashtag, Italic, Mention, Pre, Spoiler (+16 more)

### Community 123 - "Marca propia — Parlo"
Cohesion: 0.07
Nodes (26): ADR-0006: Empaquetado multiplataforma (Windows, macOS, Linux), Consecuencias, Decisiones, 1. Criterios de nombre, 2. Candidatos (6-8), 3. Chequeos que debe hacer el usuario, 4. Voz y tono (español), 5. Brief de logo (+18 more)

### Community 124 - "tc-core/src/lib.rs"
Cohesion: 0.08
Nodes (38): el_tema_como_dialogo_conserva_lectura_y_no_leidos(), fijados_primero_y_luego_por_actividad(), ForumTopic, GENERAL_TOPIC, msg(), sort_topics(), TOPIC_COLORS, TopicIcon (+30 more)

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
Nodes (12): App, apply_mine(), reaction_error_text(), account_stickers_load_in_the_picker_and_in_bubbles(), boot(), decoded(), my_reaction_moves_like_the_server_does(), net_sticker() (+4 more)

### Community 131 - "app/downloads.rs"
Cohesion: 0.13
Nodes (13): default_dir(), DIR_KEY, DlState, Cancelling, Done, Running, Starting, launch() (+5 more)

### Community 133 - ".remember_docs"
Cohesion: 0.11
Nodes (7): filter_to_tl(), total_of(), CachedSet, crate::Inner, MAX_SETS, set_of(), StickerCatalog

### Community 134 - "AudioError"
Cohesion: 0.10
Nodes (18): AudioStream, open(), OpusStream, sym_err(), AudioError, Corrupt, NoDevice, Unsupported (+10 more)

### Community 135 - "ImageKey"
Cohesion: 0.11
Nodes (12): cache_tracks_loads_failures_and_evicts_least_recently_used(), ImageKey, Avatar, Full, Sticker, Thumb, Images, img() (+4 more)

### Community 136 - "Track"
Cohesion: 0.07
Nodes (6): FakeOutput, NullOutput, Output, RodioOutput, run(), Track

### Community 138 - "Tone"
Cohesion: 0.22
Nodes (8): Tone, .ALL, Dark, Default, Light, Medium, MediumDark, MediumLight

### Community 139 - "stores.rs"
Cohesion: 0.10
Nodes (26): aad(), AAD_PREFIX, dh(), disk_bytes(), encode(), existing_session_json_is_migrated_once_and_deleted(), FIXED_LEN, media_cache() (+18 more)

### Community 140 - "PanelMsg"
Cohesion: 0.17
Nodes (12): PanelMsg, Category, Close, EmojiQuery, EmojiSubmit, Insert, Recorded, Tab (+4 more)

### Community 141 - "windows"
Cohesion: 0.29
Nodes (7): WINDOWS_CERT_PASSWORD, WINDOWS_CERT_PFX_BASE64, windows, env, name, needs, runs-on

### Community 142 - ".sticker_send_task"
Cohesion: 0.13
Nodes (8): ActiveTgs, App, AnimMsg, Frames, Hover, Loaded, Tick, Unhover

### Community 143 - "Plan de pruebas Parlo (replica-test, app de escritorio Rust)"
Cohesion: 0.10
Nodes (19): F01 Login con teléfono, F02 Login por QR, F03 Abrir chat y leer, F04 Enviar texto, F05 Responder, F06 Reenviar, F07 Editar, F08 Borrar (+11 more)

### Community 144 - "emoji.rs"
Cohesion: 0.15
Nodes (17): apply(), busqueda_en_espanol_ingles_y_por_grupo(), catalog_len(), display_name(), el_catalogo_es_completo_y_por_categorias(), es_index(), FREQUENT_SHOWN, list() (+9 more)

### Community 145 - "meta"
Cohesion: 0.40
Nodes (5): meta, name, outputs, runs-on, version

### Community 147 - "sticker_img.rs"
Cohesion: 0.05
Nodes (41): MediaKind, Audio, File, Gif, Photo, Video, VideoNote, Voice (+33 more)

### Community 149 - "linux.rs"
Cohesion: 0.25
Nodes (8): aviso_real(), DbusNotifier, DEFAULT_ACTION, DEST, Ids, ids_retira_solo_si_coincide(), lock(), PATH

### Community 150 - "tc-store/src/convert.rs"
Cohesion: 0.07
Nodes (48): voice_notes_and_music_survive_the_cache(), apply_dialogs(), apply_history(), cached_dialogs(), cached_history(), decode_entities(), decode_sticker(), decode_sticker_file() (+40 more)

### Community 151 - "DlMsg"
Cohesion: 0.13
Nodes (13): DlMsg, Cancel, ChooseDir, DirPicked, Open, ResetDir, Reveal, SaveAs (+5 more)

### Community 152 - "DownloadError"
Cohesion: 0.27
Nodes (6): DownloadError, Backend, FileReference, PartReader, storage(), too_large()

### Community 153 - "GifMsg"
Cohesion: 0.20
Nodes (10): GifMsg, AnimLoaded, More, Query, Results, Retry, Saved, Scrolled (+2 more)

### Community 154 - "vault.rs"
Cohesion: 0.10
Nodes (27): DeadVault, DeadVault, check_key(), CHUNK_HEADER_LEN, CHUNK_MAGIC, chunked_vault_reports_missing_parts_and_keeps_magic_values_intact(), chunked_vault_splits_large_values_and_cleans_up(), clear_session() (+19 more)

### Community 156 - "src/login.rs"
Cohesion: 0.17
Nodes (12): clean_code(), CODE_ID, code_where(), codigo_solo_cifras_y_recortado(), countdown(), cuentas_atras(), errores_con_texto_propio(), login_error() (+4 more)

### Community 158 - "tests_panel.rs"
Cohesion: 0.49
Nodes (9): boot_in_chat(), emoji_en_el_cursor_recientes_busqueda_y_tono(), gifs_guardados_busqueda_envio_y_reintento(), gm(), los_gif_reales_se_animan_con_el_reloj_compartido(), NOW, pestanas_atajos_y_cierre(), pm() (+1 more)

### Community 161 - "mux"
Cohesion: 0.40
Nodes (9): archivos_rotos_dan_error_sin_panico(), sample(), encode_mp4(), fbox(), full(), mux(), MuxSample, split_annex_b() (+1 more)

### Community 162 - "cached.rs"
Cohesion: 0.04
Nodes (42): ProfileApi, SecretChatsApi, SecretChatId, UserId, DEFAULT_REACTIONS, Dialog, DialogsPage, display_name_joins_parts() (+34 more)

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
Cohesion: 0.11
Nodes (18): Event, Deleted, Discarded, Duplicate, HistoryFlushed, KeyChanged, PeerLayer, PeerLayerNewer (+10 more)

### Community 169 - ".update_inner"
Cohesion: 0.05
Nodes (18): BubbleOpacity, BubbleY, App, Reply, App, ForumUi, topic_error_text(), Edit (+10 more)

### Community 170 - "app/notify.rs"
Cohesion: 0.07
Nodes (26): DoNotDisturb, Off, On, Until, Engine, NotifySettings, Preview, Full (+18 more)

### Community 171 - "f"
Cohesion: 0.13
Nodes (24): AMPLITUDE, cell_segments(), ContourParams, .DEFAULT, .SECRET, contours(), field(), lattice() (+16 more)

### Community 172 - "reply_to"
Cohesion: 0.40
Nodes (3): reply_to(), current_topic(), topic_reply_to()

### Community 173 - "Suppress"
Cohesion: 0.20
Nodes (10): Suppress, Archived, ChatOpen, ContactJoined, Disabled, DoNotDisturb, KindDisabled, Muted (+2 more)

### Community 174 - "tests_qa.rs"
Cohesion: 0.24
Nodes (14): error_text(), boot_open(), f04_doble_enter_no_duplica_el_mensaje(), f04_reintento_conserva_random_id_y_no_duplica_en_el_backend(), f04_texto_con_emoji_acentos_y_saltos_se_envia_integro(), f04_texto_de_mas_de_4096_avisa_y_no_deja_burbuja(), f04_texto_vacio_o_en_blanco_no_se_envia(), f05_borrar_el_citado_cancela_la_respuesta() (+6 more)

### Community 175 - "Step"
Cohesion: 0.33
Nodes (6): Step, Checking, Code, Password, Phone, SignUp

### Community 183 - "tests/media.rs"
Cohesion: 0.08
Nodes (32): document_file_ref(), entities_from_tl(), entities_to_tl(), entity_parts(), forward_name(), media_from_document(), media_from_tl(), photo_dims() (+24 more)

### Community 184 - "tests/search.rs"
Cohesion: 0.25
Nodes (12): busca_en_el_chat_con_texto_offset_y_limite_e_ingiere_usuarios(), cada_filtro_usa_su_input_messages_filter(), CH, chat_secreto_no_soportado_y_error_de_red_se_propaga(), en_un_tema_va_top_msg_id_y_el_scope_lo_aplica(), filter_id(), hit(), pagina_llena_da_next_y_la_ultima_no() (+4 more)

### Community 185 - "Item"
Cohesion: 0.29
Nodes (5): Item, Date, Edge, Msg, UnreadMark

### Community 186 - ".toast"
Cohesion: 0.19
Nodes (5): App, Dl, Failed, download_error_text(), Downloads

### Community 187 - "Field"
Cohesion: 0.29
Nodes (5): Field, Code, Password, Phone, Qr

### Community 188 - "Media"
Cohesion: 0.60
Nodes (6): dest_in(), download_name(), Media, el_nombre_hostil_se_sanea_y_nunca_sale_de_la_carpeta(), hostile(), nombre_repetido_recibe_sufijo()

### Community 189 - "TextStyle"
Cohesion: 0.16
Nodes (12): BODY, CAPTION, CODE, DISPLAY, EMOJI_ONLY, HEADING, META, TextStyle (+4 more)

### Community 191 - "tests_chat_actions.rs"
Cohesion: 0.27
Nodes (15): action_error_text(), a_chat_never_appears_twice_after_reloading_the_lists(), a_user(), archive_is_optimistic_reverts_on_error_and_can_be_undone(), boot(), chat_menu_items_depend_on_the_kind_of_chat(), chat_msg(), delete_key_asks_then_removes_and_restores_on_error() (+7 more)

### Community 192 - "StickerId"
Cohesion: 0.22
Nodes (5): StickerId, DEFAULT_CACHE_BYTES, DEFAULT_RECENTS, Favorites, Recents

### Community 194 - "Informe de emprendedor: qué molesta, qué falta y qué plan de mejoras para Parlo"
Cohesion: 0.20
Nodes (9): 0. Muestra y límites (leer primero), 1. Lo que MÁS molesta (quejas sobre lo que el original hace), 2. Lo que FALTA (peticiones por nombre), 3. Lo NO resuelto (trabajos o grupos que el original ignora), 4. Cruce con replica/diff-report.md y replica/features.csv, 5. Plan de mejoras priorizado, 6. Ángulo de venta (documentado, NO perseguido: uso personal), 7. Siguientes pasos (+1 more)

### Community 202 - "tc-mock/src/gifs.rs"
Cohesion: 0.12
Nodes (17): by_id(), download(), encode_gif(), encode_png(), frame(), FRAMES, GIF_BASE, guardados_busqueda_y_bytes() (+9 more)

### Community 203 - "FileClass"
Cohesion: 0.40
Nodes (5): FileClass, Audio, Image, Other, Video

### Community 204 - "tests_downloads.rs"
Cohesion: 0.18
Nodes (29): default_dir_in(), is_writable_dir(), boot(), cancelar_a_mitad_deja_la_tarjeta_como_antes_y_sin_parcial(), cancelar_antes_de_conocer_el_id_se_cancela_al_llegar(), carpeta_elegida_borrada_avisa_y_vuelve_a_la_predeterminada(), carpeta_elegida_se_usa_se_persiste_y_se_restablece(), clic_descarga_con_progreso_y_termina_en_abrir() (+21 more)

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

### Community 220 - "layout_message"
Cohesion: 0.19
Nodes (9): emoji_only(), input(), layout_message(), LayoutCache, LayoutInput, maquetacion_meta_en_linea_o_debajo(), meta_width(), runs() (+1 more)

### Community 221 - "Category"
Cohesion: 0.14
Nodes (12): Category, Activities, .ALL, Animals, Flags, Food, Objects, People (+4 more)

### Community 222 - "windows.rs"
Cohesion: 0.18
Nodes (15): bytes_a_cache_estable(), escapa_marcado(), escape_xml(), extension(), icon_path(), ruta_tal_cual(), Icon, Bytes (+7 more)

### Community 223 - "anim_tests.rs"
Cohesion: 0.14
Nodes (21): ANIM_PX, frame_at(), frame_step(), info_of(), MAX_FPS, next_frames(), Show, Frame (+13 more)

### Community 224 - "Body"
Cohesion: 0.50
Nodes (4): Body, Bubble, Emoji, Sticker

### Community 225 - "cached_actions_tests.rs"
Cohesion: 0.23
Nodes (9): MockConfig, a_user_chat(), all(), cached(), clear_and_delete_reach_the_cache(), offline_list(), removal_events_are_persisted(), unread_mark_read_mute_and_pin_persist() (+1 more)

### Community 226 - "tc_core"
Cohesion: 0.12
Nodes (13): history_params(), MAX_TEXT, random_i64(), MAX_DELETE_ROUNDS, flatten_updates(), channel_of(), CHANNEL_OFFSET, decode_group() (+5 more)

### Community 234 - "Login"
Cohesion: 0.31
Nodes (3): left(), Login, un_logged_out_repetido_no_borra_lo_escrito()

### Community 235 - "CodeKind"
Cohesion: 0.22
Nodes (9): CodeKind, App, Call, Email, FlashCall, MissedCall, Other, Sms (+1 more)

### Community 237 - "context_menu.rs"
Cohesion: 0.06
Nodes (39): ChatActions, EDIT_WINDOW_SECS, ForwardPicker, InfoView, Menu, MenuTarget, message_link(), Undo (+31 more)

### Community 238 - "PanelTab"
Cohesion: 0.13
Nodes (11): GifPanel, Search, EMOJI_SEARCH_ID, EmojiPanel, GIF_SEARCH_ID, Panel, PanelTab, .ALL (+3 more)

### Community 239 - "arc"
Cohesion: 0.05
Nodes (34): window_settings(), window_size(), CacheKey, FrameCache, capturas_de_la_descarga(), settings(), shot(), capturas_de_la_busqueda() (+26 more)

### Community 240 - "ADR-0012: Búsqueda dentro de un chat"
Cohesion: 0.50
Nodes (3): ADR-0012: Búsqueda dentro de un chat, Consecuencias, Decisiones

### Community 241 - "tc-mock/src/audio.rs"
Cohesion: 0.46
Nodes (7): BARS, deterministic_and_well_formed(), mix(), RATE, voice_wav(), wav_len(), waveform()

### Community 243 - "ADR-0008: Stickers animados (TGS con rlottie; WebM estático)"
Cohesion: 0.33
Nodes (5): ADR-0008: Stickers animados (TGS con rlottie; WebM estático), Consecuencias, Decisión, TGS (Lottie con gzip): animado con rlottie, por defecto solo en Linux, WebM (vídeo VP9 con alfa): queda estático (miniatura)

### Community 244 - "tests/mod.rs"
Cohesion: 0.12
Nodes (18): channel_group(), chat_group(), affected_history(), affected_messages(), block_pin_message_and_info_errors(), clear_and_delete_private_chat_repeat_until_done(), is_user(), leave_and_delete_groups_and_channels() (+10 more)

### Community 247 - "ADR-0009: Teclado de emojis"
Cohesion: 0.50
Nodes (3): ADR-0009: Teclado de emojis, Consecuencias, Decisiones

### Community 248 - "ADR-0010: GIFs (guardados, búsqueda, envío y animación)"
Cohesion: 0.50
Nodes (3): ADR-0010: GIFs (guardados, búsqueda, envío y animación), Consecuencias, Decisiones

### Community 275 - "tests_forum.rs"
Cohesion: 0.16
Nodes (13): Carry, initial_anchor(), abrir_un_foro_muestra_la_lista_de_temas(), abrir_un_tema_en_su_primer_no_leido_y_volver(), boot(), busy(), crear_un_tema_lo_abre(), enviar_dentro_del_tema_y_eventos_de_otros_temas() (+5 more)

### Community 285 - "VaultError"
Cohesion: 0.36
Nodes (5): ChunkedVault<V>, map_err(), VaultError, Backend, Unavailable

### Community 288 - "Plan de acción: los 5 primeros faltantes del informe de paridad"
Cohesion: 0.22
Nodes (8): A. Envío, descarga y guardado de archivos, B. Búsqueda dentro de un chat, C. Notificaciones de escritorio, Fuera de alcance por ahora, Los 5 puntos y dónde se resuelven, Orden y dependencias, Plan de acción: los 5 primeros faltantes del informe de paridad, Reparto sugerido

### Community 304 - "Informe de paridad: Parlo frente al cliente original"
Cohesion: 0.25
Nodes (7): Diferencias de comportamiento (paso 3 de la skill), Filas «partial» por no estar probadas contra Telegram real (`[sin real]`): 33, Informe de paridad: Parlo frente al cliente original, Los 10 faltantes más importantes (orden de construcción, solo must-haves en `no`), Puntuación por área (la más débil primero), Resumen, Siguientes pasos

### Community 305 - "ADR-0011: Transferencias (subidas y descargas a disco con progreso y cancelación)"
Cohesion: 0.50
Nodes (3): ADR-0011: Transferencias (subidas y descargas a disco con progreso y cancelación), Consecuencias, Decisiones

### Community 306 - "tc-ui/src/media.rs"
Cohesion: 0.15
Nodes (19): AVATAR_PX, avatars_are_square_and_downscaled(), bubble_aspect(), BUBBLE_ASPECT, bubble_photos_are_cropped_to_the_layout_ratio(), BUBBLE_PX, CACHE_BYTES, decode_avatar() (+11 more)

### Community 309 - "Nav"
Cohesion: 0.15
Nodes (10): la_rejilla_se_recorre_con_flechas_sin_salirse(), Nav, Activate, Down, Left, NextTab, PrevTab, Right (+2 more)

### Community 310 - "Bugs (replica-test, 2026-11-14)"
Cohesion: 0.40
Nodes (4): B-01 (S3) Texto de más de 4096 caracteres: burbuja condenada y aviso equivocado, Bugs (replica-test, 2026-11-14), Por comprobar (no reproducido), Por severidad

### Community 326 - "Backend"
Cohesion: 0.11
Nodes (24): Overlay, Delete, DownloadDir, Key, NewSecret, Notifications, Photo, Ttl (+16 more)

### Community 335 - "tc-ui/src/anim.rs"
Cohesion: 0.06
Nodes (29): ByteLru, ByteLru<K, V>, HIGHLIGHT_FADE_MS, MotionPreference, Full, Reduced, Property, Opacity (+21 more)

### Community 368 - "ADR-0013: Notificaciones de escritorio (crate `tc-notify`)"
Cohesion: 0.40
Nodes (4): ADR-0013: Notificaciones de escritorio (crate `tc-notify`), Consecuencias, Decisiones, Verificación (2026-11-14)

## Knowledge Gaps
- **631 isolated node(s):** `First`, `Last`, `Middle`, `Single`, `Close` (+626 more)
  These have ≤1 connection - possible missing edges or undocumented components. (Counts symbols only; 2118 node(s) total have ≤1 connection when file, concept and rationale nodes are included.)
- **192 thin communities (<3 nodes) omitted from report** — run `graphify query` to explore isolated nodes.

## Suggested Questions
_Questions this graph is uniquely positioned to answer:_

- **Why does `Msg` connect `Msg` to `style.rs`, `find.rs`, `tests_reactions.rs`, `app/downloads.rs`, `tc-telegram/src/secret.rs`, `ImageKey`, `PanelMsg`, `ChatMsg`, `.sticker_send_task`, `topics.rs`, `tests_forum.rs`, `app.rs`, `DlMsg`, `ForumMsg`, `tests_notify.rs`, `src/login.rs`, `NotifyMsg`, `tests_panel.rs`, `send_dialog.rs`, `App`, `app.rs`, `.sticker_send_task`, `.update_inner`, `app/notify.rs`, `App`, `tests_qa.rs`, `tc-ui/src/media.rs`, `Nav`, `.toast`, `tests_chat_actions.rs`, `conversation.rs`, `Backend`, `view/audio.rs`, `FilesMsg`, `tests_downloads.rs`, `.apply_auth`, `tests_audio.rs`, `tc-ui/src/audio.rs`, `reading.rs`, `ThreadRef`, `view/login.rs`, `LoginMsg`, `tests_files.rs`, `tests_find.rs`, `main.rs`, `SecretChatId`, `sidebar.rs`, `sidebar.rs`, `context_menu.rs`, `arc`, `sticker_anim.rs`, `User`?**
  _High betweenness centrality (0.167) - this node is a cross-community bridge._
- **Are the 32 inferred relationships involving `Msg` (e.g. with `Message` and `DlMsg`) actually correct?**
  _`Msg` has 32 INFERRED edges - model-reasoned connections that need verification._
- **What connects `First`, `Last`, `Middle` to the rest of the system?**
  _631 weakly-connected nodes found - possible documentation gaps or missing edges._
- **Should `style.rs` be split into smaller, more focused modules?**
  _Cohesion score 0.12505175983436853 - nodes in this community are weakly interconnected._
- **Why does `App` connect `.update_inner` to `style.rs`, `find.rs`, `tests_reactions.rs`, `Msg`, `app/downloads.rs`, `ImageKey`, `ChatMsg`, `topics.rs`, `tests_forum.rs`, `app.rs`, `tests_notify.rs`, `src/login.rs`, `tests_panel.rs`, `tc-mock/src/forum.rs`, `send_dialog.rs`, `app.rs`, `app/notify.rs`, `tc-ui/src/stickers.rs`, `tests_qa.rs`, `Nav`, `.toast`, `tests_chat_actions.rs`, `conversation.rs`, `Backend`, `view/audio.rs`, `FilesMsg`, `tests_downloads.rs`, `tests_audio.rs`, `tc-ui/src/anim.rs`, `tc-ui/src/audio.rs`, `reading.rs`, `view/login.rs`, `tests_files.rs`, `tests_find.rs`, `SecretChatId`, `sidebar.rs`, `Login`, `sidebar.rs`, `context_menu.rs`, `PanelTab`, `arc`, `sticker_anim.rs`, `Message`, `tc-core/src/lib.rs`, `User`?**
  _High betweenness centrality (0.055) - this node is a cross-community bridge._
- **Are the 10 inferred relationships involving `App` (e.g. with `ChatActions` and `Downloads`) actually correct?**
  _`App` has 10 INFERRED edges - model-reasoned connections that need verification._
- **Should `find.rs` be split into smaller, more focused modules?**
  _Cohesion score 0.06398809523809523 - nodes in this community are weakly interconnected._