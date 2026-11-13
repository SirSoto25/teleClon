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
    /// Progreso de una descarga larga (id del archivo, bytes recibidos, total conocido). Efímero.
    Download { file_id: i64, done: u64, total: u64 },
    /// El chat salió de la lista (borrado, abandonado o cerrado): se quita la fila y su historial.
    DialogRemoved { peer: PeerRef },
    /// Se vació el historial del chat (la fila sigue en la lista, sin mensajes).
    HistoryCleared { peer: PeerRef },
    /// Mensaje nuevo dentro de un tema de un grupo foro (en lugar de `MessageNew`).
    TopicMessageNew { topic: MsgId, msg: Message },
    /// Tema nuevo o cambiado (último mensaje, no leídos, título, fijado, cerrado…).
    ForumTopicUpserted(crate::forum::ForumTopic),
    /// La lista de temas del foro cambió de una forma que no se puede aplicar por partes (orden de
    /// fijados, tema borrado o desconocido): se vuelve a pedir si está a la vista.
    ForumTopicsChanged { group: GroupId },
}
