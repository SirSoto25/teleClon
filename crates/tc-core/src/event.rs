//! Eventos en tiempo real que el backend entrega a la UI.
use crate::{error::BackendError, ids::*, model::*};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConnectionState {
    Connecting,
    Online,
    Updating,
    Offline,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BackendEvent {
    Connection(ConnectionState),
    /// Fila nueva o cambiada de la lista de chats.
    DialogUpserted(Dialog),
    MessageNew(Message),
    MessageEdited(Message),
    MessagesDeleted { peer: PeerRef, ids: Vec<MsgId> },
    /// Confirmación de un envío: sustituye al pendiente con el mismo `random_id`.
    MessageSent { random_id: RandomId, msg: Message },
    MessageFailed { random_id: RandomId, error: BackendError },
    ReadInbox { peer: PeerRef, max_id: MsgId },
    ReadOutbox { peer: PeerRef, max_id: MsgId },
    Typing { peer: PeerRef, user: UserId, typing: bool },
    UserStatus { user: UserId, status: UserStatus },
    Reactions { peer: PeerRef, msg_id: MsgId, reactions: Vec<ReactionCount> },
    SecretChat(SecretChat),
}
