//! `ForumApi` del decorador: los temas y el historial por tema se piden a la red y se guardan
//! (`forum_topics`, `message_topics`); sin red se sirve lo guardado.

use async_trait::async_trait;
use tc_core::{
    Anchor, BackendResult, ChatBackend, ForumApi, ForumTopic, Gif, GroupId, Message, MsgId, OutgoingMedia, OutgoingText, PeerRef, RandomId,
    StickerId, ThreadRef, Timestamp, TopicIcon, TopicsPage, TopicsQuery, sort_topics,
};

use super::{CachedBackend, is_transient};
use crate::convert as cv;
use crate::db::Store;
use crate::error::Result as StoreResult;
use crate::forum::TopicRow;

fn topic_row(t: &ForumTopic) -> TopicRow {
    TopicRow {
        group_id: t.group.0,
        topic_id: i64::from(t.id.0),
        title: t.title.clone(),
        icon_color: i64::from(t.icon.color),
        icon_emoji: t.icon.emoji.clone(),
        custom_emoji_id: t.icon.custom_emoji_id,
        created_at: t.date.0,
        last_msg_id: t.last_message.as_ref().map(|m| i64::from(m.id.0)),
        activity: t.activity().0,
        unread_count: i64::from(t.unread_count),
        unread_mentions: i64::from(t.unread_mentions),
        read_inbox_max_id: i64::from(t.read_inbox_max_id.0),
        read_outbox_max_id: i64::from(t.read_outbox_max_id.0),
        pinned: t.pinned,
        closed: t.closed,
        hidden: t.hidden,
        my: t.my,
    }
}

fn load_topic(s: &Store, r: TopicRow) -> StoreResult<ForumTopic> {
    let peer = PeerRef::Group(GroupId(r.group_id));
    let last = match (r.last_msg_id, cv::find_chat_key(s, peer)?) {
        (Some(id), Some(ck)) => s.get_message(ck, id)?.map(|row| cv::load_message(s, peer, &row)).transpose()?,
        _ => None,
    };
    let id = |v: i64| MsgId(i32::try_from(v).unwrap_or(0));
    Ok(ForumTopic {
        group: GroupId(r.group_id),
        id: id(r.topic_id),
        title: r.title,
        icon: TopicIcon { color: u32::try_from(r.icon_color).unwrap_or(0), emoji: r.icon_emoji, custom_emoji_id: r.custom_emoji_id },
        date: Timestamp(r.created_at),
        last_message: last,
        unread_count: u32::try_from(r.unread_count).unwrap_or(0),
        unread_mentions: u32::try_from(r.unread_mentions).unwrap_or(0),
        read_inbox_max_id: id(r.read_inbox_max_id),
        read_outbox_max_id: id(r.read_outbox_max_id),
        pinned: r.pinned,
        closed: r.closed,
        hidden: r.hidden,
        my: r.my,
    })
}

/// Guarda mensajes de un tema y su etiqueta.
pub(crate) fn persist_topic_messages(s: &Store, thread: ThreadRef, msgs: &[Message]) -> StoreResult<()> {
    for m in msgs.iter().filter(|m| m.peer == thread.peer()) {
        cv::persist_message(s, m, None)?;
        let ck = cv::ensure_dialog(s, m.peer)?;
        s.set_message_topic(ck, i64::from(m.id.0), i64::from(thread.topic.0))?;
        if m.id.0 > 0 {
            s.bump_topic(thread.group.0, i64::from(thread.topic.0), i64::from(m.id.0), m.date.0)?;
        }
    }
    Ok(())
}

/// Guarda un tema (y su último mensaje, etiquetado).
pub(crate) fn persist_topic(s: &Store, t: &ForumTopic) -> StoreResult<()> {
    if let Some(m) = &t.last_message {
        persist_topic_messages(s, t.thread(), std::slice::from_ref(m))?;
    }
    s.upsert_topic(&topic_row(t))
}

impl<B: ChatBackend> CachedBackend<B> {
    /// Temas guardados de un foro (ordenados), sin pedir nada a la red.
    pub async fn cached_topics(&self, group: GroupId) -> BackendResult<Vec<ForumTopic>> {
        self.shared.db(move |s| s.topics(group.0)?.into_iter().map(|r| load_topic(s, r)).collect()).await
    }

    async fn keep_topic_msgs(&self, thread: ThreadRef, msgs: Vec<Message>) {
        if let Err(e) = self.shared.db(move |s| persist_topic_messages(s, thread, &msgs)).await {
            tracing::warn!(error = %e, "no se pudieron guardar los mensajes del tema");
        }
    }
}

#[async_trait]
impl<B: ChatBackend> ForumApi for CachedBackend<B> {
    async fn forum_topics(&self, group: GroupId, query: TopicsQuery) -> BackendResult<TopicsPage> {
        let complete = query.offset.is_none() && query.query.is_none();
        match self.shared.inner.forum_topics(group, query.clone()).await {
            Ok(page) => {
                let keep = page.clone();
                let all_in_one = complete && page.next.is_none();
                let r = self
                    .shared
                    .db(move |s| {
                        s.set_group_forum(group.0, true)?;
                        keep.topics.iter().try_for_each(|t| persist_topic(s, t))?;
                        if all_in_one {
                            let ids: Vec<i64> = keep.topics.iter().map(|t| i64::from(t.id.0)).collect();
                            s.retain_topics(group.0, &ids)?;
                        }
                        Ok(())
                    })
                    .await;
                if let Err(e) = r {
                    tracing::warn!(error = %e, "no se pudieron guardar los temas");
                }
                Ok(page)
            }
            Err(e) if is_transient(&e) && query.offset.is_none() => {
                let mut cached = self.cached_topics(group).await?;
                if let Some(q) = query.query.as_deref().map(str::to_lowercase) {
                    cached.retain(|t| t.title.to_lowercase().contains(q.trim()));
                }
                if cached.is_empty() {
                    return Err(e);
                }
                sort_topics(&mut cached);
                Ok(TopicsPage { total: cached.len() as u32, topics: cached, next: None })
            }
            Err(e) => Err(e),
        }
    }

    async fn forum_topics_by_id(&self, group: GroupId, ids: Vec<MsgId>) -> BackendResult<Vec<ForumTopic>> {
        let v = self.shared.inner.forum_topics_by_id(group, ids).await?;
        let keep = v.clone();
        if let Err(e) = self.shared.db(move |s| keep.iter().try_for_each(|t| persist_topic(s, t))).await {
            tracing::warn!(error = %e, "no se pudieron guardar los temas");
        }
        Ok(v)
    }

    async fn topic_history(&self, thread: ThreadRef, anchor: Anchor, limit: usize) -> BackendResult<Vec<Message>> {
        match self.shared.inner.topic_history(thread, anchor, limit).await {
            Ok(v) => {
                self.keep_topic_msgs(thread, v.clone()).await;
                Ok(v)
            }
            Err(e) if is_transient(&e) => {
                let peer = thread.peer();
                let cached = self
                    .shared
                    .db(move |s| {
                        let Some(ck) = cv::find_chat_key(s, peer)? else { return Ok(Vec::new()) };
                        let mut out = Vec::new();
                        for id in s.topic_message_ids(ck, i64::from(thread.topic.0), anchor, limit)? {
                            if let Some(row) = s.get_message(ck, id)? {
                                out.push(cv::load_message(s, peer, &row)?);
                            }
                        }
                        Ok(out)
                    })
                    .await?;
                if cached.is_empty() { Err(e) } else { Ok(cached) }
            }
            Err(e) => Err(e),
        }
    }

    async fn send_topic_text(&self, thread: ThreadRef, msg: OutgoingText) -> BackendResult<Message> {
        let m = self.shared.inner.send_topic_text(thread, msg).await?;
        self.keep_topic_msgs(thread, vec![m.clone()]).await;
        Ok(m)
    }

    async fn send_topic_media(&self, thread: ThreadRef, media: OutgoingMedia) -> BackendResult<Vec<Message>> {
        let v = self.shared.inner.send_topic_media(thread, media).await?;
        self.keep_topic_msgs(thread, v.clone()).await;
        Ok(v)
    }

    async fn send_topic_sticker(&self, thread: ThreadRef, sticker: StickerId, reply_to: Option<MsgId>, random_id: RandomId) -> BackendResult<Message> {
        let m = self.shared.inner.send_topic_sticker(thread, sticker, reply_to, random_id).await?;
        self.keep_topic_msgs(thread, vec![m.clone()]).await;
        Ok(m)
    }

    async fn send_topic_gif(&self, thread: ThreadRef, gif: Gif, reply_to: Option<MsgId>, random_id: RandomId) -> BackendResult<Message> {
        let m = self.shared.inner.send_topic_gif(thread, gif, reply_to, random_id).await?;
        self.keep_topic_msgs(thread, vec![m.clone()]).await;
        Ok(m)
    }

    async fn mark_topic_read(&self, thread: ThreadRef, max_id: MsgId) -> BackendResult<()> {
        self.shared.inner.mark_topic_read(thread, max_id).await?;
        self.shared.db(move |s| s.mark_topic_read(thread.group.0, i64::from(thread.topic.0), i64::from(max_id.0))).await
    }

    async fn set_topic_typing(&self, thread: ThreadRef, typing: bool) -> BackendResult<()> {
        self.shared.inner.set_topic_typing(thread, typing).await
    }

    async fn create_topic(&self, group: GroupId, title: String, icon_color: Option<u32>, random_id: RandomId) -> BackendResult<ForumTopic> {
        let t = self.shared.inner.create_topic(group, title, icon_color, random_id).await?;
        let keep = t.clone();
        if let Err(e) = self.shared.db(move |s| persist_topic(s, &keep)).await {
            tracing::warn!(error = %e, "no se pudo guardar el tema creado");
        }
        Ok(t)
    }

    async fn set_topic_pinned(&self, thread: ThreadRef, pinned: bool) -> BackendResult<()> {
        self.shared.inner.set_topic_pinned(thread, pinned).await?;
        self.shared.db(move |s| s.set_topic_flags(thread.group.0, i64::from(thread.topic.0), Some(pinned), None)).await
    }

    async fn set_topic_closed(&self, thread: ThreadRef, closed: bool) -> BackendResult<()> {
        self.shared.inner.set_topic_closed(thread, closed).await?;
        self.shared.db(move |s| s.set_topic_flags(thread.group.0, i64::from(thread.topic.0), None, Some(closed))).await
    }
}

#[cfg(test)]
mod tests {
    use futures::StreamExt;
    use tc_core::*;
    use tc_mock::{FORUM_GROUP, MockBackend, MockConfig};

    use crate::{CachedBackend, Store};

    fn cached() -> CachedBackend<MockBackend> {
        CachedBackend::new(MockBackend::new(MockConfig { now: Some(1_800_000_000), ..Default::default() }), Store::open_in_memory().unwrap())
    }

    #[tokio::test]
    async fn temas_e_historial_por_tema_se_sirven_sin_red() {
        let c = cached();
        assert!(c.group(FORUM_GROUP).await.unwrap().is_forum);
        let page = c.forum_topics(FORUM_GROUP, TopicsQuery::first(50)).await.unwrap();
        let busy = page.topics.iter().max_by_key(|t| t.unread_count).unwrap().clone();
        let h = c.topic_history(busy.thread(), Anchor::Latest, 30).await.unwrap();
        let sent = c
            .send_topic_text(busy.thread(), OutgoingText { text: "riego a las 7".into(), entities: vec![], reply_to: None, random_id: RandomId(77) })
            .await
            .unwrap();
        c.mark_topic_read(busy.thread(), sent.id).await.unwrap();

        c.inner().set_offline(true);
        let off = c.forum_topics(FORUM_GROUP, TopicsQuery::first(50)).await.unwrap();
        assert_eq!(off.topics.len(), page.topics.len());
        let t = off.topics.iter().find(|t| t.id == busy.id).unwrap();
        assert_eq!(t.unread_count, 0, "lectura guardada");
        assert_eq!(t.last_message.as_ref().map(|m| m.id), Some(sent.id));
        let cached_h = c.topic_history(busy.thread(), Anchor::Latest, 100).await.unwrap();
        assert_eq!(cached_h.len(), h.len() + 1);
        assert!(cached_h.iter().all(|m| m.peer == busy.thread().peer()));
        let q = c.forum_topics(FORUM_GROUP, TopicsQuery { query: Some("semillas".into()), offset: None, limit: 10 }).await.unwrap();
        assert_eq!(q.topics.len(), 1);
        assert!(c.group(FORUM_GROUP).await.unwrap().is_forum, "la marca de foro se guarda");
    }

    #[tokio::test]
    async fn los_eventos_de_tema_se_guardan() {
        let c = cached();
        let general = ThreadRef { group: FORUM_GROUP, topic: GENERAL_TOPIC };
        let mut ev = c.events();
        let m = c.inner().inject_topic_incoming(general, "hola foro").unwrap();
        loop {
            if let Some(BackendEvent::ForumTopicUpserted(t)) = ev.next().await {
                assert!(t.is_general());
                break;
            }
        }
        c.inner().set_offline(true);
        let h = c.topic_history(general, Anchor::Latest, 5).await.unwrap();
        assert_eq!(h.last().map(|x| x.id), Some(m.id));
    }
}
