//! Chats secretos sobre la red con TL crudo, enrutados a `tc-secret` (sans-IO).
//!
//! > **EXPERIMENTAL — SIN AUDITORÍA.** La criptografía es la de `tc-secret` (ver su
//! > README y ADR-0004). Esta capa solo se ha probado contra un servidor falso que
//! > reenvía bytes entre dos instancias de este mismo backend: **no hay ninguna prueba
//! > de interoperabilidad con clientes oficiales de Telegram**.
//!
//! Llamadas: `messages.getDhConfig`, `requestEncryption`, `acceptEncryption`,
//! `discardEncryption`, `sendEncrypted`, `sendEncryptedService`, `receivedQueue`,
//! `readEncryptedHistory` y `setEncryptedTyping`. Updates: `updateEncryption`,
//! `updateNewEncryptedMessage`, `updateEncryptedChatTyping` y
//! `updateEncryptedMessagesRead`.
//!
//! Estado: cada chat (metadatos + instantánea de `tc-secret` con claves, seq_no, capa,
//! TTL y re-keying) se guarda en un [`SecretStateStore`] inyectable **antes** de enviar
//! nada (así un reinicio no reutiliza números de secuencia). Aquí solo hay
//! [`MemorySecretStateStore`]; el almacén real (`SealedSecretStateStore` de tc-app: AES-256-GCM
//! con clave del llavero, blob en SQLite de tc-store) se inyecta con `set_secret_store`.
//!
//! Limitaciones (TODO):
//! - Solo texto. Los medios cifrados (`sendEncryptedFile`, `decryptedMessageMedia*`,
//!   clave/iv por archivo) no están: un mensaje con medio del otro extremo no se puede
//!   descifrar (constructor no soportado) y su hueco de secuencia acabaría en `Resend`.
//! - El historial vive en memoria (los mensajes los persiste otra capa); los
//!   temporizadores de autodestrucción en curso no sobreviven a un reinicio.
//! - `receivedQueue` solo se confirma con el `qts` de los updates en vivo; los que llegan
//!   por `getDifference` traen `qts = 0` desde grammers.
use std::collections::{BTreeMap, HashMap, HashSet};
use std::sync::{Arc, Mutex, Weak};
use std::time::Duration;

use grammers_tl_types as tl;
use tc_core::*;
use tc_secret::tl::{Action, Entity, EntityKind as SecretEntityKind, PlainMessage, TypingAction};
use tc_secret::{self as sec, DhParams, OsEntropy, SessionSnapshot, StateKind};
use zeroize::Zeroizing;

use crate::convert::{utf16_starts, utf16_to_chars};
use crate::session::StoreError;
use crate::{Inner, now_secs};

/// Mensajes por chat que se guardan en memoria para `history`.
const LOG_CAP: usize = 2000;
/// Mensajes cifrados que se guardan mientras la clave aún no está lista.
const EARLY_CAP: usize = 100;
const NOT_READY: &str = "la clave del chat secreto aún no está establecida";

// ---------------------------------------------------------------------------
// Persistencia inyectable

/// Registro persistente de un chat secreto.
#[derive(Clone)]
pub struct StoredSecretChat {
    pub id: SecretChatId,
    pub access_hash: i64,
    /// El otro extremo.
    pub user: UserId,
    /// Lo inicié yo.
    pub outgoing: bool,
    pub date: i64,
    pub state: SecretChatState,
    pub ttl_secs: Option<u32>,
    /// Siguiente id local de mensaje (los chats secretos no tienen ids del servidor).
    pub next_msg_id: i32,
    /// Instantánea opaca de `tc_secret::SessionSnapshot`. **Contiene la clave del chat y
    /// textos en claro recientes**: guardarla solo cifrada o en el keyring. `None` si el
    /// chat está descartado (ya no hay claves).
    pub session: Option<Zeroizing<Vec<u8>>>,
}

impl std::fmt::Debug for StoredSecretChat {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("StoredSecretChat")
            .field("id", &self.id)
            .field("user", &self.user)
            .field("state", &self.state)
            .field("session", &self.session.as_ref().map(|s| s.len()))
            .finish_non_exhaustive()
    }
}

/// Dónde se guarda el estado de los chats secretos. Síncrono (como `SessionStore`); el
/// backend lo llama desde `spawn_blocking`.
pub trait SecretStateStore: Send + Sync + 'static {
    fn load_all(&self) -> Result<Vec<StoredSecretChat>, StoreError>;
    /// Inserta o sustituye por `id`.
    fn save(&self, chat: &StoredSecretChat) -> Result<(), StoreError>;
    fn remove(&self, id: SecretChatId) -> Result<(), StoreError>;
}

/// Almacén en memoria (pruebas y valor por defecto; se pierde al cerrar).
#[derive(Default)]
pub struct MemorySecretStateStore(Mutex<BTreeMap<i32, StoredSecretChat>>);

impl MemorySecretStateStore {
    pub fn get(&self, id: SecretChatId) -> Option<StoredSecretChat> {
        self.0.lock().unwrap_or_else(|e| e.into_inner()).get(&id.0).cloned()
    }
    pub fn len(&self) -> usize {
        self.0.lock().unwrap_or_else(|e| e.into_inner()).len()
    }
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

impl SecretStateStore for MemorySecretStateStore {
    fn load_all(&self) -> Result<Vec<StoredSecretChat>, StoreError> {
        Ok(self.0.lock().unwrap_or_else(|e| e.into_inner()).values().cloned().collect())
    }
    fn save(&self, chat: &StoredSecretChat) -> Result<(), StoreError> {
        self.0.lock().unwrap_or_else(|e| e.into_inner()).insert(chat.id.0, chat.clone());
        Ok(())
    }
    fn remove(&self, id: SecretChatId) -> Result<(), StoreError> {
        self.0.lock().unwrap_or_else(|e| e.into_inner()).remove(&id.0);
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Estado en memoria

pub(crate) struct Chat {
    id: i32,
    access_hash: i64,
    user: UserId,
    outgoing: bool,
    date: i64,
    state: SecretChatState,
    ttl: Option<u32>,
    pub(crate) session: Option<sec::SecretChat>,
    log: Vec<Message>,
    next_id: i32,
    unread: u32,
    read_outbox: MsgId,
    /// Bytes cifrados de envíos de texto fallidos, para reintentar con el mismo `random_id`.
    unsent: HashMap<i64, Vec<u8>>,
    /// Mensajes recibidos antes de tener la clave (llegan antes que `encryptedChat`).
    early: Vec<(Vec<u8>, i64)>,
    timer: sec::SelfDestructTimer,
    read_sent: HashSet<i64>,
}

impl Chat {
    fn new(id: i32, access_hash: i64, user: UserId, outgoing: bool, date: i64, session: Option<sec::SecretChat>) -> Self {
        Self {
            id,
            access_hash,
            user,
            outgoing,
            date,
            state: SecretChatState::Waiting,
            ttl: None,
            session,
            log: Vec::new(),
            next_id: 1,
            unread: 0,
            read_outbox: MsgId(0),
            unsent: HashMap::new(),
            early: Vec::new(),
            timer: sec::SelfDestructTimer::new(),
            read_sent: HashSet::new(),
        }
    }

    fn peer(&self) -> PeerRef {
        PeerRef::Secret(SecretChatId(self.id))
    }

    fn model(&self) -> SecretChat {
        SecretChat { id: SecretChatId(self.id), user: self.user, state: self.state, outgoing: self.outgoing, ttl_secs: self.ttl }
    }

    fn input(&self) -> tl::enums::InputEncryptedChat {
        tl::types::InputEncryptedChat { chat_id: self.id, access_hash: self.access_hash }.into()
    }

    fn ready(&mut self) -> BackendResult<&mut sec::SecretChat> {
        match self.session.as_mut() {
            Some(s) if s.state_kind() == StateKind::Ready => Ok(s),
            _ if self.state == SecretChatState::Discarded => Err(BackendError::Forbidden("chat secreto cerrado".into())),
            _ => Err(BackendError::Forbidden(NOT_READY.into())),
        }
    }

    fn stored(&self) -> StoredSecretChat {
        StoredSecretChat {
            id: SecretChatId(self.id),
            access_hash: self.access_hash,
            user: self.user,
            outgoing: self.outgoing,
            date: self.date,
            state: self.state,
            ttl_secs: self.ttl,
            next_msg_id: self.next_id,
            session: self
                .session
                .as_ref()
                .filter(|s| s.state_kind() != StateKind::Discarded)
                .map(|s| Zeroizing::new(s.snapshot().as_bytes().to_vec())),
        }
    }

    fn alloc_id(&mut self) -> MsgId {
        let id = MsgId(self.next_id);
        self.next_id = self.next_id.saturating_add(1);
        id
    }

    fn push(&mut self, m: Message) {
        if self.log.len() >= LOG_CAP {
            self.log.remove(0);
        }
        self.log.push(m);
    }

    fn random_of(&self, id: MsgId) -> Option<i64> {
        self.log.iter().find(|m| m.id == id).and_then(|m| m.random_id).map(|r| r.0)
    }

    fn by_random(&self, rid: i64) -> Option<MsgId> {
        self.log.iter().find(|m| m.random_id == Some(RandomId(rid))).map(|m| m.id)
    }

    /// Quita del historial los mensajes con estos `random_id`; devuelve sus ids.
    fn remove_random(&mut self, rids: &[i64]) -> Vec<MsgId> {
        let set: HashSet<i64> = rids.iter().copied().collect();
        let mut ids = Vec::new();
        self.log.retain(|m| {
            let hit = m.random_id.is_some_and(|r| set.contains(&r.0));
            if hit {
                ids.push(m.id);
            }
            !hit
        });
        for r in rids {
            self.timer.cancel(*r);
            self.read_sent.remove(r);
            self.unsent.remove(r);
        }
        ids
    }
}

#[derive(Default)]
pub(crate) struct Secrets {
    loaded: bool,
    dh: Option<DhParams>,
    pub(crate) chats: BTreeMap<i32, Chat>,
    acked_qts: i32,
}

pub(crate) struct SecretRuntime {
    pub(crate) state: tokio::sync::Mutex<Secrets>,
    store: Mutex<Arc<dyn SecretStateStore>>,
}

impl SecretRuntime {
    pub(crate) fn new(store: Arc<dyn SecretStateStore>) -> Self {
        Self { state: tokio::sync::Mutex::new(Secrets::default()), store: Mutex::new(store) }
    }
    fn store(&self) -> Arc<dyn SecretStateStore> {
        Arc::clone(&self.store.lock().unwrap_or_else(|e| e.into_inner()))
    }
    /// Cambia el almacén y fuerza a recargar desde él.
    pub(crate) async fn replace_store(&self, store: Arc<dyn SecretStateStore>) {
        let mut s = self.state.lock().await;
        *self.store.lock().unwrap_or_else(|e| e.into_inner()) = store;
        *s = Secrets::default();
    }
}

/// Resultado de procesar un `Output` de tc-secret.
#[derive(Default)]
struct Processed {
    events: Vec<BackendEvent>,
    /// `random_id` → fecha del servidor de cada `sendEncrypted*` confirmado.
    sent: HashMap<i64, i32>,
    /// Primer error de red (si lo hubo).
    error: Option<BackendError>,
}

fn crypto(e: sec::SecretError) -> BackendError {
    BackendError::Crypto(e.to_string())
}

pub(crate) fn is_secret_update(u: &tl::enums::Update) -> bool {
    use tl::enums::Update as U;
    matches!(u, U::Encryption(_) | U::NewEncryptedMessage(_) | U::EncryptedChatTyping(_) | U::EncryptedMessagesRead(_))
}

async fn validate_dh(g: i32, p: Vec<u8>, version: i32) -> BackendResult<DhParams> {
    tokio::task::spawn_blocking(move || DhParams::validate(g, &p, version, &mut OsEntropy))
        .await
        .map_err(|_| BackendError::Internal)?
        .map_err(crypto)
}

// ---- entidades (tc-core en caracteres ↔ capa secreta en UTF-16) ----

fn entities_to_secret(text: &str, ents: &[TextEntity]) -> Vec<Entity> {
    let starts = utf16_starts(text);
    let n = starts.len() - 1;
    ents.iter()
        .filter_map(|e| {
            let (s, end) = (e.offset as usize, (e.offset + e.length) as usize);
            if e.length == 0 || s >= n || end > n {
                return None;
            }
            let kind = match e.kind {
                EntityKind::Bold => SecretEntityKind::Bold,
                EntityKind::Italic => SecretEntityKind::Italic,
                EntityKind::Code => SecretEntityKind::Code,
                EntityKind::Pre => SecretEntityKind::Pre { language: String::new() },
                EntityKind::Strike => SecretEntityKind::Strike,
                EntityKind::Underline => SecretEntityKind::Underline,
                EntityKind::Spoiler => SecretEntityKind::Spoiler,
                EntityKind::Mention => SecretEntityKind::Mention,
                EntityKind::Url => SecretEntityKind::Url,
                EntityKind::Hashtag => SecretEntityKind::Hashtag,
            };
            Some(Entity { kind, offset: starts[s] as i32, length: (starts[end] - starts[s]) as i32 })
        })
        .collect()
}

fn entities_from_secret(text: &str, ents: &[Entity]) -> Vec<TextEntity> {
    let starts = utf16_starts(text);
    ents.iter()
        .filter_map(|e| {
            let kind = match &e.kind {
                SecretEntityKind::Bold => EntityKind::Bold,
                SecretEntityKind::Italic => EntityKind::Italic,
                SecretEntityKind::Code => EntityKind::Code,
                SecretEntityKind::Pre { .. } => EntityKind::Pre,
                SecretEntityKind::Strike => EntityKind::Strike,
                SecretEntityKind::Underline => EntityKind::Underline,
                SecretEntityKind::Spoiler => EntityKind::Spoiler,
                SecretEntityKind::Mention => EntityKind::Mention,
                SecretEntityKind::Url | SecretEntityKind::TextUrl { .. } | SecretEntityKind::Email => EntityKind::Url,
                SecretEntityKind::Hashtag => EntityKind::Hashtag,
                _ => return None,
            };
            let (offset, length) = utf16_to_chars(&starts, e.offset, e.length)?;
            Some(TextEntity { kind, offset, length })
        })
        .collect()
}

fn history_page(log: &[Message], anchor: Anchor, limit: usize) -> Vec<Message> {
    let limit = limit.max(1);
    let v: Vec<&Message> = match anchor {
        Anchor::Latest => log.iter().rev().take(limit).collect(),
        Anchor::Before(id) => log.iter().rev().filter(|m| m.id < id).take(limit).collect(),
        Anchor::After(id) => log.iter().filter(|m| m.id > id).take(limit).collect(),
        Anchor::Around(id) => {
            let half = limit / 2;
            let mut v: Vec<&Message> = log.iter().rev().filter(|m| m.id < id).take(half).collect();
            v.extend(log.iter().filter(|m| m.id >= id).take(limit - half));
            v
        }
    };
    let mut out: Vec<Message> = v.into_iter().cloned().collect();
    out.sort_by_key(|m| m.id);
    out
}

impl Inner {
    /// Bloquea el estado de los chats secretos (cargándolo del almacén la primera vez).
    async fn secrets(&self) -> BackendResult<tokio::sync::MutexGuard<'_, Secrets>> {
        let mut s = self.secret.state.lock().await;
        if !s.loaded {
            let store = self.secret.store();
            let recs = tokio::task::spawn_blocking(move || store.load_all())
                .await
                .map_err(|_| BackendError::Internal)?
                .map_err(|e| BackendError::Storage(e.to_string()))?;
            for rec in recs {
                let chat = self.restore_chat(&mut s, rec).await;
                s.chats.insert(chat.id, chat);
            }
            s.loaded = true;
        }
        Ok(s)
    }

    async fn restore_chat(&self, s: &mut Secrets, rec: StoredSecretChat) -> Chat {
        let mut chat = Chat::new(rec.id.0, rec.access_hash, rec.user, rec.outgoing, rec.date, None);
        chat.state = rec.state;
        chat.ttl = rec.ttl_secs;
        chat.next_id = rec.next_msg_id.max(1);
        let Some(bytes) = rec.session.filter(|_| rec.state != SecretChatState::Discarded) else {
            chat.state = SecretChatState::Discarded;
            return chat;
        };
        let restored = async {
            let snap = SessionSnapshot::from_bytes(&bytes).map_err(crypto)?;
            let dh = match &s.dh {
                Some(d) if d.g() == snap.dh_g() && d.p() == snap.dh_p() => d.clone(),
                _ => {
                    let d = validate_dh(snap.dh_g() as i32, snap.dh_p().to_vec(), snap.dh_version()).await?;
                    s.dh.get_or_insert_with(|| d.clone());
                    d
                }
            };
            sec::SecretChat::restore(dh, &snap).map_err(crypto)
        }
        .await;
        match restored {
            Ok(session) => chat.session = Some(session),
            Err(e) => {
                tracing::warn!(chat = rec.id.0, error = %e, "estado de chat secreto ilegible: se marca como cerrado");
                chat.state = SecretChatState::Discarded;
            }
        }
        chat
    }

    /// `messages.getDhConfig`: parámetros (validados y cacheados) y 256 bytes del servidor.
    async fn dh_config(&self, s: &mut Secrets) -> BackendResult<(DhParams, Vec<u8>)> {
        let version = s.dh.as_ref().map_or(0, DhParams::version);
        let r = self.rpc(&tl::functions::messages::GetDhConfig { version, random_length: 256 }).await?;
        match r {
            tl::enums::messages::DhConfig::NotModified(n) => Ok((s.dh.clone().ok_or(BackendError::Internal)?, n.random)),
            tl::enums::messages::DhConfig::Config(c) => {
                let dh = validate_dh(c.g, c.p, c.version).await?;
                s.dh = Some(dh.clone());
                Ok((dh, c.random))
            }
        }
    }

    async fn persist_secret(&self, c: &Chat) {
        let rec = c.stored();
        let store = self.secret.store();
        if !matches!(tokio::task::spawn_blocking(move || store.save(&rec)).await, Ok(Ok(()))) {
            tracing::warn!(chat = c.id, "no se pudo guardar el estado del chat secreto");
        }
    }

    fn secret_dialog(&self, c: &Chat) -> Dialog {
        let (title, photo) = {
            let cache = self.cache();
            (cache.peers.title(PeerRef::User(c.user), cache.me), cache.peers.photo(PeerRef::User(c.user), cache.me))
        };
        let last = c.log.last().cloned();
        Dialog {
            peer: c.peer(),
            title,
            activity: last.as_ref().map_or(Timestamp(c.date), |m| m.date),
            last_message: last,
            unread_count: c.unread,
            unread_mentions: 0,
            pinned_order: None,
            archived: false,
            muted: false,
            draft: None,
            read_outbox_max_id: c.read_outbox,
            photo,
        }
    }

    /// Usuario al otro lado de un chat secreto (su foto es la del chat).
    pub(crate) async fn secret_partner(&self, id: SecretChatId) -> BackendResult<UserId> {
        self.secrets().await?.chats.get(&id.0).map(|c| c.user).ok_or(BackendError::NotFound)
    }

    /// Traduce los eventos de tc-secret a eventos de la UI. Devuelve plazos de TTL nuevos.
    fn absorb(&self, c: &mut Chat, events: Vec<sec::Event>, date: i64, ev: &mut Vec<BackendEvent>) -> Vec<i64> {
        let mut deadlines = Vec::new();
        let peer = c.peer();
        for e in events {
            match e {
                sec::Event::Ready { .. } => {
                    c.state = SecretChatState::Accepted;
                    ev.push(BackendEvent::SecretChat(c.model()));
                }
                sec::Event::Discarded(reason) => {
                    tracing::info!(chat = c.id, ?reason, "chat secreto cerrado");
                    c.state = SecretChatState::Discarded;
                    c.early.clear();
                    ev.push(BackendEvent::SecretChat(c.model()));
                }
                sec::Event::Message(pm) => {
                    if pm.media.is_some() {
                        tracing::debug!(chat = c.id, "decryptedMessageMediaEmpty ignorado");
                    }
                    let reply_to = pm.reply_to_random_id.and_then(|r| c.by_random(r));
                    let m = Message {
                        id: c.alloc_id(),
                        peer,
                        sender: Some(c.user),
                        date: Timestamp(date),
                        edit_date: None,
                        outgoing: false,
                        entities: entities_from_secret(&pm.message, &pm.entities),
                        text: pm.message,
                        reply_to,
                        forwarded_from: None,
                        media: None,
                        sticker: None,
                        reactions: vec![],
                        random_id: Some(RandomId(pm.random_id)),
                        state: SendState::Sent,
                        ttl_secs: u32::try_from(pm.ttl).ok().filter(|t| *t > 0),
                    };
                    c.push(m.clone());
                    c.unread += 1;
                    ev.push(BackendEvent::MessageNew(m));
                    ev.push(BackendEvent::DialogUpserted(self.secret_dialog(c)));
                }
                sec::Event::Read(rids) => {
                    // El otro leyó mensajes nuestros con TTL: arranca su cuenta atrás.
                    let now = now_secs();
                    for r in rids {
                        let ttl = c.log.iter().find(|m| m.outgoing && m.random_id == Some(RandomId(r))).and_then(|m| m.ttl_secs);
                        if let Some(at) = ttl.and_then(|t| c.timer.start(r, t as i32, now)) {
                            deadlines.push(at);
                        }
                    }
                }
                sec::Event::Deleted(rids) => {
                    let ids = c.remove_random(&rids);
                    if !ids.is_empty() {
                        ev.push(BackendEvent::MessagesDeleted { peer, ids });
                        ev.push(BackendEvent::DialogUpserted(self.secret_dialog(c)));
                    }
                }
                sec::Event::HistoryFlushed => {
                    let ids: Vec<MsgId> = c.log.drain(..).map(|m| m.id).collect();
                    c.unread = 0;
                    if !ids.is_empty() {
                        ev.push(BackendEvent::MessagesDeleted { peer, ids });
                        ev.push(BackendEvent::DialogUpserted(self.secret_dialog(c)));
                    }
                }
                sec::Event::TtlChanged(t) => {
                    c.ttl = u32::try_from(t).ok().filter(|t| *t > 0);
                    ev.push(BackendEvent::SecretChat(c.model()));
                }
                sec::Event::Typing(t) => {
                    ev.push(BackendEvent::Typing { peer, user: c.user, typing: t == TypingAction::Typing });
                }
                other => tracing::trace!(chat = c.id, event = ?other, "evento de chat secreto"),
            }
        }
        deadlines
    }

    async fn transmit(&self, c: &mut Chat, o: sec::Outbound) -> BackendResult<Option<(i64, i32)>> {
        use tl::functions::messages as m;
        match o {
            sec::Outbound::RequestEncryption { .. } => Ok(None),
            sec::Outbound::AcceptEncryption { g_b, key_fingerprint } => {
                self.rpc(&m::AcceptEncryption { peer: c.input(), g_b: g_b.to_vec(), key_fingerprint }).await?;
                Ok(None)
            }
            sec::Outbound::DiscardEncryption => {
                self.rpc(&m::DiscardEncryption { delete_history: false, chat_id: c.id }).await?;
                Ok(None)
            }
            sec::Outbound::SendEncrypted { random_id, data, service } => {
                let r = if service {
                    self.rpc(&m::SendEncryptedService { peer: c.input(), random_id, data: data.clone() }).await
                } else {
                    self.rpc(&m::SendEncrypted { silent: false, peer: c.input(), random_id, data: data.clone() }).await
                };
                match r {
                    Ok(sent) => {
                        c.unsent.remove(&random_id);
                        Ok(Some((random_id, sent.date())))
                    }
                    Err(e) => {
                        // Los de servicio se recuperan con `Resend` (seguimos en el registro).
                        if !service {
                            c.unsent.insert(random_id, data);
                        }
                        Err(e)
                    }
                }
            }
        }
    }

    /// Aplica un `Output`: eventos → estado, guarda, y envía lo pendiente en orden.
    async fn process(self: &Arc<Self>, c: &mut Chat, out: sec::Output, date: i64) -> Processed {
        let mut p = Processed::default();
        let deadlines = self.absorb(c, out.events, date, &mut p.events);
        self.persist_secret(c).await;
        for o in out.outbound {
            match self.transmit(c, o).await {
                Ok(Some((rid, d))) => {
                    p.sent.insert(rid, d);
                }
                Ok(None) => {}
                Err(e) => {
                    tracing::warn!(chat = c.id, error = %e, "envío de chat secreto fallido");
                    p.error.get_or_insert(e);
                }
            }
        }
        for at in deadlines {
            self.schedule_expiry(c.id, at);
        }
        p
    }

    /// Re-keying (PFS) si toca: más de 100 mensajes o una semana con la misma clave.
    async fn maybe_rekey(self: &Arc<Self>, c: &mut Chat) -> Vec<BackendEvent> {
        let Some(sess) = c.session.as_mut().filter(|s| s.should_rekey(now_secs())) else { return vec![] };
        match sess.start_rekey(&mut OsEntropy) {
            Ok(out) => self.process(c, out, now_secs()).await.events,
            Err(e) => {
                tracing::warn!(chat = c.id, error = %e, "no se pudo iniciar el re-keying");
                vec![]
            }
        }
    }

    fn schedule_expiry(self: &Arc<Self>, chat: i32, at: i64) {
        let weak: Weak<Inner> = Arc::downgrade(self);
        let wait = Duration::from_secs((at - now_secs()).max(0) as u64);
        tokio::spawn(async move {
            tokio::time::sleep(wait).await;
            if let Some(inner) = weak.upgrade() {
                inner.secret_expire(chat).await;
            }
        });
    }

    async fn secret_expire(self: &Arc<Self>, chat: i32) {
        let Ok(mut s) = self.secrets().await else { return };
        let Some(c) = s.chats.get_mut(&chat) else { return };
        let rids = c.timer.expire(now_secs());
        let ids = c.remove_random(&rids);
        if !ids.is_empty() {
            let d = self.secret_dialog(c);
            let peer = c.peer();
            drop(s);
            self.emit(BackendEvent::MessagesDeleted { peer, ids });
            self.emit(BackendEvent::DialogUpserted(d));
        }
    }

    // ---- API ----

    pub(crate) async fn secret_list(&self) -> BackendResult<Vec<SecretChat>> {
        let s = self.secrets().await?;
        Ok(s.chats.values().map(Chat::model).collect())
    }

    pub(crate) async fn secret_request(self: &Arc<Self>, user: UserId) -> BackendResult<SecretChat> {
        self.ensure_logged_in()?;
        let input_user = {
            let c = self.cache();
            c.peers.input_user(user, c.me, &self.net.session)?
        };
        let mut s = self.secrets().await?;
        let (dh, random) = self.dh_config(&mut s).await?;
        let (session, out) = sec::SecretChat::request(dh, &mut OsEntropy, &random).map_err(crypto)?;
        let Some(sec::Outbound::RequestEncryption { g_a }) = out.outbound.first() else { return Err(BackendError::Internal) };
        let req = tl::functions::messages::RequestEncryption { user_id: input_user, random_id: crate::backend::random_i64() as i32, g_a: g_a.to_vec() };
        let (id, access_hash, date) = match self.rpc(&req).await? {
            tl::enums::EncryptedChat::Waiting(w) => (w.id, w.access_hash, w.date),
            tl::enums::EncryptedChat::Chat(c) => (c.id, c.access_hash, c.date),
            tl::enums::EncryptedChat::Requested(r) => (r.id, r.access_hash, r.date),
            _ => return Err(BackendError::Forbidden("el servidor rechazó el chat secreto".into())),
        };
        let chat = Chat::new(id, access_hash, user, true, date.into(), Some(session));
        self.persist_secret(&chat).await;
        let (m, d) = (chat.model(), self.secret_dialog(&chat));
        s.chats.insert(id, chat);
        drop(s);
        self.emit(BackendEvent::SecretChat(m.clone()));
        self.emit(BackendEvent::DialogUpserted(d));
        Ok(m)
    }

    pub(crate) async fn secret_accept(self: &Arc<Self>, id: SecretChatId) -> BackendResult<SecretChat> {
        let mut s = self.secrets().await?;
        let c = s.chats.get(&id.0).ok_or(BackendError::NotFound)?;
        match c.session.as_ref().map(sec::SecretChat::state_kind) {
            Some(StateKind::Incoming) => {}
            Some(StateKind::Ready) => return Ok(c.model()),
            _ => return Err(BackendError::Forbidden("no hay ninguna solicitud pendiente".into())),
        }
        let (dh, random) = self.dh_config(&mut s).await?;
        let c = s.chats.get_mut(&id.0).ok_or(BackendError::NotFound)?;
        let sess = c.session.as_mut().ok_or(BackendError::Internal)?;
        let before = sess.snapshot();
        let out = sess.accept(&mut OsEntropy, &random, now_secs()).map_err(crypto)?;
        let mut outbound = out.outbound.into_iter();
        let accept = outbound.next().ok_or(BackendError::Internal)?;
        c.state = SecretChatState::Accepted;
        self.persist_secret(c).await;
        if let Err(e) = self.transmit(c, accept).await {
            // El servidor no se enteró: se vuelve a la solicitud pendiente.
            c.session = Some(sec::SecretChat::restore(dh, &before).map_err(crypto)?);
            c.state = SecretChatState::Waiting;
            self.persist_secret(c).await;
            return Err(e);
        }
        let rest = sec::Output { outbound: outbound.collect(), events: out.events };
        let mut p = self.process(c, rest, now_secs()).await;
        p.events.push(BackendEvent::DialogUpserted(self.secret_dialog(c)));
        let m = c.model();
        drop(s);
        self.emit_all(p.events);
        Ok(m)
    }

    pub(crate) async fn secret_discard(self: &Arc<Self>, id: SecretChatId) -> BackendResult<()> {
        let mut s = self.secrets().await?;
        let c = s.chats.get_mut(&id.0).ok_or(BackendError::NotFound)?;
        if c.state == SecretChatState::Discarded {
            return Ok(());
        }
        let out = match c.session.as_mut() {
            Some(sess) => sess.discard(),
            None => sec::Output { outbound: vec![sec::Outbound::DiscardEncryption], events: vec![sec::Event::Discarded(sec::DiscardReason::Local)] },
        };
        let p = self.process(c, out, now_secs()).await;
        drop(s);
        self.emit_all(p.events);
        // Cerrado localmente aunque la red fallara (las claves ya se borraron).
        if let Some(e) = p.error {
            tracing::warn!(error = %e, "discardEncryption no llegó al servidor");
        }
        Ok(())
    }

    pub(crate) async fn secret_set_ttl(self: &Arc<Self>, id: SecretChatId, ttl: Option<u32>) -> BackendResult<()> {
        let ttl_i = i32::try_from(ttl.unwrap_or(0)).map_err(|_| BackendError::InvalidInput("ttl"))?;
        let mut s = self.secrets().await?;
        let c = s.chats.get_mut(&id.0).ok_or(BackendError::NotFound)?;
        let out = c.ready()?.set_ttl(ttl_i, &mut OsEntropy).map_err(crypto)?;
        c.ttl = ttl.filter(|t| *t > 0);
        let mut p = self.process(c, out, now_secs()).await;
        p.events.push(BackendEvent::SecretChat(c.model()));
        let mut rk = self.maybe_rekey(c).await;
        p.events.append(&mut rk);
        drop(s);
        self.emit_all(p.events);
        p.error.map_or(Ok(()), Err)
    }

    pub(crate) async fn secret_visual(&self, id: SecretChatId) -> BackendResult<KeyVisual> {
        let mut s = self.secrets().await?;
        let c = s.chats.get_mut(&id.0).ok_or(BackendError::NotFound)?;
        let v = c.ready()?.key_visualization().ok_or(BackendError::Internal)?;
        Ok(KeyVisual { fingerprint: v.hex(), emoji: v.emoji().iter().map(|e| (*e).to_string()).collect() })
    }

    pub(crate) async fn secret_send_text(self: &Arc<Self>, id: SecretChatId, msg: OutgoingText) -> BackendResult<Message> {
        let mut s = self.secrets().await?;
        let c = s.chats.get_mut(&id.0).ok_or(BackendError::NotFound)?;
        let rid = msg.random_id.0;
        // Idempotente por random_id: confirmado → se devuelve; fallido → mismos bytes otra vez.
        if let Some(prev) = c.log.iter().find(|m| m.outgoing && m.random_id == Some(msg.random_id)).cloned() {
            if prev.state == SendState::Sent {
                return Ok(prev);
            }
            let data = c.unsent.get(&rid).cloned().ok_or(BackendError::Internal)?;
            let date = self.transmit(c, sec::Outbound::SendEncrypted { random_id: rid, data, service: false }).await?.map_or(now_secs(), |(_, d)| d.into());
            return Ok(self.confirm_sent(c, rid, date));
        }
        let reply_to_random_id = msg.reply_to.and_then(|r| c.random_of(r));
        let sess = c.ready()?;
        let ttl = sess.ttl();
        let plain = PlainMessage {
            random_id: rid,
            ttl,
            entities: entities_to_secret(&msg.text, &msg.entities),
            message: msg.text.clone(),
            reply_to_random_id,
            ..PlainMessage::default()
        };
        let (rid, out) = sess.send_message(plain, &mut OsEntropy).map_err(crypto)?;
        let m = Message {
            id: c.alloc_id(),
            peer: c.peer(),
            sender: self.me(),
            date: Timestamp(now_secs()),
            edit_date: None,
            outgoing: true,
            text: msg.text,
            entities: msg.entities,
            reply_to: msg.reply_to,
            forwarded_from: None,
            media: None,
            sticker: None,
            reactions: vec![],
            random_id: Some(RandomId(rid)),
            state: SendState::Failed,
            ttl_secs: u32::try_from(ttl).ok().filter(|t| *t > 0),
        };
        c.push(m);
        let p = self.process(c, out, now_secs()).await;
        let result = match p.sent.get(&rid) {
            Some(d) => Ok(self.confirm_sent(c, rid, (*d).into())),
            None => Err(p.error.clone().unwrap_or(BackendError::Internal)),
        };
        let mut evs = p.events;
        evs.append(&mut self.maybe_rekey(c).await);
        drop(s);
        self.emit_all(evs);
        result
    }

    /// Marca enviado el mensaje `rid` y emite su confirmación.
    fn confirm_sent(&self, c: &mut Chat, rid: i64, date: i64) -> Message {
        let msg = c.log.iter_mut().find(|m| m.random_id == Some(RandomId(rid))).map(|m| {
            m.state = SendState::Sent;
            m.date = Timestamp(date);
            m.clone()
        });
        let msg = msg.expect("el mensaje se acaba de registrar");
        self.emit(BackendEvent::MessageSent { random_id: RandomId(rid), msg: msg.clone() });
        self.emit(BackendEvent::DialogUpserted(self.secret_dialog(c)));
        msg
    }

    pub(crate) async fn secret_history(&self, id: SecretChatId, anchor: Anchor, limit: usize) -> BackendResult<Vec<Message>> {
        let s = self.secrets().await?;
        let c = s.chats.get(&id.0).ok_or(BackendError::NotFound)?;
        Ok(history_page(&c.log, anchor, limit))
    }

    pub(crate) async fn secret_typing(&self, id: SecretChatId, typing: bool) -> BackendResult<()> {
        let input = {
            let s = self.secrets().await?;
            s.chats.get(&id.0).ok_or(BackendError::NotFound)?.input()
        };
        // Lo ve el servidor (no va cifrado), igual que en los clientes oficiales.
        self.rpc(&tl::functions::messages::SetEncryptedTyping { peer: input, typing }).await?;
        Ok(())
    }

    pub(crate) async fn secret_mark_read(self: &Arc<Self>, id: SecretChatId, max_id: MsgId) -> BackendResult<()> {
        let mut s = self.secrets().await?;
        let c = s.chats.get_mut(&id.0).ok_or(BackendError::NotFound)?;
        let read: Vec<(i64, i64, Option<u32>)> = c
            .log
            .iter()
            .filter(|m| !m.outgoing && m.id <= max_id)
            .filter_map(|m| m.random_id.map(|r| (r.0, m.date.0, m.ttl_secs)))
            .collect();
        let max_date = read.iter().map(|r| r.1).max().unwrap_or(0);
        if max_date > 0 {
            self.rpc(&tl::functions::messages::ReadEncryptedHistory { peer: c.input(), max_date: max_date as i32 }).await?;
        }
        // Mensajes con TTL leídos por primera vez: avisar al otro y arrancar la cuenta atrás.
        let now = now_secs();
        let mut ttl_rids = Vec::new();
        for (rid, _, ttl) in &read {
            if let Some(t) = ttl
                && c.read_sent.insert(*rid)
            {
                ttl_rids.push(*rid);
                if let Some(at) = c.timer.start(*rid, *t as i32, now) {
                    self.schedule_expiry(c.id, at);
                }
            }
        }
        let mut evs = Vec::new();
        if !ttl_rids.is_empty()
            && let Ok(sess) = c.ready()
        {
            match sess.send_action(Action::ReadMessages { random_ids: ttl_rids }, &mut OsEntropy) {
                Ok(out) => evs = self.process(c, out, now).await.events,
                Err(e) => tracing::warn!(error = %e, "no se pudo avisar de la lectura"),
            }
        }
        c.unread = c.log.iter().filter(|m| !m.outgoing && m.id > max_id).count() as u32;
        evs.push(BackendEvent::ReadInbox { peer: c.peer(), max_id });
        evs.push(BackendEvent::DialogUpserted(self.secret_dialog(c)));
        drop(s);
        self.emit_all(evs);
        Ok(())
    }

    pub(crate) async fn secret_delete(self: &Arc<Self>, id: SecretChatId, ids: Vec<MsgId>, revoke: bool) -> BackendResult<()> {
        let mut s = self.secrets().await?;
        let c = s.chats.get_mut(&id.0).ok_or(BackendError::NotFound)?;
        let rids: Vec<i64> = ids.iter().filter_map(|i| c.random_of(*i)).collect();
        let mut evs = Vec::new();
        if revoke
            && !rids.is_empty()
            && let Ok(sess) = c.ready()
        {
            let out = sess.send_action(Action::DeleteMessages { random_ids: rids.clone() }, &mut OsEntropy).map_err(crypto)?;
            evs = self.process(c, out, now_secs()).await.events;
        }
        let removed = c.remove_random(&rids);
        if !removed.is_empty() {
            evs.push(BackendEvent::MessagesDeleted { peer: c.peer(), ids: removed });
            evs.push(BackendEvent::DialogUpserted(self.secret_dialog(c)));
        }
        drop(s);
        self.emit_all(evs);
        Ok(())
    }

    // ---- updates ----

    /// Aplica los updates de chats secretos de un lote y confirma la cola (`receivedQueue`).
    pub(crate) async fn apply_secret_updates(self: &Arc<Self>, ups: Vec<tl::enums::Update>) {
        use tl::enums::Update as U;
        let mut s = match self.secrets().await {
            Ok(s) => s,
            Err(e) => {
                tracing::warn!(error = %e, "no se pudo cargar el estado de los chats secretos");
                return;
            }
        };
        let mut evs = Vec::new();
        let mut max_qts = 0;
        for u in ups {
            match u {
                U::Encryption(u) => self.on_encryption(&mut s, u.chat, &mut evs).await,
                U::NewEncryptedMessage(u) => {
                    max_qts = max_qts.max(u.qts);
                    let (chat_id, bytes, date, file) = match u.message {
                        tl::enums::EncryptedMessage::Message(m) => (m.chat_id, m.bytes, m.date, !matches!(m.file, tl::enums::EncryptedFile::Empty)),
                        tl::enums::EncryptedMessage::Service(m) => (m.chat_id, m.bytes, m.date, false),
                    };
                    if file {
                        tracing::warn!(chat = chat_id, "medio cifrado recibido: no soportado todavía (TODO)");
                    }
                    let Some(c) = s.chats.get_mut(&chat_id) else { continue };
                    self.on_encrypted(c, bytes, date.into(), &mut evs).await;
                }
                U::EncryptedChatTyping(t) => {
                    if let Some(c) = s.chats.get(&t.chat_id) {
                        evs.push(BackendEvent::Typing { peer: c.peer(), user: c.user, typing: true });
                    }
                }
                U::EncryptedMessagesRead(r) => {
                    if let Some(c) = s.chats.get_mut(&r.chat_id) {
                        let max = c.log.iter().filter(|m| m.outgoing && m.date.0 <= i64::from(r.max_date)).map(|m| m.id).max();
                        if let Some(max) = max.filter(|m| *m > c.read_outbox) {
                            c.read_outbox = max;
                            evs.push(BackendEvent::ReadOutbox { peer: c.peer(), max_id: max });
                            evs.push(BackendEvent::DialogUpserted(self.secret_dialog(c)));
                        }
                    }
                }
                _ => {}
            }
        }
        // Se confirma después de guardar el estado (lo hace `process`).
        if max_qts > s.acked_qts {
            match self.rpc(&tl::functions::messages::ReceivedQueue { max_qts }).await {
                Ok(_) => s.acked_qts = max_qts,
                Err(e) => tracing::warn!(error = %e, "receivedQueue fallido"),
            }
        }
        drop(s);
        self.emit_all(evs);
    }

    async fn on_encryption(self: &Arc<Self>, s: &mut Secrets, chat: tl::enums::EncryptedChat, evs: &mut Vec<BackendEvent>) {
        use tl::enums::EncryptedChat as E;
        match chat {
            E::Requested(r) => {
                if s.chats.contains_key(&r.id) {
                    return;
                }
                let dh = match self.dh_config(s).await {
                    Ok((dh, _)) => dh,
                    Err(e) => {
                        tracing::warn!(error = %e, "sin parámetros DH para la solicitud entrante");
                        return;
                    }
                };
                match sec::SecretChat::from_request(dh, &r.g_a) {
                    Ok(session) => {
                        let chat = Chat::new(r.id, r.access_hash, UserId(r.admin_id), false, r.date.into(), Some(session));
                        self.persist_secret(&chat).await;
                        evs.push(BackendEvent::SecretChat(chat.model()));
                        evs.push(BackendEvent::DialogUpserted(self.secret_dialog(&chat)));
                        s.chats.insert(r.id, chat);
                    }
                    Err(e) => {
                        tracing::warn!(chat = r.id, error = %e, "solicitud de chat secreto inválida: se rechaza");
                        let _ = self.rpc(&tl::functions::messages::DiscardEncryption { delete_history: false, chat_id: r.id }).await;
                    }
                }
            }
            E::Chat(ec) => {
                let Some(c) = s.chats.get_mut(&ec.id) else { return };
                c.access_hash = ec.access_hash;
                let Some(sess) = c.session.as_mut().filter(|x| x.state_kind() == StateKind::Requested) else { return };
                let out = match sess.on_accepted(&ec.g_a_or_b, ec.key_fingerprint, &mut OsEntropy, now_secs()) {
                    Ok(o) => o,
                    Err(e) => {
                        tracing::warn!(chat = ec.id, error = %e, "aceptación de chat secreto no procesable");
                        return;
                    }
                };
                let mut p = self.process(c, out, now_secs()).await;
                evs.append(&mut p.events);
                for (bytes, date) in std::mem::take(&mut c.early) {
                    self.on_encrypted(c, bytes, date, evs).await;
                }
            }
            E::Waiting(w) => {
                if let Some(c) = s.chats.get_mut(&w.id) {
                    c.access_hash = w.access_hash;
                }
            }
            E::Discarded(d) => {
                let Some(c) = s.chats.get_mut(&d.id) else { return };
                if c.state == SecretChatState::Discarded {
                    return;
                }
                let out = match c.session.as_mut() {
                    Some(sess) => sess.on_remote_discarded(),
                    None => sec::Output { outbound: vec![], events: vec![sec::Event::Discarded(sec::DiscardReason::Remote)] },
                };
                let mut p = self.process(c, out, now_secs()).await;
                evs.append(&mut p.events);
                if d.history_deleted {
                    let _ = self.absorb(c, vec![sec::Event::HistoryFlushed], now_secs(), evs);
                }
            }
            E::Empty(_) => {}
        }
    }

    async fn on_encrypted(self: &Arc<Self>, c: &mut Chat, bytes: Vec<u8>, date: i64, evs: &mut Vec<BackendEvent>) {
        let Some(sess) = c.session.as_mut() else { return };
        match sess.state_kind() {
            StateKind::Requested => {
                if c.early.len() < EARLY_CAP {
                    c.early.push((bytes, date));
                }
                return;
            }
            StateKind::Ready => {}
            _ => return,
        }
        match sess.receive(&bytes, &mut OsEntropy, now_secs()) {
            Ok(out) => {
                let mut p = self.process(c, out, date).await;
                evs.append(&mut p.events);
                evs.append(&mut self.maybe_rekey(c).await);
            }
            // Manipulado, clave desconocida o TL no soportado: se ignora sin tocar el estado.
            Err(e) => tracing::debug!(chat = c.id, error = %e, "mensaje cifrado ignorado"),
        }
    }
}

#[cfg(test)]
mod unit {
    use super::*;

    #[test]
    fn entities_round_trip_through_utf16() {
        let text = "🦊 hola mundo";
        let ents = vec![
            TextEntity { kind: EntityKind::Bold, offset: 2, length: 4 },
            TextEntity { kind: EntityKind::Pre, offset: 7, length: 5 },
            TextEntity { kind: EntityKind::Url, offset: 20, length: 1 },
        ];
        let sec = entities_to_secret(text, &ents);
        assert_eq!(sec.len(), 2, "la entidad fuera del texto se descarta");
        assert_eq!((sec[0].offset, sec[0].length), (3, 4), "el emoji ocupa 2 unidades UTF-16");
        assert_eq!(entities_from_secret(text, &sec), ents[..2]);
    }

    #[test]
    fn history_pages_follow_anchors() {
        let log: Vec<Message> = (1..=10)
            .map(|i| Message {
                id: MsgId(i),
                peer: PeerRef::Secret(SecretChatId(1)),
                sender: None,
                date: Timestamp(i.into()),
                edit_date: None,
                outgoing: false,
                text: String::new(),
                entities: vec![],
                reply_to: None,
                forwarded_from: None,
                media: None,
                sticker: None,
                reactions: vec![],
                random_id: None,
                state: SendState::Sent,
                ttl_secs: None,
            })
            .collect();
        let ids = |v: Vec<Message>| v.into_iter().map(|m| m.id.0).collect::<Vec<_>>();
        assert_eq!(ids(history_page(&log, Anchor::Latest, 3)), [8, 9, 10]);
        assert_eq!(ids(history_page(&log, Anchor::Before(MsgId(5)), 2)), [3, 4]);
        assert_eq!(ids(history_page(&log, Anchor::After(MsgId(5)), 2)), [6, 7]);
        assert_eq!(ids(history_page(&log, Anchor::Around(MsgId(5)), 4)), [3, 4, 5, 6]);
    }

    #[test]
    fn memory_store_round_trip_and_censored_debug() {
        let st = MemorySecretStateStore::default();
        let rec = StoredSecretChat {
            id: SecretChatId(3),
            access_hash: 1,
            user: UserId(2),
            outgoing: true,
            date: 0,
            state: SecretChatState::Accepted,
            ttl_secs: None,
            next_msg_id: 9,
            session: Some(Zeroizing::new(b"TCS1-clave".to_vec())),
        };
        st.save(&rec).unwrap();
        assert_eq!(st.load_all().unwrap().len(), 1);
        assert!(!format!("{rec:?}").contains("clave"));
        st.remove(SecretChatId(3)).unwrap();
        assert!(st.is_empty());
    }
}
