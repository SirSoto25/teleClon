//! `MockBackend`: implementación simulada y determinista de `tc_core::ChatBackend`
//! (semilla fija, latencia configurable, modo sin conexión, mensajes entrantes guionizados
//! con tokio y un chat secreto simulado). Ver docs/architecture.md §5.
//!
//! El chat secreto de este hito es solo una simulación de estado: no ejecuta todavía el
//! cifrado real de `tc-secret` (la huella visual es ilustrativa, no deriva de una clave).
#![forbid(unsafe_code)]

mod data;
mod rng;

use std::collections::{BTreeMap, HashMap};
use std::sync::{Arc, Mutex, MutexGuard};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use async_trait::async_trait;
use futures::stream::{self, BoxStream, StreamExt};
use tc_core::*;
use tokio::sync::broadcast::{self, error::RecvError};
use tokio::task::JoinHandle;

use crate::rng::Lcg;
pub use data::{BIG_CHAT, BIG_CHAT_LEN, ME, SECRET_ID};

#[derive(Debug, Clone)]
pub struct MockConfig {
    pub seed: u64,
    /// Retardo artificial de cada operación (0 = inmediato).
    pub latency: Duration,
    /// Instante "ahora" del que cuelgan las fechas de muestra (`None` = reloj real).
    pub now: Option<i64>,
    /// Retardo con el que la otra parte acepta un chat secreto solicitado.
    pub secret_accept_delay: Duration,
}

impl Default for MockConfig {
    fn default() -> Self {
        Self { seed: 42, latency: Duration::ZERO, now: None, secret_accept_delay: Duration::from_millis(300) }
    }
}

struct State {
    me: User,
    users: BTreeMap<UserId, User>,
    groups: BTreeMap<GroupId, Group>,
    secrets: BTreeMap<SecretChatId, SecretChat>,
    dialogs: HashMap<PeerRef, Dialog>,
    messages: HashMap<PeerRef, Vec<Message>>,
    sets: Vec<StickerSet>,
    rng: Lcg,
    offline: bool,
    clock: i64,
    next_secret: i32,
}

struct Inner {
    state: Mutex<State>,
    tx: broadcast::Sender<BackendEvent>,
    cfg: MockConfig,
}

pub struct MockBackend {
    inner: Arc<Inner>,
    script: Mutex<Option<JoinHandle<()>>>,
}

fn real_now() -> i64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map_or(0, |d| d.as_secs() as i64)
}

impl Default for MockBackend {
    fn default() -> Self {
        Self::new(MockConfig::default())
    }
}

impl MockBackend {
    pub fn new(cfg: MockConfig) -> Self {
        let now = cfg.now.unwrap_or_else(real_now);
        let seed = data::build(cfg.seed, now);
        let state = State {
            me: seed.me,
            users: seed.users,
            groups: seed.groups,
            secrets: seed.secrets,
            dialogs: seed.dialogs,
            messages: seed.messages,
            sets: seed.sets,
            rng: Lcg::new(cfg.seed.wrapping_add(1)),
            offline: false,
            clock: now,
            next_secret: 2,
        };
        let (tx, _) = broadcast::channel(1024);
        Self { inner: Arc::new(Inner { state: Mutex::new(state), tx, cfg }), script: Mutex::new(None) }
    }

    /// Conmuta el modo sin conexión: las operaciones devuelven `Network` y se emite `Connection`.
    pub fn set_offline(&self, offline: bool) {
        self.inner.lock().offline = offline;
        self.inner.emit(BackendEvent::Connection(if offline {
            ConnectionState::Offline
        } else {
            ConnectionState::Online
        }));
    }

    /// Arranca el guion de tiempo real: cada `interval` llega un mensaje entrante simulado
    /// (precedido de «escribiendo…»). Requiere un runtime tokio. Sustituye a un guion previo.
    pub fn start_script(&self, interval: Duration) {
        let inner = Arc::clone(&self.inner);
        let handle = tokio::spawn(async move {
            loop {
                tokio::time::sleep(interval).await;
                inner.simulate_incoming(interval / 4).await;
            }
        });
        if let Some(old) = self.script.lock().unwrap().replace(handle) {
            old.abort();
        }
    }

    pub fn stop_script(&self) {
        if let Some(h) = self.script.lock().unwrap().take() {
            h.abort();
        }
    }

    /// Inyecta un mensaje entrante ahora mismo (útil en tests y demos).
    pub fn inject_incoming(&self, peer: PeerRef, text: &str) -> BackendResult<Message> {
        self.inner.incoming(peer, text.to_string())
    }
}

impl Drop for MockBackend {
    fn drop(&mut self) {
        self.stop_script();
    }
}

impl State {
    fn tick(&mut self) -> Timestamp {
        self.clock += 1;
        Timestamp(self.clock)
    }

    fn peer_title(&self, peer: PeerRef) -> Option<String> {
        match peer {
            PeerRef::User(u) => self.users.get(&u).map(User::display_name),
            PeerRef::Group(g) => self.groups.get(&g).map(|g| g.title.clone()),
            PeerRef::Secret(s) => self.secrets.get(&s).and_then(|s| self.users.get(&s.user)).map(User::display_name),
        }
    }

    fn ensure_peer(&self, peer: PeerRef) -> BackendResult<()> {
        if self.dialogs.contains_key(&peer) { Ok(()) } else { Err(BackendError::NotFound) }
    }

    fn ensure_writable(&self, peer: PeerRef) -> BackendResult<()> {
        self.ensure_peer(peer)?;
        match peer {
            PeerRef::Secret(s) => match self.secrets.get(&s).map(|c| c.state) {
                Some(SecretChatState::Accepted) => Ok(()),
                _ => Err(BackendError::Forbidden("el chat secreto no está activo".into())),
            },
            PeerRef::Group(g) if self.groups.get(&g).is_some_and(|g| g.kind == GroupKind::Channel) => {
                Err(BackendError::Forbidden("no puedes escribir en este canal".into()))
            }
            _ => Ok(()),
        }
    }

    fn blank_message(&self, peer: PeerRef, outgoing: bool, sender: Option<UserId>) -> Message {
        Message {
            id: MsgId(0),
            peer,
            sender,
            date: Timestamp(self.clock),
            edit_date: None,
            outgoing,
            text: String::new(),
            entities: vec![],
            reply_to: None,
            forwarded_from: None,
            media: None,
            sticker: None,
            reactions: vec![],
            random_id: None,
            state: SendState::Sent,
            ttl_secs: match peer {
                PeerRef::Secret(s) => self.secrets.get(&s).and_then(|c| c.ttl_secs),
                _ => None,
            },
        }
    }

    /// Añade el mensaje al historial, asigna id y fecha, y actualiza el diálogo.
    fn append(&mut self, mut m: Message) -> (Message, Dialog) {
        let peer = m.peer;
        m.date = self.tick();
        let list = self.messages.entry(peer).or_default();
        m.id = MsgId(list.last().map_or(0, |l| l.id.0) + 1);
        list.push(m.clone());
        let d = self.dialogs.get_mut(&peer).expect("diálogo comprobado antes");
        d.last_message = Some(m.clone());
        d.activity = m.date;
        if !m.outgoing {
            d.unread_count += 1;
        } else {
            d.draft = None;
        }
        (m, d.clone())
    }

    fn find_mut(&mut self, peer: PeerRef, id: MsgId) -> BackendResult<&mut Message> {
        let list = self.messages.get_mut(&peer).ok_or(BackendError::NotFound)?;
        let idx = list.binary_search_by_key(&id, |m| m.id).map_err(|_| BackendError::NotFound)?;
        Ok(&mut list[idx])
    }

    fn refresh_last(&mut self, peer: PeerRef) -> Dialog {
        let last = self.messages.get(&peer).and_then(|l| l.last().cloned());
        let d = self.dialogs.get_mut(&peer).expect("diálogo existente");
        d.last_message = last;
        d.clone()
    }
}

impl Inner {
    fn lock(&self) -> MutexGuard<'_, State> {
        self.state.lock().unwrap_or_else(|e| e.into_inner())
    }

    fn emit(&self, ev: BackendEvent) {
        // Sin suscriptores no es un error.
        let _ = self.tx.send(ev);
    }

    async fn gate(&self) -> BackendResult<()> {
        if !self.cfg.latency.is_zero() {
            tokio::time::sleep(self.cfg.latency).await;
        }
        if self.lock().offline { Err(BackendError::Network) } else { Ok(()) }
    }

    fn incoming(&self, peer: PeerRef, text: String) -> BackendResult<Message> {
        let (m, d) = {
            let mut st = self.lock();
            st.ensure_peer(peer)?;
            let sender = match peer {
                PeerRef::User(u) => Some(u),
                PeerRef::Secret(s) => st.secrets.get(&s).map(|c| c.user),
                PeerRef::Group(g) => {
                    if st.groups.get(&g).is_some_and(|g| g.kind == GroupKind::Channel) {
                        None
                    } else {
                        let ids: Vec<UserId> = st.users.keys().copied().collect();
                        Some(ids[st.rng.below(ids.len())])
                    }
                }
            };
            let mut m = st.blank_message(peer, false, sender);
            m.text = text;
            st.append(m)
        };
        self.emit(BackendEvent::MessageNew(m.clone()));
        self.emit(BackendEvent::DialogUpserted(d));
        Ok(m)
    }

    async fn simulate_incoming(&self, typing_for: Duration) {
        let (peer, user, text) = {
            let mut st = self.lock();
            if st.offline {
                return;
            }
            let mut peers: Vec<PeerRef> = st
                .dialogs
                .values()
                .filter(|d| !d.archived && !matches!(d.peer, PeerRef::Secret(_)))
                .map(|d| d.peer)
                .collect();
            peers.sort_by_key(|p| format!("{p:?}"));
            let peer = peers[st.rng.below(peers.len())];
            let user = match peer {
                PeerRef::User(u) => Some(u),
                _ => None,
            };
            (peer, user, data::incoming_text(&mut st.rng))
        };
        if let Some(user) = user {
            self.emit(BackendEvent::Typing { peer, user, typing: true });
            tokio::time::sleep(typing_for).await;
            self.emit(BackendEvent::Typing { peer, user, typing: false });
        }
        let _ = self.incoming(peer, text);
    }
}

#[async_trait]
impl ProfileApi for MockBackend {
    async fn me(&self) -> BackendResult<User> {
        self.inner.gate().await?;
        Ok(self.inner.lock().me.clone())
    }
    async fn user(&self, id: UserId) -> BackendResult<User> {
        self.inner.gate().await?;
        self.inner.lock().users.get(&id).cloned().ok_or(BackendError::NotFound)
    }
    async fn group(&self, id: GroupId) -> BackendResult<Group> {
        self.inner.gate().await?;
        self.inner.lock().groups.get(&id).cloned().ok_or(BackendError::NotFound)
    }
    async fn contacts(&self) -> BackendResult<Vec<User>> {
        self.inner.gate().await?;
        Ok(self.inner.lock().users.values().filter(|u| u.is_contact).cloned().collect())
    }
}

#[async_trait]
impl DialogsApi for MockBackend {
    async fn dialogs(&self, page: DialogsPage) -> BackendResult<Vec<Dialog>> {
        self.inner.gate().await?;
        let st = self.inner.lock();
        let mut list: Vec<Dialog> = st.dialogs.values().filter(|d| d.archived == page.archived).cloned().collect();
        list.sort_by(|a, b| {
            let pa = a.pinned_order.unwrap_or(u32::MAX);
            let pb = b.pinned_order.unwrap_or(u32::MAX);
            pa.cmp(&pb).then(b.activity.cmp(&a.activity)).then_with(|| format!("{:?}", a.peer).cmp(&format!("{:?}", b.peer)))
        });
        Ok(list.into_iter().skip(page.offset).take(page.limit).collect())
    }

    async fn set_pinned(&self, peer: PeerRef, pinned: bool) -> BackendResult<()> {
        self.inner.gate().await?;
        let d = {
            let mut st = self.inner.lock();
            st.ensure_peer(peer)?;
            let next = st.dialogs.values().filter_map(|d| d.pinned_order).max().map_or(0, |m| m + 1);
            let d = st.dialogs.get_mut(&peer).expect("comprobado");
            d.pinned_order = if pinned { d.pinned_order.or(Some(next)) } else { None };
            d.clone()
        };
        self.inner.emit(BackendEvent::DialogUpserted(d));
        Ok(())
    }

    async fn set_archived(&self, peer: PeerRef, archived: bool) -> BackendResult<()> {
        self.inner.gate().await?;
        let d = {
            let mut st = self.inner.lock();
            st.ensure_peer(peer)?;
            let d = st.dialogs.get_mut(&peer).expect("comprobado");
            d.archived = archived;
            if archived {
                d.pinned_order = None;
            }
            d.clone()
        };
        self.inner.emit(BackendEvent::DialogUpserted(d));
        Ok(())
    }

    async fn set_muted(&self, peer: PeerRef, muted: bool) -> BackendResult<()> {
        self.inner.gate().await?;
        let d = {
            let mut st = self.inner.lock();
            st.ensure_peer(peer)?;
            let d = st.dialogs.get_mut(&peer).expect("comprobado");
            d.muted = muted;
            d.clone()
        };
        self.inner.emit(BackendEvent::DialogUpserted(d));
        Ok(())
    }

    async fn save_draft(&self, peer: PeerRef, draft: Option<Draft>) -> BackendResult<()> {
        self.inner.gate().await?;
        let d = {
            let mut st = self.inner.lock();
            st.ensure_peer(peer)?;
            let d = st.dialogs.get_mut(&peer).expect("comprobado");
            d.draft = draft.filter(|d| !d.text.is_empty());
            d.clone()
        };
        self.inner.emit(BackendEvent::DialogUpserted(d));
        Ok(())
    }
}

#[async_trait]
impl MessagesApi for MockBackend {
    async fn history(&self, peer: PeerRef, anchor: Anchor, limit: usize) -> BackendResult<Vec<Message>> {
        self.inner.gate().await?;
        let st = self.inner.lock();
        st.ensure_peer(peer)?;
        let list: &[Message] = st.messages.get(&peer).map_or(&[], Vec::as_slice);
        // Índice del primer mensaje con id >= x.
        let lower = |x: MsgId| list.partition_point(|m| m.id < x);
        let (start, end) = match anchor {
            Anchor::Latest => (list.len().saturating_sub(limit), list.len()),
            Anchor::Before(id) => {
                let e = lower(id);
                (e.saturating_sub(limit), e)
            }
            Anchor::After(id) => {
                let s = list.partition_point(|m| m.id <= id);
                (s, (s + limit).min(list.len()))
            }
            Anchor::Around(id) => {
                let c = lower(id);
                let s = c.saturating_sub(limit / 2);
                (s, (s + limit).min(list.len()))
            }
        };
        Ok(list[start..end].to_vec())
    }

    async fn send_text(&self, peer: PeerRef, msg: OutgoingText) -> BackendResult<Message> {
        self.inner.gate().await?;
        if msg.text.trim().is_empty() {
            return Err(BackendError::InvalidInput("text"));
        }
        let (m, d) = {
            let mut st = self.inner.lock();
            st.ensure_writable(peer)?;
            if let Some(prev) = st.messages.get(&peer).and_then(|l| l.iter().rev().find(|m| m.random_id == Some(msg.random_id))) {
                return Ok(prev.clone());
            }
            if let Some(r) = msg.reply_to {
                st.find_mut(peer, r)?;
            }
            let me = st.me.id;
            let mut m = st.blank_message(peer, true, Some(me));
            m.text = msg.text;
            m.entities = msg.entities;
            m.reply_to = msg.reply_to;
            m.random_id = Some(msg.random_id);
            st.append(m)
        };
        self.inner.emit(BackendEvent::MessageSent { random_id: msg.random_id, msg: m.clone() });
        self.inner.emit(BackendEvent::DialogUpserted(d));
        Ok(m)
    }

    async fn edit_message(&self, peer: PeerRef, id: MsgId, text: String, entities: Vec<TextEntity>) -> BackendResult<Message> {
        self.inner.gate().await?;
        if text.trim().is_empty() {
            return Err(BackendError::InvalidInput("text"));
        }
        let (m, d) = {
            let mut st = self.inner.lock();
            st.ensure_peer(peer)?;
            let now = st.tick();
            let m = st.find_mut(peer, id)?;
            if !m.outgoing {
                return Err(BackendError::Forbidden("solo puedes editar tus mensajes".into()));
            }
            m.text = text;
            m.entities = entities;
            m.edit_date = Some(now);
            let m = m.clone();
            let is_last = st.messages[&peer].last().is_some_and(|l| l.id == id);
            let d = if is_last { Some(st.refresh_last(peer)) } else { None };
            (m, d)
        };
        self.inner.emit(BackendEvent::MessageEdited(m.clone()));
        if let Some(d) = d {
            self.inner.emit(BackendEvent::DialogUpserted(d));
        }
        Ok(m)
    }

    async fn delete_messages(&self, peer: PeerRef, ids: Vec<MsgId>, _revoke: bool) -> BackendResult<()> {
        self.inner.gate().await?;
        let (removed, d) = {
            let mut st = self.inner.lock();
            st.ensure_peer(peer)?;
            let list = st.messages.get_mut(&peer).ok_or(BackendError::NotFound)?;
            let removed: Vec<MsgId> = list.iter().filter(|m| ids.contains(&m.id)).map(|m| m.id).collect();
            list.retain(|m| !ids.contains(&m.id));
            (removed, st.refresh_last(peer))
        };
        if !removed.is_empty() {
            self.inner.emit(BackendEvent::MessagesDeleted { peer, ids: removed });
            self.inner.emit(BackendEvent::DialogUpserted(d));
        }
        Ok(())
    }

    async fn forward_messages(&self, from: PeerRef, ids: Vec<MsgId>, to: Vec<PeerRef>, hide_sender: bool) -> BackendResult<()> {
        self.inner.gate().await?;
        let mut events = Vec::new();
        {
            let mut st = self.inner.lock();
            st.ensure_peer(from)?;
            for dest in &to {
                st.ensure_writable(*dest)?;
            }
            let originals: Vec<Message> = st
                .messages
                .get(&from)
                .map(|l| l.iter().filter(|m| ids.contains(&m.id)).cloned().collect())
                .unwrap_or_default();
            if originals.is_empty() {
                return Err(BackendError::NotFound);
            }
            let me = st.me.id;
            for dest in to {
                for o in &originals {
                    let mut m = st.blank_message(dest, true, Some(me));
                    m.text = o.text.clone();
                    m.entities = o.entities.clone();
                    m.media = o.media.clone();
                    m.sticker = o.sticker.clone();
                    if !hide_sender {
                        let name = o.sender.and_then(|s| st.users.get(&s)).map_or_else(|| st.peer_title(from).unwrap_or_default(), User::display_name);
                        m.forwarded_from = Some(name);
                    }
                    let (m, d) = st.append(m);
                    events.push(BackendEvent::MessageNew(m));
                    events.push(BackendEvent::DialogUpserted(d));
                }
            }
        }
        events.into_iter().for_each(|e| self.inner.emit(e));
        Ok(())
    }

    async fn send_reaction(&self, peer: PeerRef, id: MsgId, emoji: Option<String>) -> BackendResult<()> {
        self.inner.gate().await?;
        let reactions = {
            let mut st = self.inner.lock();
            st.ensure_peer(peer)?;
            let m = st.find_mut(peer, id)?;
            if let Some(i) = m.reactions.iter().position(|r| r.mine) {
                m.reactions[i].count -= 1;
                m.reactions[i].mine = false;
                if m.reactions[i].count == 0 {
                    m.reactions.remove(i);
                }
            }
            if let Some(e) = emoji {
                match m.reactions.iter_mut().find(|r| r.emoji == e) {
                    Some(r) => {
                        r.count += 1;
                        r.mine = true;
                    }
                    None => m.reactions.push(ReactionCount { emoji: e, count: 1, mine: true }),
                }
            }
            m.reactions.clone()
        };
        self.inner.emit(BackendEvent::Reactions { peer, msg_id: id, reactions });
        Ok(())
    }

    async fn mark_read(&self, peer: PeerRef, max_id: MsgId) -> BackendResult<()> {
        self.inner.gate().await?;
        let d = {
            let mut st = self.inner.lock();
            st.ensure_peer(peer)?;
            let unread = st
                .messages
                .get(&peer)
                .map_or(0, |l| l.iter().filter(|m| !m.outgoing && m.id > max_id).count() as u32);
            let d = st.dialogs.get_mut(&peer).expect("comprobado");
            d.unread_count = d.unread_count.min(unread);
            d.clone()
        };
        self.inner.emit(BackendEvent::ReadInbox { peer, max_id });
        self.inner.emit(BackendEvent::DialogUpserted(d));
        Ok(())
    }

    async fn set_typing(&self, peer: PeerRef, _typing: bool) -> BackendResult<()> {
        self.inner.gate().await?;
        self.inner.lock().ensure_peer(peer)
    }
}

#[async_trait]
impl MediaApi for MockBackend {
    async fn send_media(&self, peer: PeerRef, media: OutgoingMedia) -> BackendResult<Vec<Message>> {
        self.inner.gate().await?;
        if media.items.is_empty() || media.items.len() != media.random_ids.len() {
            return Err(BackendError::InvalidInput("items"));
        }
        let mut out = Vec::new();
        let mut events = Vec::new();
        {
            let mut st = self.inner.lock();
            st.ensure_writable(peer)?;
            let me = st.me.id;
            for (i, (item, rid)) in media.items.iter().zip(&media.random_ids).enumerate() {
                let mut m = st.blank_message(peer, true, Some(me));
                m.media = Some(Media {
                    id: 90_000 + st.clock + i as i64,
                    kind: item.kind,
                    file_name: Some(item.file_name.clone()),
                    size_bytes: item.size_bytes,
                    width: item.width,
                    height: item.height,
                    duration_secs: None,
                    local_path: Some(item.path.clone()),
                });
                if i == 0 {
                    m.text = media.caption.clone();
                    m.reply_to = media.reply_to;
                }
                m.random_id = Some(*rid);
                let (m, d) = st.append(m);
                events.push(BackendEvent::MessageSent { random_id: *rid, msg: m.clone() });
                events.push(BackendEvent::DialogUpserted(d));
                out.push(m);
            }
        }
        events.into_iter().for_each(|e| self.inner.emit(e));
        Ok(out)
    }
}

#[async_trait]
impl StickersApi for MockBackend {
    async fn installed_sticker_sets(&self) -> BackendResult<Vec<StickerSet>> {
        self.inner.gate().await?;
        Ok(self.inner.lock().sets.clone())
    }

    async fn send_sticker(&self, peer: PeerRef, sticker: StickerId, reply_to: Option<MsgId>, random_id: RandomId) -> BackendResult<Message> {
        self.inner.gate().await?;
        let (m, d) = {
            let mut st = self.inner.lock();
            st.ensure_writable(peer)?;
            let s = st.sets.iter().flat_map(|s| s.stickers.iter()).find(|s| s.id == sticker).cloned().ok_or(BackendError::NotFound)?;
            let me = st.me.id;
            let mut m = st.blank_message(peer, true, Some(me));
            m.sticker = Some(s);
            m.reply_to = reply_to;
            m.random_id = Some(random_id);
            st.append(m)
        };
        self.inner.emit(BackendEvent::MessageSent { random_id, msg: m.clone() });
        self.inner.emit(BackendEvent::DialogUpserted(d));
        Ok(m)
    }
}

#[async_trait]
impl SecretChatsApi for MockBackend {
    async fn secret_chats(&self) -> BackendResult<Vec<SecretChat>> {
        self.inner.gate().await?;
        Ok(self.inner.lock().secrets.values().cloned().collect())
    }

    async fn request_secret_chat(&self, user: UserId) -> BackendResult<SecretChat> {
        self.inner.gate().await?;
        let (chat, d) = {
            let mut st = self.inner.lock();
            let title = st.users.get(&user).map(User::display_name).ok_or(BackendError::NotFound)?;
            let id = SecretChatId(st.next_secret);
            st.next_secret += 1;
            let chat = SecretChat { id, user, state: SecretChatState::Waiting, outgoing: true, ttl_secs: None };
            st.secrets.insert(id, chat.clone());
            let peer = PeerRef::Secret(id);
            let activity = st.tick();
            let d = Dialog {
                peer,
                title,
                last_message: None,
                unread_count: 0,
                unread_mentions: 0,
                pinned_order: None,
                archived: false,
                muted: false,
                draft: None,
                activity,
                read_outbox_max_id: MsgId(0),
            };
            st.dialogs.insert(peer, d.clone());
            st.messages.insert(peer, Vec::new());
            (chat, d)
        };
        self.inner.emit(BackendEvent::SecretChat(chat.clone()));
        self.inner.emit(BackendEvent::DialogUpserted(d));
        // La otra parte acepta tras un breve retraso.
        let inner = Arc::clone(&self.inner);
        let delay = inner.cfg.secret_accept_delay;
        let id = chat.id;
        tokio::spawn(async move {
            tokio::time::sleep(delay).await;
            let updated = {
                let mut st = inner.lock();
                match st.secrets.get_mut(&id) {
                    Some(c) if c.state == SecretChatState::Waiting => {
                        c.state = SecretChatState::Accepted;
                        Some(c.clone())
                    }
                    _ => None,
                }
            };
            if let Some(c) = updated {
                inner.emit(BackendEvent::SecretChat(c));
            }
        });
        Ok(chat)
    }

    async fn accept_secret_chat(&self, id: SecretChatId) -> BackendResult<SecretChat> {
        self.inner.gate().await?;
        let chat = {
            let mut st = self.inner.lock();
            let c = st.secrets.get_mut(&id).ok_or(BackendError::NotFound)?;
            if c.state == SecretChatState::Discarded {
                return Err(BackendError::Forbidden("el chat secreto fue descartado".into()));
            }
            c.state = SecretChatState::Accepted;
            c.clone()
        };
        self.inner.emit(BackendEvent::SecretChat(chat.clone()));
        Ok(chat)
    }

    async fn discard_secret_chat(&self, id: SecretChatId) -> BackendResult<()> {
        self.inner.gate().await?;
        let chat = {
            let mut st = self.inner.lock();
            let c = st.secrets.get_mut(&id).ok_or(BackendError::NotFound)?;
            c.state = SecretChatState::Discarded;
            let c = c.clone();
            st.messages.insert(PeerRef::Secret(id), Vec::new());
            c
        };
        let d = self.inner.lock().refresh_last(PeerRef::Secret(id));
        self.inner.emit(BackendEvent::SecretChat(chat));
        self.inner.emit(BackendEvent::DialogUpserted(d));
        Ok(())
    }

    async fn set_secret_ttl(&self, id: SecretChatId, ttl_secs: Option<u32>) -> BackendResult<()> {
        self.inner.gate().await?;
        let chat = {
            let mut st = self.inner.lock();
            let c = st.secrets.get_mut(&id).ok_or(BackendError::NotFound)?;
            c.ttl_secs = ttl_secs;
            c.clone()
        };
        self.inner.emit(BackendEvent::SecretChat(chat));
        Ok(())
    }

    async fn secret_key_visual(&self, id: SecretChatId) -> BackendResult<KeyVisual> {
        self.inner.gate().await?;
        if !self.inner.lock().secrets.contains_key(&id) {
            return Err(BackendError::NotFound);
        }
        // Ilustrativa: determinista por id, sin relación con ninguna clave real.
        let mut r = Lcg::new(id.0 as u64 + 777);
        const EMOJI: &[&str] = &["🌵", "🦊", "🍋", "🎈", "🔔", "🐢", "🌙", "🧭", "🍄", "🪁", "🎲", "🦉"];
        let emoji = (0..4).map(|_| EMOJI[r.below(EMOJI.len())].to_string()).collect();
        let fingerprint = (0..8).map(|_| format!("{:04x}", r.next_u32() & 0xffff)).collect::<Vec<_>>().join(" ");
        Ok(KeyVisual { fingerprint, emoji })
    }
}

impl ChatBackend for MockBackend {
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

#[cfg(test)]
mod tests;
