//! Entidades de dominio (datos puros).
use crate::ids::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UserStatus {
    Online,
    /// Visto por última vez en ese instante.
    LastSeen(Timestamp),
    Recently,
    Offline,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct User {
    pub id: UserId,
    pub first_name: String,
    pub last_name: String,
    pub username: Option<String>,
    pub status: UserStatus,
    pub is_contact: bool,
    pub is_bot: bool,
    /// Foto de perfil actual (`None`: sin foto o desconocida).
    pub photo: Option<ProfilePhoto>,
}

impl User {
    pub fn display_name(&self) -> String {
        if self.last_name.is_empty() {
            self.first_name.clone()
        } else {
            format!("{} {}", self.first_name, self.last_name)
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GroupKind {
    Basic,
    Super,
    Channel,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Group {
    pub id: GroupId,
    pub kind: GroupKind,
    pub title: String,
    pub username: Option<String>,
    pub member_count: u32,
    pub photo: Option<ProfilePhoto>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EntityKind {
    Bold,
    Italic,
    Code,
    Pre,
    Strike,
    Underline,
    Spoiler,
    Mention,
    Url,
    Hashtag,
}

/// Formato sobre el texto; `offset` y `length` en caracteres (no bytes).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TextEntity {
    pub kind: EntityKind,
    pub offset: u32,
    pub length: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MediaKind {
    Photo,
    Video,
    Gif,
    /// Nota de voz (OGG/Opus con forma de onda).
    Voice,
    File,
    /// Audio que no es nota de voz (música, podcasts): título/intérprete en `Media::audio`.
    Audio,
    /// Mensaje de vídeo redondo.
    VideoNote,
}

/// Datos de un audio (notas de voz y música) para dibujarlo y reproducirlo.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct AudioInfo {
    pub voice: bool,
    pub title: Option<String>,
    pub performer: Option<String>,
    /// Forma de onda ya decodificada: un valor 0..=31 por barra (vacía si el archivo no la trae).
    pub waveform: Vec<u8>,
    /// Tipo MIME del archivo (pista para el decodificador), p. ej. `audio/ogg`.
    pub mime: Option<String>,
}

/// Foto de perfil de un usuario, grupo o canal. `id` cambia cuando cambia la foto, así que
/// sirve de clave de caché. Se descarga con `MediaApi::download_profile_photo`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ProfilePhoto {
    pub id: i64,
    /// Centro de datos que guarda el archivo (dato del backend).
    pub dc_id: i32,
}

/// Tamaño de una foto de perfil: pequeña (~160 px) o grande (~640 px).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PhotoSize {
    Small,
    Big,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FileKind {
    Photo,
    Document,
}

/// Qué bytes de un medio se piden: la vista previa (miniatura adecuada para la burbuja) o el archivo
/// completo (la foto mayor, o el documento entero).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MediaVariant {
    Thumb,
    Full,
}

/// Referencia opaca a un archivo descargable de un mensaje. Solo la interpreta el backend que la
/// creó; la UI la devuelve tal cual a `MediaApi::download_media`.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct FileRef {
    pub kind: FileKind,
    pub id: i64,
    pub access_hash: i64,
    /// Caduca con el tiempo: el backend la refresca volviendo a pedir el mensaje.
    pub file_reference: Vec<u8>,
    pub dc_id: i32,
    /// Bytes del archivo completo (en fotos, de la variante mayor).
    pub size_bytes: u64,
    /// Variante de vista previa en el backend (p. ej. `"x"`). `None`: no hay; en fotos se usa `full`.
    pub thumb: Option<String>,
    /// Variante completa. En fotos, el tipo del tamaño mayor; en documentos, `None` (el archivo entero).
    pub full: Option<String>,
}

impl FileRef {
    /// ¿Hay algo que mostrar como imagen para esa variante?
    pub fn has_image(&self, v: MediaVariant) -> bool {
        match (self.kind, v) {
            (FileKind::Photo, _) => true,
            (FileKind::Document, MediaVariant::Thumb) => self.thumb.is_some(),
            (FileKind::Document, MediaVariant::Full) => false,
        }
    }
}

/// Medio adjunto. En el mock `local_path` es `None` (sin bytes reales); `file` permite descargarlo.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Media {
    pub id: i64,
    pub kind: MediaKind,
    pub file_name: Option<String>,
    pub size_bytes: u64,
    pub width: u32,
    pub height: u32,
    pub duration_secs: Option<u32>,
    pub local_path: Option<String>,
    /// Referencia para descargarlo (`None`: no descargable, p. ej. un envío local aún sin confirmar).
    pub file: Option<FileRef>,
    /// Notas de voz y audios: forma de onda, título, intérprete y MIME. `None` en el resto.
    pub audio: Option<AudioInfo>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StickerFormat {
    Webp,
    Lottie,
    Webm,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Sticker {
    pub id: StickerId,
    pub set_id: StickerSetId,
    pub emoji: String,
    pub width: u32,
    pub height: u32,
    pub format: StickerFormat,
    /// Documento del sticker para descargarlo con `MediaApi::download_media` (`Full` = el archivo:
    /// WebP, TGS o WebM; `Thumb` = su miniatura WebP estática, si la tiene). `None` en los stickers
    /// propios dibujados por código (pack de Parlo).
    pub file: Option<FileRef>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StickerSet {
    pub id: StickerSetId,
    pub short_name: String,
    pub title: String,
    pub stickers: Vec<Sticker>,
}

/// Reacciones que se ofrecen cuando el backend no sabe más: el principio del conjunto gratuito de
/// Telegram (sin `U+FE0F` en el corazón: la red usa «❤» a secas y rechaza «❤️»).
pub const DEFAULT_REACTIONS: [&str; 6] = ["👍", "❤", "🔥", "🥰", "👏", "😁"];

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReactionCount {
    pub emoji: String,
    pub count: u32,
    /// Si la he puesto yo.
    pub mine: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SendState {
    Pending,
    #[default]
    Sent,
    Failed,
}

/// Mensaje. `sender` es `None` en canales (mensajes de la propia entidad).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Message {
    pub id: MsgId,
    pub peer: PeerRef,
    pub sender: Option<UserId>,
    pub date: Timestamp,
    pub edit_date: Option<Timestamp>,
    pub outgoing: bool,
    pub text: String,
    pub entities: Vec<TextEntity>,
    pub reply_to: Option<MsgId>,
    pub forwarded_from: Option<String>,
    pub media: Option<Media>,
    pub sticker: Option<Sticker>,
    pub reactions: Vec<ReactionCount>,
    pub random_id: Option<RandomId>,
    pub state: SendState,
    /// Autodestrucción en segundos (solo chats secretos).
    pub ttl_secs: Option<u32>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Draft {
    pub text: String,
    pub reply_to: Option<MsgId>,
}

/// Fila de la lista de chats.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Dialog {
    pub peer: PeerRef,
    pub title: String,
    pub last_message: Option<Message>,
    pub unread_count: u32,
    pub unread_mentions: u32,
    /// Posición entre los fijados (0 = primero); `None` si no está fijado.
    pub pinned_order: Option<u32>,
    pub archived: bool,
    pub muted: bool,
    pub draft: Option<Draft>,
    /// Instante del último movimiento, para ordenar.
    pub activity: Timestamp,
    /// Id del último mensaje saliente leído por el otro lado.
    pub read_outbox_max_id: MsgId,
    /// Foto del chat (la del usuario, grupo o canal; en chats secretos, la del otro usuario).
    pub photo: Option<ProfilePhoto>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SecretChatState {
    Waiting,
    Accepted,
    Discarded,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SecretChat {
    pub id: SecretChatId,
    pub user: UserId,
    pub state: SecretChatState,
    /// Lo inicié yo.
    pub outgoing: bool,
    pub ttl_secs: Option<u32>,
}

/// Huella visual de la clave. En el mock es simulada (no deriva de una clave real).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KeyVisual {
    pub fingerprint: String,
    pub emoji: Vec<String>,
}

// ---- Entradas de operaciones ----

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DialogsPage {
    pub offset: usize,
    pub limit: usize,
    pub archived: bool,
}

/// Punto de partida de una página de historial. El resultado va siempre en orden ascendente de id.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Anchor {
    /// Los últimos mensajes.
    Latest,
    /// Anteriores a este id.
    Before(MsgId),
    /// Posteriores a este id.
    After(MsgId),
    /// Alrededor de este id (incluido).
    Around(MsgId),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OutgoingText {
    pub text: String,
    pub entities: Vec<TextEntity>,
    pub reply_to: Option<MsgId>,
    pub random_id: RandomId,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OutgoingMediaItem {
    pub kind: MediaKind,
    pub path: String,
    pub file_name: String,
    pub size_bytes: u64,
    pub width: u32,
    pub height: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OutgoingMedia {
    pub items: Vec<OutgoingMediaItem>,
    pub caption: String,
    pub reply_to: Option<MsgId>,
    /// Un `RandomId` por elemento, en el mismo orden.
    pub random_ids: Vec<RandomId>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn display_name_joins_parts() {
        let mut u = User {
            id: UserId(1),
            first_name: "Ana".into(),
            last_name: "Gil".into(),
            username: None,
            status: UserStatus::Offline,
            is_contact: false,
            is_bot: false,
            photo: None,
        };
        assert_eq!(u.display_name(), "Ana Gil");
        u.last_name.clear();
        assert_eq!(u.display_name(), "Ana");
    }

    #[test]
    fn file_ref_knows_what_it_can_show() {
        let mut f = FileRef {
            kind: FileKind::Photo,
            id: 1,
            access_hash: 2,
            file_reference: vec![3],
            dc_id: 4,
            size_bytes: 5,
            thumb: Some("x".into()),
            full: Some("y".into()),
        };
        assert!(f.has_image(MediaVariant::Thumb) && f.has_image(MediaVariant::Full));
        f.kind = FileKind::Document;
        assert!(f.has_image(MediaVariant::Thumb) && !f.has_image(MediaVariant::Full));
        f.thumb = None;
        assert!(!f.has_image(MediaVariant::Thumb));
    }

    #[test]
    fn peers_of_distinct_kinds_never_collide() {
        assert_ne!(PeerRef::User(UserId(1)), PeerRef::Group(GroupId(1)));
        assert_ne!(PeerRef::Group(GroupId(1)), PeerRef::Secret(SecretChatId(1)));
        assert_eq!(SendState::default(), SendState::Sent);
    }
}
