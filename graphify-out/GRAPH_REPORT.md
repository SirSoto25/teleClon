# Graph Report - teleClon  (2026-11-06)

## Corpus Check
- 110 files · ~98,042 words
- Verdict: corpus is large enough that graph structure adds value.
- Unclassified: 4 file(s) not represented in the graph (top: (none) 2, .example 1, .csv 1)

## Summary
- 2517 nodes · 6128 edges · 112 communities (93 shown, 19 thin omitted)
- Extraction: 92% EXTRACTED · 8% INFERRED · 0% AMBIGUOUS · INFERRED: 502 edges (avg confidence: 0.76)
- Token cost: 0 input · 0 output

## Graph Freshness
- Built from commit: `7306d44c`
- Run `git rev-parse HEAD` and compare to check if the graph is stale.
- Run `graphify update .` after code changes (no API cost).

## Community Hubs (Navigation)
- style.rs
- MockBackend
- tokens.rs
- tc-mock/src/lib.rs
- Msg
- tc-theme/src/tests.rs
- BackendEvent
- App
- Sistema de diseño de teleClon — dirección «Brasa»
- conv.rs
- format.rs
- model.rs
- Rgb
- Message
- model.rs
- format.rs
- BackendResult
- .update
- fonts.rs
- api.rs
- anim.rs
- measure.rs
- Folder
- main.rs
- backend.rs
- 00-INDEX.md
- Arquitectura de teleClon (cliente propio de la red de Telegram)
- GrammersBackend
- icon
- api.rs
- tc-ui/src/tests.rs
- EntityKind
- super
- TextStyle
- TcSession
- MsgLayout
- Recon map: Telegram Desktop (Linux)
- dh.rs
- layout_message
- Compose
- Measurer
- StickerId
- Easing
- tc-telegram/src/auth.rs
- tc-telegram/src/lib.rs
- Entropy
- e2e.rs
- build_items
- ADR 0002 — Dirección visual «Brasa»
- ADR-0003: Arquitectura del cliente (workspace, backend tras trait, caché SQLite, E2E sans-IO)
- Registro de skills usadas
- config.rs
- crypto.rs
- conversation.rs
- Cache
- dialogs.rs
- tc-stickers/src/lib.rs
- tc-store/src/lib.rs
- Compositor (componente)
- tc-secret/src/session.rs
- Frame
- tc-store/src/lib.rs
- Cabecera de conversación
- Tipografía (Atkinson Hyperlegible, Bricolage, JetBrains Mono)
- net.rs
- pack.rs
- convert.rs
- tests/mod.rs
- flows.rs
- widgets.rs
- BackendResult
- SecretChat
- secret.rs
- messages.rs
- tl_visual_ttl.rs
- tl.rs
- UserId
- mapping.rs
- tc-stickers/src/tests.rs
- types.rs
- tc_core
- StickerError
- Store
- .u32
- Event
- EntityKind
- bubble
- message_row
- fixtures.rs
- Action
- BackendError
- media_from_tl
- key_panel
- CodeKind
- Writer
- ThemeKind
- TopLevel
- GroupPos
- tc-secret
- CLAUDE.md
- .claude/CLAUDE.md

## God Nodes (most connected - your core abstractions)
1. `Msg` - 122 edges
2. `App` - 95 edges
3. `MockBackend` - 57 edges
4. `c()` - 52 edges
5. `TcSession` - 44 edges
6. `Message` - 44 edges
7. `Brasa` - 41 edges
8. `BackendResult` - 41 edges
9. `Entropy` - 39 edges
10. `SecretChat` - 38 edges

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
- 1-file cycle: `crates/tc-theme/src/tokens.rs -> crates/tc-theme/src/tokens.rs`
- 2-file cycle: `crates/tc-telegram/src/auth.rs -> crates/tc-telegram/src/lib.rs -> crates/tc-telegram/src/auth.rs`
- 2-file cycle: `crates/tc-secret/src/dh.rs -> crates/tc-secret/src/testing.rs -> crates/tc-secret/src/dh.rs`

## Hyperedges (group relationships)
- **Implementaciones y decoradores de ChatBackend** — docs_architecture_mockbackend, docs_architecture_cachedbackend, docs_architecture_tc_telegram, docs_architecture_chatbackend [EXTRACTED 0.95]
- **Fundamentos visuales de Brasa** — docs_design_system_paleta_oklch, docs_design_system_tipografia, docs_design_system_espaciado_radios, docs_design_system_elevacion, docs_design_system_movimiento [EXTRACTED 0.95]
- **Decisiones de arquitectura ADR-0001 a ADR-0004** — docs_adr_0001_stack, docs_adr_0002_direccion_visual, docs_adr_0003_arquitectura, docs_adr_0004_stickers_y_chats_secretos [EXTRACTED 1.00]
- **Pila de chats secretos (cripto, claves y UI)** — docs_architecture_tc_secret, docs_architecture_keyring_boveda, docs_design_system_chat_secreto_ui [INFERRED 0.75]
- **Implementaciones de ChatBackend (Mock, Cached, Telegram)** — docs_adr_0003_arquitectura_chatbackend, docs_adr_0003_arquitectura_mockbackend, docs_adr_0003_arquitectura_cachedbackend, docs_adr_0001_stack_grammers [INFERRED 0.85]
- **Pila de chats secretos E2E** — docs_adr_0004_stickers_y_chats_secretos_secret_chats, docs_adr_0003_arquitectura_tc_secret, docs_adr_0001_stack_keyring, docs_adr_0004_stickers_y_chats_secretos_crypto_risk [INFERRED 0.85]

## Communities (112 total, 19 thin omitted)

### Community 0 - "style.rs"
Cohesion: 0.18
Nodes (32): app_style(), avatar_ink(), badge(), bare_input(), border_fill(), Brasa, bubble(), c() (+24 more)

### Community 1 - "MockBackend"
Cohesion: 0.06
Nodes (6): Inner, MOCK_2FA_PASSWORD, MOCK_LOGIN_CODE, MockBackend, MockConfig, State

### Community 2 - "tokens.rs"
Cohesion: 0.03
Nodes (58): AVATAR_LG, AVATAR_MD, AVATAR_RATIO, AVATAR_SM, AVATAR_XL, AVATAR_XS, BREAKPOINT_SPLIT, BREAKPOINT_TRIPLE (+50 more)

### Community 3 - "tc-mock/src/lib.rs"
Cohesion: 0.04
Nodes (48): Msg, AskDelete, Backend, CancelCompose, ClearSearch, CloseChat, CloseOverlay, ConfirmDelete (+40 more)

### Community 4 - "Msg"
Cohesion: 0.06
Nodes (38): Arquitectura de teleClon, ADR-0001 Stack, ADR-0002 Dirección visual, ADR-0003 Arquitectura, ADR-0004 Stickers y chats secretos, BackendEvent, ChatBackend (trait), Modelo de concurrencia (runtime tokio único, actores) (+30 more)

### Community 5 - "tc-theme/src/tests.rs"
Cohesion: 0.14
Nodes (23): AMPLITUDE, cell_segments(), ContourParams, .DEFAULT, .SECRET, contours(), field(), lattice() (+15 more)

### Community 6 - "BackendEvent"
Cohesion: 0.10
Nodes (20): BackendEvent, Connection, DialogUpserted, MessageEdited, MessageFailed, MessageNew, MessagesDeleted, MessageSent (+12 more)

### Community 8 - "Sistema de diseño de teleClon — dirección «Brasa»"
Cohesion: 0.16
Nodes (25): about_thirty_chats_sorted_with_pinned_first(), data_is_deterministic(), delete_updates_last_message_and_emits(), edit_only_own_messages(), forward_creates_messages_with_origin(), history_after_and_around(), long_history_paging_covers_everything_without_gaps(), mark_read_clears_unread() (+17 more)

### Community 9 - "conv.rs"
Cohesion: 0.05
Nodes (50): Body, Bubble, Emoji, Sticker, DATE_H, EDGE_H, emoji_only(), FWD_H (+42 more)

### Community 10 - "format.rs"
Cohesion: 0.06
Nodes (42): Timestamp, UserStatus, LastSeen, Offline, Online, Recently, BIG_CHAT, BIG_CHAT_LEN (+34 more)

### Community 11 - "model.rs"
Cohesion: 0.16
Nodes (12): INK, WHITE, AA_NORMAL, AA_UI, Alpha, Alphas, Rgb, DARK (+4 more)

### Community 12 - "Rgb"
Cohesion: 0.14
Nodes (17): RandomId, Message, SendState, Failed, Pending, Sent, Conversation, is_local() (+9 more)

### Community 13 - "Message"
Cohesion: 0.06
Nodes (18): AnimKey, BubbleY, ChatSwitch, Overlay, SendButton, backend_events(), BackendHandle, COMPOSER_ID (+10 more)

### Community 14 - "model.rs"
Cohesion: 0.13
Nodes (17): StickerSetId, Media, MediaKind, File, Gif, Photo, Video, Voice (+9 more)

### Community 15 - "format.rs"
Cohesion: 0.14
Nodes (15): MessagesApi, MsgId, PeerRef, Group, Secret, User, Anchor, After (+7 more)

### Community 16 - "BackendResult"
Cohesion: 0.22
Nodes (3): BubbleOpacity, Reply, error_text()

### Community 17 - ".update"
Cohesion: 0.11
Nodes (18): BUNDLED, DISPLAY, DISPLAY_FAMILY, EMOJI_FAMILY, family(), MONO, MONO_FALLBACK, MONO_FAMILY (+10 more)

### Community 18 - "fonts.rs"
Cohesion: 0.19
Nodes (6): base(), Font_, IcedMeasurer, Paragraph, SHAPING, WRAP

### Community 19 - "api.rs"
Cohesion: 0.16
Nodes (15): ADR-0002: Dirección visual Brasa, Dirección Brasa (grafito cálido + acento persimón), Controlled para chrome, innovative para burbujas y fondo, Movimiento con muelles 119-221 ms y movimiento reducido, OKLCH como fuente de verdad y contraste WCAG AA (144 pares), docs/design/system.md y tokens.json, Tipografía libre (Atkinson Hyperlegible Next, Bricolage Grotesque, JetBrains Mono) e iconos Phosphor, Agente diseño (opus/high) (+7 more)

### Community 20 - "anim.rs"
Cohesion: 0.10
Nodes (8): build(), enc(), in_gamut(), lum(), oklch_to_linear(), pairs_for(), ratio(), to_hex()

### Community 21 - "measure.rs"
Cohesion: 0.21
Nodes (8): SecretChatsApi, SecretChatId, KeyVisual, SecretChat, SecretChatState, Accepted, Discarded, Waiting

### Community 22 - "Folder"
Cohesion: 0.19
Nodes (7): Folder, All, Channels, Groups, Personal, Unread, sort_dialogs()

### Community 23 - "main.rs"
Cohesion: 0.11
Nodes (11): demo_script(), env_flag(), env_u64(), main(), DemoStep, Do, Exit, ScrollUp (+3 more)

### Community 24 - "backend.rs"
Cohesion: 0.16
Nodes (11): MAX_TEXT, SECRET_UNAVAILABLE, channel_of(), CHANNEL_OFFSET, decode_group(), GroupTarget, Channel, Chat (+3 more)

### Community 25 - "00-INDEX.md"
Cohesion: 0.14
Nodes (17): Backend, Compose, Edit, Normal, Overlay, Delete, Key, boot() (+9 more)

### Community 26 - "Arquitectura de teleClon (cliente propio de la red de Telegram)"
Cohesion: 0.24
Nodes (7): handle(), HANDLES, icon(), icon_fill(), icons!, view(), header()

### Community 27 - "GrammersBackend"
Cohesion: 0.06
Nodes (7): GrammersBackend, history_params(), Inner, messages_of(), reply_to(), flatten_updates(), now_secs()

### Community 28 - "icon"
Cohesion: 0.19
Nodes (12): ADR-0001: Stack técnico, grammers (MTProto en Rust), iced (UI en Rust, wgpu + tiny-skia), keyring (claves y sesión), rusqlite bundled con migraciones por PRAGMA user_version, tc-secret: chats secretos sans-IO con RustCrypto, tc-stickers: WebP, TGS con rlottie, WebM aplazado, ADR-0004: Stickers y chats secretos en el alcance (+4 more)

### Community 29 - "api.rs"
Cohesion: 0.19
Nodes (5): ChatBackend, DialogsApi, MediaApi, StickersApi, DialogsPage

### Community 30 - "tc-ui/src/tests.rs"
Cohesion: 0.24
Nodes (12): ADR-0003: Arquitectura del cliente, Orden de construcción M1.1-M1.4, M2.1, M2.7, CachedBackend (caché como decorador, tc-store), Trait ChatBackend (sub-traits + events()), Modelo de concurrencia (tokio, hilo SQLite, canal acotado, Resync), MockBackend determinista, SQLite por cuenta (17 tablas + FTS5, WAL, chat_key), Scroll virtual propio (Fenwick, ventana ~600 mensajes) (+4 more)

### Community 31 - "EntityKind"
Cohesion: 0.24
Nodes (13): Registro de agentes, Agente arquitecto (opus/high), Agente core+mock (sonnet/medium), Agente QA (pendiente), Agente theme (sonnet/medium), Agente UI (opus/high), Bitácora (append-only), Bug iced 0.14: scrollable.direction antes de anchor_bottom (+5 more)

### Community 32 - "super"
Cohesion: 0.12
Nodes (9): below_stays_in_range(), is_deterministic(), settings_crud_and_prefix(), Store, RECENT_CAP, SET_COLS, set_from_row(), STICKER_COLS (+1 more)

### Community 33 - "TextStyle"
Cohesion: 0.17
Nodes (9): MotionPreference, Full, Reduced, Property, Opacity, Position, Scale, Anim (+1 more)

### Community 34 - "TcSession"
Cohesion: 0.07
Nodes (17): changes_mark_dirty_and_reset_forgets_everything(), debug_never_prints_the_auth_key(), file_store_is_private_and_atomic(), FileSessionStore, MemorySessionStore, rejects_garbage_and_unknown_versions(), roundtrip_keeps_keys_peers_and_state(), sample() (+9 more)

### Community 35 - "MsgLayout"
Cohesion: 0.21
Nodes (11): Índice maestro de teleClon, architecture.md (crates, ChatBackend, SQLite, F01-F18), graphify-out (graph.html, GRAPH_REPORT.md, graph.json), Reglas de trabajo (log, ADR, clean-room, credenciales, regenerar grafo), Credenciales api_id/api_hash en .env local gitignored, graphify: instalación y uso, graphifyy en ./.venv local, Registro de skills usadas (+3 more)

### Community 36 - "Recon map: Telegram Desktop (Linux)"
Cohesion: 0.18
Nodes (11): EntityKind, Bold, Code, Hashtag, Italic, Mention, Pre, Spoiler (+3 more)

### Community 37 - "dh.rs"
Cohesion: 0.05
Nodes (32): dh_matches_independent_vector(), DhParams, DhSecret, exp(), FP, GA, GB, generate_mixes_server_random_and_rejects_bad_peer() (+24 more)

### Community 38 - "layout_message"
Cohesion: 0.15
Nodes (13): HIGHLIGHT_FADE_MS, REDUCED_OPACITY_MAX_MS, SKELETON_PULSE_PERIOD_MS, Spring, .ESTANDAR, .RAPIDO, .REBOTE, .SUAVE (+5 more)

### Community 39 - "Compose"
Cohesion: 0.18
Nodes (11): BODY, CAPTION, CODE, DISPLAY, EMOJI_ONLY, HEADING, META, TextStyle (+3 more)

### Community 40 - "Measurer"
Cohesion: 0.36
Nodes (5): Animator<K>, Curve, interrumpible_hereda_posicion(), movimiento_reducido_salta_posicion_y_limita_opacidad(), muelle_converge_y_termina()

### Community 41 - "StickerId"
Cohesion: 0.24
Nodes (4): StickerId, DEFAULT_RECENTS, Favorites, Recents

### Community 42 - "Easing"
Cohesion: 0.29
Nodes (5): Easing, .ESTANDAR, .RAPIDO, .SALIDA, .SUAVE

### Community 43 - "tc-telegram/src/auth.rs"
Cohesion: 0.09
Nodes (14): AuthInner, code_kind(), code_types_map(), GrammersBackend, Inner, migrate_dc(), next_kind(), phone_validation() (+6 more)

### Community 44 - "tc-telegram/src/lib.rs"
Cohesion: 0.07
Nodes (13): TelegramConfig, Feed, None, Pool, Source, GrammersBackend, has_login_token(), Inner (+5 more)

### Community 45 - "Entropy"
Cohesion: 0.17
Nodes (17): Role, Initiator, Responder, Entropy, &mut E, random_i64(), Message, Abort (+9 more)

### Community 46 - "e2e.rs"
Cohesion: 0.14
Nodes (38): DecryptedMessage, Service, DecryptedMessageLayer, PlainMessage, chat_n(), concurrent_rekey_converges_on_one_instance(), decreasing_in_seq_aborts_chat(), dh() (+30 more)

### Community 47 - "build_items"
Cohesion: 0.31
Nodes (9): replica/features.csv, Rasgos de marca (avatares cuadrado redondeado, burbujas sin cola, fondo de isolíneas), Agente recon (sonnet/medium), Recon map: Telegram Desktop (Linux), Componentes de UI (Burbuja de mensaje, Fila de chat, Compositor...), Core loop (chats, enviar/recibir, sincronizado), Matriz de funciones (features.csv, must/should/could/skip), Flujos F01-F16 (+1 more)

### Community 48 - "ADR 0002 — Dirección visual «Brasa»"
Cohesion: 0.43
Nodes (3): Fixed, Measurer, Run

### Community 49 - "ADR-0003: Arquitectura del cliente (workspace, backend tras trait, caché SQLite, E2E sans-IO)"
Cohesion: 0.39
Nodes (9): tc-app, tc-core, tc-mock, tc-secret, tc-stickers, tc-store, tc-telegram, tc-theme (+1 more)

### Community 50 - "Registro de skills usadas"
Cohesion: 0.50
Nodes (4): GroupKind, Basic, Channel, Super

### Community 51 - "config.rs"
Cohesion: 0.16
Nodes (11): ApiHash, ConfigError, Invalid, Missing, debug_hides_the_hash(), ENV_API_HASH, ENV_API_ID, FAKE_HASH (+3 more)

### Community 52 - "crypto.rs"
Cohesion: 0.10
Nodes (27): decrypt(), derive(), encrypt(), encrypt_with_padding(), HEADER_LEN, ige_decrypt(), ige_encrypt(), IgeDec (+19 more)

### Community 53 - "conversation.rs"
Cohesion: 0.11
Nodes (7): folder_icon(), NAME_CHARS, PREVIEW_CHARS, ROW_H, hfill(), hgap(), vgap()

### Community 54 - "Cache"
Cohesion: 0.14
Nodes (12): draft_from_tl(), is_muted(), Applied, apply_batch(), apply_update(), Cache, deleted(), DialogList (+4 more)

### Community 55 - "dialogs.rs"
Cohesion: 0.12
Nodes (14): DIALOG_COLS, dialog_from_row(), draft_from_row(), drafts_set_clear_list(), list_orders_pinned_then_activity_and_filters_archived(), mark_read_and_missing_dialog_errors(), notify_scopes(), Store (+6 more)

### Community 56 - "tc-stickers/src/lib.rs"
Cohesion: 0.67
Nodes (3): Disposición (raíl, lista, conversación, panel info), Lista de chats (componente), Barra lateral (raíl de carpetas)

### Community 59 - "tc-secret/src/session.rs"
Cohesion: 0.04
Nodes (49): OsEntropy, Result, SecretError, Entropy, FingerprintMismatch, InvalidDhConfig, InvalidPublicValue, InvalidState (+41 more)

### Community 60 - "Frame"
Cohesion: 0.14
Nodes (4): CacheKey, DEFAULT_CACHE_BYTES, FrameCache, Frame

### Community 61 - "tc-store/src/lib.rs"
Cohesion: 0.08
Nodes (21): account_db_path(), APP_DIR, default_data_dir(), MIGRATIONS, SCHEMA_VERSION, Store, touch_private(), Error (+13 more)

### Community 64 - "net.rs"
Cohesion: 0.11
Nodes (15): ConnTracker, errors_map_to_domain(), flood_wait_policy_sleeps_only_short_waits(), is_network(), is_session_gone(), Net, only_specific_401s_end_the_session(), PoolTransport (+7 more)

### Community 65 - "pack.rs"
Cohesion: 0.10
Nodes (25): Def, DEFS, dots(), index_of(), Layer, len(), NATIVE_SIZE, P (+17 more)

### Community 66 - "convert.rs"
Cohesion: 0.12
Nodes (16): dialog_from_tl(), dialog_stub(), DialogCtx, draft_date(), entities_from_tl(), entities_to_tl(), entity_parts(), forward_name() (+8 more)

### Community 67 - "tests/mod.rs"
Cohesion: 0.10
Nodes (13): UpdateBatch, UpdateSource, UpdatesReceiver, Transport, update_pump_maps_events_and_recovers_from_errors(), cfg(), Fake, Handler (+5 more)

### Community 68 - "flows.rs"
Cohesion: 0.16
Nodes (23): authorization(), state(), updates(), user(), user_enum(), dialogs_page(), dialogs_paginate_with_server_offsets(), edit_delete_react_and_read() (+15 more)

### Community 69 - "widgets.rs"
Cohesion: 0.12
Nodes (9): avatar_radius(), badge_at(), avatar(), badge(), contour_bg(), ContourBg, icon_button(), t() (+1 more)

### Community 70 - "BackendResult"
Cohesion: 0.13
Nodes (16): AuthApi, AuthState, Checking, LoggedIn, LoggedOut, SignUpRequired, WaitingCode, WaitingPassword (+8 more)

### Community 71 - "SecretChat"
Cohesion: 0.12
Nodes (6): SecretChat, State, Discarded, Incoming, Ready, Requested

### Community 72 - "secret.rs"
Cohesion: 0.15
Nodes (15): chat(), CHAT_COLS, chat_from_row(), chat_roundtrip_and_fk(), messages_idempotent_blob_roundtrip_and_page(), msg(), MSG_COLS, msg_from_row() (+7 more)

### Community 73 - "messages.rs"
Cohesion: 0.15
Nodes (14): COLS, fill(), from_row(), fts_query(), fts_search_accents_prefix_scope_and_delete(), ids(), pagination_by_anchor(), pending_becomes_confirmed_by_random_id() (+6 more)

### Community 74 - "tl_visual_ttl.rs"
Cohesion: 0.09
Nodes (6): SelfDestructTimer, KeyVisualization, layer(), tl_known_encoding(), tl_rejects_unknown_and_trailing(), tl_round_trip_all_supported_constructors()

### Community 75 - "tl.rs"
Cohesion: 0.08
Nodes (24): ID_ABORT_KEY, ID_ACCEPT_KEY, ID_COMMIT_KEY, ID_DELETE, ID_FLUSH, ID_LAYER, ID_MEDIA_EMPTY, ID_MESSAGE (+16 more)

### Community 76 - "UserId"
Cohesion: 0.15
Nodes (7): ProfileApi, UserId, display_name_joins_parts(), User, PeerStore, user_from_tl(), user_status()

### Community 77 - "mapping.rs"
Cohesion: 0.17
Nodes (19): GroupId, Group, channel_group(), chat_group(), group_ids_roundtrip_and_never_collide(), group_from_tl(), channel(), chat() (+11 more)

### Community 78 - "tc-stickers/src/tests.rs"
Cohesion: 0.13
Nodes (15): cache_evicts_least_recently_used_by_bytes(), cache_get_or_insert_computes_once(), gradient(), key(), LOTTIE, pack_stickers_are_distinct_and_survive_webp(), render_without_lottie_reports_disabled(), tgs_bomb_is_rejected() (+7 more)

### Community 79 - "types.rs"
Cohesion: 0.10
Nodes (14): scope_parts(), Anchor, After, Around, Before, Latest, str_enum!, MediaRow (+6 more)

### Community 80 - "tc_core"
Cohesion: 0.11
Nodes (9): random_i64(), BIG_FILE, crate::Inner, GrammersBackend, input_media(), MAX_FILE, mime_for(), PART_SIZE (+1 more)

### Community 81 - "StickerError"
Cohesion: 0.13
Nodes (15): StickerError, BadSize, FrameOutOfRange, Gzip, Image, LottieDisabled, LottieLoad, NotLottie (+7 more)

### Community 82 - "Store"
Cohesion: 0.18
Nodes (7): favorites(), recents_order_dedupe_cap_and_cascade(), seed(), sets_and_stickers(), Store, StickerRow, StickerSetRow

### Community 83 - ".u32"
Cohesion: 0.32
Nodes (10): Message, Entity, read_action(), read_entity(), read_message(), Reader, Reader<'a>, write_action() (+2 more)

### Community 84 - "Event"
Cohesion: 0.11
Nodes (18): Event, Deleted, Discarded, Duplicate, HistoryFlushed, KeyChanged, PeerLayer, PeerLayerNewer (+10 more)

### Community 85 - "EntityKind"
Cohesion: 0.11
Nodes (17): EntityKind, Blockquote, Bold, BotCommand, Code, CustomEmoji, Email, Hashtag (+9 more)

### Community 86 - "bubble"
Cohesion: 0.19
Nodes (15): ellipsize(), preview(), to_spans(), font_of(), rounded_fill(), ui_font(), weight(), with_weight() (+7 more)

### Community 87 - "message_row"
Cohesion: 0.19
Nodes (13): now(), fill(), body(), date_pill(), edge(), extras(), hairline(), header_status() (+5 more)

### Community 88 - "fixtures.rs"
Cohesion: 0.25
Nodes (13): key_visualization_matches_independent_vector(), dialog(), hex(), message(), msg_enum(), notify(), password(), peer_channel() (+5 more)

### Community 89 - "Action"
Cohesion: 0.14
Nodes (14): Action, AbortKey, AcceptKey, CommitKey, DeleteMessages, FlushHistory, Noop, NotifyLayer (+6 more)

### Community 90 - "BackendError"
Cohesion: 0.18
Nodes (10): BackendError, Crypto, FloodWait, Forbidden, Internal, InvalidInput, Network, NotFound (+2 more)

### Community 91 - "media_from_tl"
Cohesion: 0.31
Nodes (4): media_from_document(), media_from_tl(), photo_dims(), sticker_from_document()

### Community 92 - "key_panel"
Cohesion: 0.39
Nodes (6): empty(), view(), App, key_panel(), overlay(), toasts()

### Community 93 - "CodeKind"
Cohesion: 0.25
Nodes (8): CodeKind, App, Call, Email, FlashCall, MissedCall, Other, Sms

### Community 95 - "ThemeKind"
Cohesion: 0.33
Nodes (4): bubble_out_gradient(), ThemeKind, Dark, Light

### Community 96 - "TopLevel"
Cohesion: 0.40
Nodes (3): TopLevel, Layer, LegacyService

### Community 97 - "GroupPos"
Cohesion: 0.33
Nodes (6): bubble_radius(), GroupPos, First, Last, Middle, Single

### Community 98 - "tc-secret"
Cohesion: 0.40
Nodes (4): Garantías locales, Lo que NO está hecho o NO está verificado, Módulos, tc-secret

## Knowledge Gaps
- **300 isolated node(s):** `App`, `Sms`, `Call`, `FlashCall`, `MissedCall` (+295 more)
  These have ≤1 connection - possible missing edges or undocumented components. (Counts symbols only; 834 node(s) total have ≤1 connection when file, concept and rationale nodes are included.)
- **19 thin communities (<3 nodes) omitted from report** — run `graphify query` to explore isolated nodes.

## Suggested Questions
_Questions this graph is uniquely positioned to answer:_

- **Why does `MockBackend` connect `MockBackend` to `Sistema de diseño de teleClon — dirección «Brasa»`, `00-INDEX.md`?**
  _High betweenness centrality (0.062) - this node is a cross-community bridge._
- **Are the 21 inferred relationships involving `c()` (e.g. with `body()` and `bubble()`) actually correct?**
  _`c()` has 21 INFERRED edges - model-reasoned connections that need verification._
- **What connects `App`, `Sms`, `Call` to the rest of the system?**
  _300 weakly-connected nodes found - possible documentation gaps or missing edges._
- **Should `MockBackend` be split into smaller, more focused modules?**
  _Cohesion score 0.06438213914849429 - nodes in this community are weakly interconnected._
- **Why does `Msg` connect `tc-mock/src/lib.rs` to `style.rs`, `Arquitectura de teleClon (cliente propio de la red de Telegram)`, `widgets.rs`, `App`, `Message`, `BackendResult`, `conversation.rs`, `Folder`, `main.rs`, `message_row`, `00-INDEX.md`, `bubble`, `key_panel`?**
  _High betweenness centrality (0.049) - this node is a cross-community bridge._
- **Should `tokens.rs` be split into smaller, more focused modules?**
  _Cohesion score 0.03446327683615819 - nodes in this community are weakly interconnected._
- **Why does `Result` connect `tc-secret/src/session.rs` to `super`, `dh.rs`, `secret.rs`, `messages.rs`, `tl.rs`, `crypto.rs`, `dialogs.rs`, `tc-store/src/lib.rs`?**
  _High betweenness centrality (0.048) - this node is a cross-community bridge._