//! Subidas reanudables: progreso por partes para continuar tras cerrar la app.

use crate::db::Store;
use crate::error::{Error, Result};
use crate::types::*;
use rusqlite::{OptionalExtension, Row, params};

const COLS: &str = "local_id, chat_key, secret_chat_id, path, size, file_id, part_size, parts_total, parts_done, \
    state, last_error, created_at, updated_at";

fn from_row(r: &Row<'_>) -> rusqlite::Result<UploadRow> {
    Ok(UploadRow {
        local_id: r.get(0)?,
        chat_key: r.get(1)?,
        secret_chat_id: r.get(2)?,
        path: r.get(3)?,
        size: r.get(4)?,
        file_id: r.get(5)?,
        part_size: r.get(6)?,
        parts_total: r.get(7)?,
        parts_done: r.get(8)?,
        state: r.get(9)?,
        last_error: r.get(10)?,
        created_at: r.get(11)?,
        updated_at: r.get(12)?,
    })
}

impl Store {
    /// Registra una subida nueva (en cola) y devuelve su `local_id`. Exige exactamente un destino
    /// (`chat_key` o `secret_chat_id`) y ignora `local_id`, `state`, `parts_done` y `last_error` de `u`.
    pub fn create_upload(&self, u: &UploadRow) -> Result<i64> {
        if u.chat_key.is_some() == u.secret_chat_id.is_some() {
            return Err(Error::Invalid("una subida necesita exactamente un destino (chat o chat secreto)".into()));
        }
        if u.part_size <= 0 || u.parts_total < 0 || u.size < 0 {
            return Err(Error::Invalid("tamaños de subida no válidos".into()));
        }
        Ok(self.conn.query_row(
            "INSERT INTO uploads (chat_key, secret_chat_id, path, size, file_id, part_size, parts_total, parts_done,
               state, created_at, updated_at)
             VALUES (?1,?2,?3,?4,?5,?6,?7,0,'queued',?8,?8) RETURNING local_id",
            params![u.chat_key, u.secret_chat_id, u.path, u.size, u.file_id, u.part_size, u.parts_total, u.created_at],
            |r| r.get(0),
        )?)
    }

    /// Subida por id.
    pub fn get_upload(&self, local_id: i64) -> Result<Option<UploadRow>> {
        Ok(self.conn.query_row(&format!("SELECT {COLS} FROM uploads WHERE local_id=?1"), [local_id], from_row).optional()?)
    }

    /// Subidas en un estado, la más antigua primero.
    pub fn list_uploads(&self, state: UploadState) -> Result<Vec<UploadRow>> {
        let mut st = self.conn.prepare(&format!("SELECT {COLS} FROM uploads WHERE state=?1 ORDER BY created_at, local_id"))?;
        let rows = st.query_map([state], from_row)?;
        Ok(rows.collect::<rusqlite::Result<_>>()?)
    }

    /// Subidas por completar (en cola, en curso o en pausa), la más antigua primero.
    pub fn pending_uploads(&self) -> Result<Vec<UploadRow>> {
        let mut st = self.conn.prepare(&format!(
            "SELECT {COLS} FROM uploads WHERE state IN ('queued','uploading','paused') ORDER BY created_at, local_id"
        ))?;
        let rows = st.query_map([], from_row)?;
        Ok(rows.collect::<rusqlite::Result<_>>()?)
    }

    /// Anota el progreso (partes ya subidas). Solo avanza; pasa a `uploading`, o a `done` si se
    /// completaron todas las partes. No toca subidas terminadas ni canceladas.
    pub fn set_upload_progress(&self, local_id: i64, parts_done: i64, now: i64) -> Result<()> {
        let n = self.conn.execute(
            "UPDATE uploads SET parts_done = MIN(parts_total, MAX(parts_done, ?2)),
               state = CASE WHEN MIN(parts_total, MAX(parts_done, ?2)) >= parts_total THEN 'done' ELSE 'uploading' END,
               last_error = NULL, updated_at = ?3
             WHERE local_id=?1 AND state IN ('queued','uploading','paused')",
            params![local_id, parts_done, now],
        )?;
        if n == 0 {
            return Err(Error::NotFound(format!("subida activa {local_id}")));
        }
        Ok(())
    }

    /// Cambia el estado (con el motivo si falló). Devuelve `false` si no existe.
    pub fn set_upload_state(&self, local_id: i64, state: UploadState, error: Option<&str>, now: i64) -> Result<bool> {
        Ok(self.conn.execute(
            "UPDATE uploads SET state=?2, last_error=?3, updated_at=?4 WHERE local_id=?1",
            params![local_id, state, error, now],
        )? > 0)
    }

    /// Al arrancar: lo que figuraba «en curso» se quedó a medias al cerrar; pasa a pausa. Devuelve cuántas.
    pub fn pause_interrupted_uploads(&self, now: i64) -> Result<usize> {
        Ok(self.conn.execute("UPDATE uploads SET state='paused', updated_at=?1 WHERE state='uploading'", [now])?)
    }

    /// Borra una subida.
    pub fn delete_upload(&self, local_id: i64) -> Result<bool> {
        Ok(self.conn.execute("DELETE FROM uploads WHERE local_id=?1", [local_id])? > 0)
    }

    /// Borra las subidas terminadas, fallidas o canceladas no actualizadas desde `before`.
    pub fn purge_finished_uploads(&self, before: i64) -> Result<usize> {
        Ok(self.conn.execute(
            "DELETE FROM uploads WHERE state IN ('done','failed','cancelled') AND updated_at < ?1",
            [before],
        )?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn new_upload(chat_key: Option<i64>, secret: Option<i64>, parts: i64, created: i64) -> UploadRow {
        UploadRow {
            local_id: 0,
            chat_key,
            secret_chat_id: secret,
            path: "/tmp/video.mp4".into(),
            size: parts * 512 * 1024,
            file_id: 42,
            part_size: 512 * 1024,
            parts_total: parts,
            parts_done: 99,
            state: UploadState::Done,
            last_error: Some("ignorado".into()),
            created_at: created,
            updated_at: 0,
        }
    }

    #[test]
    fn lifecycle_progress_resume_and_purge() {
        let s = Store::open_in_memory().unwrap();
        let k = s.upsert_dialog(&DialogRow::new(PeerKind::User, 1, 0)).unwrap();
        let a = s.create_upload(&new_upload(Some(k), None, 4, 10)).unwrap();
        let b = s.create_upload(&new_upload(Some(k), None, 2, 5)).unwrap();
        let up = s.get_upload(a).unwrap().unwrap();
        assert_eq!((up.state, up.parts_done, up.last_error, up.updated_at), (UploadState::Queued, 0, None, 10));
        assert_eq!(s.pending_uploads().unwrap().iter().map(|u| u.local_id).collect::<Vec<_>>(), [b, a], "más antigua primero");

        s.set_upload_progress(a, 2, 11).unwrap();
        s.set_upload_progress(a, 1, 12).unwrap(); // no retrocede
        let up = s.get_upload(a).unwrap().unwrap();
        assert_eq!((up.state, up.parts_done), (UploadState::Uploading, 2));
        assert_eq!(s.pause_interrupted_uploads(20).unwrap(), 1);
        assert_eq!(s.get_upload(a).unwrap().unwrap().state, UploadState::Paused);
        s.set_upload_progress(a, 99, 21).unwrap(); // se acota a parts_total y termina
        let up = s.get_upload(a).unwrap().unwrap();
        assert_eq!((up.state, up.parts_done), (UploadState::Done, 4));
        assert!(s.set_upload_progress(a, 4, 22).is_err(), "terminada: no se toca");

        assert!(s.set_upload_state(b, UploadState::Failed, Some("red caída"), 30).unwrap());
        assert_eq!(s.list_uploads(UploadState::Failed).unwrap()[0].last_error.as_deref(), Some("red caída"));
        assert!(s.pending_uploads().unwrap().is_empty());
        assert!(!s.set_upload_state(999, UploadState::Paused, None, 1).unwrap());
        assert_eq!(s.purge_finished_uploads(25).unwrap(), 1, "solo la terminada en 21");
        assert_eq!(s.purge_finished_uploads(100).unwrap(), 1);
        assert!(!s.delete_upload(a).unwrap());
    }

    #[test]
    fn destination_rules_and_cascade() {
        let s = Store::open_in_memory().unwrap();
        let k = s.upsert_dialog(&DialogRow::new(PeerKind::User, 1, 0)).unwrap();
        assert!(matches!(s.create_upload(&new_upload(None, None, 1, 1)), Err(Error::Invalid(_))));
        assert!(matches!(s.create_upload(&new_upload(Some(k), Some(1), 1, 1)), Err(Error::Invalid(_))));
        let mut bad = new_upload(Some(k), None, 1, 1);
        bad.part_size = 0;
        assert!(s.create_upload(&bad).is_err());
        let id = s.create_upload(&new_upload(Some(k), None, 1, 1)).unwrap();
        s.delete_dialog(k).unwrap();
        assert!(s.get_upload(id).unwrap().is_none(), "borrar el chat borra sus subidas");
    }
}
