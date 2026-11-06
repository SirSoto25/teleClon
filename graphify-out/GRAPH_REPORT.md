# Graph Report - teleClon  (2026-11-07)

## Corpus Check
- 135 files · ~133,889 words
- Verdict: corpus is large enough that graph structure adds value.
- Unclassified: 4 file(s) not represented in the graph (top: (none) 2, .example 1, .csv 1)

## Summary
- 3333 nodes · 8449 edges · 184 communities (121 shown, 63 thin omitted)
- Extraction: 92% EXTRACTED · 8% INFERRED · 0% AMBIGUOUS · INFERRED: 674 edges (avg confidence: 0.76)
- Token cost: 0 input · 0 output

## Graph Freshness
- Built from commit: `286e91f4`
- Run `git rev-parse HEAD` and compare to check if the graph is stale.
- Run `graphify update .` after code changes (no API cost).

## Community Hubs (Navigation)
- style.rs
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
- conversation.rs
- Message
- model.rs
- BackendResult
- App
- .update
- widgets.rs
- api.rs
- anim.rs
- SecretChatId
- Folder
- tests_m2.rs
- backend.rs
- tc-telegram/src/auth.rs
- icon
- GrammersBackend
- icon
- Inner
- tc-ui/src/tests.rs
- EntityKind
- groups.rs
- tc-store/src/convert.rs
- TcSession
- MsgLayout
- vault.rs
- dh.rs
- .start
- StoreError
- tc-secret/src/session.rs
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
- MotionPreference
- BackendResult
- SecretChat
- secret.rs
- messages.rs
- SelfDestructTimer
- tl.rs
- UserId
- mapping.rs
- tc-stickers/src/tests.rs
- types.rs
- tc_core
- BackendResult
- Store
- .u32
- Event
- EntityKind
- SessionSnapshot
- view/login.rs
- LoginMsg
- Action
- tc-mock/src/lib.rs
- main.rs
- .lock
- Login
- Writer
- cached_tests.rs
- tl_visual_ttl.rs
- 2. Casos por flujo
- tc-secret
- uploads.rs
- CLAUDE.md
- .claude/CLAUDE.md
- tests/secret.rs
- folders.rs
- src/login.rs
- mapping.rs
- KeyringVault
- tests/snapshot.rs
- Message
- Nav
- Spring
- reactions.rs
- Step
- State
- Marca propia — teleClon
- CodeKind
- tc-store
- Informe de barrido de marca
- GroupKind
- pattern.rs
- GroupPos
- EntityKind
- Lcg
- MemoryVault
- Overlay
- media_from_tl
- UpdateSource
- handler
- layout_message
- key_panel
- ThemeKind
- code_kind
- Body

## God Nodes (most connected - your core abstractions)
1. `Msg` - 186 edges
2. `App` - 136 edges
3. `c()` - 81 edges
4. `BackendResult` - 78 edges
5. `PeerRef` - 65 edges
6. `MockBackend` - 63 edges
7. `Brasa` - 56 edges
8. `Message` - 54 edges
9. `TcSession` - 45 edges
10. `CachedBackend<B>` - 43 edges

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
- 1-file cycle: `crates/tc-theme/src/tokens.rs -> crates/tc-theme/src/tokens.rs`
- 1-file cycle: `crates/tc-store/src/vault.rs -> crates/tc-store/src/vault.rs`
- 2-file cycle: `crates/tc-telegram/src/auth.rs -> crates/tc-telegram/src/lib.rs -> crates/tc-telegram/src/auth.rs`
- 2-file cycle: `crates/tc-telegram/src/lib.rs -> crates/tc-telegram/src/secret.rs -> crates/tc-telegram/src/lib.rs`
- 2-file cycle: `crates/tc-secret/src/session.rs -> crates/tc-secret/src/session/snapshot.rs -> crates/tc-secret/src/session.rs`
- 2-file cycle: `crates/tc-secret/src/dh.rs -> crates/tc-secret/src/testing.rs -> crates/tc-secret/src/dh.rs`
- 2-file cycle: `crates/tc-ui/src/app.rs -> crates/tc-ui/src/login.rs -> crates/tc-ui/src/app.rs`

## Hyperedges (group relationships)
- **Implementaciones y decoradores de ChatBackend** — docs_architecture_mockbackend, docs_architecture_cachedbackend, docs_architecture_tc_telegram, docs_architecture_chatbackend [EXTRACTED 0.95]
- **Fundamentos visuales de Brasa** — docs_design_system_paleta_oklch, docs_design_system_tipografia, docs_design_system_espaciado_radios, docs_design_system_elevacion, docs_design_system_movimiento [EXTRACTED 0.95]
- **Decisiones de arquitectura ADR-0001 a ADR-0004** — docs_adr_0001_stack, docs_adr_0002_direccion_visual, docs_adr_0003_arquitectura, docs_adr_0004_stickers_y_chats_secretos [EXTRACTED 1.00]
- **Pila de chats secretos (cripto, claves y UI)** — docs_architecture_tc_secret, docs_architecture_keyring_boveda, docs_design_system_chat_secreto_ui [INFERRED 0.75]
- **Implementaciones de ChatBackend (Mock, Cached, Telegram)** — docs_adr_0003_arquitectura_chatbackend, docs_adr_0003_arquitectura_mockbackend, docs_adr_0003_arquitectura_cachedbackend, docs_adr_0001_stack_grammers [INFERRED 0.85]
- **Pila de chats secretos E2E** — docs_adr_0004_stickers_y_chats_secretos_secret_chats, docs_adr_0003_arquitectura_tc_secret, docs_adr_0001_stack_keyring, docs_adr_0004_stickers_y_chats_secretos_crypto_risk [INFERRED 0.85]

## Communities (184 total, 63 thin omitted)

### Community 0 - "style.rs"
Cohesion: 0.15
Nodes (41): app_style(), avatar_ink(), badge(), bare_input(), border_fill(), Brasa, bubble(), c() (+33 more)

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
Cohesion: 0.30
Nodes (10): colores_coinciden_con_tokens_json(), contraste_todos_los_pares_declarados_en_tokens_json(), espaciado_radios_bordes_y_elevacion_coinciden(), f(), layout_coincide(), movimiento_coincide(), muelles_convergen_y_rebote_se_pasa(), patron_tokens_y_determinismo() (+2 more)

### Community 6 - "BackendEvent"
Cohesion: 0.07
Nodes (26): BackendError, Crypto, FloodWait, Forbidden, Internal, InvalidInput, Network, NotFound (+18 more)

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
Cohesion: 0.17
Nodes (25): Timestamp, UserStatus, LastSeen, Offline, Online, Recently, date_separator(), day_key() (+17 more)

### Community 11 - "model.rs"
Cohesion: 0.16
Nodes (12): INK, WHITE, AA_NORMAL, AA_UI, Alpha, Alphas, Rgb, DARK (+4 more)

### Community 12 - "conversation.rs"
Cohesion: 0.15
Nodes (30): MsgLayout, to_spans(), with_weight(), body(), bubble(), bubble_max(), build_items(), compose_strip() (+22 more)

### Community 13 - "Message"
Cohesion: 0.05
Nodes (24): Easing, .ESTANDAR, .RAPIDO, .SALIDA, .SUAVE, AnimKey, BubbleY, ChatSwitch (+16 more)

### Community 14 - "model.rs"
Cohesion: 0.12
Nodes (15): display_name_joins_parts(), Media, MediaKind, File, Gif, Photo, Video, Voice (+7 more)

### Community 15 - "BackendResult"
Cohesion: 0.14
Nodes (12): ChatBackend, DialogsApi, MediaApi, MessagesApi, ProfileApi, StickersApi, BackendResult, PeerRef (+4 more)

### Community 16 - "App"
Cohesion: 0.09
Nodes (4): BubbleOpacity, App, Reply, error_text()

### Community 17 - ".update"
Cohesion: 0.11
Nodes (18): BUNDLED, DISPLAY, DISPLAY_FAMILY, EMOJI_FAMILY, family(), MONO, MONO_FALLBACK, MONO_FAMILY (+10 more)

### Community 18 - "widgets.rs"
Cohesion: 0.06
Nodes (11): base(), Font_, IcedMeasurer, Paragraph, SHAPING, WRAP, font_of(), ui_font() (+3 more)

### Community 19 - "api.rs"
Cohesion: 0.16
Nodes (15): ADR-0002: Dirección visual Brasa, Dirección Brasa (grafito cálido + acento persimón), Controlled para chrome, innovative para burbujas y fondo, Movimiento con muelles 119-221 ms y movimiento reducido, OKLCH como fuente de verdad y contraste WCAG AA (144 pares), docs/design/system.md y tokens.json, Tipografía libre (Atkinson Hyperlegible Next, Bricolage Grotesque, JetBrains Mono) e iconos Phosphor, Agente diseño (opus/high) (+7 more)

### Community 20 - "anim.rs"
Cohesion: 0.08
Nodes (12): classify(), main(), report(), scan(), build(), enc(), in_gamut(), lum() (+4 more)

### Community 21 - "SecretChatId"
Cohesion: 0.19
Nodes (4): SecretChatsApi, SecretChatId, KeyVisual, SecretChat

### Community 22 - "Folder"
Cohesion: 0.18
Nodes (8): Folder, All, Channels, Groups, Personal, Unread, sort_dialogs(), folder_icon()

### Community 23 - "tests_m2.rs"
Cohesion: 0.07
Nodes (33): Auth, Backend, Connector, Flags, Phase, Auth, Connecting, Main (+25 more)

### Community 24 - "backend.rs"
Cohesion: 0.23
Nodes (11): MAX_TEXT, channel_of(), CHANNEL_OFFSET, decode_group(), group_ids_roundtrip_and_never_collide(), GroupTarget, Channel, Chat (+3 more)

### Community 25 - "tc-telegram/src/auth.rs"
Cohesion: 0.12
Nodes (8): AuthInner, migrate_dc(), phone_validation(), qr_url(), qr_url_is_unpadded_base64url(), Srp, srp_params(), valid_phone()

### Community 26 - "icon"
Cohesion: 0.18
Nodes (13): handle(), icon(), icon_fill(), icons!, view(), avatar_radius(), bubble_radius(), rounded_fill() (+5 more)

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
Cohesion: 0.08
Nodes (35): apply_dialogs(), apply_history(), cached_dialogs(), cached_history(), decode_entities(), decode_sticker(), dialogs_complete_key(), encode_entities() (+27 more)

### Community 34 - "TcSession"
Cohesion: 0.11
Nodes (9): changes_mark_dirty_and_reset_forgets_everything(), debug_never_prints_the_auth_key(), rejects_garbage_and_unknown_versions(), roundtrip_keeps_keys_peers_and_state(), sample(), SESSION_FORMAT, SessionError, SessionFile (+1 more)

### Community 35 - "MsgLayout"
Cohesion: 0.21
Nodes (11): Índice maestro de teleClon, architecture.md (crates, ChatBackend, SQLite, F01-F18), graphify-out (graph.html, GRAPH_REPORT.md, graph.json), Reglas de trabajo (log, ADR, clean-room, credenciales, regenerar grafo), Credenciales api_id/api_hash en .env local gitignored, graphify: instalación y uso, graphifyy en ./.venv local, Registro de skills usadas (+3 more)

### Community 36 - "vault.rs"
Cohesion: 0.28
Nodes (13): clear_session(), delete_secret_chat_key(), key_ref_is_a_name_not_a_secret(), load_secret_chat_key(), load_session(), memory_vault_roundtrip_and_isolation(), save_secret_chat_key(), save_session() (+5 more)

### Community 37 - "dh.rs"
Cohesion: 0.06
Nodes (28): dh_matches_independent_vector(), DhParams, DhSecret, exp(), FP, GA, GB, generate_mixes_server_random_and_rejects_bad_peer() (+20 more)

### Community 38 - ".start"
Cohesion: 0.36
Nodes (5): Animator<K>, Curve, interrumpible_hereda_posicion(), movimiento_reducido_salta_posicion_y_limita_opacidad(), muelle_converge_y_termina()

### Community 39 - "StoreError"
Cohesion: 0.16
Nodes (8): file_store_is_private_and_atomic(), FileSessionStore, MemorySessionStore, SessionStore, StoreError, Backend, Format, Io

### Community 40 - "tc-secret/src/session.rs"
Cohesion: 0.05
Nodes (35): DEFAULT_PEER_LAYER, MAX_QUEUED, MIN_SEND_LAYER, OUR_LAYER, Outbound, AcceptEncryption, DiscardEncryption, RequestEncryption (+27 more)

### Community 41 - "StickerId"
Cohesion: 0.24
Nodes (4): StickerId, DEFAULT_RECENTS, Favorites, Recents

### Community 42 - "tc-ui/src/stickers.rs"
Cohesion: 0.08
Nodes (29): StickerSetId, Sticker, StickerFormat, Lottie, Webm, Webp, StickerSet, SET_ID (+21 more)

### Community 44 - "tc-telegram/src/lib.rs"
Cohesion: 0.09
Nodes (8): Feed, None, Pool, Source, GrammersBackend, has_login_token(), persister(), Transport

### Community 45 - "Entropy"
Cohesion: 0.14
Nodes (20): Role, Initiator, Responder, Entropy, &mut E, OsEntropy, random_i64(), random_up_to() (+12 more)

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
Cohesion: 0.12
Nodes (22): Choice, Mock, Telegram, choose(), FAKE_HASH, missing_credentials(), por_defecto_el_mock_sin_mirar_credenciales(), sin_credenciales_vuelve_al_mock_con_aviso_que_no_ensena_valores() (+14 more)

### Community 52 - "crypto.rs"
Cohesion: 0.11
Nodes (26): decrypt(), derive(), encrypt(), encrypt_with_padding(), HEADER_LEN, ige_decrypt(), ige_encrypt(), IgeDec (+18 more)

### Community 53 - "sidebar.rs"
Cohesion: 0.09
Nodes (9): GROUPS, LIST_SCROLL_ID, SEARCH_ID, is_animated(), NAME_CHARS, PREVIEW_CHARS, ROW_H, art() (+1 more)

### Community 54 - "Cache"
Cohesion: 0.15
Nodes (11): is_muted(), Applied, apply_batch(), apply_update(), Cache, deleted(), DialogList, MSG_PEER_CAP (+3 more)

### Community 55 - "dialogs.rs"
Cohesion: 0.09
Nodes (16): history_is_servable(), DIALOG_COLS, dialog_from_row(), draft_from_row(), drafts_set_clear_list(), list_orders_pinned_then_activity_and_filters_archived(), mark_read_and_missing_dialog_errors(), notify_scopes() (+8 more)

### Community 56 - "tc-stickers/src/lib.rs"
Cohesion: 0.67
Nodes (3): Disposición (raíl, lista, conversación, panel info), Lista de chats (componente), Barra lateral (raíl de carpetas)

### Community 59 - "tc-secret/src/session.rs"
Cohesion: 0.08
Nodes (24): Result, SecretError, Entropy, FingerprintMismatch, InvalidDhConfig, InvalidPublicValue, InvalidState, Malformed (+16 more)

### Community 60 - "Frame"
Cohesion: 0.14
Nodes (4): CacheKey, DEFAULT_CACHE_BYTES, FrameCache, Frame

### Community 61 - "tc-store/src/lib.rs"
Cohesion: 0.09
Nodes (20): account_db_path(), APP_DIR, default_data_dir(), MIGRATIONS, SCHEMA_VERSION, Error, Invalid, Io (+12 more)

### Community 64 - "net.rs"
Cohesion: 0.10
Nodes (21): ConnectionState, Connecting, Offline, Online, Updating, ConnTracker, errors_map_to_domain(), flood_wait_policy_sleeps_only_short_waits() (+13 more)

### Community 65 - "pack.rs"
Cohesion: 0.11
Nodes (25): Def, DEFS, dots(), index_of(), Layer, len(), NATIVE_SIZE, P (+17 more)

### Community 66 - "convert.rs"
Cohesion: 0.11
Nodes (16): dialog_from_tl(), dialog_stub(), DialogCtx, draft_date(), draft_from_tl(), entities_from_tl(), entities_to_tl(), entity_parts() (+8 more)

### Community 67 - "tests/mod.rs"
Cohesion: 0.12
Nodes (11): startup_without_session_is_logged_out_and_secret_chats_need_login(), update_pump_maps_events_and_recovers_from_errors(), backend(), cfg(), Fake, Handler, logged_session(), logged_session_as() (+3 more)

### Community 68 - "flows.rs"
Cohesion: 0.18
Nodes (21): authorization(), chat(), state(), updates(), user(), user_enum(), dialogs_paginate_with_server_offsets(), edit_delete_react_and_read() (+13 more)

### Community 69 - "MotionPreference"
Cohesion: 0.17
Nodes (9): MotionPreference, Full, Reduced, Property, Opacity, Position, Scale, Anim (+1 more)

### Community 70 - "BackendResult"
Cohesion: 0.11
Nodes (15): AuthApi, AuthState, Checking, LoggedIn, LoggedOut, SignUpRequired, WaitingCode, WaitingPassword (+7 more)

### Community 71 - "SecretChat"
Cohesion: 0.12
Nodes (6): SecretChat, State, Discarded, Incoming, Ready, Requested

### Community 72 - "secret.rs"
Cohesion: 0.13
Nodes (16): chat(), CHAT_COLS, chat_from_row(), chat_roundtrip_and_fk(), messages_idempotent_blob_roundtrip_and_page(), msg(), MSG_COLS, msg_from_row() (+8 more)

### Community 73 - "messages.rs"
Cohesion: 0.10
Nodes (19): COLS, fill(), from_row(), fts_query(), fts_search_accents_prefix_scope_and_delete(), ids(), pagination_by_anchor(), pending_becomes_confirmed_by_random_id() (+11 more)

### Community 75 - "tl.rs"
Cohesion: 0.09
Nodes (22): ID_ABORT_KEY, ID_ACCEPT_KEY, ID_COMMIT_KEY, ID_DELETE, ID_FLUSH, ID_LAYER, ID_MEDIA_EMPTY, ID_MESSAGE (+14 more)

### Community 76 - "UserId"
Cohesion: 0.14
Nodes (7): GroupId, UserId, Group, User, PeerStore, user_from_tl(), user_status()

### Community 77 - "mapping.rs"
Cohesion: 0.22
Nodes (15): key_visualization_matches_independent_vector(), dialog(), hex(), message(), msg_enum(), notify(), password(), peer_channel() (+7 more)

### Community 78 - "tc-stickers/src/tests.rs"
Cohesion: 0.07
Nodes (30): StickerError, BadSize, FrameOutOfRange, Gzip, Image, LottieDisabled, LottieLoad, NotLottie (+22 more)

### Community 79 - "types.rs"
Cohesion: 0.15
Nodes (8): scope_parts(), str_enum!, NotifyScope, Channels, Chat, Groups, Users, NotifySettings

### Community 80 - "tc_core"
Cohesion: 0.12
Nodes (8): BIG_FILE, crate::Inner, GrammersBackend, input_media(), MAX_FILE, mime_for(), PART_SIZE, parts_for()

### Community 81 - "BackendResult"
Cohesion: 0.10
Nodes (11): Anchor, After, Around, Before, Latest, Dialog, DialogsPage, Draft (+3 more)

### Community 82 - "Store"
Cohesion: 0.13
Nodes (13): favorites(), RECENT_CAP, recents_order_dedupe_cap_and_cascade(), seed(), SET_COLS, set_from_row(), sets_and_stickers(), STICKER_COLS (+5 more)

### Community 83 - ".u32"
Cohesion: 0.32
Nodes (8): read_action(), read_entity(), read_message(), Reader, Reader<'a>, write_action(), write_entity(), write_message()

### Community 84 - "Event"
Cohesion: 0.13
Nodes (15): Event, Deleted, Discarded, Duplicate, HistoryFlushed, KeyChanged, PeerLayer, PeerLayerNewer (+7 more)

### Community 85 - "EntityKind"
Cohesion: 0.11
Nodes (17): EntityKind, Blockquote, Bold, BotCommand, Code, CustomEmoji, Email, Hashtag (+9 more)

### Community 86 - "SessionSnapshot"
Cohesion: 0.13
Nodes (6): Rd<'a>, read_ready(), SecretChat, SessionSnapshot, Wr, write_ready()

### Community 87 - "view/login.rs"
Cohesion: 0.25
Nodes (22): card_screen(), CARD_W, code(), connecting(), divider(), field(), field_error(), ghost() (+14 more)

### Community 88 - "LoginMsg"
Cohesion: 0.12
Nodes (19): LoginMsg, CodeChanged, CodeSent, PasswordChanged, PasswordChecked, PhoneChanged, QrReady, Resend (+11 more)

### Community 89 - "Action"
Cohesion: 0.12
Nodes (17): Action, AbortKey, AcceptKey, CommitKey, DeleteMessages, FlushHistory, Noop, NotifyLayer (+9 more)

### Community 90 - "tc-mock/src/lib.rs"
Cohesion: 0.08
Nodes (20): ANIMATED_SET_ID, animated_sticker_set(), BIG_CHAT, BIG_CHAT_LEN, build(), CHANNEL_POSTS, gen_channel(), gen_history() (+12 more)

### Community 91 - "main.rs"
Cohesion: 0.08
Nodes (18): asignaciones_comillas_y_comentarios(), el_entorno_manda_sobre_el_archivo_salvo_si_esta_vacio(), EnvFile, lookup(), unquote(), demo_script(), env_flag(), env_u64() (+10 more)

### Community 92 - ".lock"
Cohesion: 0.09
Nodes (3): incoming_text(), Inner, MockBackend

### Community 93 - "Login"
Cohesion: 0.17
Nodes (9): countdown(), cuentas_atras(), errores_con_texto_propio(), Field, Password, Phone, left(), Login (+1 more)

### Community 95 - "cached_tests.rs"
Cohesion: 0.13
Nodes (22): CachedBackend, all(), cache_persists_across_restarts_on_disk(), cached(), cold_cache_offline_is_an_error_and_pagination_works(), dialogs_roundtrip_exactly_and_survive_offline(), edits_deletes_reactions_read_and_failures_write_through(), events_are_persisted_and_searchable_locally() (+14 more)

### Community 96 - "tl_visual_ttl.rs"
Cohesion: 0.14
Nodes (13): DecryptedMessage, Message, Service, Media, Empty, PlainMessage, TopLevel, Layer (+5 more)

### Community 97 - "2. Casos por flujo"
Cohesion: 0.08
Nodes (23): 1. Estrategia, 2. Casos por flujo, 3. Pruebas transversales, 4. Orden y criterios, F01 Iniciar sesión con teléfono (S01 → S02 → S04 → S05 → S06 → S07), F02 Login con QR (S01 → S02 → S03 → S07), F03 Abrir chat y leer (S07 → S08 → S11), F04 Enviar mensaje de texto (S11 → S13) (+15 more)

### Community 98 - "tc-secret"
Cohesion: 0.40
Nodes (4): Garantías locales, Lo que NO está hecho o NO está verificado, Módulos, tc-secret

### Community 99 - "uploads.rs"
Cohesion: 0.16
Nodes (7): UploadRow, COLS, destination_rules_and_cascade(), from_row(), lifecycle_progress_resume_and_purge(), new_upload(), Store

### Community 106 - "tests/secret.rs"
Cohesion: 0.14
Nodes (17): ANA, BETO, ChanSource, CHAT, fingerprint(), handshake(), HASH, PEER (+9 more)

### Community 112 - "folders.rs"
Cohesion: 0.19
Nodes (8): COLS, folder(), folder_peers_replace_and_cascade(), folders_crud_order_and_reorder(), from_row(), Store, FolderPeerRow, FolderRow

### Community 113 - "src/login.rs"
Cohesion: 0.18
Nodes (8): clean_code(), CODE_ID, code_where(), codigo_solo_cifras_y_recortado(), normalize_phone(), PASSWORD_ID, PHONE_ID, telefono_en_formato_internacional()

### Community 114 - "mapping.rs"
Cohesion: 0.23
Nodes (14): channel_group(), chat_group(), group_from_tl(), channel(), cache(), doc_media(), document(), input_peers_need_known_hashes() (+6 more)

### Community 115 - "KeyringVault"
Cohesion: 0.20
Nodes (9): construction_and_validation_do_not_touch_the_system(), KeyringVault, live_roundtrip(), map_err(), VaultError, Backend, InvalidKey, Unavailable (+1 more)

### Community 116 - "tests/snapshot.rs"
Cohesion: 0.17
Nodes (14): StateKind, Discarded, Incoming, Ready, Requested, SeededEntropy, corrupt_or_foreign_snapshots_are_rejected(), dh() (+6 more)

### Community 117 - "Message"
Cohesion: 0.11
Nodes (18): MsgId, RandomId, Message, OutgoingText, SendState, Failed, Pending, Sent (+10 more)

### Community 118 - "Nav"
Cohesion: 0.15
Nodes (10): la_rejilla_se_recorre_con_flechas_sin_salirse(), Nav, Activate, Down, Left, NextTab, PrevTab, Right (+2 more)

### Community 119 - "Spring"
Cohesion: 0.15
Nodes (13): HIGHLIGHT_FADE_MS, REDUCED_OPACITY_MAX_MS, SKELETON_PULSE_PERIOD_MS, Spring, .ESTANDAR, .RAPIDO, .REBOTE, .SUAVE (+5 more)

### Community 120 - "reactions.rs"
Cohesion: 0.29
Nodes (7): message_exists(), my_reaction_moves_between_emojis(), r(), set_get_replace_and_unknown_message(), setup(), Store, ReactionRow

### Community 121 - "Step"
Cohesion: 0.16
Nodes (11): App, Code, focus_for(), Step, Checking, Code, Password, Phone (+3 more)

### Community 123 - "Marca propia — teleClon"
Cohesion: 0.22
Nodes (8): 1. Criterios de nombre, 2. Candidatos (6-8), 3. Chequeos que debe hacer el usuario, 4. Voz y tono (español), 5. Brief de logo, 6. Palabras prohibidas y restos, 7. Siguientes pasos, Marca propia — teleClon

### Community 124 - "CodeKind"
Cohesion: 0.22
Nodes (9): CodeKind, App, Call, Email, FlashCall, MissedCall, Other, Sms (+1 more)

### Community 125 - "tc-store"
Cohesion: 0.33
Nodes (5): Bóveda de secretos, CachedBackend, Contenido, Decisiones y límites, tc-store

### Community 126 - "Informe de barrido de marca"
Cohesion: 0.33
Nodes (5): A revisar (código que podría llegar a pantalla), Bloqueantes (texto visible o assets), Informe de barrido de marca, Permitidas (resumen por archivo), Resumen

### Community 127 - "GroupKind"
Cohesion: 0.50
Nodes (4): GroupKind, Basic, Channel, Super

### Community 128 - "pattern.rs"
Cohesion: 0.26
Nodes (13): AMPLITUDE, cell_segments(), ContourParams, .DEFAULT, .SECRET, contours(), field(), lattice() (+5 more)

### Community 140 - "GroupPos"
Cohesion: 0.17
Nodes (10): Item, Date, Edge, Msg, UnreadMark, GroupPos, First, Last (+2 more)

### Community 141 - "EntityKind"
Cohesion: 0.18
Nodes (11): EntityKind, Bold, Code, Hashtag, Italic, Mention, Pre, Spoiler (+3 more)

### Community 143 - "Lcg"
Cohesion: 0.31
Nodes (3): below_stays_in_range(), is_deterministic(), Lcg

### Community 144 - "MemoryVault"
Cohesion: 0.38
Nodes (3): check_key(), MemoryVault, Secret

### Community 145 - "Overlay"
Cohesion: 0.20
Nodes (8): Compose, Edit, Normal, Overlay, Delete, Key, NewSecret, Ttl

### Community 146 - "media_from_tl"
Cohesion: 0.31
Nodes (4): media_from_document(), media_from_tl(), photo_dims(), sticker_from_document()

### Community 147 - "UpdateSource"
Cohesion: 0.36
Nodes (4): UpdateBatch, UpdateSource, UpdatesReceiver, ScriptSource

### Community 148 - "handler"
Cohesion: 0.28
Nodes (4): handler(), input_chat(), name(), Server

### Community 159 - "layout_message"
Cohesion: 0.33
Nodes (8): emoji_only(), input(), layout_message(), LayoutInput, maquetacion_meta_en_linea_o_debajo(), meta_width(), runs(), tramos_por_entidades()

### Community 160 - "key_panel"
Cohesion: 0.40
Nodes (7): App, experimental_note(), key_panel(), new_secret_panel(), overlay(), toasts(), ttl_panel()

### Community 162 - "ThemeKind"
Cohesion: 0.33
Nodes (4): bubble_out_gradient(), ThemeKind, Dark, Light

### Community 163 - "code_kind"
Cohesion: 0.40
Nodes (3): code_kind(), code_types_map(), next_kind()

### Community 165 - "Body"
Cohesion: 0.50
Nodes (4): Body, Bubble, Emoji, Sticker

## Knowledge Gaps
- **347 isolated node(s):** `Mock`, `Connecting`, `Auth`, `Main`, `Do` (+342 more)
  These have ≤1 connection - possible missing edges or undocumented components. (Counts symbols only; 1061 node(s) total have ≤1 connection when file, concept and rationale nodes are included.)
- **63 thin communities (<3 nodes) omitted from report** — run `graphify query` to explore isolated nodes.

## Suggested Questions
_Questions this graph is uniquely positioned to answer:_

- **Why does `Msg` connect `tc-mock/src/lib.rs` to `style.rs`, `tc-telegram/src/secret.rs`, `conversation.rs`, `Message`, `App`, `Overlay`, `widgets.rs`, `Folder`, `tests_m2.rs`, `icon`, `key_panel`, `sidebar.rs`, `view/login.rs`, `LoginMsg`, `tc-mock/src/lib.rs`, `main.rs`, `tests/secret.rs`, `src/login.rs`, `Nav`, `Step`?**
  _High betweenness centrality (0.106) - this node is a cross-community bridge._
- **Are the 40 inferred relationships involving `c()` (e.g. with `handler()` and `.boot()`) actually correct?**
  _`c()` has 40 INFERRED edges - model-reasoned connections that need verification._
- **What connects `Mock`, `Connecting`, `Auth` to the rest of the system?**
  _347 weakly-connected nodes found - possible documentation gaps or missing edges._
- **Should `style.rs` be split into smaller, more focused modules?**
  _Cohesion score 0.14661654135338345 - nodes in this community are weakly interconnected._
- **Why does `Result` connect `tc-secret/src/session.rs` to `groups.rs`, `tc-store/src/convert.rs`, `uploads.rs`, `dh.rs`, `BackendEvent`, `tc-secret/src/session.rs`, `messages.rs`, `secret.rs`, `tl.rs`, `Entropy`, `folders.rs`, `Store`, `crypto.rs`, `dialogs.rs`, `reactions.rs`, `tc-store/src/lib.rs`?**
  _High betweenness centrality (0.069) - this node is a cross-community bridge._
- **Are the 17 inferred relationships involving `PeerRef` (e.g. with `.save_draft()` and `.set_archived()`) actually correct?**
  _`PeerRef` has 17 INFERRED edges - model-reasoned connections that need verification._
- **Should `tokens.rs` be split into smaller, more focused modules?**
  _Cohesion score 0.03446327683615819 - nodes in this community are weakly interconnected._