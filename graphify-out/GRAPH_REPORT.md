# Graph Report - teleClon  (2026-11-06)

## Corpus Check
- 63 files · ~51,773 words
- Verdict: corpus is large enough that graph structure adds value.
- Unclassified: 4 file(s) not represented in the graph (top: (none) 2, .example 1, .csv 1)

## Summary
- 1183 nodes · 2926 edges · 64 communities (53 shown, 11 thin omitted)
- Extraction: 92% EXTRACTED · 8% INFERRED · 0% AMBIGUOUS · INFERRED: 246 edges (avg confidence: 0.73)
- Token cost: 0 input · 0 output

## Community Hubs (Navigation)
- Community 0
- Community 1
- Community 2
- Community 3
- Community 4
- Community 5
- Community 6
- Community 7
- Community 8
- Community 9
- Community 10
- Community 11
- Community 12
- Community 13
- Community 14
- Community 15
- Community 16
- Community 17
- Community 18
- Community 19
- Community 20
- Community 21
- Community 22
- Community 23
- Community 24
- Community 25
- Community 26
- Community 27
- Community 28
- Community 29
- Community 30
- Community 31
- Community 32
- Community 33
- Community 34
- Community 35
- Community 36
- Community 37
- Community 38
- Community 39
- Community 40
- Community 41
- Community 42
- Community 43
- Community 44
- Community 45
- Community 46
- Community 47
- Community 48
- Community 49
- Community 50
- Community 51
- Community 53
- Community 54
- Community 55
- Community 56
- Community 57
- Community 58
- Community 62
- Community 63

## God Nodes (most connected - your core abstractions)
1. `Msg` - 122 edges
2. `App` - 95 edges
3. `c()` - 52 edges
4. `MockBackend` - 48 edges
5. `Message` - 44 edges
6. `Brasa` - 41 edges
7. `TextStyle` - 31 edges
8. `BackendResult` - 29 edges
9. `MsgId` - 29 edges
10. `PeerRef` - 29 edges

## Surprising Connections (you probably didn't know these)
- `Clean-room (sin código, assets ni marca de Telegram)` --semantically_similar_to--> `Reglas de trabajo (log, ADR, clean-room, credenciales, regenerar grafo)`  [INFERRED] [semantically similar]
  README.md → docs/00-INDEX.md
- `Dirección Brasa (grafito cálido + acento persimón)` --rationale_for--> `Clean-room (sin código, assets ni marca de Telegram)`  [INFERRED]
  docs/adr/0002-direccion-visual.md → README.md
- `radios_de_union()` --calls--> `bubble_radius()`  [INFERRED]
  crates/tc-ui/src/conv.rs → crates/tc-ui/src/style.rs
- `Índice maestro de teleClon` --references--> `Recon map: Telegram Desktop (Linux)`  [EXTRACTED]
  docs/00-INDEX.md → replica/recon.md
- `Agente recon (sonnet/medium)` --references--> `Recon map: Telegram Desktop (Linux)`  [EXTRACTED]
  docs/agents/registry.md → replica/recon.md

## Import Cycles
- 1-file cycle: `crates/tc-theme/src/tokens.rs -> crates/tc-theme/src/tokens.rs`

## Hyperedges (group relationships)
- **Implementaciones y decoradores de ChatBackend** — docs_architecture_mockbackend, docs_architecture_cachedbackend, docs_architecture_tc_telegram, docs_architecture_chatbackend [EXTRACTED 0.95]
- **Fundamentos visuales de Brasa** — docs_design_system_paleta_oklch, docs_design_system_tipografia, docs_design_system_espaciado_radios, docs_design_system_elevacion, docs_design_system_movimiento [EXTRACTED 0.95]
- **Decisiones de arquitectura ADR-0001 a ADR-0004** — docs_adr_0001_stack, docs_adr_0002_direccion_visual, docs_adr_0003_arquitectura, docs_adr_0004_stickers_y_chats_secretos [EXTRACTED 1.00]
- **Pila de chats secretos (cripto, claves y UI)** — docs_architecture_tc_secret, docs_architecture_keyring_boveda, docs_design_system_chat_secreto_ui [INFERRED 0.75]
- **Implementaciones de ChatBackend (Mock, Cached, Telegram)** — docs_adr_0003_arquitectura_chatbackend, docs_adr_0003_arquitectura_mockbackend, docs_adr_0003_arquitectura_cachedbackend, docs_adr_0001_stack_grammers [INFERRED 0.85]
- **Pila de chats secretos E2E** — docs_adr_0004_stickers_y_chats_secretos_secret_chats, docs_adr_0003_arquitectura_tc_secret, docs_adr_0001_stack_keyring, docs_adr_0004_stickers_y_chats_secretos_crypto_risk [INFERRED 0.85]

## Communities (64 total, 11 thin omitted)

### Community 0 - "Community 0"
Cohesion: 0.05
Nodes (85): now(), ellipsize(), preview(), app_style(), avatar_ink(), avatar_radius(), badge(), bare_input() (+77 more)

### Community 1 - "Community 1"
Cohesion: 0.09
Nodes (4): Inner, MockBackend, MockConfig, State

### Community 2 - "Community 2"
Cohesion: 0.04
Nodes (55): AVATAR_LG, AVATAR_MD, AVATAR_RATIO, AVATAR_SM, AVATAR_XL, AVATAR_XS, BREAKPOINT_SPLIT, BREAKPOINT_TRIPLE (+47 more)

### Community 3 - "Community 3"
Cohesion: 0.04
Nodes (48): Msg, AskDelete, Backend, CancelCompose, ClearSearch, CloseChat, CloseOverlay, ConfirmDelete (+40 more)

### Community 4 - "Community 4"
Cohesion: 0.06
Nodes (38): Arquitectura de teleClon, ADR-0001 Stack, ADR-0002 Dirección visual, ADR-0003 Arquitectura, ADR-0004 Stickers y chats secretos, BackendEvent, ChatBackend (trait), Modelo de concurrencia (runtime tokio único, actores) (+30 more)

### Community 5 - "Community 5"
Cohesion: 0.09
Nodes (24): AMPLITUDE, cell_segments(), ContourParams, .DEFAULT, .SECRET, contours(), field(), lattice() (+16 more)

### Community 6 - "Community 6"
Cohesion: 0.07
Nodes (29): BackendError, Crypto, FloodWait, Forbidden, Internal, InvalidInput, Network, NotFound (+21 more)

### Community 7 - "Community 7"
Cohesion: 0.11
Nodes (4): App, sort_dialogs(), empty(), view()

### Community 8 - "Community 8"
Cohesion: 0.17
Nodes (25): about_thirty_chats_sorted_with_pinned_first(), data_is_deterministic(), delete_updates_last_message_and_emits(), edit_only_own_messages(), forward_creates_messages_with_origin(), history_after_and_around(), long_history_paging_covers_everything_without_gaps(), mark_read_clears_unread() (+17 more)

### Community 9 - "Community 9"
Cohesion: 0.07
Nodes (27): DATE_H, EDGE_H, FWD_H, GAP_GROUP, GAP_IN_GROUP, GROUP_AVATAR, GROUP_AVATAR_COL, GROUP_WINDOW_SECS (+19 more)

### Community 10 - "Community 10"
Cohesion: 0.17
Nodes (23): Timestamp, UserStatus, LastSeen, Offline, Online, Recently, date_separator(), day_key() (+15 more)

### Community 11 - "Community 11"
Cohesion: 0.13
Nodes (14): AA_NORMAL, AA_UI, Alpha, Alphas, Rgb, bubble_out_gradient(), ThemeKind, Dark (+6 more)

### Community 12 - "Community 12"
Cohesion: 0.20
Nodes (12): MsgId, RandomId, Dialog, Draft, Message, Conversation, is_local(), msg() (+4 more)

### Community 13 - "Community 13"
Cohesion: 0.09
Nodes (8): backend_events(), BackendHandle, COMPOSER_ID, CONV_SCROLL_ID, LIST_SCROLL_ID, REACTIONS, SEARCH_ID, Toast

### Community 14 - "Community 14"
Cohesion: 0.10
Nodes (22): Anchor, After, Around, Before, Latest, DialogsPage, Media, MediaKind (+14 more)

### Community 15 - "Community 15"
Cohesion: 0.20
Nodes (7): DialogsApi, MessagesApi, BackendResult, PeerRef, Group, Secret, User

### Community 17 - "Community 17"
Cohesion: 0.11
Nodes (18): BUNDLED, DISPLAY, DISPLAY_FAMILY, EMOJI_FAMILY, family(), MONO, MONO_FALLBACK, MONO_FAMILY (+10 more)

### Community 18 - "Community 18"
Cohesion: 0.17
Nodes (7): base(), Font_, IcedMeasurer, Paragraph, SHAPING, to_spans(), WRAP

### Community 19 - "Community 19"
Cohesion: 0.16
Nodes (15): ADR-0002: Dirección visual Brasa, Dirección Brasa (grafito cálido + acento persimón), Controlled para chrome, innovative para burbujas y fondo, Movimiento con muelles 119-221 ms y movimiento reducido, OKLCH como fuente de verdad y contraste WCAG AA (144 pares), docs/design/system.md y tokens.json, Tipografía libre (Atkinson Hyperlegible Next, Bricolage Grotesque, JetBrains Mono) e iconos Phosphor, Agente diseño (opus/high) (+7 more)

### Community 20 - "Community 20"
Cohesion: 0.19
Nodes (8): build(), enc(), in_gamut(), lum(), oklch_to_linear(), pairs_for(), ratio(), to_hex()

### Community 21 - "Community 21"
Cohesion: 0.18
Nodes (10): SecretChatsApi, SecretChatId, UserId, display_name_joins_parts(), KeyVisual, SecretChat, SecretChatState, Accepted (+2 more)

### Community 22 - "Community 22"
Cohesion: 0.13
Nodes (13): DemoStep, Do, Exit, ScrollUp, Shot, Flags, Folder, All (+5 more)

### Community 23 - "Community 23"
Cohesion: 0.16
Nodes (5): demo_script(), env_flag(), env_u64(), main(), BIG_CHAT

### Community 25 - "Community 25"
Cohesion: 0.23
Nodes (12): Backend, PAGE, key_grid(), boot(), chat_secreto_huella_y_canal_de_solo_lectura(), entrante_en_vivo_reordena_la_lista(), enviar_optimista_confirmar_editar_reaccionar_borrar(), fallo_de_envio_y_reintento() (+4 more)

### Community 26 - "Community 26"
Cohesion: 0.26
Nodes (6): handle(), HANDLES, icon(), icon_fill(), icons!, view()

### Community 27 - "Community 27"
Cohesion: 0.26
Nodes (5): build(), gen_channel(), gen_history(), Seed, sticker_set()

### Community 28 - "Community 28"
Cohesion: 0.19
Nodes (12): ADR-0001: Stack técnico, grammers (MTProto en Rust), iced (UI en Rust, wgpu + tiny-skia), keyring (claves y sesión), rusqlite bundled con migraciones por PRAGMA user_version, tc-secret: chats secretos sans-IO con RustCrypto, tc-stickers: WebP, TGS con rlottie, WebM aplazado, ADR-0004: Stickers y chats secretos en el alcance (+4 more)

### Community 29 - "Community 29"
Cohesion: 0.23
Nodes (5): ChatBackend, MediaApi, ProfileApi, StickersApi, User

### Community 30 - "Community 30"
Cohesion: 0.24
Nodes (12): ADR-0003: Arquitectura del cliente, Orden de construcción M1.1-M1.4, M2.1, M2.7, CachedBackend (caché como decorador, tc-store), Trait ChatBackend (sub-traits + events()), Modelo de concurrencia (tokio, hilo SQLite, canal acotado, Resync), MockBackend determinista, SQLite por cuenta (17 tablas + FTS5, WAL, chat_key), Scroll virtual propio (Fenwick, ventana ~600 mensajes) (+4 more)

### Community 31 - "Community 31"
Cohesion: 0.24
Nodes (13): Registro de agentes, Agente arquitecto (opus/high), Agente core+mock (sonnet/medium), Agente QA (pendiente), Agente theme (sonnet/medium), Agente UI (opus/high), Bitácora (append-only), Bug iced 0.14: scrollable.direction antes de anchor_bottom (+5 more)

### Community 32 - "Community 32"
Cohesion: 0.24
Nodes (4): incoming_text(), below_stays_in_range(), is_deterministic(), Lcg

### Community 33 - "Community 33"
Cohesion: 0.20
Nodes (8): MotionPreference, Full, Reduced, Property, Opacity, Position, Scale, Curve

### Community 34 - "Community 34"
Cohesion: 0.17
Nodes (10): Item, Date, Edge, Msg, UnreadMark, same_group(), signature(), bubble_max() (+2 more)

### Community 35 - "Community 35"
Cohesion: 0.21
Nodes (11): Índice maestro de teleClon, architecture.md (crates, ChatBackend, SQLite, F01-F18), graphify-out (graph.html, GRAPH_REPORT.md, graph.json), Reglas de trabajo (log, ADR, clean-room, credenciales, regenerar grafo), Credenciales api_id/api_hash en .env local gitignored, graphify: instalación y uso, graphifyy en ./.venv local, Registro de skills usadas (+3 more)

### Community 36 - "Community 36"
Cohesion: 0.18
Nodes (11): EntityKind, Bold, Code, Hashtag, Italic, Mention, Pre, Spoiler (+3 more)

### Community 37 - "Community 37"
Cohesion: 0.18
Nodes (9): BIG_CHAT_LEN, CHANNEL_POSTS, GROUPS, LONG_PHRASE, ME, PEOPLE, PHRASES, REACTIONS (+1 more)

### Community 38 - "Community 38"
Cohesion: 0.22
Nodes (9): Spring, .ESTANDAR, .RAPIDO, .REBOTE, .SUAVE, SpringState, Kind, Linear (+1 more)

### Community 39 - "Community 39"
Cohesion: 0.18
Nodes (11): BODY, CAPTION, CODE, DISPLAY, EMOJI_ONLY, HEADING, META, TextStyle (+3 more)

### Community 40 - "Community 40"
Cohesion: 0.40
Nodes (4): Animator<K>, interrumpible_hereda_posicion(), movimiento_reducido_salta_posicion_y_limita_opacidad(), muelle_converge_y_termina()

### Community 41 - "Community 41"
Cohesion: 0.24
Nodes (8): StickerId, StickerSetId, Sticker, StickerFormat, Lottie, Webm, Webp, StickerSet

### Community 42 - "Community 42"
Cohesion: 0.20
Nodes (9): Easing, .ESTANDAR, .RAPIDO, .SALIDA, .SUAVE, HIGHLIGHT_FADE_MS, REDUCED_OPACITY_MAX_MS, SKELETON_PULSE_PERIOD_MS (+1 more)

### Community 43 - "Community 43"
Cohesion: 0.22
Nodes (6): AnimKey, BubbleOpacity, BubbleY, ChatSwitch, Overlay, SendButton

### Community 44 - "Community 44"
Cohesion: 0.22
Nodes (6): Body, Bubble, Emoji, Sticker, LayoutCache, MsgLayout

### Community 45 - "Community 45"
Cohesion: 0.22
Nodes (7): Compose, Edit, Normal, Reply, Overlay, Delete, Key

### Community 46 - "Community 46"
Cohesion: 0.33
Nodes (8): emoji_only(), input(), layout_message(), LayoutInput, maquetacion_meta_en_linea_o_debajo(), meta_width(), runs(), tramos_por_entidades()

### Community 47 - "Community 47"
Cohesion: 0.31
Nodes (9): replica/features.csv, Rasgos de marca (avatares cuadrado redondeado, burbujas sin cola, fondo de isolíneas), Agente recon (sonnet/medium), Recon map: Telegram Desktop (Linux), Componentes de UI (Burbuja de mensaje, Fila de chat, Compositor...), Core loop (chats, enviar/recibir, sincronizado), Matriz de funciones (features.csv, must/should/could/skip), Flujos F01-F16 (+1 more)

### Community 48 - "Community 48"
Cohesion: 0.43
Nodes (3): Fixed, Measurer, Run

### Community 49 - "Community 49"
Cohesion: 0.39
Nodes (8): tc-app, tc-core, tc-mock, tc-secret, tc-stickers, tc-store, tc-theme, tc-ui

### Community 50 - "Community 50"
Cohesion: 0.33
Nodes (6): GroupId, Group, GroupKind, Basic, Channel, Super

### Community 53 - "Community 53"
Cohesion: 0.50
Nodes (4): LoadState, Failed, Loading, Ready

### Community 54 - "Community 54"
Cohesion: 0.50
Nodes (4): ventana_virtual_abajo_y_arriba(), visible_window(), Window, historial_paginado_carga_los_5000_sin_huecos()

### Community 55 - "Community 55"
Cohesion: 0.67
Nodes (3): Elevation, ELEVATION, ShadowSpec

### Community 56 - "Community 56"
Cohesion: 0.67
Nodes (3): Disposición (raíl, lista, conversación, panel info), Lista de chats (componente), Barra lateral (raíl de carpetas)

## Knowledge Gaps
- **176 isolated node(s):** `Network`, `FloodWait`, `Unauthorized`, `NotFound`, `Forbidden` (+171 more)
  These have ≤1 connection - possible missing edges or undocumented components. (Counts symbols only; 411 node(s) total have ≤1 connection when file, concept and rationale nodes are included.)
- **11 thin communities (<3 nodes) omitted from report** — run `graphify query` to explore isolated nodes.

## Suggested Questions
_Questions this graph is uniquely positioned to answer:_

- **Why does `TextStyle` connect `Community 39` to `Community 0`, `Community 2`, `Community 9`, `Community 11`, `Community 48`, `Community 17`, `Community 18`?**
  _High betweenness centrality (0.139) - this node is a cross-community bridge._
- **Are the 21 inferred relationships involving `c()` (e.g. with `body()` and `bubble()`) actually correct?**
  _`c()` has 21 INFERRED edges - model-reasoned connections that need verification._
- **What connects `Network`, `FloodWait`, `Unauthorized` to the rest of the system?**
  _176 weakly-connected nodes found - possible documentation gaps or missing edges._
- **Should `Community 0` be split into smaller, more focused modules?**
  _Cohesion score 0.0500515995872033 - nodes in this community are weakly interconnected._
- **Why does `Msg` connect `Community 3` to `Community 0`, `Community 7`, `Community 43`, `Community 13`, `Community 45`, `Community 16`, `Community 22`, `Community 23`, `Community 25`?**
  _High betweenness centrality (0.118) - this node is a cross-community bridge._
- **Are the 11 inferred relationships involving `Message` (e.g. with `.send_media()` and `.edit_message()`) actually correct?**
  _`Message` has 11 INFERRED edges - model-reasoned connections that need verification._
- **Should `Community 1` be split into smaller, more focused modules?**
  _Cohesion score 0.08691910499139414 - nodes in this community are weakly interconnected._