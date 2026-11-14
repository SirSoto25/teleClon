//! Grupo con temas simulado («Huertos del barrio», `forum = true`): seis temas (el «General» es el 1,
//! uno fijado y cerrado), mensajes repartidos entre ellos, no leídos por tema y entrantes en vivo que
//! llegan a un tema (`TopicMessageNew`). Generado con su propia semilla para no alterar el resto de
//! datos del mock.
use std::collections::HashMap;

use async_trait::async_trait;
use tc_core::*;

use crate::rng::Lcg;
use crate::{Inner, MockBackend, State, data};

/// Grupo foro del mock.
pub const FORUM_GROUP: GroupId = GroupId(5100);
pub const FORUM_CHAT: PeerRef = PeerRef::Group(FORUM_GROUP);
/// Tema con más no leídos del foro («Riego automático»).
pub const FORUM_BUSY_TOPIC: usize = 4;

/// (título, color, emoji, fijado, cerrado, mensajes, no leídos)
type TopicSeed = (&'static str, u32, Option<&'static str>, bool, bool, usize, usize);

const TOPICS: &[TopicSeed] = &[
    ("General", 0x6FB9F0, None, false, false, 30, 2),
    ("Normas y bienvenida", 0xFFD67E, Some("📌"), true, true, 4, 0),
    ("Semillas e intercambio", 0x8EEE98, Some("🌱"), false, false, 40, 5),
    ("Plagas y remedios", 0xFB6F5F, Some("🐛"), false, false, 26, 0),
    ("Riego automático", 0x6FB9F0, Some("💧"), false, false, 70, 14),
    ("Quedadas y talleres", 0xCB86DB, Some("📅"), false, false, 18, 1),
];

const TOPIC_PHRASES: &[&[&str]] = &[
    &["¡Bienvenidas las nuevas parcelas!", "¿Alguien sabe a qué hora abren el portón el domingo?", "Gracias por el aviso."],
    &["Respetad los turnos de riego, por favor.", "Las herramientas se devuelven limpias al cobertizo."],
    &["Me sobran semillas de calabacín, ¿quién quiere?", "Cambio esquejes de menta por tomate cherry.", "Tengo judías de la variedad «perona»."],
    &["Los pulgones han vuelto a las habas.", "Jabón potásico diluido y listo.", "¿Alguien ha probado con mariquitas?"],
    &["El programador del sector B se ha quedado sin pilas.", "He puesto goteo en la parcela 12.", "¿Cuántos minutos regáis en agosto?"],
    &["Taller de compost el sábado a las once.", "¿Hacemos una comida al final de la cosecha?", "Yo llevo la tortilla."],
];

/// Temas por grupo y a qué tema pertenece cada mensaje.
#[derive(Default)]
pub(crate) struct Forums {
    /// Metadatos de cada tema (último mensaje y no leídos se calculan del historial).
    topics: HashMap<GroupId, Vec<ForumTopic>>,
    msg_topic: HashMap<(GroupId, MsgId), MsgId>,
}

/// Añade el foro a los datos de muestra (grupo, diálogo e historial) y devuelve sus temas.
pub(crate) fn seed(seed: &mut data::Seed, now: i64, rng_seed: u64) -> Forums {
    let mut rng = Lcg::new(rng_seed ^ 0xF0_0F);
    let g = FORUM_GROUP;
    seed.groups.insert(
        g,
        Group { id: g, kind: GroupKind::Super, title: "Huertos del barrio".into(), username: None, member_count: 312, photo: crate::images::profile_photo(g.0), is_forum: true },
    );
    let senders: Vec<UserId> = seed.users.keys().copied().collect();
    // Eventos (fecha, tema): el primero de cada tema es su creación, que da el id del tema.
    let mut events: Vec<(i64, usize)> = Vec::new();
    for (k, t) in TOPICS.iter().enumerate() {
        let created = now - 40 * 86_400 + k as i64 * 3_600;
        events.push((created, k));
        let mut at = now - 600 - rng.below(20 * 3_600) as i64;
        for _ in 1..t.5 {
            events.push((at.max(created + 1), k));
            at -= 120 + rng.below(9_000) as i64;
        }
    }
    events.sort_by_key(|(d, k)| (*d, *k));
    let mut forums = Forums::default();
    let mut topics: Vec<ForumTopic> = Vec::new();
    let mut list: Vec<Message> = Vec::new();
    for (i, (date, k)) in events.into_iter().enumerate() {
        let id = MsgId(i as i32 + 1);
        let t = &TOPICS[k];
        let first = !topics.iter().any(|x: &ForumTopic| x.title == t.0);
        let outgoing = !first && rng.chance(25);
        let sender = if outgoing { data::ME } else { senders[rng.below(senders.len())] };
        let text = if first {
            format!("Tema creado: «{}».", t.0)
        } else if rng.chance(60) {
            rng.pick(TOPIC_PHRASES[k]).to_string()
        } else {
            data::incoming_text(&mut rng)
        };
        if first {
            let mut topic = ForumTopic::new(g, id, t.0, t.1);
            topic.icon.emoji = t.2.map(str::to_string);
            topic.date = Timestamp(date);
            topic.pinned = t.3;
            topic.closed = t.4;
            topics.push(topic);
        }
        let topic_id = topics.iter().find(|x| x.title == t.0).map_or(GENERAL_TOPIC, |x| x.id);
        forums.msg_topic.insert((g, id), topic_id);
        list.push(Message {
            id,
            peer: FORUM_CHAT,
            sender: Some(sender),
            date: Timestamp(date),
            edit_date: None,
            outgoing,
            text,
            entities: vec![],
            reply_to: None,
            forwarded_from: None,
            media: None,
            sticker: None,
            reactions: vec![],
            random_id: None,
            state: SendState::Sent,
            ttl_secs: None,
        });
    }
    // Último leído de cada tema: deja `unread` entrantes por leer al final.
    for (k, topic) in topics.iter_mut().enumerate() {
        let unread = TOPICS[k].6;
        let incoming: Vec<MsgId> = list.iter().filter(|m| !m.outgoing && forums.msg_topic.get(&(g, m.id)) == Some(&topic.id)).map(|m| m.id).collect();
        topic.read_inbox_max_id = incoming.iter().rev().nth(unread).copied().unwrap_or(MsgId(0));
        topic.read_outbox_max_id = list.iter().rev().find(|m| m.outgoing).map_or(MsgId(0), |m| m.id);
    }
    let last = list.last().cloned();
    let unread: u32 = topics.iter().map(|t| unread_in(&forums.msg_topic, &list, t)).sum();
    seed.dialogs.insert(
        FORUM_CHAT,
        Dialog {
            peer: FORUM_CHAT,
            title: "Huertos del barrio".into(),
            activity: last.as_ref().map_or(Timestamp(0), |m| m.date),
            last_message: last,
            unread_count: unread,
            unread_mentions: 0,
            pinned_order: None,
            archived: false,
            muted: false,
            draft: None,
            read_outbox_max_id: MsgId(0),
            read_inbox_max_id: MsgId(0),
            photo: crate::images::profile_photo(g.0),
            unread_mark: false,
        },
    );
    seed.messages.insert(FORUM_CHAT, list);
    forums.topics.insert(g, topics);
    forums
}

fn unread_in(msg_topic: &HashMap<(GroupId, MsgId), MsgId>, list: &[Message], t: &ForumTopic) -> u32 {
    list.iter().filter(|m| !m.outgoing && m.id > t.read_inbox_max_id && msg_topic.get(&(t.group, m.id)) == Some(&t.id)).count() as u32
}

/// Página de `list` (ascendente por id) según el ancla; mismo criterio que `history`.
pub(crate) fn page_by_anchor(list: &[Message], anchor: Anchor, limit: usize) -> Vec<Message> {
    let lower = |x: MsgId| list.partition_point(|m| m.id < x);
    let (start, end) = match anchor {
        Anchor::Latest => (list.len().saturating_sub(limit), list.len()),
        Anchor::Before(id) => {
            let e = lower(id);
            (e.saturating_sub(limit), e)
        }
        Anchor::After(id) => {
            let s = list.partition_point(|m| m.id <= id);
            (s, (s + limit).min(list.len()))
        }
        Anchor::Around(id) => {
            let c = lower(id);
            let s = c.saturating_sub(limit / 2);
            (s, (s + limit).min(list.len()))
        }
    };
    list[start..end].to_vec()
}

impl State {
    fn is_forum(&self, g: GroupId) -> bool {
        self.groups.get(&g).is_some_and(|g| g.is_forum)
    }

    /// Tema con último mensaje y no leídos calculados.
    fn topic(&self, thread: ThreadRef) -> BackendResult<ForumTopic> {
        let mut t = self.forums.topics.get(&thread.group).and_then(|v| v.iter().find(|t| t.id == thread.topic)).cloned().ok_or(BackendError::NotFound)?;
        let list = self.messages.get(&thread.peer()).map_or(&[][..], Vec::as_slice);
        t.last_message = list.iter().rev().find(|m| self.forums.msg_topic.get(&(thread.group, m.id)) == Some(&thread.topic)).cloned();
        t.unread_count = unread_in(&self.forums.msg_topic, list, &t);
        Ok(t)
    }

    fn topics_of(&self, g: GroupId) -> Vec<ForumTopic> {
        let ids: Vec<MsgId> = self.forums.topics.get(&g).map(|v| v.iter().map(|t| t.id).collect()).unwrap_or_default();
        let mut out: Vec<ForumTopic> = ids.into_iter().filter_map(|id| self.topic(ThreadRef { group: g, topic: id }).ok()).collect();
        sort_topics(&mut out);
        out
    }

    fn topic_meta_mut(&mut self, thread: ThreadRef) -> BackendResult<&mut ForumTopic> {
        self.forums.topics.get_mut(&thread.group).and_then(|v| v.iter_mut().find(|t| t.id == thread.topic)).ok_or(BackendError::NotFound)
    }

    fn ensure_topic_writable(&self, thread: ThreadRef) -> BackendResult<()> {
        self.ensure_writable(thread.peer())?;
        if !self.is_forum(thread.group) {
            return Err(BackendError::InvalidInput("topic"));
        }
        if self.topic(thread)?.closed {
            return Err(BackendError::Forbidden("el tema está cerrado: solo escriben quienes lo administran".into()));
        }
        Ok(())
    }

    /// No leídos del grupo = suma de los de sus temas.
    fn forum_dialog(&mut self, g: GroupId) -> Option<Dialog> {
        let sum: u32 = self.topics_of(g).iter().map(|t| t.unread_count).sum();
        let d = self.dialogs.get_mut(&PeerRef::Group(g))?;
        d.unread_count = sum;
        Some(d.clone())
    }

    /// Mensaje recién añadido al grupo que pertenece a `thread`: lo etiqueta y devuelve los eventos
    /// del tema y del diálogo.
    fn tag_topic(&mut self, thread: ThreadRef, ids: &[MsgId]) -> Vec<BackendEvent> {
        for id in ids {
            self.forums.msg_topic.insert((thread.group, *id), thread.topic);
        }
        let mut ev = Vec::new();
        if let Ok(t) = self.topic(thread) {
            ev.push(BackendEvent::ForumTopicUpserted(t));
        }
        if let Some(d) = self.forum_dialog(thread.group) {
            ev.push(BackendEvent::DialogUpserted(d));
        }
        ev
    }
}

impl Inner {
    /// Entrante en un grupo foro: va a un tema abierto al azar (o a `topic`) y se anuncia como
    /// `TopicMessageNew`. `None` si `peer` no es un foro.
    pub(crate) fn forum_incoming(&self, peer: PeerRef, topic: Option<MsgId>, text: &str) -> Option<BackendResult<Message>> {
        let PeerRef::Group(g) = peer else { return None };
        let mut st = self.lock();
        if !st.is_forum(g) {
            return None;
        }
        let open: Vec<MsgId> = st.topics_of(g).iter().filter(|t| !t.closed).map(|t| t.id).collect();
        let topic = match topic {
            Some(t) => t,
            None if open.is_empty() => GENERAL_TOPIC,
            None => open[st.rng.below(open.len())],
        };
        let thread = ThreadRef { group: g, topic };
        if let Err(e) = st.topic(thread) {
            return Some(Err(e));
        }
        let ids: Vec<UserId> = st.users.keys().copied().collect();
        let sender = ids[st.rng.below(ids.len())];
        let mut m = st.blank_message(peer, false, Some(sender));
        m.text = text.to_string();
        let (m, _) = st.append(m);
        let mut events = vec![BackendEvent::TopicMessageNew { topic, msg: m.clone() }];
        events.extend(st.tag_topic(thread, &[m.id]));
        drop(st);
        events.into_iter().for_each(|e| self.emit(e));
        Some(Ok(m))
    }
}

impl MockBackend {
    /// Inyecta un mensaje entrante en un tema concreto (pruebas y demos).
    pub fn inject_topic_incoming(&self, thread: ThreadRef, text: &str) -> BackendResult<Message> {
        self.inner.forum_incoming(thread.peer(), Some(thread.topic), text).unwrap_or(Err(BackendError::NotFound))
    }

    /// Tras un envío normal al grupo: etiqueta los mensajes con su tema y anuncia el tema.
    fn after_topic_send(&self, thread: ThreadRef, sent: &[Message]) {
        let ids: Vec<MsgId> = sent.iter().map(|m| m.id).collect();
        let events = self.inner.lock().tag_topic(thread, &ids);
        events.into_iter().for_each(|e| self.inner.emit(e));
    }

    fn check_topic(&self, thread: ThreadRef) -> BackendResult<()> {
        self.inner.lock().ensure_topic_writable(thread)
    }
}

#[async_trait]
impl ForumApi for MockBackend {
    async fn forum_topics(&self, group: GroupId, query: TopicsQuery) -> BackendResult<TopicsPage> {
        self.inner.gate().await?;
        let st = self.inner.lock();
        st.ensure_peer(PeerRef::Group(group))?;
        if !st.is_forum(group) {
            return Err(BackendError::InvalidInput("forum"));
        }
        let q = query.query.as_deref().map(str::to_lowercase).unwrap_or_default();
        let all: Vec<ForumTopic> = st.topics_of(group).into_iter().filter(|t| q.is_empty() || t.title.to_lowercase().contains(q.trim())).collect();
        let start = query.offset.and_then(|o| all.iter().position(|t| t.id == o.topic)).map_or(0, |p| p + 1);
        let end = (start + query.limit.max(1)).min(all.len());
        let topics = all[start.min(end)..end].to_vec();
        let next = (end < all.len()).then(|| {
            let l = &all[end - 1];
            TopicsOffset { date: l.activity(), msg_id: l.last_message.as_ref().map_or(l.id, |m| m.id), topic: l.id }
        });
        Ok(TopicsPage { topics, total: all.len() as u32, next })
    }

    async fn forum_topics_by_id(&self, group: GroupId, ids: Vec<MsgId>) -> BackendResult<Vec<ForumTopic>> {
        self.inner.gate().await?;
        let st = self.inner.lock();
        Ok(ids.into_iter().filter_map(|id| st.topic(ThreadRef { group, topic: id }).ok()).collect())
    }

    async fn topic_history(&self, thread: ThreadRef, anchor: Anchor, limit: usize) -> BackendResult<Vec<Message>> {
        self.inner.gate().await?;
        let st = self.inner.lock();
        st.ensure_peer(thread.peer())?;
        st.topic(thread)?;
        let list: Vec<Message> = st
            .messages
            .get(&thread.peer())
            .map(|l| l.iter().filter(|m| st.forums.msg_topic.get(&(thread.group, m.id)) == Some(&thread.topic)).cloned().collect())
            .unwrap_or_default();
        Ok(page_by_anchor(&list, anchor, limit))
    }

    async fn search_in_topic(&self, thread: ThreadRef, query: SearchQuery) -> BackendResult<SearchPage> {
        self.inner.gate().await?;
        let st = self.inner.lock();
        st.ensure_peer(thread.peer())?;
        st.topic(thread)?;
        let list = st.messages.get(&thread.peer()).into_iter().flatten();
        Ok(crate::search::search(list.filter(|m| st.forums.msg_topic.get(&(thread.group, m.id)) == Some(&thread.topic)), &query))
    }

    async fn send_topic_text(&self, thread: ThreadRef, msg: OutgoingText) -> BackendResult<Message> {
        self.check_topic(thread)?;
        let m = self.send_text(thread.peer(), msg).await?;
        self.after_topic_send(thread, std::slice::from_ref(&m));
        Ok(m)
    }

    async fn send_topic_media(&self, thread: ThreadRef, media: OutgoingMedia) -> BackendResult<Vec<Message>> {
        self.check_topic(thread)?;
        let v = self.send_media(thread.peer(), media).await?;
        self.after_topic_send(thread, &v);
        Ok(v)
    }

    async fn send_topic_sticker(&self, thread: ThreadRef, sticker: StickerId, reply_to: Option<MsgId>, random_id: RandomId) -> BackendResult<Message> {
        self.check_topic(thread)?;
        let m = self.send_sticker(thread.peer(), sticker, reply_to, random_id).await?;
        self.after_topic_send(thread, std::slice::from_ref(&m));
        Ok(m)
    }

    async fn send_topic_gif(&self, thread: ThreadRef, gif: Gif, reply_to: Option<MsgId>, random_id: RandomId) -> BackendResult<Message> {
        self.check_topic(thread)?;
        let m = self.send_gif(thread.peer(), gif, reply_to, random_id).await?;
        self.after_topic_send(thread, std::slice::from_ref(&m));
        Ok(m)
    }

    async fn mark_topic_read(&self, thread: ThreadRef, max_id: MsgId) -> BackendResult<()> {
        self.inner.gate().await?;
        let events = {
            let mut st = self.inner.lock();
            let t = st.topic_meta_mut(thread)?;
            t.read_inbox_max_id = t.read_inbox_max_id.max(max_id);
            st.tag_topic(thread, &[])
        };
        events.into_iter().for_each(|e| self.inner.emit(e));
        Ok(())
    }

    async fn set_topic_typing(&self, thread: ThreadRef, _typing: bool) -> BackendResult<()> {
        self.inner.gate().await?;
        self.inner.lock().topic(thread).map(|_| ())
    }

    async fn create_topic(&self, group: GroupId, title: String, icon_color: Option<u32>, random_id: RandomId) -> BackendResult<ForumTopic> {
        self.inner.gate().await?;
        let title = title.trim().to_string();
        if title.is_empty() || title.chars().count() > 128 {
            return Err(BackendError::InvalidInput("title"));
        }
        let (topic, events) = {
            let mut st = self.inner.lock();
            let peer = PeerRef::Group(group);
            st.ensure_writable(peer)?;
            if !st.is_forum(group) {
                return Err(BackendError::InvalidInput("forum"));
            }
            let me = st.me.id;
            let mut m = st.blank_message(peer, true, Some(me));
            m.text = format!("Tema creado: «{title}».");
            m.random_id = Some(random_id);
            let (m, _) = st.append(m);
            let mut t = ForumTopic::new(group, m.id, title, icon_color.unwrap_or(TOPIC_COLORS[m.id.0 as usize % TOPIC_COLORS.len()]));
            t.date = m.date;
            t.my = true;
            t.read_inbox_max_id = m.id;
            st.forums.topics.entry(group).or_default().push(t);
            let events = st.tag_topic(ThreadRef { group, topic: m.id }, &[m.id]);
            (st.topic(ThreadRef { group, topic: m.id })?, events)
        };
        events.into_iter().for_each(|e| self.inner.emit(e));
        Ok(topic)
    }

    async fn set_topic_pinned(&self, thread: ThreadRef, pinned: bool) -> BackendResult<()> {
        self.inner.gate().await?;
        let events = {
            let mut st = self.inner.lock();
            st.topic_meta_mut(thread)?.pinned = pinned;
            st.tag_topic(thread, &[])
        };
        events.into_iter().for_each(|e| self.inner.emit(e));
        Ok(())
    }

    async fn set_topic_closed(&self, thread: ThreadRef, closed: bool) -> BackendResult<()> {
        self.inner.gate().await?;
        let events = {
            let mut st = self.inner.lock();
            st.topic_meta_mut(thread)?.closed = closed;
            st.tag_topic(thread, &[])
        };
        events.into_iter().for_each(|e| self.inner.emit(e));
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use futures::StreamExt;

    use super::*;
    use crate::MockConfig;

    fn mock() -> MockBackend {
        MockBackend::new(MockConfig { now: Some(1_700_000_000), ..Default::default() })
    }

    fn thread(topic: MsgId) -> ThreadRef {
        ThreadRef { group: FORUM_GROUP, topic }
    }

    #[tokio::test]
    async fn el_foro_tiene_temas_con_general_y_no_leidos_que_suman() {
        let b = mock();
        assert!(b.group(FORUM_GROUP).await.unwrap().is_forum);
        let page = b.forum_topics(FORUM_GROUP, TopicsQuery::first(50)).await.unwrap();
        assert_eq!((page.topics.len(), page.total, page.next), (TOPICS.len(), TOPICS.len() as u32, None));
        assert!(page.topics[0].pinned && page.topics[0].closed, "el fijado va primero");
        assert!(page.topics.iter().any(|t| t.is_general()));
        let sum: u32 = page.topics.iter().map(|t| t.unread_count).sum();
        let d = b.dialogs(DialogsPage { offset: 0, limit: 500, archived: false }).await.unwrap().into_iter().find(|d| d.peer == FORUM_CHAT).unwrap();
        assert_eq!(d.unread_count, sum);
        assert_eq!(sum, TOPICS.iter().map(|t| t.6 as u32).sum::<u32>());
        // Paginación y búsqueda por título.
        let p1 = b.forum_topics(FORUM_GROUP, TopicsQuery::first(4)).await.unwrap();
        let p2 = b.forum_topics(FORUM_GROUP, TopicsQuery { query: None, offset: p1.next, limit: 4 }).await.unwrap();
        assert_eq!((p1.topics.len(), p2.topics.len(), p2.next), (4, 2, None));
        let q = b.forum_topics(FORUM_GROUP, TopicsQuery { query: Some("RIEGO".into()), offset: None, limit: 10 }).await.unwrap();
        assert_eq!(q.topics.iter().map(|t| t.title.as_str()).collect::<Vec<_>>(), ["Riego automático"]);
        // Un grupo normal no es un foro.
        assert!(b.forum_topics(GroupId(5002), TopicsQuery::first(5)).await.is_err());
    }

    #[tokio::test]
    async fn historial_envio_y_lectura_por_tema() {
        let b = mock();
        let topics = b.forum_topics(FORUM_GROUP, TopicsQuery::first(50)).await.unwrap().topics;
        let riego = topics.iter().find(|t| t.title == "Riego automático").unwrap().clone();
        let th = riego.thread();
        let h = b.topic_history(th, Anchor::Latest, 100).await.unwrap();
        assert_eq!(h.len(), TOPICS[FORUM_BUSY_TOPIC].5);
        assert!(h.windows(2).all(|w| w[0].id < w[1].id));
        assert_eq!(h[0].id, th.topic, "el primer mensaje es el de creación");
        let around = b.topic_history(th, Anchor::Around(riego.read_inbox_max_id), 20).await.unwrap();
        assert!(around.iter().any(|m| m.id > riego.read_inbox_max_id));

        let mut ev = b.events();
        let out = OutgoingText { text: "Pongo el programador nuevo".into(), entities: vec![], reply_to: None, random_id: RandomId(1) };
        let m = b.send_topic_text(th, out.clone()).await.unwrap();
        assert_eq!(b.send_topic_text(th, out).await.unwrap().id, m.id, "idempotente por random_id");
        assert_eq!(b.topic_history(th, Anchor::Latest, 1).await.unwrap()[0].id, m.id);
        let general = b.topic_history(thread(GENERAL_TOPIC), Anchor::Latest, 200).await.unwrap();
        assert!(general.iter().all(|x| x.id != m.id), "no se cuela en otro tema");
        let mut saw_topic = false;
        while let Some(e) = ev.next().await {
            if let BackendEvent::ForumTopicUpserted(t) = e {
                assert_eq!(t.last_message.as_ref().map(|x| x.id), Some(m.id));
                saw_topic = true;
                break;
            }
        }
        assert!(saw_topic);

        b.mark_topic_read(th, m.id).await.unwrap();
        let t = b.forum_topics_by_id(FORUM_GROUP, vec![th.topic]).await.unwrap().remove(0);
        assert_eq!(t.unread_count, 0);
        // El cerrado no admite mensajes.
        let normas = topics.iter().find(|t| t.closed).unwrap().thread();
        let no = OutgoingText { text: "hola".into(), entities: vec![], reply_to: None, random_id: RandomId(2) };
        assert!(matches!(b.send_topic_text(normas, no).await, Err(BackendError::Forbidden(_))));
    }

    #[tokio::test]
    async fn entrante_en_tema_crear_y_fijar() {
        let b = mock();
        let mut ev = b.events();
        let m = b.inject_topic_incoming(thread(GENERAL_TOPIC), "¿Quién tiene la llave?").unwrap();
        assert_eq!(ev.next().await, Some(BackendEvent::TopicMessageNew { topic: GENERAL_TOPIC, msg: m.clone() }));
        assert!(matches!(ev.next().await, Some(BackendEvent::ForumTopicUpserted(t)) if t.is_general() && t.unread_count >= 1));
        let t = b.create_topic(FORUM_GROUP, "Compostaje".into(), None, RandomId(9)).await.unwrap();
        assert!(t.my && t.title == "Compostaje" && t.unread_count == 0);
        b.set_topic_pinned(t.thread(), true).await.unwrap();
        b.set_topic_closed(t.thread(), true).await.unwrap();
        let first = b.forum_topics(FORUM_GROUP, TopicsQuery::first(50)).await.unwrap().topics;
        assert!(first.iter().take_while(|x| x.pinned).any(|x| x.id == t.id));
        assert!(b.create_topic(FORUM_GROUP, "  ".into(), None, RandomId(10)).await.is_err());
    }
}
