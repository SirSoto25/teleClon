# Graph Report - teleClon  (2026-11-13)

## Corpus Check
- 224 files · ~237,202 words
- Verdict: corpus is large enough that graph structure adds value.
- Unclassified: 13 file(s) not represented in the graph (top: (none) 2, .example 1, .opus 1)

## Summary
- 5737 nodes · 15284 edges · 285 communities (167 shown, 118 thin omitted)
- Extraction: 91% EXTRACTED · 9% INFERRED · 0% AMBIGUOUS · INFERRED: 1329 edges (avg confidence: 0.74)
- Token cost: 0 input · 0 output

## Graph Freshness
- Built from commit: `1f91f748`
- Run `git rev-parse HEAD` and compare to check if the graph is stale.
- Run `graphify update .` after code changes (no API cost).

## Community Hubs (Navigation)
- style.rs
- MockBackend
- tokens.rs
- Msg
- Msg
- f
- cached.rs
- tc-telegram/src/secret.rs
- Sistema de diseño de teleClon — dirección «Brasa»
- conv.rs
- format.rs
- model.rs
- tc-ui/src/media.rs
- ci.yml
- ChatMsg
- player.rs
- BackendResult
- .update
- sidebar.rs
- api.rs
- anim.rs
- cached.rs
- super
- app.rs
- Event
- tc_core
- ForumMsg
- GrammersBackend
- icon
- tc-telegram/src/lib.rs
- tc-ui/src/tests.rs
- EntityKind
- tc-mock/src/forum.rs
- TcSession
- MsgLayout
- VaultResult
- dh.rs
- tc-ui/src/gifs.rs
- tests_m2.rs
- with
- StickerId
- tc-ui/src/stickers.rs
- tc-telegram/src/auth.rs
- src/login.rs
- Entropy
- e2e.rs
- build_items
- tests/forum.rs
- ADR-0003: Arquitectura del cliente (workspace, backend tras trait, caché SQLite, E2E sans-IO)
- tc-stickers/src/tests.rs
- config.rs
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
- convert.rs
- conversation.rs
- flows.rs
- tc-ui/src/anim.rs
- media_cache.rs
- view/audio.rs
- secret.rs
- messages.rs
- Login
- tl.rs
- tc-store/src/gifs.rs
- decode.rs
- Backend
- tc-ui/src/audio.rs
- .download_media
- CachedBackend<B>
- Store
- reading.rs
- SecretError
- EntityKind
- SessionSnapshot
- view/login.rs
- LoginMsg
- tests/snapshot.rs
- tc-mock/src/lib.rs
- main.rs
- tests_audio.rs
- MediaCacheLayer<B>
- super
- cached_tests.rs
- tc-mock/src/dialog_actions.rs
- 2. Casos por flujo
- tc-secret
- uploads.rs
- CLAUDE.md
- .claude/CLAUDE.md
- StoreError
- tests/secret.rs
- StickerError
- AuthState
- folders.rs
- files.rs
- sticker_anim.rs
- tl_visual_ttl.rs
- Message
- GifAnims
- jobs
- reactions.rs
- KeyVisualization
- PeerRef
- Marca propia — Parlo
- Category
- tc-store
- Informe de barrido de marca
- env
- release
- tests_reactions.rs
- topics.rs
- peers.rs
- AudioError
- ImageKey
- Track
- NoDevice
- stores.rs
- ByteLru<K, V>
- windows
- .sticker_send_task
- AnimMsg
- emoji.rs
- meta
- 0005-nombre-parlo.md
- model.rs
- tc-telegram/src/media.rs
- tc-store/src/convert.rs
- tc-stickers/src/anim.rs
- tc-telegram/src/reactions.rs
- Timestamp
- profile_photo_key
- tc-store/src/forum.rs
- tests/mod.rs
- view/mod.rs
- Gif
- apply_media_extra
- App
- waveform.rs
- MotionPreference
- .download_media
- App
- Step
- tests_sticker_anim.rs
- .start
- .on_reacted
- CodeKind
- .remember_gifs
- hashmap
- .available_reactions
- Nav
- MuteFor
- tests_chat_actions.rs
- PeerRef
- tc-mock/src/gifs.rs
- .get
- PanelMsg
- tests_forum.rs
- ADR-0007: Audio (notas de voz y música)
- make_fixtures.sh
- on
- build.rs
- install-user.sh
- env
- build-macos.sh
- tests_reading.rs
- build-linux.sh
- AnimMsg
- Recents
- FontRole
- anim_tests.rs
- cached_actions_tests.rs
- Folder
- messages_of
- tests_panel_shots.rs
- context_menu.rs
- input_media
- tc-mock/src/audio.rs
- ADR-0008: Stickers animados (TGS con rlottie; WebM estático)
- .apply_auth
- Elevation
- ADR-0009: Teclado de emojis
- ADR-0010: GIFs (guardados, búsqueda, envío y animación)
- topic_reply_to
- emoji_es.rs

## God Nodes (most connected - your core abstractions)
1. `Msg` - 344 edges
2. `App` - 229 edges
3. `BackendResult` - 210 edges
4. `PeerRef` - 134 edges
5. `c()` - 110 edges
6. `MockBackend` - 97 edges
7. `Message` - 89 edges
8. `MsgId` - 81 edges
9. `ThreadScope` - 74 edges
10. `MediaCacheLayer<B>` - 69 edges

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
- 1-file cycle: `crates/tc-theme/src/tokens.rs -> crates/tc-theme/src/tokens.rs`
- 1-file cycle: `crates/tc-store/src/vault.rs -> crates/tc-store/src/vault.rs`
- 2-file cycle: `crates/tc-secret/src/session.rs -> crates/tc-secret/src/session/snapshot.rs -> crates/tc-secret/src/session.rs`
- 2-file cycle: `crates/tc-audio/src/decode.rs -> crates/tc-audio/src/lib.rs -> crates/tc-audio/src/decode.rs`
- 2-file cycle: `crates/tc-audio/src/lib.rs -> crates/tc-audio/src/player.rs -> crates/tc-audio/src/lib.rs`
- 2-file cycle: `crates/tc-mock/src/forum.rs -> crates/tc-mock/src/lib.rs -> crates/tc-mock/src/forum.rs`
- 2-file cycle: `crates/tc-telegram/src/auth.rs -> crates/tc-telegram/src/lib.rs -> crates/tc-telegram/src/auth.rs`
- 2-file cycle: `crates/tc-telegram/src/lib.rs -> crates/tc-telegram/src/secret.rs -> crates/tc-telegram/src/lib.rs`
- 2-file cycle: `crates/tc-secret/src/dh.rs -> crates/tc-secret/src/testing.rs -> crates/tc-secret/src/dh.rs`
- 2-file cycle: `crates/tc-ui/src/app.rs -> crates/tc-ui/src/media.rs -> crates/tc-ui/src/app.rs`
- 2-file cycle: `crates/tc-ui/src/app.rs -> crates/tc-ui/src/login.rs -> crates/tc-ui/src/app.rs`
- 3-file cycle: `crates/tc-audio/src/decode.rs -> crates/tc-audio/src/ogg.rs -> crates/tc-audio/src/lib.rs -> crates/tc-audio/src/decode.rs`
- 3-file cycle: `crates/tc-audio/src/decode.rs -> crates/tc-audio/src/lib.rs -> crates/tc-audio/src/player.rs -> crates/tc-audio/src/decode.rs`
- 4-file cycle: `crates/tc-audio/src/decode.rs -> crates/tc-audio/src/ogg.rs -> crates/tc-audio/src/lib.rs -> crates/tc-audio/src/player.rs -> crates/tc-audio/src/decode.rs`

## Hyperedges (group relationships)
- **Implementaciones y decoradores de ChatBackend** — docs_architecture_mockbackend, docs_architecture_cachedbackend, docs_architecture_tc_telegram, docs_architecture_chatbackend [EXTRACTED 0.95]
- **Fundamentos visuales de Brasa** — docs_design_system_paleta_oklch, docs_design_system_tipografia, docs_design_system_espaciado_radios, docs_design_system_elevacion, docs_design_system_movimiento [EXTRACTED 0.95]
- **Decisiones de arquitectura ADR-0001 a ADR-0004** — docs_adr_0001_stack, docs_adr_0002_direccion_visual, docs_adr_0003_arquitectura, docs_adr_0004_stickers_y_chats_secretos [EXTRACTED 1.00]
- **Pila de chats secretos (cripto, claves y UI)** — docs_architecture_tc_secret, docs_architecture_keyring_boveda, docs_design_system_chat_secreto_ui [INFERRED 0.75]
- **Implementaciones de ChatBackend (Mock, Cached, Telegram)** — docs_adr_0003_arquitectura_chatbackend, docs_adr_0003_arquitectura_mockbackend, docs_adr_0003_arquitectura_cachedbackend, docs_adr_0001_stack_grammers [INFERRED 0.85]
- **Pila de chats secretos E2E** — docs_adr_0004_stickers_y_chats_secretos_secret_chats, docs_adr_0003_arquitectura_tc_secret, docs_adr_0001_stack_keyring, docs_adr_0004_stickers_y_chats_secretos_crypto_risk [INFERRED 0.85]

## Communities (285 total, 118 thin omitted)

### Community 0 - "style.rs"
Cohesion: 0.16
Nodes (42): app_style(), avatar_ink(), avatar_radius(), badge(), bare_input(), border_fill(), Brasa, bubble() (+34 more)

### Community 2 - "tokens.rs"
Cohesion: 0.03
Nodes (65): AVATAR_LG, AVATAR_MD, AVATAR_RATIO, AVATAR_SM, AVATAR_XL, AVATAR_XS, BODY, BREAKPOINT_SPLIT (+57 more)

### Community 3 - "Msg"
Cohesion: 0.02
Nodes (75): Msg, AcceptSecret, AskDelete, AskDiscardSecret, Backend, CancelCompose, Chat, ClearSearch (+67 more)

### Community 4 - "Msg"
Cohesion: 0.06
Nodes (38): Arquitectura de teleClon, ADR-0001 Stack, ADR-0002 Dirección visual, ADR-0003 Arquitectura, ADR-0004 Stickers y chats secretos, BackendEvent, ChatBackend (trait), Modelo de concurrencia (runtime tokio único, actores) (+30 more)

### Community 5 - "f"
Cohesion: 0.08
Nodes (24): Inner, AMPLITUDE, cell_segments(), ContourParams, .DEFAULT, .SECRET, contours(), field() (+16 more)

### Community 6 - "cached.rs"
Cohesion: 0.03
Nodes (49): ChatBackend, MediaApi, StickersApi, UNSUPPORTED, BackendError, Crypto, FloodWait, Forbidden (+41 more)

### Community 7 - "tc-telegram/src/secret.rs"
Cohesion: 0.06
Nodes (23): Entity, utf16_starts(), now_secs(), Chat, crypto(), EARLY_CAP, entities_from_secret(), entities_round_trip_through_utf16() (+15 more)

### Community 8 - "Sistema de diseño de teleClon — dirección «Brasa»"
Cohesion: 0.15
Nodes (26): about_thirty_chats_sorted_with_pinned_first(), data_is_deterministic(), delete_updates_last_message_and_emits(), edit_only_own_messages(), forward_creates_messages_with_origin(), grupo_con_muchos_no_leidos_y_lectura_parcial(), history_after_and_around(), long_history_paging_covers_everything_without_gaps() (+18 more)

### Community 9 - "conv.rs"
Cohesion: 0.04
Nodes (58): TextStyle, DATE_H, EDGE_H, emoji_only(), Fixed, FWD_H, GAP_GROUP, GAP_IN_GROUP (+50 more)

### Community 10 - "format.rs"
Cohesion: 0.15
Nodes (27): Timestamp, UserStatus, LastSeen, Offline, Online, Recently, todas_las_palabras_en_espanol_apuntan_a_emojis_reales(), date_separator() (+19 more)

### Community 11 - "model.rs"
Cohesion: 0.14
Nodes (13): INK, WHITE, AA_NORMAL, AA_UI, Alpha, Alphas, Rgb, bubble_out_gradient() (+5 more)

### Community 12 - "tc-ui/src/media.rs"
Cohesion: 0.07
Nodes (33): StickerFormat, Lottie, Webm, Webp, AVATAR_PX, avatars_are_square_and_downscaled(), bubble_aspect(), BUBBLE_ASPECT (+25 more)

### Community 13 - "ci.yml"
Cohesion: 0.07
Nodes (29): name, runs-on, steps, strategy, concurrency, cancel-in-progress, group, env (+21 more)

### Community 14 - "ChatMsg"
Cohesion: 0.04
Nodes (62): App, ChatActions, ChatMsg, Archive, ArchivedLoaded, AskBlock, AskClear, AskDeleteChat (+54 more)

### Community 15 - "player.rs"
Cohesion: 0.13
Nodes (15): Cmd, Paused, Play, Speed, Stop, Current, fake(), lock() (+7 more)

### Community 16 - "BackendResult"
Cohesion: 0.06
Nodes (11): DialogsApi, MessagesApi, unsupported(), BackendResult, ThreadScope, PeerRef, Group, Secret (+3 more)

### Community 17 - ".update"
Cohesion: 0.17
Nodes (11): BUNDLED, DISPLAY, DISPLAY_FAMILY, EMOJI_FAMILY, MONO, MONO_FALLBACK, MONO_FAMILY, UI_FALLBACK (+3 more)

### Community 18 - "sidebar.rs"
Cohesion: 0.16
Nodes (15): badge_at(), folder_icon(), LINE_H, list(), rail(), rail_button(), RIGHT_W, ROW_H (+7 more)

### Community 19 - "api.rs"
Cohesion: 0.16
Nodes (15): ADR-0002: Dirección visual Brasa, Dirección Brasa (grafito cálido + acento persimón), Controlled para chrome, innovative para burbujas y fondo, Movimiento con muelles 119-221 ms y movimiento reducido, OKLCH como fuente de verdad y contraste WCAG AA (144 pares), docs/design/system.md y tokens.json, Tipografía libre (Atkinson Hyperlegible Next, Bricolage Grotesque, JetBrains Mono) e iconos Phosphor, Agente diseño (opus/high) (+7 more)

### Community 20 - "anim.rs"
Cohesion: 0.06
Nodes (20): classify(), main(), report(), scan(), build(), enc(), in_gamut(), lum() (+12 more)

### Community 21 - "cached.rs"
Cohesion: 0.08
Nodes (12): ProfileApi, SecretChatsApi, SecretChatId, UserId, display_name_joins_parts(), KeyVisual, SecretChat, SecretChatState (+4 more)

### Community 22 - "super"
Cohesion: 0.11
Nodes (13): key_is_created_once_in_the_vault_and_reused(), roundtrip_and_every_kind_of_tampering_is_rejected(), SEAL_KEY_LEN, Sealed, SealError, BadKey, Entropy, Rejected (+5 more)

### Community 23 - "app.rs"
Cohesion: 0.04
Nodes (33): Easing, .ESTANDAR, .RAPIDO, .SALIDA, .SUAVE, AnimKey, ChatSwitch, Overlay (+25 more)

### Community 24 - "Event"
Cohesion: 0.10
Nodes (24): DiscardReason, FingerprintMismatch, InvalidPublicValue, Local, RekeyFailure, Remote, ResendUnavailable, SeqViolation (+16 more)

### Community 25 - "tc_core"
Cohesion: 0.08
Nodes (24): code_kind(), code_types_map(), next_kind(), phone_validation(), qr_url(), qr_url_is_unpadded_base64url(), valid_phone(), MAX_TEXT (+16 more)

### Community 26 - "ForumMsg"
Cohesion: 0.05
Nodes (33): App, ForumMsg, BackToTopics, ClearQuery, Close, CloseList, CloseMenu, Create (+25 more)

### Community 28 - "icon"
Cohesion: 0.19
Nodes (12): ADR-0001: Stack técnico, grammers (MTProto en Rust), iced (UI en Rust, wgpu + tiny-skia), keyring (claves y sesión), rusqlite bundled con migraciones por PRAGMA user_version, tc-secret: chats secretos sans-IO con RustCrypto, tc-stickers: WebP, TGS con rlottie, WebM aplazado, ADR-0004: Stickers y chats secretos en el alcance (+4 more)

### Community 29 - "tc-telegram/src/lib.rs"
Cohesion: 0.05
Nodes (31): Feed, None, Pool, Source, GrammersBackend, has_login_token(), Inner, persister() (+23 more)

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
Cohesion: 0.08
Nodes (10): changes_mark_dirty_and_reset_forgets_everything(), debug_never_prints_the_auth_key(), MemorySessionStore, rejects_garbage_and_unknown_versions(), roundtrip_keeps_keys_peers_and_state(), sample(), SESSION_FORMAT, SessionError (+2 more)

### Community 35 - "MsgLayout"
Cohesion: 0.21
Nodes (11): Índice maestro de teleClon, architecture.md (crates, ChatBackend, SQLite, F01-F18), graphify-out (graph.html, GRAPH_REPORT.md, graph.json), Reglas de trabajo (log, ADR, clean-room, credenciales, regenerar grafo), Credenciales api_id/api_hash en .env local gitignored, graphify: instalación y uso, graphifyy en ./.venv local, Registro de skills usadas (+3 more)

### Community 36 - "VaultResult"
Cohesion: 0.12
Nodes (22): DeadVault, DeadVault, check_key(), CHUNK_HEADER_LEN, CHUNK_MAGIC, chunked_vault_reports_missing_parts_and_keeps_magic_values_intact(), chunked_vault_splits_large_values_and_cleans_up(), clear_session() (+14 more)

### Community 37 - "dh.rs"
Cohesion: 0.07
Nodes (26): dh_matches_independent_vector(), DhParams, DhSecret, exp(), FP, GA, GB, generate_mixes_server_random_and_rejects_bad_peer() (+18 more)

### Community 38 - "tc-ui/src/gifs.rs"
Cohesion: 0.12
Nodes (17): ANIM_PX, CACHE_BYTES, CELL_GAP, CELL_H, CELL_MIN_W, decode(), decodifica_escala_y_recorre_los_fotogramas(), DEFAULT_DELAY_MS (+9 more)

### Community 39 - "tests_m2.rs"
Cohesion: 0.09
Nodes (27): demo_script(), Auth, Connector, DemoStep, Do, Exit, ScrollUp, Shot (+19 more)

### Community 40 - "with"
Cohesion: 0.20
Nodes (17): steps, steps, steps, steps, id, if, name, run (+9 more)

### Community 41 - "StickerId"
Cohesion: 0.42
Nodes (3): StickerId, DEFAULT_RECENTS, Favorites

### Community 42 - "tc-ui/src/stickers.rs"
Cohesion: 0.10
Nodes (34): StickerSetId, Sticker, StickerSet, SET_ID, ANIMATED_SET_ID, Art, art_index(), ART_PX (+26 more)

### Community 43 - "tc-telegram/src/auth.rs"
Cohesion: 0.12
Nodes (6): AuthInner, GrammersBackend, Inner, migrate_dc(), Srp, srp_params()

### Community 44 - "src/login.rs"
Cohesion: 0.19
Nodes (11): clean_code(), CODE_ID, codigo_solo_cifras_y_recortado(), countdown(), cuentas_atras(), errores_con_texto_propio(), login_error(), normalize_phone() (+3 more)

### Community 45 - "Entropy"
Cohesion: 0.06
Nodes (48): Role, Initiator, Responder, Entropy, &mut E, OsEntropy, random_i64(), random_up_to() (+40 more)

### Community 46 - "e2e.rs"
Cohesion: 0.17
Nodes (34): chat_n(), concurrent_rekey_converges_on_one_instance(), decreasing_in_seq_aborts_chat(), dh(), discard_local_and_remote(), duplicates_are_dropped(), fingerprint_mismatch_discards_chat(), forge() (+26 more)

### Community 47 - "build_items"
Cohesion: 0.31
Nodes (9): replica/features.csv, Rasgos de marca (avatares cuadrado redondeado, burbujas sin cola, fondo de isolíneas), Agente recon (sonnet/medium), Recon map: Telegram Desktop (Linux), Componentes de UI (Burbuja de mensaje, Fila de chat, Compositor...), Core loop (chats, enviar/recibir, sincronizado), Matriz de funciones (features.csv, must/should/could/skip), Flujos F01-F16 (+1 more)

### Community 48 - "tests/forum.rs"
Cohesion: 0.23
Nodes (14): peer_channel(), CH, el_canal_con_forum_es_un_foro_y_las_respuestas_al_tema_no_son_respuestas(), envio_en_tema_lleva_top_msg_id_y_actualiza_el_tema(), forum_channel(), group(), header(), is_channel() (+6 more)

### Community 49 - "ADR-0003: Arquitectura del cliente (workspace, backend tras trait, caché SQLite, E2E sans-IO)"
Cohesion: 0.44
Nodes (10): tc-app, tc-audio, tc-core, tc-mock, tc-secret, tc-stickers, tc-store, tc-telegram (+2 more)

### Community 50 - "tc-stickers/src/tests.rs"
Cohesion: 0.13
Nodes (15): cache_evicts_least_recently_used_by_bytes(), cache_get_or_insert_computes_once(), gradient(), key(), LOTTIE, pack_stickers_are_distinct_and_survive_webp(), render_without_lottie_reports_disabled(), tgs_bomb_is_rejected() (+7 more)

### Community 51 - "config.rs"
Cohesion: 0.12
Nodes (22): Choice, Mock, Telegram, choose(), FAKE_HASH, missing_credentials(), por_defecto_el_mock_sin_mirar_credenciales(), sin_credenciales_vuelve_al_mock_con_aviso_que_no_ensena_valores() (+14 more)

### Community 52 - "crypto.rs"
Cohesion: 0.08
Nodes (28): decrypt(), derive(), encrypt(), encrypt_with_padding(), HEADER_LEN, ige_decrypt(), ige_encrypt(), IgeDec (+20 more)

### Community 53 - "widgets.rs"
Cohesion: 0.10
Nodes (7): initials(), WRAP, avatar(), avatar_photo(), badge(), ContourBg, icon_button()

### Community 54 - "Cache"
Cohesion: 0.12
Nodes (13): is_muted(), reactions_from_tl(), Applied, apply_batch(), apply_update(), Cache, deleted(), DialogList (+5 more)

### Community 55 - "dialogs.rs"
Cohesion: 0.06
Nodes (23): DIALOG_COLS, dialog_from_row(), draft_from_row(), drafts_set_clear_list(), list_orders_pinned_then_activity_and_filters_archived(), mark_read_and_missing_dialog_errors(), notify_scopes(), scope_parts() (+15 more)

### Community 56 - "tc-stickers/src/lib.rs"
Cohesion: 0.67
Nodes (3): Disposición (raíl, lista, conversación, panel info), Lista de chats (componente), Barra lateral (raíl de carpetas)

### Community 59 - "tc-telegram/src/forum.rs"
Cohesion: 0.12
Nodes (4): GrammersBackend, in_topic(), Inner, topic_from_tl()

### Community 60 - "Frame"
Cohesion: 0.15
Nodes (5): CacheKey, DEFAULT_CACHE_BYTES, FrameCache, Frame, ByteLru

### Community 61 - "tc-store/src/lib.rs"
Cohesion: 0.09
Nodes (21): account_db_path(), APP_DIR, DATA_DIR_NAME, default_data_dir(), LEGACY_APP_DIR, MIGRATIONS, resolve_data_dir(), SCHEMA_VERSION (+13 more)

### Community 65 - "pack.rs"
Cohesion: 0.11
Nodes (25): Def, DEFS, dots(), index_of(), Layer, len(), NATIVE_SIZE, P (+17 more)

### Community 66 - "convert.rs"
Cohesion: 0.06
Nodes (35): dialog_from_tl(), dialog_stub(), DialogCtx, document_file_ref(), draft_date(), draft_from_tl(), entities_from_tl(), entities_to_tl() (+27 more)

### Community 67 - "conversation.rs"
Cohesion: 0.10
Nodes (38): now(), Body, Bubble, Emoji, Sticker, MsgLayout, same_group(), signature() (+30 more)

### Community 68 - "flows.rs"
Cohesion: 0.08
Nodes (59): key_visualization_matches_independent_vector(), authorization(), dialog(), hex(), message(), msg_enum(), notify(), password() (+51 more)

### Community 69 - "tc-ui/src/anim.rs"
Cohesion: 0.14
Nodes (15): HIGHLIGHT_FADE_MS, REDUCED_OPACITY_MAX_MS, SKELETON_PULSE_PERIOD_MS, Spring, .ESTANDAR, .RAPIDO, .REBOTE, .SUAVE (+7 more)

### Community 70 - "media_cache.rs"
Cohesion: 0.09
Nodes (15): account_cache_lives_next_to_the_database_and_survives_reopen(), check_key(), create_private_dir(), fetch_cached(), fetch_cached_goes_to_the_network_once_and_never_for_secrets(), io(), keys_depend_on_id_and_size_and_skip_secrets(), media_key() (+7 more)

### Community 71 - "view/audio.rs"
Cohesion: 0.08
Nodes (18): AudioInfo, Media, TrackView, is_real_gif_media(), Bar, BAR_GAP, BAR_W, bubble() (+10 more)

### Community 72 - "secret.rs"
Cohesion: 0.11
Nodes (18): chat(), CHAT_COLS, chat_from_row(), chat_roundtrip_and_fk(), messages_idempotent_blob_roundtrip_and_page(), msg(), MSG_COLS, msg_from_row() (+10 more)

### Community 73 - "messages.rs"
Cohesion: 0.14
Nodes (14): COLS, fill(), from_row(), fts_query(), fts_search_accents_prefix_scope_and_delete(), ids(), pagination_by_anchor(), pending_becomes_confirmed_by_random_id() (+6 more)

### Community 74 - "Login"
Cohesion: 0.18
Nodes (8): Field, Code, Password, Phone, left(), Login, Qr, un_logged_out_repetido_no_borra_lo_escrito()

### Community 75 - "tl.rs"
Cohesion: 0.05
Nodes (53): Action, AbortKey, AcceptKey, CommitKey, DeleteMessages, FlushHistory, Noop, NotifyLayer (+45 more)

### Community 76 - "tc-store/src/gifs.rs"
Cohesion: 0.15
Nodes (12): blocking(), EMOJI_KEEP, emoji_usage(), gif(), gifs_guardados_ida_y_vuelta_y_reemplazo(), record_emoji_use(), saved_gifs(), sin_red_se_devuelven_los_guardados() (+4 more)

### Community 77 - "decode.rs"
Cohesion: 0.09
Nodes (17): check_tone(), copy_out(), decode_all(), decodes_music_formats(), decodes_stereo_opus(), decodes_telegram_style_voice_note(), fixture(), freq() (+9 more)

### Community 78 - "Backend"
Cohesion: 0.14
Nodes (22): Backend, Event, boot(), chat_secreto_huella_y_canal_de_solo_lectura(), entrante_en_vivo_reordena_la_lista(), enviar_optimista_confirmar_editar_reaccionar_borrar(), fallo_de_envio_y_reintento(), historial_paginado_carga_los_5000_sin_huecos() (+14 more)

### Community 79 - "tc-ui/src/audio.rs"
Cohesion: 0.06
Nodes (32): PlaybackState, Ended, Failed, Paused, Playing, Starting, SPEEDS, App (+24 more)

### Community 80 - ".download_media"
Cohesion: 0.15
Nodes (6): crate::Inner, DownloadError, Backend, FileReference, GrammersBackend, media_location()

### Community 81 - "CachedBackend<B>"
Cohesion: 0.08
Nodes (5): Dialog, DialogsPage, CachedBackend<B>, is_transient(), Shared<B>

### Community 82 - "Store"
Cohesion: 0.13
Nodes (12): favorites(), recents_order_dedupe_cap_and_cascade(), seed(), SET_COLS, set_from_row(), sets_and_stickers(), STICKER_COLS, sticker_from_row() (+4 more)

### Community 83 - "reading.rs"
Cohesion: 0.08
Nodes (24): Message, CONV_SCROLL_ID, error_text(), App, Carry, desplazamiento_deja_la_marca_arriba_con_contexto(), first_unread(), FLUSH_EVERY (+16 more)

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
Cohesion: 0.29
Nodes (22): card_screen(), CARD_W, code(), connecting(), divider(), field(), field_error(), ghost() (+14 more)

### Community 88 - "LoginMsg"
Cohesion: 0.12
Nodes (19): LoginMsg, CodeChanged, CodeSent, PasswordChanged, PasswordChecked, PhoneChanged, QrReady, Resend (+11 more)

### Community 89 - "tests/snapshot.rs"
Cohesion: 0.16
Nodes (14): StateKind, Discarded, Incoming, Ready, Requested, SeededEntropy, corrupt_or_foreign_snapshots_are_rejected(), dh() (+6 more)

### Community 90 - "tc-mock/src/lib.rs"
Cohesion: 0.05
Nodes (26): ANIMATED_SET_ID, animated_sticker_set(), BIG_CHAT_LEN, build(), CHANNEL_POSTS, gen_channel(), gen_history(), GROUPS (+18 more)

### Community 91 - "main.rs"
Cohesion: 0.09
Nodes (17): asignaciones_comillas_y_comentarios(), el_entorno_manda_sobre_el_archivo_salvo_si_esta_vacio(), EnvFile, lookup(), unquote(), account_id(), env_flag(), env_u64() (+9 more)

### Community 92 - "tests_audio.rs"
Cohesion: 0.19
Nodes (17): AudioBytes, TrackState, Failed, Idle, Loading, Paused, Playing, boot() (+9 more)

### Community 94 - "super"
Cohesion: 0.08
Nodes (16): Store, unread_mark_and_clear_history(), Error, Invalid, Io, NoDataDir, NotFound, SchemaTooNew (+8 more)

### Community 95 - "cached_tests.rs"
Cohesion: 0.13
Nodes (24): all(), cache_persists_across_restarts_on_disk(), cached(), cold_cache_offline_is_an_error_and_pagination_works(), dialogs_roundtrip_exactly_and_survive_offline(), downloads_are_cached_on_disk_but_never_for_secret_chats(), edits_deletes_reactions_read_and_failures_write_through(), events_are_persisted_and_searchable_locally() (+16 more)

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
Cohesion: 0.16
Nodes (12): backend(), decode(), Rd, Rd<'a>, VaultSessionStore, file_store_is_private_and_atomic(), FileSessionStore, SessionStore (+4 more)

### Community 106 - "tests/secret.rs"
Cohesion: 0.10
Nodes (22): UpdateBatch, ScriptSource, ANA, BETO, ChanSource, CHAT, fingerprint(), handler() (+14 more)

### Community 107 - "StickerError"
Cohesion: 0.10
Nodes (17): render_batch(), Tgs, StickerError, BadSize, FrameOutOfRange, Gzip, Image, LottieDisabled (+9 more)

### Community 110 - "AuthState"
Cohesion: 0.11
Nodes (16): AuthApi, AuthState, Checking, LoggedIn, LoggedOut, SignUpRequired, WaitingCode, WaitingPassword (+8 more)

### Community 112 - "folders.rs"
Cohesion: 0.23
Nodes (6): folder(), folder_peers_replace_and_cascade(), folders_crud_order_and_reorder(), Store, FolderPeerRow, FolderRow

### Community 113 - "files.rs"
Cohesion: 0.17
Nodes (7): cache_index_orders_by_last_use(), CacheEntry, media_refs_follow_their_media(), MediaRefRow, peer_photos_roundtrip_and_never_for_secrets(), photo_kind(), Store

### Community 115 - "sticker_anim.rs"
Cohesion: 0.11
Nodes (20): Playback, carga_una_vez_y_reintenta_tras_fallo(), DATA_CACHE_BYTES, FRAME_CACHE_BYTES, FULL, Job, Load, Render (+12 more)

### Community 116 - "tl_visual_ttl.rs"
Cohesion: 0.11
Nodes (8): TypingAction, Cancel, Typing, SelfDestructTimer, layer(), tl_known_encoding(), tl_rejects_unknown_and_trailing(), tl_round_trip_all_supported_constructors()

### Community 117 - "Message"
Cohesion: 0.08
Nodes (19): ForumApi, ThreadRef, MsgId, RandomId, Message, OutgoingMedia, OutgoingText, SendState (+11 more)

### Community 118 - "GifAnims"
Cohesion: 0.19
Nodes (7): GifAnims, GifFrames, la_cache_de_animaciones_respeta_el_tope(), Slot, Failed, Loading, Ready

### Community 119 - "jobs"
Cohesion: 0.18
Nodes (10): env, CARGO_INCREMENTAL, CARGO_TERM_COLOR, RUST_TOOLCHAIN, jobs, linux, name, needs (+2 more)

### Community 120 - "reactions.rs"
Cohesion: 0.29
Nodes (7): message_exists(), my_reaction_moves_between_emojis(), r(), set_get_replace_and_unknown_message(), setup(), Store, ReactionRow

### Community 121 - "KeyVisualization"
Cohesion: 0.13
Nodes (4): EMOJI, GRID_SIDE, KeyVisualization, VISUAL_LEN

### Community 123 - "Marca propia — Parlo"
Cohesion: 0.07
Nodes (24): ADR-0006: Empaquetado multiplataforma (Windows, macOS, Linux), Consecuencias, Decisiones, 1. Criterios de nombre, 2. Candidatos (6-8), 3. Chequeos que debe hacer el usuario, 4. Voz y tono (español), 5. Brief de logo (+16 more)

### Community 124 - "Category"
Cohesion: 0.14
Nodes (12): Category, Activities, .ALL, Animals, Flags, Food, Objects, People (+4 more)

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
Cohesion: 0.13
Nodes (11): apply_mine(), reaction_error_text(), account_stickers_load_in_the_picker_and_in_bubbles(), boot(), decoded(), my_reaction_moves_like_the_server_does(), net_sticker(), NOW (+3 more)

### Community 131 - "topics.rs"
Cohesion: 0.14
Nodes (28): topic_initial(), plain_letters(), preview(), ellipsize_to(), text_width(), with_weight(), in_column(), badge() (+20 more)

### Community 133 - "peers.rs"
Cohesion: 0.10
Nodes (30): channel_group(), chat_group(), chat_photo(), group_from_tl(), user_from_tl(), user_photo(), affected_history(), affected_messages() (+22 more)

### Community 134 - "AudioError"
Cohesion: 0.10
Nodes (17): AudioStream, open(), OpusStream, AudioError, Corrupt, NoDevice, Unsupported, corrupt() (+9 more)

### Community 135 - "ImageKey"
Cohesion: 0.12
Nodes (12): cache_tracks_loads_failures_and_evicts_least_recently_used(), ImageKey, Avatar, Full, Sticker, Thumb, Images, img() (+4 more)

### Community 136 - "Track"
Cohesion: 0.07
Nodes (7): FakeOutput, guarded(), NullOutput, Output, RodioOutput, run(), Track

### Community 139 - "stores.rs"
Cohesion: 0.11
Nodes (27): aad(), AAD_PREFIX, dh(), disk_bytes(), encode(), existing_session_json_is_migrated_once_and_deleted(), FIXED_LEN, media_cache() (+19 more)

### Community 141 - "windows"
Cohesion: 0.29
Nodes (7): WINDOWS_CERT_PASSWORD, WINDOWS_CERT_PFX_BASE64, windows, env, name, needs, runs-on

### Community 142 - ".sticker_send_task"
Cohesion: 0.12
Nodes (6): App, LOCAL_SEND_PX, send_local_pack_sticker(), ImageRequest, Avatar, Sticker

### Community 143 - "AnimMsg"
Cohesion: 0.15
Nodes (4): ActiveTgs, App, off_thread(), plays()

### Community 144 - "emoji.rs"
Cohesion: 0.08
Nodes (27): apply(), busqueda_en_espanol_ingles_y_por_grupo(), catalog_len(), display_name(), el_catalogo_es_completo_y_por_categorias(), es_index(), FREQUENT_SHOWN, list() (+19 more)

### Community 145 - "meta"
Cohesion: 0.40
Nodes (5): meta, name, outputs, runs-on, version

### Community 147 - "model.rs"
Cohesion: 0.05
Nodes (46): DEFAULT_REACTIONS, EntityKind, Bold, Code, Hashtag, Italic, Mention, Pre (+38 more)

### Community 149 - "tc-telegram/src/media.rs"
Cohesion: 0.16
Nodes (13): BIG_FILE, is_file_reference_error(), MAX_DOWNLOADS, MAX_FILE, MAX_IN_MEMORY, PART_SIZE, parts_for(), audio_attr() (+5 more)

### Community 150 - "tc-store/src/convert.rs"
Cohesion: 0.07
Nodes (47): voice_notes_and_music_survive_the_cache(), apply_dialogs(), apply_history(), cached_dialogs(), cached_history(), decode_entities(), decode_sticker(), decode_sticker_file() (+39 more)

### Community 151 - "tc-stickers/src/anim.rs"
Cohesion: 0.26
Nodes (11): ANIM_PX, frame_at(), frame_step(), info_of(), MAX_FPS, next_frames(), Show, Frame (+3 more)

### Community 152 - "tc-telegram/src/reactions.rs"
Cohesion: 0.18
Nodes (13): av(), Available, chat_allowed(), CHAT_TTL, ChatAllowed, All, None, Some (+5 more)

### Community 153 - "Timestamp"
Cohesion: 0.10
Nodes (22): el_tema_como_dialogo_conserva_lectura_y_no_leidos(), fijados_primero_y_luego_por_actividad(), ForumTopic, GENERAL_TOPIC, msg(), sort_topics(), TOPIC_COLORS, TopicIcon (+14 more)

### Community 155 - "tc-store/src/forum.rs"
Cohesion: 0.12
Nodes (8): COLS, from_row(), Store, temas_marca_de_foro_y_mensajes_por_tema(), TopicRow, group_roundtrip_update_and_lookup(), Store, GroupRow

### Community 156 - "tests/mod.rs"
Cohesion: 0.12
Nodes (7): Fake, Handler, logged_session(), logged_session_as(), ME_ID, ok(), Session

### Community 161 - "view/mod.rs"
Cohesion: 0.06
Nodes (32): handle(), HANDLES, icon(), icon_fill(), icons!, view(), PANEL_H, CELL (+24 more)

### Community 162 - "Gif"
Cohesion: 0.30
Nodes (5): Gif, GIF_MIME, InlineResult, GifPanel, Search

### Community 164 - "apply_media_extra"
Cohesion: 0.23
Nodes (4): apply_media_extra(), MediaExtraRow, Store, voice()

### Community 165 - "App"
Cohesion: 0.17
Nodes (3): AnimTarget, App, Gif

### Community 166 - "waveform.rs"
Cohesion: 0.39
Nodes (8): decode(), decodes_known_bit_layout(), encode(), from_samples(), MAX, resample(), resample_and_from_samples(), roundtrip_any_length()

### Community 167 - "MotionPreference"
Cohesion: 0.22
Nodes (7): MotionPreference, Full, Reduced, Property, Opacity, Position, Scale

### Community 169 - "App"
Cohesion: 0.08
Nodes (5): BubbleOpacity, BubbleY, App, Reply, sort_dialogs()

### Community 170 - "Step"
Cohesion: 0.33
Nodes (6): Step, Checking, Code, Password, Phone, SignUp

### Community 171 - "tests_sticker_anim.rs"
Cohesion: 0.19
Nodes (12): ThemeKind, Dark, Light, PAGE, app_with_tgs_bubble(), el_fotograma_animado_llega_a_la_burbuja_y_se_congela_sin_foco(), el_reloj_solo_corre_con_un_tgs_visible_con_foco_y_sin_movimiento_reducido(), en_el_selector_se_anima_el_sticker_bajo_el_raton() (+4 more)

### Community 172 - ".start"
Cohesion: 0.40
Nodes (5): Animator<K>, Curve, interrumpible_hereda_posicion(), movimiento_reducido_salta_posicion_y_limita_opacidad(), muelle_converge_y_termina()

### Community 175 - "CodeKind"
Cohesion: 0.22
Nodes (9): CodeKind, App, Call, Email, FlashCall, MissedCall, Other, Sms (+1 more)

### Community 179 - ".remember_gifs"
Cohesion: 0.33
Nodes (4): GIF_BOT, gif_from_document(), GifCatalog, input_doc()

### Community 183 - "hashmap"
Cohesion: 0.25
Nodes (5): CachedSet, crate::Inner, MAX_SETS, set_of(), StickerCatalog

### Community 185 - "Nav"
Cohesion: 0.15
Nodes (10): la_rejilla_se_recorre_con_flechas_sin_salirse(), Nav, Activate, Down, Left, NextTab, PrevTab, Right (+2 more)

### Community 186 - "MuteFor"
Cohesion: 0.20
Nodes (6): MuteFor, .ALL, EightHours, Forever, Hour, TwoDays

### Community 191 - "tests_chat_actions.rs"
Cohesion: 0.24
Nodes (15): action_error_text(), message_link(), a_user(), archive_is_optimistic_reverts_on_error_and_can_be_undone(), boot(), chat_menu_items_depend_on_the_kind_of_chat(), chat_msg(), delete_key_asks_then_removes_and_restores_on_error() (+7 more)

### Community 197 - "PeerRef"
Cohesion: 0.13
Nodes (3): Inner, reply_to(), flatten_updates()

### Community 202 - "tc-mock/src/gifs.rs"
Cohesion: 0.14
Nodes (16): by_id(), download(), encode_gif(), encode_png(), frame(), FRAMES, GIF_BASE, guardados_busqueda_y_bytes() (+8 more)

### Community 203 - ".get"
Cohesion: 0.17
Nodes (10): ChunkedVault<V>, construction_and_validation_do_not_touch_the_system(), KeyringVault, live_roundtrip(), map_err(), VaultError, Backend, InvalidKey (+2 more)

### Community 205 - "PanelMsg"
Cohesion: 0.07
Nodes (37): GifMsg, AnimLoaded, More, Query, Results, Retry, Saved, Search (+29 more)

### Community 206 - "tests_forum.rs"
Cohesion: 0.20
Nodes (12): BIG_CHAT, abrir_un_foro_muestra_la_lista_de_temas(), abrir_un_tema_en_su_primer_no_leido_y_volver(), boot(), busy(), crear_un_tema_lo_abre(), enviar_dentro_del_tema_y_eventos_de_otros_temas(), marcar_leido_desde_el_menu_y_tema_cerrado() (+4 more)

### Community 207 - "ADR-0007: Audio (notas de voz y música)"
Cohesion: 0.50
Nodes (3): ADR-0007: Audio (notas de voz y música), Consecuencias, Decisiones

### Community 211 - "on"
Cohesion: 0.50
Nodes (4): on, push, workflow_dispatch, tags

### Community 215 - "env"
Cohesion: 0.67
Nodes (3): GH_TOKEN, VERSION, env

### Community 217 - "tests_reading.rs"
Cohesion: 0.27
Nodes (9): abre_en_el_primer_no_leido_con_contexto_y_sin_marcar_todo(), boot(), ids(), ir_al_final_carga_los_ultimos_y_lo_nuevo_espera_fuera_de_la_ventana(), lectura_progresiva_con_espera(), NOW, open(), pagina_hacia_lo_nuevo_sin_duplicados_y_llega_al_final() (+1 more)

### Community 220 - "AnimMsg"
Cohesion: 0.25
Nodes (8): AnimMsg, Focus, Frames, Hover, Loaded, Tick, Unhover, focus_event()

### Community 222 - "FontRole"
Cohesion: 0.29
Nodes (7): family(), FontRole, Display, Emoji, Mono, Ui, ts()

### Community 223 - "anim_tests.rs"
Cohesion: 0.23
Nodes (10): fotograma_por_tiempo_a_30_fps_como_mucho(), gzip(), info(), movimiento_reducido_foco_y_visibilidad(), MOVING, ms(), siguientes_fotogramas_en_bucle(), sin_lottie_no_hay_animacion() (+2 more)

### Community 224 - "cached_actions_tests.rs"
Cohesion: 0.33
Nodes (8): a_user_chat(), all(), cached(), clear_and_delete_reach_the_cache(), offline_list(), removal_events_are_persisted(), unread_mark_read_mute_and_pin_persist(), CachedBackend

### Community 225 - "Folder"
Cohesion: 0.29
Nodes (6): Folder, All, Channels, Groups, Personal, Unread

### Community 236 - "tests_panel_shots.rs"
Cohesion: 0.22
Nodes (6): MockConfig, App, capturas_del_panel(), colorful(), settings(), shot()

### Community 237 - "context_menu.rs"
Cohesion: 0.08
Nodes (30): act(), archive_row(), buttons(), chat(), chat_items(), confirm_panel(), CursorProbe, danger() (+22 more)

### Community 241 - "tc-mock/src/audio.rs"
Cohesion: 0.46
Nodes (7): BARS, deterministic_and_well_formed(), mix(), RATE, voice_wav(), wav_len(), waveform()

### Community 243 - "ADR-0008: Stickers animados (TGS con rlottie; WebM estático)"
Cohesion: 0.33
Nodes (5): ADR-0008: Stickers animados (TGS con rlottie; WebM estático), Consecuencias, Decisión, TGS (Lottie con gzip): animado con rlottie, por defecto solo en Linux, WebM (vídeo VP9 con alfa): queda estático (miniatura)

### Community 246 - "Elevation"
Cohesion: 0.67
Nodes (3): Elevation, ELEVATION, ShadowSpec

### Community 247 - "ADR-0009: Teclado de emojis"
Cohesion: 0.50
Nodes (3): ADR-0009: Teclado de emojis, Consecuencias, Decisiones

### Community 248 - "ADR-0010: GIFs (guardados, búsqueda, envío y animación)"
Cohesion: 0.50
Nodes (3): ADR-0010: GIFs (guardados, búsqueda, envío y animación), Consecuencias, Decisiones

## Knowledge Gaps
- **524 isolated node(s):** `Newer`, `Latest`, `Flush`, `Backend`, `Paused` (+519 more)
  These have ≤1 connection - possible missing edges or undocumented components. (Counts symbols only; 1739 node(s) total have ≤1 connection when file, concept and rationale nodes are included.)
- **118 thin communities (<3 nodes) omitted from report** — run `graphify query` to explore isolated nodes.

## Suggested Questions
_Questions this graph is uniquely positioned to answer:_

- **Why does `Msg` connect `Msg` to `style.rs`, `tests_reactions.rs`, `topics.rs`, `tc-telegram/src/secret.rs`, `ImageKey`, `tc-ui/src/media.rs`, `ChatMsg`, `.sticker_send_task`, `AnimMsg`, `sidebar.rs`, `app.rs`, `ForumMsg`, `tests/mod.rs`, `view/mod.rs`, `App`, `tests_m2.rs`, `App`, `tests_sticker_anim.rs`, `src/login.rs`, `.on_reacted`, `widgets.rs`, `Nav`, `tests_chat_actions.rs`, `conversation.rs`, `view/audio.rs`, `PanelMsg`, `Backend`, `tc-ui/src/audio.rs`, `tests_forum.rs`, `reading.rs`, `view/login.rs`, `LoginMsg`, `tests_reading.rs`, `main.rs`, `AnimMsg`, `tests_audio.rs`, `Folder`, `tests_panel_shots.rs`, `context_menu.rs`, `sticker_anim.rs`, `.apply_auth`?**
  _High betweenness centrality (0.119) - this node is a cross-community bridge._
- **Are the 30 inferred relationships involving `Msg` (e.g. with `ChatMsg` and `Message`) actually correct?**
  _`Msg` has 30 INFERRED edges - model-reasoned connections that need verification._
- **What connects `Newer`, `Latest`, `Flush` to the rest of the system?**
  _524 weakly-connected nodes found - possible documentation gaps or missing edges._
- **Should `MockBackend` be split into smaller, more focused modules?**
  _Cohesion score 0.0647342995169082 - nodes in this community are weakly interconnected._
- **Why does `PeerRef` connect `BackendResult` to `view/mod.rs`, `conversation.rs`, `peers.rs`, `cached.rs`, `ImageKey`, `view/audio.rs`, `conv.rs`, `tc-ui/src/media.rs`, `context_menu.rs`, `.sticker_send_task`, `CachedBackend<B>`, `model.rs`, `cached.rs`, `Message`, `tc-store/src/convert.rs`, `Timestamp`, `tc_core`?**
  _High betweenness centrality (0.055) - this node is a cross-community bridge._
- **Are the 5 inferred relationships involving `App` (e.g. with `ChatActions` and `ForumUi`) actually correct?**
  _`App` has 5 INFERRED edges - model-reasoned connections that need verification._
- **Should `tokens.rs` be split into smaller, more focused modules?**
  _Cohesion score 0.029850746268656716 - nodes in this community are weakly interconnected._