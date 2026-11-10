//! Implementación de los sub-traits de `ChatBackend` con funciones TL en crudo.
use std::collections::HashMap;

use async_trait::async_trait;
use futures::stream::{self, BoxStream, StreamExt};
use grammers_tl_types as tl;
use tc_core::*;
use tokio::sync::broadcast::error::RecvError;

use crate::convert::{DialogCtx, dialog_from_tl, entities_to_tl};
use crate::events::flatten_updates;
use crate::ids::{GroupTarget, channel_of, decode_group, peer_from_tl};
use crate::{GrammersBackend, Inner, now_secs};

/// Longitud máxima de un mensaje de texto (caracteres).
pub(crate) const MAX_TEXT: usize = 4096;

pub(crate) fn random_i64() -> i64 {
    let mut b = [0u8; 8];
    getrandom::fill(&mut b).expect("el sistema debe dar aleatoriedad");
    i64::from_le_bytes(b)
}

/// Parámetros de `messages.getHistory` para cada ancla: `(offset_id, add_offset, min_id)`.
/// Telegram devuelve de nuevo a viejo empezando justo por debajo de `offset_id`;
/// un `add_offset` negativo desplaza la ventana hacia mensajes más nuevos.
pub(crate) fn history_params(anchor: Anchor, limit: i32) -> (i32, i32, i32) {
    match anchor {
        Anchor::Latest => (0, 0, 0),
        Anchor::Before(id) => (id.0, 0, 0),
        Anchor::After(id) => (id.0.saturating_add(1), -limit, id.0),
        Anchor::Around(id) => (id.0.saturating_add(1), -(limit / 2), 0),
    }
}

fn reply_to(id: Option<MsgId>) -> Option<tl::enums::InputReplyTo> {
    id.map(|r| {
        tl::types::InputReplyToMessage {
            reply_to_msg_id: r.0,
            top_msg_id: None,
            reply_to_peer_id: None,
            quote_text: None,
            quote_entities: None,
            quote_offset: None,
            monoforum_peer_id: None,
            todo_item_id: None,
            poll_option: None,
        }
        .into()
    })
}

impl Inner {
    pub(crate) fn input_peer(&self, peer: PeerRef) -> BackendResult<tl::enums::InputPeer> {
        let c = self.cache();
        c.peers.input_peer(peer, c.me, &self.net.session)
    }

    pub(crate) fn ensure_logged_in(&self) -> BackendResult<UserId> {
        self.me().filter(|_| *self.logged_in.borrow()).ok_or(BackendError::Unauthorized)
    }

    /// Carga una página más de la lista (`archived` = carpeta 1).
    async fn fetch_dialog_page(&self, archived: bool, limit: i32) -> BackendResult<()> {
        let (offset_date, offset_id, offset_peer) = {
            let c = self.cache();
            let next = if archived { c.archive.next } else { c.main.next };
            match next {
                Some((d, id, p)) => (d, id, c.peers.input_peer(p, c.me, &self.net.session).unwrap_or(tl::enums::InputPeer::Empty)),
                None => (0, 0, tl::enums::InputPeer::Empty),
            }
        };
        let req = tl::functions::messages::GetDialogs {
            exclude_pinned: false,
            folder_id: archived.then_some(1),
            offset_date,
            offset_id,
            offset_peer,
            limit,
            hash: 0,
        };
        let (dialogs, messages, chats, users, complete) = match self.rpc(&req).await? {
            tl::enums::messages::Dialogs::Dialogs(d) => (d.dialogs, d.messages, d.chats, d.users, true),
            tl::enums::messages::Dialogs::Slice(d) => (d.dialogs, d.messages, d.chats, d.users, false),
            tl::enums::messages::Dialogs::NotModified(_) => (vec![], vec![], vec![], vec![], true),
        };
        self.ingest(&users, &chats);
        let now = now_secs();
        let mut c = self.cache();
        let mut tops: HashMap<(PeerRef, i32), Message> = HashMap::new();
        for m in &messages {
            if let Some(m) = c.message(m) {
                tops.insert((m.peer, m.id.0), m);
            }
        }
        let got = dialogs.len();
        let mut pinned = c.dialogs.values().filter(|d| d.archived == archived && d.pinned_order.is_some()).count() as u32;
        if (if archived { &c.archive } else { &c.main }).order.is_empty() {
            pinned = 0;
        }
        let mut next = None;
        for d in dialogs {
            let tl::enums::Dialog::Dialog(d) = d else { continue };
            let peer = peer_from_tl(&d.peer);
            let last = tops.get(&(peer, d.top_message)).cloned();
            let order = d.pinned.then(|| {
                pinned += 1;
                pinned - 1
            });
            let row = {
                let ctx = DialogCtx { peers: &c.peers, me: c.me, now };
                dialog_from_tl(&d, last.clone(), order, archived, &ctx)
            };
            next = Some((last.as_ref().map_or(0, |m| m.date.0 as i32), d.top_message, peer));
            c.dialogs.insert(peer, row);
            let list = c.list_mut(archived);
            if !list.order.contains(&peer) {
                list.order.push(peer);
            }
        }
        let list = c.list_mut(archived);
        list.next = next.or(list.next);
        list.exhausted = complete || got == 0 || got < limit as usize;
        Ok(())
    }

    /// Envío común: registra el `random_id`, llama y saca el mensaje confirmado.
    async fn send_and_confirm(
        &self,
        peer: PeerRef,
        random_id: RandomId,
        call: impl std::future::Future<Output = BackendResult<tl::enums::Updates>>,
        local: impl FnOnce(i32, i32, Option<tl::enums::MessageMedia>) -> Message,
    ) -> BackendResult<Message> {
        if let Some(prev) = self.cache().sent.get(&random_id.0).cloned() {
            return Ok(prev);
        }
        self.cache().pending.insert(random_id.0, peer);
        let res = call.await;
        let updates = match res {
            Ok(u) => u,
            Err(e) => {
                self.cache().pending.remove(&random_id.0);
                return Err(e);
            }
        };
        let (ups, users, chats, short) = flatten_updates(updates);
        if let Some(s) = short {
            let mut m = local(s.id, s.date, s.media);
            m.random_id = Some(random_id);
            let d = {
                let mut c = self.cache();
                c.remember(&m);
                c.assign(random_id.0, s.id);
                c.sent.insert(random_id.0, m.clone());
                let stub = crate::convert::dialog_stub(peer, &c.ctx(now_secs()));
                let d = c.dialogs.entry(peer).or_insert(stub);
                d.last_message = Some(m.clone());
                d.activity = d.activity.max(m.date);
                d.draft = None;
                d.clone()
            };
            self.emit(BackendEvent::MessageSent { random_id, msg: m.clone() });
            self.emit(BackendEvent::DialogUpserted(d));
            return Ok(m);
        }
        let applied = self.apply_response(ups, users, chats);
        let found = applied.events.iter().find_map(|e| match e {
            BackendEvent::MessageSent { random_id: r, msg } if *r == random_id => Some(msg.clone()),
            _ => None,
        });
        self.emit_all(applied.events);
        let msg = found.or_else(|| self.cache().sent.get(&random_id.0).cloned()).ok_or(BackendError::Internal)?;
        Ok(msg)
    }

    /// Mensaje saliente local a partir de lo enviado (cuando el servidor solo da id y fecha).
    pub(crate) fn outgoing(&self, peer: PeerRef, id: i32, date: i32, text: String, entities: Vec<TextEntity>, reply: Option<MsgId>) -> Message {
        Message {
            id: MsgId(id),
            peer,
            sender: self.me(),
            date: Timestamp(date.into()),
            edit_date: None,
            outgoing: true,
            text,
            entities,
            reply_to: reply,
            forwarded_from: None,
            media: None,
            sticker: None,
            reactions: vec![],
            random_id: None,
            state: SendState::Sent,
            ttl_secs: None,
        }
    }

    pub(crate) async fn send_media_item(
        &self,
        peer: PeerRef,
        media: tl::enums::InputMedia,
        caption: String,
        reply: Option<MsgId>,
        random_id: RandomId,
    ) -> BackendResult<Message> {
        let input = self.input_peer(peer)?;
        let req = tl::functions::messages::SendMedia {
            silent: false,
            background: false,
            clear_draft: false,
            noforwards: false,
            update_stickersets_order: false,
            invert_media: false,
            allow_paid_floodskip: false,
            peer: input,
            reply_to: reply_to(reply),
            media,
            message: caption.clone(),
            random_id: random_id.0,
            reply_markup: None,
            entities: None,
            schedule_date: None,
            schedule_repeat_period: None,
            send_as: None,
            quick_reply_shortcut: None,
            effect: None,
            allow_paid_stars: None,
            suggested_post: None,
        };
        let me = self;
        self.send_and_confirm(peer, random_id, self.rpc(&req), move |id, date, m| {
            let mut msg = me.outgoing(peer, id, date, caption, vec![], reply);
            if let Some(m) = m {
                (msg.media, msg.sticker) = crate::convert::media_from_tl(&m);
            }
            msg
        })
        .await
    }

    fn dialog_changed(&self, peer: PeerRef, f: impl FnOnce(&mut Dialog)) {
        let d = {
            let mut c = self.cache();
            c.dialogs.get_mut(&peer).map(|d| {
                f(d);
                d.clone()
            })
        };
        if let Some(d) = d {
            self.emit(BackendEvent::DialogUpserted(d));
        }
    }
}

pub(crate) fn messages_of(r: tl::enums::messages::Messages) -> (Vec<tl::enums::Message>, Vec<tl::enums::User>, Vec<tl::enums::Chat>) {
    match r {
        tl::enums::messages::Messages::Messages(m) => (m.messages, m.users, m.chats),
        tl::enums::messages::Messages::Slice(m) => (m.messages, m.users, m.chats),
        tl::enums::messages::Messages::ChannelMessages(m) => (m.messages, m.users, m.chats),
        tl::enums::messages::Messages::NotModified(_) => (vec![], vec![], vec![]),
    }
}

#[async_trait]
impl ProfileApi for GrammersBackend {
    async fn me(&self) -> BackendResult<User> {
        let me = self.inner.ensure_logged_in()?;
        if let Some(u) = self.inner.cache().peers.users.get(&me).cloned() {
            return Ok(u);
        }
        let users = self.inner.rpc(&tl::functions::users::GetUsers { id: vec![tl::enums::InputUser::UserSelf] }).await?;
        self.inner.ingest(&users, &[]);
        self.inner.cache().peers.users.get(&me).cloned().ok_or(BackendError::NotFound)
    }

    async fn user(&self, id: UserId) -> BackendResult<User> {
        if let Some(u) = self.inner.cache().peers.users.get(&id).cloned() {
            return Ok(u);
        }
        let input = {
            let c = self.inner.cache();
            c.peers.input_user(id, c.me, &self.inner.net.session)?
        };
        let users = self.inner.rpc(&tl::functions::users::GetUsers { id: vec![input] }).await?;
        self.inner.ingest(&users, &[]);
        self.inner.cache().peers.users.get(&id).cloned().ok_or(BackendError::NotFound)
    }

    async fn group(&self, id: GroupId) -> BackendResult<Group> {
        if let Some(g) = self.inner.cache().peers.groups.get(&id).cloned() {
            return Ok(g);
        }
        let chats = match decode_group(id).ok_or(BackendError::NotFound)? {
            GroupTarget::Chat(c) => self.inner.rpc(&tl::functions::messages::GetChats { id: vec![c] }).await?,
            GroupTarget::Channel(c) => {
                let input = self.inner.cache().peers.input_channel(c, &self.inner.net.session)?;
                self.inner.rpc(&tl::functions::channels::GetChannels { id: vec![input] }).await?
            }
        };
        let chats = match chats {
            tl::enums::messages::Chats::Chats(c) => c.chats,
            tl::enums::messages::Chats::Slice(c) => c.chats,
        };
        self.inner.ingest(&[], &chats);
        self.inner.cache().peers.groups.get(&id).cloned().ok_or(BackendError::NotFound)
    }

    async fn contacts(&self) -> BackendResult<Vec<User>> {
        match self.inner.rpc(&tl::functions::contacts::GetContacts { hash: 0 }).await? {
            tl::enums::contacts::Contacts::Contacts(c) => {
                self.inner.ingest(&c.users, &[]);
                let cache = self.inner.cache();
                Ok(c.users
                    .iter()
                    .filter_map(|u| match u {
                        tl::enums::User::User(u) => cache.peers.users.get(&UserId(u.id)).cloned(),
                        _ => None,
                    })
                    .collect())
            }
            tl::enums::contacts::Contacts::NotModified => Ok(vec![]),
        }
    }
}

#[async_trait]
impl DialogsApi for GrammersBackend {
    async fn dialogs(&self, page: DialogsPage) -> BackendResult<Vec<Dialog>> {
        self.inner.ensure_logged_in()?;
        if page.offset == 0 {
            // Primera página: se recarga desde el principio.
            *self.inner.cache().list_mut(page.archived) = Default::default();
        }
        let want = page.offset + page.limit;
        loop {
            let (have, exhausted) = {
                let mut c = self.inner.cache();
                let l = c.list_mut(page.archived);
                (l.order.len(), l.exhausted)
            };
            if have >= want || exhausted {
                break;
            }
            let lim = (want - have).clamp(20, 100) as i32;
            self.inner.fetch_dialog_page(page.archived, lim).await?;
        }
        let c = self.inner.cache();
        let l = if page.archived { &c.archive } else { &c.main };
        Ok(l.order.iter().skip(page.offset).take(page.limit).filter_map(|p| c.dialogs.get(p).cloned()).collect())
    }

    async fn set_pinned(&self, peer: PeerRef, pinned: bool) -> BackendResult<()> {
        let input = self.inner.input_peer(peer)?;
        self.inner
            .rpc(&tl::functions::messages::ToggleDialogPin { pinned, peer: tl::types::InputDialogPeer { peer: input }.into() })
            .await?;
        let next = {
            let c = self.inner.cache();
            c.dialogs.values().filter_map(|d| d.pinned_order).max().map_or(0, |m| m + 1)
        };
        self.inner.dialog_changed(peer, |d| d.pinned_order = if pinned { d.pinned_order.or(Some(next)) } else { None });
        Ok(())
    }

    async fn set_archived(&self, peer: PeerRef, archived: bool) -> BackendResult<()> {
        let input = self.inner.input_peer(peer)?;
        let r = self
            .inner
            .rpc(&tl::functions::folders::EditPeerFolders {
                folder_peers: vec![tl::types::InputFolderPeer { peer: input, folder_id: i32::from(archived) }.into()],
            })
            .await?;
        let (ups, users, chats, _) = flatten_updates(r);
        let applied = self.inner.apply_response(ups, users, chats);
        self.inner.emit_all(applied.events);
        {
            let mut c = self.inner.cache();
            c.list_mut(!archived).order.retain(|p| *p != peer);
            let l = c.list_mut(archived);
            if !l.order.contains(&peer) {
                l.order.insert(0, peer);
            }
        }
        self.inner.dialog_changed(peer, |d| {
            d.archived = archived;
            if archived {
                d.pinned_order = None;
            }
        });
        Ok(())
    }

    async fn set_muted(&self, peer: PeerRef, muted: bool) -> BackendResult<()> {
        let input = self.inner.input_peer(peer)?;
        let settings = tl::types::InputPeerNotifySettings {
            show_previews: None,
            silent: None,
            mute_until: Some(if muted { i32::MAX } else { 0 }),
            sound: None,
            stories_muted: None,
            stories_hide_sender: None,
            stories_sound: None,
        };
        self.inner
            .rpc(&tl::functions::account::UpdateNotifySettings { peer: tl::types::InputNotifyPeer { peer: input }.into(), settings: settings.into() })
            .await?;
        self.inner.dialog_changed(peer, |d| d.muted = muted);
        Ok(())
    }

    async fn save_draft(&self, peer: PeerRef, draft: Option<Draft>) -> BackendResult<()> {
        let input = self.inner.input_peer(peer)?;
        let draft = draft.filter(|d| !d.text.is_empty());
        let (text, reply) = draft.as_ref().map_or((String::new(), None), |d| (d.text.clone(), d.reply_to));
        self.inner
            .rpc(&tl::functions::messages::SaveDraft {
                no_webpage: false,
                invert_media: false,
                reply_to: reply_to(reply),
                peer: input,
                message: text,
                entities: None,
                media: None,
                effect: None,
                suggested_post: None,
                rich_message: None,
            })
            .await?;
        self.inner.dialog_changed(peer, |d| d.draft = draft);
        Ok(())
    }
}

#[async_trait]
impl MessagesApi for GrammersBackend {
    async fn history(&self, peer: PeerRef, anchor: Anchor, limit: usize) -> BackendResult<Vec<Message>> {
        if let PeerRef::Secret(id) = peer {
            return self.inner.secret_history(id, anchor, limit).await;
        }
        let input = self.inner.input_peer(peer)?;
        let lim = limit.clamp(1, 100) as i32;
        let (offset_id, add_offset, min_id) = history_params(anchor, lim);
        let r = self
            .inner
            .rpc(&tl::functions::messages::GetHistory { peer: input, offset_id, offset_date: 0, add_offset, limit: lim, max_id: 0, min_id, hash: 0 })
            .await?;
        let (msgs, users, chats) = messages_of(r);
        self.inner.ingest(&users, &chats);
        let mut c = self.inner.cache();
        let mut out: Vec<Message> = msgs.iter().filter_map(|m| c.message(m)).filter(|m| m.peer == peer).collect();
        if let Anchor::After(id) = anchor {
            out.retain(|m| m.id > id);
        }
        out.sort_by_key(|m| m.id);
        out.dedup_by_key(|m| m.id);
        Ok(out)
    }

    async fn send_text(&self, peer: PeerRef, msg: OutgoingText) -> BackendResult<Message> {
        if msg.text.trim().is_empty() || msg.text.chars().count() > MAX_TEXT {
            return Err(BackendError::InvalidInput("text"));
        }
        if let PeerRef::Secret(id) = peer {
            return self.inner.secret_send_text(id, msg).await;
        }
        let input = self.inner.input_peer(peer).inspect_err(|e| {
            tracing::warn!(error = %e, "send_text: no se pudo resolver el destinatario (falta access_hash en caché)");
        })?;
        let req = tl::functions::messages::SendMessage {
            no_webpage: false,
            silent: false,
            background: false,
            clear_draft: true,
            noforwards: false,
            update_stickersets_order: false,
            invert_media: false,
            allow_paid_floodskip: false,
            peer: input,
            reply_to: reply_to(msg.reply_to),
            message: msg.text.clone(),
            random_id: msg.random_id.0,
            reply_markup: None,
            entities: entities_to_tl(&msg.text, &msg.entities),
            schedule_date: None,
            schedule_repeat_period: None,
            send_as: None,
            quick_reply_shortcut: None,
            effect: None,
            allow_paid_stars: None,
            suggested_post: None,
            rich_message: None,
        };
        let inner = &self.inner;
        let OutgoingText { text, entities, reply_to: reply, random_id } = msg;
        inner
            .send_and_confirm(peer, random_id, inner.rpc(&req), move |id, date, media| {
                let mut m = inner.outgoing(peer, id, date, text, entities, reply);
                if let Some(media) = media {
                    (m.media, m.sticker) = crate::convert::media_from_tl(&media);
                }
                m
            })
            .await
    }

    async fn edit_message(&self, peer: PeerRef, id: MsgId, text: String, entities: Vec<TextEntity>) -> BackendResult<Message> {
        if text.trim().is_empty() || text.chars().count() > MAX_TEXT {
            return Err(BackendError::InvalidInput("text"));
        }
        let input = self.inner.input_peer(peer)?;
        let r = self
            .inner
            .rpc(&tl::functions::messages::EditMessage {
                no_webpage: false,
                invert_media: false,
                peer: input,
                id: id.0,
                message: Some(text.clone()),
                media: None,
                reply_markup: None,
                entities: entities_to_tl(&text, &entities),
                schedule_date: None,
                schedule_repeat_period: None,
                quick_reply_shortcut_id: None,
                rich_message: None,
            })
            .await?;
        let (ups, users, chats, _) = flatten_updates(r);
        let applied = self.inner.apply_response(ups, users, chats);
        let found = applied.events.iter().find_map(|e| match e {
            BackendEvent::MessageEdited(m) if m.id == id && m.peer == peer => Some(m.clone()),
            _ => None,
        });
        self.inner.emit_all(applied.events);
        found.ok_or(BackendError::Internal)
    }

    async fn delete_messages(&self, peer: PeerRef, ids: Vec<MsgId>, revoke: bool) -> BackendResult<()> {
        if ids.is_empty() {
            return Ok(());
        }
        if let PeerRef::Secret(id) = peer {
            return self.inner.secret_delete(id, ids, revoke).await;
        }
        let raw: Vec<i32> = ids.iter().map(|i| i.0).collect();
        match channel_of(peer) {
            Some(ch) => {
                let input = self.inner.cache().peers.input_channel(ch, &self.inner.net.session)?;
                self.inner.rpc(&tl::functions::channels::DeleteMessages { channel: input, id: raw }).await?;
            }
            None => {
                self.inner.rpc(&tl::functions::messages::DeleteMessages { revoke, id: raw }).await?;
            }
        }
        // La confirmación propia no llega como update de borrado: se emite aquí.
        let mut ev = Vec::new();
        {
            let mut c = self.inner.cache();
            crate::events::deleted(&mut c, peer, ids, &mut ev);
        }
        self.inner.emit_all(ev);
        Ok(())
    }

    async fn forward_messages(&self, from: PeerRef, ids: Vec<MsgId>, to: Vec<PeerRef>, hide_sender: bool) -> BackendResult<()> {
        let from_peer = self.inner.input_peer(from)?;
        for dest in to {
            let to_peer = self.inner.input_peer(dest)?;
            let req = tl::functions::messages::ForwardMessages {
                silent: false,
                background: false,
                with_my_score: false,
                drop_author: hide_sender,
                drop_media_captions: false,
                noforwards: false,
                allow_paid_floodskip: false,
                from_ephemeral: false,
                from_peer: from_peer.clone(),
                id: ids.iter().map(|i| i.0).collect(),
                random_id: ids.iter().map(|_| random_i64()).collect(),
                to_peer,
                top_msg_id: None,
                reply_to: None,
                schedule_date: None,
                schedule_repeat_period: None,
                send_as: None,
                quick_reply_shortcut: None,
                effect: None,
                video_timestamp: None,
                allow_paid_stars: None,
                suggested_post: None,
            };
            let (ups, users, chats, _) = flatten_updates(self.inner.rpc(&req).await?);
            let applied = self.inner.apply_response(ups, users, chats);
            self.inner.emit_all(applied.events);
        }
        Ok(())
    }

    async fn send_reaction(&self, peer: PeerRef, id: MsgId, emoji: Option<String>) -> BackendResult<()> {
        let input = self.inner.input_peer(peer)?;
        let reaction = Some(emoji.map_or_else(Vec::new, |e| vec![tl::types::ReactionEmoji { emoticon: e }.into()]));
        let r = self
            .inner
            .rpc(&tl::functions::messages::SendReaction { big: false, add_to_recent: true, peer: input, msg_id: id.0, reaction })
            .await?;
        let (ups, users, chats, _) = flatten_updates(r);
        let applied = self.inner.apply_response(ups, users, chats);
        self.inner.emit_all(applied.events);
        Ok(())
    }

    /// Ver `reactions.rs`.
    async fn available_reactions(&self, peer: PeerRef) -> BackendResult<Vec<String>> {
        self.inner.available_reactions(peer).await
    }

    async fn mark_read(&self, peer: PeerRef, max_id: MsgId) -> BackendResult<()> {
        if let PeerRef::Secret(id) = peer {
            return self.inner.secret_mark_read(id, max_id).await;
        }
        match channel_of(peer) {
            Some(ch) => {
                let input = self.inner.cache().peers.input_channel(ch, &self.inner.net.session)?;
                self.inner.rpc(&tl::functions::channels::ReadHistory { channel: input, max_id: max_id.0 }).await?;
            }
            None => {
                let input = self.inner.input_peer(peer)?;
                self.inner.rpc(&tl::functions::messages::ReadHistory { peer: input, max_id: max_id.0 }).await?;
            }
        }
        self.inner.emit(BackendEvent::ReadInbox { peer, max_id });
        self.inner.dialog_changed(peer, |d| {
            if d.last_message.as_ref().is_none_or(|l| l.id <= max_id) {
                d.unread_count = 0;
            }
        });
        Ok(())
    }

    async fn set_typing(&self, peer: PeerRef, typing: bool) -> BackendResult<()> {
        if let PeerRef::Secret(id) = peer {
            return self.inner.secret_typing(id, typing).await;
        }
        let input = self.inner.input_peer(peer)?;
        let action =
            if typing { tl::enums::SendMessageAction::SendMessageTypingAction } else { tl::enums::SendMessageAction::SendMessageCancelAction };
        self.inner.rpc(&tl::functions::messages::SetTyping { peer: input, top_msg_id: None, action }).await?;
        Ok(())
    }
}

#[async_trait]
impl StickersApi for GrammersBackend {
    /// Ver `stickers.rs` (packs con caché por `hash`, tope `MAX_SETS`, un pack roto no tumba la lista).
    async fn installed_sticker_sets(&self) -> BackendResult<Vec<StickerSet>> {
        self.inner.installed_sticker_sets().await
    }

    async fn send_sticker(&self, peer: PeerRef, sticker: StickerId, reply_to: Option<MsgId>, random_id: RandomId) -> BackendResult<Message> {
        self.inner.send_sticker(peer, sticker, reply_to, random_id).await
    }

    async fn recent_stickers(&self) -> BackendResult<Vec<Sticker>> {
        self.inner.recent_stickers().await
    }

    async fn faved_stickers(&self) -> BackendResult<Vec<Sticker>> {
        self.inner.faved_stickers().await
    }
}

/// EXPERIMENTAL, sin auditoría y sin prueba de interoperabilidad con clientes oficiales
/// (ver `secret.rs` y `crates/tc-secret/README.md`).
#[async_trait]
impl SecretChatsApi for GrammersBackend {
    async fn secret_chats(&self) -> BackendResult<Vec<SecretChat>> {
        self.inner.secret_list().await
    }
    async fn request_secret_chat(&self, user: UserId) -> BackendResult<SecretChat> {
        self.inner.secret_request(user).await
    }
    async fn accept_secret_chat(&self, id: SecretChatId) -> BackendResult<SecretChat> {
        self.inner.secret_accept(id).await
    }
    async fn discard_secret_chat(&self, id: SecretChatId) -> BackendResult<()> {
        self.inner.secret_discard(id).await
    }
    async fn set_secret_ttl(&self, id: SecretChatId, ttl_secs: Option<u32>) -> BackendResult<()> {
        self.inner.secret_set_ttl(id, ttl_secs).await
    }
    async fn secret_key_visual(&self, id: SecretChatId) -> BackendResult<KeyVisual> {
        self.inner.secret_visual(id).await
    }
}

impl ChatBackend for GrammersBackend {
    fn events(&self) -> BoxStream<'static, BackendEvent> {
        let rx = self.inner.tx.subscribe();
        stream::unfold(rx, |mut rx| async move {
            loop {
                match rx.recv().await {
                    Ok(ev) => return Some((ev, rx)),
                    Err(RecvError::Lagged(_)) => continue,
                    Err(RecvError::Closed) => return None,
                }
            }
        })
        .boxed()
    }
}
