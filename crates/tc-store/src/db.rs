//! Apertura de la base, PRAGMAs, migraciones y rutas XDG.

use crate::error::{Error, Result};
use rusqlite::Connection;
use std::path::{Path, PathBuf};
use std::time::Duration;

/// Nombre de la app para las rutas XDG y el servicio del llavero (marca: Parlo, ADR-0005).
pub const APP_DIR: &str = "parlo";

/// Nombre anterior (provisional) de las rutas y del servicio del llavero; solo se lee para migrar.
pub const LEGACY_APP_DIR: &str = "teleclon";

/// Migraciones en orden; la posición + 1 es el `user_version` resultante.
const MIGRATIONS: &[&str] = &[
    include_str!("../migrations/0001_init.sql"),
    include_str!("../migrations/0002_groups_folders_reactions_uploads.sql"),
    include_str!("../migrations/0003_secret_chat_states.sql"),
];

/// Última versión de esquema que conoce este binario.
pub const SCHEMA_VERSION: i64 = MIGRATIONS.len() as i64;

/// Caché local de una cuenta. Síncrona: pensada para usarse dentro de `spawn_blocking`
/// (p. ej. `Arc<Mutex<Store>>`). `Store` es `Send` pero no `Sync`.
pub struct Store {
    pub(crate) conn: Connection,
}

impl Store {
    /// Abre (creando si falta, con permisos 0600) la base en `path` y aplica migraciones.
    pub fn open(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref();
        if let Some(parent) = path.parent().filter(|p| !p.as_os_str().is_empty()) {
            std::fs::create_dir_all(parent).map_err(|source| Error::Io { path: parent.into(), source })?;
        }
        touch_private(path)?;
        Self::init(Connection::open(path)?)
    }

    /// Base en memoria (pruebas, modo efímero).
    pub fn open_in_memory() -> Result<Self> {
        Self::init(Connection::open_in_memory()?)
    }

    /// Abre la base de una cuenta bajo `root`: `<root>/accounts/<account_id>/cache.sqlite3`.
    pub fn open_account(root: impl AsRef<Path>, account_id: &str) -> Result<Self> {
        Self::open(account_db_path(root.as_ref(), account_id)?)
    }

    fn init(conn: Connection) -> Result<Self> {
        conn.busy_timeout(Duration::from_millis(5000))?;
        // journal_mode devuelve una fila ("wal", o "memory" en bases en memoria).
        conn.pragma_update_and_check(None, "journal_mode", "WAL", |r| r.get::<_, String>(0))?;
        conn.pragma_update(None, "synchronous", "NORMAL")?;
        conn.pragma_update(None, "foreign_keys", "ON")?;
        conn.pragma_update(None, "secure_delete", "ON")?;
        let mut store = Self { conn };
        store.migrate()?;
        Ok(store)
    }

    /// Versión de esquema actual (`PRAGMA user_version`).
    pub fn schema_version(&self) -> Result<i64> {
        Ok(self.conn.pragma_query_value(None, "user_version", |r| r.get(0))?)
    }

    fn migrate(&mut self) -> Result<()> {
        let current = self.schema_version()?;
        if current > SCHEMA_VERSION {
            return Err(Error::SchemaTooNew { found: current, supported: SCHEMA_VERSION });
        }
        for (i, sql) in MIGRATIONS.iter().enumerate() {
            let version = i as i64 + 1;
            if version <= current {
                continue;
            }
            // Cada migración en su transacción; si falla, no queda a medias.
            let tx = self.conn.transaction()?;
            tx.execute_batch(sql)?;
            tx.pragma_update(None, "user_version", version)?;
            tx.commit()?;
            tracing::info!(version, "migración aplicada");
        }
        Ok(())
    }
}

/// Directorio de datos por defecto: `$XDG_DATA_HOME/parlo` o `~/.local/share/parlo`.
/// Si solo existe el directorio antiguo (`teleclon`), se renombra una vez; si no se puede
/// renombrar, se sigue usando el antiguo para no perder datos.
pub fn default_data_dir() -> Result<PathBuf> {
    let new = directories::ProjectDirs::from("", "", APP_DIR).map(|d| d.data_dir().to_path_buf()).ok_or(Error::NoDataDir)?;
    let legacy = directories::ProjectDirs::from("", "", LEGACY_APP_DIR).map(|d| d.data_dir().to_path_buf());
    Ok(match legacy {
        Some(old) => resolve_data_dir(new, &old),
        None => new,
    })
}

/// Elige entre el directorio nuevo y el antiguo, migrando el antiguo si hace falta.
pub fn resolve_data_dir(new: PathBuf, legacy: &Path) -> PathBuf {
    if new.exists() || !legacy.is_dir() {
        return new;
    }
    if let Some(parent) = new.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    match std::fs::rename(legacy, &new) {
        Ok(()) => {
            tracing::info!("datos migrados de {} a {}", legacy.display(), new.display());
            new
        }
        Err(e) => {
            tracing::warn!("no se pudo migrar {} a {} ({}); se usa el directorio antiguo", legacy.display(), new.display(), e.kind());
            legacy.to_path_buf()
        }
    }
}

/// Ruta de la base de una cuenta. El id solo admite `[A-Za-z0-9_-]` (evita escapar del directorio).
pub fn account_db_path(root: &Path, account_id: &str) -> Result<PathBuf> {
    let ok = !account_id.is_empty()
        && account_id.len() <= 64
        && account_id.chars().all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-');
    if !ok {
        return Err(Error::Invalid(format!("id de cuenta no válido: {account_id:?}")));
    }
    Ok(root.join("accounts").join(account_id).join("cache.sqlite3"))
}

#[cfg(unix)]
fn touch_private(path: &Path) -> Result<()> {
    use std::os::unix::fs::OpenOptionsExt;
    std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .mode(0o600)
        .open(path)
        .map(drop)
        .map_err(|source| Error::Io { path: path.into(), source })
}

#[cfg(not(unix))]
fn touch_private(path: &Path) -> Result<()> {
    std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .map(drop)
        .map_err(|source| Error::Io { path: path.into(), source })
}
