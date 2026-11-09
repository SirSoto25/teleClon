//! Repositorios de la migración 0004: fotos de perfil por interlocutor, referencias de descarga de
//! medios y el índice LRU de la caché de archivos en disco (`MediaCache`).

use crate::db::Store;
use crate::error::{Error, Result};
use crate::types::PeerKind;
use rusqlite::{OptionalExtension, params};

/// Referencia de descarga de un medio (espejo de `tc_core::FileRef`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MediaRefRow {
    pub media_id: i64,
    /// `photo` o `document`.
    pub kind: String,
    pub access_hash: i64,
    pub file_reference: Vec<u8>,
    pub dc_id: i64,
    pub size: i64,
    pub thumb: Option<String>,
    pub full: Option<String>,
}

/// Entrada del índice de la caché de archivos.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CacheEntry {
    pub key: String,
    /// Nombre del archivo dentro del directorio de la caché.
    pub file: String,
    pub size: i64,
    /// Último uso en milisegundos Unix.
    pub last_used_at: i64,
}

fn photo_kind(kind: PeerKind) -> Result<&'static str> {
    match kind {
        PeerKind::User | PeerKind::Group => Ok(kind.as_str()),
        PeerKind::Secret => Err(Error::Invalid("los chats secretos no guardan foto".into())),
    }
}

impl Store {
    /// Fija (o borra con `None`) la foto de perfil `(photo_id, dc_id)` de un usuario o grupo.
    pub fn set_peer_photo(&self, kind: PeerKind, peer_id: i64, photo: Option<(i64, i64)>) -> Result<()> {
        let k = photo_kind(kind)?;
        match photo {
            Some((photo_id, dc)) => self.conn.execute(
                "INSERT INTO peer_photos (peer_kind, peer_id, photo_id, dc_id) VALUES (?1,?2,?3,?4)
                 ON CONFLICT(peer_kind, peer_id) DO UPDATE SET photo_id=excluded.photo_id, dc_id=excluded.dc_id",
                params![k, peer_id, photo_id, dc],
            )?,
            None => self.conn.execute("DELETE FROM peer_photos WHERE peer_kind=?1 AND peer_id=?2", params![k, peer_id])?,
        };
        Ok(())
    }

    /// Foto de perfil `(photo_id, dc_id)` guardada. Los chats secretos nunca tienen.
    pub fn get_peer_photo(&self, kind: PeerKind, peer_id: i64) -> Result<Option<(i64, i64)>> {
        if kind == PeerKind::Secret {
            return Ok(None);
        }
        Ok(self
            .conn
            .query_row("SELECT photo_id, dc_id FROM peer_photos WHERE peer_kind=?1 AND peer_id=?2", params![kind.as_str(), peer_id], |r| {
                Ok((r.get(0)?, r.get(1)?))
            })
            .optional()?)
    }

    /// Guarda la referencia de descarga de un medio (el medio debe existir en `media`).
    pub fn upsert_media_ref(&self, r: &MediaRefRow) -> Result<()> {
        self.conn.execute(
            "INSERT INTO media_refs (media_id, kind, access_hash, file_reference, dc_id, size, thumb, full)
             VALUES (?1,?2,?3,?4,?5,?6,?7,?8)
             ON CONFLICT(media_id) DO UPDATE SET kind=excluded.kind, access_hash=excluded.access_hash,
               file_reference=excluded.file_reference, dc_id=excluded.dc_id, size=excluded.size,
               thumb=excluded.thumb, full=excluded.full",
            params![r.media_id, r.kind, r.access_hash, r.file_reference, r.dc_id, r.size, r.thumb, r.full],
        )?;
        Ok(())
    }

    pub fn get_media_ref(&self, media_id: i64) -> Result<Option<MediaRefRow>> {
        Ok(self
            .conn
            .query_row(
                "SELECT media_id, kind, access_hash, file_reference, dc_id, size, thumb, full FROM media_refs WHERE media_id=?1",
                [media_id],
                |r| {
                    Ok(MediaRefRow {
                        media_id: r.get(0)?,
                        kind: r.get(1)?,
                        access_hash: r.get(2)?,
                        file_reference: r.get(3)?,
                        dc_id: r.get(4)?,
                        size: r.get(5)?,
                        thumb: r.get(6)?,
                        full: r.get(7)?,
                    })
                },
            )
            .optional()?)
    }

    pub fn delete_media_ref(&self, media_id: i64) -> Result<()> {
        self.conn.execute("DELETE FROM media_refs WHERE media_id=?1", [media_id])?;
        Ok(())
    }

    // ---- índice de la caché de archivos -------------------------------------------------------

    pub fn cache_entry(&self, key: &str) -> Result<Option<CacheEntry>> {
        Ok(self
            .conn
            .query_row("SELECT key, file, size, last_used_at FROM file_cache WHERE key=?1", [key], |r| {
                Ok(CacheEntry { key: r.get(0)?, file: r.get(1)?, size: r.get(2)?, last_used_at: r.get(3)? })
            })
            .optional()?)
    }

    pub fn cache_put(&self, e: &CacheEntry) -> Result<()> {
        self.conn.execute(
            "INSERT INTO file_cache (key, file, size, last_used_at) VALUES (?1,?2,?3,?4)
             ON CONFLICT(key) DO UPDATE SET file=excluded.file, size=excluded.size, last_used_at=excluded.last_used_at",
            params![e.key, e.file, e.size, e.last_used_at],
        )?;
        Ok(())
    }

    pub fn cache_touch(&self, key: &str, now_ms: i64) -> Result<()> {
        self.conn.execute("UPDATE file_cache SET last_used_at=?2 WHERE key=?1", params![key, now_ms])?;
        Ok(())
    }

    pub fn cache_remove(&self, key: &str) -> Result<bool> {
        Ok(self.conn.execute("DELETE FROM file_cache WHERE key=?1", [key])? > 0)
    }

    /// Bytes totales indexados.
    pub fn cache_total(&self) -> Result<i64> {
        Ok(self.conn.query_row("SELECT COALESCE(SUM(size), 0) FROM file_cache", [], |r| r.get(0))?)
    }

    /// Entradas de uso más antiguo primero.
    pub fn cache_oldest(&self, limit: usize) -> Result<Vec<CacheEntry>> {
        let mut st = self.conn.prepare("SELECT key, file, size, last_used_at FROM file_cache ORDER BY last_used_at, key LIMIT ?1")?;
        let rows = st.query_map([i64::try_from(limit).unwrap_or(i64::MAX)], |r| {
            Ok(CacheEntry { key: r.get(0)?, file: r.get(1)?, size: r.get(2)?, last_used_at: r.get(3)? })
        })?;
        Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
    }

    pub fn cache_all(&self) -> Result<Vec<CacheEntry>> {
        self.cache_oldest(usize::MAX)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::MediaRow;

    #[test]
    fn peer_photos_roundtrip_and_never_for_secrets() {
        let s = Store::open_in_memory().unwrap();
        s.set_peer_photo(PeerKind::User, 1, Some((10, 2))).unwrap();
        s.set_peer_photo(PeerKind::Group, 1, Some((11, 4))).unwrap();
        assert_eq!(s.get_peer_photo(PeerKind::User, 1).unwrap(), Some((10, 2)));
        assert_eq!(s.get_peer_photo(PeerKind::Group, 1).unwrap(), Some((11, 4)));
        s.set_peer_photo(PeerKind::User, 1, Some((12, 2))).unwrap();
        assert_eq!(s.get_peer_photo(PeerKind::User, 1).unwrap(), Some((12, 2)));
        s.set_peer_photo(PeerKind::User, 1, None).unwrap();
        assert_eq!(s.get_peer_photo(PeerKind::User, 1).unwrap(), None);
        assert!(s.set_peer_photo(PeerKind::Secret, 1, Some((1, 1))).is_err());
        assert_eq!(s.get_peer_photo(PeerKind::Secret, 1).unwrap(), None);
    }

    #[test]
    fn media_refs_follow_their_media() {
        let s = Store::open_in_memory().unwrap();
        s.upsert_media(&MediaRow { id: 5, kind: "photo".into(), updated_at: 1, ..Default::default() }).unwrap();
        let r = MediaRefRow {
            media_id: 5,
            kind: "photo".into(),
            access_hash: 7,
            file_reference: vec![1, 2, 3],
            dc_id: 4,
            size: 900,
            thumb: Some("x".into()),
            full: Some("y".into()),
        };
        s.upsert_media_ref(&r).unwrap();
        assert_eq!(s.get_media_ref(5).unwrap(), Some(r));
        // Sin medio no hay referencia (clave foránea).
        let orphan = MediaRefRow { media_id: 6, kind: "document".into(), access_hash: 0, file_reference: vec![], dc_id: 1, size: 0, thumb: None, full: None };
        assert!(s.upsert_media_ref(&orphan).is_err());
    }

    #[test]
    fn cache_index_orders_by_last_use() {
        let s = Store::open_in_memory().unwrap();
        for (k, t) in [("a", 3), ("b", 1), ("c", 2)] {
            s.cache_put(&CacheEntry { key: k.into(), file: format!("{k}.bin"), size: 10, last_used_at: t }).unwrap();
        }
        assert_eq!(s.cache_total().unwrap(), 30);
        let keys: Vec<String> = s.cache_oldest(2).unwrap().into_iter().map(|e| e.key).collect();
        assert_eq!(keys, ["b", "c"]);
        s.cache_touch("b", 9).unwrap();
        assert_eq!(s.cache_oldest(1).unwrap()[0].key, "c");
        assert!(s.cache_remove("c").unwrap());
        assert_eq!(s.cache_entry("c").unwrap(), None);
        assert_eq!(s.cache_all().unwrap().len(), 2);
    }
}
