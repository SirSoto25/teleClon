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
    /// Bloquea o desbloquea a un usuario (`contacts.block` / `contacts.unblock`).
    async fn set_blocked(&self, user: UserId, blocked: bool) -> BackendResult<()> {
        let _ = (user, blocked);
        Err(unsupported())
    }
    /// Datos de «Ver información» (`users.getFullUser`, `messages.getFullChat`, `channels.getFullChannel`).
    async fn chat_info(&self, peer: PeerRef) -> BackendResult<ChatInfo> {
        let _ = peer;
        Err(unsupported())
    }
}

/// Error de las operaciones que un backend aún no implementa (implementación por defecto de los
/// métodos añadidos después; la UI lo explica con un texto propio).
pub const UNSUPPORTED: &str = "NOT_SUPPORTED";

pub fn unsupported() -> crate::BackendError {
    crate::BackendError::Forbidden(UNSUPPORTED.into())
}

#[async_trait]
pub trait DialogsApi: Send + Sync {
    /// Página de la lista, ya ordenada: fijados primero, luego por actividad descendente.
    async fn dialogs(&self, page: DialogsPage) -> BackendResult<Vec<Dialog>>;
    async fn set_pinned(&self, peer: PeerRef, pinned: bool) -> BackendResult<()>;
    async fn set_archived(&self, peer: PeerRef, archived: bool) -> BackendResult<()>;
    async fn set_muted(&self, peer: PeerRef, muted: bool) -> BackendResult<()>;
    async fn save_draft(&self, peer: PeerRef, draft: Option<Draft>) -> BackendResult<()>;

    // ---- Gestión de chats (menú contextual de la lista). Implementación por defecto: no soportado.

    /// Silencia hasta `until` (`None` = para siempre). Reactivar es `set_muted(peer, false)`.
    /// Por defecto, silencia para siempre.
    async fn mute_until(&self, peer: PeerRef, until: Option<Timestamp>) -> BackendResult<()> {
        let _ = until;
        self.set_muted(peer, true).await
    }
    /// Pone o quita la marca manual de «no leído» (`messages.markDialogUnread`).
    async fn set_unread_mark(&self, peer: PeerRef, unread: bool) -> BackendResult<()> {
        let _ = (peer, unread);
        Err(unsupported())
    }
    /// Lee todo el chat: lectura hasta el último mensaje (`messages.readHistory` /
    /// `channels.readHistory`) y quita la marca manual de no leído.
    async fn mark_dialog_read(&self, peer: PeerRef) -> BackendResult<()> {
        let _ = peer;
        Err(unsupported())
    }
    /// Vacía el historial sin quitar el chat (`messages.deleteHistory` con `just_clear`;
    /// en supergrupos, `channels.deleteHistory` solo para mí). Emite `HistoryCleared`.
    async fn clear_history(&self, peer: PeerRef) -> BackendResult<()> {
        let _ = peer;
        Err(unsupported())
    }
    /// Borra el chat de la lista. Privado: `messages.deleteHistory` (con `revoke`, también para el
    /// otro). Grupo básico: sale y borra el historial. Canal/supergrupo: `channels.leaveChannel`.
    /// Chat secreto: lo cierra. Emite `DialogRemoved`.
    async fn delete_dialog(&self, peer: PeerRef, revoke: bool) -> BackendResult<()> {
        let _ = (peer, revoke);
        Err(unsupported())
    }
    /// Sale del grupo o canal (`messages.deleteChatUser` sobre mí / `channels.leaveChannel`).
    /// Emite `DialogRemoved`.
    async fn leave_chat(&self, peer: PeerRef) -> BackendResult<()> {
        let _ = peer;
        Err(unsupported())
    }
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
    /// Reacciones que esta cuenta puede poner en `peer`, en el orden en que se ofrecen (las más
    /// usadas primero). Vacío: en ese chat no se puede reaccionar (p. ej. «Mensajes guardados» sin
    /// Premium, o un grupo con las reacciones desactivadas). Solo emoji, nunca premium sin Premium.
    async fn available_reactions(&self, peer: PeerRef) -> BackendResult<Vec<String>> {
        let _ = peer;
        Ok(crate::model::DEFAULT_REACTIONS.iter().map(|e| (*e).to_string()).collect())
    }
    async fn mark_read(&self, peer: PeerRef, max_id: MsgId) -> BackendResult<()>;
    async fn set_typing(&self, peer: PeerRef, typing: bool) -> BackendResult<()>;
    /// Fija o desfija un mensaje del chat (`messages.updatePinnedMessage`). Por defecto, no soportado.
    async fn pin_message(&self, peer: PeerRef, id: MsgId, pinned: bool) -> BackendResult<()> {
        let _ = (peer, id, pinned);
        Err(unsupported())
    }
    /// Busca mensajes dentro de un chat (ADR-0012), del más reciente al más antiguo. En un
    /// `ThreadScope` se limita al tema. Por defecto, no soportado.
    async fn search_in_chat(&self, peer: PeerRef, query: crate::search::SearchQuery) -> BackendResult<crate::search::SearchPage> {
        let _ = (peer, query);
        Err(unsupported())
    }
}

#[async_trait]
pub trait MediaApi: Send + Sync {
    /// Envía fotos/archivos (un mensaje por elemento; la leyenda va en el primero).
    ///
    /// Progreso y cancelación (ADR-0011; opcional: un backend que no lo soporte no emite nada y la
    /// UI muestra progreso indeterminado). Toda la llamada es **una** transferencia con id
    /// `TransferId::Upload(media.random_ids[0])` (ver `TransferId::upload_of`), `total` = suma de
    /// `size_bytes` y `name` = nombre del primer elemento: `TransferProgress` con `Running` y `done`
    /// creciente; al final `Done` (después de los `MessageSent`), `Failed(e)` (y la llamada devuelve
    /// `Err(e)`) o `Cancelled` si se llamó a `cancel_transfer` con ese id (y la llamada devuelve
    /// `Err(BackendError::Cancelled)`; lo ya confirmado con `MessageSent` queda enviado). Los errores de
    /// validación previos (lista vacía, chat de solo lectura) se devuelven sin emitir ningún evento.
    async fn send_media(&self, peer: PeerRef, media: OutgoingMedia) -> BackendResult<Vec<Message>>;
    /// Descarga un medio del mensaje `msg` de `peer` **a disco**, en `dest`, por trozos y sin cargarlo
    /// entero en memoria (ADR-0011). Vuelve en cuanto la transferencia arranca, con su id; el resto
    /// llega como `BackendEvent::TransferProgress` (`Running` con `done` creciente y un único estado
    /// terminal). Escribe en `partial_path(dest)` (`<dest>.part`, mismo directorio) y al terminar bien
    /// lo renombra a `dest` **reemplazando** lo que hubiera (quien llama elige `dest` con
    /// `unique_path` o tras confirmar «Guardar como…»); `Done` se emite con el archivo ya en su sitio.
    /// Si falla o se cancela, borra el parcial y no toca `dest`.
    ///
    /// Errores inmediatos (sin eventos): `NotFound` si el mensaje o el archivo no existen,
    /// `InvalidInput("dest")` si el directorio de `dest` no existe o `dest` es un directorio, `Network`
    /// sin conexión. `msg = MsgId(0)` como en `download_media`. Por defecto, no soportado.
    async fn download_media_to(
        &self,
        peer: PeerRef,
        msg: MsgId,
        file: FileRef,
        variant: MediaVariant,
        dest: std::path::PathBuf,
    ) -> BackendResult<crate::transfer::TransferId> {
        let _ = (peer, msg, file, variant, dest);
        Err(unsupported())
    }
    /// Pide cancelar una transferencia (subida o descarga). Idempotente: si ya terminó o no se conoce,
    /// devuelve `Ok(())` sin hacer nada. La cancelación se confirma con `TransferState::Cancelled` (o,
    /// si llegó tarde, la transferencia acaba igualmente en `Done`). Por defecto, no soportado.
    async fn cancel_transfer(&self, id: crate::transfer::TransferId) -> BackendResult<()> {
        let _ = id;
        Err(unsupported())
    }
    /// Bytes de la foto de perfil `photo` del chat `peer` (usuario, grupo, canal; en un chat secreto,
    /// la del otro usuario). Formato de imagen tal cual lo da la red (JPEG en Telegram, PNG en el mock).
    async fn download_profile_photo(&self, peer: PeerRef, photo: ProfilePhoto, size: PhotoSize) -> BackendResult<Vec<u8>>;
    /// Bytes de un medio del mensaje `msg` de `peer` (la vista previa o el archivo completo). `msg`
    /// permite al backend refrescar una `file_reference` caducada. Los archivos grandes pueden emitir
    /// `BackendEvent::Download` con el progreso. Para los stickers de un pack (sin mensaje) se pasa
    /// `msg = MsgId(0)`: la referencia se refresca volviendo a pedir el pack.
    async fn download_media(&self, peer: PeerRef, msg: MsgId, file: FileRef, variant: MediaVariant) -> BackendResult<Vec<u8>>;
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
    /// Stickers usados hace poco en la cuenta (más reciente primero). Por defecto, ninguno.
    async fn recent_stickers(&self) -> BackendResult<Vec<Sticker>> {
        Ok(Vec::new())
    }
    /// Stickers favoritos de la cuenta. Por defecto, ninguno.
    async fn faved_stickers(&self) -> BackendResult<Vec<Sticker>> {
        Ok(Vec::new())
    }

    // --- GIFs y emojis del panel del compositor (ADR-0009, ADR-0010). Todos con valor por defecto
    // para que los backends que no los tienen sigan compilando.

    /// GIFs guardados de la cuenta (más reciente primero). Por defecto, ninguno.
    async fn saved_gifs(&self) -> BackendResult<Vec<crate::gif::Gif>> {
        Ok(Vec::new())
    }
    /// Busca GIFs (en Telegram, con el bot inline `@gif`). `offset` vacío = primera página.
    async fn search_gifs(&self, query: String, offset: String) -> BackendResult<crate::gif::GifPage> {
        let _ = (query, offset);
        Ok(crate::gif::GifPage::default())
    }
    /// Envía un GIF (documento, o resultado del bot si viene de una búsqueda).
    async fn send_gif(&self, peer: PeerRef, gif: crate::gif::Gif, reply_to: Option<MsgId>, random_id: RandomId) -> BackendResult<Message> {
        let _ = (peer, gif, reply_to, random_id);
        Err(crate::error::BackendError::NotFound)
    }
    /// Uso local de emojis del panel (recientes y frecuentes). Por defecto no se guarda nada.
    async fn emoji_usage(&self) -> BackendResult<Vec<crate::gif::EmojiUse>> {
        Ok(Vec::new())
    }
    /// Suma un uso de `emoji` en el instante `at`.
    async fn record_emoji_use(&self, emoji: String, at: Timestamp) -> BackendResult<()> {
        let _ = (emoji, at);
        Ok(())
    }
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
    ProfileApi + DialogsApi + MessagesApi + MediaApi + StickersApi + SecretChatsApi + crate::forum::ForumApi + Send + Sync + 'static
{
    /// Flujo de eventos en tiempo real, en orden de llegada. Cada llamada crea un
    /// suscriptor nuevo que solo ve eventos posteriores a la suscripción.
    fn events(&self) -> BoxStream<'static, BackendEvent>;
}
