# Graph Report - teleClon  (2026-11-14)

## Corpus Check
- 247 files · ~272,382 words
- Verdict: corpus is large enough that graph structure adds value.
- Unclassified: 13 file(s) not represented in the graph (top: (none) 2, .example 1, .opus 1)

## Summary
- 6298 nodes · 16916 edges · 324 communities (175 shown, 149 thin omitted)
- Extraction: 91% EXTRACTED · 9% INFERRED · 0% AMBIGUOUS · INFERRED: 1496 edges (avg confidence: 0.74)
- Token cost: 0 input · 0 output

## Graph Freshness
- Built from commit: `41a49619`
- Run `git rev-parse HEAD` and compare to check if the graph is stale.
- Run `graphify update .` after code changes (no API cost).

## Community Hubs (Navigation)
- style.rs
- MockBackend
- tokens.rs
- Msg
- Msg
- f
- BackendEvent
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
- anim.rs
- decode_mp4
- super
- transfers/tests.rs
- tests/transfers.rs
- .send_text
- ForumMsg
- icon
- tc-telegram/src/lib.rs
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
- StickerId
- tc-ui/src/stickers.rs
- tc-telegram/src/auth.rs
- Entropy
- e2e.rs
- build_items
- TelegramConfig
- ADR-0003: Arquitectura del cliente (workspace, backend tras trait, caché SQLite, E2E sans-IO)
- mp4.rs
- tc-ui/src/media.rs
- crypto.rs
- widgets.rs
- Cache
- dialogs.rs
- tc-stickers/src/lib.rs
- tc-store/src/lib.rs
- Compositor (componente)
- tc-telegram/src/forum.rs
- Frame
- tc-store/src/lib.rs
- Cabecera de conversación
- Tipografía (Atkinson Hyperlegible, Bricolage, JetBrains Mono)
- crate::Inner
- pack.rs
- peers.rs
- App
- restored
- fixtures.rs
- media_cache.rs
- view/audio.rs
- secret.rs
- messages.rs
- FilesMsg
- tl.rs
- Gif
- decode.rs
- tests_audio.rs
- tc-ui/src/audio.rs
- tc-telegram/src/media.rs
- CachedBackend<B>
- Store
- reading.rs
- SecretError
- EntityKind
- SessionSnapshot
- view/login.rs
- LoginMsg
- SecretChat
- tc-mock/src/lib.rs
- main.rs
- tc-video/src/decode.rs
- MediaCacheLayer<B>
- super
- cached_tests.rs
- tc-mock/src/dialog_actions.rs
- 2. Casos por flujo
- tc-secret
- uploads.rs
- CLAUDE.md
- .claude/CLAUDE.md
- TextStyle
- StoreError
- sticker_img.rs
- tests/media.rs
- tests/secret.rs
- StickerError
- Output
- Backend
- UserId
- TransferId
- folders.rs
- files.rs
- sticker_anim.rs
- tl_visual_ttl.rs
- Message
- GifAnims
- jobs
- reactions.rs
- EntityKind
- PeerRef
- Marca propia — Parlo
- UserId
- tc-store
- Informe de barrido de marca
- env
- release
- tests_reactions.rs
- tc_core
- layout_message
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
- model.rs
- tc-telegram/src/convert.rs
- tc-store/src/convert.rs
- MockConfig
- file_name.rs
- tc-core/src/lib.rs
- vault.rs
- tc-store/src/forum.rs
- net.rs
- view/mod.rs
- Item
- apply_media_extra
- App
- waveform.rs
- tc-telegram/src/forum.rs
- .download_media
- App
- Login
- tests_sticker_anim.rs
- tests_panel.rs
- tests_qa.rs
- src/login.rs
- tc-ui/src/anim.rs
- ImageRequest
- profile_photo_key
- messages_of
- tests_chat_actions.rs
- .next
- PeerRef
- tc-mock/src/gifs.rs
- PanelMsg
- ADR-0007: Audio (notas de voz y música)
- make_fixtures.sh
- on
- build.rs
- install-user.sh
- env
- build-macos.sh
- build-linux.sh
- anim_tests.rs
- tc-telegram/src/auth.rs
- tests_panel_shots.rs
- context_menu.rs
- tc-mock/src/audio.rs
- ADR-0008: Stickers animados (TGS con rlottie; WebM estático)
- ADR-0009: Teclado de emojis
- ADR-0010: GIFs (guardados, búsqueda, envío y animación)
- emoji_es.rs
- media_key
- Event
- Plan de acción: los 5 primeros faltantes del informe de paridad
- .sticker_send_task
- CodeKind
- Rekey
- Informe de paridad: Parlo frente al cliente original
- ADR-0011: Transferencias (subidas y descargas a disco con progreso y cancelación)
- .apply_auth
- Step
- Picker
- Bugs (replica-test, 2026-11-14)
- Body

## God Nodes (most connected - your core abstractions)
1. `Msg` - 373 edges
2. `App` - 249 edges
3. `BackendResult` - 219 edges
4. `PeerRef` - 141 edges
5. `c()` - 119 edges
6. `MockBackend` - 107 edges
7. `Message` - 93 edges
8. `MsgId` - 88 edges
9. `ThreadScope` - 76 edges
10. `Brasa` - 73 edges

## Surprising Connections (you probably didn't know these)
- `Clean-room (sin código, assets ni marca de Telegram)` --semantically_similar_to--> `Reglas de trabajo (log, ADR, clean-room, credenciales, regenerar grafo)`  [INFERRED] [semantically similar]
  README.md → docs/00-INDEX.md
- `Dirección Brasa (grafito cálido + acento persimón)` --rationale_for--> `Clean-room (sin código, assets ni marca de Telegram)`  [INFERRED]
  docs/adr/0002-direccion-visual.md → README.md
- `display_name_joins_parts()` --references--> `UserId`  [INFERRED]
  crates/tc-core/src/model.rs → crates/tc-core/src/ids.rs
- `Cache` --references--> `Topics`  [INFERRED]
  crates/tc-telegram/src/events.rs → crates/tc-telegram/src/forum.rs
- `radios_de_union()` --calls--> `bubble_radius()`  [INFERRED]
  crates/tc-ui/src/conv.rs → crates/tc-ui/src/style.rs

## Import Cycles
- 1-file cycle: `crates/tc-theme/src/tokens.rs -> crates/tc-theme/src/tokens.rs`
- 1-file cycle: `crates/tc-stickers/src/anim.rs -> crates/tc-stickers/src/anim.rs`
- 1-file cycle: `crates/tc-store/src/vault.rs -> crates/tc-store/src/vault.rs`
- 2-file cycle: `crates/tc-secret/src/session.rs -> crates/tc-secret/src/session/snapshot.rs -> crates/tc-secret/src/session.rs`
- 2-file cycle: `crates/tc-audio/src/decode.rs -> crates/tc-audio/src/lib.rs -> crates/tc-audio/src/decode.rs`
- 2-file cycle: `crates/tc-audio/src/lib.rs -> crates/tc-audio/src/player.rs -> crates/tc-audio/src/lib.rs`
- 2-file cycle: `crates/tc-video/src/decode.rs -> crates/tc-video/src/lib.rs -> crates/tc-video/src/decode.rs`
- 2-file cycle: `crates/tc-video/src/lib.rs -> crates/tc-video/src/mp4.rs -> crates/tc-video/src/lib.rs`
- 2-file cycle: `crates/tc-mock/src/forum.rs -> crates/tc-mock/src/lib.rs -> crates/tc-mock/src/forum.rs`
- 2-file cycle: `crates/tc-mock/src/lib.rs -> crates/tc-mock/src/transfers.rs -> crates/tc-mock/src/lib.rs`
- 2-file cycle: `crates/tc-telegram/src/auth.rs -> crates/tc-telegram/src/lib.rs -> crates/tc-telegram/src/auth.rs`
- 2-file cycle: `crates/tc-telegram/src/lib.rs -> crates/tc-telegram/src/secret.rs -> crates/tc-telegram/src/lib.rs`
- 2-file cycle: `crates/tc-secret/src/dh.rs -> crates/tc-secret/src/testing.rs -> crates/tc-secret/src/dh.rs`
- 2-file cycle: `crates/tc-ui/src/app.rs -> crates/tc-ui/src/media.rs -> crates/tc-ui/src/app.rs`
- 2-file cycle: `crates/tc-ui/src/app.rs -> crates/tc-ui/src/login.rs -> crates/tc-ui/src/app.rs`
- 3-file cycle: `crates/tc-audio/src/decode.rs -> crates/tc-audio/src/lib.rs -> crates/tc-audio/src/player.rs -> crates/tc-audio/src/decode.rs`
- 3-file cycle: `crates/tc-audio/src/decode.rs -> crates/tc-audio/src/ogg.rs -> crates/tc-audio/src/lib.rs -> crates/tc-audio/src/decode.rs`
- 3-file cycle: `crates/tc-video/src/decode.rs -> crates/tc-video/src/mp4.rs -> crates/tc-video/src/lib.rs -> crates/tc-video/src/decode.rs`
- 4-file cycle: `crates/tc-audio/src/decode.rs -> crates/tc-audio/src/ogg.rs -> crates/tc-audio/src/lib.rs -> crates/tc-audio/src/player.rs -> crates/tc-audio/src/decode.rs`

## Hyperedges (group relationships)
- **Implementaciones y decoradores de ChatBackend** — docs_architecture_mockbackend, docs_architecture_cachedbackend, docs_architecture_tc_telegram, docs_architecture_chatbackend [EXTRACTED 0.95]
- **Fundamentos visuales de Brasa** — docs_design_system_paleta_oklch, docs_design_system_tipografia, docs_design_system_espaciado_radios, docs_design_system_elevacion, docs_design_system_movimiento [EXTRACTED 0.95]
- **Decisiones de arquitectura ADR-0001 a ADR-0004** — docs_adr_0001_stack, docs_adr_0002_direccion_visual, docs_adr_0003_arquitectura, docs_adr_0004_stickers_y_chats_secretos [EXTRACTED 1.00]
- **Pila de chats secretos (cripto, claves y UI)** — docs_architecture_tc_secret, docs_architecture_keyring_boveda, docs_design_system_chat_secreto_ui [INFERRED 0.75]
- **Implementaciones de ChatBackend (Mock, Cached, Telegram)** — docs_adr_0003_arquitectura_chatbackend, docs_adr_0003_arquitectura_mockbackend, docs_adr_0003_arquitectura_cachedbackend, docs_adr_0001_stack_grammers [INFERRED 0.85]
- **Pila de chats secretos E2E** — docs_adr_0004_stickers_y_chats_secretos_secret_chats, docs_adr_0003_arquitectura_tc_secret, docs_adr_0001_stack_keyring, docs_adr_0004_stickers_y_chats_secretos_crypto_risk [INFERRED 0.85]

## Communities (324 total, 149 thin omitted)

### Community 0 - "style.rs"
Cohesion: 0.15
Nodes (41): app_style(), avatar_ink(), badge(), bare_input(), border_fill(), Brasa, bubble(), bubble_radius() (+33 more)

### Community 1 - "MockBackend"
Cohesion: 0.05
Nodes (3): Inner, MockBackend, MockConfig

### Community 2 - "tokens.rs"
Cohesion: 0.03
Nodes (58): AVATAR_LG, AVATAR_MD, AVATAR_RATIO, AVATAR_SM, AVATAR_XL, AVATAR_XS, BREAKPOINT_SPLIT, BREAKPOINT_TRIPLE (+50 more)

### Community 3 - "Msg"
Cohesion: 0.02
Nodes (75): Msg, AcceptSecret, AskDelete, AskDiscardSecret, Backend, CancelCompose, Chat, ClearSearch (+67 more)

### Community 4 - "Msg"
Cohesion: 0.06
Nodes (38): Arquitectura de teleClon, ADR-0001 Stack, ADR-0002 Dirección visual, ADR-0003 Arquitectura, ADR-0004 Stickers y chats secretos, BackendEvent, ChatBackend (trait), Modelo de concurrencia (runtime tokio único, actores) (+30 more)

### Community 5 - "f"
Cohesion: 0.08
Nodes (24): Inner, AMPLITUDE, cell_segments(), ContourParams, .DEFAULT, .SECRET, contours(), field() (+16 more)

### Community 6 - "BackendEvent"
Cohesion: 0.05
Nodes (31): BackendEvent, Connection, DialogRemoved, DialogUpserted, Download, ForumTopicsChanged, ForumTopicUpserted, HistoryCleared (+23 more)

### Community 7 - "tc-telegram/src/secret.rs"
Cohesion: 0.06
Nodes (23): Entity, utf16_starts(), now_secs(), Chat, crypto(), EARLY_CAP, entities_from_secret(), entities_round_trip_through_utf16() (+15 more)

### Community 8 - "Sistema de diseño de teleClon — dirección «Brasa»"
Cohesion: 0.15
Nodes (26): about_thirty_chats_sorted_with_pinned_first(), data_is_deterministic(), delete_updates_last_message_and_emits(), edit_only_own_messages(), forward_creates_messages_with_origin(), grupo_con_muchos_no_leidos_y_lectura_parcial(), history_after_and_around(), long_history_paging_covers_everything_without_gaps() (+18 more)

### Community 9 - "conv.rs"
Cohesion: 0.06
Nodes (35): DATE_H, EDGE_H, FILE_CARD_H, FILE_CARD_W, FWD_H, GAP_GROUP, GAP_IN_GROUP, GROUP_AVATAR (+27 more)

### Community 10 - "format.rs"
Cohesion: 0.18
Nodes (23): todas_las_palabras_en_espanol_apuntan_a_emojis_reales(), date_separator(), day_key(), ellipsize(), fold(), hhmm(), horas_y_separadores_en_utc(), huella_determinista_y_en_rango() (+15 more)

### Community 11 - "model.rs"
Cohesion: 0.15
Nodes (13): INK, WHITE, AA_NORMAL, AA_UI, Alpha, Alphas, Rgb, bubble_out_gradient() (+5 more)

### Community 12 - "tests_files.rs"
Cohesion: 0.07
Nodes (52): ALBUM_MAX, CAPTION_COUNTER_FROM, CAPTION_ID, caption_len(), clases_por_extension(), classify(), clipboard_image_to_png(), FILE_WARN_BYTES (+44 more)

### Community 13 - "ci.yml"
Cohesion: 0.07
Nodes (29): name, runs-on, steps, strategy, concurrency, cancel-in-progress, group, env (+21 more)

### Community 14 - "ChatMsg"
Cohesion: 0.04
Nodes (61): action_error_text(), App, ChatActions, ChatMsg, Archive, ArchivedLoaded, AskBlock, AskClear (+53 more)

### Community 15 - "player.rs"
Cohesion: 0.11
Nodes (18): Cmd, Paused, Play, Speed, Stop, Current, fake(), FakeOutput (+10 more)

### Community 16 - "BackendResult"
Cohesion: 0.08
Nodes (9): DialogsApi, MessagesApi, BackendResult, ThreadScope, PeerRef, Group, Secret, User (+1 more)

### Community 17 - ".update"
Cohesion: 0.11
Nodes (18): BUNDLED, DISPLAY, DISPLAY_FAMILY, EMOJI_FAMILY, family(), MONO, MONO_FALLBACK, MONO_FAMILY (+10 more)

### Community 18 - "topics.rs"
Cohesion: 0.09
Nodes (38): topic_initial(), preview(), ellipsize_to(), text_width(), with_weight(), badge(), badge_at(), chat_row() (+30 more)

### Community 19 - "api.rs"
Cohesion: 0.16
Nodes (15): ADR-0002: Dirección visual Brasa, Dirección Brasa (grafito cálido + acento persimón), Controlled para chrome, innovative para burbujas y fondo, Movimiento con muelles 119-221 ms y movimiento reducido, OKLCH como fuente de verdad y contraste WCAG AA (144 pares), docs/design/system.md y tokens.json, Tipografía libre (Atkinson Hyperlegible Next, Bricolage Grotesque, JetBrains Mono) e iconos Phosphor, Agente diseño (opus/high) (+7 more)

### Community 20 - "anim.rs"
Cohesion: 0.06
Nodes (20): classify(), main(), report(), scan(), build(), enc(), in_gamut(), lum() (+12 more)

### Community 21 - "decode_mp4"
Cohesion: 0.13
Nodes (22): Clip, Collector, decode_mp4(), decode_track(), Frame, L, looks_like_mp4(), encoge_y_despues_quita_fotogramas_para_caber() (+14 more)

### Community 22 - "super"
Cohesion: 0.11
Nodes (13): key_is_created_once_in_the_vault_and_reused(), roundtrip_and_every_kind_of_tampering_is_rejected(), SEAL_KEY_LEN, Sealed, SealError, BadKey, Entropy, Rejected (+5 more)

### Community 23 - "transfers/tests.rs"
Cohesion: 0.06
Nodes (30): partial_path(), partial_path_is_a_sibling(), check_dest(), Inner, MAX_STEPS, MockBackend, storage(), terminal() (+22 more)

### Community 24 - "tests/transfers.rs"
Cohesion: 0.12
Nodes (31): ANA, assert_well_formed(), content(), Delayed, DOC, doc_location(), doc_media(), doc_ref() (+23 more)

### Community 26 - "ForumMsg"
Cohesion: 0.05
Nodes (33): App, ForumMsg, BackToTopics, ClearQuery, Close, CloseList, CloseMenu, Create (+25 more)

### Community 28 - "icon"
Cohesion: 0.19
Nodes (12): ADR-0001: Stack técnico, grammers (MTProto en Rust), iced (UI en Rust, wgpu + tiny-skia), keyring (claves y sesión), rusqlite bundled con migraciones por PRAGMA user_version, tc-secret: chats secretos sans-IO con RustCrypto, tc-stickers: WebP, TGS con rlottie, WebM aplazado, ADR-0004: Stickers y chats secretos en el alcance (+4 more)

### Community 29 - "tc-telegram/src/lib.rs"
Cohesion: 0.08
Nodes (13): Feed, None, Pool, Source, GrammersBackend, has_login_token(), Inner, persister() (+5 more)

### Community 30 - "tc-ui/src/tests.rs"
Cohesion: 0.24
Nodes (12): ADR-0003: Arquitectura del cliente, Orden de construcción M1.1-M1.4, M2.1, M2.7, CachedBackend (caché como decorador, tc-store), Trait ChatBackend (sub-traits + events()), Modelo de concurrencia (tokio, hilo SQLite, canal acotado, Resync), MockBackend determinista, SQLite por cuenta (17 tablas + FTS5, WAL, chat_key), Scroll virtual propio (Fenwick, ventana ~600 mensajes) (+4 more)

### Community 31 - "EntityKind"
Cohesion: 0.24
Nodes (13): Registro de agentes, Agente arquitecto (opus/high), Agente core+mock (sonnet/medium), Agente QA (pendiente), Agente theme (sonnet/medium), Agente UI (opus/high), Bitácora (append-only), Bug iced 0.14: scrollable.direction antes de anchor_bottom (+5 more)

### Community 32 - "tc-mock/src/forum.rs"
Cohesion: 0.07
Nodes (28): el_foro_tiene_temas_con_general_y_no_leidos_que_suman(), entrante_en_tema_crear_y_fijar(), FORUM_CHAT, FORUM_GROUP, Forums, historial_envio_y_lectura_por_tema(), Inner, mock() (+20 more)

### Community 34 - "TcSession"
Cohesion: 0.09
Nodes (9): changes_mark_dirty_and_reset_forgets_everything(), debug_never_prints_the_auth_key(), rejects_garbage_and_unknown_versions(), roundtrip_keeps_keys_peers_and_state(), sample(), SESSION_FORMAT, SessionError, SessionFile (+1 more)

### Community 35 - "MsgLayout"
Cohesion: 0.21
Nodes (11): Índice maestro de teleClon, architecture.md (crates, ChatBackend, SQLite, F01-F18), graphify-out (graph.html, GRAPH_REPORT.md, graph.json), Reglas de trabajo (log, ADR, clean-room, credenciales, regenerar grafo), Credenciales api_id/api_hash en .env local gitignored, graphify: instalación y uso, graphifyy en ./.venv local, Registro de skills usadas (+3 more)

### Community 36 - "send_dialog.rs"
Cohesion: 0.17
Nodes (20): Upload, bubble(), BAR_H, big_preview(), cancel_button(), compress_choice(), dialog(), DIALOG_W (+12 more)

### Community 37 - "dh.rs"
Cohesion: 0.07
Nodes (26): dh_matches_independent_vector(), DhParams, DhSecret, exp(), FP, GA, GB, generate_mixes_server_random_and_rejects_bad_peer() (+18 more)

### Community 38 - "tc-ui/src/gifs.rs"
Cohesion: 0.08
Nodes (27): ANIM_BUDGET, ANIM_PX, animates(), CACHE_BYTES, CELL_GAP, CELL_H, CELL_MIN_W, decode() (+19 more)

### Community 39 - "app.rs"
Cohesion: 0.05
Nodes (31): AnimKey, ChatSwitch, Overlay, SendButton, auth_events(), AuthHandle, backend_events(), BackendHandle (+23 more)

### Community 40 - "with"
Cohesion: 0.20
Nodes (17): steps, steps, steps, steps, id, if, name, run (+9 more)

### Community 41 - "StickerId"
Cohesion: 0.24
Nodes (4): StickerId, DEFAULT_RECENTS, Favorites, Recents

### Community 42 - "tc-ui/src/stickers.rs"
Cohesion: 0.12
Nodes (32): StickerSetId, Sticker, StickerSet, ANIMATED_SET_ID, Art, art_index(), ART_PX, CELL_ART (+24 more)

### Community 43 - "tc-telegram/src/auth.rs"
Cohesion: 0.18
Nodes (3): GrammersBackend, Inner, map_error()

### Community 45 - "Entropy"
Cohesion: 0.06
Nodes (43): Role, Initiator, Responder, Entropy, &mut E, OsEntropy, random_i64(), random_up_to() (+35 more)

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
Cohesion: 0.38
Nodes (11): tc-app, tc-audio, tc-core, tc-mock, tc-secret, tc-stickers, tc-store, tc-telegram (+3 more)

### Community 50 - "mp4.rs"
Cohesion: 0.19
Nodes (28): VideoError, Container, Decode, NoVideo, Unsupported, annex_b(), annex_b_pone_codigos_de_inicio(), bad() (+20 more)

### Community 51 - "tc-ui/src/media.rs"
Cohesion: 0.14
Nodes (20): AVATAR_PX, avatars_are_square_and_downscaled(), bubble_aspect(), BUBBLE_ASPECT, bubble_photos_are_cropped_to_the_layout_ratio(), BUBBLE_PX, CACHE_BYTES, decode_avatar() (+12 more)

### Community 52 - "crypto.rs"
Cohesion: 0.07
Nodes (31): decrypt(), derive(), encrypt(), encrypt_with_padding(), HEADER_LEN, ige_decrypt(), ige_encrypt(), IgeDec (+23 more)

### Community 53 - "widgets.rs"
Cohesion: 0.06
Nodes (19): base(), Font_, IcedMeasurer, Paragraph, SHAPING, to_spans(), WRAP, avatar_radius() (+11 more)

### Community 54 - "Cache"
Cohesion: 0.10
Nodes (17): dialog_from_tl(), dialog_stub(), DialogCtx, draft_date(), draft_from_tl(), is_muted(), reactions_from_tl(), Applied (+9 more)

### Community 55 - "dialogs.rs"
Cohesion: 0.06
Nodes (23): DIALOG_COLS, dialog_from_row(), draft_from_row(), drafts_set_clear_list(), list_orders_pinned_then_activity_and_filters_archived(), mark_read_and_missing_dialog_errors(), notify_scopes(), scope_parts() (+15 more)

### Community 56 - "tc-stickers/src/lib.rs"
Cohesion: 0.67
Nodes (3): Disposición (raíl, lista, conversación, panel info), Lista de chats (componente), Barra lateral (raíl de carpetas)

### Community 59 - "tc-telegram/src/forum.rs"
Cohesion: 0.16
Nodes (4): flatten_updates(), GrammersBackend, Inner, topic_from_tl()

### Community 60 - "Frame"
Cohesion: 0.08
Nodes (19): CacheKey, DEFAULT_CACHE_BYTES, FrameCache, Frame, cache_evicts_least_recently_used_by_bytes(), cache_get_or_insert_computes_once(), gradient(), key() (+11 more)

### Community 61 - "tc-store/src/lib.rs"
Cohesion: 0.11
Nodes (20): account_db_path(), APP_DIR, DATA_DIR_NAME, default_data_dir(), LEGACY_APP_DIR, MIGRATIONS, resolve_data_dir(), SCHEMA_VERSION (+12 more)

### Community 64 - "crate::Inner"
Cohesion: 0.08
Nodes (10): crate::Inner, GIF_BOT, gif_from_document(), GifCatalog, input_doc(), CachedSet, crate::Inner, MAX_SETS (+2 more)

### Community 65 - "pack.rs"
Cohesion: 0.09
Nodes (27): Def, DEFS, dots(), index_of(), Layer, len(), NATIVE_SIZE, P (+19 more)

### Community 66 - "peers.rs"
Cohesion: 0.12
Nodes (23): channel_group(), chat_group(), group_ids_roundtrip_and_never_collide(), chat_photo(), group_from_tl(), PeerStore, user_from_tl(), user_photo() (+15 more)

### Community 67 - "App"
Cohesion: 0.09
Nodes (39): SendState, Failed, Pending, Sent, now(), Conversation, is_local(), MsgLayout (+31 more)

### Community 68 - "restored"
Cohesion: 0.09
Nodes (42): authorization(), password(), state(), updates(), user(), user_enum(), dialogs_page(), dialogs_paginate_with_server_offsets() (+34 more)

### Community 69 - "fixtures.rs"
Cohesion: 0.10
Nodes (34): message_from_tl(), dialog(), message(), msg_enum(), notify(), peer_channel(), peer_chat(), peer_user() (+26 more)

### Community 70 - "media_cache.rs"
Cohesion: 0.10
Nodes (16): account_cache_lives_next_to_the_database_and_survives_reopen(), check_key(), create_private_dir(), DEFAULT_MEDIA_CACHE_LIMIT, fetch_cached(), fetch_cached_goes_to_the_network_once_and_never_for_secrets(), io(), keys_depend_on_id_and_size_and_skip_secrets() (+8 more)

### Community 71 - "view/audio.rs"
Cohesion: 0.10
Nodes (14): TrackView, Bar, BAR_GAP, BAR_W, bubble(), BUTTON, fraction_at(), Ink (+6 more)

### Community 72 - "secret.rs"
Cohesion: 0.11
Nodes (18): chat(), CHAT_COLS, chat_from_row(), chat_roundtrip_and_fk(), messages_idempotent_blob_roundtrip_and_page(), msg(), MSG_COLS, msg_from_row() (+10 more)

### Community 73 - "messages.rs"
Cohesion: 0.14
Nodes (14): COLS, fill(), from_row(), fts_query(), fts_search_accents_prefix_scope_and_delete(), ids(), pagination_by_anchor(), pending_becomes_confirmed_by_random_id() (+6 more)

### Community 74 - "FilesMsg"
Cohesion: 0.09
Nodes (19): App, FilesMsg, Cancel, Caption, Close, Compress, Inspected, Paste (+11 more)

### Community 75 - "tl.rs"
Cohesion: 0.05
Nodes (53): Action, AbortKey, AcceptKey, CommitKey, DeleteMessages, FlushHistory, Noop, NotifyLayer (+45 more)

### Community 76 - "Gif"
Cohesion: 0.06
Nodes (34): StickersApi, BackendError, Cancelled, Crypto, FloodWait, Forbidden, Internal, InvalidInput (+26 more)

### Community 77 - "decode.rs"
Cohesion: 0.10
Nodes (16): check_tone(), copy_out(), decode_all(), decodes_music_formats(), decodes_stereo_opus(), decodes_telegram_style_voice_note(), fixture(), freq() (+8 more)

### Community 78 - "tests_audio.rs"
Cohesion: 0.19
Nodes (17): AudioBytes, TrackState, Failed, Idle, Loading, Paused, Playing, boot() (+9 more)

### Community 79 - "tc-ui/src/audio.rs"
Cohesion: 0.06
Nodes (32): PlaybackState, Ended, Failed, Paused, Playing, Starting, SPEEDS, App (+24 more)

### Community 80 - "tc-telegram/src/media.rs"
Cohesion: 0.06
Nodes (28): BIG_FILE, Cancel, check_dest(), crate::Inner, DownloadError, Backend, FileReference, GrammersBackend (+20 more)

### Community 81 - "CachedBackend<B>"
Cohesion: 0.09
Nodes (3): CachedBackend<B>, is_transient(), Shared<B>

### Community 82 - "Store"
Cohesion: 0.13
Nodes (12): favorites(), recents_order_dedupe_cap_and_cascade(), seed(), SET_COLS, set_from_row(), sets_and_stickers(), STICKER_COLS, sticker_from_row() (+4 more)

### Community 83 - "reading.rs"
Cohesion: 0.10
Nodes (22): Message, App, Carry, desplazamiento_deja_la_marca_arriba_con_contexto(), first_unread(), FLUSH_EVERY, from_bottom_for(), height_after() (+14 more)

### Community 84 - "SecretError"
Cohesion: 0.14
Nodes (12): Result, SecretError, Entropy, FingerprintMismatch, InvalidDhConfig, InvalidPublicValue, InvalidState, Malformed (+4 more)

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
Cohesion: 0.13
Nodes (18): LoginMsg, CodeChanged, CodeSent, PasswordChanged, PasswordChecked, PhoneChanged, QrReady, Resend (+10 more)

### Community 89 - "SecretChat"
Cohesion: 0.17
Nodes (14): StateKind, Discarded, Incoming, Ready, Requested, SeededEntropy, corrupt_or_foreign_snapshots_are_rejected(), dh() (+6 more)

### Community 90 - "tc-mock/src/lib.rs"
Cohesion: 0.07
Nodes (26): animated_sticker_set(), BIG_CHAT, BIG_CHAT_LEN, build(), CHANNEL_POSTS, gen_channel(), gen_history(), GROUPS (+18 more)

### Community 91 - "main.rs"
Cohesion: 0.09
Nodes (18): asignaciones_comillas_y_comentarios(), el_entorno_manda_sobre_el_archivo_salvo_si_esta_vacio(), EnvFile, lookup(), unquote(), account_id(), env_flag(), env_u64() (+10 more)

### Community 92 - "tc-video/src/decode.rs"
Cohesion: 0.14
Nodes (17): basura_o_vacio_es_error(), center(), decodifica_un_mp4_codificado_con_openh264(), dominant(), escala_y_reparte_segun_el_plan(), MAX_SAMPLES, archivos_rotos_dan_error_sin_panico(), lee_la_tabla_de_muestras_y_los_parametros() (+9 more)

### Community 94 - "super"
Cohesion: 0.08
Nodes (16): Store, unread_mark_and_clear_history(), Error, Invalid, Io, NoDataDir, NotFound, SchemaTooNew (+8 more)

### Community 95 - "cached_tests.rs"
Cohesion: 0.12
Nodes (25): all(), cache_persists_across_restarts_on_disk(), cached(), cold_cache_offline_is_an_error_and_pagination_works(), dialogs_roundtrip_exactly_and_survive_offline(), downloads_are_cached_on_disk_but_never_for_secret_chats(), edits_deletes_reactions_read_and_failures_write_through(), events_are_persisted_and_searchable_locally() (+17 more)

### Community 96 - "tc-mock/src/dialog_actions.rs"
Cohesion: 0.12
Nodes (14): Acts, all(), block_info_and_pinned_messages(), check_pin_limit(), clear_history_keeps_the_row_delete_removes_it(), deleting_a_secret_chat_discards_it(), leave_only_groups_and_channels_cannot_be_cleared(), mock() (+6 more)

### Community 97 - "2. Casos por flujo"
Cohesion: 0.08
Nodes (23): 1. Estrategia, 2. Casos por flujo, 3. Pruebas transversales, 4. Orden y criterios, F01 Iniciar sesión con teléfono (S01 → S02 → S04 → S05 → S06 → S07), F02 Login con QR (S01 → S02 → S03 → S07), F03 Abrir chat y leer (S07 → S08 → S11), F04 Enviar mensaje de texto (S11 → S13) (+15 more)

### Community 98 - "tc-secret"
Cohesion: 0.40
Nodes (4): Garantías locales, Lo que NO está hecho o NO está verificado, Módulos, tc-secret

### Community 99 - "uploads.rs"
Cohesion: 0.19
Nodes (5): UploadRow, destination_rules_and_cascade(), lifecycle_progress_resume_and_purge(), new_upload(), Store

### Community 102 - "TextStyle"
Cohesion: 0.15
Nodes (14): BODY, CAPTION, CODE, DISPLAY, EMOJI_ONLY, HEADING, META, TextStyle (+6 more)

### Community 103 - "StoreError"
Cohesion: 0.12
Nodes (11): backend(), Rd<'a>, VaultSessionStore, file_store_is_private_and_atomic(), FileSessionStore, MemorySessionStore, SessionStore, StoreError (+3 more)

### Community 104 - "sticker_img.rs"
Cohesion: 0.12
Nodes (16): StickerFormat, Lottie, Webm, Webp, LOCAL_SEND_PX, BUBBLE_PX, CELL_PX, decode() (+8 more)

### Community 105 - "tests/media.rs"
Cohesion: 0.19
Nodes (15): channel(), at_most_four_downloads_run_at_once(), channel_reference_refresh_uses_channels_get_messages_and_long_flood_is_reported(), channel_with_photo(), documents_keep_their_thumbnail_as_preview(), expired_reference_is_refreshed_and_file_migrate_followed(), file_part(), photo() (+7 more)

### Community 106 - "tests/secret.rs"
Cohesion: 0.06
Nodes (31): UpdateBatch, UpdateSource, UpdatesReceiver, Fake, Handler, logged_session(), logged_session_as(), ME_ID (+23 more)

### Community 107 - "StickerError"
Cohesion: 0.08
Nodes (28): ANIM_PX, frame_at(), frame_step(), info_of(), MAX_FPS, next_frames(), render_batch(), Show (+20 more)

### Community 108 - "Output"
Cohesion: 0.12
Nodes (3): Output, RodioOutput, NoDevice

### Community 109 - "Backend"
Cohesion: 0.04
Nodes (74): demo_script(), ThemeKind, Dark, Light, Auth, Backend, Connector, DemoStep (+66 more)

### Community 110 - "UserId"
Cohesion: 0.08
Nodes (18): ProfileApi, AuthApi, AuthState, Checking, LoggedIn, LoggedOut, SignUpRequired, WaitingCode (+10 more)

### Community 111 - "TransferId"
Cohesion: 0.12
Nodes (14): default_methods_are_unsupported(), fraction_and_finished(), Transfer, TransferId, Download, Upload, TransferKind, Download (+6 more)

### Community 112 - "folders.rs"
Cohesion: 0.19
Nodes (8): COLS, folder(), folder_peers_replace_and_cascade(), folders_crud_order_and_reorder(), from_row(), Store, FolderPeerRow, FolderRow

### Community 113 - "files.rs"
Cohesion: 0.13
Nodes (10): decode_sticker_file(), file_ref_from(), file_ref_row(), cache_index_orders_by_last_use(), CacheEntry, media_refs_follow_their_media(), MediaRefRow, peer_photos_roundtrip_and_never_for_secrets() (+2 more)

### Community 115 - "sticker_anim.rs"
Cohesion: 0.09
Nodes (28): Playback, AnimMsg, Focus, Frames, Hover, Loaded, Tick, Unhover (+20 more)

### Community 116 - "tl_visual_ttl.rs"
Cohesion: 0.07
Nodes (11): TypingAction, Cancel, Typing, SelfDestructTimer, KeyVisualization, key_visualization_matches_independent_vector(), layer(), tl_known_encoding() (+3 more)

### Community 117 - "Message"
Cohesion: 0.09
Nodes (17): unsupported(), ForumApi, ThreadRef, MsgId, RandomId, Anchor, After, Around (+9 more)

### Community 118 - "GifAnims"
Cohesion: 0.20
Nodes (6): GifAnims, la_cache_de_animaciones_respeta_el_tope(), Slot, Failed, Loading, Ready

### Community 119 - "jobs"
Cohesion: 0.18
Nodes (10): env, CARGO_INCREMENTAL, CARGO_TERM_COLOR, RUST_TOOLCHAIN, jobs, linux, name, needs (+2 more)

### Community 120 - "reactions.rs"
Cohesion: 0.29
Nodes (7): message_exists(), my_reaction_moves_between_emojis(), r(), set_get_replace_and_unknown_message(), setup(), Store, ReactionRow

### Community 121 - "EntityKind"
Cohesion: 0.18
Nodes (11): EntityKind, Bold, Code, Hashtag, Italic, Mention, Pre, Spoiler (+3 more)

### Community 123 - "Marca propia — Parlo"
Cohesion: 0.07
Nodes (25): ADR-0006: Empaquetado multiplataforma (Windows, macOS, Linux), Consecuencias, Decisiones, 1. Criterios de nombre, 2. Candidatos (6-8), 3. Chequeos que debe hacer el usuario, 4. Voz y tono (español), 5. Brief de logo (+17 more)

### Community 124 - "UserId"
Cohesion: 0.09
Nodes (7): SecretChatsApi, SecretChatId, UserId, KeyVisual, SecretChat, msg(), pendiente_se_sustituye_por_el_real()

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
Cohesion: 0.12
Nodes (11): App, apply_mine(), reaction_error_text(), account_stickers_load_in_the_picker_and_in_bubbles(), boot(), decoded(), my_reaction_moves_like_the_server_does(), NOW (+3 more)

### Community 131 - "tc_core"
Cohesion: 0.21
Nodes (11): MAX_TEXT, MAX_DELETE_ROUNDS, channel_of(), CHANNEL_OFFSET, decode_group(), GroupTarget, Channel, Chat (+3 more)

### Community 133 - "layout_message"
Cohesion: 0.19
Nodes (9): emoji_only(), input(), layout_message(), LayoutCache, LayoutInput, maquetacion_meta_en_linea_o_debajo(), meta_width(), runs() (+1 more)

### Community 134 - "AudioError"
Cohesion: 0.10
Nodes (18): AudioStream, open(), OpusStream, sym_err(), AudioError, Corrupt, NoDevice, Unsupported (+10 more)

### Community 135 - "ImageKey"
Cohesion: 0.13
Nodes (11): cache_tracks_loads_failures_and_evicts_least_recently_used(), ImageKey, Avatar, Full, Sticker, Thumb, Images, Slot (+3 more)

### Community 138 - "Tone"
Cohesion: 0.16
Nodes (10): apply(), Tone, .ALL, Dark, Default, Light, Medium, MediumDark (+2 more)

### Community 139 - "stores.rs"
Cohesion: 0.11
Nodes (28): aad(), AAD_PREFIX, decode(), dh(), disk_bytes(), encode(), existing_session_json_is_migrated_once_and_deleted(), FIXED_LEN (+20 more)

### Community 140 - "PanelMsg"
Cohesion: 0.09
Nodes (22): GifMsg, AnimLoaded, More, Query, Results, Retry, Saved, Scrolled (+14 more)

### Community 141 - "windows"
Cohesion: 0.29
Nodes (7): WINDOWS_CERT_PASSWORD, WINDOWS_CERT_PFX_BASE64, windows, env, name, needs, runs-on

### Community 142 - ".sticker_send_task"
Cohesion: 0.15
Nodes (4): ActiveTgs, App, off_thread(), plays()

### Community 143 - "Plan de pruebas Parlo (replica-test, app de escritorio Rust)"
Cohesion: 0.10
Nodes (19): F01 Login con teléfono, F02 Login por QR, F03 Abrir chat y leer, F04 Enviar texto, F05 Responder, F06 Reenviar, F07 Editar, F08 Borrar (+11 more)

### Community 144 - "emoji.rs"
Cohesion: 0.09
Nodes (27): busqueda_en_espanol_ingles_y_por_grupo(), catalog_len(), Category, Activities, .ALL, Animals, Flags, Food (+19 more)

### Community 145 - "meta"
Cohesion: 0.40
Nodes (5): meta, name, outputs, runs-on, version

### Community 147 - "model.rs"
Cohesion: 0.04
Nodes (46): ChatBackend, MediaApi, UNSUPPORTED, AudioInfo, DEFAULT_REACTIONS, display_name_joins_parts(), FileKind, Document (+38 more)

### Community 149 - "tc-telegram/src/convert.rs"
Cohesion: 0.08
Nodes (24): document_file_ref(), entities_from_tl(), entities_to_tl(), entity_parts(), forward_name(), media_from_document(), media_from_tl(), photo_dims() (+16 more)

### Community 150 - "tc-store/src/convert.rs"
Cohesion: 0.07
Nodes (45): voice_notes_and_music_survive_the_cache(), apply_dialogs(), apply_history(), cached_dialogs(), cached_history(), decode_entities(), decode_sticker(), dialogs_complete_key() (+37 more)

### Community 151 - "MockConfig"
Cohesion: 0.33
Nodes (8): a_user_chat(), all(), cached(), clear_and_delete_reach_the_cache(), offline_list(), removal_events_are_persisted(), unread_mark_read_mute_and_pin_persist(), CachedBackend

### Community 152 - "file_name.rs"
Cohesion: 0.07
Nodes (37): assert_safe(), bidi_control(), DEFAULT_FILE_NAME, examples(), forbidden(), is_reserved(), long_names_keep_their_extension_and_char_boundaries(), MAX_EXT_BYTES (+29 more)

### Community 153 - "tc-core/src/lib.rs"
Cohesion: 0.12
Nodes (20): el_tema_como_dialogo_conserva_lectura_y_no_leidos(), fijados_primero_y_luego_por_actividad(), ForumTopic, GENERAL_TOPIC, msg(), sort_topics(), TOPIC_COLORS, TopicIcon (+12 more)

### Community 154 - "vault.rs"
Cohesion: 0.08
Nodes (32): DeadVault, DeadVault, check_key(), CHUNK_HEADER_LEN, CHUNK_MAGIC, chunked_vault_reports_missing_parts_and_keeps_magic_values_intact(), chunked_vault_splits_large_values_and_cleans_up(), ChunkedVault<V> (+24 more)

### Community 155 - "tc-store/src/forum.rs"
Cohesion: 0.11
Nodes (8): COLS, from_row(), Store, temas_marca_de_foro_y_mensajes_por_tema(), TopicRow, group_roundtrip_update_and_lookup(), Store, GroupRow

### Community 156 - "net.rs"
Cohesion: 0.09
Nodes (20): ConnectionState, Connecting, Offline, Online, Updating, ConnTracker, errors_map_to_domain(), flood_wait_policy_sleeps_only_short_waits() (+12 more)

### Community 161 - "view/mod.rs"
Cohesion: 0.06
Nodes (32): handle(), HANDLES, icon(), icon_fill(), icons!, view(), PANEL_H, CELL (+24 more)

### Community 162 - "Item"
Cohesion: 0.29
Nodes (5): Item, Date, Edge, Msg, UnreadMark

### Community 164 - "apply_media_extra"
Cohesion: 0.23
Nodes (4): apply_media_extra(), MediaExtraRow, Store, voice()

### Community 165 - "App"
Cohesion: 0.12
Nodes (11): AnimTarget, App, Gif, Nav, Activate, Down, Left, NextTab (+3 more)

### Community 166 - "waveform.rs"
Cohesion: 0.39
Nodes (8): decode(), decodes_known_bit_layout(), encode(), from_samples(), MAX, resample(), resample_and_from_samples(), roundtrip_any_length()

### Community 167 - "tc-telegram/src/forum.rs"
Cohesion: 0.13
Nodes (8): apply_update(), current_topic(), in_topic(), is_forum(), route_new_message(), topic_of(), topic_reply_to(), Topics

### Community 169 - "App"
Cohesion: 0.08
Nodes (6): BubbleOpacity, BubbleY, App, Reply, Files, sort_dialogs()

### Community 170 - "Login"
Cohesion: 0.19
Nodes (7): Field, Code, Password, Phone, left(), Login, un_logged_out_repetido_no_borra_lo_escrito()

### Community 171 - "tests_sticker_anim.rs"
Cohesion: 0.33
Nodes (8): app_with_tgs_bubble(), el_fotograma_animado_llega_a_la_burbuja_y_se_congela_sin_foco(), el_reloj_solo_corre_con_un_tgs_visible_con_foco_y_sin_movimiento_reducido(), en_el_selector_se_anima_el_sticker_bajo_el_raton(), FILE, MOVING, NOW, tgs_sticker()

### Community 172 - "tests_panel.rs"
Cohesion: 0.49
Nodes (9): boot_in_chat(), emoji_en_el_cursor_recientes_busqueda_y_tono(), gifs_guardados_busqueda_envio_y_reintento(), gm(), los_gif_reales_se_animan_con_el_reloj_compartido(), NOW, pestanas_atajos_y_cierre(), pm() (+1 more)

### Community 174 - "tests_qa.rs"
Cohesion: 0.24
Nodes (14): error_text(), boot_open(), f04_doble_enter_no_duplica_el_mensaje(), f04_reintento_conserva_random_id_y_no_duplica_en_el_backend(), f04_texto_con_emoji_acentos_y_saltos_se_envia_integro(), f04_texto_de_mas_de_4096_avisa_y_no_deja_burbuja(), f04_texto_vacio_o_en_blanco_no_se_envia(), f05_borrar_el_citado_cancela_la_respuesta() (+6 more)

### Community 179 - "src/login.rs"
Cohesion: 0.17
Nodes (12): clean_code(), CODE_ID, codigo_solo_cifras_y_recortado(), countdown(), cuentas_atras(), errores_con_texto_propio(), login_error(), normalize_phone() (+4 more)

### Community 183 - "tc-ui/src/anim.rs"
Cohesion: 0.05
Nodes (34): ByteLru, ByteLru<K, V>, Easing, .ESTANDAR, .RAPIDO, .SALIDA, .SUAVE, HIGHLIGHT_FADE_MS (+26 more)

### Community 185 - "ImageRequest"
Cohesion: 0.40
Nodes (3): ImageRequest, Avatar, Sticker

### Community 191 - "tests_chat_actions.rs"
Cohesion: 0.11
Nodes (23): MuteFor, .ALL, EightHours, Forever, Hour, TwoDays, a_chat_never_appears_twice_after_reloading_the_lists(), a_user() (+15 more)

### Community 202 - "tc-mock/src/gifs.rs"
Cohesion: 0.12
Nodes (17): by_id(), download(), encode_gif(), encode_png(), frame(), FRAMES, GIF_BASE, guardados_busqueda_y_bytes() (+9 more)

### Community 205 - "PanelMsg"
Cohesion: 0.13
Nodes (10): recientes_por_fecha_y_frecuentes_por_veces(), Usage, EMOJI_SEARCH_ID, EmojiPanel, Panel, PanelTab, .ALL, Emoji (+2 more)

### Community 207 - "ADR-0007: Audio (notas de voz y música)"
Cohesion: 0.50
Nodes (3): ADR-0007: Audio (notas de voz y música), Consecuencias, Decisiones

### Community 211 - "on"
Cohesion: 0.50
Nodes (4): on, push, workflow_dispatch, tags

### Community 215 - "env"
Cohesion: 0.67
Nodes (3): GH_TOKEN, VERSION, env

### Community 223 - "anim_tests.rs"
Cohesion: 0.26
Nodes (10): fotograma_por_tiempo_a_30_fps_como_mucho(), gzip(), info(), movimiento_reducido_foco_y_visibilidad(), MOVING, ms(), siguientes_fotogramas_en_bucle(), sin_lottie_no_hay_animacion() (+2 more)

### Community 226 - "tc-telegram/src/auth.rs"
Cohesion: 0.10
Nodes (11): AuthInner, code_kind(), code_types_map(), migrate_dc(), next_kind(), phone_validation(), qr_url(), qr_url_is_unpadded_base64url() (+3 more)

### Community 236 - "tests_panel_shots.rs"
Cohesion: 0.29
Nodes (5): App, capturas_del_panel(), colorful(), settings(), shot()

### Community 237 - "context_menu.rs"
Cohesion: 0.08
Nodes (31): InfoView, act(), archive_row(), buttons(), chat(), chat_items(), confirm_panel(), CursorProbe (+23 more)

### Community 241 - "tc-mock/src/audio.rs"
Cohesion: 0.46
Nodes (7): BARS, deterministic_and_well_formed(), mix(), RATE, voice_wav(), wav_len(), waveform()

### Community 243 - "ADR-0008: Stickers animados (TGS con rlottie; WebM estático)"
Cohesion: 0.33
Nodes (5): ADR-0008: Stickers animados (TGS con rlottie; WebM estático), Consecuencias, Decisión, TGS (Lottie con gzip): animado con rlottie, por defecto solo en Linux, WebM (vídeo VP9 con alfa): queda estático (miniatura)

### Community 247 - "ADR-0009: Teclado de emojis"
Cohesion: 0.50
Nodes (3): ADR-0009: Teclado de emojis, Consecuencias, Decisiones

### Community 248 - "ADR-0010: GIFs (guardados, búsqueda, envío y animación)"
Cohesion: 0.50
Nodes (3): ADR-0010: GIFs (guardados, búsqueda, envío y animación), Consecuencias, Decisiones

### Community 285 - "Event"
Cohesion: 0.10
Nodes (24): DiscardReason, FingerprintMismatch, InvalidPublicValue, Local, RekeyFailure, Remote, ResendUnavailable, SeqViolation (+16 more)

### Community 288 - "Plan de acción: los 5 primeros faltantes del informe de paridad"
Cohesion: 0.22
Nodes (8): A. Envío, descarga y guardado de archivos, B. Búsqueda dentro de un chat, C. Notificaciones de escritorio, Fuera de alcance por ahora, Los 5 puntos y dónde se resuelven, Orden y dependencias, Plan de acción: los 5 primeros faltantes del informe de paridad, Reparto sugerido

### Community 300 - "CodeKind"
Cohesion: 0.22
Nodes (9): CodeKind, App, Call, Email, FlashCall, MissedCall, Other, Sms (+1 more)

### Community 301 - "Rekey"
Cohesion: 0.40
Nodes (5): Rekey, Accepted, Committed, Idle, Requested

### Community 304 - "Informe de paridad: Parlo frente al cliente original"
Cohesion: 0.25
Nodes (7): Diferencias de comportamiento (paso 3 de la skill), Filas «partial» por no estar probadas contra Telegram real (`[sin real]`): 33, Informe de paridad: Parlo frente al cliente original, Los 10 faltantes más importantes (orden de construcción, solo must-haves en `no`), Puntuación por área (la más débil primero), Resumen, Siguientes pasos

### Community 305 - "ADR-0011: Transferencias (subidas y descargas a disco con progreso y cancelación)"
Cohesion: 0.50
Nodes (3): ADR-0011: Transferencias (subidas y descargas a disco con progreso y cancelación), Consecuencias, Decisiones

### Community 308 - "Step"
Cohesion: 0.29
Nodes (7): Step, Checking, Code, Password, Phone, Qr, SignUp

### Community 310 - "Bugs (replica-test, 2026-11-14)"
Cohesion: 0.40
Nodes (4): B-01 (S3) Texto de más de 4096 caracteres: burbuja condenada y aviso equivocado, Bugs (replica-test, 2026-11-14), Por comprobar (no reproducido), Por severidad

### Community 311 - "Body"
Cohesion: 0.50
Nodes (4): Body, Bubble, Emoji, Sticker

## Knowledge Gaps
- **554 isolated node(s):** `Backend`, `After`, `Around`, `Before`, `Latest` (+549 more)
  These have ≤1 connection - possible missing edges or undocumented components. (Counts symbols only; 1889 node(s) total have ≤1 connection when file, concept and rationale nodes are included.)
- **149 thin communities (<3 nodes) omitted from report** — run `graphify query` to explore isolated nodes.

## Suggested Questions
_Questions this graph is uniquely positioned to answer:_

- **Why does `Msg` connect `Msg` to `style.rs`, `tests_reactions.rs`, `ImageKey`, `PanelMsg`, `tests_files.rs`, `ChatMsg`, `.sticker_send_task`, `topics.rs`, `ForumMsg`, `tc-mock/src/forum.rs`, `view/mod.rs`, `send_dialog.rs`, `App`, `app.rs`, `.sticker_send_task`, `App`, `tests_sticker_anim.rs`, `tests_panel.rs`, `tests_qa.rs`, `tc-ui/src/media.rs`, `src/login.rs`, `.apply_auth`, `widgets.rs`, `Frame`, `tests_chat_actions.rs`, `pack.rs`, `App`, `view/audio.rs`, `FilesMsg`, `tests_audio.rs`, `tc-ui/src/audio.rs`, `reading.rs`, `view/login.rs`, `LoginMsg`, `main.rs`, `tests/secret.rs`, `tests_panel_shots.rs`, `Backend`, `context_menu.rs`, `sticker_anim.rs`?**
  _High betweenness centrality (0.104) - this node is a cross-community bridge._
- **Are the 31 inferred relationships involving `Msg` (e.g. with `ChatMsg` and `Message`) actually correct?**
  _`Msg` has 31 INFERRED edges - model-reasoned connections that need verification._
- **What connects `Backend`, `After`, `Around` to the rest of the system?**
  _554 weakly-connected nodes found - possible documentation gaps or missing edges._
- **Should `MockBackend` be split into smaller, more focused modules?**
  _Cohesion score 0.054426705370101594 - nodes in this community are weakly interconnected._
- **Why does `App` connect `App` to `style.rs`, `tests_reactions.rs`, `Msg`, `ImageKey`, `tests_files.rs`, `ChatMsg`, `topics.rs`, `tc-store/src/convert.rs`, `tc-core/src/lib.rs`, `ForumMsg`, `tc-mock/src/forum.rs`, `view/mod.rs`, `send_dialog.rs`, `app.rs`, `Login`, `tc-ui/src/stickers.rs`, `tests_panel.rs`, `tests_sticker_anim.rs`, `tests_qa.rs`, `src/login.rs`, `Picker`, `widgets.rs`, `tc-ui/src/anim.rs`, `tests_chat_actions.rs`, `App`, `view/audio.rs`, `FilesMsg`, `PanelMsg`, `tests_audio.rs`, `tc-ui/src/audio.rs`, `reading.rs`, `view/login.rs`, `main.rs`, `tests_panel_shots.rs`, `Backend`, `context_menu.rs`, `sticker_anim.rs`?**
  _High betweenness centrality (0.049) - this node is a cross-community bridge._
- **Are the 6 inferred relationships involving `App` (e.g. with `ChatActions` and `ForumUi`) actually correct?**
  _`App` has 6 INFERRED edges - model-reasoned connections that need verification._
- **Should `tokens.rs` be split into smaller, more focused modules?**
  _Cohesion score 0.03446327683615819 - nodes in this community are weakly interconnected._