//! Grupos con temas (`channel.forum`) sobre TL en crudo:
//!
//! - Lista: `messages.getForumTopics` (paginada por fecha/mensaje/tema, con búsqueda `q`) y
//!   `messages.getForumTopicsByID`.
//! - Historial de un tema: `messages.getReplies` con `msg_id` = id del tema (su mensaje superior).
//! - Envío: el `reply_to` de todos los envíos sale de `backend::reply_to`, que dentro de
//!   [`in_topic`] añade `top_msg_id` (y, si no se responde a nada, `reply_to_msg_id` = el tema).
//!   En el «General» (id 1) no se pone `reply_to`.
//! - Lectura: `messages.readDiscussion`.
//! - Tiempo real: los mensajes nuevos de un foro salen como `TopicMessageNew` (tema por
//!   `reply_to.reply_to_top_id`, o `reply_to_msg_id` con `forum_topic`; sin cabecera, el General) y
//!   actualizan el tema en caché (`ForumTopicUpserted`); `updateReadChannelDiscussionInbox` y
//!   `updatePinnedForumTopic` también; lo que no se puede aplicar por partes da `ForumTopicsChanged`.
//!
//! Sin probar contra los servidores reales (solo con el transporte falso de las pruebas).
use std::collections::{BTreeMap, HashMap};

use async_trait::async_trait;
use grammers_tl_types as tl;
use tc_core::*;

use crate::backend::{history_params, messages_of};
use crate::events::{Cache, flatten_updates};
use crate::ids::{channel_group, peer_from_tl};
use crate::{GrammersBackend, Inner};

tokio::task_local! {
    /// Tema del envío en curso (lo consulta `backend::reply_to`).
    static TOPIC: MsgId;
}

/// Tema del envío en curso, si se está enviando dentro de [`in_topic`].
pub(crate) fn current_topic() -> Option<MsgId> {
    TOPIC.try_with(|t| *t).ok()
}

/// Ejecuta un envío dentro de un tema.
pub(crate) async fn in_topic<F: std::future::Future>(topic: MsgId, f: F) -> F::Output {
    TOPIC.scope(topic, f).await
}

/// `InputReplyTo` de un envío dentro de `topic` (`reply` = a qué mensaje se responde).
pub(crate) fn topic_reply_to(topic: MsgId, reply: Option<MsgId>) -> Option<tl::enums::InputReplyTo> {
    if topic == GENERAL_TOPIC && reply.is_none() {
        return None;
    }
    Some(
        tl::types::InputReplyToMessage {
            reply_to_msg_id: reply.unwrap_or(topic).0,
            top_msg_id: (topic != GENERAL_TOPIC).then_some(topic.0),
            reply_to_peer_id: None,
            quote_text: None,
            quote_entities: None,
            quote_offset: None,
            monoforum_peer_id: None,
            todo_item_id: None,
            poll_option: None,
        }
        .into(),
    )
}

/// Temas conocidos por grupo (los que ha traído `getForumTopics` o han ido llegando).
pub(crate) type Topics = HashMap<GroupId, BTreeMap<i32, ForumTopic>>;

/// Tema de un mensaje TL de un foro.
pub(crate) fn topic_of(m: &tl::enums::Message) -> MsgId {
    let (id, reply, action) = match m {
        tl::enums::Message::Message(m) => (m.id, m.reply_to.as_ref(), None),
        tl::enums::Message::Service(m) => (m.id, m.reply_to.as_ref(), Some(&m.action)),
        tl::enums::Message::Empty(_) => return GENERAL_TOPIC,
    };
    if matches!(action, Some(tl::enums::MessageAction::TopicCreate(_))) {
        return MsgId(id);
    }
    match reply {
        Some(tl::enums::MessageReplyHeader::Header(h)) if h.forum_topic => MsgId(h.reply_to_top_id.or(h.reply_to_msg_id).unwrap_or(1)),
        _ => GENERAL_TOPIC,
    }
}

fn topic_from_tl(t: &tl::types::ForumTopic, group: GroupId, last: Option<Message>) -> ForumTopic {
    ForumTopic {
        group,
        id: MsgId(t.id),
        title: t.title.clone(),
        icon: TopicIcon { color: t.icon_color as u32 & 0xFF_FFFF, emoji: None, custom_emoji_id: t.icon_emoji_id.filter(|e| *e != 0) },
        date: Timestamp(t.date.into()),
        last_message: last,
        unread_count: t.unread_count.max(0) as u32,
        unread_mentions: t.unread_mentions_count.max(0) as u32,
        read_inbox_max_id: MsgId(t.read_inbox_max_id),
        read_outbox_max_id: MsgId(t.read_outbox_max_id),
        pinned: t.pinned,
        closed: t.closed,
        hidden: t.hidden,
        my: t.my,
    }
}

pub(crate) fn is_forum(cache: &Cache, g: GroupId) -> bool {
    cache.peers.groups.get(&g).is_some_and(|g| g.is_forum)
}

/// Tras `updateNewChannelMessage` (con el último evento ya puesto: `MessageNew` o `MessageSent`):
/// en un foro, `MessageNew` pasa a `TopicMessageNew` y se actualiza el tema.
pub(crate) fn route_new_message(cache: &mut Cache, raw: &tl::enums::Message, ev: &mut Vec<BackendEvent>) {
    let msg = match ev.last() {
        Some(BackendEvent::MessageNew(m) | BackendEvent::MessageSent { msg: m, .. }) => m.clone(),
        _ => return,
    };
    let PeerRef::Group(g) = msg.peer else { return };
    if !is_forum(cache, g) {
        return;
    }
    let topic = topic_of(raw);
    if matches!(ev.last(), Some(BackendEvent::MessageNew(_)))
        && let Some(BackendEvent::MessageNew(m)) = ev.pop()
    {
        ev.push(BackendEvent::TopicMessageNew { topic, msg: m });
    }
    let action = match raw {
        tl::enums::Message::Service(s) => Some(&s.action),
        _ => None,
    };
    let topics = cache.forums.entry(g).or_default();
    if let Some(tl::enums::MessageAction::TopicCreate(c)) = action {
        let mut t = ForumTopic::new(g, msg.id, c.title.clone(), c.icon_color as u32 & 0xFF_FFFF);
        t.icon.custom_emoji_id = c.icon_emoji_id;
        t.date = msg.date;
        t.my = msg.outgoing;
        t.read_inbox_max_id = msg.id;
        t.last_message = Some(msg);
        topics.insert(t.id.0, t.clone());
        ev.push(BackendEvent::ForumTopicUpserted(t));
        return;
    }
    match topics.get_mut(&topic.0) {
        Some(t) => {
            if t.last_message.as_ref().is_none_or(|l| l.id <= msg.id) {
                t.last_message = Some(msg.clone());
            }
            if msg.outgoing {
                t.read_inbox_max_id = t.read_inbox_max_id.max(msg.id);
            } else if msg.id > t.read_inbox_max_id {
                t.unread_count += 1;
            }
            ev.push(BackendEvent::ForumTopicUpserted(t.clone()));
        }
        None => ev.push(BackendEvent::ForumTopicsChanged { group: g }),
    }
    if matches!(action, Some(tl::enums::MessageAction::TopicEdit(_))) {
        ev.push(BackendEvent::ForumTopicsChanged { group: g });
    }
}

/// Updates propios de los foros que no son mensajes. Devuelve `true` si lo ha tratado.
pub(crate) fn apply_update(cache: &mut Cache, u: &tl::enums::Update, ev: &mut Vec<BackendEvent>) -> bool {
    use tl::enums::Update as U;
    match u {
        U::ReadChannelDiscussionInbox(r) => {
            let g = channel_group(r.channel_id);
            if !is_forum(cache, g) {
                return true;
            }
            match cache.forums.get_mut(&g).and_then(|m| m.get_mut(&r.top_msg_id)) {
                Some(t) => {
                    t.read_inbox_max_id = t.read_inbox_max_id.max(MsgId(r.read_max_id));
                    if t.last_message.as_ref().is_none_or(|l| l.id <= t.read_inbox_max_id) {
                        t.unread_count = 0;
                    }
                    ev.push(BackendEvent::ForumTopicUpserted(t.clone()));
                }
                None => ev.push(BackendEvent::ForumTopicsChanged { group: g }),
            }
            true
        }
        U::PinnedForumTopic(p) => {
            let PeerRef::Group(g) = peer_from_tl(&p.peer) else { return true };
            match cache.forums.get_mut(&g).and_then(|m| m.get_mut(&p.topic_id)) {
                Some(t) => {
                    t.pinned = p.pinned;
                    ev.push(BackendEvent::ForumTopicUpserted(t.clone()));
                }
                None => ev.push(BackendEvent::ForumTopicsChanged { group: g }),
            }
            true
        }
        U::PinnedForumTopics(p) => {
            if let PeerRef::Group(g) = peer_from_tl(&p.peer) {
                ev.push(BackendEvent::ForumTopicsChanged { group: g });
            }
            true
        }
        _ => false,
    }
}

impl Inner {
    /// Traduce una respuesta `messages.forumTopics` y guarda los temas en caché.
    fn ingest_topics(&self, group: GroupId, r: tl::enums::messages::ForumTopics) -> (Vec<ForumTopic>, u32) {
        let tl::enums::messages::ForumTopics::Topics(r) = r;
        self.ingest(&r.users, &r.chats);
        let mut c = self.cache();
        let mut tops: HashMap<i32, Message> = HashMap::new();
        for m in &r.messages {
            if let Some(m) = c.message(m).filter(|m| m.peer == PeerRef::Group(group)) {
                tops.insert(m.id.0, m);
            }
        }
        let mut out = Vec::new();
        for t in &r.topics {
            match t {
                tl::enums::ForumTopic::Topic(t) => {
                    let topic = topic_from_tl(t, group, tops.get(&t.top_message).cloned());
                    c.forums.entry(group).or_default().insert(topic.id.0, topic.clone());
                    out.push(topic);
                }
                tl::enums::ForumTopic::Deleted(d) => {
                    if let Some(m) = c.forums.get_mut(&group) {
                        m.remove(&d.id);
                    }
                }
            }
        }
        (out, r.count.max(0) as u32)
    }

    fn topic_changed(&self, thread: ThreadRef, f: impl FnOnce(&mut ForumTopic)) {
        let t = {
            let mut c = self.cache();
            c.forums.get_mut(&thread.group).and_then(|m| m.get_mut(&thread.topic.0)).map(|t| {
                f(t);
                t.clone()
            })
        };
        if let Some(t) = t {
            self.emit(BackendEvent::ForumTopicUpserted(t));
        }
    }

    fn apply_forum_updates(&self, r: tl::enums::Updates) {
        let (ups, users, chats, _) = flatten_updates(r);
        let applied = self.apply_response(ups, users, chats);
        self.emit_all(applied.events);
    }
}

#[async_trait]
impl ForumApi for GrammersBackend {
    async fn forum_topics(&self, group: GroupId, query: TopicsQuery) -> BackendResult<TopicsPage> {
        self.inner.ensure_logged_in()?;
        let peer = self.inner.input_peer(PeerRef::Group(group))?;
        let limit = query.limit.clamp(1, 100) as i32;
        let off = query.offset;
        let req = tl::functions::messages::GetForumTopics {
            peer,
            q: query.query.clone().filter(|q| !q.trim().is_empty()),
            offset_date: off.map_or(0, |o| o.date.0 as i32),
            offset_id: off.map_or(0, |o| o.msg_id.0),
            offset_topic: off.map_or(0, |o| o.topic.0),
            limit,
        };
        let r = self.inner.rpc(&req).await?;
        let (topics, total) = self.inner.ingest_topics(group, r);
        let next = (topics.len() as i32 >= limit).then(|| topics.last()).flatten().map(|l| TopicsOffset {
            date: l.last_message.as_ref().map_or(l.date, |m| m.date),
            msg_id: l.last_message.as_ref().map_or(l.id, |m| m.id),
            topic: l.id,
        });
        Ok(TopicsPage { topics, total, next })
    }

    async fn forum_topics_by_id(&self, group: GroupId, ids: Vec<MsgId>) -> BackendResult<Vec<ForumTopic>> {
        if ids.is_empty() {
            return Ok(Vec::new());
        }
        let peer = self.inner.input_peer(PeerRef::Group(group))?;
        let r = self.inner.rpc(&tl::functions::messages::GetForumTopicsById { peer, topics: ids.iter().map(|i| i.0).collect() }).await?;
        Ok(self.inner.ingest_topics(group, r).0)
    }

    async fn topic_history(&self, thread: ThreadRef, anchor: Anchor, limit: usize) -> BackendResult<Vec<Message>> {
        let peer = thread.peer();
        let input = self.inner.input_peer(peer)?;
        let lim = limit.clamp(1, 100) as i32;
        let (offset_id, add_offset, min_id) = history_params(anchor, lim);
        let r = self
            .inner
            .rpc(&tl::functions::messages::GetReplies { peer: input, msg_id: thread.topic.0, offset_id, offset_date: 0, add_offset, limit: lim, max_id: 0, min_id, hash: 0 })
            .await?;
        let (msgs, users, chats) = messages_of(r);
        self.inner.ingest(&users, &chats);
        let mut c = self.inner.cache();
        let mut out: Vec<Message> = msgs.iter().filter_map(|m| c.message(m)).filter(|m| m.peer == peer).collect();
        if let Anchor::After(id) = anchor {
            out.retain(|m| m.id > id);
        }
        out.sort_by_key(|m| m.id);
        out.dedup_by_key(|m| m.id);
        Ok(out)
    }

    async fn search_in_topic(&self, thread: ThreadRef, query: SearchQuery) -> BackendResult<SearchPage> {
        use crate::backend::MessagesApiSearch;
        self.search_with(thread.peer(), Some(thread.topic), query).await
    }

    async fn send_topic_text(&self, thread: ThreadRef, msg: OutgoingText) -> BackendResult<Message> {
        in_topic(thread.topic, self.send_text(thread.peer(), msg)).await
    }

    async fn send_topic_media(&self, thread: ThreadRef, media: OutgoingMedia) -> BackendResult<Vec<Message>> {
        in_topic(thread.topic, self.send_media(thread.peer(), media)).await
    }

    async fn send_topic_sticker(&self, thread: ThreadRef, sticker: StickerId, reply_to: Option<MsgId>, random_id: RandomId) -> BackendResult<Message> {
        in_topic(thread.topic, self.send_sticker(thread.peer(), sticker, reply_to, random_id)).await
    }

    async fn send_topic_gif(&self, thread: ThreadRef, gif: Gif, reply_to: Option<MsgId>, random_id: RandomId) -> BackendResult<Message> {
        in_topic(thread.topic, self.send_gif(thread.peer(), gif, reply_to, random_id)).await
    }

    async fn mark_topic_read(&self, thread: ThreadRef, max_id: MsgId) -> BackendResult<()> {
        let peer = self.inner.input_peer(thread.peer())?;
        self.inner.rpc(&tl::functions::messages::ReadDiscussion { peer, msg_id: thread.topic.0, read_max_id: max_id.0 }).await?;
        self.inner.topic_changed(thread, |t| {
            t.read_inbox_max_id = t.read_inbox_max_id.max(max_id);
            if t.last_message.as_ref().is_none_or(|l| l.id <= t.read_inbox_max_id) {
                t.unread_count = 0;
            }
        });
        Ok(())
    }

    async fn set_topic_typing(&self, thread: ThreadRef, typing: bool) -> BackendResult<()> {
        let peer = self.inner.input_peer(thread.peer())?;
        let action = if typing { tl::enums::SendMessageAction::SendMessageTypingAction } else { tl::enums::SendMessageAction::SendMessageCancelAction };
        let top = (thread.topic != GENERAL_TOPIC).then_some(thread.topic.0);
        self.inner.rpc(&tl::functions::messages::SetTyping { peer, top_msg_id: top, action }).await?;
        Ok(())
    }

    async fn create_topic(&self, group: GroupId, title: String, icon_color: Option<u32>, random_id: RandomId) -> BackendResult<ForumTopic> {
        let title = title.trim().to_string();
        if title.is_empty() || title.chars().count() > 128 {
            return Err(BackendError::InvalidInput("title"));
        }
        let peer = self.inner.input_peer(PeerRef::Group(group))?;
        let req = tl::functions::messages::CreateForumTopic {
            title_missing: false,
            peer,
            title,
            icon_color: icon_color.map(|c| c as i32),
            icon_emoji_id: None,
            random_id: random_id.0,
            send_as: None,
        };
        let (ups, users, chats, _) = flatten_updates(self.inner.rpc(&req).await?);
        let applied = self.inner.apply_response(ups, users, chats);
        let created = applied.events.iter().find_map(|e| match e {
            BackendEvent::ForumTopicUpserted(t) if t.group == group && t.my => Some(t.clone()),
            _ => None,
        });
        self.inner.emit_all(applied.events);
        created.ok_or(BackendError::Internal)
    }

    async fn set_topic_pinned(&self, thread: ThreadRef, pinned: bool) -> BackendResult<()> {
        let peer = self.inner.input_peer(thread.peer())?;
        let r = self.inner.rpc(&tl::functions::messages::UpdatePinnedForumTopic { peer, topic_id: thread.topic.0, pinned }).await?;
        self.inner.apply_forum_updates(r);
        self.inner.topic_changed(thread, |t| t.pinned = pinned);
        Ok(())
    }

    async fn set_topic_closed(&self, thread: ThreadRef, closed: bool) -> BackendResult<()> {
        let peer = self.inner.input_peer(thread.peer())?;
        let req = tl::functions::messages::EditForumTopic { peer, topic_id: thread.topic.0, title: None, icon_emoji_id: None, closed: Some(closed), hidden: None };
        let r = self.inner.rpc(&req).await?;
        self.inner.apply_forum_updates(r);
        self.inner.topic_changed(thread, |t| t.closed = closed);
        Ok(())
    }
}

impl GrammersBackend {
    /// «Marcar como leído» en un grupo con temas: `messages.readHistory` solo limpia el General, así
    /// que se leen también todos los temas con mensajes sin leer (`messages.readDiscussion`).
    pub(crate) async fn mark_all_topics_read(&self, group: GroupId) {
        let mut query = TopicsQuery::first(100);
        for _ in 0..10 {
            let Ok(page) = self.forum_topics(group, query.clone()).await else { return };
            for t in page.topics.iter().filter(|t| t.unread_count > 0 || t.unread_mentions > 0) {
                let Some(max) = t.last_message.as_ref().map(|m| m.id) else { continue };
                let thread = ThreadRef { group, topic: t.id };
                if let Err(e) = self.mark_topic_read(thread, max).await {
                    tracing::warn!(error = %e, "no se pudo marcar un tema como leído");
                }
            }
            match page.next {
                Some(next) => query.offset = Some(next),
                None => return,
            }
        }
    }
}
