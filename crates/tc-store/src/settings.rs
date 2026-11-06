//! Ajustes clave/valor de la cuenta (tabla `meta`). Nunca guardar secretos aquí.

use crate::db::Store;
use crate::error::Result;
use rusqlite::{OptionalExtension, params};

impl Store {
    /// Valor de un ajuste.
    pub fn get_setting(&self, key: &str) -> Result<Option<String>> {
        Ok(self.conn.query_row("SELECT value FROM meta WHERE key=?1", [key], |r| r.get(0)).optional()?)
    }

    /// Guarda (upsert) un ajuste.
    pub fn set_setting(&self, key: &str, value: &str) -> Result<()> {
        self.conn.execute(
            "INSERT INTO meta (key, value) VALUES (?1, ?2) ON CONFLICT(key) DO UPDATE SET value=excluded.value",
            params![key, value],
        )?;
        Ok(())
    }

    /// Borra un ajuste; devuelve si existía.
    pub fn remove_setting(&self, key: &str) -> Result<bool> {
        Ok(self.conn.execute("DELETE FROM meta WHERE key=?1", [key])? > 0)
    }

    /// Ajustes cuyo nombre empieza por `prefix`, ordenados por clave.
    pub fn list_settings(&self, prefix: &str) -> Result<Vec<(String, String)>> {
        // Se escapan los comodines de LIKE para que el prefijo sea literal.
        let esc = prefix.replace('\\', "\\\\").replace('%', "\\%").replace('_', "\\_");
        let mut st = self.conn.prepare("SELECT key, value FROM meta WHERE key LIKE ?1 || '%' ESCAPE '\\' ORDER BY key")?;
        let rows = st.query_map([esc], |r| Ok((r.get(0)?, r.get(1)?)))?;
        Ok(rows.collect::<rusqlite::Result<_>>()?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn settings_crud_and_prefix() {
        let s = Store::open_in_memory().unwrap();
        assert!(s.get_setting("ui.theme").unwrap().is_none());
        s.set_setting("ui.theme", "dark").unwrap();
        s.set_setting("ui.theme", "light").unwrap();
        s.set_setting("ui.motion", "reduced").unwrap();
        s.set_setting("uiXother", "x").unwrap();
        assert_eq!(s.get_setting("ui.theme").unwrap().as_deref(), Some("light"));
        assert_eq!(s.list_settings("ui.").unwrap().len(), 2);
        assert!(s.remove_setting("ui.theme").unwrap());
        assert!(!s.remove_setting("ui.theme").unwrap());
    }
}
