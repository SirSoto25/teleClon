# Graph Report - teleClon  (2026-11-06)

## Corpus Check
- 61 files · ~51,474 words
- Verdict: corpus is large enough that graph structure adds value.
- Unclassified: 4 file(s) not represented in the graph (top: (none) 2, .example 1, .csv 1)

## Summary
- 1161 nodes · 2839 edges · 58 communities (53 shown, 5 thin omitted)
- Extraction: 93% EXTRACTED · 7% INFERRED · 0% AMBIGUOUS · INFERRED: 204 edges (avg confidence: 0.71)
- Token cost: 0 input · 0 output

## Graph Freshness
- Built from commit: `3dabfe64`
- Run `git rev-parse HEAD` and compare to check if the graph is stale.
- Run `graphify update .` after code changes (no API cost).

## Community Hubs (Navigation)
- conversation.rs
- MockBackend
- tokens.rs
- tc-mock/src/lib.rs
- Msg
- tc-theme/src/tests.rs
- BackendEvent
- App
- Sistema de diseño de teleClon — dirección «Brasa»
- tc-mock/src/tests.rs
- conv.rs
- model.rs
- Rgb
- Message
- app.rs
- format.rs
- BackendResult
- .update
- fonts.rs
- api.rs
- anim.rs
- measure.rs
- palette.py
- main.rs
- MotionPreference
- 00-INDEX.md
- Arquitectura de teleClon (cliente propio de la red de Telegram)
- Folder
- icon
- SecretChatsApi
- tc-ui/src/tests.rs
- EntityKind
- .start
- TextStyle
- .on_key
- MsgLayout
- Recon map: Telegram Desktop (Linux)
- .send
- layout_message
- Compose
- Measurer
- architecture.md
- tc-app
- Item
- Bitácora (append-only)
- Anchor
- Easing
- build_items
- ADR 0002 — Dirección visual «Brasa»
- ADR-0003: Arquitectura del cliente (workspace, backend tras trait, caché SQLite, E2E sans-IO)
- Registro de skills usadas
- SendState
- LoadState
- Skills de diseño poco conocidas (investigación 2026-11-06)
- visible_window

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
- `radios_de_union()` --calls--> `bubble_radius()`  [INFERRED]
  crates/tc-ui/src/conv.rs → crates/tc-ui/src/style.rs
- `BackendEvent` --references--> `MsgId`  [INFERRED]
  crates/tc-core/src/event.rs → crates/tc-core/src/ids.rs
- `BackendEvent` --references--> `PeerRef`  [INFERRED]
  crates/tc-core/src/event.rs → crates/tc-core/src/ids.rs
- `BackendEvent` --references--> `RandomId`  [INFERRED]
  crates/tc-core/src/event.rs → crates/tc-core/src/ids.rs
- `BackendEvent` --references--> `UserId`  [INFERRED]
  crates/tc-core/src/event.rs → crates/tc-core/src/ids.rs

## Import Cycles
- 1-file cycle: `crates/tc-theme/src/tokens.rs -> crates/tc-theme/src/tokens.rs`

## Communities (58 total, 5 thin omitted)

### Community 0 - "conversation.rs"
Cohesion: 0.05
Nodes (85): now(), ellipsize(), preview(), app_style(), avatar_ink(), avatar_radius(), badge(), bare_input() (+77 more)

### Community 1 - "MockBackend"
Cohesion: 0.09
Nodes (4): Inner, MockBackend, MockConfig, State

### Community 2 - "tokens.rs"
Cohesion: 0.03
Nodes (58): AVATAR_LG, AVATAR_MD, AVATAR_RATIO, AVATAR_SM, AVATAR_XL, AVATAR_XS, BREAKPOINT_SPLIT, BREAKPOINT_TRIPLE (+50 more)

### Community 3 - "tc-mock/src/lib.rs"
Cohesion: 0.06
Nodes (19): BIG_CHAT_LEN, build(), CHANNEL_POSTS, gen_channel(), gen_history(), GROUPS, incoming_text(), LONG_PHRASE (+11 more)

### Community 4 - "Msg"
Cohesion: 0.04
Nodes (47): Msg, AskDelete, Backend, CancelCompose, ClearSearch, CloseChat, CloseOverlay, ConfirmDelete (+39 more)

### Community 5 - "tc-theme/src/tests.rs"
Cohesion: 0.09
Nodes (24): AMPLITUDE, cell_segments(), ContourParams, .DEFAULT, .SECRET, contours(), field(), lattice() (+16 more)

### Community 6 - "BackendEvent"
Cohesion: 0.07
Nodes (29): BackendError, Crypto, FloodWait, Forbidden, Internal, InvalidInput, Network, NotFound (+21 more)

### Community 7 - "App"
Cohesion: 0.09
Nodes (3): App, empty(), view()

### Community 8 - "Sistema de diseño de teleClon — dirección «Brasa»"
Cohesion: 0.06
Nodes (31): 10. Preferencias de usuario persistidas, 11. Qué falta / siguiente fase, 1. Dirección estética, 2.1 Reglas de uso, 2.2 Paleta (oscuro y claro), 2.3 Transparencias (tokens `alpha`), 2.4 Contraste verificado (extracto), 2. Color (+23 more)

### Community 9 - "tc-mock/src/tests.rs"
Cohesion: 0.17
Nodes (25): about_thirty_chats_sorted_with_pinned_first(), data_is_deterministic(), delete_updates_last_message_and_emits(), edit_only_own_messages(), forward_creates_messages_with_origin(), history_after_and_around(), long_history_paging_covers_everything_without_gaps(), mark_read_clears_unread() (+17 more)

### Community 10 - "conv.rs"
Cohesion: 0.07
Nodes (27): DATE_H, EDGE_H, FWD_H, GAP_GROUP, GAP_IN_GROUP, GROUP_AVATAR, GROUP_AVATAR_COL, GROUP_WINDOW_SECS (+19 more)

### Community 11 - "model.rs"
Cohesion: 0.09
Nodes (24): GroupId, StickerId, StickerSetId, DialogsPage, Group, GroupKind, Basic, Channel (+16 more)

### Community 12 - "Rgb"
Cohesion: 0.13
Nodes (14): AA_NORMAL, AA_UI, Alpha, Alphas, Rgb, bubble_out_gradient(), ThemeKind, Dark (+6 more)

### Community 13 - "Message"
Cohesion: 0.19
Nodes (13): MsgId, RandomId, UserId, display_name_joins_parts(), Message, OutgoingMedia, Conversation, is_local() (+5 more)

### Community 14 - "app.rs"
Cohesion: 0.09
Nodes (8): backend_events(), BackendHandle, COMPOSER_ID, CONV_SCROLL_ID, LIST_SCROLL_ID, REACTIONS, SEARCH_ID, Toast

### Community 15 - "format.rs"
Cohesion: 0.21
Nodes (19): Timestamp, date_separator(), day_key(), fold(), hhmm(), horas_y_separadores_en_utc(), huella_determinista_y_en_rango(), iniciales_y_elipsis() (+11 more)

### Community 16 - "BackendResult"
Cohesion: 0.19
Nodes (9): DialogsApi, MessagesApi, BackendResult, PeerRef, Group, Secret, User, Dialog (+1 more)

### Community 18 - "fonts.rs"
Cohesion: 0.11
Nodes (18): BUNDLED, DISPLAY, DISPLAY_FAMILY, EMOJI_FAMILY, family(), MONO, MONO_FALLBACK, MONO_FAMILY (+10 more)

### Community 19 - "api.rs"
Cohesion: 0.15
Nodes (10): ChatBackend, MediaApi, ProfileApi, StickersApi, User, UserStatus, LastSeen, Offline (+2 more)

### Community 20 - "anim.rs"
Cohesion: 0.15
Nodes (11): Spring, .ESTANDAR, .RAPIDO, .REBOTE, .SUAVE, SpringState, Anim, Animator (+3 more)

### Community 21 - "measure.rs"
Cohesion: 0.17
Nodes (7): base(), Font_, IcedMeasurer, Paragraph, SHAPING, to_spans(), WRAP

### Community 22 - "palette.py"
Cohesion: 0.19
Nodes (8): build(), enc(), in_gamut(), lum(), oklch_to_linear(), pairs_for(), ratio(), to_hex()

### Community 23 - "main.rs"
Cohesion: 0.15
Nodes (5): demo_script(), env_flag(), env_u64(), main(), BIG_CHAT

### Community 24 - "MotionPreference"
Cohesion: 0.15
Nodes (11): HIGHLIGHT_FADE_MS, MotionPreference, Full, Reduced, Property, Opacity, Position, Scale (+3 more)

### Community 25 - "00-INDEX.md"
Cohesion: 0.17
Nodes (7): Dónde está cada cosa, Estado, Reglas de trabajo, teleClon — Índice maestro (leer primero tras perder contexto), Registro de agentes, graphify, teleClon

### Community 26 - "Arquitectura de teleClon (cliente propio de la red de Telegram)"
Cohesion: 0.12
Nodes (16): 0. Resumen, 10. Arranque y configuración (tc-app), 11. Módulos y operaciones por flujo, 12. Las partes que muerden, 13. Orden de construcción (rebanada vertical delgada primero), 14. Riesgos principales y siguiente paso, 1. Stack (versiones verificadas el 2026-11-06 con `cargo search`/`cargo info`, Rust 1.99.1), 2. Workspace (+8 more)

### Community 27 - "Folder"
Cohesion: 0.14
Nodes (13): DemoStep, Do, Exit, ScrollUp, Shot, Flags, Folder, All (+5 more)

### Community 28 - "icon"
Cohesion: 0.26
Nodes (6): handle(), HANDLES, icon(), icon_fill(), icons!, view()

### Community 29 - "SecretChatsApi"
Cohesion: 0.21
Nodes (8): SecretChatsApi, SecretChatId, KeyVisual, SecretChat, SecretChatState, Accepted, Discarded, Waiting

### Community 30 - "tc-ui/src/tests.rs"
Cohesion: 0.26
Nodes (12): Backend, PAGE, boot(), chat_secreto_huella_y_canal_de_solo_lectura(), entrante_en_vivo_reordena_la_lista(), enviar_optimista_confirmar_editar_reaccionar_borrar(), fallo_de_envio_y_reintento(), historial_paginado_carga_los_5000_sin_huecos() (+4 more)

### Community 31 - "EntityKind"
Cohesion: 0.15
Nodes (13): EntityKind, Bold, Code, Hashtag, Italic, Mention, Pre, Spoiler (+5 more)

### Community 32 - ".start"
Cohesion: 0.36
Nodes (5): Animator<K>, Curve, interrumpible_hereda_posicion(), movimiento_reducido_salta_posicion_y_limita_opacidad(), muelle_converge_y_termina()

### Community 33 - "TextStyle"
Cohesion: 0.18
Nodes (11): BODY, CAPTION, CODE, DISPLAY, EMOJI_ONLY, HEADING, META, TextStyle (+3 more)

### Community 35 - "MsgLayout"
Cohesion: 0.22
Nodes (6): Body, Bubble, Emoji, Sticker, LayoutCache, MsgLayout

### Community 36 - "Recon map: Telegram Desktop (Linux)"
Cohesion: 0.20
Nodes (10): Components, Core loop, Feature matrix, Flows, Inferred data model, Out of scope (cannot or should not be cloned), Recon map: Telegram Desktop (Linux), Screens (+2 more)

### Community 37 - ".send"
Cohesion: 0.25
Nodes (6): AnimKey, BubbleOpacity, BubbleY, ChatSwitch, Overlay, SendButton

### Community 38 - "layout_message"
Cohesion: 0.33
Nodes (8): emoji_only(), input(), layout_message(), LayoutInput, maquetacion_meta_en_linea_o_debajo(), meta_width(), runs(), tramos_por_entidades()

### Community 39 - "Compose"
Cohesion: 0.25
Nodes (6): Compose, Edit, Normal, Overlay, Delete, Key

### Community 40 - "Measurer"
Cohesion: 0.43
Nodes (3): Fixed, Measurer, Run

### Community 41 - "architecture.md"
Cohesion: 0.29
Nodes (3): ADR-0001: Stack técnico, Confirmación y correcciones (2026-11-06, agente arquitecto), ADR-0004: Stickers y chats secretos entran en el alcance

### Community 42 - "tc-app"
Cohesion: 0.39
Nodes (8): tc-app, tc-core, tc-mock, tc-secret, tc-stickers, tc-store, tc-theme, tc-ui

### Community 43 - "Item"
Cohesion: 0.29
Nodes (5): Item, Date, Edge, Msg, UnreadMark

### Community 44 - "Bitácora (append-only)"
Cohesion: 0.29
Nodes (7): 2026-11-06, 2026-11-06 — Arquitectura (agente ARQUITECTO, Opus/high), 2026-11-06 — Core + Mock (agente CORE, Sonnet/medium), 2026-11-06 — Recon (agente RECON), 2026-11-06 — Tema (agente THEME, Sonnet/medium), 2026-11-06 — UI (agente UI, Opus/high): M1.1 lista, M1.2 conversación, M1.3 compositor, Bitácora (append-only)

### Community 45 - "Anchor"
Cohesion: 0.40
Nodes (5): Anchor, After, Around, Before, Latest

### Community 46 - "Easing"
Cohesion: 0.40
Nodes (5): Easing, .ESTANDAR, .RAPIDO, .SALIDA, .SUAVE

### Community 47 - "build_items"
Cohesion: 0.40
Nodes (5): same_group(), signature(), bubble_max(), build_items(), meta_label()

### Community 48 - "ADR 0002 — Dirección visual «Brasa»"
Cohesion: 0.40
Nodes (5): ADR 0002 — Dirección visual «Brasa», Alternativas consideradas, Consecuencias, Contexto, Decisión

### Community 49 - "ADR-0003: Arquitectura del cliente (workspace, backend tras trait, caché SQLite, E2E sans-IO)"
Cohesion: 0.40
Nodes (5): ADR-0003: Arquitectura del cliente (workspace, backend tras trait, caché SQLite, E2E sans-IO), Alternativas descartadas, Consecuencias, Contexto, Decisión

### Community 50 - "Registro de skills usadas"
Cohesion: 0.40
Nodes (5): 2026-11-06 — agente CORE (Sonnet), 2026-11-06 — agente UI (Opus/high), Nota de transparencia, Registro de skills usadas, Skills previstas en fases siguientes

### Community 51 - "SendState"
Cohesion: 0.50
Nodes (4): SendState, Failed, Pending, Sent

### Community 52 - "LoadState"
Cohesion: 0.50
Nodes (4): LoadState, Failed, Loading, Ready

### Community 53 - "Skills de diseño poco conocidas (investigación 2026-11-06)"
Cohesion: 0.50
Nodes (4): Candidatas poco conocidas, Decisión de seguridad, Skills de diseño poco conocidas (investigación 2026-11-06), Skills ya disponibles localmente que sí se usan

### Community 54 - "visible_window"
Cohesion: 0.67
Nodes (3): ventana_virtual_abajo_y_arriba(), visible_window(), Window

## Knowledge Gaps
- **151 isolated node(s):** `Network`, `FloodWait`, `Unauthorized`, `NotFound`, `Forbidden` (+146 more)
  These have ≤1 connection - possible missing edges or undocumented components. (Counts symbols only; 460 node(s) total have ≤1 connection when file, concept and rationale nodes are included.)
- **5 thin communities (<3 nodes) omitted from report** — run `graphify query` to explore isolated nodes.

## Suggested Questions
_Questions this graph is uniquely positioned to answer:_

- **Why does `TextStyle` connect `TextStyle` to `conversation.rs`, `tokens.rs`, `Measurer`, `conv.rs`, `Rgb`, `fonts.rs`, `measure.rs`?**
  _High betweenness centrality (0.143) - this node is a cross-community bridge._
- **Are the 21 inferred relationships involving `c()` (e.g. with `body()` and `bubble()`) actually correct?**
  _`c()` has 21 INFERRED edges - model-reasoned connections that need verification._
- **What connects `Network`, `FloodWait`, `Unauthorized` to the rest of the system?**
  _151 weakly-connected nodes found - possible documentation gaps or missing edges._
- **Should `conversation.rs` be split into smaller, more focused modules?**
  _Cohesion score 0.0500515995872033 - nodes in this community are weakly interconnected._
- **Why does `Msg` connect `Msg` to `conversation.rs`, `.on_key`, `.send`, `App`, `Compose`, `app.rs`, `.update`, `main.rs`, `Folder`, `tc-ui/src/tests.rs`?**
  _High betweenness centrality (0.124) - this node is a cross-community bridge._
- **Are the 11 inferred relationships involving `Message` (e.g. with `.send_media()` and `.edit_message()`) actually correct?**
  _`Message` has 11 INFERRED edges - model-reasoned connections that need verification._
- **Should `MockBackend` be split into smaller, more focused modules?**
  _Cohesion score 0.08874522480164561 - nodes in this community are weakly interconnected._