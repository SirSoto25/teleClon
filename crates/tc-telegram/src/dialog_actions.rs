//! Gestión de chats con funciones TL en crudo (menú contextual de la lista y de mensajes):
//!
//! | operación | TL |
//! | --- | --- |
//! | marca de no leído | `messages.markDialogUnread` |
//! | leer todo | `messages.readHistory` / `channels.readHistory` (+ quitar la marca) |
//! | silenciar con plazo | `account.updateNotifySettings` (`mute_until`) |
//! | vaciar historial | `messages.deleteHistory{just_clear}` / `channels.deleteHistory` |
//! | borrar chat | privado: `messages.deleteHistory{revoke?}`; grupo básico: `messages.deleteChatUser` + `deleteHistory`; canal/supergrupo: `channels.leaveChannel`; secreto: `messages.discardEncryption` |
//! | salir | `messages.deleteChatUser(self)` / `channels.leaveChannel` |
//! | bloquear | `contacts.block` / `contacts.unblock` |
//! | información | `users.getFullUser` / `messages.getFullChat` / `channels.getFullChannel` |
//! | fijar mensaje | `messages.updatePinnedMessage` |
//!
//! `deleteHistory` borra por tandas: mientras devuelve `offset > 0` hay que repetirla.
use std::sync::Arc;

use grammers_tl_types as tl;
use tc_core::*;

use crate::events::flatten_updates;
use crate::ids::{GroupTarget, channel_of, decode_group};
use crate::{Inner, now_secs};

/// Tope de repeticiones de `messages.deleteHistory` (cada una borra una tanda del servidor).
const MAX_DELETE_ROUNDS: usize = 64;

impl Inner {
    /// Cambia la fila del chat (si está en caché) y la anuncia.
    fn touch_dialog(&self, peer: PeerRef, f: impl FnOnce(&mut Dialog)) {
        let d = self.cache().dialogs.get_mut(&peer).map(|d| {
            f(d);
            d.clone()
        });
        if let Some(d) = d {
            self.emit(BackendEvent::DialogUpserted(d));
        }
    }

    /// Quita el chat de la caché y de las dos listas, y lo anuncia.
    fn drop_dialog(&self, peer: PeerRef) {
        {
            let mut c = self.cache();
            c.dialogs.remove(&peer);
            c.main.order.retain(|p| *p != peer);
            c.archive.order.retain(|p| *p != peer);
        }
        self.emit(BackendEvent::DialogRemoved { peer });
    }

    fn last_id(&self, peer: PeerRef) -> Option<MsgId> {
        self.cache().dialogs.get(&peer).and_then(|d| d.last_message.as_ref()).map(|m| m.id)
    }

    fn apply_updates(&self, u: tl::enums::Updates) {
        let (ups, users, chats, _) = flatten_updates(u);
        let applied = self.apply_response(ups, users, chats);
        self.emit_all(applied.events);
    }

    fn input_dialog_peer(&self, peer: PeerRef) -> BackendResult<tl::enums::InputDialogPeer> {
        Ok(tl::types::InputDialogPeer { peer: self.input_peer(peer)? }.into())
    }

    pub(crate) async fn act_unread_mark(&self, peer: PeerRef, unread: bool) -> BackendResult<()> {
        if matches!(peer, PeerRef::Secret(_)) {
            return Err(BackendError::Forbidden("CHAT_NOT_SUPPORTED".into()));
        }
        let req = tl::functions::messages::MarkDialogUnread { unread, parent_peer: None, peer: self.input_dialog_peer(peer)? };
        self.rpc(&req).await?;
        self.touch_dialog(peer, |d| d.unread_mark = unread);
        Ok(())
    }

    /// Lectura hasta el último mensaje conocido y, si estaba marcado a mano, quita la marca.
    pub(crate) async fn act_mark_dialog_read(&self, peer: PeerRef, read_upto: impl std::future::Future<Output = BackendResult<()>>) -> BackendResult<()> {
        let marked = self.cache().dialogs.get(&peer).is_some_and(|d| d.unread_mark);
        read_upto.await?;
        if marked && !matches!(peer, PeerRef::Secret(_)) {
            let req = tl::functions::messages::MarkDialogUnread { unread: false, parent_peer: None, peer: self.input_dialog_peer(peer)? };
            self.rpc(&req).await?;
        }
        self.touch_dialog(peer, |d| {
            d.unread_count = 0;
            d.unread_mentions = 0;
            d.unread_mark = false;
        });
        Ok(())
    }

    /// Último mensaje para leer todo el chat (`0` si no se conoce: la red lee entonces todo).
    pub(crate) fn read_all_target(&self, peer: PeerRef) -> MsgId {
        self.last_id(peer).unwrap_or(MsgId(0))
    }

    pub(crate) async fn act_mute_until(&self, peer: PeerRef, until: Option<Timestamp>) -> BackendResult<()> {
        let until = until.map_or(i32::MAX, |t| i32::try_from(t.0).unwrap_or(i32::MAX));
        if i64::from(until) <= now_secs() {
            return Err(BackendError::InvalidInput("mute_until"));
        }
        let settings = tl::types::InputPeerNotifySettings {
            show_previews: None,
            silent: None,
            mute_until: Some(until),
            sound: None,
            stories_muted: None,
            stories_hide_sender: None,
            stories_sound: None,
        };
        let peer_in = self.input_peer(peer)?;
        self.rpc(&tl::functions::account::UpdateNotifySettings { peer: tl::types::InputNotifyPeer { peer: peer_in }.into(), settings: settings.into() })
            .await?;
        self.touch_dialog(peer, |d| d.muted = true);
        Ok(())
    }

    /// `messages.deleteHistory` repetida mientras el servidor diga que quedan tandas.
    async fn delete_history_rounds(&self, peer: PeerRef, just_clear: bool, revoke: bool) -> BackendResult<()> {
        let input = self.input_peer(peer)?;
        for _ in 0..MAX_DELETE_ROUNDS {
            let req = tl::functions::messages::DeleteHistory { just_clear, revoke, peer: input.clone(), max_id: 0, min_date: None, max_date: None };
            let tl::enums::messages::AffectedHistory::History(h) = self.rpc(&req).await?;
            if h.offset <= 0 {
                return Ok(());
            }
        }
        tracing::warn!("deleteHistory: quedan tandas tras {MAX_DELETE_ROUNDS} rondas");
        Ok(())
    }

    pub(crate) async fn act_clear_history(&self, peer: PeerRef) -> BackendResult<()> {
        match peer {
            PeerRef::Secret(_) => return Err(BackendError::Forbidden("CHAT_NOT_SUPPORTED".into())),
            PeerRef::Group(g) => {
                if self.cache().peers.groups.get(&g).is_some_and(|g| g.kind == GroupKind::Channel) {
                    return Err(BackendError::Forbidden("CHAT_ADMIN_REQUIRED".into()));
                }
                if let Some(ch) = channel_of(peer) {
                    let channel = self.cache().peers.input_channel(ch, &self.net.session)?;
                    let max_id = self.last_id(peer).map_or(i32::MAX, |m| m.0);
                    let u = self.rpc(&tl::functions::channels::DeleteHistory { for_everyone: false, channel, max_id }).await?;
                    self.apply_updates(u);
                } else {
                    self.delete_history_rounds(peer, true, false).await?;
                }
            }
            PeerRef::User(_) => self.delete_history_rounds(peer, true, false).await?,
        }
        self.touch_dialog(peer, |d| {
            d.last_message = None;
            d.unread_count = 0;
            d.unread_mentions = 0;
            d.unread_mark = false;
        });
        self.emit(BackendEvent::HistoryCleared { peer });
        Ok(())
    }

    /// Sale de un grupo o canal. `then_delete`: en grupos básicos, borra también el historial
    /// (que si no queda en la lista como chat abandonado).
    async fn leave(&self, peer: PeerRef, then_delete: bool) -> BackendResult<()> {
        let PeerRef::Group(g) = peer else { return Err(BackendError::InvalidInput("peer")) };
        match decode_group(g).ok_or(BackendError::NotFound)? {
            GroupTarget::Channel(ch) => {
                let channel = self.cache().peers.input_channel(ch, &self.net.session)?;
                let u = self.rpc(&tl::functions::channels::LeaveChannel { channel }).await?;
                self.apply_updates(u);
            }
            GroupTarget::Chat(chat_id) => {
                let req = tl::functions::messages::DeleteChatUser { revoke_history: then_delete, chat_id, user_id: tl::enums::InputUser::UserSelf };
                match self.rpc(&req).await {
                    Ok(u) => self.apply_updates(u),
                    // Ya fuera del grupo (p. ej. expulsado): se sigue con el borrado.
                    Err(e @ (BackendError::Network | BackendError::Unauthorized | BackendError::FloodWait(_))) => return Err(e),
                    Err(e) if !then_delete => return Err(e),
                    Err(e) => tracing::debug!(error = %e, "deleteChatUser falló; se borra el historial igualmente"),
                }
                if then_delete {
                    self.delete_history_rounds(peer, false, false).await?;
                }
            }
        }
        Ok(())
    }

    pub(crate) async fn act_leave_chat(&self, peer: PeerRef) -> BackendResult<()> {
        self.leave(peer, false).await?;
        self.drop_dialog(peer);
        Ok(())
    }

    pub(crate) async fn act_delete_dialog(self: &Arc<Self>, peer: PeerRef, revoke: bool) -> BackendResult<()> {
        match peer {
            PeerRef::User(u) => {
                // «Para ambos» no aplica a «Mensajes guardados».
                let revoke = revoke && Some(u) != self.me();
                self.delete_history_rounds(peer, false, revoke).await?;
            }
            PeerRef::Group(_) => self.leave(peer, true).await?,
            PeerRef::Secret(id) => self.secret_discard(id).await?,
        }
        self.drop_dialog(peer);
        Ok(())
    }

    pub(crate) async fn act_set_blocked(&self, user: UserId, blocked: bool) -> BackendResult<()> {
        if Some(user) == self.me() {
            return Err(BackendError::InvalidInput("user"));
        }
        let id = self.input_peer(PeerRef::User(user))?;
        if blocked {
            self.rpc(&tl::functions::contacts::Block { my_stories_from: false, id }).await?;
        } else {
            self.rpc(&tl::functions::contacts::Unblock { my_stories_from: false, id }).await?;
        }
        Ok(())
    }

    pub(crate) async fn act_chat_info(self: &Arc<Self>, peer: PeerRef) -> BackendResult<ChatInfo> {
        let user = match peer {
            PeerRef::User(u) => Some(u),
            PeerRef::Secret(s) => Some(self.secret_partner(s).await?),
            PeerRef::Group(_) => None,
        };
        if let Some(u) = user {
            let input = {
                let c = self.cache();
                c.peers.input_user(u, c.me, &self.net.session)?
            };
            let tl::enums::users::UserFull::Full(full) = self.rpc(&tl::functions::users::GetFullUser { id: input }).await?;
            self.ingest(&full.users, &full.chats);
            let tl::enums::UserFull::Full(f) = full.full_user;
            let username = self.cache().peers.users.get(&u).and_then(|u| u.username.clone());
            return Ok(ChatInfo { about: f.about.filter(|a| !a.is_empty()), username, phone: None, members: None, blocked: f.blocked });
        }
        let PeerRef::Group(g) = peer else { unreachable!("usuarios resueltos arriba") };
        let full = match decode_group(g).ok_or(BackendError::NotFound)? {
            GroupTarget::Chat(chat_id) => self.rpc(&tl::functions::messages::GetFullChat { chat_id }).await?,
            GroupTarget::Channel(ch) => {
                let channel = self.cache().peers.input_channel(ch, &self.net.session)?;
                self.rpc(&tl::functions::channels::GetFullChannel { channel }).await?
            }
        };
        let tl::enums::messages::ChatFull::Full(full) = full;
        self.ingest(&full.users, &full.chats);
        let (about, members) = match full.full_chat {
            tl::enums::ChatFull::Full(c) => {
                let n = match c.participants {
                    tl::enums::ChatParticipants::Participants(p) => u32::try_from(p.participants.len()).ok(),
                    tl::enums::ChatParticipants::Forbidden(_) => None,
                };
                (c.about, n)
            }
            tl::enums::ChatFull::ChannelFull(c) => (c.about, c.participants_count.and_then(|n| u32::try_from(n).ok())),
            tl::enums::ChatFull::CommunityFull(_) => (String::new(), None),
        };
        let username = self.cache().peers.groups.get(&g).and_then(|g| g.username.clone());
        Ok(ChatInfo { about: Some(about).filter(|a| !a.is_empty()), username, phone: None, members, blocked: false })
    }

    pub(crate) async fn act_pin_message(&self, peer: PeerRef, id: MsgId, pinned: bool) -> BackendResult<()> {
        if matches!(peer, PeerRef::Secret(_)) {
            return Err(BackendError::Forbidden("CHAT_NOT_SUPPORTED".into()));
        }
        let req = tl::functions::messages::UpdatePinnedMessage { silent: true, unpin: !pinned, pm_oneside: false, peer: self.input_peer(peer)?, id: id.0 };
        let u = self.rpc(&req).await?;
        self.apply_updates(u);
        Ok(())
    }
}
