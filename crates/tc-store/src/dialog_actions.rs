//! Estado de las acciones de gestión de chats (menú contextual de la lista): marca manual de «no
//! leído» (migración 0006) y vaciado del historial local de un chat.

use crate::db::Store;
use crate::error::{Error, Result};
use rusqlite::{OptionalExtension, params};

impl Store {
    /// Pone o quita la marca manual de «no leído».
    pub fn set_dialog_unread_mark(&self, chat_key: i64, unread: bool) -> Result<()> {
        if self.conn.execute("UPDATE dialogs SET unread_mark=?2 WHERE chat_key=?1", params![chat_key, unread])? == 0 {
            return Err(Error::NotFound(format!("diálogo {chat_key}")));
        }
        Ok(())
    }

    /// Marca manual de «no leído» (falso si el diálogo no existe).
    pub fn dialog_unread_mark(&self, chat_key: i64) -> Result<bool> {
        Ok(self
            .conn
            .query_row("SELECT unread_mark FROM dialogs WHERE chat_key=?1", [chat_key], |r| r.get::<_, bool>(0))
            .optional()?
            .unwrap_or(false))
    }

    /// Borra todos los mensajes del chat (y, en cascada, sus reacciones y entradas del FTS) sin
    /// quitar la fila de la lista. Devuelve cuántos se borraron.
    pub fn clear_dialog_history(&self, chat_key: i64) -> Result<usize> {
        let tx = self.conn.unchecked_transaction()?;
        let n = tx.execute("DELETE FROM messages WHERE chat_key=?1", [chat_key])?;
        tx.execute(
            "UPDATE dialogs SET top_message_id=NULL, unread_count=0, unread_mentions=0, unread_reactions=0, unread_mark=0
             WHERE chat_key=?1",
            [chat_key],
        )?;
        tx.commit()?;
        Ok(n)
    }
}

#[cfg(test)]
mod tests {
    use crate::db::Store;
    use crate::types::{DialogRow, PeerKind};

    #[test]
    fn unread_mark_and_clear_history() {
        let s = Store::open_in_memory().unwrap();
        let ck = s.upsert_dialog(&DialogRow::new(PeerKind::User, 5, 1)).unwrap();
        assert!(!s.dialog_unread_mark(ck).unwrap());
        s.set_dialog_unread_mark(ck, true).unwrap();
        assert!(s.dialog_unread_mark(ck).unwrap());
        assert!(!s.dialog_unread_mark(9999).unwrap());
        assert!(s.set_dialog_unread_mark(9999, true).is_err());
        assert_eq!(s.clear_dialog_history(ck).unwrap(), 0);
        assert!(!s.dialog_unread_mark(ck).unwrap());
    }
}
