//! Traducción TL → dominio (y dominio → TL para entidades). Funciones puras, probadas en
//! `tests/mapping.rs` con objetos TL construidos a mano.
use grammers_tl_types as tl;
use tc_core::*;

use crate::ids::{peer_from_tl, user_of};
use crate::peers::PeerStore;

// ---------- Entidades: Telegram cuenta en unidades UTF-16; el dominio, en caracteres ----------

/// Para cada carácter, su desplazamiento UTF-16 inicial; con un elemento final = longitud total.
pub(crate) fn utf16_starts(text: &str) -> Vec<u32> {
    let mut v = Vec::with_capacity(text.len() + 1);
    let mut acc = 0u32;
    for c in text.chars() {
        v.push(acc);
        acc += c.len_utf16() as u32;
    }
    v.push(acc);
    v
}

/// Convierte (offset, length) UTF-16 a caracteres; `None` si cae fuera del texto.
pub(crate) fn utf16_to_chars(starts: &[u32], offset: i32, length: i32) -> Option<(u32, u32)> {
    if offset < 0 || length <= 0 {
        return None;
    }
    let (o, end) = (offset as u32, (offset + length) as u32);
    let s = starts.partition_point(|&x| x < o);
    let e = starts.partition_point(|&x| x < end);
    if s >= starts.len() || s >= e {
        return None;
    }
    Some((s as u32, (e.min(starts.len() - 1) - s) as u32))
}

fn entity_parts(e: &tl::enums::MessageEntity) -> Option<(EntityKind, i32, i32)> {
    use tl::enums::MessageEntity as E;
    Some(match e {
        E::Bold(x) => (EntityKind::Bold, x.offset, x.length),
        E::Italic(x) => (EntityKind::Italic, x.offset, x.length),
        E::Code(x) => (EntityKind::Code, x.offset, x.length),
        E::Pre(x) => (EntityKind::Pre, x.offset, x.length),
        E::Strike(x) => (EntityKind::Strike, x.offset, x.length),
        E::Underline(x) => (EntityKind::Underline, x.offset, x.length),
        E::Spoiler(x) => (EntityKind::Spoiler, x.offset, x.length),
        E::Mention(x) => (EntityKind::Mention, x.offset, x.length),
        E::MentionName(x) => (EntityKind::Mention, x.offset, x.length),
        E::Url(x) => (EntityKind::Url, x.offset, x.length),
        E::TextUrl(x) => (EntityKind::Url, x.offset, x.length),
        E::Email(x) => (EntityKind::Url, x.offset, x.length),
        E::Hashtag(x) => (EntityKind::Hashtag, x.offset, x.length),
        E::Cashtag(x) => (EntityKind::Hashtag, x.offset, x.length),
        _ => return None,
    })
}

pub fn entities_from_tl(text: &str, ents: Option<&Vec<tl::enums::MessageEntity>>) -> Vec<TextEntity> {
    let Some(ents) = ents else { return Vec::new() };
    let starts = utf16_starts(text);
    ents.iter()
        .filter_map(|e| {
            let (kind, o, l) = entity_parts(e)?;
            let (offset, length) = utf16_to_chars(&starts, o, l)?;
            Some(TextEntity { kind, offset, length })
        })
        .collect()
}

pub fn entities_to_tl(text: &str, ents: &[TextEntity]) -> Option<Vec<tl::enums::MessageEntity>> {
    if ents.is_empty() {
        return None;
    }
    let starts = utf16_starts(text);
    let n = starts.len() - 1;
    let out: Vec<tl::enums::MessageEntity> = ents
        .iter()
        .filter_map(|e| {
            let s = e.offset as usize;
            let end = (e.offset + e.length) as usize;
            if e.length == 0 || s >= n || end > n {
                return None;
            }
            let offset = starts[s] as i32;
            let length = (starts[end] - starts[s]) as i32;
            Some(match e.kind {
                EntityKind::Bold => tl::types::MessageEntityBold { offset, length }.into(),
                EntityKind::Italic => tl::types::MessageEntityItalic { offset, length }.into(),
                EntityKind::Code => tl::types::MessageEntityCode { offset, length }.into(),
                EntityKind::Pre => tl::types::MessageEntityPre { offset, length, language: String::new() }.into(),
                EntityKind::Strike => tl::types::MessageEntityStrike { offset, length }.into(),
                EntityKind::Underline => tl::types::MessageEntityUnderline { offset, length }.into(),
                EntityKind::Spoiler => tl::types::MessageEntitySpoiler { offset, length }.into(),
                EntityKind::Mention => tl::types::MessageEntityMention { offset, length }.into(),
                EntityKind::Url => tl::types::MessageEntityUrl { offset, length }.into(),
                EntityKind::Hashtag => tl::types::MessageEntityHashtag { offset, length }.into(),
            })
        })
        .collect();
    if out.is_empty() { None } else { Some(out) }
}

// ---------- Medios y stickers ----------

fn photo_dims(sizes: &[tl::enums::PhotoSize]) -> (u32, u32, u64) {
    // El tamaño mayor suele ir al final.
    sizes.iter().fold((0, 0, 0), |acc, s| {
        let (w, h, bytes) = match s {
            tl::enums::PhotoSize::Size(s) => (s.w, s.h, s.size as i64),
            tl::enums::PhotoSize::Progressive(s) => (s.w, s.h, s.sizes.iter().copied().max().unwrap_or(0) as i64),
            tl::enums::PhotoSize::PhotoCachedSize(s) => (s.w, s.h, s.bytes.len() as i64),
            _ => return acc,
        };
        let (w, h) = (w.max(0) as u32, h.max(0) as u32);
        if w * h >= acc.0 * acc.1 { (w, h, bytes.max(0) as u64) } else { acc }
    })
}

/// `(tipo, ancho, alto, bytes)` de un tamaño descargable (sin las miniaturas incrustadas «i»/«j»).
fn size_info(s: &tl::enums::PhotoSize) -> Option<(&str, i32, i32, i64)> {
    match s {
        tl::enums::PhotoSize::Size(s) => Some((s.r#type.as_str(), s.w, s.h, i64::from(s.size))),
        tl::enums::PhotoSize::Progressive(s) => Some((s.r#type.as_str(), s.w, s.h, i64::from(s.sizes.iter().copied().max().unwrap_or(0)))),
        tl::enums::PhotoSize::PhotoCachedSize(s) => Some((s.r#type.as_str(), s.w, s.h, s.bytes.len() as i64)),
        _ => None,
    }
}

/// Lado máximo de la vista previa: cabe en la burbuja (320 px lógicos) con pantallas HiDPI.
pub(crate) const THUMB_MAX_SIDE: i32 = 800;

/// Variantes de una foto: `(vista previa, completa, bytes de la completa)`. La vista previa es el
/// tamaño mayor que no pasa de `THUMB_MAX_SIDE` (o el menor si todos pasan); la completa, el mayor.
pub(crate) fn pick_sizes(sizes: &[tl::enums::PhotoSize]) -> (Option<String>, Option<String>, u64) {
    let all: Vec<(&str, i32, i32, i64)> = sizes.iter().filter_map(size_info).filter(|s| s.1 > 0 && s.2 > 0).collect();
    let area = |s: &(&str, i32, i32, i64)| i64::from(s.1) * i64::from(s.2);
    let full = all.iter().max_by_key(|s| area(s));
    let thumb = all
        .iter()
        .filter(|s| s.1.max(s.2) <= THUMB_MAX_SIDE)
        .max_by_key(|s| area(s))
        .or_else(|| all.iter().min_by_key(|s| area(s)));
    (thumb.map(|s| s.0.to_string()), full.map(|s| s.0.to_string()), full.map_or(0, |s| s.3.max(0) as u64))
}

/// Referencia de descarga de una foto.
pub fn photo_file_ref(ph: &tl::types::Photo) -> FileRef {
    let (thumb, full, size) = pick_sizes(&ph.sizes);
    FileRef {
        kind: FileKind::Photo,
        id: ph.id,
        access_hash: ph.access_hash,
        file_reference: ph.file_reference.clone(),
        dc_id: ph.dc_id,
        size_bytes: size,
        thumb,
        full,
    }
}

/// Referencia de descarga de un documento (vista previa = su miniatura, si tiene).
pub fn document_file_ref(d: &tl::types::Document) -> FileRef {
    let (thumb, _, _) = d.thumbs.as_deref().map_or((None, None, 0), pick_sizes);
    FileRef {
        kind: FileKind::Document,
        id: d.id,
        access_hash: d.access_hash,
        file_reference: d.file_reference.clone(),
        dc_id: d.dc_id,
        size_bytes: d.size.max(0) as u64,
        thumb,
        full: None,
    }
}

/// Sticker si el documento lo es.
pub fn sticker_from_document(d: &tl::types::Document) -> Option<Sticker> {
    let mut sticker = None;
    let (mut w, mut h) = (0u32, 0u32);
    for a in &d.attributes {
        match a {
            tl::enums::DocumentAttribute::Sticker(s) => {
                let set_id = match &s.stickerset {
                    tl::enums::InputStickerSet::Id(x) => x.id,
                    _ => 0,
                };
                sticker = Some((s.alt.clone(), set_id));
            }
            tl::enums::DocumentAttribute::ImageSize(s) => (w, h) = (s.w.max(0) as u32, s.h.max(0) as u32),
            tl::enums::DocumentAttribute::Video(v) => (w, h) = (v.w.max(0) as u32, v.h.max(0) as u32),
            _ => {}
        }
    }
    let (emoji, set_id) = sticker?;
    let format = match d.mime_type.as_str() {
        "application/x-tgsticker" => StickerFormat::Lottie,
        "video/webm" => StickerFormat::Webm,
        _ => StickerFormat::Webp,
    };
    Some(Sticker { id: StickerId(d.id), set_id: StickerSetId(set_id), emoji, width: w, height: h, format })
}

pub fn media_from_document(d: &tl::types::Document, voice_flag: bool, video_flag: bool) -> Media {
    let mut kind = MediaKind::File;
    let (mut w, mut h, mut dur) = (0u32, 0u32, None);
    let mut file_name = None;
    for a in &d.attributes {
        match a {
            tl::enums::DocumentAttribute::Filename(f) => file_name = Some(f.file_name.clone()),
            tl::enums::DocumentAttribute::ImageSize(s) => (w, h) = (s.w.max(0) as u32, s.h.max(0) as u32),
            tl::enums::DocumentAttribute::Video(v) => {
                (w, h) = (v.w.max(0) as u32, v.h.max(0) as u32);
                dur = Some(v.duration.max(0.0).round() as u32);
                if kind == MediaKind::File {
                    kind = MediaKind::Video;
                }
            }
            tl::enums::DocumentAttribute::Audio(a) => {
                dur = Some(a.duration.max(0) as u32);
                if a.voice {
                    kind = MediaKind::Voice;
                }
            }
            tl::enums::DocumentAttribute::Animated => kind = MediaKind::Gif,
            _ => {}
        }
    }
    if voice_flag {
        kind = MediaKind::Voice;
    } else if video_flag && kind == MediaKind::File {
        kind = MediaKind::Video;
    }
    Media {
        id: d.id,
        kind,
        file_name,
        size_bytes: d.size.max(0) as u64,
        width: w,
        height: h,
        duration_secs: dur,
        local_path: None,
        file: Some(document_file_ref(d)),
    }
}

/// `(media, sticker)` de un `MessageMedia`. Lo no soportado (encuestas, ubicación…) da `(None, None)`.
pub fn media_from_tl(m: &tl::enums::MessageMedia) -> (Option<Media>, Option<Sticker>) {
    match m {
        tl::enums::MessageMedia::Photo(p) => match &p.photo {
            Some(tl::enums::Photo::Photo(ph)) => {
                let (w, h, size) = photo_dims(&ph.sizes);
                (
                    Some(Media {
                        id: ph.id,
                        kind: MediaKind::Photo,
                        file_name: None,
                        size_bytes: size,
                        width: w,
                        height: h,
                        duration_secs: None,
                        local_path: None,
                        file: Some(photo_file_ref(ph)),
                    }),
                    None,
                )
            }
            _ => (None, None),
        },
        tl::enums::MessageMedia::Document(d) => match &d.document {
            Some(tl::enums::Document::Document(doc)) => match sticker_from_document(doc) {
                Some(s) => (None, Some(s)),
                None => (Some(media_from_document(doc, d.voice, d.video || d.round)), None),
            },
            _ => (None, None),
        },
        _ => (None, None),
    }
}

// ---------- Reacciones ----------

pub fn reactions_from_tl(r: Option<&tl::enums::MessageReactions>) -> Vec<ReactionCount> {
    let Some(tl::enums::MessageReactions::Reactions(r)) = r else { return Vec::new() };
    r.results
        .iter()
        .filter_map(|c| {
            let tl::enums::ReactionCount::Count(c) = c;
            let emoji = match &c.reaction {
                tl::enums::Reaction::Emoji(e) => e.emoticon.clone(),
                // El dominio solo tiene emoji normales; los personalizados se identifican por id.
                tl::enums::Reaction::CustomEmoji(e) => format!("custom:{}", e.document_id),
                _ => return None,
            };
            Some(ReactionCount { emoji, count: c.count.max(0) as u32, mine: c.chosen_order.is_some() })
        })
        .collect()
}

// ---------- Mensajes ----------

/// Texto propio para los mensajes de servicio más comunes.
pub fn service_text(a: &tl::enums::MessageAction, peers: &PeerStore) -> String {
    use tl::enums::MessageAction as A;
    let name = |id: i64| peers.users.get(&UserId(id)).map_or_else(|| "alguien".to_string(), User::display_name);
    match a {
        A::ChatCreate(c) => format!("Grupo «{}» creado", c.title),
        A::ChannelCreate(c) => format!("Canal «{}» creado", c.title),
        A::ChatEditTitle(c) => format!("Nuevo nombre: «{}»", c.title),
        A::ChatEditPhoto(_) => "Foto del grupo actualizada".into(),
        A::ChatDeletePhoto => "Foto del grupo quitada".into(),
        A::ChatAddUser(u) => {
            let names: Vec<String> = u.users.iter().map(|&i| name(i)).collect();
            format!("Se unió: {}", names.join(", "))
        }
        A::ChatDeleteUser(u) => format!("Salió: {}", name(u.user_id)),
        A::ChatJoinedByLink(_) | A::ChatJoinedByRequest => "Alguien entró con un enlace".into(),
        A::PinMessage => "Mensaje fijado".into(),
        A::HistoryClear => "Historial borrado".into(),
        A::ScreenshotTaken => "Captura de pantalla".into(),
        A::ContactSignUp => "Se ha unido a la red".into(),
        A::PhoneCall(_) => "Llamada".into(),
        A::SetMessagesTtl(_) => "Temporizador de borrado cambiado".into(),
        A::ChatMigrateTo(_) | A::ChannelMigrateFrom(_) => "El grupo pasó a supergrupo".into(),
        _ => "Acción en el chat".into(),
    }
}

/// Mensaje TL → dominio. `me` decide el remitente de los salientes.
pub fn message_from_tl(m: &tl::enums::Message, me: Option<UserId>, peers: &PeerStore) -> Option<Message> {
    match m {
        tl::enums::Message::Empty(_) => None,
        tl::enums::Message::Message(m) => {
            let peer = peer_from_tl(&m.peer_id);
            let sender = sender_of(m.from_id.as_ref(), peer, m.out, me);
            // En «Mensajes guardados» Telegram no siempre marca `out`: si el remitente soy yo, es saliente.
            let outgoing = m.out || (me.is_some() && sender == me);
            let (media, sticker) = m.media.as_ref().map_or((None, None), media_from_tl);
            Some(Message {
                id: MsgId(m.id),
                peer,
                sender,
                date: Timestamp(m.date.into()),
                edit_date: if m.edit_hide { None } else { m.edit_date.map(|d| Timestamp(d.into())) },
                outgoing,
                text: m.message.clone(),
                entities: entities_from_tl(&m.message, m.entities.as_ref()),
                reply_to: reply_id(m.reply_to.as_ref()),
                forwarded_from: m.fwd_from.as_ref().and_then(|f| forward_name(f, peers)),
                media,
                sticker,
                reactions: reactions_from_tl(m.reactions.as_ref()),
                random_id: None,
                state: SendState::Sent,
                ttl_secs: None,
            })
        }
        tl::enums::Message::Service(m) => {
            let peer = peer_from_tl(&m.peer_id);
            Some(Message {
                id: MsgId(m.id),
                peer,
                sender: sender_of(m.from_id.as_ref(), peer, m.out, me),
                date: Timestamp(m.date.into()),
                edit_date: None,
                outgoing: m.out,
                text: service_text(&m.action, peers),
                entities: vec![],
                reply_to: reply_id(m.reply_to.as_ref()),
                forwarded_from: None,
                media: None,
                sticker: None,
                reactions: reactions_from_tl(m.reactions.as_ref()),
                random_id: None,
                state: SendState::Sent,
                ttl_secs: None,
            })
        }
    }
}

fn sender_of(from: Option<&tl::enums::Peer>, peer: PeerRef, out: bool, me: Option<UserId>) -> Option<UserId> {
    if let Some(f) = from {
        // Un remitente que es un canal (mensajes «en nombre del canal») no es un usuario.
        return user_of(f);
    }
    if out {
        return me;
    }
    // Sin `from_id`: en privados es el otro; en canales, la propia entidad (sin usuario).
    match peer {
        PeerRef::User(u) => Some(u),
        _ => None,
    }
}

fn reply_id(r: Option<&tl::enums::MessageReplyHeader>) -> Option<MsgId> {
    match r {
        Some(tl::enums::MessageReplyHeader::Header(h)) => h.reply_to_msg_id.map(MsgId),
        _ => None,
    }
}

fn forward_name(f: &tl::enums::MessageFwdHeader, peers: &PeerStore) -> Option<String> {
    let tl::enums::MessageFwdHeader::Header(f) = f;
    f.from_name.clone().or_else(|| f.from_id.as_ref().and_then(|p| peers.name_of(p))).or_else(|| Some("Desconocido".to_string()))
}

// ---------- Diálogos ----------

/// Contexto para construir filas de la lista.
pub struct DialogCtx<'a> {
    pub peers: &'a PeerStore,
    pub me: Option<UserId>,
    /// Instante actual (segundos UTC), para saber si un silencio sigue vigente.
    pub now: i64,
}

pub fn is_muted(n: &tl::enums::PeerNotifySettings, now: i64) -> bool {
    let tl::enums::PeerNotifySettings::Settings(n) = n;
    n.mute_until.is_some_and(|t| i64::from(t) > now)
}

pub fn draft_from_tl(d: Option<&tl::enums::DraftMessage>) -> Option<Draft> {
    match d {
        Some(tl::enums::DraftMessage::Message(d)) if !d.message.is_empty() => Some(Draft {
            text: d.message.clone(),
            reply_to: match &d.reply_to {
                Some(tl::enums::InputReplyTo::Message(r)) => Some(MsgId(r.reply_to_msg_id)),
                _ => None,
            },
        }),
        _ => None,
    }
}

fn draft_date(d: Option<&tl::enums::DraftMessage>) -> i64 {
    match d {
        Some(tl::enums::DraftMessage::Message(d)) => d.date.into(),
        _ => 0,
    }
}

/// Fila de la lista. `last` es el mensaje superior ya traducido (si vino en la respuesta).
pub fn dialog_from_tl(d: &tl::types::Dialog, last: Option<Message>, pinned_order: Option<u32>, archived: bool, ctx: &DialogCtx) -> Dialog {
    let peer = peer_from_tl(&d.peer);
    let activity = last.as_ref().map_or(0, |m| m.date.0).max(draft_date(d.draft.as_ref()));
    Dialog {
        peer,
        title: ctx.peers.title(peer, ctx.me),
        last_message: last,
        unread_count: d.unread_count.max(0) as u32,
        unread_mentions: d.unread_mentions_count.max(0) as u32,
        pinned_order: if d.pinned { pinned_order } else { None },
        archived,
        muted: is_muted(&d.notify_settings, ctx.now),
        draft: draft_from_tl(d.draft.as_ref()),
        activity: Timestamp(activity),
        read_outbox_max_id: MsgId(d.read_outbox_max_id),
        photo: ctx.peers.photo(peer, ctx.me),
    }
}

/// Fila mínima para un chat que aún no se ha cargado (llega un mensaje nuevo de él).
pub fn dialog_stub(peer: PeerRef, ctx: &DialogCtx) -> Dialog {
    Dialog {
        peer,
        title: ctx.peers.title(peer, ctx.me),
        last_message: None,
        unread_count: 0,
        unread_mentions: 0,
        pinned_order: None,
        archived: false,
        muted: false,
        draft: None,
        activity: Timestamp(0),
        read_outbox_max_id: MsgId(0),
        photo: ctx.peers.photo(peer, ctx.me),
    }
}
