# Graph Report - teleClon  (2026-11-07)

## Corpus Check
- 148 files · ~145,201 words
- Verdict: corpus is large enough that graph structure adds value.
- Unclassified: 11 file(s) not represented in the graph (top: (none) 2, .example 1, .desktop 1)

## Summary
- 3614 nodes · 8986 edges · 230 communities (135 shown, 95 thin omitted)
- Extraction: 92% EXTRACTED · 8% INFERRED · 0% AMBIGUOUS · INFERRED: 687 edges (avg confidence: 0.77)
- Token cost: 0 input · 0 output

## Graph Freshness
- Built from commit: `50aea0be`
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
- mapping.rs
- ci.yml
- model.rs
- tc-secret/src/session.rs
- App
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
- tc-store/src/convert.rs
- TcSession
- MsgLayout
- VaultResult
- dh.rs
- Flags
- tests_m2.rs
- with
- StickerId
- tc-ui/src/stickers.rs
- tc-telegram/src/auth.rs
- tc-telegram/src/lib.rs
- Entropy
- e2e.rs
- build_items
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
- tc-core/src/lib.rs
- SecretChat
- secret.rs
- messages.rs
- Folder
- tl.rs
- cached.rs
- EntityKind
- StickerError
- types.rs
- tc_core
- BackendResult
- Store
- .u32
- tl_visual_ttl.rs
- EntityKind
- SessionSnapshot
- view/login.rs
- LoginMsg
- Action
- tc-mock/src/lib.rs
- main.rs
- tc-ui/src/tests.rs
- Login
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
- src/login.rs
- Overlay
- VaultSessionStore
- Event
- Message
- Nav
- jobs
- reactions.rs
- Step
- State
- Marca propia — Parlo
- CodeKind
- tc-store
- Informe de barrido de marca
- env
- release
- PeerRef
- stores.rs
- GroupPos
- windows
- tests/snapshot.rs
- ConnectionState
- meta
- 0005-nombre-parlo.md
- Lcg
- Phase
- on
- build.rs
- BackendError
- .start
- env
- build-macos.sh
- select.rs
- build-linux.sh
- MotionPreference
- MsgLayout
- ChunkedVault<V>
- ThemeKind
- Anchor
- TopLevel
- Easing
- GroupKind

## God Nodes (most connected - your core abstractions)
1. `Msg` - 186 edges
2. `App` - 136 edges
3. `c()` - 81 edges
4. `BackendResult` - 77 edges
5. `PeerRef` - 65 edges
6. `MockBackend` - 62 edges
7. `Brasa` - 56 edges
8. `Message` - 54 edges
9. `TcSession` - 45 edges
10. `SecretChat` - 45 edges

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
- 1-file cycle: `crates/tc-store/src/vault.rs -> crates/tc-store/src/vault.rs`
- 1-file cycle: `crates/tc-theme/src/tokens.rs -> crates/tc-theme/src/tokens.rs`
- 2-file cycle: `crates/tc-ui/src/app.rs -> crates/tc-ui/src/login.rs -> crates/tc-ui/src/app.rs`
- 2-file cycle: `crates/tc-telegram/src/auth.rs -> crates/tc-telegram/src/lib.rs -> crates/tc-telegram/src/auth.rs`
- 2-file cycle: `crates/tc-telegram/src/lib.rs -> crates/tc-telegram/src/secret.rs -> crates/tc-telegram/src/lib.rs`
- 2-file cycle: `crates/tc-secret/src/session.rs -> crates/tc-secret/src/session/snapshot.rs -> crates/tc-secret/src/session.rs`
- 2-file cycle: `crates/tc-secret/src/dh.rs -> crates/tc-secret/src/testing.rs -> crates/tc-secret/src/dh.rs`

## Hyperedges (group relationships)
- **Implementaciones y decoradores de ChatBackend** — docs_architecture_mockbackend, docs_architecture_cachedbackend, docs_architecture_tc_telegram, docs_architecture_chatbackend [EXTRACTED 0.95]
- **Fundamentos visuales de Brasa** — docs_design_system_paleta_oklch, docs_design_system_tipografia, docs_design_system_espaciado_radios, docs_design_system_elevacion, docs_design_system_movimiento [EXTRACTED 0.95]
- **Decisiones de arquitectura ADR-0001 a ADR-0004** — docs_adr_0001_stack, docs_adr_0002_direccion_visual, docs_adr_0003_arquitectura, docs_adr_0004_stickers_y_chats_secretos [EXTRACTED 1.00]
- **Pila de chats secretos (cripto, claves y UI)** — docs_architecture_tc_secret, docs_architecture_keyring_boveda, docs_design_system_chat_secreto_ui [INFERRED 0.75]
- **Implementaciones de ChatBackend (Mock, Cached, Telegram)** — docs_adr_0003_arquitectura_chatbackend, docs_adr_0003_arquitectura_mockbackend, docs_adr_0003_arquitectura_cachedbackend, docs_adr_0001_stack_grammers [INFERRED 0.85]
- **Pila de chats secretos E2E** — docs_adr_0004_stickers_y_chats_secretos_secret_chats, docs_adr_0003_arquitectura_tc_secret, docs_adr_0001_stack_keyring, docs_adr_0004_stickers_y_chats_secretos_crypto_risk [INFERRED 0.85]

## Communities (230 total, 95 thin omitted)

### Community 0 - "style.rs"
Cohesion: 0.13
Nodes (44): app_style(), avatar_ink(), badge(), bare_input(), border_fill(), Brasa, bubble(), bubble_radius() (+36 more)

### Community 1 - "MockBackend"
Cohesion: 0.07
Nodes (7): ANIMATED_SET_ID, Inner, MOCK_2FA_PASSWORD, MOCK_FLOOD_SECS, MOCK_LOGIN_CODE, MockBackend, MockConfig

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
Cohesion: 0.13
Nodes (15): BackendEvent, Connection, DialogUpserted, MessageEdited, MessageFailed, MessageNew, MessagesDeleted, MessageSent (+7 more)

### Community 7 - "tc-telegram/src/secret.rs"
Cohesion: 0.06
Nodes (23): utf16_starts(), utf16_to_chars(), now_secs(), Chat, crypto(), EARLY_CAP, entities_from_secret(), entities_round_trip_through_utf16() (+15 more)

### Community 8 - "Sistema de diseño de teleClon — dirección «Brasa»"
Cohesion: 0.16
Nodes (25): about_thirty_chats_sorted_with_pinned_first(), data_is_deterministic(), delete_updates_last_message_and_emits(), edit_only_own_messages(), forward_creates_messages_with_origin(), history_after_and_around(), long_history_paging_covers_everything_without_gaps(), mark_read_clears_unread() (+17 more)

### Community 9 - "conv.rs"
Cohesion: 0.06
Nodes (40): DATE_H, EDGE_H, emoji_only(), FWD_H, GAP_GROUP, GAP_IN_GROUP, GROUP_AVATAR, GROUP_AVATAR_COL (+32 more)

### Community 10 - "format.rs"
Cohesion: 0.16
Nodes (26): Timestamp, UserStatus, LastSeen, Offline, Online, Recently, date_separator(), day_key() (+18 more)

### Community 11 - "model.rs"
Cohesion: 0.16
Nodes (12): INK, WHITE, AA_NORMAL, AA_UI, Alpha, Alphas, Rgb, DARK (+4 more)

### Community 12 - "mapping.rs"
Cohesion: 0.20
Nodes (16): dialog(), message(), msg_enum(), notify(), peer_channel(), peer_chat(), peer_user(), RFC3526_2048 (+8 more)

### Community 13 - "ci.yml"
Cohesion: 0.07
Nodes (28): name, runs-on, steps, strategy, concurrency, cancel-in-progress, group, env (+20 more)

### Community 14 - "model.rs"
Cohesion: 0.10
Nodes (22): StickerSetId, Media, MediaKind, File, Gif, Photo, Video, Voice (+14 more)

### Community 15 - "tc-secret/src/session.rs"
Cohesion: 0.05
Nodes (34): DEFAULT_PEER_LAYER, MAX_QUEUED, MIN_SEND_LAYER, OUR_LAYER, Outbound, AcceptEncryption, DiscardEncryption, RequestEncryption (+26 more)

### Community 16 - "App"
Cohesion: 0.07
Nodes (15): AnimKey, BubbleOpacity, BubbleY, ChatSwitch, Overlay, SendButton, App, Reply (+7 more)

### Community 17 - ".update"
Cohesion: 0.11
Nodes (18): BUNDLED, DISPLAY, DISPLAY_FAMILY, EMOJI_FAMILY, family(), MONO, MONO_FALLBACK, MONO_FAMILY (+10 more)

### Community 18 - "measure.rs"
Cohesion: 0.17
Nodes (7): base(), Font_, IcedMeasurer, Paragraph, SHAPING, to_spans(), WRAP

### Community 19 - "api.rs"
Cohesion: 0.16
Nodes (15): ADR-0002: Dirección visual Brasa, Dirección Brasa (grafito cálido + acento persimón), Controlled para chrome, innovative para burbujas y fondo, Movimiento con muelles 119-221 ms y movimiento reducido, OKLCH como fuente de verdad y contraste WCAG AA (144 pares), docs/design/system.md y tokens.json, Tipografía libre (Atkinson Hyperlegible Next, Bricolage Grotesque, JetBrains Mono) e iconos Phosphor, Agente diseño (opus/high) (+7 more)

### Community 20 - "anim.rs"
Cohesion: 0.06
Nodes (20): classify(), main(), report(), scan(), build(), enc(), in_gamut(), lum() (+12 more)

### Community 21 - "SecretChatId"
Cohesion: 0.15
Nodes (8): record(), SecretChatsApi, BackendResult, SecretChatId, UserId, display_name_joins_parts(), KeyVisual, SecretChat

### Community 22 - "super"
Cohesion: 0.11
Nodes (13): key_is_created_once_in_the_vault_and_reused(), roundtrip_and_every_kind_of_tampering_is_rejected(), SEAL_KEY_LEN, Sealed, SealError, BadKey, Entropy, Rejected (+5 more)

### Community 23 - "app.rs"
Cohesion: 0.07
Nodes (13): auth_events(), AuthHandle, BackendHandle, Compose, Edit, Normal, COMPOSER_ID, CONV_SCROLL_ID (+5 more)

### Community 24 - "backend.rs"
Cohesion: 0.11
Nodes (27): GroupId, Group, MAX_TEXT, channel_group(), channel_of(), CHANNEL_OFFSET, chat_group(), decode_group() (+19 more)

### Community 25 - "tc-telegram/src/auth.rs"
Cohesion: 0.10
Nodes (11): AuthInner, code_kind(), code_types_map(), migrate_dc(), next_kind(), phone_validation(), qr_url(), qr_url_is_unpadded_base64url() (+3 more)

### Community 26 - "widgets.rs"
Cohesion: 0.12
Nodes (6): avatar_radius(), avatar(), badge(), contour_bg(), ContourBg, icon_button()

### Community 27 - "GrammersBackend"
Cohesion: 0.06
Nodes (6): GrammersBackend, history_params(), Inner, messages_of(), reply_to(), flatten_updates()

### Community 28 - "icon"
Cohesion: 0.19
Nodes (12): ADR-0001: Stack técnico, grammers (MTProto en Rust), iced (UI en Rust, wgpu + tiny-skia), keyring (claves y sesión), rusqlite bundled con migraciones por PRAGMA user_version, tc-secret: chats secretos sans-IO con RustCrypto, tc-stickers: WebP, TGS con rlottie, WebM aplazado, ADR-0004: Stickers y chats secretos en el alcance (+4 more)

### Community 29 - "Inner"
Cohesion: 0.16
Nodes (5): Inner, pump(), run_source(), startup(), wait_logged_in()

### Community 30 - "tc-ui/src/tests.rs"
Cohesion: 0.24
Nodes (12): ADR-0003: Arquitectura del cliente, Orden de construcción M1.1-M1.4, M2.1, M2.7, CachedBackend (caché como decorador, tc-store), Trait ChatBackend (sub-traits + events()), Modelo de concurrencia (tokio, hilo SQLite, canal acotado, Resync), MockBackend determinista, SQLite por cuenta (17 tablas + FTS5, WAL, chat_key), Scroll virtual propio (Fenwick, ventana ~600 mensajes) (+4 more)

### Community 31 - "EntityKind"
Cohesion: 0.24
Nodes (13): Registro de agentes, Agente arquitecto (opus/high), Agente core+mock (sonnet/medium), Agente QA (pendiente), Agente theme (sonnet/medium), Agente UI (opus/high), Bitácora (append-only), Bug iced 0.14: scrollable.direction antes de anchor_bottom (+5 more)

### Community 32 - "groups.rs"
Cohesion: 0.11
Nodes (7): COLS, from_row(), group_roundtrip_update_and_lookup(), Store, settings_crud_and_prefix(), Store, GroupRow

### Community 33 - "tc-store/src/convert.rs"
Cohesion: 0.07
Nodes (41): apply_dialogs(), apply_history(), cached_dialogs(), cached_history(), decode_entities(), decode_sticker(), dialogs_complete_key(), encode_entities() (+33 more)

### Community 34 - "TcSession"
Cohesion: 0.08
Nodes (10): changes_mark_dirty_and_reset_forgets_everything(), debug_never_prints_the_auth_key(), MemorySessionStore, rejects_garbage_and_unknown_versions(), roundtrip_keeps_keys_peers_and_state(), sample(), SESSION_FORMAT, SessionError (+2 more)

### Community 35 - "MsgLayout"
Cohesion: 0.21
Nodes (11): Índice maestro de teleClon, architecture.md (crates, ChatBackend, SQLite, F01-F18), graphify-out (graph.html, GRAPH_REPORT.md, graph.json), Reglas de trabajo (log, ADR, clean-room, credenciales, regenerar grafo), Credenciales api_id/api_hash en .env local gitignored, graphify: instalación y uso, graphifyy en ./.venv local, Registro de skills usadas (+3 more)

### Community 36 - "VaultResult"
Cohesion: 0.09
Nodes (28): DeadVault, DeadVault, check_key(), CHUNK_HEADER_LEN, CHUNK_MAGIC, chunked_vault_reports_missing_parts_and_keeps_magic_values_intact(), chunked_vault_splits_large_values_and_cleans_up(), ChunkedVault (+20 more)

### Community 37 - "dh.rs"
Cohesion: 0.07
Nodes (26): dh_matches_independent_vector(), DhParams, DhSecret, exp(), FP, GA, GB, generate_mixes_server_random_and_rejects_bad_peer() (+18 more)

### Community 38 - "Flags"
Cohesion: 0.17
Nodes (11): demo_script(), Auth, Connector, DemoStep, Do, Exit, ScrollUp, Shot (+3 more)

### Community 39 - "tests_m2.rs"
Cohesion: 0.18
Nodes (13): boot_logged_out(), boot_main(), crear_chat_secreto_ttl_y_cerrar(), invitacion_entrante_se_acepta(), lm(), login_por_qr_numero_sin_cuenta_y_escape(), login_telefono_espera_codigo_y_2fa(), NOW (+5 more)

### Community 40 - "with"
Cohesion: 0.20
Nodes (17): steps, steps, steps, steps, id, if, name, run (+9 more)

### Community 41 - "StickerId"
Cohesion: 0.24
Nodes (4): StickerId, DEFAULT_RECENTS, Favorites, Recents

### Community 42 - "tc-ui/src/stickers.rs"
Cohesion: 0.08
Nodes (19): HANDLES, ANIMATED_SET_ID, Art, art_index(), ART_PX, CELL, CELL_ART, CELL_GAP (+11 more)

### Community 43 - "tc-telegram/src/auth.rs"
Cohesion: 0.20
Nodes (3): GrammersBackend, Inner, map_error()

### Community 44 - "tc-telegram/src/lib.rs"
Cohesion: 0.11
Nodes (8): TelegramConfig, Feed, None, Pool, Source, GrammersBackend, has_login_token(), persister()

### Community 45 - "Entropy"
Cohesion: 0.13
Nodes (21): Role, Initiator, Responder, Entropy, &mut E, OsEntropy, random_i64(), random_up_to() (+13 more)

### Community 46 - "e2e.rs"
Cohesion: 0.17
Nodes (35): DecryptedMessageLayer, chat_n(), concurrent_rekey_converges_on_one_instance(), decreasing_in_seq_aborts_chat(), dh(), discard_local_and_remote(), duplicates_are_dropped(), fingerprint_mismatch_discards_chat() (+27 more)

### Community 47 - "build_items"
Cohesion: 0.31
Nodes (9): replica/features.csv, Rasgos de marca (avatares cuadrado redondeado, burbujas sin cola, fondo de isolíneas), Agente recon (sonnet/medium), Recon map: Telegram Desktop (Linux), Componentes de UI (Burbuja de mensaje, Fila de chat, Compositor...), Core loop (chats, enviar/recibir, sincronizado), Matriz de funciones (features.csv, must/should/could/skip), Flujos F01-F16 (+1 more)

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
Cohesion: 0.08
Nodes (28): decrypt(), derive(), encrypt(), encrypt_with_padding(), HEADER_LEN, ige_decrypt(), ige_encrypt(), IgeDec (+20 more)

### Community 53 - "sidebar.rs"
Cohesion: 0.08
Nodes (23): is_animated(), App, experimental_note(), key_panel(), new_secret_panel(), overlay(), toasts(), ttl_panel() (+15 more)

### Community 54 - "Cache"
Cohesion: 0.12
Nodes (13): draft_from_tl(), is_muted(), reactions_from_tl(), Applied, apply_batch(), apply_update(), Cache, deleted() (+5 more)

### Community 55 - "dialogs.rs"
Cohesion: 0.09
Nodes (15): DIALOG_COLS, dialog_from_row(), draft_from_row(), drafts_set_clear_list(), list_orders_pinned_then_activity_and_filters_archived(), mark_read_and_missing_dialog_errors(), notify_scopes(), Store (+7 more)

### Community 56 - "tc-stickers/src/lib.rs"
Cohesion: 0.67
Nodes (3): Disposición (raíl, lista, conversación, panel info), Lista de chats (componente), Barra lateral (raíl de carpetas)

### Community 59 - "SecretError"
Cohesion: 0.08
Nodes (23): Result, SecretError, Entropy, FingerprintMismatch, InvalidDhConfig, InvalidPublicValue, InvalidState, Malformed (+15 more)

### Community 60 - "Frame"
Cohesion: 0.14
Nodes (4): CacheKey, DEFAULT_CACHE_BYTES, FrameCache, Frame

### Community 61 - "tc-store/src/lib.rs"
Cohesion: 0.08
Nodes (27): events_stream_ends_when_the_decorator_and_backend_are_gone(), account_db_path(), APP_DIR, DATA_DIR_NAME, default_data_dir(), LEGACY_APP_DIR, MIGRATIONS, resolve_data_dir() (+19 more)

### Community 64 - "net.rs"
Cohesion: 0.12
Nodes (16): ConnTracker, errors_map_to_domain(), flood_wait_policy_sleeps_only_short_waits(), is_network(), is_session_gone(), Net, only_specific_401s_end_the_session(), PoolTransport (+8 more)

### Community 65 - "pack.rs"
Cohesion: 0.10
Nodes (27): Def, DEFS, dots(), index_of(), Layer, len(), NATIVE_SIZE, P (+19 more)

### Community 66 - "convert.rs"
Cohesion: 0.10
Nodes (17): dialog_from_tl(), dialog_stub(), DialogCtx, draft_date(), entities_from_tl(), entities_to_tl(), entity_parts(), forward_name() (+9 more)

### Community 67 - "conversation.rs"
Cohesion: 0.10
Nodes (32): handle(), icon(), icon_fill(), icons!, view(), GroupPos, First, Last (+24 more)

### Community 68 - "tests/mod.rs"
Cohesion: 0.09
Nodes (32): authorization(), state(), updates(), user(), user_enum(), dialogs_paginate_with_server_offsets(), edit_delete_react_and_read(), flood_wait_short_is_slept_long_is_returned() (+24 more)

### Community 69 - "anim.rs"
Cohesion: 0.14
Nodes (15): HIGHLIGHT_FADE_MS, REDUCED_OPACITY_MAX_MS, SKELETON_PULSE_PERIOD_MS, Spring, .ESTANDAR, .RAPIDO, .REBOTE, .SUAVE (+7 more)

### Community 70 - "tc-core/src/lib.rs"
Cohesion: 0.18
Nodes (4): SignInResult, LoggedIn, PasswordNeeded, SignUpNeeded

### Community 71 - "SecretChat"
Cohesion: 0.14
Nodes (5): SecretChat, State, Discarded, Incoming, Requested

### Community 72 - "secret.rs"
Cohesion: 0.11
Nodes (18): chat(), CHAT_COLS, chat_from_row(), chat_roundtrip_and_fk(), messages_idempotent_blob_roundtrip_and_page(), msg(), MSG_COLS, msg_from_row() (+10 more)

### Community 73 - "messages.rs"
Cohesion: 0.14
Nodes (14): COLS, fill(), from_row(), fts_query(), fts_search_accents_prefix_scope_and_delete(), ids(), pagination_by_anchor(), pending_becomes_confirmed_by_random_id() (+6 more)

### Community 74 - "Folder"
Cohesion: 0.29
Nodes (6): Folder, All, Channels, Groups, Personal, Unread

### Community 75 - "tl.rs"
Cohesion: 0.08
Nodes (23): ID_ABORT_KEY, ID_ACCEPT_KEY, ID_COMMIT_KEY, ID_DELETE, ID_FLUSH, ID_LAYER, ID_MEDIA_EMPTY, ID_MESSAGE (+15 more)

### Community 76 - "cached.rs"
Cohesion: 0.13
Nodes (7): ChatBackend, MediaApi, ProfileApi, StickersApi, User, CachedBackend, Shared

### Community 77 - "EntityKind"
Cohesion: 0.18
Nodes (11): EntityKind, Bold, Code, Hashtag, Italic, Mention, Pre, Spoiler (+3 more)

### Community 78 - "StickerError"
Cohesion: 0.13
Nodes (15): StickerError, BadSize, FrameOutOfRange, Gzip, Image, LottieDisabled, LottieLoad, NotLottie (+7 more)

### Community 79 - "types.rs"
Cohesion: 0.15
Nodes (8): scope_parts(), str_enum!, NotifyScope, Channels, Chat, Groups, Users, NotifySettings

### Community 80 - "tc_core"
Cohesion: 0.12
Nodes (8): BIG_FILE, crate::Inner, GrammersBackend, input_media(), MAX_FILE, mime_for(), PART_SIZE, parts_for()

### Community 81 - "BackendResult"
Cohesion: 0.15
Nodes (5): Dialog, DialogsPage, CachedBackend<B>, is_transient(), Shared<B>

### Community 82 - "Store"
Cohesion: 0.13
Nodes (13): favorites(), RECENT_CAP, recents_order_dedupe_cap_and_cascade(), seed(), SET_COLS, set_from_row(), sets_and_stickers(), STICKER_COLS (+5 more)

### Community 83 - ".u32"
Cohesion: 0.32
Nodes (10): Message, Entity, read_action(), read_entity(), read_message(), Reader, Reader<'a>, write_action() (+2 more)

### Community 84 - "tl_visual_ttl.rs"
Cohesion: 0.09
Nodes (11): DecryptedMessage, Service, PlainMessage, SelfDestructTimer, key_visualization_matches_independent_vector(), layer(), tl_known_encoding(), tl_rejects_unknown_and_trailing() (+3 more)

### Community 85 - "EntityKind"
Cohesion: 0.11
Nodes (17): EntityKind, Blockquote, Bold, BotCommand, Code, CustomEmoji, Email, Hashtag (+9 more)

### Community 86 - "SessionSnapshot"
Cohesion: 0.12
Nodes (8): Rd, Rd<'a>, read_ready(), role_byte(), SecretChat, SessionSnapshot, Wr, write_ready()

### Community 87 - "view/login.rs"
Cohesion: 0.26
Nodes (23): card_screen(), CARD_W, code(), connecting(), divider(), field(), field_error(), ghost() (+15 more)

### Community 88 - "LoginMsg"
Cohesion: 0.11
Nodes (20): App, focus_for(), LoginMsg, CodeChanged, CodeSent, PasswordChanged, PasswordChecked, PhoneChanged (+12 more)

### Community 89 - "Action"
Cohesion: 0.14
Nodes (14): Action, AbortKey, AcceptKey, CommitKey, DeleteMessages, FlushHistory, Noop, NotifyLayer (+6 more)

### Community 90 - "tc-mock/src/lib.rs"
Cohesion: 0.14
Nodes (16): animated_sticker_set(), BIG_CHAT, BIG_CHAT_LEN, build(), CHANNEL_POSTS, gen_channel(), gen_history(), GROUPS (+8 more)

### Community 91 - "main.rs"
Cohesion: 0.11
Nodes (10): env_flag(), env_u64(), load_env_file(), main(), session_path(), telegram_config(), telegram_connector(), TelegramPaths (+2 more)

### Community 92 - "tc-ui/src/tests.rs"
Cohesion: 0.18
Nodes (12): Backend, PAGE, boot(), chat_secreto_huella_y_canal_de_solo_lectura(), entrante_en_vivo_reordena_la_lista(), enviar_optimista_confirmar_editar_reaccionar_borrar(), fallo_de_envio_y_reintento(), historial_paginado_carga_los_5000_sin_huecos() (+4 more)

### Community 93 - "Login"
Cohesion: 0.16
Nodes (9): Field, Code, Password, Phone, left(), Login, Qr, un_logged_out_repetido_no_borra_lo_escrito() (+1 more)

### Community 95 - "cached_tests.rs"
Cohesion: 0.14
Nodes (20): all(), cache_persists_across_restarts_on_disk(), cached(), cold_cache_offline_is_an_error_and_pagination_works(), dialogs_roundtrip_exactly_and_survive_offline(), edits_deletes_reactions_read_and_failures_write_through(), events_are_persisted_and_searchable_locally(), history_is_lossless_paginates_and_serves_offline() (+12 more)

### Community 96 - "EnvFile"
Cohesion: 0.24
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
Cohesion: 0.19
Nodes (10): decode(), Rd, Rd<'a>, file_store_is_private_and_atomic(), FileSessionStore, SessionStore, StoreError, Backend (+2 more)

### Community 106 - "tests/secret.rs"
Cohesion: 0.10
Nodes (24): UpdateBatch, UpdateSource, UpdatesReceiver, ANA, BETO, ChanSource, CHAT, fingerprint() (+16 more)

### Community 107 - "tc-stickers/src/tests.rs"
Cohesion: 0.14
Nodes (14): cache_evicts_least_recently_used_by_bytes(), cache_get_or_insert_computes_once(), gradient(), key(), LOTTIE, render_without_lottie_reports_disabled(), tgs_bomb_is_rejected(), tgs_roundtrip_and_validation() (+6 more)

### Community 110 - "AuthState"
Cohesion: 0.14
Nodes (12): AuthApi, AuthState, Checking, LoggedIn, LoggedOut, SignUpRequired, WaitingCode, WaitingPassword (+4 more)

### Community 112 - "folders.rs"
Cohesion: 0.19
Nodes (8): COLS, folder(), folder_peers_replace_and_cascade(), folders_crud_order_and_reorder(), from_row(), Store, FolderPeerRow, FolderRow

### Community 113 - "src/login.rs"
Cohesion: 0.15
Nodes (11): clean_code(), CODE_ID, codigo_solo_cifras_y_recortado(), countdown(), cuentas_atras(), errores_con_texto_propio(), login_error(), normalize_phone() (+3 more)

### Community 114 - "Overlay"
Cohesion: 0.33
Nodes (5): Overlay, Delete, Key, NewSecret, Ttl

### Community 115 - "VaultSessionStore"
Cohesion: 0.33
Nodes (6): backend(), VaultSessionStore, map_err(), VaultError, Backend, Unavailable

### Community 116 - "Event"
Cohesion: 0.11
Nodes (18): Event, Deleted, Discarded, Duplicate, HistoryFlushed, KeyChanged, PeerLayer, PeerLayerNewer (+10 more)

### Community 117 - "Message"
Cohesion: 0.14
Nodes (16): MsgId, RandomId, Message, OutgoingText, SendState, Failed, Pending, Sent (+8 more)

### Community 118 - "Nav"
Cohesion: 0.15
Nodes (10): la_rejilla_se_recorre_con_flechas_sin_salirse(), Nav, Activate, Down, Left, NextTab, PrevTab, Right (+2 more)

### Community 119 - "jobs"
Cohesion: 0.18
Nodes (10): env, CARGO_INCREMENTAL, CARGO_TERM_COLOR, RUST_TOOLCHAIN, jobs, linux, name, needs (+2 more)

### Community 120 - "reactions.rs"
Cohesion: 0.29
Nodes (7): message_exists(), my_reaction_moves_between_emojis(), r(), set_get_replace_and_unknown_message(), setup(), Store, ReactionRow

### Community 121 - "Step"
Cohesion: 0.33
Nodes (6): Step, Checking, Code, Password, Phone, SignUp

### Community 123 - "Marca propia — Parlo"
Cohesion: 0.08
Nodes (23): ADR-0006: Empaquetado multiplataforma (Windows, macOS, Linux), Consecuencias, Decisiones, 1. Criterios de nombre, 2. Candidatos (6-8), 3. Chequeos que debe hacer el usuario, 4. Voz y tono (español), 5. Brief de logo (+15 more)

### Community 124 - "CodeKind"
Cohesion: 0.22
Nodes (9): CodeKind, App, Call, Email, FlashCall, MissedCall, Other, Sms (+1 more)

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

### Community 133 - "PeerRef"
Cohesion: 0.14
Nodes (8): DialogsApi, MessagesApi, PeerRef, Group, Secret, User, Draft, LocalHit

### Community 139 - "stores.rs"
Cohesion: 0.12
Nodes (24): aad(), AAD_PREFIX, dh(), disk_bytes(), encode(), existing_session_json_is_migrated_once_and_deleted(), FIXED_LEN, NOW (+16 more)

### Community 140 - "GroupPos"
Cohesion: 0.29
Nodes (5): Item, Date, Edge, Msg, UnreadMark

### Community 141 - "windows"
Cohesion: 0.29
Nodes (7): WINDOWS_CERT_PASSWORD, WINDOWS_CERT_PFX_BASE64, windows, env, name, needs, runs-on

### Community 143 - "tests/snapshot.rs"
Cohesion: 0.17
Nodes (14): StateKind, Discarded, Incoming, Ready, Requested, SeededEntropy, corrupt_or_foreign_snapshots_are_rejected(), dh() (+6 more)

### Community 144 - "ConnectionState"
Cohesion: 0.33
Nodes (5): ConnectionState, Connecting, Offline, Online, Updating

### Community 145 - "meta"
Cohesion: 0.40
Nodes (5): meta, name, outputs, runs-on, version

### Community 147 - "Lcg"
Cohesion: 0.22
Nodes (4): incoming_text(), below_stays_in_range(), is_deterministic(), Lcg

### Community 165 - "Phase"
Cohesion: 0.50
Nodes (4): Phase, Auth, Connecting, Main

### Community 211 - "on"
Cohesion: 0.50
Nodes (4): on, push, workflow_dispatch, tags

### Community 213 - "BackendError"
Cohesion: 0.17
Nodes (11): BackendError, Crypto, FloodWait, Forbidden, Internal, InvalidInput, Network, NotFound (+3 more)

### Community 214 - ".start"
Cohesion: 0.36
Nodes (5): Animator<K>, Curve, interrumpible_hereda_posicion(), movimiento_reducido_salta_posicion_y_limita_opacidad(), muelle_converge_y_termina()

### Community 215 - "env"
Cohesion: 0.67
Nodes (3): GH_TOKEN, VERSION, env

### Community 217 - "select.rs"
Cohesion: 0.31
Nodes (10): Choice, Mock, Telegram, choose(), FAKE_HASH, missing_credentials(), por_defecto_el_mock_sin_mirar_credenciales(), sin_credenciales_vuelve_al_mock_con_aviso_que_no_ensena_valores() (+2 more)

### Community 220 - "MotionPreference"
Cohesion: 0.22
Nodes (7): MotionPreference, Full, Reduced, Property, Opacity, Position, Scale

### Community 221 - "MsgLayout"
Cohesion: 0.22
Nodes (6): Body, Bubble, Emoji, Sticker, LayoutCache, MsgLayout

### Community 223 - "ThemeKind"
Cohesion: 0.33
Nodes (4): bubble_out_gradient(), ThemeKind, Dark, Light

### Community 224 - "Anchor"
Cohesion: 0.33
Nodes (5): Anchor, After, Around, Before, Latest

### Community 225 - "TopLevel"
Cohesion: 0.40
Nodes (3): TopLevel, Layer, LegacyService

### Community 226 - "Easing"
Cohesion: 0.40
Nodes (5): Easing, .ESTANDAR, .RAPIDO, .SALIDA, .SUAVE

### Community 227 - "GroupKind"
Cohesion: 0.50
Nodes (4): GroupKind, Basic, Channel, Super

## Knowledge Gaps
- **351 isolated node(s):** `LastSeen`, `Offline`, `Online`, `Recently`, `Dark` (+346 more)
  These have ≤1 connection - possible missing edges or undocumented components. (Counts symbols only; 1178 node(s) total have ≤1 connection when file, concept and rationale nodes are included.)
- **95 thin communities (<3 nodes) omitted from report** — run `graphify query` to explore isolated nodes.

## Suggested Questions
_Questions this graph is uniquely positioned to answer:_

- **Why does `Msg` connect `tc-mock/src/lib.rs` to `style.rs`, `tc-telegram/src/secret.rs`, `App`, `app.rs`, `widgets.rs`, `Inner`, `Flags`, `tests_m2.rs`, `sidebar.rs`, `conversation.rs`, `tc-core/src/lib.rs`, `Folder`, `view/login.rs`, `LoginMsg`, `tc-mock/src/lib.rs`, `tc-ui/src/tests.rs`, `Login`, `tests/secret.rs`, `tc-stickers/src/tests.rs`, `src/login.rs`, `Overlay`, `Nav`?**
  _High betweenness centrality (0.090) - this node is a cross-community bridge._
- **Are the 40 inferred relationships involving `c()` (e.g. with `handler()` and `.boot()`) actually correct?**
  _`c()` has 40 INFERRED edges - model-reasoned connections that need verification._
- **What connects `LastSeen`, `Offline`, `Online` to the rest of the system?**
  _351 weakly-connected nodes found - possible documentation gaps or missing edges._
- **Should `style.rs` be split into smaller, more focused modules?**
  _Cohesion score 0.12956107879428874 - nodes in this community are weakly interconnected._
- **Why does `App` connect `App` to `style.rs`, `tc-mock/src/lib.rs`, `tc-telegram/src/secret.rs`, `app.rs`, `Inner`, `Phase`, `Flags`, `tests_m2.rs`, `tc-ui/src/stickers.rs`, `sidebar.rs`, `conversation.rs`, `anim.rs`, `tc-core/src/lib.rs`, `Folder`, `view/login.rs`, `tc-mock/src/lib.rs`, `tc-ui/src/tests.rs`, `Login`, `src/login.rs`, `Overlay`, `Message`, `Nav`?**
  _High betweenness centrality (0.044) - this node is a cross-community bridge._
- **Are the 17 inferred relationships involving `PeerRef` (e.g. with `.save_draft()` and `.set_archived()`) actually correct?**
  _`PeerRef` has 17 INFERRED edges - model-reasoned connections that need verification._
- **Should `MockBackend` be split into smaller, more focused modules?**
  _Cohesion score 0.06585365853658537 - nodes in this community are weakly interconnected._