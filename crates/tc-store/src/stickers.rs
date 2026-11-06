//! Medios mínimos, packs de stickers, recientes y favoritos.

use crate::db::Store;
use crate::error::Result;
use crate::types::*;
use rusqlite::{OptionalExtension, Row, params};

/// Máximo de stickers recientes que se conservan.
pub const RECENT_CAP: i64 = 200;

const STICKER_COLS: &str = "s.id, s.set_id, s.media_id, s.emoji, s.format, s.position";

fn sticker_from_row(r: &Row<'_>) -> rusqlite::Result<StickerRow> {
    Ok(StickerRow { id: r.get(0)?, set_id: r.get(1)?, media_id: r.get(2)?, emoji: r.get(3)?, format: r.get(4)?, position: r.get(5)? })
}

fn set_from_row(r: &Row<'_>) -> rusqlite::Result<StickerSetRow> {
    Ok(StickerSetRow {
        id: r.get(0)?,
        access_hash: r.get(1)?,
        short_name: r.get(2)?,
        title: r.get(3)?,
        format: r.get(4)?,
        is_emoji: r.get(5)?,
        thumb_media_id: r.get(6)?,
        installed: r.get(7)?,
        installed_order: r.get(8)?,
        archived: r.get(9)?,
        remote_hash: r.get(10)?,
        is_local: r.get(11)?,
        updated_at: r.get(12)?,
    })
}

const SET_COLS: &str = "id, access_hash, short_name, title, format, is_emoji, thumb_media_id, installed, \
    installed_order, archived, remote_hash, is_local, updated_at";

impl Store {
    /// Inserta o actualiza un medio (descargas: ver `local_path`).
    pub fn upsert_media(&self, m: &MediaRow) -> Result<()> {
        self.conn.execute(
            "INSERT INTO media (id, kind, mime, size, width, height, file_name, thumb_path, local_path, updated_at, duration_ms)
             VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11)
             ON CONFLICT(id) DO UPDATE SET kind=excluded.kind, mime=excluded.mime, size=excluded.size,
               width=excluded.width, height=excluded.height, file_name=excluded.file_name,
               thumb_path=excluded.thumb_path, local_path=excluded.local_path, updated_at=excluded.updated_at,
               duration_ms=excluded.duration_ms",
            params![m.id, m.kind, m.mime, m.size, m.width, m.height, m.file_name, m.thumb_path, m.local_path, m.updated_at, m.duration_ms],
        )?;
        Ok(())
    }

    /// Medio por id.
    pub fn get_media(&self, id: i64) -> Result<Option<MediaRow>> {
        Ok(self
            .conn
            .query_row(
                "SELECT id, kind, mime, size, width, height, file_name, thumb_path, local_path, updated_at, duration_ms
                 FROM media WHERE id=?1",
                [id],
                |r| {
                    Ok(MediaRow {
                        id: r.get(0)?,
                        kind: r.get(1)?,
                        mime: r.get(2)?,
                        size: r.get(3)?,
                        width: r.get(4)?,
                        height: r.get(5)?,
                        file_name: r.get(6)?,
                        thumb_path: r.get(7)?,
                        local_path: r.get(8)?,
                        updated_at: r.get(9)?,
                        duration_ms: r.get(10)?,
                    })
                },
            )
            .optional()?)
    }

    /// Guarda un pack y su contenido (los medios de los stickers deben existir). Los stickers
    /// que ya no están en `stickers` se eliminan (con sus recientes/favoritos).
    pub fn upsert_sticker_set(&self, set: &StickerSetRow, stickers: &[StickerRow]) -> Result<()> {
        let tx = self.conn.unchecked_transaction()?;
        tx.execute(
            "INSERT INTO sticker_sets (id, access_hash, short_name, title, format, is_emoji, sticker_count, thumb_media_id,
               installed, installed_order, archived, remote_hash, is_local, updated_at)
             VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14)
             ON CONFLICT(id) DO UPDATE SET access_hash=excluded.access_hash, short_name=excluded.short_name,
               title=excluded.title, format=excluded.format, is_emoji=excluded.is_emoji,
               sticker_count=excluded.sticker_count, thumb_media_id=excluded.thumb_media_id,
               installed=excluded.installed, installed_order=excluded.installed_order, archived=excluded.archived,
               remote_hash=excluded.remote_hash, is_local=excluded.is_local, updated_at=excluded.updated_at",
            params![
                set.id, set.access_hash, set.short_name, set.title, set.format, set.is_emoji, stickers.len() as i64,
                set.thumb_media_id, set.installed, set.installed_order, set.archived, set.remote_hash, set.is_local,
                set.updated_at
            ],
        )?;
        // Borrado de los que sobran (se compara en Rust: los packs son pequeños).
        let existing: Vec<i64> = {
            let mut st = tx.prepare("SELECT id FROM stickers WHERE set_id=?1")?;
            st.query_map([set.id], |r| r.get(0))?.collect::<rusqlite::Result<_>>()?
        };
        for id in existing.into_iter().filter(|id| !stickers.iter().any(|s| s.id == *id)) {
            tx.execute("DELETE FROM stickers WHERE id=?1", [id])?;
        }
        for s in stickers {
            tx.execute(
                "INSERT INTO stickers (id, set_id, media_id, emoji, format, position) VALUES (?1,?2,?3,?4,?5,?6)
                 ON CONFLICT(id) DO UPDATE SET set_id=excluded.set_id, media_id=excluded.media_id,
                   emoji=excluded.emoji, format=excluded.format, position=excluded.position",
                params![s.id, set.id, s.media_id, s.emoji, s.format, s.position],
            )?;
        }
        tx.commit()?;
        Ok(())
    }

    /// Pack por id.
    pub fn get_sticker_set(&self, id: i64) -> Result<Option<StickerSetRow>> {
        Ok(self
            .conn
            .query_row(&format!("SELECT {SET_COLS} FROM sticker_sets WHERE id=?1"), [id], set_from_row)
            .optional()?)
    }

    /// Packs instalados (y no archivados) en su orden.
    pub fn installed_sticker_sets(&self) -> Result<Vec<StickerSetRow>> {
        let mut st = self.conn.prepare(&format!(
            "SELECT {SET_COLS} FROM sticker_sets WHERE installed=1 AND archived=0 ORDER BY installed_order, id"
        ))?;
        let rows = st.query_map([], set_from_row)?;
        Ok(rows.collect::<rusqlite::Result<_>>()?)
    }

    /// Instala o desinstala un pack.
    pub fn set_sticker_set_installed(&self, id: i64, installed: bool, order: Option<i64>) -> Result<()> {
        let n = self.conn.execute(
            "UPDATE sticker_sets SET installed=?2, installed_order=?3 WHERE id=?1",
            params![id, installed, if installed { order } else { None }],
        )?;
        if n == 0 {
            return Err(crate::Error::NotFound(format!("pack {id}")));
        }
        Ok(())
    }

    /// Stickers de un pack en su orden.
    pub fn set_stickers(&self, set_id: i64) -> Result<Vec<StickerRow>> {
        let mut st = self.conn.prepare(&format!("SELECT {STICKER_COLS} FROM stickers s WHERE s.set_id=?1 ORDER BY s.position, s.id"))?;
        let rows = st.query_map([set_id], sticker_from_row)?;
        Ok(rows.collect::<rusqlite::Result<_>>()?)
    }

    /// Stickers de packs instalados asociados a un emoji.
    pub fn stickers_by_emoji(&self, emoji: &str) -> Result<Vec<StickerRow>> {
        let mut st = self.conn.prepare(&format!(
            "SELECT {STICKER_COLS} FROM stickers s JOIN sticker_sets p ON p.id=s.set_id
             WHERE s.emoji=?1 AND p.installed=1 ORDER BY p.installed_order, s.position"
        ))?;
        let rows = st.query_map([emoji], sticker_from_row)?;
        Ok(rows.collect::<rusqlite::Result<_>>()?)
    }

    /// Registra un uso (el más reciente primero) y recorta a [`RECENT_CAP`].
    pub fn record_recent_sticker(&self, sticker_id: i64, now: i64) -> Result<()> {
        let tx = self.conn.unchecked_transaction()?;
        tx.execute(
            "INSERT INTO sticker_recent (sticker_id, used_at) VALUES (?1,?2)
             ON CONFLICT(sticker_id) DO UPDATE SET used_at=excluded.used_at",
            params![sticker_id, now],
        )?;
        tx.execute(
            "DELETE FROM sticker_recent WHERE sticker_id NOT IN
               (SELECT sticker_id FROM sticker_recent ORDER BY used_at DESC, rowid DESC LIMIT ?1)",
            [RECENT_CAP],
        )?;
        tx.commit()?;
        Ok(())
    }

    /// Recientes, el más reciente primero.
    pub fn recent_stickers(&self, limit: usize) -> Result<Vec<StickerRow>> {
        let mut st = self.conn.prepare(&format!(
            "SELECT {STICKER_COLS} FROM sticker_recent r JOIN stickers s ON s.id=r.sticker_id
             ORDER BY r.used_at DESC, r.rowid DESC LIMIT ?1"
        ))?;
        let rows = st.query_map([limit as i64], sticker_from_row)?;
        Ok(rows.collect::<rusqlite::Result<_>>()?)
    }

    /// Vacía los recientes.
    pub fn clear_recent_stickers(&self) -> Result<()> {
        self.conn.execute("DELETE FROM sticker_recent", [])?;
        Ok(())
    }

    /// Añade a favoritos (idempotente).
    pub fn add_favorite_sticker(&self, sticker_id: i64, now: i64) -> Result<()> {
        self.conn.execute("INSERT OR IGNORE INTO sticker_faves (sticker_id, added_at) VALUES (?1,?2)", params![sticker_id, now])?;
        Ok(())
    }

    /// Quita de favoritos; devuelve si estaba.
    pub fn remove_favorite_sticker(&self, sticker_id: i64) -> Result<bool> {
        Ok(self.conn.execute("DELETE FROM sticker_faves WHERE sticker_id=?1", [sticker_id])? > 0)
    }

    /// ¿Es favorito?
    pub fn is_favorite_sticker(&self, sticker_id: i64) -> Result<bool> {
        Ok(self.conn.query_row("SELECT EXISTS(SELECT 1 FROM sticker_faves WHERE sticker_id=?1)", [sticker_id], |r| r.get(0))?)
    }

    /// Favoritos, el último añadido primero.
    pub fn favorite_stickers(&self) -> Result<Vec<StickerRow>> {
        let mut st = self.conn.prepare(&format!(
            "SELECT {STICKER_COLS} FROM sticker_faves f JOIN stickers s ON s.id=f.sticker_id ORDER BY f.added_at DESC, f.rowid DESC"
        ))?;
        let rows = st.query_map([], sticker_from_row)?;
        Ok(rows.collect::<rusqlite::Result<_>>()?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn seed(s: &Store, n: i64) -> (StickerSetRow, Vec<StickerRow>) {
        let set = StickerSetRow {
            id: 1,
            access_hash: None,
            short_name: "brasa".into(),
            title: "Brasa".into(),
            format: "static".into(),
            is_emoji: false,
            thumb_media_id: None,
            installed: true,
            installed_order: Some(0),
            archived: false,
            remote_hash: None,
            is_local: true,
            updated_at: 1,
        };
        let mut st = Vec::new();
        for i in 1..=n {
            s.upsert_media(&MediaRow { id: 1000 + i, kind: "sticker".into(), updated_at: 1, ..Default::default() }).unwrap();
            st.push(StickerRow { id: i, set_id: 1, media_id: 1000 + i, emoji: if i % 2 == 0 { "😀".into() } else { "🔥".into() }, format: "webp".into(), position: i });
        }
        s.upsert_sticker_set(&set, &st).unwrap();
        (set, st)
    }

    #[test]
    fn sets_and_stickers() {
        let s = Store::open_in_memory().unwrap();
        let (set, st) = seed(&s, 4);
        assert_eq!(s.installed_sticker_sets().unwrap().len(), 1);
        assert_eq!(s.get_sticker_set(1).unwrap().unwrap().short_name, "brasa");
        assert_eq!(s.set_stickers(1).unwrap(), st);
        assert_eq!(s.stickers_by_emoji("😀").unwrap().len(), 2);
        // re-sincronizar con menos stickers elimina los que sobran
        s.upsert_sticker_set(&set, &st[..2]).unwrap();
        assert_eq!(s.set_stickers(1).unwrap().len(), 2);
        let n: i64 = s.conn.query_row("SELECT sticker_count FROM sticker_sets WHERE id=1", [], |r| r.get(0)).unwrap();
        assert_eq!(n, 2);
        s.set_sticker_set_installed(1, false, None).unwrap();
        assert!(s.installed_sticker_sets().unwrap().is_empty());
        assert!(s.stickers_by_emoji("😀").unwrap().is_empty());
        assert!(s.set_sticker_set_installed(9, true, None).is_err());
    }

    #[test]
    fn recents_order_dedupe_cap_and_cascade() {
        let s = Store::open_in_memory().unwrap();
        let (set, st) = seed(&s, 3);
        s.record_recent_sticker(1, 10).unwrap();
        s.record_recent_sticker(2, 20).unwrap();
        s.record_recent_sticker(1, 30).unwrap();
        let ids: Vec<i64> = s.recent_stickers(10).unwrap().iter().map(|x| x.id).collect();
        assert_eq!(ids, [1, 2]);
        // tope
        let many: Vec<_> = (1..=RECENT_CAP + 20).map(|i| {
            s.upsert_media(&MediaRow { id: 5000 + i, kind: "sticker".into(), updated_at: 1, ..Default::default() }).unwrap();
            StickerRow { id: 100 + i, set_id: 1, media_id: 5000 + i, emoji: String::new(), format: "webp".into(), position: 10 + i }
        }).collect();
        let mut all = st.clone();
        all.extend(many.clone());
        s.upsert_sticker_set(&set, &all).unwrap();
        for (i, m) in many.iter().enumerate() {
            s.record_recent_sticker(m.id, 100 + i as i64).unwrap();
        }
        assert_eq!(s.recent_stickers(1000).unwrap().len() as i64, RECENT_CAP);
        assert_eq!(s.recent_stickers(1).unwrap()[0].id, many.last().unwrap().id);
        // borrar un sticker del pack limpia sus recientes
        s.upsert_sticker_set(&set, &st).unwrap();
        assert!(s.recent_stickers(1000).unwrap().iter().all(|x| x.id <= 3));
        s.clear_recent_stickers().unwrap();
        assert!(s.recent_stickers(10).unwrap().is_empty());
    }

    #[test]
    fn favorites() {
        let s = Store::open_in_memory().unwrap();
        seed(&s, 3);
        s.add_favorite_sticker(2, 10).unwrap();
        s.add_favorite_sticker(2, 11).unwrap();
        s.add_favorite_sticker(3, 12).unwrap();
        assert!(s.is_favorite_sticker(2).unwrap());
        assert_eq!(s.favorite_stickers().unwrap().iter().map(|x| x.id).collect::<Vec<_>>(), [3, 2]);
        assert!(s.remove_favorite_sticker(2).unwrap());
        assert!(!s.remove_favorite_sticker(2).unwrap());
        assert!(!s.is_favorite_sticker(2).unwrap());
        assert!(s.add_favorite_sticker(999, 1).is_err(), "FK: el sticker debe existir");
    }
}
