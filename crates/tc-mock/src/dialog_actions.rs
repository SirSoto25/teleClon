//! Gestión de chats en el mock (menú contextual de la lista y de mensajes): marca de no leído,
//! leer todo, silenciar con plazo, vaciar, borrar, salir, bloquear, información y mensajes fijados.
//! Todo en memoria y con los mismos eventos que la red real.

use std::collections::{BTreeSet, HashMap, HashSet};

use tc_core::*;

use crate::MockBackend;

/// Fijados en la lista principal sin Premium (como en Telegram).
pub const PINNED_LIMIT: usize = 5;

/// Estado extra del mock para estas acciones.
#[derive(Default)]
pub(crate) struct Acts {
    pub blocked: HashSet<UserId>,
    pub pinned_msgs: HashMap<PeerRef, BTreeSet<MsgId>>,
    /// Fin del silencio por chat (`None` = para siempre).
    pub mute_until: HashMap<PeerRef, Option<Timestamp>>,
}

/// `PINNED_DIALOGS_TOO_MUCH` si fijar `peer` supera el límite de su lista.
pub(crate) fn check_pin_limit(dialogs: &HashMap<PeerRef, Dialog>, peer: PeerRef, pinned: bool) -> BackendResult<()> {
    let Some(d) = dialogs.get(&peer) else { return Err(BackendError::NotFound) };
    if !pinned || d.pinned_order.is_some() {
        return Ok(());
    }
    let n = dialogs.values().filter(|x| x.archived == d.archived && x.pinned_order.is_some()).count();
    if n >= PINNED_LIMIT { Err(BackendError::Forbidden("PINNED_DIALOGS_TOO_MUCH".into())) } else { Ok(()) }
}

impl MockBackend {
    /// Cambia el diálogo con `f` y anuncia la fila nueva.
    async fn act_dialog(&self, peer: PeerRef, f: impl FnOnce(&mut Dialog)) -> BackendResult<()> {
        self.inner.gate().await?;
        let d = {
            let mut st = self.inner.lock();
            let d = st.dialogs.get_mut(&peer).ok_or(BackendError::NotFound)?;
            f(d);
            d.clone()
        };
        self.inner.emit(BackendEvent::DialogUpserted(d));
        Ok(())
    }

    pub(crate) async fn act_mute_until(&self, peer: PeerRef, until: Option<Timestamp>) -> BackendResult<()> {
        self.act_dialog(peer, |d| d.muted = true).await?;
        self.inner.lock().acts.mute_until.insert(peer, until);
        Ok(())
    }

    /// Fin del silencio de `peer` (`Some(None)` = para siempre; `None` = sin plazo registrado).
    pub fn mute_deadline(&self, peer: PeerRef) -> Option<Option<Timestamp>> {
        self.inner.lock().acts.mute_until.get(&peer).copied()
    }

    pub(crate) async fn act_unread_mark(&self, peer: PeerRef, unread: bool) -> BackendResult<()> {
        self.act_dialog(peer, |d| d.unread_mark = unread).await
    }

    pub(crate) async fn act_mark_dialog_read(&self, peer: PeerRef) -> BackendResult<()> {
        self.inner.gate().await?;
        let (d, max_id) = {
            let mut st = self.inner.lock();
            let max_id = st.messages.get(&peer).and_then(|l| l.last()).map_or(MsgId(0), |m| m.id);
            let d = st.dialogs.get_mut(&peer).ok_or(BackendError::NotFound)?;
            d.unread_count = 0;
            d.unread_mentions = 0;
            d.unread_mark = false;
            (d.clone(), max_id)
        };
        self.inner.emit(BackendEvent::ReadInbox { peer, max_id });
        self.inner.emit(BackendEvent::DialogUpserted(d));
        Ok(())
    }

    pub(crate) async fn act_clear_history(&self, peer: PeerRef) -> BackendResult<()> {
        self.inner.gate().await?;
        let d = {
            let mut st = self.inner.lock();
            st.ensure_peer(peer)?;
            if matches!(peer, PeerRef::Group(g) if st.groups.get(&g).is_some_and(|g| g.kind == GroupKind::Channel)) {
                return Err(BackendError::Forbidden("CHAT_ADMIN_REQUIRED".into()));
            }
            st.messages.insert(peer, Vec::new());
            st.acts.pinned_msgs.remove(&peer);
            let d = st.dialogs.get_mut(&peer).expect("comprobado");
            d.last_message = None;
            d.unread_count = 0;
            d.unread_mentions = 0;
            d.unread_mark = false;
            d.clone()
        };
        self.inner.emit(BackendEvent::HistoryCleared { peer });
        self.inner.emit(BackendEvent::DialogUpserted(d));
        Ok(())
    }

    pub(crate) async fn act_delete_dialog(&self, peer: PeerRef, _revoke: bool) -> BackendResult<()> {
        self.inner.gate().await?;
        let secret = {
            let mut st = self.inner.lock();
            st.ensure_peer(peer)?;
            st.dialogs.remove(&peer);
            st.messages.remove(&peer);
            st.acts.pinned_msgs.remove(&peer);
            st.acts.mute_until.remove(&peer);
            match peer {
                PeerRef::Secret(s) => st.secrets.get_mut(&s).map(|c| {
                    c.state = SecretChatState::Discarded;
                    c.clone()
                }),
                _ => None,
            }
        };
        if let Some(c) = secret {
            self.inner.emit(BackendEvent::SecretChat(c));
        }
        self.inner.emit(BackendEvent::DialogRemoved { peer });
        Ok(())
    }

    pub(crate) async fn act_leave_chat(&self, peer: PeerRef) -> BackendResult<()> {
        if !matches!(peer, PeerRef::Group(_)) {
            return Err(BackendError::InvalidInput("peer"));
        }
        self.act_delete_dialog(peer, false).await
    }

    pub(crate) async fn act_set_blocked(&self, user: UserId, blocked: bool) -> BackendResult<()> {
        self.inner.gate().await?;
        let mut st = self.inner.lock();
        if user == st.me.id {
            return Err(BackendError::InvalidInput("user"));
        }
        if !st.users.contains_key(&user) {
            return Err(BackendError::NotFound);
        }
        if blocked {
            st.acts.blocked.insert(user);
        } else {
            st.acts.blocked.remove(&user);
        }
        Ok(())
    }

    /// ¿Está bloqueado? (para pruebas).
    pub fn is_blocked(&self, user: UserId) -> bool {
        self.inner.lock().acts.blocked.contains(&user)
    }

    pub(crate) async fn act_chat_info(&self, peer: PeerRef) -> BackendResult<ChatInfo> {
        self.inner.gate().await?;
        let st = self.inner.lock();
        st.ensure_peer(peer)?;
        let user = match peer {
            PeerRef::User(u) => Some(u),
            PeerRef::Secret(s) => st.secrets.get(&s).map(|c| c.user),
            PeerRef::Group(_) => None,
        };
        Ok(match (peer, user) {
            (PeerRef::Group(g), _) => {
                let g = st.groups.get(&g).ok_or(BackendError::NotFound)?;
                let about = match g.kind {
                    GroupKind::Channel => "Canal de prueba del backend simulado.",
                    _ => "Grupo de prueba del backend simulado.",
                };
                ChatInfo { about: Some(about.into()), username: g.username.clone(), phone: None, members: Some(g.member_count), blocked: false }
            }
            (_, Some(u)) => {
                let usr = st.users.get(&u);
                ChatInfo {
                    about: usr.filter(|u| !u.is_bot).map(|_| "Persona de prueba del backend simulado.".into()),
                    username: usr.and_then(|u| u.username.clone()),
                    phone: None,
                    members: None,
                    blocked: st.acts.blocked.contains(&u),
                }
            }
            _ => ChatInfo::default(),
        })
    }

    pub(crate) async fn act_pin_message(&self, peer: PeerRef, id: MsgId, pinned: bool) -> BackendResult<()> {
        self.inner.gate().await?;
        let mut st = self.inner.lock();
        st.ensure_peer(peer)?;
        if matches!(peer, PeerRef::Secret(_)) {
            return Err(BackendError::Forbidden("CHAT_NOT_SUPPORTED".into()));
        }
        let exists = st.messages.get(&peer).is_some_and(|l| l.binary_search_by_key(&id, |m| m.id).is_ok());
        if !exists {
            return Err(BackendError::NotFound);
        }
        let set = st.acts.pinned_msgs.entry(peer).or_default();
        if pinned {
            set.insert(id);
        } else {
            set.remove(&id);
        }
        Ok(())
    }

    /// Mensajes fijados de un chat (para pruebas).
    pub fn pinned_messages(&self, peer: PeerRef) -> Vec<MsgId> {
        self.inner.lock().acts.pinned_msgs.get(&peer).map(|s| s.iter().copied().collect()).unwrap_or_default()
    }
}

#[cfg(test)]
mod tests {
    use futures::StreamExt;
    use tc_core::*;

    use crate::{BIG_CHAT, MockBackend, MockConfig};

    fn mock() -> MockBackend {
        MockBackend::new(MockConfig { now: Some(1_700_000_000), ..MockConfig::default() })
    }

    async fn all(b: &MockBackend, archived: bool) -> Vec<Dialog> {
        b.dialogs(DialogsPage { offset: 0, limit: 500, archived }).await.unwrap()
    }

    fn user_chat(list: &[Dialog]) -> PeerRef {
        list.iter().find(|d| matches!(d.peer, PeerRef::User(_)) && d.peer != BIG_CHAT && d.last_message.is_some()).unwrap().peer
    }

    #[tokio::test]
    async fn unread_mark_and_read_all() {
        let b = mock();
        let list = all(&b, false).await;
        let peer = user_chat(&list);
        let mut ev = b.events();
        b.set_unread_mark(peer, true).await.unwrap();
        assert!(matches!(ev.next().await, Some(BackendEvent::DialogUpserted(d)) if d.unread_mark && d.looks_unread()));
        b.mark_dialog_read(peer).await.unwrap();
        let last = all(&b, false).await.into_iter().find(|d| d.peer == peer).unwrap();
        assert!(!last.unread_mark && last.unread_count == 0);
        assert!(matches!(ev.next().await, Some(BackendEvent::ReadInbox { peer: p, max_id }) if p == peer && Some(max_id) == last.last_message.map(|m| m.id)));
    }

    #[tokio::test]
    async fn pin_limit_is_an_error() {
        let b = mock();
        let list = all(&b, false).await;
        let free: Vec<PeerRef> = list.iter().filter(|d| d.pinned_order.is_none()).map(|d| d.peer).collect();
        let already = list.len() - free.len();
        for p in free.iter().take(super::PINNED_LIMIT - already) {
            b.set_pinned(*p, true).await.unwrap();
        }
        let err = b.set_pinned(free[super::PINNED_LIMIT - already], true).await.unwrap_err();
        assert_eq!(err, BackendError::Forbidden("PINNED_DIALOGS_TOO_MUCH".into()));
        // Volver a fijar uno ya fijado no cuenta.
        b.set_pinned(free[0], true).await.unwrap();
    }

    #[tokio::test]
    async fn mute_for_a_while_then_unmute() {
        let b = mock();
        let peer = user_chat(&all(&b, false).await);
        b.mute_until(peer, Some(Timestamp(1_700_003_600))).await.unwrap();
        assert!(all(&b, false).await.iter().any(|d| d.peer == peer && d.muted));
        assert_eq!(b.mute_deadline(peer), Some(Some(Timestamp(1_700_003_600))));
        b.set_muted(peer, false).await.unwrap();
        assert!(all(&b, false).await.iter().any(|d| d.peer == peer && !d.muted));
    }

    #[tokio::test]
    async fn clear_history_keeps_the_row_delete_removes_it() {
        let b = mock();
        let peer = user_chat(&all(&b, false).await);
        let mut ev = b.events();
        b.clear_history(peer).await.unwrap();
        assert_eq!(ev.next().await, Some(BackendEvent::HistoryCleared { peer }));
        assert!(b.history(peer, Anchor::Latest, 50).await.unwrap().is_empty());
        let row = all(&b, false).await.into_iter().find(|d| d.peer == peer).unwrap();
        assert!(row.last_message.is_none());
        let _ = ev.next().await; // DialogUpserted
        b.delete_dialog(peer, true).await.unwrap();
        assert_eq!(ev.next().await, Some(BackendEvent::DialogRemoved { peer }));
        assert!(all(&b, false).await.iter().all(|d| d.peer != peer));
        assert_eq!(b.history(peer, Anchor::Latest, 50).await.unwrap_err(), BackendError::NotFound);
    }

    #[tokio::test]
    async fn leave_only_groups_and_channels_cannot_be_cleared() {
        let b = mock();
        let list = all(&b, false).await;
        let user = user_chat(&list);
        assert!(b.leave_chat(user).await.is_err());
        let channel = list.iter().find(|d| matches!(d.peer, PeerRef::Group(_)) && d.muted).map(|d| d.peer);
        let group = list.iter().find(|d| matches!(d.peer, PeerRef::Group(_))).unwrap().peer;
        b.leave_chat(group).await.unwrap();
        assert!(all(&b, false).await.iter().all(|d| d.peer != group));
        if let Some(ch) = channel.filter(|c| *c != group)
            && let PeerRef::Group(g) = ch
            && b.group(g).await.unwrap().kind == GroupKind::Channel
        {
            assert_eq!(b.clear_history(ch).await.unwrap_err(), BackendError::Forbidden("CHAT_ADMIN_REQUIRED".into()));
        }
    }

    #[tokio::test]
    async fn block_info_and_pinned_messages() {
        let b = mock();
        let peer = user_chat(&all(&b, false).await);
        let PeerRef::User(u) = peer else { unreachable!() };
        assert!(!b.chat_info(peer).await.unwrap().blocked);
        b.set_blocked(u, true).await.unwrap();
        assert!(b.is_blocked(u) && b.chat_info(peer).await.unwrap().blocked);
        b.set_blocked(u, false).await.unwrap();
        assert!(!b.is_blocked(u));
        let me = b.me().await.unwrap().id;
        assert_eq!(b.set_blocked(me, true).await.unwrap_err(), BackendError::InvalidInput("user"));
        let last = b.history(peer, Anchor::Latest, 1).await.unwrap()[0].id;
        b.pin_message(peer, last, true).await.unwrap();
        assert_eq!(b.pinned_messages(peer), vec![last]);
        b.pin_message(peer, last, false).await.unwrap();
        assert!(b.pinned_messages(peer).is_empty());
        assert_eq!(b.pin_message(peer, MsgId(999_999), true).await.unwrap_err(), BackendError::NotFound);
    }

    #[tokio::test]
    async fn deleting_a_secret_chat_discards_it() {
        let b = mock();
        let secret = PeerRef::Secret(crate::SECRET_ID);
        let mut ev = b.events();
        b.delete_dialog(secret, false).await.unwrap();
        assert!(matches!(ev.next().await, Some(BackendEvent::SecretChat(c)) if c.state == SecretChatState::Discarded));
        assert_eq!(ev.next().await, Some(BackendEvent::DialogRemoved { peer: secret }));
    }
}
