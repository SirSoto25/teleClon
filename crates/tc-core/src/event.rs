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
    /// Progreso de una descarga larga a memoria (`download_media`: id del archivo, bytes recibidos,
    /// total conocido). Efímero. Las transferencias a disco y las subidas usan `TransferProgress`.
    Download { file_id: i64, done: u64, total: u64 },
    /// Progreso o final de una subida (`send_media`) o una descarga a disco (`download_media_to`).
    /// Los `Running` son efímeros (gana el último por id); el estado terminal llega una sola vez.
    /// Puede llegar antes de que la UI reciba el `TransferId` de `download_media_to`. ADR-0011.
    TransferProgress(crate::transfer::Transfer),
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
