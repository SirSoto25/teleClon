//! Panel del compositor: uso de emojis (recientes y frecuentes) y caché de los GIFs guardados
//! (migración 0007). `MediaCacheLayer` y `CachedBackend` lo usan para:
//!
//! * `emoji_usage` / `record_emoji_use`: se guardan en local (la red no los conoce).
//! * `saved_gifs`: cada lista que llega de la red reemplaza la guardada; si la red falla por algo
//!   pasajero (sin conexión, `FLOOD_WAIT`) se devuelve la guardada para que el panel no quede vacío.
use std::sync::Arc;

use rusqlite::params;
use tc_core::{BackendError, BackendResult, EmojiUse, FileKind, FileRef, Gif, Timestamp};

use crate::db::Store;
use crate::error::Result;
use crate::media_cache::MediaCache;

/// Recientes que se conservan (los más viejos se borran).
pub const EMOJI_KEEP: usize = 200;

impl Store {
    /// Uso de emojis, más reciente primero.
    pub fn emoji_usage(&self) -> Result<Vec<EmojiUse>> {
        let mut st = self.conn.prepare("SELECT emoji, uses, last_used FROM emoji_usage ORDER BY last_used DESC, uses DESC, emoji")?;
        let rows = st.query_map([], |r| Ok(EmojiUse { emoji: r.get(0)?, count: r.get::<_, i64>(1)?.clamp(0, i64::from(u32::MAX)) as u32, last_used: Timestamp(r.get(2)?) }))?;
        Ok(rows.collect::<rusqlite::Result<_>>()?)
    }

    /// Suma un uso de `emoji` en `at` y recorta la tabla a `EMOJI_KEEP` filas.
    pub fn record_emoji_use(&self, emoji: &str, at: i64) -> Result<()> {
        self.conn.execute(
            "INSERT INTO emoji_usage (emoji, uses, last_used) VALUES (?1, 1, ?2)
             ON CONFLICT(emoji) DO UPDATE SET uses = uses + 1, last_used = max(last_used, excluded.last_used)",
            params![emoji, at],
        )?;
        self.conn.execute(
            "DELETE FROM emoji_usage WHERE emoji NOT IN (SELECT emoji FROM emoji_usage ORDER BY last_used DESC LIMIT ?1)",
            [EMOJI_KEEP as i64],
        )?;
        Ok(())
    }

    /// GIFs guardados de la última carga correcta, en su orden.
    pub fn saved_gifs(&self) -> Result<Vec<Gif>> {
        let mut st = self.conn.prepare(
            "SELECT id, mime, width, height, duration_secs, title, access_hash, file_reference, dc_id, size_bytes, thumb
             FROM saved_gifs ORDER BY position",
        )?;
        let rows = st.query_map([], |r| {
            let id: i64 = r.get(0)?;
            Ok(Gif {
                id,
                mime: r.get(1)?,
                width: r.get::<_, i64>(2)?.max(0) as u32,
                height: r.get::<_, i64>(3)?.max(0) as u32,
                duration_secs: r.get::<_, Option<i64>>(4)?.map(|d| d.max(0) as u32),
                title: r.get(5)?,
                file: Some(FileRef {
                    kind: FileKind::Document,
                    id,
                    access_hash: r.get(6)?,
                    file_reference: r.get(7)?,
                    dc_id: r.get(8)?,
                    size_bytes: r.get::<_, i64>(9)?.max(0) as u64,
                    thumb: r.get(10)?,
                    full: None,
                }),
                inline: None,
            })
        })?;
        Ok(rows.collect::<rusqlite::Result<_>>()?)
    }

    /// Sustituye la lista de GIFs guardados (los que no tienen documento no se guardan).
    pub fn put_saved_gifs(&self, gifs: &[Gif]) -> Result<()> {
        let tx = self.conn.unchecked_transaction()?;
        tx.execute("DELETE FROM saved_gifs", [])?;
        {
            let mut ins = tx.prepare(
                "INSERT OR REPLACE INTO saved_gifs (id, position, mime, width, height, duration_secs, title, access_hash, file_reference, dc_id, size_bytes, thumb)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)",
            )?;
            for (i, g) in gifs.iter().enumerate() {
                let Some(f) = &g.file else { continue };
                ins.execute(params![
                    g.id,
                    i as i64,
                    g.mime,
                    i64::from(g.width),
                    i64::from(g.height),
                    g.duration_secs.map(i64::from),
                    g.title,
                    f.access_hash,
                    f.file_reference,
                    f.dc_id,
                    f.size_bytes as i64,
                    f.thumb
                ])?;
            }
        }
        tx.commit()?;
        Ok(())
    }
}

fn storage(e: crate::Error) -> BackendError {
    BackendError::Storage(e.to_string())
}

/// ¿Merece la pena devolver lo guardado en lugar del error?
fn transient(e: &BackendError) -> bool {
    matches!(e, BackendError::Network | BackendError::FloodWait(_) | BackendError::Internal)
}

async fn blocking<T: Send + 'static>(f: impl FnOnce() -> Result<T> + Send + 'static) -> BackendResult<T> {
    tokio::task::spawn_blocking(f).await.map_err(|_| BackendError::Internal)?.map_err(storage)
}

/// Lógica compartida por los decoradores de tc-store sobre una base de la cuenta.
pub(crate) async fn emoji_usage(cache: Arc<MediaCache>) -> BackendResult<Vec<EmojiUse>> {
    blocking(move || cache.store().emoji_usage()).await
}

pub(crate) async fn record_emoji_use(cache: Arc<MediaCache>, emoji: String, at: Timestamp) -> BackendResult<()> {
    blocking(move || cache.store().record_emoji_use(&emoji, at.0)).await
}

/// Resultado de la red guardado; o, si falló por algo pasajero, lo último guardado.
pub(crate) async fn saved_gifs(cache: Arc<MediaCache>, net: BackendResult<Vec<Gif>>) -> BackendResult<Vec<Gif>> {
    match net {
        Ok(list) => {
            let keep = list.clone();
            if let Err(e) = blocking(move || cache.store().put_saved_gifs(&keep)).await {
                tracing::warn!(error = %e, "no se pudo guardar la lista de GIFs");
            }
            Ok(list)
        }
        Err(e) if transient(&e) => {
            let cached = blocking(move || cache.store().saved_gifs()).await.unwrap_or_default();
            if cached.is_empty() { Err(e) } else { Ok(cached) }
        }
        Err(e) => Err(e),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn gif(id: i64, mime: &str) -> Gif {
        Gif {
            id,
            file: Some(FileRef { kind: FileKind::Document, id, access_hash: id * 3, file_reference: vec![1, 2], dc_id: 4, size_bytes: 900, thumb: Some("m".into()), full: None }),
            mime: mime.into(),
            width: 320,
            height: 180,
            duration_secs: Some(2),
            title: format!("g{id}"),
            inline: None,
        }
    }

    #[test]
    fn uso_de_emojis_suma_y_ordena_por_reciente() {
        let s = Store::open_in_memory().unwrap();
        s.record_emoji_use("😀", 10).unwrap();
        s.record_emoji_use("👍", 20).unwrap();
        s.record_emoji_use("😀", 5).unwrap();
        let u = s.emoji_usage().unwrap();
        assert_eq!(u.iter().map(|e| (e.emoji.as_str(), e.count, e.last_used.0)).collect::<Vec<_>>(), [("👍", 1, 20), ("😀", 2, 10)]);
        for i in 0..(EMOJI_KEEP as i64 + 5) {
            s.record_emoji_use(&format!("e{i}"), 100 + i).unwrap();
        }
        assert_eq!(s.emoji_usage().unwrap().len(), EMOJI_KEEP);
    }

    #[test]
    fn gifs_guardados_ida_y_vuelta_y_reemplazo() {
        let s = Store::open_in_memory().unwrap();
        assert!(s.saved_gifs().unwrap().is_empty());
        let list = vec![gif(7, "video/mp4"), gif(3, "image/gif")];
        s.put_saved_gifs(&list).unwrap();
        assert_eq!(s.saved_gifs().unwrap(), list);
        s.put_saved_gifs(&list[1..]).unwrap();
        assert_eq!(s.saved_gifs().unwrap(), list[1..]);
    }

    #[tokio::test]
    async fn sin_red_se_devuelven_los_guardados() {
        let dir = std::env::temp_dir().join(format!("tc-store-gifs-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let cache = Arc::new(MediaCache::open(&dir, Store::open_in_memory().unwrap(), 1 << 20).unwrap());
        assert_eq!(saved_gifs(cache.clone(), Err(BackendError::Network)).await, Err(BackendError::Network), "nada guardado aún");
        let list = vec![gif(1, "video/mp4")];
        assert_eq!(saved_gifs(cache.clone(), Ok(list.clone())).await.unwrap(), list);
        assert_eq!(saved_gifs(cache.clone(), Err(BackendError::Network)).await.unwrap(), list);
        assert_eq!(saved_gifs(cache.clone(), Err(BackendError::Unauthorized)).await, Err(BackendError::Unauthorized));
        record_emoji_use(cache.clone(), "🔥".into(), Timestamp(9)).await.unwrap();
        assert_eq!(emoji_usage(cache).await.unwrap()[0].emoji, "🔥");
        let _ = std::fs::remove_dir_all(&dir);
    }
}
