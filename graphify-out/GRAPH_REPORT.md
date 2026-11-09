# Graph Report - teleClon  (2026-11-09)

## Corpus Check
- 155 files · ~157,639 words
- Verdict: corpus is large enough that graph structure adds value.
- Unclassified: 11 file(s) not represented in the graph (top: (none) 2, .example 1, .desktop 1)

## Summary
- 3936 nodes · 9858 edges · 219 communities (131 shown, 88 thin omitted)
- Extraction: 92% EXTRACTED · 8% INFERRED · 0% AMBIGUOUS · INFERRED: 750 edges (avg confidence: 0.76)
- Token cost: 0 input · 0 output

## Graph Freshness
- Built from commit: `dd028ef6`
- Run `git rev-parse HEAD` and compare to check if the graph is stale.
- Run `graphify update .` after code changes (no API cost).

## Community Hubs (Navigation)
- style.rs
- MockBackend
- tokens.rs
- tc-mock/src/lib.rs
- Msg
- f
- BackendEvent
- tc-telegram/src/secret.rs
- Sistema de diseño de teleClon — dirección «Brasa»
- conv.rs
- format.rs
- model.rs
- tc-ui/src/media.rs
- ci.yml
- model.rs
- Outbound
- .update_inner
- .update
- measure.rs
- api.rs
- anim.rs
- SecretChatId
- super
- app.rs
- backend.rs
- tc-telegram/src/auth.rs
- widgets.rs
- GrammersBackend
- icon
- Inner
- tc-ui/src/tests.rs
- EntityKind
- groups.rs
- load_dialog
- TcSession
- MsgLayout
- VaultResult
- dh.rs
- layout_message
- tests_m2.rs
- with
- StickerId
- tc-ui/src/stickers.rs
- tc-telegram/src/auth.rs
- tc-telegram/src/lib.rs
- Entropy
- e2e.rs
- build_items
- peers.rs
- ADR-0003: Arquitectura del cliente (workspace, backend tras trait, caché SQLite, E2E sans-IO)
- TextStyle
- config.rs
- crypto.rs
- sidebar.rs
- Cache
- dialogs.rs
- tc-stickers/src/lib.rs
- tc-store/src/lib.rs
- Compositor (componente)
- SecretError
- Frame
- tc-store/src/lib.rs
- Cabecera de conversación
- Tipografía (Atkinson Hyperlegible, Bricolage, JetBrains Mono)
- net.rs
- pack.rs
- convert.rs
- conversation.rs
- tests/mod.rs
- anim.rs
- media_cache.rs
- SecretChat
- secret.rs
- messages.rs
- Folder
- tl.rs
- cached.rs
- EntityKind
- StickerError
- types.rs
- .download_location
- BackendResult
- Store
- .u32
- tl_visual_ttl.rs
- EntityKind
- SessionSnapshot
- view/login.rs
- src/login.rs
- Action
- tc-mock/src/lib.rs
- main.rs
- tc-ui/src/tests.rs
- MediaCacheLayer<B>
- Writer
- cached_tests.rs
- EnvFile
- 2. Casos por flujo
- tc-secret
- uploads.rs
- CLAUDE.md
- .claude/CLAUDE.md
- StoreError
- tests/secret.rs
- tc-stickers/src/tests.rs
- AuthState
- folders.rs
- files.rs
- vault.rs
- Event
- Message
- Nav
- jobs
- reactions.rs
- Marca propia — Parlo
- BackendResult
- tc-store
- Informe de barrido de marca
- env
- release
- .send_and_confirm
- tests/media.rs
- stores.rs
- GroupPos
- windows
- tests/snapshot.rs
- ConnectionState
- meta
- 0005-nombre-parlo.md
- images.rs
- tests/mod.rs
- Store
- tc-store/src/convert.rs
- Frame
- apply_history
- media_key
- GroupRow
- State
- icon
- .history
- flatten_updates
- input_media
- .new
- presence
- App
- on
- build.rs
- .start
- env
- build-macos.sh
- select.rs
- build-linux.sh
- MotionPreference
- LayoutCache
- TopLevel

## God Nodes (most connected - your core abstractions)
1. `Msg` - 197 edges
2. `App` - 148 edges
3. `BackendResult` - 81 edges
4. `c()` - 80 edges
5. `PeerRef` - 76 edges
6. `MockBackend` - 64 edges
7. `Brasa` - 57 edges
8. `Message` - 55 edges
9. `Store` - 54 edges
10. `CachedBackend<B>` - 48 edges

## Surprising Connections (you probably didn't know these)
- `Clean-room (sin código, assets ni marca de Telegram)` --semantically_similar_to--> `Reglas de trabajo (log, ADR, clean-room, credenciales, regenerar grafo)`  [INFERRED] [semantically similar]
  README.md → docs/00-INDEX.md
- `Dirección Brasa (grafito cálido + acento persimón)` --rationale_for--> `Clean-room (sin código, assets ni marca de Telegram)`  [INFERRED]
  docs/adr/0002-direccion-visual.md → README.md
- `display_name_joins_parts()` --references--> `UserId`  [INFERRED]
  crates/tc-core/src/model.rs → crates/tc-core/src/ids.rs
- `radios_de_union()` --calls--> `bubble_radius()`  [INFERRED]
  crates/tc-ui/src/conv.rs → crates/tc-ui/src/style.rs
- `Fuera de alcance (red MTProto, marca, llamadas, historias, bots, pagos)` --conceptually_related_to--> `grammers (MTProto en Rust)`  [INFERRED]
  replica/recon.md → docs/adr/0001-stack.md

## Import Cycles
- 1-file cycle: `crates/tc-store/src/vault.rs -> crates/tc-store/src/vault.rs`
- 1-file cycle: `crates/tc-theme/src/tokens.rs -> crates/tc-theme/src/tokens.rs`
- 2-file cycle: `crates/tc-secret/src/session.rs -> crates/tc-secret/src/session/snapshot.rs -> crates/tc-secret/src/session.rs`
- 2-file cycle: `crates/tc-ui/src/app.rs -> crates/tc-ui/src/login.rs -> crates/tc-ui/src/app.rs`
- 2-file cycle: `crates/tc-ui/src/app.rs -> crates/tc-ui/src/media.rs -> crates/tc-ui/src/app.rs`
- 2-file cycle: `crates/tc-telegram/src/auth.rs -> crates/tc-telegram/src/lib.rs -> crates/tc-telegram/src/auth.rs`
- 2-file cycle: `crates/tc-telegram/src/lib.rs -> crates/tc-telegram/src/secret.rs -> crates/tc-telegram/src/lib.rs`
- 2-file cycle: `crates/tc-secret/src/dh.rs -> crates/tc-secret/src/testing.rs -> crates/tc-secret/src/dh.rs`

## Hyperedges (group relationships)
- **Implementaciones y decoradores de ChatBackend** — docs_architecture_mockbackend, docs_architecture_cachedbackend, docs_architecture_tc_telegram, docs_architecture_chatbackend [EXTRACTED 0.95]
- **Fundamentos visuales de Brasa** — docs_design_system_paleta_oklch, docs_design_system_tipografia, docs_design_system_espaciado_radios, docs_design_system_elevacion, docs_design_system_movimiento [EXTRACTED 0.95]
- **Decisiones de arquitectura ADR-0001 a ADR-0004** — docs_adr_0001_stack, docs_adr_0002_direccion_visual, docs_adr_0003_arquitectura, docs_adr_0004_stickers_y_chats_secretos [EXTRACTED 1.00]
- **Pila de chats secretos (cripto, claves y UI)** — docs_architecture_tc_secret, docs_architecture_keyring_boveda, docs_design_system_chat_secreto_ui [INFERRED 0.75]
- **Implementaciones de ChatBackend (Mock, Cached, Telegram)** — docs_adr_0003_arquitectura_chatbackend, docs_adr_0003_arquitectura_mockbackend, docs_adr_0003_arquitectura_cachedbackend, docs_adr_0001_stack_grammers [INFERRED 0.85]
- **Pila de chats secretos E2E** — docs_adr_0004_stickers_y_chats_secretos_secret_chats, docs_adr_0003_arquitectura_tc_secret, docs_adr_0001_stack_keyring, docs_adr_0004_stickers_y_chats_secretos_crypto_risk [INFERRED 0.85]

## Communities (219 total, 88 thin omitted)

### Community 0 - "style.rs"
Cohesion: 0.16
Nodes (39): app_style(), avatar_ink(), badge(), bare_input(), border_fill(), Brasa, bubble(), c() (+31 more)

### Community 1 - "MockBackend"
Cohesion: 0.10
Nodes (3): incoming_text(), Inner, MockBackend

### Community 2 - "tokens.rs"
Cohesion: 0.03
Nodes (58): AVATAR_LG, AVATAR_MD, AVATAR_RATIO, AVATAR_SM, AVATAR_XL, AVATAR_XS, BREAKPOINT_SPLIT, BREAKPOINT_TRIPLE (+50 more)

### Community 3 - "tc-mock/src/lib.rs"
Cohesion: 0.03
Nodes (67): Msg, AcceptSecret, AskDelete, AskDiscardSecret, Backend, CancelCompose, ClearSearch, CloseChat (+59 more)

### Community 4 - "Msg"
Cohesion: 0.06
Nodes (38): Arquitectura de teleClon, ADR-0001 Stack, ADR-0002 Dirección visual, ADR-0003 Arquitectura, ADR-0004 Stickers y chats secretos, BackendEvent, ChatBackend (trait), Modelo de concurrencia (runtime tokio único, actores) (+30 more)

### Community 5 - "f"
Cohesion: 0.14
Nodes (23): AMPLITUDE, cell_segments(), ContourParams, .DEFAULT, .SECRET, contours(), field(), lattice() (+15 more)

### Community 6 - "BackendEvent"
Cohesion: 0.07
Nodes (27): BackendError, Crypto, FloodWait, Forbidden, Internal, InvalidInput, Network, NotFound (+19 more)

### Community 7 - "tc-telegram/src/secret.rs"
Cohesion: 0.06
Nodes (23): Entity, utf16_starts(), now_secs(), Chat, crypto(), EARLY_CAP, entities_from_secret(), entities_round_trip_through_utf16() (+15 more)

### Community 8 - "Sistema de diseño de teleClon — dirección «Brasa»"
Cohesion: 0.16
Nodes (25): about_thirty_chats_sorted_with_pinned_first(), data_is_deterministic(), delete_updates_last_message_and_emits(), edit_only_own_messages(), forward_creates_messages_with_origin(), history_after_and_around(), long_history_paging_covers_everything_without_gaps(), mark_read_clears_unread() (+17 more)

### Community 9 - "conv.rs"
Cohesion: 0.06
Nodes (32): DATE_H, EDGE_H, FWD_H, GAP_GROUP, GAP_IN_GROUP, GROUP_AVATAR, GROUP_AVATAR_COL, GROUP_WINDOW_SECS (+24 more)

### Community 10 - "format.rs"
Cohesion: 0.15
Nodes (26): Timestamp, UserStatus, LastSeen, Offline, Online, Recently, date_separator(), day_key() (+18 more)

### Community 11 - "model.rs"
Cohesion: 0.12
Nodes (16): INK, WHITE, AA_NORMAL, AA_UI, Alpha, Alphas, Rgb, bubble_out_gradient() (+8 more)

### Community 12 - "tc-ui/src/media.rs"
Cohesion: 0.07
Nodes (29): AVATAR_PX, avatars_are_square_and_downscaled(), bubble_aspect(), BUBBLE_ASPECT, bubble_photos_are_cropped_to_the_layout_ratio(), BUBBLE_PX, CACHE_BYTES, cache_tracks_loads_failures_and_evicts_least_recently_used() (+21 more)

### Community 13 - "ci.yml"
Cohesion: 0.07
Nodes (28): name, runs-on, steps, strategy, concurrency, cancel-in-progress, group, env (+20 more)

### Community 14 - "model.rs"
Cohesion: 0.07
Nodes (29): StickerSetId, Anchor, After, Around, Before, Latest, display_name_joins_parts(), FileKind (+21 more)

### Community 15 - "Outbound"
Cohesion: 0.40
Nodes (5): Outbound, AcceptEncryption, DiscardEncryption, RequestEncryption, SendEncrypted

### Community 16 - ".update_inner"
Cohesion: 0.13
Nodes (8): AnimKey, BubbleOpacity, BubbleY, ChatSwitch, Overlay, SendButton, Reply, error_text()

### Community 17 - ".update"
Cohesion: 0.11
Nodes (18): BUNDLED, DISPLAY, DISPLAY_FAMILY, EMOJI_FAMILY, family(), MONO, MONO_FALLBACK, MONO_FAMILY (+10 more)

### Community 18 - "measure.rs"
Cohesion: 0.13
Nodes (10): base(), Font_, IcedMeasurer, Paragraph, SHAPING, to_spans(), WRAP, font_of() (+2 more)

### Community 19 - "api.rs"
Cohesion: 0.16
Nodes (15): ADR-0002: Dirección visual Brasa, Dirección Brasa (grafito cálido + acento persimón), Controlled para chrome, innovative para burbujas y fondo, Movimiento con muelles 119-221 ms y movimiento reducido, OKLCH como fuente de verdad y contraste WCAG AA (144 pares), docs/design/system.md y tokens.json, Tipografía libre (Atkinson Hyperlegible Next, Bricolage Grotesque, JetBrains Mono) e iconos Phosphor, Agente diseño (opus/high) (+7 more)

### Community 20 - "anim.rs"
Cohesion: 0.06
Nodes (20): classify(), main(), report(), scan(), build(), enc(), in_gamut(), lum() (+12 more)

### Community 21 - "SecretChatId"
Cohesion: 0.20
Nodes (8): SecretChatsApi, SecretChatId, KeyVisual, SecretChat, SecretChatState, Accepted, Discarded, Waiting

### Community 22 - "super"
Cohesion: 0.11
Nodes (13): key_is_created_once_in_the_vault_and_reused(), roundtrip_and_every_kind_of_tampering_is_rejected(), SEAL_KEY_LEN, Sealed, SealError, BadKey, Entropy, Rejected (+5 more)

### Community 23 - "app.rs"
Cohesion: 0.07
Nodes (17): auth_events(), AuthHandle, backend_events(), BackendHandle, Compose, Edit, Normal, COMPOSER_ID (+9 more)

### Community 24 - "backend.rs"
Cohesion: 0.08
Nodes (30): MAX_TEXT, channel_group(), channel_of(), CHANNEL_OFFSET, chat_group(), decode_group(), group_ids_roundtrip_and_never_collide(), GroupTarget (+22 more)

### Community 25 - "tc-telegram/src/auth.rs"
Cohesion: 0.10
Nodes (11): AuthInner, code_kind(), code_types_map(), migrate_dc(), next_kind(), phone_validation(), qr_url(), qr_url_is_unpadded_base64url() (+3 more)

### Community 26 - "widgets.rs"
Cohesion: 0.09
Nodes (6): avatar_radius(), avatar(), avatar_photo(), contour_bg(), ContourBg, icon_button()

### Community 28 - "icon"
Cohesion: 0.19
Nodes (12): ADR-0001: Stack técnico, grammers (MTProto en Rust), iced (UI en Rust, wgpu + tiny-skia), keyring (claves y sesión), rusqlite bundled con migraciones por PRAGMA user_version, tc-secret: chats secretos sans-IO con RustCrypto, tc-stickers: WebP, TGS con rlottie, WebM aplazado, ADR-0004: Stickers y chats secretos en el alcance (+4 more)

### Community 29 - "Inner"
Cohesion: 0.14
Nodes (5): Inner, pump(), run_source(), startup(), wait_logged_in()

### Community 30 - "tc-ui/src/tests.rs"
Cohesion: 0.24
Nodes (12): ADR-0003: Arquitectura del cliente, Orden de construcción M1.1-M1.4, M2.1, M2.7, CachedBackend (caché como decorador, tc-store), Trait ChatBackend (sub-traits + events()), Modelo de concurrencia (tokio, hilo SQLite, canal acotado, Resync), MockBackend determinista, SQLite por cuenta (17 tablas + FTS5, WAL, chat_key), Scroll virtual propio (Fenwick, ventana ~600 mensajes) (+4 more)

### Community 31 - "EntityKind"
Cohesion: 0.24
Nodes (13): Registro de agentes, Agente arquitecto (opus/high), Agente core+mock (sonnet/medium), Agente QA (pendiente), Agente theme (sonnet/medium), Agente UI (opus/high), Bitácora (append-only), Bug iced 0.14: scrollable.direction antes de anchor_bottom (+5 more)

### Community 32 - "groups.rs"
Cohesion: 0.13
Nodes (6): below_stays_in_range(), is_deterministic(), COLS, from_row(), settings_crud_and_prefix(), Store

### Community 33 - "load_dialog"
Cohesion: 0.24
Nodes (13): apply_dialogs(), cached_dialogs(), dialogs_complete_key(), ensure_dialog(), find_chat_key(), is_secret(), known_title(), load_dialog() (+5 more)

### Community 34 - "TcSession"
Cohesion: 0.09
Nodes (10): changes_mark_dirty_and_reset_forgets_everything(), debug_never_prints_the_auth_key(), MemorySessionStore, rejects_garbage_and_unknown_versions(), roundtrip_keeps_keys_peers_and_state(), sample(), SESSION_FORMAT, SessionError (+2 more)

### Community 35 - "MsgLayout"
Cohesion: 0.21
Nodes (11): Índice maestro de teleClon, architecture.md (crates, ChatBackend, SQLite, F01-F18), graphify-out (graph.html, GRAPH_REPORT.md, graph.json), Reglas de trabajo (log, ADR, clean-room, credenciales, regenerar grafo), Credenciales api_id/api_hash en .env local gitignored, graphify: instalación y uso, graphifyy en ./.venv local, Registro de skills usadas (+3 more)

### Community 36 - "VaultResult"
Cohesion: 0.11
Nodes (12): DeadVault, DeadVault, check_key(), ChunkedVault<V>, construction_and_validation_do_not_touch_the_system(), KeyringVault, live_roundtrip(), MemoryVault (+4 more)

### Community 37 - "dh.rs"
Cohesion: 0.05
Nodes (36): dh_matches_independent_vector(), DhParams, DhSecret, exp(), FP, GA, GB, generate_mixes_server_random_and_rejects_bad_peer() (+28 more)

### Community 38 - "layout_message"
Cohesion: 0.24
Nodes (10): demo_script(), emoji_only(), input(), layout_message(), LayoutInput, maquetacion_meta_en_linea_o_debajo(), meta_width(), msg() (+2 more)

### Community 39 - "tests_m2.rs"
Cohesion: 0.10
Nodes (23): Auth, Connector, DemoStep, Do, Exit, ScrollUp, Shot, Flags (+15 more)

### Community 40 - "with"
Cohesion: 0.20
Nodes (17): steps, steps, steps, steps, id, if, name, run (+9 more)

### Community 41 - "StickerId"
Cohesion: 0.22
Nodes (4): StickerId, DEFAULT_RECENTS, Favorites, Recents

### Community 42 - "tc-ui/src/stickers.rs"
Cohesion: 0.08
Nodes (22): HANDLES, ANIMATED_SET_ID, Art, art_index(), ART_PX, CELL, CELL_ART, CELL_GAP (+14 more)

### Community 43 - "tc-telegram/src/auth.rs"
Cohesion: 0.20
Nodes (3): GrammersBackend, Inner, map_error()

### Community 44 - "tc-telegram/src/lib.rs"
Cohesion: 0.10
Nodes (10): TelegramConfig, Feed, None, Pool, Source, GrammersBackend, has_login_token(), persister() (+2 more)

### Community 45 - "Entropy"
Cohesion: 0.10
Nodes (33): Role, Initiator, Responder, Entropy, &mut E, random_i64(), random_up_to(), DEFAULT_PEER_LAYER (+25 more)

### Community 46 - "e2e.rs"
Cohesion: 0.17
Nodes (34): chat_n(), concurrent_rekey_converges_on_one_instance(), decreasing_in_seq_aborts_chat(), dh(), discard_local_and_remote(), duplicates_are_dropped(), fingerprint_mismatch_discards_chat(), forge() (+26 more)

### Community 47 - "build_items"
Cohesion: 0.31
Nodes (9): replica/features.csv, Rasgos de marca (avatares cuadrado redondeado, burbujas sin cola, fondo de isolíneas), Agente recon (sonnet/medium), Recon map: Telegram Desktop (Linux), Componentes de UI (Burbuja de mensaje, Fila de chat, Compositor...), Core loop (chats, enviar/recibir, sincronizado), Matriz de funciones (features.csv, must/should/could/skip), Flujos F01-F16 (+1 more)

### Community 48 - "peers.rs"
Cohesion: 0.11
Nodes (11): ProfileApi, GroupId, UserId, Group, ProfilePhoto, User, chat_photo(), PeerStore (+3 more)

### Community 49 - "ADR-0003: Arquitectura del cliente (workspace, backend tras trait, caché SQLite, E2E sans-IO)"
Cohesion: 0.50
Nodes (9): tc-app, tc-core, tc-mock, tc-secret, tc-stickers, tc-store, tc-telegram, tc-theme (+1 more)

### Community 50 - "TextStyle"
Cohesion: 0.15
Nodes (14): BODY, CAPTION, CODE, DISPLAY, EMOJI_ONLY, HEADING, META, TextStyle (+6 more)

### Community 51 - "config.rs"
Cohesion: 0.16
Nodes (11): ApiHash, ConfigError, Invalid, Missing, debug_hides_the_hash(), ENV_API_HASH, ENV_API_ID, FAKE_HASH (+3 more)

### Community 52 - "crypto.rs"
Cohesion: 0.11
Nodes (26): decrypt(), derive(), encrypt(), encrypt_with_padding(), HEADER_LEN, ige_decrypt(), ige_encrypt(), IgeDec (+18 more)

### Community 53 - "sidebar.rs"
Cohesion: 0.09
Nodes (16): LIST_SCROLL_ID, SEARCH_ID, TTL_OPTIONS, App, experimental_note(), key_panel(), new_secret_panel(), overlay() (+8 more)

### Community 54 - "Cache"
Cohesion: 0.14
Nodes (14): dialog_stub(), DialogCtx, draft_from_tl(), is_muted(), Applied, apply_batch(), apply_update(), Cache (+6 more)

### Community 55 - "dialogs.rs"
Cohesion: 0.09
Nodes (14): DIALOG_COLS, dialog_from_row(), draft_from_row(), drafts_set_clear_list(), list_orders_pinned_then_activity_and_filters_archived(), mark_read_and_missing_dialog_errors(), notify_scopes(), Store (+6 more)

### Community 56 - "tc-stickers/src/lib.rs"
Cohesion: 0.67
Nodes (3): Disposición (raíl, lista, conversación, panel info), Lista de chats (componente), Barra lateral (raíl de carpetas)

### Community 59 - "SecretError"
Cohesion: 0.08
Nodes (23): Result, SecretError, Entropy, FingerprintMismatch, InvalidDhConfig, InvalidPublicValue, InvalidState, Malformed (+15 more)

### Community 60 - "Frame"
Cohesion: 0.17
Nodes (3): CacheKey, DEFAULT_CACHE_BYTES, FrameCache

### Community 61 - "tc-store/src/lib.rs"
Cohesion: 0.09
Nodes (23): account_db_path(), APP_DIR, DATA_DIR_NAME, default_data_dir(), LEGACY_APP_DIR, MIGRATIONS, resolve_data_dir(), SCHEMA_VERSION (+15 more)

### Community 64 - "net.rs"
Cohesion: 0.12
Nodes (16): ConnTracker, errors_map_to_domain(), flood_wait_policy_sleeps_only_short_waits(), is_network(), is_session_gone(), Net, only_specific_401s_end_the_session(), PoolTransport (+8 more)

### Community 65 - "pack.rs"
Cohesion: 0.10
Nodes (26): Def, DEFS, dots(), index_of(), Layer, len(), NATIVE_SIZE, P (+18 more)

### Community 66 - "convert.rs"
Cohesion: 0.08
Nodes (22): document_file_ref(), draft_date(), entities_from_tl(), entities_to_tl(), entity_parts(), forward_name(), media_from_document(), media_from_tl() (+14 more)

### Community 67 - "conversation.rs"
Cohesion: 0.11
Nodes (38): now(), Body, Bubble, Emoji, Sticker, MsgLayout, Media, bubble_radius() (+30 more)

### Community 68 - "tests/mod.rs"
Cohesion: 0.11
Nodes (43): key_visualization_matches_independent_vector(), authorization(), channel(), chat(), dialog(), hex(), message(), msg_enum() (+35 more)

### Community 69 - "anim.rs"
Cohesion: 0.11
Nodes (18): Easing, .ESTANDAR, .RAPIDO, .SALIDA, .SUAVE, HIGHLIGHT_FADE_MS, REDUCED_OPACITY_MAX_MS, SKELETON_PULSE_PERIOD_MS (+10 more)

### Community 70 - "media_cache.rs"
Cohesion: 0.10
Nodes (11): account_cache_lives_next_to_the_database_and_survives_reopen(), create_private_dir(), DEFAULT_MEDIA_CACHE_LIMIT, io(), MEDIA_DIR, MediaCache, MediaCacheLayer, now_ms() (+3 more)

### Community 72 - "secret.rs"
Cohesion: 0.11
Nodes (18): chat(), CHAT_COLS, chat_from_row(), chat_roundtrip_and_fk(), messages_idempotent_blob_roundtrip_and_page(), msg(), MSG_COLS, msg_from_row() (+10 more)

### Community 73 - "messages.rs"
Cohesion: 0.14
Nodes (14): COLS, fill(), from_row(), fts_query(), fts_search_accents_prefix_scope_and_delete(), ids(), pagination_by_anchor(), pending_becomes_confirmed_by_random_id() (+6 more)

### Community 74 - "Folder"
Cohesion: 0.19
Nodes (7): Folder, All, Channels, Groups, Personal, Unread, sort_dialogs()

### Community 75 - "tl.rs"
Cohesion: 0.09
Nodes (21): ID_ABORT_KEY, ID_ACCEPT_KEY, ID_COMMIT_KEY, ID_DELETE, ID_FLUSH, ID_LAYER, ID_MEDIA_EMPTY, ID_MESSAGE (+13 more)

### Community 76 - "cached.rs"
Cohesion: 0.10
Nodes (11): ChatBackend, DialogsApi, MediaApi, StickersApi, Draft, PhotoSize, Big, Small (+3 more)

### Community 77 - "EntityKind"
Cohesion: 0.18
Nodes (11): EntityKind, Bold, Code, Hashtag, Italic, Mention, Pre, Spoiler (+3 more)

### Community 78 - "StickerError"
Cohesion: 0.13
Nodes (15): StickerError, BadSize, FrameOutOfRange, Gzip, Image, LottieDisabled, LottieLoad, NotLottie (+7 more)

### Community 79 - "types.rs"
Cohesion: 0.15
Nodes (8): scope_parts(), str_enum!, NotifyScope, Channels, Chat, Groups, Users, NotifySettings

### Community 80 - ".download_location"
Cohesion: 0.14
Nodes (8): random_i64(), crate::Inner, DownloadError, Backend, FileReference, GrammersBackend, media_location(), parts_for()

### Community 81 - "BackendResult"
Cohesion: 0.14
Nodes (6): BackendResult, Dialog, DialogsPage, CachedBackend<B>, is_transient(), Shared<B>

### Community 82 - "Store"
Cohesion: 0.13
Nodes (13): favorites(), RECENT_CAP, recents_order_dedupe_cap_and_cascade(), seed(), SET_COLS, set_from_row(), sets_and_stickers(), STICKER_COLS (+5 more)

### Community 83 - ".u32"
Cohesion: 0.45
Nodes (7): read_action(), read_entity(), read_message(), Reader, write_action(), write_entity(), write_message()

### Community 84 - "tl_visual_ttl.rs"
Cohesion: 0.07
Nodes (12): DecryptedMessage, Message, Service, Media, Empty, PlainMessage, SelfDestructTimer, KeyVisualization (+4 more)

### Community 85 - "EntityKind"
Cohesion: 0.11
Nodes (17): EntityKind, Blockquote, Bold, BotCommand, Code, CustomEmoji, Email, Hashtag (+9 more)

### Community 86 - "SessionSnapshot"
Cohesion: 0.09
Nodes (18): MAGIC, Rd, Rd<'a>, read_ready(), RK_ACCEPTED, RK_COMMITTED, RK_IDLE, RK_REQUESTED (+10 more)

### Community 87 - "view/login.rs"
Cohesion: 0.29
Nodes (22): card_screen(), CARD_W, code(), connecting(), divider(), field(), field_error(), ghost() (+14 more)

### Community 88 - "src/login.rs"
Cohesion: 0.05
Nodes (50): Phase, Auth, Connecting, Main, App, clean_code(), CODE_ID, codigo_solo_cifras_y_recortado() (+42 more)

### Community 89 - "Action"
Cohesion: 0.14
Nodes (14): Action, AbortKey, AcceptKey, CommitKey, DeleteMessages, FlushHistory, Noop, NotifyLayer (+6 more)

### Community 90 - "tc-mock/src/lib.rs"
Cohesion: 0.07
Nodes (21): animated_sticker_set(), BIG_CHAT, BIG_CHAT_LEN, build(), CHANNEL_POSTS, gen_channel(), gen_history(), GROUPS (+13 more)

### Community 91 - "main.rs"
Cohesion: 0.11
Nodes (11): env_flag(), env_u64(), load_env_file(), main(), session_path(), telegram_config(), telegram_connector(), TelegramPaths (+3 more)

### Community 92 - "tc-ui/src/tests.rs"
Cohesion: 0.10
Nodes (24): Backend, Overlay, Delete, Key, NewSecret, Photo, Ttl, PAGE (+16 more)

### Community 95 - "cached_tests.rs"
Cohesion: 0.13
Nodes (23): all(), cache_persists_across_restarts_on_disk(), cached(), cold_cache_offline_is_an_error_and_pagination_works(), dialogs_roundtrip_exactly_and_survive_offline(), downloads_are_cached_on_disk_but_never_for_secret_chats(), edits_deletes_reactions_read_and_failures_write_through(), events_are_persisted_and_searchable_locally() (+15 more)

### Community 96 - "EnvFile"
Cohesion: 0.17
Nodes (6): asignaciones_comillas_y_comentarios(), el_entorno_manda_sobre_el_archivo_salvo_si_esta_vacio(), EnvFile, lookup(), unquote(), account_id()

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
Nodes (16): backend(), decode(), Rd, Rd<'a>, VaultSessionStore, map_err(), VaultError, Backend (+8 more)

### Community 106 - "tests/secret.rs"
Cohesion: 0.10
Nodes (22): UpdateBatch, ScriptSource, ANA, BETO, ChanSource, CHAT, fingerprint(), handler() (+14 more)

### Community 107 - "tc-stickers/src/tests.rs"
Cohesion: 0.25
Nodes (8): cache_evicts_least_recently_used_by_bytes(), cache_get_or_insert_computes_once(), key(), LOTTIE, render_without_lottie_reports_disabled(), tgs_bomb_is_rejected(), tgs_roundtrip_and_validation(), compress()

### Community 110 - "AuthState"
Cohesion: 0.06
Nodes (25): AuthApi, AuthState, Checking, LoggedIn, LoggedOut, SignUpRequired, WaitingCode, WaitingPassword (+17 more)

### Community 112 - "folders.rs"
Cohesion: 0.19
Nodes (8): COLS, folder(), folder_peers_replace_and_cascade(), folders_crud_order_and_reorder(), from_row(), Store, FolderPeerRow, FolderRow

### Community 113 - "files.rs"
Cohesion: 0.14
Nodes (9): file_ref_from(), file_ref_row(), cache_index_orders_by_last_use(), CacheEntry, media_refs_follow_their_media(), MediaRefRow, peer_photos_roundtrip_and_never_for_secrets(), photo_kind() (+1 more)

### Community 115 - "vault.rs"
Cohesion: 0.15
Nodes (18): CHUNK_HEADER_LEN, CHUNK_MAGIC, chunked_vault_reports_missing_parts_and_keeps_magic_values_intact(), chunked_vault_splits_large_values_and_cleans_up(), ChunkedVault, clear_session(), delete_secret_chat_key(), key_ref_is_a_name_not_a_secret() (+10 more)

### Community 116 - "Event"
Cohesion: 0.11
Nodes (18): Event, Deleted, Discarded, Duplicate, HistoryFlushed, KeyChanged, PeerLayer, PeerLayerNewer (+10 more)

### Community 117 - "Message"
Cohesion: 0.08
Nodes (29): MessagesApi, MsgId, PeerRef, Group, Secret, User, RandomId, FileRef (+21 more)

### Community 118 - "Nav"
Cohesion: 0.15
Nodes (10): la_rejilla_se_recorre_con_flechas_sin_salirse(), Nav, Activate, Down, Left, NextTab, PrevTab, Right (+2 more)

### Community 119 - "jobs"
Cohesion: 0.18
Nodes (10): env, CARGO_INCREMENTAL, CARGO_TERM_COLOR, RUST_TOOLCHAIN, jobs, linux, name, needs (+2 more)

### Community 120 - "reactions.rs"
Cohesion: 0.29
Nodes (7): message_exists(), my_reaction_moves_between_emojis(), r(), set_get_replace_and_unknown_message(), setup(), Store, ReactionRow

### Community 123 - "Marca propia — Parlo"
Cohesion: 0.07
Nodes (24): ADR-0006: Empaquetado multiplataforma (Windows, macOS, Linux), Consecuencias, Decisiones, 1. Criterios de nombre, 2. Candidatos (6-8), 3. Chequeos que debe hacer el usuario, 4. Voz y tono (español), 5. Brief de logo (+16 more)

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

### Community 133 - "tests/media.rs"
Cohesion: 0.17
Nodes (12): dialog_from_tl(), at_most_four_downloads_run_at_once(), channel_with_photo(), file_part(), photo(), photo_media(), photo_sizes_pick_a_bubble_thumb_and_the_largest_full(), profile_photo_in_another_dc_copies_auth_and_joins_parts() (+4 more)

### Community 139 - "stores.rs"
Cohesion: 0.12
Nodes (27): aad(), AAD_PREFIX, dh(), disk_bytes(), encode(), existing_session_json_is_migrated_once_and_deleted(), FIXED_LEN, media_cache() (+19 more)

### Community 140 - "GroupPos"
Cohesion: 0.33
Nodes (5): Item, Date, Edge, Msg, UnreadMark

### Community 141 - "windows"
Cohesion: 0.29
Nodes (7): WINDOWS_CERT_PASSWORD, WINDOWS_CERT_PFX_BASE64, windows, env, name, needs, runs-on

### Community 143 - "tests/snapshot.rs"
Cohesion: 0.12
Nodes (16): OsEntropy, StateKind, Discarded, Incoming, Ready, Requested, SeededEntropy, corrupt_or_foreign_snapshots_are_rejected() (+8 more)

### Community 144 - "ConnectionState"
Cohesion: 0.33
Nodes (5): ConnectionState, Connecting, Offline, Online, Updating

### Community 145 - "meta"
Cohesion: 0.40
Nodes (5): meta, name, outputs, runs-on, version

### Community 147 - "images.rs"
Cohesion: 0.14
Nodes (10): document(), PHOTO_BASE, png(), png_is_valid_and_deterministic(), profile_photo(), profile_side(), tone(), variant_dims() (+2 more)

### Community 149 - "tests/mod.rs"
Cohesion: 0.13
Nodes (8): cfg(), Fake, Handler, logged_session(), logged_session_as(), ME_ID, ok(), Reply

### Community 150 - "Store"
Cohesion: 0.20
Nodes (9): load_group(), load_photo(), load_user(), peer_from(), persist_group(), persist_photo(), persist_user(), Store (+1 more)

### Community 151 - "tc-store/src/convert.rs"
Cohesion: 0.13
Nodes (8): decode_entities(), decode_sticker(), encode_entities(), encode_sticker(), ENTITY_NAMES, media_kind_parse(), media_kind_str(), pending_local_id()

### Community 152 - "Frame"
Cohesion: 0.17
Nodes (8): Frame, gradient(), pack_stickers_are_distinct_and_survive_webp(), webp_garbage_is_error(), webp_roundtrip_is_lossless(), decode(), encode_lossless(), MAX_SIDE

### Community 153 - "apply_history"
Cohesion: 0.19
Nodes (12): apply_history(), cached_history(), history_is_servable(), load_message(), map_anchor(), msg_id32(), Anchor, After (+4 more)

### Community 154 - "media_key"
Cohesion: 0.27
Nodes (6): check_key(), fetch_cached_goes_to_the_network_once_and_never_for_secrets(), keys_depend_on_id_and_size_and_skip_secrets(), media_key(), photo_ref(), profile_photo_key()

### Community 155 - "GroupRow"
Cohesion: 0.27
Nodes (3): group_roundtrip_update_and_lookup(), Store, GroupRow

### Community 161 - "icon"
Cohesion: 0.15
Nodes (15): handle(), icon(), icon_fill(), icons!, view(), badge_at(), chat_row(), folder_icon() (+7 more)

### Community 211 - "on"
Cohesion: 0.50
Nodes (4): on, push, workflow_dispatch, tags

### Community 214 - ".start"
Cohesion: 0.25
Nodes (7): Anim, Animator, Animator<K>, Curve, interrumpible_hereda_posicion(), movimiento_reducido_salta_posicion_y_limita_opacidad(), muelle_converge_y_termina()

### Community 215 - "env"
Cohesion: 0.67
Nodes (3): GH_TOKEN, VERSION, env

### Community 217 - "select.rs"
Cohesion: 0.31
Nodes (10): Choice, Mock, Telegram, choose(), FAKE_HASH, missing_credentials(), por_defecto_el_mock_sin_mirar_credenciales(), sin_credenciales_vuelve_al_mock_con_aviso_que_no_ensena_valores() (+2 more)

### Community 220 - "MotionPreference"
Cohesion: 0.22
Nodes (7): MotionPreference, Full, Reduced, Property, Opacity, Position, Scale

### Community 225 - "TopLevel"
Cohesion: 0.31
Nodes (4): Reader<'a>, TopLevel, Layer, LegacyService

## Knowledge Gaps
- **366 isolated node(s):** `Connecting`, `Online`, `Updating`, `Offline`, `Connection` (+361 more)
  These have ≤1 connection - possible missing edges or undocumented components. (Counts symbols only; 1251 node(s) total have ≤1 connection when file, concept and rationale nodes are included.)
- **88 thin communities (<3 nodes) omitted from report** — run `graphify query` to explore isolated nodes.

## Suggested Questions
_Questions this graph is uniquely positioned to answer:_

- **Why does `MockBackend` connect `MockBackend` to `State`, `.new`, `Sistema de diseño de teleClon — dirección «Brasa»`, `tests_m2.rs`, `.accept_secret_chat`, `tc-ui/src/tests.rs`, `images.rs`, `tc-mock/src/lib.rs`, `State`, `cached_tests.rs`?**
  _High betweenness centrality (0.073) - this node is a cross-community bridge._
- **Are the 2 inferred relationships involving `Msg` (e.g. with `Event` and `Decoded`) actually correct?**
  _`Msg` has 2 INFERRED edges - model-reasoned connections that need verification._
- **What connects `Connecting`, `Online`, `Updating` to the rest of the system?**
  _366 weakly-connected nodes found - possible documentation gaps or missing edges._
- **Should `MockBackend` be split into smaller, more focused modules?**
  _Cohesion score 0.09851551956815115 - nodes in this community are weakly interconnected._
- **Why does `Result` connect `tests/snapshot.rs` to `groups.rs`, `uploads.rs`, `dh.rs`, `media_cache.rs`, `secret.rs`, `messages.rs`, `tl.rs`, `cached.rs`, `Entropy`, `folders.rs`, `files.rs`, `Store`, `crypto.rs`, `dialogs.rs`, `SessionSnapshot`, `tc-store/src/convert.rs`, `reactions.rs`, `tc-store/src/lib.rs`?**
  _High betweenness centrality (0.061) - this node is a cross-community bridge._
- **Are the 42 inferred relationships involving `c()` (e.g. with `handler()` and `.boot()`) actually correct?**
  _`c()` has 42 INFERRED edges - model-reasoned connections that need verification._
- **Should `tokens.rs` be split into smaller, more focused modules?**
  _Cohesion score 0.03446327683615819 - nodes in this community are weakly interconnected._