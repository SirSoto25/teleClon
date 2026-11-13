//! Grupos con temas (migración 0008): marca de foro del grupo, última lista de temas de cada foro y
//! a qué tema pertenece cada mensaje cacheado. El decorador está en `cached/forum.rs`.

use crate::db::Store;
use crate::error::Result;
use rusqlite::{OptionalExtension, Row, params};
use tc_core::Anchor;

/// Fila de `forum_topics`.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct TopicRow {
    pub group_id: i64,
    pub topic_id: i64,
    pub title: String,
    pub icon_color: i64,
    pub icon_emoji: Option<String>,
    pub custom_emoji_id: Option<i64>,
    pub created_at: i64,
    pub last_msg_id: Option<i64>,
    pub activity: i64,
    pub unread_count: i64,
    pub unread_mentions: i64,
    pub read_inbox_max_id: i64,
    pub read_outbox_max_id: i64,
    pub pinned: bool,
    pub closed: bool,
    pub hidden: bool,
    pub my: bool,
}

const COLS: &str = "group_id, topic_id, title, icon_color, icon_emoji, custom_emoji_id, created_at, last_msg_id, activity, unread_count, \
                    unread_mentions, read_inbox_max_id, read_outbox_max_id, pinned, closed, hidden, my";

fn from_row(r: &Row<'_>) -> rusqlite::Result<TopicRow> {
    Ok(TopicRow {
        group_id: r.get(0)?,
        topic_id: r.get(1)?,
        title: r.get(2)?,
        icon_color: r.get(3)?,
        icon_emoji: r.get(4)?,
        custom_emoji_id: r.get(5)?,
        created_at: r.get(6)?,
        last_msg_id: r.get(7)?,
        activity: r.get(8)?,
        unread_count: r.get(9)?,
        unread_mentions: r.get(10)?,
        read_inbox_max_id: r.get(11)?,
        read_outbox_max_id: r.get(12)?,
        pinned: r.get(13)?,
        closed: r.get(14)?,
        hidden: r.get(15)?,
        my: r.get(16)?,
    })
}

impl Store {
    /// Marca (o desmarca) el grupo como foro. Sin fila del grupo no hace nada.
    pub fn set_group_forum(&self, group_id: i64, forum: bool) -> Result<()> {
        self.conn.execute("UPDATE groups SET is_forum=?2 WHERE id=?1", params![group_id, forum])?;
        Ok(())
    }

    pub fn group_is_forum(&self, group_id: i64) -> Result<bool> {
        Ok(self.conn.query_row("SELECT is_forum FROM groups WHERE id=?1", [group_id], |r| r.get(0)).optional()?.unwrap_or(false))
    }

    /// Inserta o actualiza un tema (idempotente por `(group_id, topic_id)`).
    pub fn upsert_topic(&self, t: &TopicRow) -> Result<()> {
        self.conn.execute(
            &format!(
                "INSERT INTO forum_topics ({COLS}) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15,?16,?17)
                 ON CONFLICT(group_id, topic_id) DO UPDATE SET title=excluded.title, icon_color=excluded.icon_color,
                   icon_emoji=excluded.icon_emoji, custom_emoji_id=excluded.custom_emoji_id, created_at=excluded.created_at,
                   last_msg_id=COALESCE(excluded.last_msg_id, forum_topics.last_msg_id), activity=MAX(excluded.activity, forum_topics.activity),
                   unread_count=excluded.unread_count, unread_mentions=excluded.unread_mentions,
                   read_inbox_max_id=MAX(excluded.read_inbox_max_id, forum_topics.read_inbox_max_id),
                   read_outbox_max_id=MAX(excluded.read_outbox_max_id, forum_topics.read_outbox_max_id),
                   pinned=excluded.pinned, closed=excluded.closed, hidden=excluded.hidden, my=excluded.my"
            ),
            params![
                t.group_id,
                t.topic_id,
                t.title,
                t.icon_color,
                t.icon_emoji,
                t.custom_emoji_id,
                t.created_at,
                t.last_msg_id,
                t.activity,
                t.unread_count,
                t.unread_mentions,
                t.read_inbox_max_id,
                t.read_outbox_max_id,
                t.pinned,
                t.closed,
                t.hidden,
                t.my
            ],
        )?;
        Ok(())
    }

    pub fn get_topic(&self, group_id: i64, topic_id: i64) -> Result<Option<TopicRow>> {
        Ok(self
            .conn
            .query_row(&format!("SELECT {COLS} FROM forum_topics WHERE group_id=?1 AND topic_id=?2"), params![group_id, topic_id], from_row)
            .optional()?)
    }

    /// Temas de un foro: fijados primero, luego por actividad.
    pub fn topics(&self, group_id: i64) -> Result<Vec<TopicRow>> {
        let mut st = self.conn.prepare(&format!("SELECT {COLS} FROM forum_topics WHERE group_id=?1 ORDER BY pinned DESC, activity DESC, topic_id DESC"))?;
        let rows = st.query_map([group_id], from_row)?;
        Ok(rows.collect::<rusqlite::Result<_>>()?)
    }

    /// Quita los temas del foro que ya no están en `keep` (tras recibir la lista completa).
    pub fn retain_topics(&self, group_id: i64, keep: &[i64]) -> Result<usize> {
        let current: Vec<i64> = self.topics(group_id)?.into_iter().map(|t| t.topic_id).collect();
        let mut n = 0;
        for id in current.into_iter().filter(|id| !keep.contains(id)) {
            n += self.conn.execute("DELETE FROM forum_topics WHERE group_id=?1 AND topic_id=?2", params![group_id, id])?;
        }
        Ok(n)
    }

    /// Lectura del tema hasta `max_id`; si llega al último mensaje, sin no leídos.
    pub fn mark_topic_read(&self, group_id: i64, topic_id: i64, max_id: i64) -> Result<()> {
        self.conn.execute(
            "UPDATE forum_topics SET read_inbox_max_id=MAX(read_inbox_max_id, ?3),
               unread_count=CASE WHEN last_msg_id IS NULL OR last_msg_id<=?3 THEN 0 ELSE unread_count END
             WHERE group_id=?1 AND topic_id=?2",
            params![group_id, topic_id, max_id],
        )?;
        Ok(())
    }

    /// Un mensaje más nuevo del tema pasa a ser el último (si el tema está guardado).
    pub fn bump_topic(&self, group_id: i64, topic_id: i64, msg_id: i64, date: i64) -> Result<()> {
        self.conn.execute(
            "UPDATE forum_topics SET last_msg_id=MAX(COALESCE(last_msg_id, 0), ?3), activity=MAX(activity, ?4)
             WHERE group_id=?1 AND topic_id=?2",
            params![group_id, topic_id, msg_id, date],
        )?;
        Ok(())
    }

    pub fn set_topic_flags(&self, group_id: i64, topic_id: i64, pinned: Option<bool>, closed: Option<bool>) -> Result<()> {
        self.conn.execute(
            "UPDATE forum_topics SET pinned=COALESCE(?3, pinned), closed=COALESCE(?4, closed) WHERE group_id=?1 AND topic_id=?2",
            params![group_id, topic_id, pinned, closed],
        )?;
        Ok(())
    }

    /// Anota el tema de un mensaje ya guardado (sin el mensaje no hace nada).
    pub fn set_message_topic(&self, chat_key: i64, msg_id: i64, topic_id: i64) -> Result<()> {
        self.conn.execute(
            "INSERT INTO message_topics (chat_key, msg_id, topic_id)
             SELECT ?1, ?2, ?3 WHERE EXISTS (SELECT 1 FROM messages WHERE chat_key=?1 AND msg_id=?2)
             ON CONFLICT(chat_key, msg_id) DO UPDATE SET topic_id=excluded.topic_id",
            params![chat_key, msg_id, topic_id],
        )?;
        Ok(())
    }

    pub fn message_topic(&self, chat_key: i64, msg_id: i64) -> Result<Option<i64>> {
        Ok(self
            .conn
            .query_row("SELECT topic_id FROM message_topics WHERE chat_key=?1 AND msg_id=?2", params![chat_key, msg_id], |r| r.get(0))
            .optional()?)
    }

    /// Ids (ascendentes) de una página del historial cacheado de un tema; solo confirmados.
    pub fn topic_message_ids(&self, chat_key: i64, topic_id: i64, anchor: Anchor, limit: usize) -> Result<Vec<i64>> {
        let lim = limit as i64;
        let run = |cond: &str, order: &str, a: i64, n: i64| -> Result<Vec<i64>> {
            let sql = format!("SELECT msg_id FROM message_topics WHERE chat_key=?1 AND topic_id=?2 AND msg_id>0 {cond} ORDER BY msg_id {order} LIMIT ?4");
            let mut st = self.conn.prepare(&sql)?;
            let rows = st.query_map(params![chat_key, topic_id, a, n], |r| r.get(0))?;
            Ok(rows.collect::<rusqlite::Result<_>>()?)
        };
        Ok(match anchor {
            Anchor::Latest => {
                let mut v = run("AND msg_id>=?3", "DESC", i64::MIN, lim)?;
                v.reverse();
                v
            }
            Anchor::Before(id) => {
                let mut v = run("AND msg_id<?3", "DESC", i64::from(id.0), lim)?;
                v.reverse();
                v
            }
            Anchor::After(id) => run("AND msg_id>?3", "ASC", i64::from(id.0), lim)?,
            Anchor::Around(id) => {
                let mut v = run("AND msg_id<=?3", "DESC", i64::from(id.0), (lim + 1) / 2)?;
                v.reverse();
                v.extend(run("AND msg_id>?3", "ASC", i64::from(id.0), lim / 2)?);
                v
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{GroupKindRow, GroupRow, MessageRow};
    use tc_core::MsgId;

    #[test]
    fn temas_marca_de_foro_y_mensajes_por_tema() {
        let s = Store::open_in_memory().unwrap();
        assert!(s.schema_version().unwrap() >= 8);
        s.upsert_group(&GroupRow::new(9, GroupKindRow::Supergroup, "Huertos", 1)).unwrap();
        assert!(!s.group_is_forum(9).unwrap());
        s.set_group_forum(9, true).unwrap();
        assert!(s.group_is_forum(9).unwrap());

        let mut t = TopicRow { group_id: 9, topic_id: 1, title: "General".into(), activity: 10, unread_count: 2, last_msg_id: Some(30), ..Default::default() };
        s.upsert_topic(&t).unwrap();
        t.topic_id = 5;
        t.title = "Riego".into();
        t.pinned = true;
        t.activity = 5;
        s.upsert_topic(&t).unwrap();
        let ids: Vec<i64> = s.topics(9).unwrap().iter().map(|t| t.topic_id).collect();
        assert_eq!(ids, [5, 1], "fijado primero");
        s.mark_topic_read(9, 1, 30).unwrap();
        assert_eq!(s.get_topic(9, 1).unwrap().unwrap().unread_count, 0);
        s.set_topic_flags(9, 5, Some(false), Some(true)).unwrap();
        let r = s.get_topic(9, 5).unwrap().unwrap();
        assert!(!r.pinned && r.closed);
        assert_eq!(s.retain_topics(9, &[1]).unwrap(), 1);

        let ck = crate::convert::ensure_dialog(&s, tc_core::PeerRef::Group(tc_core::GroupId(9))).unwrap();
        for id in 1..=6 {
            s.upsert_message(&MessageRow::text(ck, id, id * 10, false, "x")).unwrap();
            s.set_message_topic(ck, id, if id % 2 == 0 { 2 } else { 1 }).unwrap();
        }
        s.set_message_topic(ck, 99, 1).unwrap();
        assert_eq!(s.message_topic(ck, 99).unwrap(), None, "sin mensaje no se anota");
        assert_eq!(s.topic_message_ids(ck, 2, Anchor::Latest, 10).unwrap(), [2, 4, 6]);
        assert_eq!(s.topic_message_ids(ck, 1, Anchor::Before(MsgId(5)), 1).unwrap(), [3]);
        assert_eq!(s.topic_message_ids(ck, 1, Anchor::Around(MsgId(3)), 3).unwrap(), [1, 3, 5]);
        s.delete_messages(ck, &[4]).unwrap();
        assert_eq!(s.topic_message_ids(ck, 2, Anchor::After(MsgId(0)), 10).unwrap(), [2, 6], "en cascada");
    }
}
