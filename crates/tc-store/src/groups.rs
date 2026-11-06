//! Grupos, supergrupos y canales.

use crate::db::Store;
use crate::error::Result;
use crate::types::*;
use rusqlite::{OptionalExtension, Row, params};

const COLS: &str = "id, kind, access_hash, title, about, username, participants_count, my_role, can_send, created_at, updated_at";

fn from_row(r: &Row<'_>) -> rusqlite::Result<GroupRow> {
    Ok(GroupRow {
        id: r.get(0)?,
        kind: r.get(1)?,
        access_hash: r.get(2)?,
        title: r.get(3)?,
        about: r.get(4)?,
        username: r.get(5)?,
        participants_count: r.get(6)?,
        my_role: r.get(7)?,
        can_send: r.get(8)?,
        created_at: r.get(9)?,
        updated_at: r.get(10)?,
    })
}

impl Store {
    /// Inserta o actualiza un grupo (idempotente). No toca la foto.
    pub fn upsert_group(&self, g: &GroupRow) -> Result<()> {
        self.conn.execute(
            "INSERT INTO groups (id, kind, access_hash, title, about, username, participants_count, my_role, can_send, created_at, updated_at)
             VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11)
             ON CONFLICT(id) DO UPDATE SET kind=excluded.kind, access_hash=excluded.access_hash, title=excluded.title,
               about=excluded.about, username=excluded.username, participants_count=excluded.participants_count,
               my_role=excluded.my_role, can_send=excluded.can_send, created_at=excluded.created_at,
               updated_at=excluded.updated_at",
            params![g.id, g.kind, g.access_hash, g.title, g.about, g.username, g.participants_count, g.my_role, g.can_send, g.created_at, g.updated_at],
        )?;
        Ok(())
    }

    /// Grupo por id.
    pub fn get_group(&self, id: i64) -> Result<Option<GroupRow>> {
        Ok(self.conn.query_row(&format!("SELECT {COLS} FROM groups WHERE id=?1"), [id], from_row).optional()?)
    }

    /// Grupo por `@usuario` (sin la arroba).
    pub fn find_group_by_username(&self, username: &str) -> Result<Option<GroupRow>> {
        Ok(self
            .conn
            .query_row(&format!("SELECT {COLS} FROM groups WHERE username=?1 COLLATE NOCASE"), [username], from_row)
            .optional()?)
    }

    /// Todos los grupos conocidos, por título.
    pub fn list_groups(&self) -> Result<Vec<GroupRow>> {
        let mut st = self.conn.prepare(&format!("SELECT {COLS} FROM groups ORDER BY title COLLATE NOCASE, id"))?;
        let rows = st.query_map([], from_row)?;
        Ok(rows.collect::<rusqlite::Result<_>>()?)
    }

    /// Borra un grupo (no toca su diálogo).
    pub fn delete_group(&self, id: i64) -> Result<bool> {
        Ok(self.conn.execute("DELETE FROM groups WHERE id=?1", [id])? > 0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn group_roundtrip_update_and_lookup() {
        let s = Store::open_in_memory().unwrap();
        let mut g = GroupRow::new(7, GroupKindRow::Supergroup, "Rust ES", 10);
        g.username = Some("RustES".into());
        g.participants_count = Some(1200);
        s.upsert_group(&g).unwrap();
        assert_eq!(s.get_group(7).unwrap().unwrap(), g);
        g.title = "Rust España".into();
        g.my_role = GroupRole::Admin;
        g.can_send = false;
        s.upsert_group(&g).unwrap();
        assert_eq!(s.get_group(7).unwrap().unwrap(), g);
        assert_eq!(s.find_group_by_username("rustes").unwrap().unwrap().id, 7);
        s.upsert_group(&GroupRow::new(3, GroupKindRow::Channel, "avisos", 1)).unwrap();
        let titles: Vec<_> = s.list_groups().unwrap().into_iter().map(|g| g.title).collect();
        assert_eq!(titles, ["avisos", "Rust España"]);
        assert!(s.delete_group(3).unwrap());
        assert!(!s.delete_group(3).unwrap());
        assert!(s.get_group(3).unwrap().is_none());
    }

    #[test]
    fn user_extra_and_contacts() {
        let s = Store::open_in_memory().unwrap();
        let u = |id, name: &str, me, contact| UserRow { id, first_name: name.into(), is_self: me, is_contact: contact, updated_at: 1, ..Default::default() };
        s.upsert_user(&u(1, "Yo", true, false)).unwrap();
        s.upsert_user(&u(2, "beto", false, true)).unwrap();
        s.upsert_user(&u(3, "Ana", false, true)).unwrap();
        s.upsert_user(&u(4, "Zoe", false, false)).unwrap();
        assert_eq!(s.get_self_user().unwrap().unwrap().id, 1);
        let ids: Vec<_> = s.list_contacts().unwrap().iter().map(|u| u.id).collect();
        assert_eq!(ids, [3, 2], "sin distinguir mayúsculas");
        let x = UserExtra { is_bot: true, status: PresenceKind::Offline, last_seen_at: Some(99) };
        s.set_user_extra(2, &x).unwrap();
        assert_eq!(s.get_user_extra(2).unwrap().unwrap(), x);
        assert_eq!(s.get_user_extra(3).unwrap().unwrap().status, PresenceKind::Unknown);
        assert!(s.set_user_extra(99, &x).is_err());
        assert!(s.get_user_extra(99).unwrap().is_none());
    }
}
