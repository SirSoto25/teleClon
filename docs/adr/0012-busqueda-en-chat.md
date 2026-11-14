# ADR-0012: Búsqueda dentro de un chat

- **Fecha:** 2026-11-14 · **Estado:** aceptado (fase B1 de `docs/plans/archivos-busqueda-notificaciones.md`:
  contrato en `tc-core`, FTS5 en `tc-store`, `messages.search` en `tc-telegram`, simulación en `tc-mock`).
  La UI (B2) viene después. Probado solo con el mock y el transporte falso; sin Telegram real.
- **Contexto:** la UI necesita Ctrl+F con contador, saltos y filtros por tipo (S23). El almacén local ya
  tenía FTS5 (acentos, mayúsculas y prefijo probados) pero solo expuesto como `search_local`, sin filtros, sin
  temas y sin paginación; y `search_in_chat` estaba solo en la tabla de `architecture.md`.

## Decisiones
1. **Contrato** (`tc-core/src/search.rs`; método con implementación por defecto «no soportado», como los GIF):
   `MessagesApi::search_in_chat(peer, SearchQuery) -> BackendResult<SearchPage>`.
   - `SearchQuery { text, filter: SearchFilter, offset: Option<MsgId>, limit }`; `SearchFilter = All | Photos |
     Videos | Files | Links | Voice`. Texto vacío solo vale con un filtro distinto de `All` (lista todas las
     fotos, etc.); `All` sin texto devuelve la página vacía sin preguntar a nadie (`SearchQuery::is_empty`).
   - `SearchPage { hits: Vec<Message>, total: Option<u32>, next: Option<MsgId> }`.
   - Orden **más reciente primero**. `offset` = «solo ids menores que»; `next` es el `offset` de la página
     siguiente (`None` = no hay más). La UI recorre con `query.after(next)`. Elegido porque `messages.search`
     pagina así (`offset_id`) y porque un id no se repite ni se desplaza al borrar (un número de página sí).
   - `SearchFilter::accepts(&Message)` y `text_matches(query, text)` (acentos, mayúsculas, AND de palabras,
     prefijo en la última) dan la definición común que usa el mock y que el SQL de `tc-store` reproduce.
   - Videos excluye los vídeos redondos; Voice son solo notas de voz (no música); Links = entidad URL o
     `http://`, `https://`, `www.` en el texto; Files = documentos (no audio ni voz).
2. **Temas de foro:** `ForumApi::search_in_topic(ThreadRef, SearchQuery)` (por defecto no soportado) y
   `ThreadScope::search_in_chat` lo usa cuando el peer es el grupo del tema; así la UI de la conversación
   dentro de un tema reutiliza la misma barra. En red es `messages.search` con `top_msg_id` = id del tema.
3. **tc-store:** `Store::search_chat(chat_key, topic, text, filter, before, limit) -> (filas, total)` sobre
   `messages_fts` unido a `media` (tipo) y `message_topics` (tema); solo mensajes confirmados (`msg_id > 0`);
   un mensaje borrado sale del índice por sus triggers y deja de aparecer. Sin migración nueva.
4. **tc-telegram:** `messages.search` con `InputMessagesFilter{Empty,Photos,Video,Document,Url,Voice}`,
   `offset_id`, `limit` (1..=100), `top_msg_id`; ingesta usuarios y chats de la respuesta y convierte con la
   misma caché que el historial. `total` = `count` del servidor; `next` = id más bajo si la página vino llena.
   Chats secretos: no soportado (el servidor no los conoce; ya no se indexan en local).
5. **CachedBackend:** secretos → backend. Si el diálogo tiene `history_complete`, responde solo con lo local
   (instantáneo, sin red). Si no, pregunta al servidor y fusiona sin duplicar por id: dentro del tramo que
   cubre la página del servidor se añaden los locales que él no devolvió; `total` = máximo entre el del
   servidor y lo fusionado. Red caída, flood o «no soportado» → página local. **Los resultados del servidor no
   se guardan** en la caché de historial: dejarían mensajes sueltos que `history` (`cached.len() >= limit`) podría
   tomar por un tramo contiguo. En temas siempre se pregunta primero al servidor (el almacén solo conoce los
   mensajes de tema que ya pasaron por `topic_history`/envíos); sin red, lo guardado con su tema.
6. **tc-mock:** busca sobre sus mensajes con las mismas reglas (incluye temas por `msg_topic`), respeta
   `set_offline` y borrados.

## Consecuencias
- Para la UI (B2): `search_in_chat(peer, query)`; el contador «3 de 17» sale de `total` y de la posición en los
  `hits` ya acumulados; ↑/↓ avanzan por los resultados y piden la página siguiente con `next` al llegar al final.
  Saltar a un resultado no cargado reutiliza `Anchor::Around(id)`.
- Un resultado puede desaparecer mientras se navega (borrado): la UI debe tolerarlo y reordenar con
  `MessagesDeleted`.
- El total del servidor es aproximado en canales grandes; el fusionado puede superarlo.
- Pendiente: probar contra Telegram real (filtros de vídeo/URL y `top_msg_id` en el General), búsqueda global
  entre chats (F11) y por remitente.
