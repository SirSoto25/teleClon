//! Grupos con temas («comunidades», supergrupos con `forum = true`).
//!
//! Un tema es un hilo dentro del supergrupo, identificado por el id de su mensaje superior (el de
//! creación del tema); el tema «General» siempre es el 1. Los mensajes de un tema siguen siendo
//! mensajes del grupo (`PeerRef::Group`), así que el resto del dominio no cambia: lo que es propio
//! de un tema va en [`ForumApi`] con un [`ThreadRef`].
//!
//! [`ThreadScope`] envuelve un backend y lo «ata» a un tema: historial, envíos, lectura y
//! «escribiendo…» de ese grupo van al tema; todo lo demás pasa igual. La UI lo usa para reutilizar
//! la conversación normal dentro de un tema sin tocar cada llamada.
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use async_trait::async_trait;
use futures::stream::BoxStream;

use crate::api::*;
use crate::error::BackendResult;
use crate::event::BackendEvent;
use crate::gif::{EmojiUse, Gif, GifPage};
use crate::ids::*;
use crate::model::*;

/// Id del tema «General» (siempre existe en un foro; no se puede borrar, solo ocultar).
pub const GENERAL_TOPIC: MsgId = MsgId(1);

/// Colores de icono que ofrece la red al crear un tema (RGB).
pub const TOPIC_COLORS: [u32; 6] = [0x6FB9F0, 0xFFD67E, 0xCB86DB, 0x8EEE98, 0xFF93B2, 0xFB6F5F];

/// Icono de un tema: un color (círculo) y, si lo tiene, un emoji.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct TopicIcon {
    /// Color RGB (`0xRRGGBB`).
    pub color: u32,
    /// Emoji normal para dibujar (el mock lo rellena; la red usa emoji personalizados).
    pub emoji: Option<String>,
    /// Emoji personalizado de la red (documento); la UI lo dibuja como el color si no lo conoce.
    pub custom_emoji_id: Option<i64>,
}

/// Un tema de un foro (fila de la lista de temas).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ForumTopic {
    pub group: GroupId,
    /// Id del mensaje superior del tema (= id del tema).
    pub id: MsgId,
    pub title: String,
    pub icon: TopicIcon,
    /// Creación del tema.
    pub date: Timestamp,
    pub last_message: Option<Message>,
    pub unread_count: u32,
    pub unread_mentions: u32,
    /// Último entrante leído por mí dentro del tema (para abrir en el primer no leído).
    pub read_inbox_max_id: MsgId,
    pub read_outbox_max_id: MsgId,
    pub pinned: bool,
    /// Cerrado: solo los administradores pueden escribir.
    pub closed: bool,
    /// Oculto (solo el «General» puede ocultarse).
    pub hidden: bool,
    /// Lo creé yo.
    pub my: bool,
}

impl ForumTopic {
    /// Tema mínimo (sin mensajes ni no leídos).
    pub fn new(group: GroupId, id: MsgId, title: impl Into<String>, color: u32) -> Self {
        Self {
            group,
            id,
            title: title.into(),
            icon: TopicIcon { color, emoji: None, custom_emoji_id: None },
            date: Timestamp(0),
            last_message: None,
            unread_count: 0,
            unread_mentions: 0,
            read_inbox_max_id: MsgId(0),
            read_outbox_max_id: MsgId(0),
            pinned: false,
            closed: false,
            hidden: false,
            my: false,
        }
    }

    pub fn thread(&self) -> ThreadRef {
        ThreadRef { group: self.group, topic: self.id }
    }

    pub fn is_general(&self) -> bool {
        self.id == GENERAL_TOPIC
    }

    /// Instante del último movimiento (último mensaje o creación), para ordenar.
    pub fn activity(&self) -> Timestamp {
        self.last_message.as_ref().map_or(self.date, |m| m.date.max(self.date))
    }

    /// Fila vista como un diálogo (no leídos, último leído y último mensaje): así la lectura de la
    /// conversación (abrir en el primer no leído, marcar leído a trozos) sirve igual en un tema.
    pub fn as_dialog(&self, title: String) -> Dialog {
        Dialog {
            peer: PeerRef::Group(self.group),
            title,
            last_message: self.last_message.clone(),
            unread_count: self.unread_count,
            unread_mentions: self.unread_mentions,
            pinned_order: None,
            archived: false,
            muted: false,
            draft: None,
            activity: self.activity(),
            read_outbox_max_id: self.read_outbox_max_id,
            read_inbox_max_id: self.read_inbox_max_id,
            photo: None,
            unread_mark: false,
        }
    }
}

/// Orden de la lista de temas: fijados primero (conservando su orden relativo), luego por actividad.
pub fn sort_topics(list: &mut [ForumTopic]) {
    list.sort_by(|a, b| b.pinned.cmp(&a.pinned).then(b.activity().cmp(&a.activity())).then(b.id.cmp(&a.id)));
}

/// Tema concreto de un grupo foro.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ThreadRef {
    pub group: GroupId,
    pub topic: MsgId,
}

impl ThreadRef {
    pub fn peer(self) -> PeerRef {
        PeerRef::Group(self.group)
    }
}

/// Punto de partida de la página siguiente de temas (lo da la página anterior).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TopicsOffset {
    pub date: Timestamp,
    pub msg_id: MsgId,
    pub topic: MsgId,
}

/// Petición de una página de temas.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TopicsQuery {
    /// Filtro por título (búsqueda del servidor); `None` = todos.
    pub query: Option<String>,
    /// `None` = primera página.
    pub offset: Option<TopicsOffset>,
    pub limit: usize,
}

impl TopicsQuery {
    pub fn first(limit: usize) -> Self {
        Self { query: None, offset: None, limit }
    }
}

/// Página de temas, ya ordenada (fijados primero, luego por actividad).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct TopicsPage {
    pub topics: Vec<ForumTopic>,
    /// Total de temas del foro según el servidor.
    pub total: u32,
    /// `None`: no hay más páginas.
    pub next: Option<TopicsOffset>,
}

/// Operaciones de los grupos con temas. Todo tiene una implementación por defecto («no soportado»)
/// para que un backend sin foros siga compilando.
#[async_trait]
pub trait ForumApi: Send + Sync {
    /// Página de temas (`messages.getForumTopics`).
    async fn forum_topics(&self, group: GroupId, query: TopicsQuery) -> BackendResult<TopicsPage> {
        let _ = (group, query);
        Err(unsupported())
    }
    /// Temas concretos por id (`messages.getForumTopicsByID`).
    async fn forum_topics_by_id(&self, group: GroupId, ids: Vec<MsgId>) -> BackendResult<Vec<ForumTopic>> {
        let _ = (group, ids);
        Err(unsupported())
    }
    /// Historial de un tema, ascendente (`messages.getReplies` con el mensaje superior).
    async fn topic_history(&self, thread: ThreadRef, anchor: Anchor, limit: usize) -> BackendResult<Vec<Message>> {
        let _ = (thread, anchor, limit);
        Err(unsupported())
    }
    /// Envía texto dentro de un tema (`reply_to` con `top_msg_id`). Idempotente por `random_id`.
    async fn send_topic_text(&self, thread: ThreadRef, msg: OutgoingText) -> BackendResult<Message> {
        let _ = (thread, msg);
        Err(unsupported())
    }
    async fn send_topic_media(&self, thread: ThreadRef, media: OutgoingMedia) -> BackendResult<Vec<Message>> {
        let _ = (thread, media);
        Err(unsupported())
    }
    async fn send_topic_sticker(&self, thread: ThreadRef, sticker: StickerId, reply_to: Option<MsgId>, random_id: RandomId) -> BackendResult<Message> {
        let _ = (thread, sticker, reply_to, random_id);
        Err(unsupported())
    }
    async fn send_topic_gif(&self, thread: ThreadRef, gif: Gif, reply_to: Option<MsgId>, random_id: RandomId) -> BackendResult<Message> {
        let _ = (thread, gif, reply_to, random_id);
        Err(unsupported())
    }
    /// Marca leído el tema hasta `max_id` (`messages.readDiscussion`).
    async fn mark_topic_read(&self, thread: ThreadRef, max_id: MsgId) -> BackendResult<()> {
        let _ = (thread, max_id);
        Err(unsupported())
    }
    /// «Escribiendo…» dentro del tema.
    async fn set_topic_typing(&self, thread: ThreadRef, typing: bool) -> BackendResult<()> {
        let _ = (thread, typing);
        Ok(())
    }
    /// Busca dentro de un tema (`messages.search` con `top_msg_id`), como `MessagesApi::search_in_chat`.
    async fn search_in_topic(&self, thread: ThreadRef, query: crate::search::SearchQuery) -> BackendResult<crate::search::SearchPage> {
        let _ = (thread, query);
        Err(unsupported())
    }
    /// Crea un tema (hace falta permiso de gestionar temas, o que el grupo lo permita a todos).
    async fn create_topic(&self, group: GroupId, title: String, icon_color: Option<u32>, random_id: RandomId) -> BackendResult<ForumTopic> {
        let _ = (group, title, icon_color, random_id);
        Err(unsupported())
    }
    async fn set_topic_pinned(&self, thread: ThreadRef, pinned: bool) -> BackendResult<()> {
        let _ = (thread, pinned);
        Err(unsupported())
    }
    async fn set_topic_closed(&self, thread: ThreadRef, closed: bool) -> BackendResult<()> {
        let _ = (thread, closed);
        Err(unsupported())
    }
}

/// Vista de un backend «atada» a un tema: las operaciones de mensajes sobre su grupo van al tema
/// (`topic_history`, `send_topic_*`, `mark_topic_read`, `set_topic_typing`); el resto (y los demás
/// chats) se reenvía tal cual.
///
/// `alive` permite descartar respuestas tardías: si se apaga (se salió del tema), una página de
/// historial que llegue después nunca se entrega (el futuro queda pendiente y se suelta con la
/// tarea), así que no puede colarse en la conversación de otro tema del mismo grupo.
pub struct ThreadScope {
    inner: Arc<dyn ChatBackend>,
    thread: ThreadRef,
    alive: Arc<AtomicBool>,
}

impl ThreadScope {
    pub fn new(inner: Arc<dyn ChatBackend>, thread: ThreadRef, alive: Arc<AtomicBool>) -> Self {
        Self { inner, thread, alive }
    }

    pub fn thread(&self) -> ThreadRef {
        self.thread
    }

    fn mine(&self, peer: PeerRef) -> bool {
        peer == self.thread.peer()
    }

    async fn unless_stale<T>(&self, r: T) -> T {
        if self.alive.load(Ordering::Acquire) {
            r
        } else {
            std::future::pending().await
        }
    }
}

#[async_trait]
impl ProfileApi for ThreadScope {
    async fn me(&self) -> BackendResult<User> {
        self.inner.me().await
    }
    async fn user(&self, id: UserId) -> BackendResult<User> {
        self.inner.user(id).await
    }
    async fn group(&self, id: GroupId) -> BackendResult<Group> {
        self.inner.group(id).await
    }
    async fn contacts(&self) -> BackendResult<Vec<User>> {
        self.inner.contacts().await
    }
    async fn set_blocked(&self, user: UserId, blocked: bool) -> BackendResult<()> {
        self.inner.set_blocked(user, blocked).await
    }
    async fn chat_info(&self, peer: PeerRef) -> BackendResult<ChatInfo> {
        self.inner.chat_info(peer).await
    }
}

#[async_trait]
impl DialogsApi for ThreadScope {
    async fn dialogs(&self, page: DialogsPage) -> BackendResult<Vec<Dialog>> {
        self.inner.dialogs(page).await
    }
    async fn set_pinned(&self, peer: PeerRef, pinned: bool) -> BackendResult<()> {
        self.inner.set_pinned(peer, pinned).await
    }
    async fn set_archived(&self, peer: PeerRef, archived: bool) -> BackendResult<()> {
        self.inner.set_archived(peer, archived).await
    }
    async fn set_muted(&self, peer: PeerRef, muted: bool) -> BackendResult<()> {
        self.inner.set_muted(peer, muted).await
    }
    /// Los borradores de un tema no se guardan en el grupo (pisarían el del grupo).
    async fn save_draft(&self, peer: PeerRef, draft: Option<Draft>) -> BackendResult<()> {
        if self.mine(peer) {
            return Ok(());
        }
        self.inner.save_draft(peer, draft).await
    }
    async fn mute_until(&self, peer: PeerRef, until: Option<Timestamp>) -> BackendResult<()> {
        self.inner.mute_until(peer, until).await
    }
    async fn set_unread_mark(&self, peer: PeerRef, unread: bool) -> BackendResult<()> {
        self.inner.set_unread_mark(peer, unread).await
    }
    async fn mark_dialog_read(&self, peer: PeerRef) -> BackendResult<()> {
        self.inner.mark_dialog_read(peer).await
    }
    async fn clear_history(&self, peer: PeerRef) -> BackendResult<()> {
        self.inner.clear_history(peer).await
    }
    async fn delete_dialog(&self, peer: PeerRef, revoke: bool) -> BackendResult<()> {
        self.inner.delete_dialog(peer, revoke).await
    }
    async fn leave_chat(&self, peer: PeerRef) -> BackendResult<()> {
        self.inner.leave_chat(peer).await
    }
}

#[async_trait]
impl MessagesApi for ThreadScope {
    async fn history(&self, peer: PeerRef, anchor: Anchor, limit: usize) -> BackendResult<Vec<Message>> {
        if self.mine(peer) {
            let r = self.inner.topic_history(self.thread, anchor, limit).await;
            return self.unless_stale(r).await;
        }
        self.inner.history(peer, anchor, limit).await
    }
    async fn send_text(&self, peer: PeerRef, msg: OutgoingText) -> BackendResult<Message> {
        if self.mine(peer) {
            return self.inner.send_topic_text(self.thread, msg).await;
        }
        self.inner.send_text(peer, msg).await
    }
    async fn edit_message(&self, peer: PeerRef, id: MsgId, text: String, entities: Vec<TextEntity>) -> BackendResult<Message> {
        self.inner.edit_message(peer, id, text, entities).await
    }
    async fn delete_messages(&self, peer: PeerRef, ids: Vec<MsgId>, revoke: bool) -> BackendResult<()> {
        self.inner.delete_messages(peer, ids, revoke).await
    }
    async fn forward_messages(&self, from: PeerRef, ids: Vec<MsgId>, to: Vec<PeerRef>, hide_sender: bool) -> BackendResult<()> {
        self.inner.forward_messages(from, ids, to, hide_sender).await
    }
    async fn send_reaction(&self, peer: PeerRef, id: MsgId, emoji: Option<String>) -> BackendResult<()> {
        self.inner.send_reaction(peer, id, emoji).await
    }
    async fn available_reactions(&self, peer: PeerRef) -> BackendResult<Vec<String>> {
        self.inner.available_reactions(peer).await
    }
    async fn mark_read(&self, peer: PeerRef, max_id: MsgId) -> BackendResult<()> {
        if self.mine(peer) {
            return self.inner.mark_topic_read(self.thread, max_id).await;
        }
        self.inner.mark_read(peer, max_id).await
    }
    async fn set_typing(&self, peer: PeerRef, typing: bool) -> BackendResult<()> {
        if self.mine(peer) {
            return self.inner.set_topic_typing(self.thread, typing).await;
        }
        self.inner.set_typing(peer, typing).await
    }
    async fn pin_message(&self, peer: PeerRef, id: MsgId, pinned: bool) -> BackendResult<()> {
        self.inner.pin_message(peer, id, pinned).await
    }
    async fn search_in_chat(&self, peer: PeerRef, query: crate::search::SearchQuery) -> BackendResult<crate::search::SearchPage> {
        if self.mine(peer) {
            let r = self.inner.search_in_topic(self.thread, query).await;
            return self.unless_stale(r).await;
        }
        self.inner.search_in_chat(peer, query).await
    }
}

#[async_trait]
impl MediaApi for ThreadScope {
    async fn send_media(&self, peer: PeerRef, media: OutgoingMedia) -> BackendResult<Vec<Message>> {
        if self.mine(peer) {
            return self.inner.send_topic_media(self.thread, media).await;
        }
        self.inner.send_media(peer, media).await
    }
    async fn download_profile_photo(&self, peer: PeerRef, photo: ProfilePhoto, size: PhotoSize) -> BackendResult<Vec<u8>> {
        self.inner.download_profile_photo(peer, photo, size).await
    }
    async fn download_media(&self, peer: PeerRef, msg: MsgId, file: FileRef, variant: MediaVariant) -> BackendResult<Vec<u8>> {
        self.inner.download_media(peer, msg, file, variant).await
    }
    async fn download_media_to(
        &self,
        peer: PeerRef,
        msg: MsgId,
        file: FileRef,
        variant: MediaVariant,
        dest: std::path::PathBuf,
    ) -> BackendResult<crate::transfer::TransferId> {
        self.inner.download_media_to(peer, msg, file, variant, dest).await
    }
    async fn cancel_transfer(&self, id: crate::transfer::TransferId) -> BackendResult<()> {
        self.inner.cancel_transfer(id).await
    }
}

#[async_trait]
impl StickersApi for ThreadScope {
    async fn installed_sticker_sets(&self) -> BackendResult<Vec<StickerSet>> {
        self.inner.installed_sticker_sets().await
    }
    async fn send_sticker(&self, peer: PeerRef, sticker: StickerId, reply_to: Option<MsgId>, random_id: RandomId) -> BackendResult<Message> {
        if self.mine(peer) {
            return self.inner.send_topic_sticker(self.thread, sticker, reply_to, random_id).await;
        }
        self.inner.send_sticker(peer, sticker, reply_to, random_id).await
    }
    async fn recent_stickers(&self) -> BackendResult<Vec<Sticker>> {
        self.inner.recent_stickers().await
    }
    async fn faved_stickers(&self) -> BackendResult<Vec<Sticker>> {
        self.inner.faved_stickers().await
    }
    async fn saved_gifs(&self) -> BackendResult<Vec<Gif>> {
        self.inner.saved_gifs().await
    }
    async fn search_gifs(&self, query: String, offset: String) -> BackendResult<GifPage> {
        self.inner.search_gifs(query, offset).await
    }
    async fn send_gif(&self, peer: PeerRef, gif: Gif, reply_to: Option<MsgId>, random_id: RandomId) -> BackendResult<Message> {
        if self.mine(peer) {
            return self.inner.send_topic_gif(self.thread, gif, reply_to, random_id).await;
        }
        self.inner.send_gif(peer, gif, reply_to, random_id).await
    }
    async fn emoji_usage(&self) -> BackendResult<Vec<EmojiUse>> {
        self.inner.emoji_usage().await
    }
    async fn record_emoji_use(&self, emoji: String, at: Timestamp) -> BackendResult<()> {
        self.inner.record_emoji_use(emoji, at).await
    }
}

#[async_trait]
impl SecretChatsApi for ThreadScope {
    async fn secret_chats(&self) -> BackendResult<Vec<SecretChat>> {
        self.inner.secret_chats().await
    }
    async fn request_secret_chat(&self, user: UserId) -> BackendResult<SecretChat> {
        self.inner.request_secret_chat(user).await
    }
    async fn accept_secret_chat(&self, id: SecretChatId) -> BackendResult<SecretChat> {
        self.inner.accept_secret_chat(id).await
    }
    async fn discard_secret_chat(&self, id: SecretChatId) -> BackendResult<()> {
        self.inner.discard_secret_chat(id).await
    }
    async fn set_secret_ttl(&self, id: SecretChatId, ttl_secs: Option<u32>) -> BackendResult<()> {
        self.inner.set_secret_ttl(id, ttl_secs).await
    }
    async fn secret_key_visual(&self, id: SecretChatId) -> BackendResult<KeyVisual> {
        self.inner.secret_key_visual(id).await
    }
}

#[async_trait]
impl ForumApi for ThreadScope {
    async fn forum_topics(&self, group: GroupId, query: TopicsQuery) -> BackendResult<TopicsPage> {
        self.inner.forum_topics(group, query).await
    }
    async fn forum_topics_by_id(&self, group: GroupId, ids: Vec<MsgId>) -> BackendResult<Vec<ForumTopic>> {
        self.inner.forum_topics_by_id(group, ids).await
    }
    async fn topic_history(&self, thread: ThreadRef, anchor: Anchor, limit: usize) -> BackendResult<Vec<Message>> {
        self.inner.topic_history(thread, anchor, limit).await
    }
    async fn search_in_topic(&self, thread: ThreadRef, query: crate::search::SearchQuery) -> BackendResult<crate::search::SearchPage> {
        self.inner.search_in_topic(thread, query).await
    }
    async fn send_topic_text(&self, thread: ThreadRef, msg: OutgoingText) -> BackendResult<Message> {
        self.inner.send_topic_text(thread, msg).await
    }
    async fn send_topic_media(&self, thread: ThreadRef, media: OutgoingMedia) -> BackendResult<Vec<Message>> {
        self.inner.send_topic_media(thread, media).await
    }
    async fn send_topic_sticker(&self, thread: ThreadRef, sticker: StickerId, reply_to: Option<MsgId>, random_id: RandomId) -> BackendResult<Message> {
        self.inner.send_topic_sticker(thread, sticker, reply_to, random_id).await
    }
    async fn send_topic_gif(&self, thread: ThreadRef, gif: Gif, reply_to: Option<MsgId>, random_id: RandomId) -> BackendResult<Message> {
        self.inner.send_topic_gif(thread, gif, reply_to, random_id).await
    }
    async fn mark_topic_read(&self, thread: ThreadRef, max_id: MsgId) -> BackendResult<()> {
        self.inner.mark_topic_read(thread, max_id).await
    }
    async fn set_topic_typing(&self, thread: ThreadRef, typing: bool) -> BackendResult<()> {
        self.inner.set_topic_typing(thread, typing).await
    }
    async fn create_topic(&self, group: GroupId, title: String, icon_color: Option<u32>, random_id: RandomId) -> BackendResult<ForumTopic> {
        self.inner.create_topic(group, title, icon_color, random_id).await
    }
    async fn set_topic_pinned(&self, thread: ThreadRef, pinned: bool) -> BackendResult<()> {
        self.inner.set_topic_pinned(thread, pinned).await
    }
    async fn set_topic_closed(&self, thread: ThreadRef, closed: bool) -> BackendResult<()> {
        self.inner.set_topic_closed(thread, closed).await
    }
}

impl ChatBackend for ThreadScope {
    fn events(&self) -> BoxStream<'static, BackendEvent> {
        self.inner.events()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn msg(id: i32, date: i64) -> Message {
        Message {
            id: MsgId(id),
            peer: PeerRef::Group(GroupId(5)),
            sender: None,
            date: Timestamp(date),
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
        }
    }

    #[test]
    fn fijados_primero_y_luego_por_actividad() {
        let g = GroupId(5);
        let mut a = ForumTopic::new(g, MsgId(1), "General", 0);
        a.last_message = Some(msg(10, 100));
        let mut b = ForumTopic::new(g, MsgId(2), "Ayuda", 0);
        b.last_message = Some(msg(11, 300));
        let mut c = ForumTopic::new(g, MsgId(3), "Normas", 0);
        c.pinned = true;
        c.date = Timestamp(5);
        let mut v = vec![a, b, c];
        sort_topics(&mut v);
        let ids: Vec<i32> = v.iter().map(|t| t.id.0).collect();
        assert_eq!(ids, [3, 2, 1]);
        assert!(v[2].is_general());
        assert_eq!(v[1].activity(), Timestamp(300));
    }

    #[test]
    fn el_tema_como_dialogo_conserva_lectura_y_no_leidos() {
        let mut t = ForumTopic::new(GroupId(5), MsgId(7), "Ayuda", 0);
        t.unread_count = 3;
        t.read_inbox_max_id = MsgId(40);
        t.last_message = Some(msg(44, 9));
        let d = t.as_dialog("Grupo".into());
        assert_eq!((d.peer, d.unread_count, d.read_inbox_max_id), (PeerRef::Group(GroupId(5)), 3, MsgId(40)));
        assert_eq!(d.last_message.map(|m| m.id), Some(MsgId(44)));
    }
}
