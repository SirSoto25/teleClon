//! Chats secretos y sus mensajes. El cuerpo llega ya cifrado (nonce + texto cifrado): este módulo
//! solo guarda blobs y metadatos. Las claves viven en el keyring (`key_ref`), nunca aquí, y los
//! mensajes secretos no entran en el índice FTS.

use crate::db::Store;
use crate::error::{Error, Result};
use crate::types::*;
use rusqlite::{OptionalExtension, Row, params};

const CHAT_COLS: &str = "id, access_hash, peer_user_id, state, is_originator, layer, in_seq_no, out_seq_no, \
    ttl_seconds, key_ref, key_fingerprint, key_used_count, key_created_at, rekey_state, rekey_exchange_id, \
    created_at, updated_at";

const MSG_COLS: &str = "rowid, secret_chat_id, random_id, is_out, date, seq_in, seq_out, body_nonce, body_enc, \
    media_id, ttl_seconds, read_at, destroy_at, send_state";

/// Longitud del nonce de AES-GCM.
pub const NONCE_LEN: usize = 12;

fn chat_from_row(r: &Row<'_>) -> rusqlite::Result<SecretChatRow> {
    Ok(SecretChatRow {
        id: r.get(0)?,
        access_hash: r.get(1)?,
        peer_user_id: r.get(2)?,
        state: r.get(3)?,
        is_originator: r.get(4)?,
        layer: r.get(5)?,
        in_seq_no: r.get(6)?,
        out_seq_no: r.get(7)?,
        ttl_seconds: r.get(8)?,
        key_ref: r.get(9)?,
        key_fingerprint: r.get(10)?,
        key_used_count: r.get(11)?,
        key_created_at: r.get(12)?,
        rekey_state: r.get(13)?,
        rekey_exchange_id: r.get(14)?,
        created_at: r.get(15)?,
        updated_at: r.get(16)?,
    })
}

fn msg_from_row(r: &Row<'_>) -> rusqlite::Result<SecretMessageRow> {
    Ok(SecretMessageRow {
        rowid: r.get(0)?,
        secret_chat_id: r.get(1)?,
        random_id: r.get(2)?,
        is_out: r.get(3)?,
        date: r.get(4)?,
        seq_in: r.get(5)?,
        seq_out: r.get(6)?,
        body_nonce: r.get(7)?,
        body_enc: r.get(8)?,
        media_id: r.get(9)?,
        ttl_seconds: r.get(10)?,
        read_at: r.get(11)?,
        destroy_at: r.get(12)?,
        send_state: r.get(13)?,
    })
}

impl Store {
    /// Inserta o actualiza un chat secreto. El usuario par (`peer_user_id`) debe existir.
    pub fn upsert_secret_chat(&self, c: &SecretChatRow) -> Result<()> {
        self.conn.execute(
            &format!(
                "INSERT INTO secret_chats ({CHAT_COLS}) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15,?16,?17)
                 ON CONFLICT(id) DO UPDATE SET access_hash=excluded.access_hash, peer_user_id=excluded.peer_user_id,
                   state=excluded.state, is_originator=excluded.is_originator, layer=excluded.layer,
                   in_seq_no=excluded.in_seq_no, out_seq_no=excluded.out_seq_no, ttl_seconds=excluded.ttl_seconds,
                   key_ref=excluded.key_ref, key_fingerprint=excluded.key_fingerprint,
                   key_used_count=excluded.key_used_count, key_created_at=excluded.key_created_at,
                   rekey_state=excluded.rekey_state, rekey_exchange_id=excluded.rekey_exchange_id,
                   updated_at=excluded.updated_at"
            ),
            params![
                c.id, c.access_hash, c.peer_user_id, c.state, c.is_originator, c.layer, c.in_seq_no, c.out_seq_no,
                c.ttl_seconds, c.key_ref, c.key_fingerprint, c.key_used_count, c.key_created_at, c.rekey_state,
                c.rekey_exchange_id, c.created_at, c.updated_at
            ],
        )?;
        Ok(())
    }

    /// Chat secreto por id.
    pub fn get_secret_chat(&self, id: i64) -> Result<Option<SecretChatRow>> {
        Ok(self.conn.query_row(&format!("SELECT {CHAT_COLS} FROM secret_chats WHERE id=?1"), [id], chat_from_row).optional()?)
    }

    /// Todos los chats secretos no descartados.
    pub fn list_secret_chats(&self) -> Result<Vec<SecretChatRow>> {
        let mut st = self.conn.prepare(&format!(
            "SELECT {CHAT_COLS} FROM secret_chats WHERE state<>'discarded' ORDER BY updated_at DESC, id"
        ))?;
        let rows = st.query_map([], chat_from_row)?;
        Ok(rows.collect::<rusqlite::Result<_>>()?)
    }

    /// Borra el chat secreto y, en cascada, sus mensajes. El llamador debe borrar `key_ref` del keyring.
    pub fn delete_secret_chat(&self, id: i64) -> Result<bool> {
        Ok(self.conn.execute("DELETE FROM secret_chats WHERE id=?1", [id])? > 0)
    }

    /// Inserta un mensaje secreto ya cifrado. Idempotente por `(secret_chat_id, random_id)`:
    /// devuelve `true` si es nuevo; si ya existía solo actualiza el estado de envío y las secuencias.
    pub fn insert_secret_message(&self, m: &SecretMessageRow) -> Result<bool> {
        if m.body_nonce.len() != NONCE_LEN {
            return Err(Error::Invalid(format!("el nonce debe medir {NONCE_LEN} bytes, mide {}", m.body_nonce.len())));
        }
        let tx = self.conn.unchecked_transaction()?;
        let inserted = tx.execute(
            "INSERT OR IGNORE INTO secret_messages (secret_chat_id, random_id, is_out, date, seq_in, seq_out, body_nonce,
               body_enc, media_id, ttl_seconds, read_at, destroy_at, send_state)
             VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13)",
            params![
                m.secret_chat_id, m.random_id, m.is_out, m.date, m.seq_in, m.seq_out, m.body_nonce, m.body_enc,
                m.media_id, m.ttl_seconds, m.read_at, m.destroy_at, m.send_state
            ],
        )? > 0;
        if !inserted {
            tx.execute(
                "UPDATE secret_messages SET send_state=?3, seq_in=COALESCE(?4, seq_in), seq_out=COALESCE(?5, seq_out)
                 WHERE secret_chat_id=?1 AND random_id=?2",
                params![m.secret_chat_id, m.random_id, m.send_state, m.seq_in, m.seq_out],
            )?;
        }
        tx.commit()?;
        Ok(inserted)
    }

    /// Mensaje secreto por `random_id`.
    pub fn get_secret_message(&self, secret_chat_id: i64, random_id: i64) -> Result<Option<SecretMessageRow>> {
        Ok(self
            .conn
            .query_row(
                &format!("SELECT {MSG_COLS} FROM secret_messages WHERE secret_chat_id=?1 AND random_id=?2"),
                params![secret_chat_id, random_id],
                msg_from_row,
            )
            .optional()?)
    }

    /// Página del historial secreto en orden ascendente. `before_rowid = None` = los últimos.
    pub fn secret_messages_page(&self, secret_chat_id: i64, before_rowid: Option<i64>, limit: usize) -> Result<Vec<SecretMessageRow>> {
        let mut st = self.conn.prepare(&format!(
            "SELECT {MSG_COLS} FROM secret_messages WHERE secret_chat_id=?1 AND rowid<?2 ORDER BY rowid DESC LIMIT ?3"
        ))?;
        let rows = st.query_map(params![secret_chat_id, before_rowid.unwrap_or(i64::MAX), limit as i64], msg_from_row)?;
        let mut v: Vec<_> = rows.collect::<rusqlite::Result<_>>()?;
        v.reverse();
        Ok(v)
    }

    /// Marca un mensaje como leído; si tiene TTL, `destroy_at = now + ttl`. No cambia si ya estaba leído.
    pub fn mark_secret_message_read(&self, secret_chat_id: i64, random_id: i64, now: i64) -> Result<()> {
        self.conn.execute(
            "UPDATE secret_messages SET read_at=?3,
               destroy_at=CASE WHEN ttl_seconds>0 THEN ?3 + ttl_seconds ELSE destroy_at END
             WHERE secret_chat_id=?1 AND random_id=?2 AND read_at IS NULL",
            params![secret_chat_id, random_id, now],
        )?;
        Ok(())
    }

    /// Borra los mensajes cuyo `destroy_at` ya pasó (con `secure_delete`). Devuelve cuántos.
    pub fn purge_expired_secret_messages(&self, now: i64) -> Result<usize> {
        Ok(self.conn.execute("DELETE FROM secret_messages WHERE destroy_at IS NOT NULL AND destroy_at<=?1", [now])?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn chat(id: i64) -> SecretChatRow {
        SecretChatRow {
            id,
            access_hash: 99,
            peer_user_id: 5,
            state: SecretState::Accepted,
            is_originator: true,
            layer: 8,
            in_seq_no: 0,
            out_seq_no: 0,
            ttl_seconds: 0,
            key_ref: Some(format!("acct:1:secret:{id}")),
            key_fingerprint: Some(-123456789),
            key_used_count: 0,
            key_created_at: Some(1),
            rekey_state: RekeyState::Idle,
            rekey_exchange_id: None,
            created_at: 1,
            updated_at: 1,
        }
    }

    fn msg(chat: i64, rid: i64, date: i64) -> SecretMessageRow {
        SecretMessageRow {
            rowid: 0,
            secret_chat_id: chat,
            random_id: rid,
            is_out: false,
            date,
            seq_in: Some(rid),
            seq_out: None,
            body_nonce: vec![7; NONCE_LEN],
            body_enc: vec![0xAB, 0xCD, rid as u8],
            media_id: None,
            ttl_seconds: 0,
            read_at: None,
            destroy_at: None,
            send_state: SendState::Sent,
        }
    }

    fn setup() -> Store {
        let s = Store::open_in_memory().unwrap();
        s.upsert_user(&UserRow { id: 5, first_name: "Peer".into(), updated_at: 1, ..Default::default() }).unwrap();
        s.upsert_secret_chat(&chat(1)).unwrap();
        s
    }

    #[test]
    fn chat_roundtrip_and_fk() {
        let s = setup();
        assert_eq!(s.get_secret_chat(1).unwrap().unwrap(), chat(1));
        let mut c = chat(1);
        c.out_seq_no = 4;
        c.rekey_state = RekeyState::Requested;
        s.upsert_secret_chat(&c).unwrap();
        assert_eq!(s.get_secret_chat(1).unwrap().unwrap().out_seq_no, 4);
        let mut bad = chat(2);
        bad.peer_user_id = 404;
        assert!(s.upsert_secret_chat(&bad).is_err(), "FK al usuario par");
        let mut d = chat(3);
        d.state = SecretState::Discarded;
        s.upsert_secret_chat(&d).unwrap();
        assert_eq!(s.list_secret_chats().unwrap().len(), 1);
    }

    #[test]
    fn no_key_material_columns() {
        let s = setup();
        let mut st = s.conn.prepare("SELECT name FROM pragma_table_info('secret_chats')").unwrap();
        let cols: Vec<String> = st.query_map([], |r| r.get(0)).unwrap().map(|x| x.unwrap()).collect();
        assert!(cols.iter().all(|c| !matches!(c.as_str(), "key" | "auth_key" | "shared_key" | "secret")), "{cols:?}");
        let mut st = s.conn.prepare("SELECT name FROM pragma_table_info('secret_chats') WHERE type='BLOB'").unwrap();
        assert_eq!(st.query_map([], |r| r.get::<_, String>(0)).unwrap().count(), 0);
    }

    #[test]
    fn messages_idempotent_blob_roundtrip_and_page() {
        let s = setup();
        assert!(s.insert_secret_message(&msg(1, 10, 100)).unwrap());
        let mut dup = msg(1, 10, 100);
        dup.send_state = SendState::Failed;
        dup.body_enc = vec![9, 9]; // un duplicado no pisa el cuerpo
        assert!(!s.insert_secret_message(&dup).unwrap());
        let got = s.get_secret_message(1, 10).unwrap().unwrap();
        assert_eq!(got.body_enc, vec![0xAB, 0xCD, 10]);
        assert_eq!(got.body_nonce, vec![7; NONCE_LEN]);
        assert_eq!(got.send_state, SendState::Failed);
        for i in 11..=20 {
            s.insert_secret_message(&msg(1, i, 100 + i)).unwrap();
        }
        let last = s.secret_messages_page(1, None, 4).unwrap();
        assert_eq!(last.iter().map(|m| m.random_id).collect::<Vec<_>>(), [17, 18, 19, 20]);
        let prev = s.secret_messages_page(1, Some(last[0].rowid), 4).unwrap();
        assert_eq!(prev.iter().map(|m| m.random_id).collect::<Vec<_>>(), [13, 14, 15, 16]);
        let mut bad = msg(1, 99, 1);
        bad.body_nonce = vec![0; 5];
        assert!(matches!(s.insert_secret_message(&bad), Err(Error::Invalid(_))));
    }

    #[test]
    fn ttl_starts_on_read_and_purges() {
        let s = setup();
        let mut a = msg(1, 1, 10);
        a.ttl_seconds = 30;
        s.insert_secret_message(&a).unwrap();
        s.insert_secret_message(&msg(1, 2, 11)).unwrap(); // sin TTL
        assert_eq!(s.purge_expired_secret_messages(10_000).unwrap(), 0, "sin leer no caduca");
        s.mark_secret_message_read(1, 1, 100).unwrap();
        s.mark_secret_message_read(1, 1, 500).unwrap(); // no reinicia el contador
        s.mark_secret_message_read(1, 2, 100).unwrap();
        let m = s.get_secret_message(1, 1).unwrap().unwrap();
        assert_eq!((m.read_at, m.destroy_at), (Some(100), Some(130)));
        assert_eq!(s.get_secret_message(1, 2).unwrap().unwrap().destroy_at, None);
        assert_eq!(s.purge_expired_secret_messages(129).unwrap(), 0);
        assert_eq!(s.purge_expired_secret_messages(130).unwrap(), 1);
        assert!(s.get_secret_message(1, 1).unwrap().is_none());
        assert!(s.get_secret_message(1, 2).unwrap().is_some());
    }

    #[test]
    fn not_in_fts_and_cascade_delete() {
        let s = setup();
        s.insert_secret_message(&msg(1, 1, 1)).unwrap();
        let n: i64 = s.conn.query_row("SELECT COUNT(*) FROM messages_fts WHERE messages_fts MATCH 'AB'", [], |r| r.get(0)).unwrap();
        assert_eq!(n, 0);
        assert!(s.delete_secret_chat(1).unwrap());
        assert!(s.secret_messages_page(1, None, 10).unwrap().is_empty());
    }
}
