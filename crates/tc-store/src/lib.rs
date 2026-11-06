//! Caché local SQLite de teleClon (un archivo por cuenta): apertura con PRAGMAs (WAL,
//! `foreign_keys`, `secure_delete`), migraciones con `PRAGMA user_version` (ver `migrations/`)
//! y repositorios síncronos para diálogos, mensajes (paginación por ancla, upsert idempotente,
//! búsqueda FTS5), borradores, ajustes, stickers y chats secretos.
//!
//! La API es síncrona y sencilla, pensada para `tokio::task::spawn_blocking` (p. ej. con
//! `Arc<Mutex<Store>>`). Nunca se guardan claves aquí: los chats secretos solo guardan blobs ya
//! cifrados y metadatos; las claves viven en el keyring. Ver docs/architecture.md §7.
//!
//! Pendiente (otros hitos): `CachedBackend` y la bóveda de claves del keyring.
#![forbid(unsafe_code)]

mod db;
mod dialogs;
mod error;
mod messages;
mod secret;
mod settings;
mod stickers;
mod types;

pub use db::{APP_DIR, SCHEMA_VERSION, Store, account_db_path, default_data_dir};
pub use error::{Error, Result};
pub use messages::fts_query;
pub use secret::NONCE_LEN;
pub use stickers::RECENT_CAP;
pub use types::*;

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    /// Directorio temporal único que se borra al terminar.
    struct TmpDir(PathBuf);
    impl TmpDir {
        fn new(tag: &str) -> Self {
            let p = std::env::temp_dir().join(format!("tc-store-{tag}-{}-{:?}", std::process::id(), std::thread::current().id()));
            let _ = std::fs::remove_dir_all(&p);
            std::fs::create_dir_all(&p).unwrap();
            Self(p)
        }
    }
    impl Drop for TmpDir {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn migrates_from_scratch_in_memory() {
        let s = Store::open_in_memory().unwrap();
        assert_eq!(s.schema_version().unwrap(), SCHEMA_VERSION);
        assert_eq!(SCHEMA_VERSION, 1);
        let n: i64 = s.conn.query_row("SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name IN ('dialogs','messages','messages_fts','secret_messages','sticker_sets','meta')", [], |r| r.get(0)).unwrap();
        assert_eq!(n, 6);
        let fk: i64 = s.conn.pragma_query_value(None, "foreign_keys", |r| r.get(0)).unwrap();
        assert_eq!(fk, 1);
        let fkv: Vec<String> = s.conn.prepare("PRAGMA foreign_key_check").unwrap().query_map([], |r| r.get(0)).unwrap().map(|x| x.unwrap()).collect();
        assert!(fkv.is_empty());
    }

    #[test]
    fn file_db_wal_perms_reopen_persists_and_is_idempotent() {
        let dir = TmpDir::new("file");
        let path = dir.0.join("nested/dir/cache.sqlite3");
        {
            let s = Store::open(&path).unwrap();
            let mode: String = s.conn.pragma_query_value(None, "journal_mode", |r| r.get(0)).unwrap();
            assert_eq!(mode, "wal");
            let sd: i64 = s.conn.pragma_query_value(None, "secure_delete", |r| r.get(0)).unwrap();
            assert_eq!(sd, 1);
            let k = s.upsert_dialog(&DialogRow::new(PeerKind::User, 1, 1)).unwrap();
            s.upsert_message(&MessageRow::text(k, 1, 1, false, "persistente")).unwrap();
            s.set_setting("a", "b").unwrap();
        }
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            assert_eq!(std::fs::metadata(&path).unwrap().permissions().mode() & 0o777, 0o600);
        }
        let s = Store::open(&path).unwrap(); // reabrir no re-ejecuta la migración (fallaría: tablas ya existen)
        assert_eq!(s.schema_version().unwrap(), 1);
        assert_eq!(s.search_messages("persistente", None, 5).unwrap().len(), 1);
        assert_eq!(s.get_setting("a").unwrap().as_deref(), Some("b"));
    }

    #[test]
    fn rejects_newer_schema() {
        let dir = TmpDir::new("newer");
        let path = dir.0.join("c.sqlite3");
        drop(Store::open(&path).unwrap());
        rusqlite::Connection::open(&path).unwrap().pragma_update(None, "user_version", 99).unwrap();
        assert!(matches!(Store::open(&path), Err(Error::SchemaTooNew { found: 99, supported: 1 })));
    }

    #[test]
    fn failed_migration_rolls_back() {
        // Una base con user_version=0 pero con una tabla `meta` ya existente hace fallar la 0001:
        // no debe quedar nada a medias ni subir la versión.
        let dir = TmpDir::new("rollback");
        let path = dir.0.join("c.sqlite3");
        {
            let c = rusqlite::Connection::open(&path).unwrap();
            c.execute_batch("CREATE TABLE meta (x)").unwrap();
        }
        assert!(Store::open(&path).is_err());
        let c = rusqlite::Connection::open(&path).unwrap();
        let v: i64 = c.pragma_query_value(None, "user_version", |r| r.get(0)).unwrap();
        let n: i64 = c.query_row("SELECT COUNT(*) FROM sqlite_master WHERE type='table'", [], |r| r.get(0)).unwrap();
        assert_eq!((v, n), (0, 1));
    }

    #[test]
    fn account_paths() {
        let root = PathBuf::from("/x");
        assert_eq!(account_db_path(&root, "acct-1_a").unwrap(), PathBuf::from("/x/accounts/acct-1_a/cache.sqlite3"));
        for bad in ["", "..", "a/b", "a b", "../etc"] {
            assert!(account_db_path(&root, bad).is_err(), "{bad}");
        }
        let dir = TmpDir::new("acct");
        let s = Store::open_account(&dir.0, "u1").unwrap();
        assert_eq!(s.schema_version().unwrap(), 1);
        assert!(dir.0.join("accounts/u1/cache.sqlite3").exists());
    }

    #[test]
    fn default_dir_follows_xdg_layout() {
        let d = default_data_dir().unwrap();
        assert_eq!(d.file_name().unwrap(), APP_DIR);
    }

    #[test]
    fn store_is_send() {
        fn assert_send<T: Send>() {}
        assert_send::<Store>();
    }

    #[test]
    fn works_across_spawn_blocking_style_threads() {
        let s = std::sync::Arc::new(std::sync::Mutex::new(Store::open_in_memory().unwrap()));
        let s2 = s.clone();
        std::thread::spawn(move || s2.lock().unwrap().set_setting("k", "v").unwrap()).join().unwrap();
        assert_eq!(s.lock().unwrap().get_setting("k").unwrap().as_deref(), Some("v"));
    }
}
