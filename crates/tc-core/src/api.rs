//! Trait `ChatBackend` y sub-traits por área. Se usa como `Arc<dyn ChatBackend>`.
use async_trait::async_trait;
use futures::stream::BoxStream;

use crate::{error::BackendResult, event::BackendEvent, ids::*, model::*};

#[async_trait]
pub trait ProfileApi: Send + Sync {
    /// Usuario de la cuenta actual.
    async fn me(&self) -> BackendResult<User>;
    async fn user(&self, id: UserId) -> BackendResult<User>;
    async fn group(&self, id: GroupId) -> BackendResult<Group>;
    async fn contacts(&self) -> BackendResult<Vec<User>>;
}

#[async_trait]
pub trait DialogsApi: Send + Sync {
    /// Página de la lista, ya ordenada: fijados primero, luego por actividad descendente.
    async fn dialogs(&self, page: DialogsPage) -> BackendResult<Vec<Dialog>>;
    async fn set_pinned(&self, peer: PeerRef, pinned: bool) -> BackendResult<()>;
    async fn set_archived(&self, peer: PeerRef, archived: bool) -> BackendResult<()>;
    async fn set_muted(&self, peer: PeerRef, muted: bool) -> BackendResult<()>;
    async fn save_draft(&self, peer: PeerRef, draft: Option<Draft>) -> BackendResult<()>;
}

#[async_trait]
pub trait MessagesApi: Send + Sync {
    /// Página de historial en orden ascendente de id.
    async fn history(&self, peer: PeerRef, anchor: Anchor, limit: usize) -> BackendResult<Vec<Message>>;
    /// Idempotente por `random_id`: reenviar el mismo devuelve el mensaje ya creado.
    async fn send_text(&self, peer: PeerRef, msg: OutgoingText) -> BackendResult<Message>;
    async fn edit_message(
        &self,
        peer: PeerRef,
        id: MsgId,
        text: String,
        entities: Vec<TextEntity>,
    ) -> BackendResult<Message>;
    /// `revoke` borra también para el otro lado.
    async fn delete_messages(&self, peer: PeerRef, ids: Vec<MsgId>, revoke: bool) -> BackendResult<()>;
    async fn forward_messages(
        &self,
        from: PeerRef,
        ids: Vec<MsgId>,
        to: Vec<PeerRef>,
        hide_sender: bool,
    ) -> BackendResult<()>;
    /// `None` quita mi reacción.
    async fn send_reaction(&self, peer: PeerRef, id: MsgId, emoji: Option<String>) -> BackendResult<()>;
    async fn mark_read(&self, peer: PeerRef, max_id: MsgId) -> BackendResult<()>;
    async fn set_typing(&self, peer: PeerRef, typing: bool) -> BackendResult<()>;
}

#[async_trait]
pub trait MediaApi: Send + Sync {
    /// Envía fotos/archivos (un mensaje por elemento; la leyenda va en el primero).
    async fn send_media(&self, peer: PeerRef, media: OutgoingMedia) -> BackendResult<Vec<Message>>;
}

#[async_trait]
pub trait StickersApi: Send + Sync {
    async fn installed_sticker_sets(&self) -> BackendResult<Vec<StickerSet>>;
    async fn send_sticker(
        &self,
        peer: PeerRef,
        sticker: StickerId,
        reply_to: Option<MsgId>,
        random_id: RandomId,
    ) -> BackendResult<Message>;
}

#[async_trait]
pub trait SecretChatsApi: Send + Sync {
    async fn secret_chats(&self) -> BackendResult<Vec<SecretChat>>;
    async fn request_secret_chat(&self, user: UserId) -> BackendResult<SecretChat>;
    async fn accept_secret_chat(&self, id: SecretChatId) -> BackendResult<SecretChat>;
    async fn discard_secret_chat(&self, id: SecretChatId) -> BackendResult<()>;
    async fn set_secret_ttl(&self, id: SecretChatId, ttl_secs: Option<u32>) -> BackendResult<()>;
    async fn secret_key_visual(&self, id: SecretChatId) -> BackendResult<KeyVisual>;
}

pub trait ChatBackend:
    ProfileApi + DialogsApi + MessagesApi + MediaApi + StickersApi + SecretChatsApi + Send + Sync + 'static
{
    /// Flujo de eventos en tiempo real, en orden de llegada. Cada llamada crea un
    /// suscriptor nuevo que solo ve eventos posteriores a la suscripción.
    fn events(&self) -> BoxStream<'static, BackendEvent>;
}
