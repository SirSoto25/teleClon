//! Identificadores con tipo propio para no mezclar espacios de ids distintos.

/// Id de usuario de la red.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct UserId(pub i64);
/// Id de grupo, supergrupo o canal.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct GroupId(pub i64);
/// Id de chat secreto.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct SecretChatId(pub i32);
/// Id de mensaje, único dentro de un chat y creciente.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct MsgId(pub i32);
/// Id aleatorio del cliente que hace idempotente el envío.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct RandomId(pub i64);
/// Id de sticker.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct StickerId(pub i64);
/// Id de pack de stickers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct StickerSetId(pub i64);

/// Segundos Unix en UTC.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Default)]
pub struct Timestamp(pub i64);

/// Identidad de un chat: los tres espacios de ids son distintos.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum PeerRef {
    User(UserId),
    Group(GroupId),
    Secret(SecretChatId),
}
