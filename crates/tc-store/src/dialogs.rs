//! Usuarios, diálogos, borradores y ajustes de notificación.

use crate::db::Store;
use crate::error::{Error, Result};
use crate::types::*;
use rusqlite::{OptionalExtension, Row, params};

const DIALOG_COLS: &str = "chat_key, peer_kind, peer_id, top_message_id, last_message_at, unread_count, \
    unread_mentions, unread_reactions, read_inbox_max_id, read_outbox_max_id, pinned_order, archived, \
    mute_until, pinned_message_id, history_complete, updated_at";

fn dialog_from_row(r: &Row<'_>) -> rusqlite::Result<DialogRow> {
    Ok(DialogRow {
        chat_key: r.get(0)?,
        peer_kind: r.get(1)?,
        peer_id: r.get(2)?,
        top_message_id: r.get(3)?,
        last_message_at: r.get(4)?,
        unread_count: r.get(5)?,
        unread_mentions: r.get(6)?,
        unread_reactions: r.get(7)?,
        read_inbox_max_id: r.get(8)?,
        read_outbox_max_id: r.get(9)?,
        pinned_order: r.get(10)?,
        archived: r.get(11)?,
        mute_until: r.get(12)?,
        pinned_message_id: r.get(13)?,
        history_complete: r.get(14)?,
        updated_at: r.get(15)?,
    })
}

fn draft_from_row(r: &Row<'_>, base: usize) -> rusqlite::Result<Draft> {
    Ok(Draft {
        text: r.get(base)?,
        entities: r.get(base + 1)?,
        reply_to: r.get(base + 2)?,
        date: r.get::<_, Option<i64>>(base + 3)?.unwrap_or(0),
    })
}

impl Store {
    // ---- usuarios -------------------------------------------------------------------------

    /// Inserta o actualiza un usuario (idempotente). No toca `is_blocked`, estado ni foto.
    pub fn upsert_user(&self, u: &UserRow) -> Result<()> {
        self.conn.execute(
            "INSERT INTO users (id, access_hash, first_name, last_name, username, phone, is_self, is_contact, updated_at)
             VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9)
             ON CONFLICT(id) DO UPDATE SET access_hash=excluded.access_hash, first_name=excluded.first_name,
               last_name=excluded.last_name, username=excluded.username, phone=excluded.phone,
               is_self=excluded.is_self, is_contact=excluded.is_contact, updated_at=excluded.updated_at",
            params![u.id, u.access_hash, u.first_name, u.last_name, u.username, u.phone, u.is_self, u.is_contact, u.updated_at],
        )?;
        Ok(())
    }

    /// Usuario por id.
    pub fn get_user(&self, id: i64) -> Result<Option<UserRow>> {
        Ok(self
            .conn
            .query_row(
                "SELECT id, access_hash, first_name, last_name, username, phone, is_self, is_contact, updated_at
                 FROM users WHERE id=?1",
                [id],
                |r| {
                    Ok(UserRow {
                        id: r.get(0)?,
                        access_hash: r.get(1)?,
                        first_name: r.get(2)?,
                        last_name: r.get(3)?,
                        username: r.get(4)?,
                        phone: r.get(5)?,
                        is_self: r.get(6)?,
                        is_contact: r.get(7)?,
                        updated_at: r.get(8)?,
                    })
                },
            )
            .optional()?)
    }

    // ---- diálogos -------------------------------------------------------------------------

    /// Inserta o actualiza un diálogo por `(peer_kind, peer_id)` y devuelve su `chat_key`.
    /// No modifica el borrador.
    pub fn upsert_dialog(&self, d: &DialogRow) -> Result<i64> {
        Ok(self.conn.query_row(
            "INSERT INTO dialogs (peer_kind, peer_id, top_message_id, last_message_at, unread_count, unread_mentions,
               unread_reactions, read_inbox_max_id, read_outbox_max_id, pinned_order, archived, mute_until,
               pinned_message_id, history_complete, updated_at)
             VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15)
             ON CONFLICT(peer_kind, peer_id) DO UPDATE SET top_message_id=excluded.top_message_id,
               last_message_at=excluded.last_message_at, unread_count=excluded.unread_count,
               unread_mentions=excluded.unread_mentions, unread_reactions=excluded.unread_reactions,
               read_inbox_max_id=excluded.read_inbox_max_id, read_outbox_max_id=excluded.read_outbox_max_id,
               pinned_order=excluded.pinned_order, archived=excluded.archived, mute_until=excluded.mute_until,
               pinned_message_id=excluded.pinned_message_id, history_complete=excluded.history_complete,
               updated_at=excluded.updated_at
             RETURNING chat_key",
            params![
                d.peer_kind, d.peer_id, d.top_message_id, d.last_message_at, d.unread_count, d.unread_mentions,
                d.unread_reactions, d.read_inbox_max_id, d.read_outbox_max_id, d.pinned_order, d.archived,
                d.mute_until, d.pinned_message_id, d.history_complete, d.updated_at
            ],
            |r| r.get(0),
        )?)
    }

    /// Diálogo por `chat_key`.
    pub fn get_dialog(&self, chat_key: i64) -> Result<Option<DialogRow>> {
        Ok(self
            .conn
            .query_row(&format!("SELECT {DIALOG_COLS} FROM dialogs WHERE chat_key=?1"), [chat_key], dialog_from_row)
            .optional()?)
    }

    /// Diálogo por interlocutor.
    pub fn find_dialog(&self, kind: PeerKind, peer_id: i64) -> Result<Option<DialogRow>> {
        Ok(self
            .conn
            .query_row(
                &format!("SELECT {DIALOG_COLS} FROM dialogs WHERE peer_kind=?1 AND peer_id=?2"),
                params![kind, peer_id],
                dialog_from_row,
            )
            .optional()?)
    }

    /// Lista de chats: fijados primero (por `pinned_order`), luego por actividad descendente.
    pub fn list_dialogs(&self, archived: bool, limit: usize) -> Result<Vec<DialogRow>> {
        let mut st = self.conn.prepare(&format!(
            "SELECT {DIALOG_COLS} FROM dialogs WHERE archived=?1
             ORDER BY pinned_order IS NULL, pinned_order, last_message_at DESC, chat_key DESC LIMIT ?2"
        ))?;
        let rows = st.query_map(params![archived, limit as i64], dialog_from_row)?;
        Ok(rows.collect::<rusqlite::Result<_>>()?)
    }

    /// Borra el diálogo y, en cascada, sus mensajes (y entradas del FTS).
    pub fn delete_dialog(&self, chat_key: i64) -> Result<bool> {
        Ok(self.conn.execute("DELETE FROM dialogs WHERE chat_key=?1", [chat_key])? > 0)
    }

    /// Fija (`Some(orden)`) o desfija (`None`) un diálogo.
    pub fn set_dialog_pinned(&self, chat_key: i64, order: Option<i64>) -> Result<()> {
        self.dialog_update("UPDATE dialogs SET pinned_order=?2 WHERE chat_key=?1", chat_key, order)
    }

    /// Archiva o desarchiva.
    pub fn set_dialog_archived(&self, chat_key: i64, archived: bool) -> Result<()> {
        self.dialog_update("UPDATE dialogs SET archived=?2 WHERE chat_key=?1", chat_key, archived)
    }

    /// Silencia hasta `until` (0 = no silenciado).
    pub fn set_dialog_mute(&self, chat_key: i64, until: i64) -> Result<()> {
        self.dialog_update("UPDATE dialogs SET mute_until=?2 WHERE chat_key=?1", chat_key, until)
    }

    /// Marca como leído hasta `max_id`: pone a cero los contadores y avanza `read_inbox_max_id`.
    pub fn mark_dialog_read(&self, chat_key: i64, max_id: i64) -> Result<()> {
        self.dialog_update(
            "UPDATE dialogs SET unread_count=0, unread_mentions=0, unread_reactions=0,
               read_inbox_max_id=MAX(read_inbox_max_id, ?2) WHERE chat_key=?1",
            chat_key,
            max_id,
        )
    }

    fn dialog_update(&self, sql: &str, chat_key: i64, v: impl rusqlite::ToSql) -> Result<()> {
        if self.conn.execute(sql, params![chat_key, v])? == 0 {
            return Err(Error::NotFound(format!("diálogo {chat_key}")));
        }
        Ok(())
    }

    // ---- borradores -----------------------------------------------------------------------

    /// Guarda el borrador de un chat; `None` o texto en blanco sin respuesta lo borra.
    pub fn set_draft(&self, chat_key: i64, draft: Option<&Draft>) -> Result<()> {
        let n = match draft.filter(|d| !d.text.trim().is_empty() || d.reply_to.is_some()) {
            Some(d) => self.conn.execute(
                "UPDATE dialogs SET draft_text=?2, draft_entities=?3, draft_reply_to=?4, draft_at=?5 WHERE chat_key=?1",
                params![chat_key, d.text, d.entities, d.reply_to, d.date],
            )?,
            None => self.conn.execute(
                "UPDATE dialogs SET draft_text=NULL, draft_entities=NULL, draft_reply_to=NULL, draft_at=NULL WHERE chat_key=?1",
                [chat_key],
            )?,
        };
        if n == 0 {
            return Err(Error::NotFound(format!("diálogo {chat_key}")));
        }
        Ok(())
    }

    /// Borrador de un chat, si hay.
    pub fn get_draft(&self, chat_key: i64) -> Result<Option<Draft>> {
        Ok(self
            .conn
            .query_row(
                "SELECT draft_text, draft_entities, draft_reply_to, draft_at FROM dialogs
                 WHERE chat_key=?1 AND draft_text IS NOT NULL",
                [chat_key],
                |r| draft_from_row(r, 0),
            )
            .optional()?)
    }

    /// Todos los borradores, el más reciente primero.
    pub fn list_drafts(&self) -> Result<Vec<(i64, Draft)>> {
        let mut st = self.conn.prepare(
            "SELECT chat_key, draft_text, draft_entities, draft_reply_to, draft_at FROM dialogs
             WHERE draft_text IS NOT NULL ORDER BY draft_at DESC, chat_key",
        )?;
        let rows = st.query_map([], |r| Ok((r.get(0)?, draft_from_row(r, 1)?)))?;
        Ok(rows.collect::<rusqlite::Result<_>>()?)
    }

    // ---- ajustes de notificación ----------------------------------------------------------

    /// Ajustes de notificación de un ámbito.
    pub fn get_notify(&self, scope: NotifyScope) -> Result<Option<NotifySettings>> {
        let (s, k) = scope_parts(scope)?;
        Ok(self
            .conn
            .query_row(
                "SELECT mute_until, show_previews, sound, updated_at FROM notify_settings WHERE scope=?1 AND chat_key=?2",
                params![s, k],
                |r| {
                    Ok(NotifySettings {
                        mute_until: r.get(0)?,
                        show_previews: r.get(1)?,
                        sound: r.get(2)?,
                        updated_at: r.get(3)?,
                    })
                },
            )
            .optional()?)
    }

    /// Guarda (upsert) los ajustes de notificación de un ámbito.
    pub fn set_notify(&self, scope: NotifyScope, n: &NotifySettings) -> Result<()> {
        let (s, k) = scope_parts(scope)?;
        self.conn.execute(
            "INSERT INTO notify_settings (scope, chat_key, mute_until, show_previews, sound, updated_at)
             VALUES (?1,?2,?3,?4,?5,?6)
             ON CONFLICT(scope, chat_key) DO UPDATE SET mute_until=excluded.mute_until,
               show_previews=excluded.show_previews, sound=excluded.sound, updated_at=excluded.updated_at",
            params![s, k, n.mute_until, n.show_previews, n.sound, n.updated_at],
        )?;
        Ok(())
    }
}

fn scope_parts(scope: NotifyScope) -> Result<(&'static str, i64)> {
    Ok(match scope {
        NotifyScope::Users => ("users", 0),
        NotifyScope::Groups => ("groups", 0),
        NotifyScope::Channels => ("channels", 0),
        NotifyScope::Chat(0) => return Err(Error::Invalid("chat_key 0 no es válido para un chat".into())),
        NotifyScope::Chat(k) => ("chat", k),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn store() -> Store {
        Store::open_in_memory().unwrap()
    }

    #[test]
    fn upsert_dialog_is_idempotent_and_keeps_draft() {
        let s = store();
        let mut d = DialogRow::new(PeerKind::User, 7, 100);
        let k1 = s.upsert_dialog(&d).unwrap();
        s.set_draft(k1, Some(&Draft { text: "hola".into(), entities: None, reply_to: None, date: 5 })).unwrap();
        d.unread_count = 3;
        let k2 = s.upsert_dialog(&d).unwrap();
        assert_eq!(k1, k2);
        assert_eq!(s.get_dialog(k1).unwrap().unwrap().unread_count, 3);
        assert_eq!(s.get_draft(k1).unwrap().unwrap().text, "hola");
        // mismo id en otro espacio de ids = otro diálogo
        assert_ne!(s.upsert_dialog(&DialogRow::new(PeerKind::Group, 7, 100)).unwrap(), k1);
    }

    #[test]
    fn list_orders_pinned_then_activity_and_filters_archived() {
        let s = store();
        let mk = |id, at, pin: Option<i64>, arch| {
            let mut d = DialogRow::new(PeerKind::User, id, at);
            d.last_message_at = Some(at);
            d.pinned_order = pin;
            d.archived = arch;
            s.upsert_dialog(&d).unwrap()
        };
        let _a = mk(1, 100, None, false);
        let b = mk(2, 300, None, false);
        let p2 = mk(3, 50, Some(2), false);
        let p1 = mk(4, 10, Some(1), false);
        let arch = mk(5, 999, None, true);
        let order: Vec<i64> = s.list_dialogs(false, 10).unwrap().iter().map(|d| d.chat_key).collect();
        assert_eq!(order[..3], [p1, p2, b]);
        assert_eq!(order.len(), 4);
        assert_eq!(s.list_dialogs(true, 10).unwrap()[0].chat_key, arch);
        s.set_dialog_archived(b, true).unwrap();
        assert_eq!(s.list_dialogs(true, 10).unwrap().len(), 2);
    }

    #[test]
    fn mark_read_and_missing_dialog_errors() {
        let s = store();
        let mut d = DialogRow::new(PeerKind::User, 1, 1);
        d.unread_count = 5;
        d.unread_mentions = 2;
        let k = s.upsert_dialog(&d).unwrap();
        s.mark_dialog_read(k, 42).unwrap();
        let d = s.get_dialog(k).unwrap().unwrap();
        assert_eq!((d.unread_count, d.unread_mentions, d.read_inbox_max_id), (0, 0, 42));
        s.mark_dialog_read(k, 10).unwrap();
        assert_eq!(s.get_dialog(k).unwrap().unwrap().read_inbox_max_id, 42);
        assert!(matches!(s.set_dialog_mute(999, 1), Err(Error::NotFound(_))));
    }

    #[test]
    fn drafts_set_clear_list() {
        let s = store();
        let a = s.upsert_dialog(&DialogRow::new(PeerKind::User, 1, 1)).unwrap();
        let b = s.upsert_dialog(&DialogRow::new(PeerKind::User, 2, 1)).unwrap();
        let dr = |t: &str, date| Draft { text: t.into(), entities: Some("[]".into()), reply_to: Some(9), date };
        s.set_draft(a, Some(&dr("uno", 10))).unwrap();
        s.set_draft(b, Some(&dr("dos", 20))).unwrap();
        let l = s.list_drafts().unwrap();
        assert_eq!(l.iter().map(|(k, _)| *k).collect::<Vec<_>>(), [b, a]);
        assert_eq!(l[0].1.reply_to, Some(9));
        s.set_draft(a, Some(&Draft { text: "  ".into(), entities: None, reply_to: None, date: 1 })).unwrap();
        assert!(s.get_draft(a).unwrap().is_none());
        s.set_draft(b, None).unwrap();
        assert!(s.list_drafts().unwrap().is_empty());
    }

    #[test]
    fn notify_scopes() {
        let s = store();
        let k = s.upsert_dialog(&DialogRow::new(PeerKind::User, 1, 1)).unwrap();
        let n = NotifySettings { mute_until: 0, show_previews: false, sound: Some("pop".into()), updated_at: 1 };
        s.set_notify(NotifyScope::Users, &n).unwrap();
        s.set_notify(NotifyScope::Chat(k), &NotifySettings { mute_until: 99, ..n.clone() }).unwrap();
        assert_eq!(s.get_notify(NotifyScope::Users).unwrap().unwrap(), n);
        assert_eq!(s.get_notify(NotifyScope::Chat(k)).unwrap().unwrap().mute_until, 99);
        assert!(s.get_notify(NotifyScope::Groups).unwrap().is_none());
        assert!(s.set_notify(NotifyScope::Chat(0), &n).is_err());
    }

    #[test]
    fn users_roundtrip() {
        let s = store();
        let u = UserRow { id: 5, first_name: "Ana".into(), username: Some("ana".into()), is_contact: true, updated_at: 3, ..Default::default() };
        s.upsert_user(&u).unwrap();
        s.upsert_user(&UserRow { last_name: "Ruiz".into(), ..u.clone() }).unwrap();
        assert_eq!(s.get_user(5).unwrap().unwrap().last_name, "Ruiz");
        assert!(s.get_user(6).unwrap().is_none());
    }
}
