//! Mensajes de chats normales: upsert idempotente, paginación por ancla y búsqueda FTS5.

use crate::db::Store;
use crate::error::Result;
use crate::types::*;
use rusqlite::{OptionalExtension, Row, params};

const COLS: &str = "chat_key, msg_id, random_id, sender_id, date, edit_date, is_out, kind, text, entities, \
    reply_to_id, reply_quote, fwd_from_id, fwd_from_name, fwd_date, media_id, sticker_id, grouped_id, \
    service_action, is_pinned, views, send_state, sticker_json";

fn from_row(r: &Row<'_>) -> rusqlite::Result<MessageRow> {
    Ok(MessageRow {
        chat_key: r.get(0)?,
        msg_id: r.get(1)?,
        random_id: r.get(2)?,
        sender_id: r.get(3)?,
        date: r.get(4)?,
        edit_date: r.get(5)?,
        is_out: r.get(6)?,
        kind: r.get(7)?,
        text: r.get(8)?,
        entities: r.get(9)?,
        reply_to_id: r.get(10)?,
        reply_quote: r.get(11)?,
        fwd_from_id: r.get(12)?,
        fwd_from_name: r.get(13)?,
        fwd_date: r.get(14)?,
        media_id: r.get(15)?,
        sticker_id: r.get(16)?,
        grouped_id: r.get(17)?,
        service_action: r.get(18)?,
        is_pinned: r.get(19)?,
        views: r.get(20)?,
        send_state: r.get(21)?,
        sticker_json: r.get(22)?,
    })
}

/// Convierte la entrada del usuario en una consulta FTS5 segura: cada palabra entre comillas
/// (AND implícito) y la última como prefijo. Devuelve `None` si no hay términos.
pub fn fts_query(input: &str) -> Option<String> {
    let words: Vec<&str> = input.split_whitespace().collect();
    if words.is_empty() {
        return None;
    }
    let last = words.len() - 1;
    let parts: Vec<String> = words
        .iter()
        .enumerate()
        .map(|(i, w)| format!("\"{}\"{}", w.replace('"', "\"\""), if i == last { "*" } else { "" }))
        .collect();
    Some(parts.join(" "))
}

impl Store {
    /// Inserta o actualiza un mensaje (idempotente por `(chat_key, msg_id)`).
    ///
    /// Si trae `random_id` y ya existe un mensaje pendiente con ese `random_id` (id local
    /// negativo), ese mensaje se convierte en el confirmado en lugar de duplicarse.
    pub fn upsert_message(&self, m: &MessageRow) -> Result<()> {
        let tx = self.conn.unchecked_transaction()?;
        upsert_in(&tx, m)?;
        tx.commit()?;
        Ok(())
    }

    /// Como [`Self::upsert_message`] para un lote, en una sola transacción.
    pub fn upsert_messages(&self, msgs: &[MessageRow]) -> Result<()> {
        let tx = self.conn.unchecked_transaction()?;
        for m in msgs {
            upsert_in(&tx, m)?;
        }
        tx.commit()?;
        Ok(())
    }

    /// Mensaje concreto.
    pub fn get_message(&self, chat_key: i64, msg_id: i64) -> Result<Option<MessageRow>> {
        Ok(self
            .conn
            .query_row(&format!("SELECT {COLS} FROM messages WHERE chat_key=?1 AND msg_id=?2"), params![chat_key, msg_id], from_row)
            .optional()?)
    }

    /// Página del historial en orden ascendente (más antiguo primero). Solo mensajes confirmados.
    pub fn messages_page(&self, chat_key: i64, anchor: Anchor, limit: usize) -> Result<Vec<MessageRow>> {
        let lim = limit as i64;
        let q = |cond: &str, order: &str| format!("SELECT {COLS} FROM messages WHERE chat_key=?1 AND msg_id>0 {cond} ORDER BY msg_id {order} LIMIT ?3");
        let run = |sql: &str, a: i64, n: i64| -> Result<Vec<MessageRow>> {
            let mut st = self.conn.prepare(sql)?;
            let rows = st.query_map(params![chat_key, a, n], from_row)?;
            Ok(rows.collect::<rusqlite::Result<_>>()?)
        };
        Ok(match anchor {
            Anchor::Latest => {
                let mut v = run(&q("AND msg_id>=?2", "DESC"), i64::MIN, lim)?;
                v.reverse();
                v
            }
            Anchor::Before(id) => {
                let mut v = run(&q("AND msg_id<?2", "DESC"), id, lim)?;
                v.reverse();
                v
            }
            Anchor::After(id) => run(&q("AND msg_id>?2", "ASC"), id, lim)?,
            Anchor::Around(id) => {
                let mut v = run(&q("AND msg_id<=?2", "DESC"), id, (lim + 1) / 2)?;
                v.reverse();
                v.extend(run(&q("AND msg_id>?2", "ASC"), id, lim / 2)?);
                v
            }
        })
    }

    /// Mensajes locales aún sin confirmar (pendientes o fallidos) de un chat, en orden de fecha.
    pub fn unsent_messages(&self, chat_key: i64) -> Result<Vec<MessageRow>> {
        let mut st = self.conn.prepare(&format!(
            "SELECT {COLS} FROM messages WHERE chat_key=?1 AND send_state<>'sent' ORDER BY date, msg_id"
        ))?;
        let rows = st.query_map([chat_key], from_row)?;
        Ok(rows.collect::<rusqlite::Result<_>>()?)
    }

    /// Marca como fallido el mensaje pendiente con ese `random_id`; devuelve cuántos cambió.
    pub fn mark_failed_by_random(&self, random_id: i64) -> Result<usize> {
        Ok(self.conn.execute(
            "UPDATE messages SET send_state='failed' WHERE random_id=?1 AND send_state='pending'",
            [random_id],
        )?)
    }

    /// Ids confirmados del chat entre `lo` y `hi` (ambos incluidos), ascendentes.
    pub fn message_ids_between(&self, chat_key: i64, lo: i64, hi: i64) -> Result<Vec<i64>> {
        let mut st = self.conn.prepare(
            "SELECT msg_id FROM messages WHERE chat_key=?1 AND msg_id>0 AND msg_id BETWEEN ?2 AND ?3 ORDER BY msg_id",
        )?;
        let rows = st.query_map(params![chat_key, lo, hi], |r| r.get(0))?;
        Ok(rows.collect::<rusqlite::Result<_>>()?)
    }

    /// Borra mensajes por id; devuelve cuántos existían.
    pub fn delete_messages(&self, chat_key: i64, ids: &[i64]) -> Result<usize> {
        let tx = self.conn.unchecked_transaction()?;
        let mut n = 0;
        for id in ids {
            n += tx.execute("DELETE FROM messages WHERE chat_key=?1 AND msg_id=?2", params![chat_key, id])?;
        }
        tx.commit()?;
        Ok(n)
    }

    /// Búsqueda de texto (FTS5, sin distinguir acentos ni mayúsculas), mejor coincidencia primero.
    /// `chat_key = None` busca en todos los chats. Los chats secretos nunca se indexan.
    pub fn search_messages(&self, query: &str, chat_key: Option<i64>, limit: usize) -> Result<Vec<SearchHit>> {
        let Some(q) = fts_query(query) else { return Ok(Vec::new()) };
        let mut st = self.conn.prepare(
            "SELECT m.chat_key, m.msg_id, m.date, snippet(messages_fts, 0, '«', '»', '…', 12)
             FROM messages_fts JOIN messages m ON m.rowid = messages_fts.rowid
             WHERE messages_fts MATCH ?1 AND (?2 IS NULL OR m.chat_key = ?2)
             ORDER BY rank, m.date DESC LIMIT ?3",
        )?;
        let rows = st.query_map(params![q, chat_key, limit as i64], |r| {
            Ok(SearchHit { chat_key: r.get(0)?, msg_id: r.get(1)?, date: r.get(2)?, snippet: r.get(3)? })
        })?;
        Ok(rows.collect::<rusqlite::Result<_>>()?)
    }

    /// Búsqueda dentro de un chat (ADR-0012): mensajes confirmados que cumplen el texto (FTS5, sin
    /// acentos ni mayúsculas, prefijo en la última palabra; vacío = sin condición de texto) y el
    /// filtro por tipo, **del más reciente al más antiguo**. `topic` limita a un tema de foro,
    /// `before` a ids menores (paginación). Devuelve los `limit` primeros y el total de coincidencias
    /// (sin contar `before`).
    pub fn search_chat(
        &self,
        chat_key: i64,
        topic: Option<i64>,
        text: &str,
        filter: tc_core::SearchFilter,
        before: Option<i64>,
        limit: usize,
    ) -> Result<(Vec<MessageRow>, usize)> {
        use tc_core::SearchFilter as F;
        let fts = fts_query(text);
        if fts.is_none() && !filter.lists_without_text() {
            return Ok((Vec::new(), 0));
        }
        let cond = match filter {
            F::All => "1",
            F::Photos => "md.kind = 'photo'",
            F::Videos => "md.kind = 'video' AND NOT EXISTS (SELECT 1 FROM media_extra e WHERE e.media_id = md.id AND e.round = 1)",
            F::Files => "md.kind = 'document'",
            F::Voice => "md.kind = 'voice'",
            F::Links => "(m.entities LIKE '%\"url\"%' OR m.text LIKE '%http://%' OR m.text LIKE '%https://%' OR m.text LIKE '%www.%')",
        };
        let from = if fts.is_some() {
            "messages_fts JOIN messages m ON m.rowid = messages_fts.rowid"
        } else {
            "messages m"
        };
        let mut wher = format!(
            "m.chat_key = ?1 AND m.msg_id > 0 AND (?3 IS NULL OR mt.topic_id = ?3) AND {cond}"
        );
        if fts.is_some() {
            wher.push_str(" AND messages_fts MATCH ?2");
        }
        let joins = "LEFT JOIN media md ON md.id = m.media_id LEFT JOIN message_topics mt ON mt.chat_key = m.chat_key AND mt.msg_id = m.msg_id";
        let total: i64 = self.conn.query_row(
            &format!("SELECT COUNT(*) FROM {from} {joins} WHERE {wher}"),
            params![chat_key, fts, topic],
            |r| r.get(0),
        )?;
        let cols = COLS.split(", ").map(|c| format!("m.{c}")).collect::<Vec<_>>().join(", ");
        let mut st = self.conn.prepare(&format!(
            "SELECT {cols} FROM {from} {joins} WHERE {wher} AND (?4 IS NULL OR m.msg_id < ?4) ORDER BY m.msg_id DESC LIMIT ?5"
        ))?;
        let rows = st.query_map(params![chat_key, fts, topic, before, limit as i64], from_row)?;
        Ok((rows.collect::<rusqlite::Result<_>>()?, usize::try_from(total).unwrap_or(0)))
    }
}

fn upsert_in(tx: &rusqlite::Connection, m: &MessageRow) -> Result<()> {
    if let Some(rid) = m.random_id {
        let existing: Option<(i64, i64)> = tx
            .query_row("SELECT chat_key, msg_id FROM messages WHERE random_id=?1", [rid], |r| Ok((r.get(0)?, r.get(1)?)))
            .optional()?;
        if let Some((ck, mid)) = existing.filter(|&(ck, mid)| (ck, mid) != (m.chat_key, m.msg_id)) {
            let target_exists: bool = tx.query_row(
                "SELECT EXISTS(SELECT 1 FROM messages WHERE chat_key=?1 AND msg_id=?2)",
                params![m.chat_key, m.msg_id],
                |r| r.get(0),
            )?;
            if ck == m.chat_key && !target_exists {
                // El pendiente pasa a ser el confirmado (las reacciones lo siguen por ON UPDATE CASCADE).
                tx.execute("UPDATE messages SET msg_id=?3 WHERE chat_key=?1 AND msg_id=?2", params![ck, mid, m.msg_id])?;
            } else {
                tx.execute("DELETE FROM messages WHERE chat_key=?1 AND msg_id=?2", params![ck, mid])?;
            }
        }
    }
    tx.execute(
        &format!(
            "INSERT INTO messages ({COLS}) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15,?16,?17,?18,?19,?20,?21,?22,?23)
             ON CONFLICT(chat_key, msg_id) DO UPDATE SET random_id=excluded.random_id, sender_id=excluded.sender_id,
               date=excluded.date, edit_date=excluded.edit_date, is_out=excluded.is_out, kind=excluded.kind,
               text=excluded.text, entities=excluded.entities, reply_to_id=excluded.reply_to_id,
               reply_quote=excluded.reply_quote, fwd_from_id=excluded.fwd_from_id, fwd_from_name=excluded.fwd_from_name,
               fwd_date=excluded.fwd_date, media_id=excluded.media_id, sticker_id=excluded.sticker_id,
               grouped_id=excluded.grouped_id, service_action=excluded.service_action, is_pinned=excluded.is_pinned,
               views=excluded.views, send_state=excluded.send_state, sticker_json=excluded.sticker_json"
        ),
        params![
            m.chat_key, m.msg_id, m.random_id, m.sender_id, m.date, m.edit_date, m.is_out, m.kind, m.text, m.entities,
            m.reply_to_id, m.reply_quote, m.fwd_from_id, m.fwd_from_name, m.fwd_date, m.media_id, m.sticker_id,
            m.grouped_id, m.service_action, m.is_pinned, m.views, m.send_state, m.sticker_json
        ],
    )?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn setup() -> (Store, i64) {
        let s = Store::open_in_memory().unwrap();
        let k = s.upsert_dialog(&DialogRow::new(PeerKind::User, 1, 0)).unwrap();
        (s, k)
    }

    fn fill(s: &Store, k: i64, n: i64) {
        let msgs: Vec<_> = (1..=n).map(|i| MessageRow::text(k, i, 1000 + i, i % 2 == 0, &format!("mensaje {i}"))).collect();
        s.upsert_messages(&msgs).unwrap();
    }

    fn ids(v: &[MessageRow]) -> Vec<i64> {
        v.iter().map(|m| m.msg_id).collect()
    }

    #[test]
    fn upsert_is_idempotent_and_edits() {
        let (s, k) = setup();
        let mut m = MessageRow::text(k, 10, 5, true, "hola");
        s.upsert_message(&m).unwrap();
        s.upsert_message(&m).unwrap();
        m.text = "hola editado".into();
        m.edit_date = Some(9);
        s.upsert_message(&m).unwrap();
        assert_eq!(s.messages_page(k, Anchor::Latest, 10).unwrap().len(), 1);
        let got = s.get_message(k, 10).unwrap().unwrap();
        assert_eq!((got.text.as_str(), got.edit_date), ("hola editado", Some(9)));
        // el FTS sigue al texto editado
        assert!(s.search_messages("editado", None, 5).unwrap().len() == 1);
        assert!(s.search_messages("hola editado", None, 5).unwrap().len() == 1);
    }

    #[test]
    fn pagination_by_anchor() {
        let (s, k) = setup();
        fill(&s, k, 100);
        assert_eq!(ids(&s.messages_page(k, Anchor::Latest, 3).unwrap()), [98, 99, 100]);
        assert_eq!(ids(&s.messages_page(k, Anchor::Before(98), 3).unwrap()), [95, 96, 97]);
        assert_eq!(ids(&s.messages_page(k, Anchor::After(98), 5).unwrap()), [99, 100]);
        assert_eq!(ids(&s.messages_page(k, Anchor::Around(50), 6).unwrap()), [48, 49, 50, 51, 52, 53]);
        assert!(s.messages_page(k, Anchor::Before(1), 10).unwrap().is_empty());
        // recorrer todo el historial hacia atrás sin huecos ni repeticiones
        let mut all = Vec::new();
        let mut page = s.messages_page(k, Anchor::Latest, 30).unwrap();
        while !page.is_empty() {
            let first = page[0].msg_id;
            all.splice(0..0, ids(&page));
            page = s.messages_page(k, Anchor::Before(first), 30).unwrap();
        }
        assert_eq!(all, (1..=100).collect::<Vec<_>>());
    }

    #[test]
    fn pending_becomes_confirmed_by_random_id() {
        let (s, k) = setup();
        let mut p = MessageRow::text(k, -1, 50, true, "enviando");
        p.random_id = Some(777);
        p.send_state = SendState::Pending;
        s.upsert_message(&p).unwrap();
        assert!(s.messages_page(k, Anchor::Latest, 10).unwrap().is_empty());
        assert_eq!(s.unsent_messages(k).unwrap().len(), 1);
        s.conn.execute("INSERT INTO reactions (chat_key,msg_id,emoji,count,chosen) VALUES (?1,-1,'👍',1,1)", [k]).unwrap();

        let mut c = p.clone();
        c.msg_id = 200;
        c.send_state = SendState::Sent;
        s.upsert_message(&c).unwrap();
        s.upsert_message(&c).unwrap();
        assert!(s.get_message(k, -1).unwrap().is_none());
        assert_eq!(s.get_message(k, 200).unwrap().unwrap().send_state, SendState::Sent);
        assert!(s.unsent_messages(k).unwrap().is_empty());
        let n: i64 = s.conn.query_row("SELECT COUNT(*) FROM reactions WHERE msg_id=200", [], |r| r.get(0)).unwrap();
        assert_eq!(n, 1, "la reacción sigue al mensaje");

        // si la confirmación del servidor llegó antes que el eco local, no se duplica
        let mut p2 = MessageRow::text(k, -2, 60, true, "otro");
        p2.random_id = Some(888);
        s.upsert_message(&p2).unwrap();
        let mut c2 = p2.clone();
        c2.msg_id = 201;
        s.upsert_message(&c2).unwrap();
        let total: i64 = s.conn.query_row("SELECT COUNT(*) FROM messages", [], |r| r.get(0)).unwrap();
        assert_eq!(total, 2);
    }

    fn media_msg(s: &Store, k: i64, id: i64, media: i64, kind: &str, caption: &str) {
        s.upsert_media(&MediaRow { id: media, kind: kind.into(), ..Default::default() }).unwrap();
        let mut m = MessageRow::text(k, id, 100 + id, false, caption);
        m.kind = MessageKind::Media;
        m.media_id = Some(media);
        s.upsert_message(&m).unwrap();
    }

    #[test]
    fn search_chat_text_filters_pages_topics_and_deletes() {
        use tc_core::SearchFilter as F;
        let (s, k) = setup();
        let k2 = s.upsert_dialog(&DialogRow::new(PeerKind::User, 2, 0)).unwrap();
        s.upsert_messages(&[
            MessageRow::text(k, 1, 10, false, "Reunión en la CAFETERÍA"),
            MessageRow::text(k, 2, 11, true, "nada que ver"),
            MessageRow::text(k, 3, 12, false, "otra cafetería más"),
            MessageRow::text(k2, 1, 13, false, "cafetería de otro chat"),
        ])
        .unwrap();
        let ids = |v: &[MessageRow]| v.iter().map(|m| m.msg_id).collect::<Vec<_>>();
        let (h, total) = s.search_chat(k, None, "cafeteria", F::All, None, 10).unwrap();
        assert_eq!((ids(&h), total), (vec![3, 1], 2), "acentos, mayúsculas y más reciente primero; solo ese chat");
        assert_eq!(ids(&s.search_chat(k, None, "CAFE", F::All, None, 10).unwrap().0), [3, 1], "prefijo");
        assert!(s.search_chat(k, None, "xyzqw", F::All, None, 10).unwrap().0.is_empty());
        assert_eq!(s.search_chat(k, None, "  ", F::All, None, 10).unwrap(), (vec![], 0));
        for q in ["\"", "a OR", "NEAR(", "-"] {
            s.search_chat(k, None, q, F::All, None, 10).unwrap();
        }
        // paginación: límite y `before`
        let (p1, t) = s.search_chat(k, None, "cafeteria", F::All, None, 1).unwrap();
        assert_eq!((ids(&p1), t), (vec![3], 2));
        assert_eq!(ids(&s.search_chat(k, None, "cafeteria", F::All, Some(3), 1).unwrap().0), [1]);
        // filtros por tipo
        media_msg(&s, k, 10, 501, "photo", "foto de la cafetería");
        media_msg(&s, k, 11, 502, "document", "");
        media_msg(&s, k, 12, 503, "voice", "");
        media_msg(&s, k, 13, 504, "video", "");
        s.upsert_message(&MessageRow::text(k, 14, 120, false, "mira https://example.org/x")).unwrap();
        let mut with_entity = MessageRow::text(k, 15, 121, false, "enlace sin esquema");
        with_entity.entities = Some("[[\"url\",0,6]]".into());
        s.upsert_message(&with_entity).unwrap();
        let f = |flt, text: &str| ids(&s.search_chat(k, None, text, flt, None, 20).unwrap().0);
        assert_eq!(f(F::Photos, ""), [10]);
        assert_eq!(f(F::Photos, "cafeteria"), [10]);
        assert_eq!(f(F::Files, ""), [11]);
        assert_eq!(f(F::Voice, ""), [12]);
        assert_eq!(f(F::Videos, ""), [13]);
        assert_eq!(f(F::Links, ""), [15, 14]);
        assert_eq!(f(F::All, "cafeteria"), [10, 3, 1]);
        // vídeo redondo: no es «vídeo»
        s.conn.execute("INSERT INTO media_extra (media_id, round) VALUES (504, 1)", []).unwrap();
        assert!(f(F::Videos, "").is_empty());
        // temas de foro
        s.set_message_topic(k, 1, 7).unwrap();
        s.set_message_topic(k, 3, 8).unwrap();
        assert_eq!(ids(&s.search_chat(k, Some(7), "cafeteria", F::All, None, 10).unwrap().0), [1]);
        assert_eq!(s.search_chat(k, Some(8), "cafeteria", F::All, None, 10).unwrap().1, 1);
        // un mensaje borrado desaparece de los resultados (y de su índice)
        s.delete_messages(k, &[3]).unwrap();
        assert_eq!(ids(&s.search_chat(k, None, "cafeteria", F::All, None, 10).unwrap().0), [10, 1]);
        // un pendiente local no sale
        let mut p = MessageRow::text(k, -5, 500, true, "cafeteria pendiente");
        p.send_state = SendState::Pending;
        s.upsert_message(&p).unwrap();
        assert_eq!(s.search_chat(k, None, "pendiente", F::All, None, 10).unwrap().1, 0);
    }

    #[test]
    fn fts_search_accents_prefix_scope_and_delete() {
        let (s, k) = setup();
        let k2 = s.upsert_dialog(&DialogRow::new(PeerKind::User, 2, 0)).unwrap();
        s.upsert_messages(&[
            MessageRow::text(k, 1, 10, false, "Reunión mañana en la CAFETERÍA"),
            MessageRow::text(k, 2, 11, true, "nos vemos luego"),
            MessageRow::text(k2, 1, 12, false, "la cafetería cierra"),
        ])
        .unwrap();
        let all = s.search_messages("cafeteria", None, 10).unwrap();
        assert_eq!(all.len(), 2, "sin acentos ni mayúsculas");
        assert!(all[0].snippet.contains('«'));
        assert_eq!(s.search_messages("cafeteria", Some(k), 10).unwrap().len(), 1);
        assert_eq!(s.search_messages("reun", None, 10).unwrap().len(), 1, "prefijo en la última palabra");
        assert_eq!(s.search_messages("reunion cafe", None, 10).unwrap().len(), 1, "AND de palabras");
        assert!(s.search_messages("   ", None, 10).unwrap().is_empty());
        // la entrada hostil no rompe la sintaxis FTS5
        for q in ["\"", "a OR", "NEAR(", "col:x *", "-"] {
            s.search_messages(q, None, 10).unwrap();
        }
        assert_eq!(s.delete_messages(k, &[1, 99]).unwrap(), 1);
        assert_eq!(s.search_messages("cafeteria", None, 10).unwrap().len(), 1);
        // borrar el diálogo limpia también el índice
        s.delete_dialog(k2).unwrap();
        assert!(s.search_messages("cafeteria", None, 10).unwrap().is_empty());
        s.conn.execute_batch("INSERT INTO messages_fts(messages_fts) VALUES('integrity-check')").unwrap();
    }
}
