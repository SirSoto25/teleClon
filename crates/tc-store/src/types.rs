//! Tipos de fila y enumeraciones de texto. Los ids son los de la red (enteros de 64 bits);
//! los tiempos son segundos Unix UTC.

use rusqlite::types::{FromSql, FromSqlError, FromSqlResult, ToSql, ToSqlOutput, ValueRef};

macro_rules! str_enum {
    ($(#[$m:meta])* $name:ident { $($(#[$vm:meta])* $var:ident => $s:literal),+ $(,)? }) => {
        $(#[$m])*
        #[derive(Debug, Clone, Copy, PartialEq, Eq)]
        pub enum $name { $($(#[$vm])* $var),+ }
        impl $name {
            /// Valor tal como se guarda en SQLite.
            pub fn as_str(self) -> &'static str {
                match self { $(Self::$var => $s),+ }
            }
            /// Inverso de [`Self::as_str`].
            pub fn parse(s: &str) -> Option<Self> {
                match s { $($s => Some(Self::$var),)+ _ => None }
            }
        }
        impl FromSql for $name {
            fn column_result(value: ValueRef<'_>) -> FromSqlResult<Self> {
                let s = value.as_str()?;
                Self::parse(s)
                    .ok_or_else(|| FromSqlError::Other(format!("valor desconocido: {s}").into()))
            }
        }
        impl ToSql for $name {
            fn to_sql(&self) -> rusqlite::Result<ToSqlOutput<'_>> {
                Ok(ToSqlOutput::from(self.as_str()))
            }
        }
    };
}

str_enum!(
    /// Tipo de interlocutor de un diálogo.
    PeerKind { User => "user", Group => "group", Secret => "secret" }
);
str_enum!(
    /// Presencia de un usuario.
    PresenceKind {
        Unknown => "unknown", Online => "online", Offline => "offline", Recently => "recently",
        LastWeek => "last_week", LastMonth => "last_month"
    }
);
str_enum!(
    /// Tipo de grupo.
    GroupKindRow { Group => "group", Supergroup => "supergroup", Channel => "channel" }
);
str_enum!(
    /// Mi papel en un grupo.
    GroupRole { Creator => "creator", Admin => "admin", Member => "member", Left => "left", Banned => "banned" }
);
str_enum!(
    /// Cómo cuenta un chat dentro de una carpeta.
    FolderPeerMode { Include => "include", Exclude => "exclude", Pinned => "pinned" }
);
str_enum!(
    /// Estado de una subida.
    UploadState {
        Queued => "queued", Uploading => "uploading", Paused => "paused", Done => "done",
        Failed => "failed", Cancelled => "cancelled"
    }
);
str_enum!(
    /// Tipo de mensaje.
    MessageKind { Text => "text", Media => "media", Sticker => "sticker", Service => "service" }
);
str_enum!(
    /// Estado de envío.
    SendState { Pending => "pending", Sent => "sent", Failed => "failed" }
);
str_enum!(
    /// Estado del protocolo de un chat secreto.
    SecretState {
        Requested => "requested", Waiting => "waiting", Accepted => "accepted", Discarded => "discarded"
    }
);
str_enum!(
    /// Estado del cambio de clave (re-keying) de un chat secreto.
    RekeyState {
        Idle => "idle", Requested => "requested", Accepted => "accepted", Committing => "committing"
    }
);

/// Usuario conocido (campos esenciales).
#[derive(Debug, Clone, PartialEq, Default)]
pub struct UserRow {
    pub id: i64,
    pub access_hash: Option<i64>,
    pub first_name: String,
    pub last_name: String,
    pub username: Option<String>,
    pub phone: Option<String>,
    pub is_self: bool,
    pub is_contact: bool,
    pub updated_at: i64,
}

/// Fila de la lista de chats. El borrador se gestiona aparte (ver [`Draft`]).
#[derive(Debug, Clone, PartialEq)]
pub struct DialogRow {
    /// Clave local; se ignora al insertar (la devuelve `upsert_dialog`).
    pub chat_key: i64,
    pub peer_kind: PeerKind,
    pub peer_id: i64,
    pub top_message_id: Option<i64>,
    pub last_message_at: Option<i64>,
    pub unread_count: i64,
    pub unread_mentions: i64,
    pub unread_reactions: i64,
    pub read_inbox_max_id: i64,
    pub read_outbox_max_id: i64,
    /// `None` = no fijado; los fijados salen primero por este orden ascendente.
    pub pinned_order: Option<i64>,
    pub archived: bool,
    pub mute_until: i64,
    pub pinned_message_id: Option<i64>,
    pub history_complete: bool,
    pub updated_at: i64,
    /// Título visible (nombre del usuario/grupo en el momento de guardar).
    pub title: String,
}

impl DialogRow {
    /// Diálogo vacío para un interlocutor.
    pub fn new(peer_kind: PeerKind, peer_id: i64, now: i64) -> Self {
        Self {
            chat_key: 0,
            peer_kind,
            peer_id,
            top_message_id: None,
            last_message_at: None,
            unread_count: 0,
            unread_mentions: 0,
            unread_reactions: 0,
            read_inbox_max_id: 0,
            read_outbox_max_id: 0,
            pinned_order: None,
            archived: false,
            mute_until: 0,
            pinned_message_id: None,
            history_complete: false,
            updated_at: now,
            title: String::new(),
        }
    }
}

/// Borrador de un chat.
#[derive(Debug, Clone, PartialEq)]
pub struct Draft {
    pub text: String,
    /// JSON de entidades.
    pub entities: Option<String>,
    pub reply_to: Option<i64>,
    pub date: i64,
}

/// Mensaje de un chat normal. `msg_id` negativo = pendiente local.
#[derive(Debug, Clone, PartialEq)]
pub struct MessageRow {
    pub chat_key: i64,
    pub msg_id: i64,
    pub random_id: Option<i64>,
    pub sender_id: Option<i64>,
    pub date: i64,
    pub edit_date: Option<i64>,
    pub is_out: bool,
    pub kind: MessageKind,
    pub text: String,
    /// JSON `[{type, offset, length, url?}]`.
    pub entities: Option<String>,
    pub reply_to_id: Option<i64>,
    pub reply_quote: Option<String>,
    pub fwd_from_id: Option<i64>,
    pub fwd_from_name: Option<String>,
    pub fwd_date: Option<i64>,
    pub media_id: Option<i64>,
    pub sticker_id: Option<i64>,
    pub grouped_id: Option<i64>,
    pub service_action: Option<String>,
    pub is_pinned: bool,
    pub views: Option<i64>,
    pub send_state: SendState,
    /// JSON del sticker adjunto (la caché de mensajes no depende de que el pack esté guardado).
    pub sticker_json: Option<String>,
}

impl MessageRow {
    /// Mensaje de texto sencillo (el resto de campos vacíos).
    pub fn text(chat_key: i64, msg_id: i64, date: i64, is_out: bool, text: &str) -> Self {
        Self {
            chat_key,
            msg_id,
            random_id: None,
            sender_id: None,
            date,
            edit_date: None,
            is_out,
            kind: MessageKind::Text,
            text: text.to_owned(),
            entities: None,
            reply_to_id: None,
            reply_quote: None,
            fwd_from_id: None,
            fwd_from_name: None,
            fwd_date: None,
            media_id: None,
            sticker_id: None,
            grouped_id: None,
            service_action: None,
            is_pinned: false,
            views: None,
            send_state: SendState::Sent,
            sticker_json: None,
        }
    }
}

/// Resultado de búsqueda de texto.
#[derive(Debug, Clone, PartialEq)]
pub struct SearchHit {
    pub chat_key: i64,
    pub msg_id: i64,
    pub date: i64,
    /// Fragmento con las coincidencias entre «» .
    pub snippet: String,
}

/// Ancla de paginación del historial. Solo cuenta mensajes ya confirmados (`msg_id > 0`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Anchor {
    /// Los últimos mensajes.
    Latest,
    /// Los anteriores a `msg_id` (excluido).
    Before(i64),
    /// Los posteriores a `msg_id` (excluido).
    After(i64),
    /// Alrededor de `msg_id` (incluido): mitad anterior, mitad posterior.
    Around(i64),
}

/// Ámbito de ajustes de notificación.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NotifyScope {
    Users,
    Groups,
    Channels,
    /// Por chat (`chat_key` != 0).
    Chat(i64),
}

/// Ajustes de notificación.
#[derive(Debug, Clone, PartialEq)]
pub struct NotifySettings {
    pub mute_until: i64,
    pub show_previews: bool,
    pub sound: Option<String>,
    pub updated_at: i64,
}

/// Medio (campos esenciales; sirve de FK para stickers y mensajes).
#[derive(Debug, Clone, PartialEq, Default)]
pub struct MediaRow {
    pub id: i64,
    /// photo, video, document, voice, audio, sticker, gif, avatar, geo.
    pub kind: String,
    pub mime: Option<String>,
    pub size: Option<i64>,
    pub width: Option<i64>,
    pub height: Option<i64>,
    pub file_name: Option<String>,
    pub thumb_path: Option<String>,
    pub local_path: Option<String>,
    pub duration_ms: Option<i64>,
    pub updated_at: i64,
}

/// Pack de stickers.
#[derive(Debug, Clone, PartialEq)]
pub struct StickerSetRow {
    pub id: i64,
    pub access_hash: Option<i64>,
    pub short_name: String,
    pub title: String,
    /// static, animated, video, mixed.
    pub format: String,
    pub is_emoji: bool,
    pub thumb_media_id: Option<i64>,
    pub installed: bool,
    pub installed_order: Option<i64>,
    pub archived: bool,
    pub remote_hash: Option<i64>,
    pub is_local: bool,
    pub updated_at: i64,
}

/// Sticker de un pack.
#[derive(Debug, Clone, PartialEq)]
pub struct StickerRow {
    pub id: i64,
    pub set_id: i64,
    pub media_id: i64,
    pub emoji: String,
    /// webp, tgs, webm.
    pub format: String,
    pub position: i64,
}

/// Chat secreto. Nunca contiene la clave: solo `key_ref` (entrada del keyring) y la huella pública.
#[derive(Debug, Clone, PartialEq)]
pub struct SecretChatRow {
    pub id: i64,
    pub access_hash: i64,
    pub peer_user_id: i64,
    pub state: SecretState,
    pub is_originator: bool,
    pub layer: i64,
    pub in_seq_no: i64,
    pub out_seq_no: i64,
    pub ttl_seconds: i64,
    pub key_ref: Option<String>,
    pub key_fingerprint: Option<i64>,
    pub key_used_count: i64,
    pub key_created_at: Option<i64>,
    pub rekey_state: RekeyState,
    pub rekey_exchange_id: Option<i64>,
    pub created_at: i64,
    pub updated_at: i64,
}

/// Mensaje de chat secreto: el cuerpo llega ya cifrado (nonce de 12 bytes + texto cifrado).
#[derive(Debug, Clone, PartialEq)]
pub struct SecretMessageRow {
    /// rowid local; se ignora al insertar.
    pub rowid: i64,
    pub secret_chat_id: i64,
    pub random_id: i64,
    pub is_out: bool,
    pub date: i64,
    pub seq_in: Option<i64>,
    pub seq_out: Option<i64>,
    pub body_nonce: Vec<u8>,
    pub body_enc: Vec<u8>,
    pub media_id: Option<i64>,
    pub ttl_seconds: i64,
    pub read_at: Option<i64>,
    pub destroy_at: Option<i64>,
    pub send_state: SendState,
}

/// Bot y presencia de un usuario (columnas que `upsert_user` no toca).
#[derive(Debug, Clone, PartialEq)]
pub struct UserExtra {
    pub is_bot: bool,
    pub status: PresenceKind,
    /// Segundos Unix de la última conexión conocida.
    pub last_seen_at: Option<i64>,
}

/// Grupo, supergrupo o canal.
#[derive(Debug, Clone, PartialEq)]
pub struct GroupRow {
    pub id: i64,
    pub kind: GroupKindRow,
    pub access_hash: Option<i64>,
    pub title: String,
    pub about: Option<String>,
    pub username: Option<String>,
    pub participants_count: Option<i64>,
    pub my_role: GroupRole,
    pub can_send: bool,
    pub created_at: Option<i64>,
    pub updated_at: i64,
}

impl GroupRow {
    /// Grupo mínimo (miembro, puedo escribir).
    pub fn new(id: i64, kind: GroupKindRow, title: &str, now: i64) -> Self {
        Self {
            id,
            kind,
            access_hash: None,
            title: title.to_owned(),
            about: None,
            username: None,
            participants_count: None,
            my_role: GroupRole::Member,
            can_send: true,
            created_at: None,
            updated_at: now,
        }
    }
}

/// Carpeta (filtro de chats).
#[derive(Debug, Clone, PartialEq, Default)]
pub struct FolderRow {
    pub id: i64,
    pub title: String,
    pub emoticon: Option<String>,
    pub sort_order: i64,
    pub inc_contacts: bool,
    pub inc_non_contacts: bool,
    pub inc_groups: bool,
    pub inc_channels: bool,
    pub inc_bots: bool,
    pub exc_muted: bool,
    pub exc_read: bool,
    pub exc_archived: bool,
    pub updated_at: i64,
}

/// Pertenencia explícita de un chat a una carpeta.
#[derive(Debug, Clone, PartialEq)]
pub struct FolderPeerRow {
    pub chat_key: i64,
    pub mode: FolderPeerMode,
    /// Orden de los fijados dentro de la carpeta.
    pub position: Option<i64>,
}

/// Reacción agregada de un mensaje.
#[derive(Debug, Clone, PartialEq)]
pub struct ReactionRow {
    pub emoji: String,
    pub count: i64,
    /// La he puesto yo.
    pub chosen: bool,
}

/// Subida reanudable. Exactamente uno de `chat_key` / `secret_chat_id` está presente.
#[derive(Debug, Clone, PartialEq)]
pub struct UploadRow {
    /// Se ignora al crear (la devuelve `create_upload`).
    pub local_id: i64,
    pub chat_key: Option<i64>,
    pub secret_chat_id: Option<i64>,
    pub path: String,
    pub size: i64,
    /// Id aleatorio de subida de la red.
    pub file_id: i64,
    pub part_size: i64,
    pub parts_total: i64,
    pub parts_done: i64,
    pub state: UploadState,
    pub last_error: Option<String>,
    pub created_at: i64,
    pub updated_at: i64,
}
