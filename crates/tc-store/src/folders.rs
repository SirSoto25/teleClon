//! Carpetas (filtros de chats) y sus chats explícitos.

use crate::db::Store;
use crate::error::{Error, Result};
use crate::types::*;
use rusqlite::{OptionalExtension, Row, params};

const COLS: &str = "id, title, emoticon, sort_order, inc_contacts, inc_non_contacts, inc_groups, inc_channels, \
    inc_bots, exc_muted, exc_read, exc_archived, updated_at";

fn from_row(r: &Row<'_>) -> rusqlite::Result<FolderRow> {
    Ok(FolderRow {
        id: r.get(0)?,
        title: r.get(1)?,
        emoticon: r.get(2)?,
        sort_order: r.get(3)?,
        inc_contacts: r.get(4)?,
        inc_non_contacts: r.get(5)?,
        inc_groups: r.get(6)?,
        inc_channels: r.get(7)?,
        inc_bots: r.get(8)?,
        exc_muted: r.get(9)?,
        exc_read: r.get(10)?,
        exc_archived: r.get(11)?,
        updated_at: r.get(12)?,
    })
}

impl Store {
    /// Inserta o actualiza una carpeta (idempotente por `id`).
    pub fn upsert_folder(&self, f: &FolderRow) -> Result<()> {
        self.conn.execute(
            "INSERT INTO folders (id, title, emoticon, sort_order, inc_contacts, inc_non_contacts, inc_groups,
               inc_channels, inc_bots, exc_muted, exc_read, exc_archived, updated_at)
             VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13)
             ON CONFLICT(id) DO UPDATE SET title=excluded.title, emoticon=excluded.emoticon,
               sort_order=excluded.sort_order, inc_contacts=excluded.inc_contacts,
               inc_non_contacts=excluded.inc_non_contacts, inc_groups=excluded.inc_groups,
               inc_channels=excluded.inc_channels, inc_bots=excluded.inc_bots, exc_muted=excluded.exc_muted,
               exc_read=excluded.exc_read, exc_archived=excluded.exc_archived, updated_at=excluded.updated_at",
            params![
                f.id, f.title, f.emoticon, f.sort_order, f.inc_contacts, f.inc_non_contacts, f.inc_groups,
                f.inc_channels, f.inc_bots, f.exc_muted, f.exc_read, f.exc_archived, f.updated_at
            ],
        )?;
        Ok(())
    }

    /// Carpeta por id.
    pub fn get_folder(&self, id: i64) -> Result<Option<FolderRow>> {
        Ok(self.conn.query_row(&format!("SELECT {COLS} FROM folders WHERE id=?1"), [id], from_row).optional()?)
    }

    /// Carpetas en su orden.
    pub fn list_folders(&self) -> Result<Vec<FolderRow>> {
        let mut st = self.conn.prepare(&format!("SELECT {COLS} FROM folders ORDER BY sort_order, id"))?;
        let rows = st.query_map([], from_row)?;
        Ok(rows.collect::<rusqlite::Result<_>>()?)
    }

    /// Borra la carpeta y sus chats explícitos (no los diálogos).
    pub fn delete_folder(&self, id: i64) -> Result<bool> {
        Ok(self.conn.execute("DELETE FROM folders WHERE id=?1", [id])? > 0)
    }

    /// Reordena: `ids` recibe `sort_order` 0, 1, 2... y las carpetas no mencionadas pasan detrás,
    /// conservando su orden relativo.
    pub fn reorder_folders(&self, ids: &[i64]) -> Result<()> {
        let tx = self.conn.unchecked_transaction()?;
        let rest: Vec<i64> = {
            let mut st = tx.prepare("SELECT id FROM folders ORDER BY sort_order, id")?;
            let all: Vec<i64> = st.query_map([], |r| r.get(0))?.collect::<rusqlite::Result<_>>()?;
            all.into_iter().filter(|i| !ids.contains(i)).collect()
        };
        for (pos, id) in ids.iter().chain(rest.iter()).enumerate() {
            tx.execute("UPDATE folders SET sort_order=?2 WHERE id=?1", params![id, pos as i64])?;
        }
        tx.commit()?;
        Ok(())
    }

    /// Sustituye de golpe los chats explícitos de una carpeta.
    pub fn set_folder_peers(&self, folder_id: i64, peers: &[FolderPeerRow]) -> Result<()> {
        let tx = self.conn.unchecked_transaction()?;
        let exists: bool = tx.query_row("SELECT EXISTS(SELECT 1 FROM folders WHERE id=?1)", [folder_id], |r| r.get(0))?;
        if !exists {
            return Err(Error::NotFound(format!("carpeta {folder_id}")));
        }
        tx.execute("DELETE FROM folder_peers WHERE folder_id=?1", [folder_id])?;
        for p in peers {
            tx.execute(
                "INSERT OR REPLACE INTO folder_peers (folder_id, chat_key, mode, position) VALUES (?1,?2,?3,?4)",
                params![folder_id, p.chat_key, p.mode, p.position],
            )?;
        }
        tx.commit()?;
        Ok(())
    }

    /// Chats explícitos de una carpeta (fijados por posición primero).
    pub fn folder_peers(&self, folder_id: i64) -> Result<Vec<FolderPeerRow>> {
        let mut st = self.conn.prepare(
            "SELECT chat_key, mode, position FROM folder_peers WHERE folder_id=?1
             ORDER BY mode <> 'pinned', position, chat_key",
        )?;
        let rows = st.query_map([folder_id], |r| Ok(FolderPeerRow { chat_key: r.get(0)?, mode: r.get(1)?, position: r.get(2)? }))?;
        Ok(rows.collect::<rusqlite::Result<_>>()?)
    }

    /// Carpetas que incluyen explícitamente (o fijan) un chat.
    pub fn folders_of_chat(&self, chat_key: i64) -> Result<Vec<i64>> {
        let mut st = self.conn.prepare(
            "SELECT DISTINCT folder_id FROM folder_peers WHERE chat_key=?1 AND mode IN ('include','pinned') ORDER BY folder_id",
        )?;
        let rows = st.query_map([chat_key], |r| r.get(0))?;
        Ok(rows.collect::<rusqlite::Result<_>>()?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn folder(id: i64, title: &str, order: i64) -> FolderRow {
        FolderRow { id, title: title.into(), sort_order: order, updated_at: 1, ..Default::default() }
    }

    #[test]
    fn folders_crud_order_and_reorder() {
        let s = Store::open_in_memory().unwrap();
        let mut f = folder(2, "Trabajo", 0);
        f.emoticon = Some("💼".into());
        f.inc_groups = true;
        f.exc_muted = true;
        s.upsert_folder(&f).unwrap();
        s.upsert_folder(&folder(5, "Personal", 1)).unwrap();
        s.upsert_folder(&folder(9, "Bots", 2)).unwrap();
        assert_eq!(s.get_folder(2).unwrap().unwrap(), f);
        f.title = "Curro".into();
        s.upsert_folder(&f).unwrap();
        assert_eq!(s.get_folder(2).unwrap().unwrap().title, "Curro");
        s.reorder_folders(&[9, 2]).unwrap();
        let order: Vec<_> = s.list_folders().unwrap().into_iter().map(|f| f.id).collect();
        assert_eq!(order, [9, 2, 5]);
        assert!(s.delete_folder(2).unwrap());
        assert!(!s.delete_folder(2).unwrap());
    }

    #[test]
    fn folder_peers_replace_and_cascade() {
        let s = Store::open_in_memory().unwrap();
        s.upsert_folder(&folder(1, "A", 0)).unwrap();
        s.upsert_folder(&folder(2, "B", 1)).unwrap();
        let k1 = s.upsert_dialog(&DialogRow::new(PeerKind::User, 1, 0)).unwrap();
        let k2 = s.upsert_dialog(&DialogRow::new(PeerKind::Group, 2, 0)).unwrap();
        let k3 = s.upsert_dialog(&DialogRow::new(PeerKind::User, 3, 0)).unwrap();
        let p = |chat_key, mode, position| FolderPeerRow { chat_key, mode, position };
        s.set_folder_peers(1, &[p(k1, FolderPeerMode::Include, None), p(k2, FolderPeerMode::Pinned, Some(0)), p(k3, FolderPeerMode::Exclude, None)]).unwrap();
        s.set_folder_peers(2, &[p(k1, FolderPeerMode::Include, None)]).unwrap();
        let got = s.folder_peers(1).unwrap();
        assert_eq!(got[0].chat_key, k2, "los fijados primero");
        assert_eq!(got.len(), 3);
        assert_eq!(s.folders_of_chat(k1).unwrap(), [1, 2]);
        assert_eq!(s.folders_of_chat(k3).unwrap(), Vec::<i64>::new(), "excluir no es pertenecer");
        s.set_folder_peers(1, &[p(k3, FolderPeerMode::Include, None)]).unwrap();
        assert_eq!(s.folder_peers(1).unwrap().len(), 1);
        assert!(matches!(s.set_folder_peers(99, &[]), Err(Error::NotFound(_))));
        s.delete_dialog(k1).unwrap();
        assert!(s.folder_peers(2).unwrap().is_empty(), "borrar el diálogo lo saca de las carpetas");
        s.delete_folder(1).unwrap();
        let n: i64 = s.conn.query_row("SELECT COUNT(*) FROM folder_peers WHERE folder_id=1", [], |r| r.get(0)).unwrap();
        assert_eq!(n, 0);
    }
}
