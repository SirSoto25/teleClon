//! Búsqueda dentro de un chat o tema (ADR-0012) con `messages.search`.
//!
//! - Filtros: `InputMessagesFilter{Empty,Photos,Video,Document,Url,Voice}` según [`SearchFilter`].
//! - Tema de foro: `top_msg_id` = id del tema (su mensaje superior).
//! - Paginación: `offset_id` = «mensajes con id menor»; sale de nuevo a viejo, como pide el contrato.
//! - Los usuarios y chats de la respuesta se ingieren y los mensajes se convierten con la misma
//!   caché que el historial. Los chats secretos no se buscan en el servidor (no soportado).
//!
//! Sin probar contra los servidores reales (solo con el transporte falso de las pruebas).
use async_trait::async_trait;
use grammers_tl_types as tl;
use tc_core::*;

use crate::GrammersBackend;
use crate::backend::MessagesApiSearch;

pub(crate) fn filter_to_tl(f: SearchFilter) -> tl::enums::MessagesFilter {
    match f {
        SearchFilter::All => tl::types::InputMessagesFilterEmpty {}.into(),
        SearchFilter::Photos => tl::types::InputMessagesFilterPhotos {}.into(),
        SearchFilter::Videos => tl::types::InputMessagesFilterVideo {}.into(),
        SearchFilter::Files => tl::types::InputMessagesFilterDocument {}.into(),
        SearchFilter::Links => tl::types::InputMessagesFilterUrl {}.into(),
        SearchFilter::Voice => tl::types::InputMessagesFilterVoice {}.into(),
    }
}

/// Total que declara el servidor (`count` de las páginas parciales).
fn total_of(r: &tl::enums::messages::Messages) -> Option<u32> {
    let n = match r {
        tl::enums::messages::Messages::Messages(m) => m.messages.len() as i64,
        tl::enums::messages::Messages::Slice(m) => i64::from(m.count),
        tl::enums::messages::Messages::ChannelMessages(m) => i64::from(m.count),
        tl::enums::messages::Messages::NotModified(_) => return None,
    };
    u32::try_from(n).ok()
}

#[async_trait]
impl MessagesApiSearch for GrammersBackend {
    async fn search_with(&self, peer: PeerRef, top: Option<MsgId>, query: SearchQuery) -> BackendResult<SearchPage> {
        if matches!(peer, PeerRef::Secret(_)) {
            return Err(unsupported());
        }
        if query.limit == 0 || query.is_empty() {
            return Ok(SearchPage::default());
        }
        let input = self.inner.input_peer(peer)?;
        let lim = query.limit.clamp(1, 100) as i32;
        let r = self
            .inner
            .rpc(&tl::functions::messages::Search {
                peer: input,
                q: query.text.clone(),
                from_id: None,
                saved_peer_id: None,
                saved_reaction: None,
                top_msg_id: top.map(|t| t.0),
                filter: filter_to_tl(query.filter),
                min_date: 0,
                max_date: 0,
                offset_id: query.offset.map_or(0, |o| o.0),
                add_offset: 0,
                limit: lim,
                max_id: 0,
                min_id: 0,
                hash: 0,
            })
            .await?;
        let total = total_of(&r);
        let (msgs, users, chats) = crate::backend::messages_of(r);
        self.inner.ingest(&users, &chats);
        let raw = msgs.len();
        let mut c = self.inner.cache();
        let mut hits: Vec<Message> = msgs.iter().filter_map(|m| c.message(m)).filter(|m| m.peer == peer).collect();
        hits.sort_by_key(|m| std::cmp::Reverse(m.id));
        hits.dedup_by_key(|m| m.id);
        // Más resultados si la página vino llena (y, si se conoce el total, aún queda por ver).
        let next = (raw >= lim as usize).then(|| hits.last().map(|m| m.id)).flatten();
        Ok(SearchPage { hits, total, next })
    }
}
