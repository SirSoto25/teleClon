//! Estado compartido de la conexión (pares, diálogos cargados, envíos en curso) y
//! aplicación de los updates de Telegram → `BackendEvent`.
use std::collections::HashMap;

use grammers_tl_types as tl;
use tc_core::*;

use crate::convert::{DialogCtx, dialog_stub, draft_from_tl, is_muted, message_from_tl, reactions_from_tl};
use crate::ids::{channel_group, chat_group, peer_from_tl};
use crate::peers::{PeerStore, user_status};

/// Lista de diálogos cargada por páginas (principal o archivo).
#[derive(Default)]
pub struct DialogList {
    pub order: Vec<PeerRef>,
    /// Desplazamiento de la página siguiente: (fecha, id del mensaje superior, par).
    pub next: Option<(i32, i32, PeerRef)>,
    pub exhausted: bool,
}

#[derive(Default)]
pub struct Cache {
    pub peers: PeerStore,
    pub me: Option<UserId>,
    pub dialogs: HashMap<PeerRef, Dialog>,
    pub main: DialogList,
    pub archive: DialogList,
    /// En privados y grupos básicos los ids de mensaje son globales de la cuenta y
    /// `updateDeleteMessages` no dice de qué chat son: se recuerda aquí.
    msg_peer: HashMap<i32, PeerRef>,
    /// Envíos en curso por `random_id` → chat.
    pub pending: HashMap<i64, PeerRef>,
    /// Ids ya asignados a envíos (`updateMessageID`) → `random_id`.
    sent_ids: HashMap<(PeerRef, i32), RandomId>,
    /// Envíos confirmados, para responder igual si la UI reintenta el mismo `random_id`.
    pub sent: HashMap<i64, Message>,
    /// Documentos de stickers conocidos (para enviarlos).
    pub stickers: HashMap<i64, tl::types::InputDocument>,
    /// Temas conocidos de los grupos foro (ver `forum.rs`).
    pub forums: crate::forum::Topics,
}

const MSG_PEER_CAP: usize = 200_000;

impl Cache {
    pub fn ctx(&self, now: i64) -> DialogCtx<'_> {
        DialogCtx { peers: &self.peers, me: self.me, now }
    }

    pub fn list_mut(&mut self, archived: bool) -> &mut DialogList {
        if archived { &mut self.archive } else { &mut self.main }
    }

    /// Traduce y recuerda el chat de un mensaje.
    pub fn message(&mut self, m: &tl::enums::Message) -> Option<Message> {
        let msg = message_from_tl(m, self.me, &self.peers)?;
        self.remember(&msg);
        Some(msg)
    }

    pub fn remember(&mut self, m: &Message) {
        if crate::ids::channel_of(m.peer).is_none() {
            if self.msg_peer.len() >= MSG_PEER_CAP {
                self.msg_peer.clear();
            }
            self.msg_peer.insert(m.id.0, m.peer);
        }
    }

    pub fn forget(&mut self, peer: PeerRef, ids: &[MsgId]) {
        if crate::ids::channel_of(peer).is_none() {
            for id in ids {
                self.msg_peer.remove(&id.0);
            }
        }
    }

    /// El servidor asignó `id` al envío `random_id`.
    pub fn assign(&mut self, random_id: i64, id: i32) {
        if let Some(peer) = self.pending.get(&random_id) {
            self.sent_ids.insert((*peer, id), RandomId(random_id));
        }
    }

    pub fn random_of(&self, peer: PeerRef, id: MsgId) -> Option<RandomId> {
        self.sent_ids.get(&(peer, id.0)).copied()
    }

    /// Diálogo tras un mensaje nuevo (crea una fila mínima si no estaba cargado).
    fn bump(&mut self, m: &Message, now: i64) -> Dialog {
        let stub = dialog_stub(m.peer, &self.ctx(now));
        let d = self.dialogs.entry(m.peer).or_insert(stub);
        if d.last_message.as_ref().is_none_or(|l| l.id <= m.id) {
            d.last_message = Some(m.clone());
            d.activity = d.activity.max(m.date);
        }
        if m.outgoing {
            d.draft = None;
        } else {
            d.unread_count += 1;
        }
        d.clone()
    }

    fn with_dialog(&mut self, peer: PeerRef, f: impl FnOnce(&mut Dialog)) -> Option<Dialog> {
        let d = self.dialogs.get_mut(&peer)?;
        f(d);
        Some(d.clone())
    }
}

fn typing_of(a: &tl::enums::SendMessageAction) -> Option<bool> {
    use tl::enums::SendMessageAction as A;
    match a {
        A::SendMessageCancelAction => Some(false),
        A::SendMessageTypingAction
        | A::SendMessageRecordVideoAction
        | A::SendMessageUploadVideoAction(_)
        | A::SendMessageRecordAudioAction
        | A::SendMessageUploadAudioAction(_)
        | A::SendMessageUploadPhotoAction(_)
        | A::SendMessageUploadDocumentAction(_)
        | A::SendMessageRecordRoundAction
        | A::SendMessageUploadRoundAction(_)
        | A::SendMessageChooseStickerAction => Some(true),
        _ => None,
    }
}

/// Resultado de aplicar un lote: eventos para la UI y si hubo `updateLoginToken` (QR escaneado).
#[derive(Debug, Default, PartialEq)]
pub struct Applied {
    pub events: Vec<BackendEvent>,
    pub login_token: bool,
}

/// Aplica un update (los usuarios/chats del lote ya deben estar en `cache.peers`).
pub fn apply_update(cache: &mut Cache, u: &tl::enums::Update, now: i64, out: &mut Applied) {
    use tl::enums::Update as U;
    let ev = &mut out.events;
    match u {
        U::NewMessage(tl::types::UpdateNewMessage { message, .. })
        | U::NewChannelMessage(tl::types::UpdateNewChannelMessage { message, .. }) => {
            let Some(mut m) = cache.message(message) else { return };
            let d = cache.bump(&m, now);
            match (m.outgoing, cache.random_of(m.peer, m.id)) {
                (true, Some(rid)) => {
                    m.random_id = Some(rid);
                    cache.pending.remove(&rid.0);
                    cache.sent.insert(rid.0, m.clone());
                    ev.push(BackendEvent::MessageSent { random_id: rid, msg: m });
                }
                _ => ev.push(BackendEvent::MessageNew(m)),
            }
            // En un foro: `TopicMessageNew` y el tema actualizado (ver `forum.rs`).
            crate::forum::route_new_message(cache, message, ev);
            ev.push(BackendEvent::DialogUpserted(d));
        }
        U::EditMessage(tl::types::UpdateEditMessage { message, .. })
        | U::EditChannelMessage(tl::types::UpdateEditChannelMessage { message, .. }) => {
            let Some(m) = cache.message(message) else { return };
            let d = cache.with_dialog(m.peer, |d| {
                if d.last_message.as_ref().is_some_and(|l| l.id == m.id) {
                    d.last_message = Some(m.clone());
                }
            });
            let is_last = d.as_ref().and_then(|d| d.last_message.as_ref()).is_some_and(|l| l.id == m.id);
            ev.push(BackendEvent::MessageEdited(m));
            if let (true, Some(d)) = (is_last, d) {
                ev.push(BackendEvent::DialogUpserted(d));
            }
        }
        U::DeleteMessages(d) => {
            let mut by_peer: HashMap<PeerRef, Vec<MsgId>> = HashMap::new();
            for id in &d.messages {
                if let Some(p) = cache.msg_peer.remove(id) {
                    by_peer.entry(p).or_default().push(MsgId(*id));
                }
            }
            let mut peers: Vec<_> = by_peer.into_iter().collect();
            peers.sort_by_key(|(p, _)| *p);
            for (peer, ids) in peers {
                deleted(cache, peer, ids, ev);
            }
        }
        U::DeleteChannelMessages(d) => {
            let peer = PeerRef::Group(channel_group(d.channel_id));
            deleted(cache, peer, d.messages.iter().map(|i| MsgId(*i)).collect(), ev);
        }
        U::ReadHistoryInbox(r) => read_inbox(cache, peer_from_tl(&r.peer), r.max_id, r.still_unread_count, ev),
        U::ReadChannelInbox(r) => read_inbox(cache, PeerRef::Group(channel_group(r.channel_id)), r.max_id, r.still_unread_count, ev),
        U::ReadHistoryOutbox(r) => read_outbox(cache, peer_from_tl(&r.peer), r.max_id, ev),
        U::ReadChannelOutbox(r) => read_outbox(cache, PeerRef::Group(channel_group(r.channel_id)), r.max_id, ev),
        U::UserTyping(t) => {
            if let Some(typing) = typing_of(&t.action) {
                ev.push(BackendEvent::Typing { peer: PeerRef::User(UserId(t.user_id)), user: UserId(t.user_id), typing });
            }
        }
        U::ChatUserTyping(t) => {
            if let (Some(typing), Some(user)) = (typing_of(&t.action), crate::ids::user_of(&t.from_id)) {
                ev.push(BackendEvent::Typing { peer: PeerRef::Group(chat_group(t.chat_id)), user, typing });
            }
        }
        U::ChannelUserTyping(t) => {
            if let (Some(typing), Some(user)) = (typing_of(&t.action), crate::ids::user_of(&t.from_id)) {
                ev.push(BackendEvent::Typing { peer: PeerRef::Group(channel_group(t.channel_id)), user, typing });
            }
        }
        U::UserStatus(s) => {
            let status = user_status(Some(&s.status));
            if let Some(u) = cache.peers.users.get_mut(&UserId(s.user_id)) {
                u.status = status;
            }
            ev.push(BackendEvent::UserStatus { user: UserId(s.user_id), status });
        }
        U::MessageReactions(r) => {
            ev.push(BackendEvent::Reactions {
                peer: peer_from_tl(&r.peer),
                msg_id: MsgId(r.msg_id),
                reactions: reactions_from_tl(Some(&r.reactions)),
            });
        }
        U::DialogPinned(p) => {
            if let tl::enums::DialogPeer::Peer(dp) = &p.peer {
                let peer = peer_from_tl(&dp.peer);
                if p.pinned {
                    // Sin el orden completo: el recién fijado pasa delante.
                    for d in cache.dialogs.values_mut() {
                        if let Some(o) = d.pinned_order.as_mut() {
                            *o += 1;
                        }
                    }
                }
                if let Some(d) = cache.with_dialog(peer, |d| d.pinned_order = if p.pinned { Some(0) } else { None }) {
                    ev.push(BackendEvent::DialogUpserted(d));
                }
            }
        }
        U::NotifySettings(n) => {
            if let tl::enums::NotifyPeer::Peer(np) = &n.peer {
                let muted = is_muted(&n.notify_settings, now);
                if let Some(d) = cache.with_dialog(peer_from_tl(&np.peer), |d| d.muted = muted) {
                    ev.push(BackendEvent::DialogUpserted(d));
                }
            }
        }
        U::DraftMessage(dm) => {
            let draft = draft_from_tl(Some(&dm.draft));
            if let Some(d) = cache.with_dialog(peer_from_tl(&dm.peer), |d| d.draft = draft) {
                ev.push(BackendEvent::DialogUpserted(d));
            }
        }
        U::MessageId(m) => cache.assign(m.random_id, m.id),
        U::LoginToken => out.login_token = true,
        other => {
            crate::forum::apply_update(cache, other, ev);
        }
    }
}

pub(crate) fn deleted(cache: &mut Cache, peer: PeerRef, ids: Vec<MsgId>, ev: &mut Vec<BackendEvent>) {
    if ids.is_empty() {
        return;
    }
    cache.forget(peer, &ids);
    let d = cache.with_dialog(peer, |d| {
        if d.last_message.as_ref().is_some_and(|l| ids.contains(&l.id)) {
            d.last_message = None;
        }
    });
    ev.push(BackendEvent::MessagesDeleted { peer, ids });
    if let Some(d) = d {
        ev.push(BackendEvent::DialogUpserted(d));
    }
}

fn read_inbox(cache: &mut Cache, peer: PeerRef, max_id: i32, still_unread: i32, ev: &mut Vec<BackendEvent>) {
    ev.push(BackendEvent::ReadInbox { peer, max_id: MsgId(max_id) });
    if let Some(d) = cache.with_dialog(peer, |d| {
        d.unread_count = still_unread.max(0) as u32;
        d.read_inbox_max_id = d.read_inbox_max_id.max(MsgId(max_id));
    }) {
        ev.push(BackendEvent::DialogUpserted(d));
    }
}

fn read_outbox(cache: &mut Cache, peer: PeerRef, max_id: i32, ev: &mut Vec<BackendEvent>) {
    let _ = cache.with_dialog(peer, |d| d.read_outbox_max_id = d.read_outbox_max_id.max(MsgId(max_id)));
    ev.push(BackendEvent::ReadOutbox { peer, max_id: MsgId(max_id) });
}

/// Aplica un lote completo de updates con sus usuarios y chats.
pub fn apply_batch(cache: &mut Cache, updates: &[tl::enums::Update], users: &[tl::enums::User], chats: &[tl::enums::Chat], now: i64) -> (Applied, Vec<grammers_session::types::PeerInfo>) {
    let mut infos = cache.peers.ingest_users(users);
    infos.extend(cache.peers.ingest_chats(chats));
    let mut out = Applied::default();
    for u in updates {
        apply_update(cache, u, now, &mut out);
    }
    (out, infos)
}

/// Desempaqueta la respuesta `Updates` de una llamada (enviar, editar, reenviar…) en
/// `(updates, users, chats)`. `UpdateShortSentMessage` no trae el mensaje entero y se
/// devuelve aparte.
pub fn flatten_updates(u: tl::enums::Updates) -> (Vec<tl::enums::Update>, Vec<tl::enums::User>, Vec<tl::enums::Chat>, Option<tl::types::UpdateShortSentMessage>) {
    match u {
        tl::enums::Updates::Updates(u) => (u.updates, u.users, u.chats, None),
        tl::enums::Updates::Combined(u) => (u.updates, u.users, u.chats, None),
        tl::enums::Updates::UpdateShort(u) => (vec![u.update], vec![], vec![], None),
        tl::enums::Updates::UpdateShortSentMessage(s) => (vec![], vec![], vec![], Some(s)),
        // Mensajes cortos de otros (no llegan como respuesta a una llamada propia).
        _ => (vec![], vec![], vec![], None),
    }
}
