//! Reacciones agregadas por mensaje (recuentos de la red + cuál es la mía).

use crate::db::Store;
use crate::error::Result;
use crate::types::*;
use rusqlite::params;
use std::collections::HashMap;

impl Store {
    /// Sustituye las reacciones de un mensaje. Devuelve `false` (sin cambios) si el mensaje no está
    /// en la caché. Los recuentos a cero se descartan.
    pub fn set_reactions(&self, chat_key: i64, msg_id: i64, reactions: &[ReactionRow]) -> Result<bool> {
        let tx = self.conn.unchecked_transaction()?;
        if !message_exists(&tx, chat_key, msg_id)? {
            return Ok(false);
        }
        tx.execute("DELETE FROM reactions WHERE chat_key=?1 AND msg_id=?2", params![chat_key, msg_id])?;
        for r in reactions.iter().filter(|r| r.count > 0) {
            tx.execute(
                "INSERT INTO reactions (chat_key, msg_id, emoji, count, chosen) VALUES (?1,?2,?3,?4,?5)
                 ON CONFLICT(chat_key, msg_id, emoji) DO UPDATE SET count=excluded.count, chosen=excluded.chosen",
                params![chat_key, msg_id, r.emoji, r.count, r.chosen],
            )?;
        }
        tx.commit()?;
        Ok(true)
    }

    /// Reacciones de un mensaje, por recuento descendente y luego por emoji.
    pub fn get_reactions(&self, chat_key: i64, msg_id: i64) -> Result<Vec<ReactionRow>> {
        let mut st = self.conn.prepare(
            "SELECT emoji, count, chosen FROM reactions WHERE chat_key=?1 AND msg_id=?2 ORDER BY count DESC, emoji",
        )?;
        let rows = st.query_map(params![chat_key, msg_id], |r| {
            Ok(ReactionRow { emoji: r.get(0)?, count: r.get(1)?, chosen: r.get(2)? })
        })?;
        Ok(rows.collect::<rusqlite::Result<_>>()?)
    }

    /// Reacciones de varios mensajes de un chat de una vez.
    pub fn reactions_for(&self, chat_key: i64, msg_ids: &[i64]) -> Result<HashMap<i64, Vec<ReactionRow>>> {
        let mut out: HashMap<i64, Vec<ReactionRow>> = HashMap::new();
        let mut st = self.conn.prepare(
            "SELECT emoji, count, chosen FROM reactions WHERE chat_key=?1 AND msg_id=?2 ORDER BY count DESC, emoji",
        )?;
        for &id in msg_ids {
            let rows = st
                .query_map(params![chat_key, id], |r| Ok(ReactionRow { emoji: r.get(0)?, count: r.get(1)?, chosen: r.get(2)? }))?
                .collect::<rusqlite::Result<Vec<_>>>()?;
            if !rows.is_empty() {
                out.insert(id, rows);
            }
        }
        Ok(out)
    }

    /// Aplica localmente mi reacción: quita la anterior (si la había) y pone `emoji` (o ninguna).
    /// Devuelve `false` si el mensaje no está en la caché.
    pub fn apply_my_reaction(&self, chat_key: i64, msg_id: i64, emoji: Option<&str>) -> Result<bool> {
        let tx = self.conn.unchecked_transaction()?;
        if !message_exists(&tx, chat_key, msg_id)? {
            return Ok(false);
        }
        tx.execute(
            "UPDATE reactions SET count=count-1, chosen=0 WHERE chat_key=?1 AND msg_id=?2 AND chosen=1",
            params![chat_key, msg_id],
        )?;
        if let Some(e) = emoji {
            tx.execute(
                "INSERT INTO reactions (chat_key, msg_id, emoji, count, chosen) VALUES (?1,?2,?3,1,1)
                 ON CONFLICT(chat_key, msg_id, emoji) DO UPDATE SET count=count+1, chosen=1",
                params![chat_key, msg_id, e],
            )?;
        }
        tx.execute("DELETE FROM reactions WHERE chat_key=?1 AND msg_id=?2 AND count<=0", params![chat_key, msg_id])?;
        tx.commit()?;
        Ok(true)
    }
}

fn message_exists(c: &rusqlite::Connection, chat_key: i64, msg_id: i64) -> Result<bool> {
    Ok(c.query_row("SELECT EXISTS(SELECT 1 FROM messages WHERE chat_key=?1 AND msg_id=?2)", params![chat_key, msg_id], |r| r.get(0))?)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn setup() -> (Store, i64) {
        let s = Store::open_in_memory().unwrap();
        let k = s.upsert_dialog(&DialogRow::new(PeerKind::User, 1, 0)).unwrap();
        s.upsert_messages(&[MessageRow::text(k, 1, 1, false, "a"), MessageRow::text(k, 2, 2, true, "b")]).unwrap();
        (s, k)
    }

    fn r(e: &str, count: i64, chosen: bool) -> ReactionRow {
        ReactionRow { emoji: e.into(), count, chosen }
    }

    #[test]
    fn set_get_replace_and_unknown_message() {
        let (s, k) = setup();
        assert!(s.set_reactions(k, 1, &[r("👍", 3, true), r("🔥", 5, false), r("💤", 0, false)]).unwrap());
        assert_eq!(s.get_reactions(k, 1).unwrap(), [r("🔥", 5, false), r("👍", 3, true)]);
        assert!(s.set_reactions(k, 1, &[r("❤", 1, false)]).unwrap());
        assert_eq!(s.get_reactions(k, 1).unwrap(), [r("❤", 1, false)]);
        assert!(!s.set_reactions(k, 99, &[r("👍", 1, true)]).unwrap());
        let m = s.reactions_for(k, &[1, 2]).unwrap();
        assert_eq!(m.len(), 1);
        assert!(m.contains_key(&1));
        s.delete_messages(k, &[1]).unwrap();
        assert!(s.get_reactions(k, 1).unwrap().is_empty(), "cascada al borrar el mensaje");
    }

    #[test]
    fn my_reaction_moves_between_emojis() {
        let (s, k) = setup();
        s.set_reactions(k, 2, &[r("👍", 2, false)]).unwrap();
        assert!(s.apply_my_reaction(k, 2, Some("👍")).unwrap());
        assert_eq!(s.get_reactions(k, 2).unwrap(), [r("👍", 3, true)]);
        assert!(s.apply_my_reaction(k, 2, Some("🔥")).unwrap());
        assert_eq!(s.get_reactions(k, 2).unwrap(), [r("👍", 2, false), r("🔥", 1, true)]);
        assert!(s.apply_my_reaction(k, 2, None).unwrap());
        assert_eq!(s.get_reactions(k, 2).unwrap(), [r("👍", 2, false)]);
        assert!(!s.apply_my_reaction(k, 99, Some("👍")).unwrap());
    }
}
