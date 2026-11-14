//! Linux/BSD: `org.freedesktop.Notifications` por D-Bus con zbus (Rust puro, sin libdbus).
//!
//! Spec: <https://specifications.freedesktop.org/notification-spec/latest/>. Un aviso por chat:
//! el siguiente reutiliza su id (`replaces_id`). Clic en el cuerpo = acción `default`. Dos hilos
//! escuchan `ActionInvoked` y `NotificationClosed`; las suscripciones se crean antes del primer
//! aviso para no perder señales.

use std::{
    collections::HashMap,
    sync::{Arc, Mutex, MutexGuard},
};

use zbus::{
    blocking::{Connection, Proxy},
    zvariant::Value,
};

use crate::{
    AppInfo, Capabilities, ChatKey, EventSender, Notification, Notifier, NotifyError, NotifyEvent,
    Result, icon::escape_xml, icon::icon_path,
};

const DEST: &str = "org.freedesktop.Notifications";
const PATH: &str = "/org/freedesktop/Notifications";
const DEFAULT_ACTION: &str = "default";

#[derive(Default)]
struct Ids {
    by_chat: HashMap<ChatKey, u32>,
    by_id: HashMap<u32, ChatKey>,
}

impl Ids {
    fn remove_id(&mut self, id: u32) {
        if let Some(chat) = self.by_id.remove(&id)
            && self.by_chat.get(&chat) == Some(&id)
        {
            self.by_chat.remove(&chat);
        }
    }
}

pub(crate) struct DbusNotifier {
    proxy: Proxy<'static>,
    app: AppInfo,
    ids: Arc<Mutex<Ids>>,
    caps: Capabilities,
    markup: bool,
}

fn backend(e: zbus::Error) -> NotifyError {
    NotifyError::Backend(e.to_string())
}

fn lock(ids: &Mutex<Ids>) -> MutexGuard<'_, Ids> {
    ids.lock().unwrap_or_else(|e| e.into_inner())
}

impl DbusNotifier {
    pub(crate) fn connect(app: &AppInfo, tx: EventSender) -> Result<Self> {
        let conn = Connection::session().map_err(|e| NotifyError::Unavailable(e.to_string()))?;
        let proxy = Proxy::new(&conn, DEST, PATH, DEST).map_err(backend)?;
        // Sin servidor de notificaciones esto falla (o activa uno por D-Bus si existe).
        let server_caps: Vec<String> = proxy
            .call("GetCapabilities", &())
            .map_err(|e| NotifyError::Unavailable(e.to_string()))?;
        let has = |c: &str| server_caps.iter().any(|s| s == c);
        let caps = Capabilities {
            actions: has("actions"),
            replace: true,
            withdraw: true,
            click: has("actions"),
        };

        let ids = Arc::new(Mutex::new(Ids::default()));
        let actions = proxy.receive_signal("ActionInvoked").map_err(backend)?;
        let closed = proxy
            .receive_signal("NotificationClosed")
            .map_err(backend)?;

        let ids_a = Arc::clone(&ids);
        std::thread::Builder::new()
            .name("tc-notify-actions".into())
            .spawn(move || {
                for msg in actions {
                    let Ok((id, key)) = msg.body().deserialize::<(u32, String)>() else {
                        continue;
                    };
                    let Some(chat) = lock(&ids_a).by_id.get(&id).cloned() else {
                        continue;
                    };
                    tx.send(if key == DEFAULT_ACTION {
                        NotifyEvent::Clicked(chat)
                    } else {
                        NotifyEvent::Action(chat, key)
                    });
                }
            })
            .map_err(|e| NotifyError::Backend(e.to_string()))?;
        let ids_c = Arc::clone(&ids);
        std::thread::Builder::new()
            .name("tc-notify-closed".into())
            .spawn(move || {
                for msg in closed {
                    if let Ok((id, _reason)) = msg.body().deserialize::<(u32, u32)>() {
                        lock(&ids_c).remove_id(id);
                    }
                }
            })
            .map_err(|e| NotifyError::Backend(e.to_string()))?;

        Ok(Self {
            proxy,
            app: app.clone(),
            ids,
            caps,
            markup: has("body-markup"),
        })
    }
}

impl Notifier for DbusNotifier {
    fn notify(&self, n: Notification) -> Result<()> {
        let replaces = lock(&self.ids)
            .by_chat
            .get(&n.chat_key)
            .copied()
            .unwrap_or(0);
        let mut actions: Vec<String> = Vec::new();
        if self.caps.actions {
            // La acción `default` es el clic en el cuerpo; su etiqueta no se suele mostrar.
            actions.extend([DEFAULT_ACTION.to_owned(), "Abrir".to_owned()]);
            for a in &n.actions {
                actions.extend([a.id.clone(), a.label.clone()]);
            }
        }
        let image = n
            .icon
            .as_ref()
            .and_then(icon_path)
            .map(|p| p.to_string_lossy().into_owned());
        let mut hints: HashMap<&str, Value<'_>> = HashMap::new();
        hints.insert(
            "desktop-entry",
            Value::from(self.app.desktop_entry.as_str()),
        );
        hints.insert("category", Value::from("im.received"));
        hints.insert("urgency", Value::U8(1));
        if n.silent {
            hints.insert("suppress-sound", Value::Bool(true));
        } else {
            hints.insert("sound-name", Value::from("message-new-instant"));
        }
        if let Some(img) = &image {
            hints.insert("image-path", Value::from(img.as_str()));
        }
        let body = if self.markup {
            escape_xml(&n.body)
        } else {
            n.body.clone()
        };
        let id: u32 = self
            .proxy
            .call(
                "Notify",
                &(
                    self.app.name.as_str(),
                    replaces,
                    // Icono de la app por nombre del tema (instalado con parlo.desktop).
                    self.app.desktop_entry.as_str(),
                    n.title.as_str(),
                    body.as_str(),
                    actions,
                    hints,
                    -1i32,
                ),
            )
            .map_err(backend)?;
        let mut ids = lock(&self.ids);
        if replaces != 0 && replaces != id {
            ids.remove_id(replaces);
        }
        ids.by_chat.insert(n.chat_key.clone(), id);
        ids.by_id.insert(id, n.chat_key);
        Ok(())
    }

    fn withdraw(&self, chat: &ChatKey) -> Result<()> {
        let id = {
            let mut ids = lock(&self.ids);
            let Some(id) = ids.by_chat.remove(chat) else {
                return Ok(());
            };
            ids.by_id.remove(&id);
            id
        };
        self.proxy
            .call::<_, _, ()>("CloseNotification", &(id,))
            .map_err(backend)
    }

    fn capabilities(&self) -> Capabilities {
        self.caps
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ids_retira_solo_si_coincide() {
        let mut ids = Ids::default();
        ids.by_chat.insert("u:1".into(), 7);
        ids.by_id.insert(7, "u:1".into());
        ids.by_id.insert(5, "u:1".into());
        ids.remove_id(5);
        assert_eq!(ids.by_chat.get(&ChatKey::from("u:1")), Some(&7));
        ids.remove_id(7);
        assert!(ids.by_chat.is_empty() && ids.by_id.is_empty());
    }

    /// Requiere una sesión gráfica con servidor de notificaciones: `cargo test -p tc-notify -- --ignored`.
    #[test]
    #[ignore = "necesita D-Bus de sesión y un servidor de notificaciones"]
    fn aviso_real() {
        let (tx, _rx) = crate::event_channel();
        let n = DbusNotifier::connect(&AppInfo::parlo(), tx).expect("D-Bus");
        let chat = ChatKey::from("prueba");
        let mk = |body: &str| Notification {
            chat_key: chat.clone(),
            title: "Parlo · prueba".into(),
            body: body.into(),
            icon: None,
            actions: vec![crate::Action::new("mark-read", "Marcar como leído")],
            silent: true,
        };
        n.notify(mk("primer <aviso> & más")).unwrap();
        n.notify(mk("reemplaza al primero")).unwrap();
        assert_eq!(lock(&n.ids).by_chat.len(), 1);
        std::thread::sleep(std::time::Duration::from_secs(2));
        n.withdraw(&chat).unwrap();
    }
}
