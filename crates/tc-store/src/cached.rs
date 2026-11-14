//! `CachedBackend`: decorador de `ChatBackend` con la caché SQLite.
//!
//! * **Lecturas** (`dialogs`, `history`): si la caché basta para responder, se sirve de inmediato y en
//!   segundo plano se pide lo mismo al backend; lo que cambió se guarda y se anuncia por `events()`
//!   (`DialogUpserted`, `MessageNew`, `MessageEdited`, `MessagesDeleted`). Si no basta, se espera al
//!   backend (y se guarda); si la red falla se devuelve lo que haya en caché.
//! * **Escrituras**: se reenvían al backend y, si salen bien, se reflejan en la caché. Los envíos de
//!   texto dejan antes un pendiente en la base, que pasa a «fallido» si el backend falla.
//! * **Eventos**: cada evento de `events()` se persiste antes de entregarse (al consumir el stream).
//! * **Búsqueda local** FTS5 con [`CachedBackend::search_local`].
//! * **Chats secretos**: todo pasa por el backend sin escribir contenido en la base (ver `convert.rs`).
//! * **Descargas** (fotos de perfil y medios): con [`CachedBackend::with_media_cache`] los bytes se sirven
//!   desde la caché de archivos (`MediaCache`, LRU en disco) y solo se piden a la red una vez; nunca los
//!   de chats secretos.
//!
//! Todo el acceso a SQLite va por `spawn_blocking`, por lo que requiere un runtime de tokio.

use crate::convert as cv;
use crate::db::Store;
use crate::error::Result as StoreResult;
use crate::types as db;
use async_trait::async_trait;
use futures::StreamExt;
use futures::stream::BoxStream;
use std::future::Future;
use std::sync::{Arc, Mutex, Weak};
use crate::media_cache::{MediaCache, fetch_cached, media_key, profile_photo_key};
use tc_core::{
    Anchor, BackendError, BackendEvent, BackendResult, ChatBackend, Dialog, DialogsApi, DialogsPage, Draft, FileRef, Group, GroupId,
    KeyVisual, MediaApi, MediaVariant, Message, MessagesApi, MsgId, OutgoingMedia, OutgoingText, PeerRef, PhotoSize, ProfileApi,
    ProfilePhoto, RandomId, SecretChat, SecretChatId, SecretChatsApi, SendState, StickerId, StickerSet, StickersApi, TextEntity,
    SearchPage, SearchQuery, ThreadRef, Timestamp, User, UserId,
};
use tokio::sync::broadcast;

/// `ForumApi` del decorador (temas y historial por tema).
mod forum;

/// Resultado de la búsqueda local.
#[derive(Debug, Clone, PartialEq)]
pub struct LocalHit {
    pub peer: PeerRef,
    pub msg_id: MsgId,
    pub date: Timestamp,
    /// Fragmento con las coincidencias entre «».
    pub snippet: String,
}

struct Shared<B> {
    inner: B,
    store: Mutex<Store>,
    /// Eventos generados por la propia caché (refrescos en segundo plano).
    local: broadcast::Sender<BackendEvent>,
    /// Caché de archivos descargados (opcional).
    media: Option<Arc<MediaCache>>,
}

/// Decorador de caché sobre un backend. Se clona barato (comparte la base y el backend).
pub struct CachedBackend<B: ChatBackend> {
    shared: Arc<Shared<B>>,
}

impl<B: ChatBackend> Clone for CachedBackend<B> {
    fn clone(&self) -> Self {
        Self { shared: Arc::clone(&self.shared) }
    }
}

fn storage(e: crate::Error) -> BackendError {
    BackendError::Storage(e.to_string())
}

/// Errores tras los cuales tiene sentido servir lo que haya en caché.
fn is_transient(e: &BackendError) -> bool {
    matches!(e, BackendError::Network | BackendError::FloodWait(_) | BackendError::Internal)
}

impl<B: ChatBackend> Shared<B> {
    async fn db<T, F>(self: &Arc<Self>, f: F) -> BackendResult<T>
    where
        F: FnOnce(&Store) -> StoreResult<T> + Send + 'static,
        T: Send + 'static,
    {
        let me = Arc::clone(self);
        tokio::task::spawn_blocking(move || {
            let guard = me.store.lock().unwrap_or_else(|e| e.into_inner());
            f(&guard)
        })
        .await
        .map_err(|_| BackendError::Internal)?
        .map_err(storage)
    }

    fn emit(&self, events: Vec<BackendEvent>) {
        for e in events {
            let _ = self.local.send(e); // sin suscriptores no pasa nada
        }
    }

    /// Lanza `f` en segundo plano si hay runtime.
    fn spawn_bg<F, Fut>(self: &Arc<Self>, f: F)
    where
        F: FnOnce(Arc<Self>) -> Fut,
        Fut: Future<Output = ()> + Send + 'static,
    {
        if let Ok(h) = tokio::runtime::Handle::try_current() {
            h.spawn(f(Arc::clone(self)));
        }
    }

    async fn refresh_dialogs(self: &Arc<Self>, page: DialogsPage, announce: bool) -> BackendResult<Vec<Dialog>> {
        let fetched = self.inner.dialogs(page).await?;
        let to_store = fetched.clone();
        let changed = self.db(move |s| cv::apply_dialogs(s, &to_store, page)).await?;
        if announce {
            self.emit(changed.into_iter().map(BackendEvent::DialogUpserted).collect());
        }
        Ok(fetched)
    }

    async fn refresh_history(self: &Arc<Self>, peer: PeerRef, anchor: Anchor, limit: usize, announce: bool) -> BackendResult<Vec<Message>> {
        let fetched = self.inner.history(peer, anchor, limit).await?;
        let to_store = fetched.clone();
        let events = self.db(move |s| cv::apply_history(s, peer, anchor, limit, &to_store)).await?;
        if announce {
            self.emit(events);
        }
        Ok(fetched)
    }

    /// Persiste un evento del backend. Los fallos se registran y no frenan la entrega.
    async fn persist_event(self: &Arc<Self>, ev: BackendEvent) {
        let r = self
            .db(move |s| -> StoreResult<()> {
                match ev {
                    BackendEvent::DialogUpserted(d) => {
                        cv::persist_dialog(s, &d)?;
                    }
                    BackendEvent::MessageNew(m) | BackendEvent::MessageEdited(m) => cv::persist_message(s, &m, None)?,
                    BackendEvent::MessageSent { random_id, msg } => cv::persist_message(s, &msg, Some(random_id))?,
                    BackendEvent::MessageFailed { random_id, .. } => {
                        s.mark_failed_by_random(random_id.0)?;
                    }
                    BackendEvent::MessagesDeleted { peer, ids } => {
                        if let Some(ck) = cv::find_chat_key(s, peer)? {
                            let ids: Vec<i64> = ids.iter().map(|i| i64::from(i.0)).collect();
                            s.delete_messages(ck, &ids)?;
                        }
                    }
                    BackendEvent::ReadInbox { peer, max_id } => {
                        if let Some(ck) = cv::find_chat_key(s, peer)? {
                            s.mark_dialog_read(ck, i64::from(max_id.0))?;
                        }
                    }
                    BackendEvent::ReadOutbox { peer, max_id } => {
                        if let Some(ck) = cv::find_chat_key(s, peer)? {
                            s.set_dialog_read_outbox(ck, i64::from(max_id.0))?;
                        }
                    }
                    BackendEvent::UserStatus { user, status } => cv::persist_user_status(s, user.0, status)?,
                    BackendEvent::Reactions { peer, msg_id, reactions } => {
                        if let Some(ck) = cv::find_chat_key(s, peer).ok().flatten().filter(|_| !cv::is_secret(peer)) {
                            let rx: Vec<db::ReactionRow> = reactions
                                .iter()
                                .map(|r| db::ReactionRow { emoji: r.emoji.clone(), count: i64::from(r.count), chosen: r.mine })
                                .collect();
                            s.set_reactions(ck, i64::from(msg_id.0), &rx)?;
                        }
                    }
                    // Sin efecto en la base: estado de conexión, «escribiendo…» y chats secretos (su estado de
                    // protocolo y claves los gestiona quien implementa el backend, no esta caché).
                    BackendEvent::Connection(_)
                    | BackendEvent::Typing { .. }
                    | BackendEvent::SecretChat(_)
                    | BackendEvent::Download { .. }
                    | BackendEvent::TransferProgress(_) => {}
                    BackendEvent::DialogRemoved { peer } => {
                        if let Some(ck) = cv::find_chat_key(s, peer)? {
                            s.delete_dialog(ck)?;
                        }
                    }
                    BackendEvent::HistoryCleared { peer } => {
                        if let Some(ck) = cv::find_chat_key(s, peer)? {
                            s.clear_dialog_history(ck)?;
                        }
                    }
                    // Grupos con temas (`cached/forum.rs`).
                    BackendEvent::TopicMessageNew { topic, msg } => {
                        if let PeerRef::Group(group) = msg.peer {
                            forum::persist_topic_messages(s, tc_core::ThreadRef { group, topic }, std::slice::from_ref(&msg))?;
                        }
                    }
                    BackendEvent::ForumTopicUpserted(t) => forum::persist_topic(s, &t)?,
                    BackendEvent::ForumTopicsChanged { .. } => {}
                }
                Ok(())
            })
            .await;
        if let Err(e) = r {
            tracing::warn!(error = %e, "no se pudo persistir un evento");
        }
    }
}

impl<B: ChatBackend> CachedBackend<B> {
    /// Envuelve `inner` con la caché `store` (una por cuenta).
    pub fn new(inner: B, store: Store) -> Self {
        Self::build(inner, store, None)
    }

    /// Como [`CachedBackend::new`], y además sirve las descargas desde `media` (caché de archivos).
    pub fn with_media_cache(inner: B, store: Store, media: Arc<MediaCache>) -> Self {
        Self::build(inner, store, Some(media))
    }

    fn build(inner: B, store: Store, media: Option<Arc<MediaCache>>) -> Self {
        let (local, _) = broadcast::channel(256);
        Self { shared: Arc::new(Shared { inner, store: Mutex::new(store), local, media }) }
    }

    /// La caché de archivos, si la hay.
    pub fn media_cache(&self) -> Option<&Arc<MediaCache>> {
        self.shared.media.as_ref()
    }

    /// El backend decorado.
    pub fn inner(&self) -> &B {
        &self.shared.inner
    }

    /// Ejecuta `f` sobre la base (en `spawn_blocking`): para los repositorios que el decorador no
    /// expone (carpetas, subidas, ajustes...).
    pub async fn with_store<T, F>(&self, f: F) -> BackendResult<T>
    where
        F: FnOnce(&Store) -> StoreResult<T> + Send + 'static,
        T: Send + 'static,
    {
        self.shared.db(f).await
    }

    /// Búsqueda de texto local (FTS5: sin acentos ni mayúsculas, prefijo en la última palabra).
    /// `peer = None` busca en todos los chats. Los chats secretos nunca se indexan.
    pub async fn search_local(&self, query: &str, peer: Option<PeerRef>, limit: usize) -> BackendResult<Vec<LocalHit>> {
        let query = query.to_owned();
        self.shared
            .db(move |s| {
                let ck = match peer {
                    Some(p) => match cv::find_chat_key(s, p)? {
                        Some(ck) => Some(ck),
                        None => return Ok(Vec::new()),
                    },
                    None => None,
                };
                let mut out = Vec::new();
                for h in s.search_messages(&query, ck, limit)? {
                    let Some(d) = s.get_dialog(h.chat_key)? else { continue };
                    let Some(peer) = cv::peer_from(d.peer_kind, d.peer_id) else { continue };
                    out.push(LocalHit { peer, msg_id: MsgId(i32::try_from(h.msg_id).unwrap_or(0)), date: Timestamp(h.date), snippet: h.snippet });
                }
                Ok(out)
            })
            .await
    }

    /// Búsqueda en un chat o tema (ADR-0012). `trust_local`: el almacén local es completo (historial
    /// completo) y basta para responder sin red. Si no, se pide al servidor y se fusiona con lo local
    /// sin duplicar por id; sin red (o sin soporte) se sirve lo local. No guarda los resultados del
    /// servidor en la caché de historial (dejarían huecos que `history` tomaría por contiguos).
    pub(crate) async fn search_merged(
        &self,
        peer: PeerRef,
        topic: Option<ThreadRef>,
        query: SearchQuery,
        trust_local: bool,
    ) -> BackendResult<SearchPage> {
        if query.limit == 0 || query.is_empty() {
            return Ok(SearchPage::default());
        }
        let local = {
            let q = query.clone();
            self.shared
                .db(move |s| {
                    let Some(ck) = cv::find_chat_key(s, peer)? else { return Ok((Vec::new(), 0, false)) };
                    let (rows, total) =
                        s.search_chat(ck, topic.map(|t| i64::from(t.topic.0)), &q.text, q.filter, q.offset.map(|o| i64::from(o.0)), q.limit + 1)?;
                    let complete = trust_local && s.get_dialog(ck)?.is_some_and(|d| d.history_complete);
                    let msgs = rows.iter().map(|r| cv::load_message(s, peer, r)).collect::<StoreResult<Vec<_>>>()?;
                    Ok((msgs, total, complete))
                })
                .await?
        };
        let (mut rows, total, complete) = local;
        let more = rows.len() > query.limit;
        rows.truncate(query.limit);
        let local_page = |rows: Vec<Message>, more: bool| {
            let next = if more { rows.last().map(|m| m.id) } else { None };
            SearchPage { hits: rows, total: u32::try_from(total).ok(), next }
        };
        if complete {
            return Ok(local_page(rows, more));
        }
        let remote = match topic {
            Some(t) => self.shared.inner.search_in_topic(t, query.clone()).await,
            None => self.shared.inner.search_in_chat(peer, query.clone()).await,
        };
        match remote {
            Ok(mut page) => {
                // Dentro del tramo que cubre el servidor se añade lo local que él no devolvió.
                let floor = if page.next.is_some() { page.hits.last().map(|m| m.id) } else { None };
                for m in rows {
                    if floor.is_none_or(|f| m.id >= f) && page.hits.iter().all(|h| h.id != m.id) {
                        page.hits.push(m);
                    }
                }
                page.hits.sort_by_key(|m| std::cmp::Reverse(m.id));
                let merged = u32::try_from(page.hits.len()).unwrap_or(u32::MAX);
                page.total = page.total.map(|t| t.max(merged));
                Ok(page)
            }
            Err(e) if is_transient(&e) || e == BackendError::Forbidden(tc_core::UNSUPPORTED.into()) => Ok(local_page(rows, more)),
            Err(e) => Err(e),
        }
    }

    /// Mensajes locales sin confirmar (pendientes o fallidos) de un chat, para ofrecer «reintentar».
    pub async fn unsent_messages(&self, peer: PeerRef) -> BackendResult<Vec<Message>> {
        self.shared
            .db(move |s| {
                let Some(ck) = cv::find_chat_key(s, peer)? else { return Ok(Vec::new()) };
                s.unsent_messages(ck)?.iter().map(|r| cv::load_message(s, peer, r)).collect()
            })
            .await
    }

    /// Pide la página al backend, la guarda y la devuelve (sin pasar por la caché).
    pub async fn refresh_dialogs(&self, page: DialogsPage) -> BackendResult<Vec<Dialog>> {
        self.shared.refresh_dialogs(page, true).await
    }

    /// Pide la página de historial al backend, la guarda y la devuelve (sin pasar por la caché).
    pub async fn refresh_history(&self, peer: PeerRef, anchor: Anchor, limit: usize) -> BackendResult<Vec<Message>> {
        self.shared.refresh_history(peer, anchor, limit, true).await
    }

    /// Guarda en la base un mensaje que acaba de confirmar el backend.
    async fn remember(&self, m: Vec<Message>, rid: Option<RandomId>) {
        if let Err(e) = self.shared.db(move |s| m.iter().try_for_each(|m| cv::persist_message(s, m, rid))).await {
            tracing::warn!(error = %e, "no se pudo guardar el mensaje enviado");
        }
    }
}

#[async_trait]
impl<B: ChatBackend> ProfileApi for CachedBackend<B> {
    async fn me(&self) -> BackendResult<User> {
        match self.shared.inner.me().await {
            Ok(u) => {
                let c = u.clone();
                self.shared.db(move |s| cv::persist_user(s, &c, Some(true))).await?;
                Ok(u)
            }
            Err(e) if is_transient(&e) => {
                let cached = self.shared.db(|s| s.get_self_user()?.map(|r| cv::load_user(s, r)).transpose()).await?;
                cached.ok_or(e)
            }
            Err(e) => Err(e),
        }
    }

    async fn user(&self, id: UserId) -> BackendResult<User> {
        match self.shared.inner.user(id).await {
            Ok(u) => {
                let c = u.clone();
                self.shared.db(move |s| cv::persist_user(s, &c, None)).await?;
                Ok(u)
            }
            Err(e) if is_transient(&e) => {
                let cached = self.shared.db(move |s| s.get_user(id.0)?.map(|r| cv::load_user(s, r)).transpose()).await?;
                cached.ok_or(e)
            }
            Err(e) => Err(e),
        }
    }

    async fn group(&self, id: GroupId) -> BackendResult<Group> {
        match self.shared.inner.group(id).await {
            Ok(g) => {
                let c = g.clone();
                self.shared.db(move |s| cv::persist_group(s, &c)).await?;
                Ok(g)
            }
            Err(e) if is_transient(&e) => {
                let cached = self.shared.db(move |s| s.get_group(id.0)?.map(|r| cv::load_group(s, r)).transpose()).await?;
                cached.ok_or(e)
            }
            Err(e) => Err(e),
        }
    }

    async fn contacts(&self) -> BackendResult<Vec<User>> {
        match self.shared.inner.contacts().await {
            Ok(list) => {
                let c = list.clone();
                self.shared.db(move |s| c.iter().try_for_each(|u| cv::persist_user(s, u, None))).await?;
                Ok(list)
            }
            Err(e) if is_transient(&e) => {
                let cached = self.shared.db(|s| s.list_contacts()?.into_iter().map(|r| cv::load_user(s, r)).collect::<StoreResult<Vec<_>>>()).await?;
                if cached.is_empty() { Err(e) } else { Ok(cached) }
            }
            Err(e) => Err(e),
        }
    }

    async fn set_blocked(&self, user: UserId, blocked: bool) -> BackendResult<()> {
        self.shared.inner.set_blocked(user, blocked).await
    }

    async fn chat_info(&self, peer: PeerRef) -> BackendResult<tc_core::ChatInfo> {
        self.shared.inner.chat_info(peer).await
    }
}

#[async_trait]
impl<B: ChatBackend> DialogsApi for CachedBackend<B> {
    async fn dialogs(&self, page: DialogsPage) -> BackendResult<Vec<Dialog>> {
        let complete_key = cv::dialogs_complete_key(page.archived);
        let (cached, complete) = self
            .shared
            .db(move |s| Ok((cv::cached_dialogs(s, page)?, s.get_setting(&complete_key)?.is_some())))
            .await?;
        if page.limit == 0 {
            return Ok(Vec::new());
        }
        if !cached.is_empty() && (cached.len() >= page.limit || complete) {
            self.shared.spawn_bg(|sh| async move {
                if let Err(e) = sh.refresh_dialogs(page, true).await {
                    tracing::debug!(error = %e, "refresco de diálogos en segundo plano fallido");
                }
            });
            return Ok(cached);
        }
        match self.shared.refresh_dialogs(page, false).await {
            Ok(v) => Ok(v),
            Err(e) if is_transient(&e) && !cached.is_empty() => Ok(cached),
            Err(e) => Err(e),
        }
    }

    async fn set_pinned(&self, peer: PeerRef, pinned: bool) -> BackendResult<()> {
        self.shared.inner.set_pinned(peer, pinned).await?;
        self.shared
            .db(move |s| {
                let ck = cv::ensure_dialog(s, peer)?;
                let current = s.get_dialog(ck)?.and_then(|d| d.pinned_order);
                let order = if pinned { Some(current.map_or_else(|| s.next_pin_order(), Ok)?) } else { None };
                s.set_dialog_pinned(ck, order)
            })
            .await
    }

    async fn set_archived(&self, peer: PeerRef, archived: bool) -> BackendResult<()> {
        self.shared.inner.set_archived(peer, archived).await?;
        self.shared
            .db(move |s| {
                let ck = cv::ensure_dialog(s, peer)?;
                s.set_dialog_archived(ck, archived)?;
                if archived {
                    s.set_dialog_pinned(ck, None)?;
                }
                Ok(())
            })
            .await
    }

    async fn set_muted(&self, peer: PeerRef, muted: bool) -> BackendResult<()> {
        self.shared.inner.set_muted(peer, muted).await?;
        self.shared
            .db(move |s| {
                let ck = cv::ensure_dialog(s, peer)?;
                s.set_dialog_mute(ck, if muted { i64::from(i32::MAX) } else { 0 })
            })
            .await
    }

    /// El borrador se guarda en local **antes** de enviarlo (sobrevive a un cierre sin red), salvo en
    /// chats secretos, cuyo texto no se escribe en claro.
    async fn save_draft(&self, peer: PeerRef, draft: Option<Draft>) -> BackendResult<()> {
        if !cv::is_secret(peer) {
            let local = draft.clone();
            self.shared
                .db(move |s| {
                    let ck = cv::ensure_dialog(s, peer)?;
                    let row = local.map(|d| db::Draft { text: d.text, entities: None, reply_to: d.reply_to.map(|m| i64::from(m.0)), date: cv::now() });
                    s.set_draft(ck, row.as_ref())
                })
                .await?;
        }
        self.shared.inner.save_draft(peer, draft).await
    }

    async fn mute_until(&self, peer: PeerRef, until: Option<Timestamp>) -> BackendResult<()> {
        self.shared.inner.mute_until(peer, until).await?;
        let until = until.map_or(i64::from(i32::MAX), |t| t.0);
        self.shared.db(move |s| s.set_dialog_mute(cv::ensure_dialog(s, peer)?, until)).await
    }

    async fn set_unread_mark(&self, peer: PeerRef, unread: bool) -> BackendResult<()> {
        self.shared.inner.set_unread_mark(peer, unread).await?;
        self.shared.db(move |s| s.set_dialog_unread_mark(cv::ensure_dialog(s, peer)?, unread)).await
    }

    async fn mark_dialog_read(&self, peer: PeerRef) -> BackendResult<()> {
        self.shared.inner.mark_dialog_read(peer).await?;
        self.shared
            .db(move |s| {
                if let Some(ck) = cv::find_chat_key(s, peer)? {
                    let top = s.get_dialog(ck)?.and_then(|d| d.top_message_id).unwrap_or(0);
                    s.mark_dialog_read(ck, top)?;
                    s.set_dialog_unread_mark(ck, false)?;
                }
                Ok(())
            })
            .await
    }

    async fn clear_history(&self, peer: PeerRef) -> BackendResult<()> {
        self.shared.inner.clear_history(peer).await?;
        self.shared
            .db(move |s| {
                if let Some(ck) = cv::find_chat_key(s, peer)? {
                    s.clear_dialog_history(ck)?;
                }
                Ok(())
            })
            .await
    }

    async fn delete_dialog(&self, peer: PeerRef, revoke: bool) -> BackendResult<()> {
        self.shared.inner.delete_dialog(peer, revoke).await?;
        self.forget_dialog(peer).await
    }

    async fn leave_chat(&self, peer: PeerRef) -> BackendResult<()> {
        self.shared.inner.leave_chat(peer).await?;
        self.forget_dialog(peer).await
    }
}

impl<B: ChatBackend> CachedBackend<B> {
    /// Quita la fila y, en cascada, sus mensajes de la caché.
    async fn forget_dialog(&self, peer: PeerRef) -> BackendResult<()> {
        self.shared
            .db(move |s| {
                if let Some(ck) = cv::find_chat_key(s, peer)? {
                    s.delete_dialog(ck)?;
                }
                Ok(())
            })
            .await
    }
}

#[async_trait]
impl<B: ChatBackend> MessagesApi for CachedBackend<B> {
    async fn history(&self, peer: PeerRef, anchor: Anchor, limit: usize) -> BackendResult<Vec<Message>> {
        if cv::is_secret(peer) {
            return self.shared.inner.history(peer, anchor, limit).await;
        }
        let (cached, dialog) = self.shared.db(move |s| cv::cached_history(s, peer, anchor, limit)).await?;
        if cv::history_is_servable(&cached, dialog.as_ref(), anchor, limit) {
            self.shared.spawn_bg(move |sh| async move {
                if let Err(e) = sh.refresh_history(peer, anchor, limit, true).await {
                    tracing::debug!(error = %e, "refresco de historial en segundo plano fallido");
                }
            });
            return Ok(cached);
        }
        match self.shared.refresh_history(peer, anchor, limit, false).await {
            Ok(v) => Ok(v),
            Err(e) if is_transient(&e) && !cached.is_empty() => Ok(cached),
            Err(e) => Err(e),
        }
    }

    async fn send_text(&self, peer: PeerRef, msg: OutgoingText) -> BackendResult<Message> {
        let rid = msg.random_id;
        if !cv::is_secret(peer) {
            // Pendiente local: sobrevive a un cierre y se ve como «fallido» si el envío no llega.
            let (text, entities, reply_to) = (msg.text.clone(), msg.entities.clone(), msg.reply_to);
            let pending = Message {
                id: MsgId(i32::try_from(cv::pending_local_id(rid)).unwrap_or(-1)),
                peer,
                sender: None,
                date: Timestamp(cv::now()),
                edit_date: None,
                outgoing: true,
                text,
                entities,
                reply_to,
                forwarded_from: None,
                media: None,
                sticker: None,
                reactions: Vec::new(),
                random_id: Some(rid),
                state: SendState::Pending,
                ttl_secs: None,
            };
            self.shared.db(move |s| cv::persist_message(s, &pending, None)).await?;
        }
        match self.shared.inner.send_text(peer, msg).await {
            Ok(m) => {
                self.remember(vec![m.clone()], Some(rid)).await;
                Ok(m)
            }
            Err(e) => {
                if !cv::is_secret(peer) {
                    let _ = self.shared.db(move |s| s.mark_failed_by_random(rid.0)).await;
                }
                Err(e)
            }
        }
    }

    async fn edit_message(&self, peer: PeerRef, id: MsgId, text: String, entities: Vec<TextEntity>) -> BackendResult<Message> {
        let m = self.shared.inner.edit_message(peer, id, text, entities).await?;
        self.remember(vec![m.clone()], None).await;
        Ok(m)
    }

    async fn delete_messages(&self, peer: PeerRef, ids: Vec<MsgId>, revoke: bool) -> BackendResult<()> {
        self.shared.inner.delete_messages(peer, ids.clone(), revoke).await?;
        if cv::is_secret(peer) {
            return Ok(());
        }
        self.shared
            .db(move |s| {
                if let Some(ck) = cv::find_chat_key(s, peer)? {
                    let ids: Vec<i64> = ids.iter().map(|i| i64::from(i.0)).collect();
                    s.delete_messages(ck, &ids)?;
                }
                Ok(())
            })
            .await
    }

    async fn forward_messages(&self, from: PeerRef, ids: Vec<MsgId>, to: Vec<PeerRef>, hide_sender: bool) -> BackendResult<()> {
        // Los reenviados llegan como eventos `MessageNew` y se guardan al consumir `events()`.
        self.shared.inner.forward_messages(from, ids, to, hide_sender).await
    }

    async fn send_reaction(&self, peer: PeerRef, id: MsgId, emoji: Option<String>) -> BackendResult<()> {
        self.shared.inner.send_reaction(peer, id, emoji.clone()).await?;
        if cv::is_secret(peer) {
            return Ok(());
        }
        self.shared
            .db(move |s| {
                if let Some(ck) = cv::find_chat_key(s, peer)? {
                    s.apply_my_reaction(ck, i64::from(id.0), emoji.as_deref())?;
                }
                Ok(())
            })
            .await
    }

    async fn available_reactions(&self, peer: PeerRef) -> BackendResult<Vec<String>> {
        self.shared.inner.available_reactions(peer).await
    }

    async fn mark_read(&self, peer: PeerRef, max_id: MsgId) -> BackendResult<()> {
        self.shared.inner.mark_read(peer, max_id).await?;
        self.shared
            .db(move |s| {
                if let Some(ck) = cv::find_chat_key(s, peer)? {
                    s.mark_dialog_read(ck, i64::from(max_id.0))?;
                }
                Ok(())
            })
            .await
    }

    async fn set_typing(&self, peer: PeerRef, typing: bool) -> BackendResult<()> {
        self.shared.inner.set_typing(peer, typing).await
    }

    async fn pin_message(&self, peer: PeerRef, id: MsgId, pinned: bool) -> BackendResult<()> {
        self.shared.inner.pin_message(peer, id, pinned).await
    }

    async fn search_in_chat(&self, peer: PeerRef, query: SearchQuery) -> BackendResult<SearchPage> {
        if cv::is_secret(peer) {
            return self.shared.inner.search_in_chat(peer, query).await;
        }
        self.search_merged(peer, None, query, true).await
    }
}

#[async_trait]
impl<B: ChatBackend> MediaApi for CachedBackend<B> {
    async fn send_media(&self, peer: PeerRef, media: OutgoingMedia) -> BackendResult<Vec<Message>> {
        let sent = self.shared.inner.send_media(peer, media).await?;
        self.remember(sent.clone(), None).await;
        Ok(sent)
    }

    async fn download_profile_photo(&self, peer: PeerRef, photo: ProfilePhoto, size: PhotoSize) -> BackendResult<Vec<u8>> {
        let key = profile_photo_key(peer, photo, size);
        fetch_cached(self.shared.media.clone(), key, self.shared.inner.download_profile_photo(peer, photo, size)).await
    }

    async fn download_media(&self, peer: PeerRef, msg: MsgId, file: FileRef, variant: MediaVariant) -> BackendResult<Vec<u8>> {
        let key = media_key(peer, &file, variant);
        fetch_cached(self.shared.media.clone(), key, self.shared.inner.download_media(peer, msg, file, variant)).await
    }

    /// Pasa tal cual (ADR-0011): el archivo va a donde elige el usuario, no a la caché; el progreso
    /// llega por el bombeo de eventos como cualquier otro.
    async fn download_media_to(&self, peer: PeerRef, msg: MsgId, file: FileRef, variant: MediaVariant, dest: std::path::PathBuf) -> BackendResult<tc_core::TransferId> {
        self.shared.inner.download_media_to(peer, msg, file, variant, dest).await
    }

    async fn cancel_transfer(&self, id: tc_core::TransferId) -> BackendResult<()> {
        self.shared.inner.cancel_transfer(id).await
    }
}

#[async_trait]
impl<B: ChatBackend> StickersApi for CachedBackend<B> {
    async fn installed_sticker_sets(&self) -> BackendResult<Vec<StickerSet>> {
        // Los packs los cachea tc-stickers en disco; aquí se reenvía tal cual.
        self.shared.inner.installed_sticker_sets().await
    }

    async fn send_sticker(&self, peer: PeerRef, sticker: StickerId, reply_to: Option<MsgId>, random_id: RandomId) -> BackendResult<Message> {
        let m = self.shared.inner.send_sticker(peer, sticker, reply_to, random_id).await?;
        self.remember(vec![m.clone()], Some(random_id)).await;
        Ok(m)
    }

    async fn recent_stickers(&self) -> BackendResult<Vec<tc_core::Sticker>> {
        self.shared.inner.recent_stickers().await
    }

    async fn faved_stickers(&self) -> BackendResult<Vec<tc_core::Sticker>> {
        self.shared.inner.faved_stickers().await
    }

    /// GIFs guardados: la lista de la red se guarda; sin red se devuelve la guardada (ver `gifs.rs`).
    async fn saved_gifs(&self) -> BackendResult<Vec<tc_core::Gif>> {
        match self.shared.inner.saved_gifs().await {
            Ok(list) => {
                let keep = list.clone();
                if let Err(e) = self.shared.db(move |s| s.put_saved_gifs(&keep)).await {
                    tracing::warn!(error = %e, "no se pudo guardar la lista de GIFs");
                }
                Ok(list)
            }
            Err(e) if is_transient(&e) => match self.shared.db(|s| s.saved_gifs()).await {
                Ok(cached) if !cached.is_empty() => Ok(cached),
                _ => Err(e),
            },
            Err(e) => Err(e),
        }
    }

    async fn search_gifs(&self, query: String, offset: String) -> BackendResult<tc_core::GifPage> {
        self.shared.inner.search_gifs(query, offset).await
    }

    async fn send_gif(&self, peer: PeerRef, gif: tc_core::Gif, reply_to: Option<MsgId>, random_id: RandomId) -> BackendResult<Message> {
        let m = self.shared.inner.send_gif(peer, gif, reply_to, random_id).await?;
        self.remember(vec![m.clone()], Some(random_id)).await;
        Ok(m)
    }

    async fn emoji_usage(&self) -> BackendResult<Vec<tc_core::EmojiUse>> {
        self.shared.db(|s| s.emoji_usage()).await
    }

    async fn record_emoji_use(&self, emoji: String, at: tc_core::Timestamp) -> BackendResult<()> {
        self.shared.db(move |s| s.record_emoji_use(&emoji, at.0)).await
    }
}

/// Los chats secretos pasan íntegros: su estado de protocolo y sus claves no son de esta caché.
#[async_trait]
impl<B: ChatBackend> SecretChatsApi for CachedBackend<B> {
    async fn secret_chats(&self) -> BackendResult<Vec<SecretChat>> {
        self.shared.inner.secret_chats().await
    }
    async fn request_secret_chat(&self, user: UserId) -> BackendResult<SecretChat> {
        self.shared.inner.request_secret_chat(user).await
    }
    async fn accept_secret_chat(&self, id: SecretChatId) -> BackendResult<SecretChat> {
        self.shared.inner.accept_secret_chat(id).await
    }
    async fn discard_secret_chat(&self, id: SecretChatId) -> BackendResult<()> {
        self.shared.inner.discard_secret_chat(id).await
    }
    async fn set_secret_ttl(&self, id: SecretChatId, ttl_secs: Option<u32>) -> BackendResult<()> {
        self.shared.inner.set_secret_ttl(id, ttl_secs).await
    }
    async fn secret_key_visual(&self, id: SecretChatId) -> BackendResult<KeyVisual> {
        self.shared.inner.secret_key_visual(id).await
    }
}

impl<B: ChatBackend> ChatBackend for CachedBackend<B> {
    /// Eventos del backend (persistidos al entregarse) mezclados con los refrescos en segundo plano.
    /// La persistencia ocurre al consumir el stream: si nadie lo consume, nada se guarda.
    fn events(&self) -> BoxStream<'static, BackendEvent> {
        let weak: Weak<Shared<B>> = Arc::downgrade(&self.shared);
        let from_backend = self
            .shared
            .inner
            .events()
            .then(move |ev| {
                let weak = weak.clone();
                async move {
                    if let Some(sh) = weak.upgrade() {
                        sh.persist_event(ev.clone()).await;
                    }
                    ev
                }
            })
            .boxed();
        let from_cache = futures::stream::unfold(self.shared.local.subscribe(), |mut rx| async move {
            loop {
                match rx.recv().await {
                    Ok(e) => return Some((e, rx)),
                    Err(broadcast::error::RecvError::Lagged(_)) => continue,
                    Err(broadcast::error::RecvError::Closed) => return None,
                }
            }
        })
        .boxed();
        futures::stream::select(from_backend, from_cache).boxed()
    }
}
