//! Conversión entre el dominio (`tc-core`) y las filas de la caché, y operaciones compuestas que
//! `CachedBackend` ejecuta dentro de `spawn_blocking`.
//!
//! Regla de privacidad: **nada de un chat secreto se escribe en `messages`, en el FTS ni en los
//! borradores** (esas tablas están en claro). Los diálogos secretos se guardan sin último mensaje
//! ni borrador; el contenido secreto sigue el camino cifrado de `secret.rs`.

use crate::db::Store;
use crate::error::Result;
use crate::types as db;
use serde_json::{Value, json};
use std::time::{SystemTime, UNIX_EPOCH};
use tc_core as core;
use tc_core::{GroupId, MsgId, PeerRef, RandomId, SecretChatId, StickerId, StickerSetId, Timestamp, UserId};

/// Segundos Unix actuales.
pub(crate) fn now() -> i64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map_or(0, |d| d.as_secs() as i64)
}

pub(crate) fn peer_parts(p: PeerRef) -> (db::PeerKind, i64) {
    match p {
        PeerRef::User(u) => (db::PeerKind::User, u.0),
        PeerRef::Group(g) => (db::PeerKind::Group, g.0),
        PeerRef::Secret(s) => (db::PeerKind::Secret, i64::from(s.0)),
    }
}

pub(crate) fn peer_from(kind: db::PeerKind, id: i64) -> Option<PeerRef> {
    Some(match kind {
        db::PeerKind::User => PeerRef::User(UserId(id)),
        db::PeerKind::Group => PeerRef::Group(GroupId(id)),
        db::PeerKind::Secret => PeerRef::Secret(SecretChatId(i32::try_from(id).ok()?)),
    })
}

pub(crate) fn is_secret(p: PeerRef) -> bool {
    matches!(p, PeerRef::Secret(_))
}

fn msg_id32(v: i64) -> MsgId {
    MsgId(i32::try_from(v).unwrap_or(0))
}

/// Id local negativo y estable para un envío pendiente, derivado del `random_id`.
pub(crate) fn pending_local_id(rid: RandomId) -> i64 {
    -((rid.0.unsigned_abs() % 0x7fff_fffe) as i64) - 1
}

pub(crate) fn map_anchor(a: core::Anchor) -> db::Anchor {
    match a {
        core::Anchor::Latest => db::Anchor::Latest,
        core::Anchor::Before(m) => db::Anchor::Before(i64::from(m.0)),
        core::Anchor::After(m) => db::Anchor::After(i64::from(m.0)),
        core::Anchor::Around(m) => db::Anchor::Around(i64::from(m.0)),
    }
}

// ---- entidades de texto y stickers (JSON compacto) ------------------------------------------

const ENTITY_NAMES: [(core::EntityKind, &str); 10] = [
    (core::EntityKind::Bold, "bold"),
    (core::EntityKind::Italic, "italic"),
    (core::EntityKind::Code, "code"),
    (core::EntityKind::Pre, "pre"),
    (core::EntityKind::Strike, "strike"),
    (core::EntityKind::Underline, "underline"),
    (core::EntityKind::Spoiler, "spoiler"),
    (core::EntityKind::Mention, "mention"),
    (core::EntityKind::Url, "url"),
    (core::EntityKind::Hashtag, "hashtag"),
];

fn encode_entities(e: &[core::TextEntity]) -> Option<String> {
    if e.is_empty() {
        return None;
    }
    let items: Vec<Value> = e
        .iter()
        .filter_map(|t| {
            let name = ENTITY_NAMES.iter().find(|(k, _)| *k == t.kind)?.1;
            Some(json!([name, t.offset, t.length]))
        })
        .collect();
    Some(Value::Array(items).to_string())
}

fn decode_entities(s: Option<&str>) -> Vec<core::TextEntity> {
    let Some(Value::Array(items)) = s.and_then(|s| serde_json::from_str::<Value>(s).ok()) else { return Vec::new() };
    items
        .iter()
        .filter_map(|v| {
            let a = v.as_array()?;
            let name = a.first()?.as_str()?;
            let kind = ENTITY_NAMES.iter().find(|(_, n)| *n == name)?.0;
            Some(core::TextEntity { kind, offset: u32::try_from(a.get(1)?.as_u64()?).ok()?, length: u32::try_from(a.get(2)?.as_u64()?).ok()? })
        })
        .collect()
}

fn encode_sticker(s: &core::Sticker) -> String {
    let fmt = match s.format {
        core::StickerFormat::Webp => "webp",
        core::StickerFormat::Lottie => "lottie",
        core::StickerFormat::Webm => "webm",
    };
    let mut v = json!({"id": s.id.0, "set": s.set_id.0, "emoji": s.emoji, "w": s.width, "h": s.height, "fmt": fmt});
    // Referencia de descarga del documento (los stickers no van en `media`/`media_refs`).
    if let Some(f) = &s.file {
        let hex: String = f.file_reference.iter().map(|b| format!("{b:02x}")).collect();
        v["file"] = json!({"id": f.id, "ah": f.access_hash, "fr": hex, "dc": f.dc_id, "size": f.size_bytes, "thumb": f.thumb});
    }
    v.to_string()
}

fn decode_sticker_file(v: &Value) -> Option<core::FileRef> {
    let hex = v.get("fr")?.as_str()?;
    let file_reference = (0..hex.len())
        .step_by(2)
        .map(|i| hex.get(i..i + 2).and_then(|h| u8::from_str_radix(h, 16).ok()))
        .collect::<Option<Vec<u8>>>()?;
    Some(core::FileRef {
        kind: core::FileKind::Document,
        id: v.get("id")?.as_i64()?,
        access_hash: v.get("ah")?.as_i64()?,
        file_reference,
        dc_id: i32::try_from(v.get("dc")?.as_i64()?).ok()?,
        size_bytes: v.get("size")?.as_u64()?,
        thumb: v.get("thumb").and_then(Value::as_str).map(str::to_owned),
        full: None,
    })
}

fn decode_sticker(s: &str) -> Option<core::Sticker> {
    let v: Value = serde_json::from_str(s).ok()?;
    Some(core::Sticker {
        file: v.get("file").and_then(decode_sticker_file),
        id: StickerId(v.get("id")?.as_i64()?),
        set_id: StickerSetId(v.get("set")?.as_i64()?),
        emoji: v.get("emoji")?.as_str()?.to_owned(),
        width: u32::try_from(v.get("w")?.as_u64()?).ok()?,
        height: u32::try_from(v.get("h")?.as_u64()?).ok()?,
        format: match v.get("fmt")?.as_str()? {
            "webp" => core::StickerFormat::Webp,
            "lottie" => core::StickerFormat::Lottie,
            "webm" => core::StickerFormat::Webm,
            _ => return None,
        },
    })
}

fn media_kind_str(k: core::MediaKind) -> &'static str {
    match k {
        core::MediaKind::Photo => "photo",
        core::MediaKind::Video => "video",
        core::MediaKind::Gif => "gif",
        core::MediaKind::Voice => "voice",
        core::MediaKind::File => "document",
        core::MediaKind::Audio => "audio",
        // `media.kind` no admite otro valor: `media_extra.round` lo distingue (migración 0005).
        core::MediaKind::VideoNote => "video",
    }
}

fn media_kind_parse(s: &str) -> core::MediaKind {
    match s {
        "photo" => core::MediaKind::Photo,
        "video" => core::MediaKind::Video,
        "gif" => core::MediaKind::Gif,
        "voice" => core::MediaKind::Voice,
        "audio" => core::MediaKind::Audio,
        _ => core::MediaKind::File,
    }
}

// ---- fotos de perfil y referencias de descarga ---------------------------------------------

/// Guarda la foto de perfil del interlocutor (usuarios y grupos; los chats secretos nunca).
fn persist_photo(store: &Store, kind: db::PeerKind, id: i64, photo: Option<core::ProfilePhoto>) -> Result<()> {
    if kind == db::PeerKind::Secret {
        return Ok(());
    }
    store.set_peer_photo(kind, id, photo.map(|p| (p.id, i64::from(p.dc_id))))
}

fn load_photo(store: &Store, kind: db::PeerKind, id: i64) -> Result<Option<core::ProfilePhoto>> {
    Ok(store.get_peer_photo(kind, id)?.map(|(id, dc)| core::ProfilePhoto { id, dc_id: i32::try_from(dc).unwrap_or(0) }))
}

fn file_ref_row(media_id: i64, f: &core::FileRef) -> crate::files::MediaRefRow {
    crate::files::MediaRefRow {
        media_id,
        kind: match f.kind {
            core::FileKind::Photo => "photo",
            core::FileKind::Document => "document",
        }
        .into(),
        access_hash: f.access_hash,
        file_reference: f.file_reference.clone(),
        dc_id: i64::from(f.dc_id),
        size: i64::try_from(f.size_bytes).unwrap_or(i64::MAX),
        thumb: f.thumb.clone(),
        full: f.full.clone(),
    }
}

fn file_ref_from(r: crate::files::MediaRefRow) -> core::FileRef {
    core::FileRef {
        kind: if r.kind == "photo" { core::FileKind::Photo } else { core::FileKind::Document },
        id: r.media_id,
        access_hash: r.access_hash,
        file_reference: r.file_reference,
        dc_id: i32::try_from(r.dc_id).unwrap_or(0),
        size_bytes: u64::try_from(r.size).unwrap_or(0),
        thumb: r.thumb,
        full: r.full,
    }
}

// ---- diálogos -------------------------------------------------------------------------------

/// Título provisional de un chat que aún no llegó en la lista: el de su usuario o grupo, si se conocen.
fn known_title(store: &Store, kind: db::PeerKind, id: i64) -> Result<String> {
    Ok(match kind {
        db::PeerKind::User => store
            .get_user(id)?
            .map(|u| if u.last_name.is_empty() { u.first_name } else { format!("{} {}", u.first_name, u.last_name) })
            .unwrap_or_default(),
        db::PeerKind::Group => store.get_group(id)?.map(|g| g.title).unwrap_or_default(),
        db::PeerKind::Secret => String::new(),
    })
}

/// `chat_key` del chat, creando una fila vacía si no existía.
pub(crate) fn ensure_dialog(store: &Store, peer: PeerRef) -> Result<i64> {
    let (kind, id) = peer_parts(peer);
    if let Some(d) = store.find_dialog(kind, id)? {
        return Ok(d.chat_key);
    }
    let mut row = db::DialogRow::new(kind, id, now());
    row.title = known_title(store, kind, id)?;
    store.upsert_dialog(&row)
}

pub(crate) fn find_chat_key(store: &Store, peer: PeerRef) -> Result<Option<i64>> {
    let (kind, id) = peer_parts(peer);
    Ok(store.find_dialog(kind, id)?.map(|d| d.chat_key))
}

/// Guarda una fila de la lista de chats y devuelve su `chat_key`.
pub(crate) fn persist_dialog(store: &Store, d: &core::Dialog) -> Result<i64> {
    let (kind, id) = peer_parts(d.peer);
    let old = store.find_dialog(kind, id)?;
    let secret = is_secret(d.peer);
    let mut row = old.clone().unwrap_or_else(|| db::DialogRow::new(kind, id, now()));
    row.title = d.title.clone();
    row.top_message_id = if secret { None } else { d.last_message.as_ref().map(|m| i64::from(m.id.0)).or(row.top_message_id) };
    row.last_message_at = Some(d.activity.0);
    row.unread_count = i64::from(d.unread_count);
    row.unread_mentions = i64::from(d.unread_mentions);
    row.pinned_order = d.pinned_order.map(i64::from);
    row.archived = d.archived;
    row.mute_until = match (d.muted, old.as_ref()) {
        (false, _) => 0,
        (true, Some(o)) if o.mute_until > now() => o.mute_until,
        (true, _) => i64::from(i32::MAX),
    };
    row.read_outbox_max_id = i64::from(d.read_outbox_max_id.0);
    // Solo avanza: una fila parcial (sin el dato, 0) no borra lo ya sabido.
    row.read_inbox_max_id = row.read_inbox_max_id.max(i64::from(d.read_inbox_max_id.0));
    row.updated_at = now();
    let ck = store.upsert_dialog(&row)?;
    store.set_dialog_unread_mark(ck, d.unread_mark)?;
    if !secret {
        persist_photo(store, kind, id, d.photo)?;
        let draft = d.draft.as_ref().map(|dr| db::Draft { text: dr.text.clone(), entities: None, reply_to: dr.reply_to.map(|m| i64::from(m.0)), date: now() });
        store.set_draft(ck, draft.as_ref())?;
        if let Some(m) = &d.last_message {
            persist_message(store, m, None)?;
        }
    }
    Ok(ck)
}

/// Fila → dominio. `None` si es un chat sin título resuelto (fila parcial creada por un evento) o ilegible.
pub(crate) fn load_dialog(store: &Store, row: &db::DialogRow) -> Result<Option<core::Dialog>> {
    let Some(peer) = peer_from(row.peer_kind, row.peer_id) else { return Ok(None) };
    let title = if row.title.is_empty() { known_title(store, row.peer_kind, row.peer_id)? } else { row.title.clone() };
    if title.is_empty() {
        return Ok(None);
    }
    let secret = is_secret(peer);
    let last_message = match row.top_message_id.filter(|_| !secret) {
        Some(mid) => match store.get_message(row.chat_key, mid)? {
            Some(m) => Some(load_message(store, peer, &m)?),
            None => None,
        },
        None => None,
    };
    let draft = if secret {
        None
    } else {
        store.get_draft(row.chat_key)?.map(|d| core::Draft { text: d.text, reply_to: d.reply_to.map(msg_id32) })
    };
    Ok(Some(core::Dialog {
        peer,
        title,
        last_message,
        unread_count: u32::try_from(row.unread_count).unwrap_or(0),
        unread_mentions: u32::try_from(row.unread_mentions).unwrap_or(0),
        pinned_order: row.pinned_order.and_then(|p| u32::try_from(p).ok()),
        archived: row.archived,
        muted: row.mute_until > now(),
        draft,
        activity: Timestamp(row.last_message_at.unwrap_or(row.updated_at)),
        read_outbox_max_id: msg_id32(row.read_outbox_max_id),
        read_inbox_max_id: msg_id32(row.read_inbox_max_id),
        photo: load_photo(store, row.peer_kind, row.peer_id)?,
        unread_mark: store.dialog_unread_mark(row.chat_key)?,
    }))
}

pub(crate) fn load_dialog_by_peer(store: &Store, peer: PeerRef) -> Result<Option<core::Dialog>> {
    let (kind, id) = peer_parts(peer);
    match store.find_dialog(kind, id)? {
        Some(row) => load_dialog(store, &row),
        None => Ok(None),
    }
}

/// Página de la lista de chats desde la caché.
pub(crate) fn cached_dialogs(store: &Store, page: core::DialogsPage) -> Result<Vec<core::Dialog>> {
    let mut out = Vec::new();
    for row in store.list_dialogs(page.archived, page.offset.saturating_add(page.limit))?.iter().skip(page.offset) {
        if let Some(d) = load_dialog(store, row)? {
            out.push(d);
        }
    }
    Ok(out)
}

pub(crate) fn dialogs_complete_key(archived: bool) -> String {
    format!("cache.dialogs_complete.{}", u8::from(archived))
}

/// Guarda una página recibida del backend y devuelve los diálogos que cambiaron respecto a la caché.
pub(crate) fn apply_dialogs(store: &Store, fetched: &[core::Dialog], page: core::DialogsPage) -> Result<Vec<core::Dialog>> {
    let mut changed = Vec::new();
    for d in fetched {
        let before = load_dialog_by_peer(store, d.peer)?;
        persist_dialog(store, d)?;
        if before != load_dialog_by_peer(store, d.peer)? {
            changed.push(d.clone());
        }
    }
    if fetched.len() < page.limit {
        store.set_setting(&dialogs_complete_key(page.archived), "1")?;
    }
    Ok(changed)
}

// ---- mensajes -------------------------------------------------------------------------------

/// Guarda un mensaje (con su medio, sticker y reacciones). Los de chats secretos se ignoran.
/// `fallback_rid` rellena el `random_id` cuando el mensaje no lo trae (confirmaciones de envío).
pub(crate) fn persist_message(store: &Store, m: &core::Message, fallback_rid: Option<RandomId>) -> Result<()> {
    if is_secret(m.peer) {
        return Ok(());
    }
    let ck = ensure_dialog(store, m.peer)?;
    if let Some(md) = &m.media {
        store.upsert_media(&db::MediaRow {
            id: md.id,
            kind: media_kind_str(md.kind).into(),
            size: i64::try_from(md.size_bytes).ok(),
            width: Some(i64::from(md.width)),
            height: Some(i64::from(md.height)),
            file_name: md.file_name.clone(),
            local_path: md.local_path.clone(),
            duration_ms: md.duration_secs.map(|s| i64::from(s) * 1000),
            updated_at: now(),
            ..Default::default()
        })?;
        match &md.file {
            Some(f) => store.upsert_media_ref(&file_ref_row(md.id, f))?,
            None => store.delete_media_ref(md.id)?,
        }
        store.save_media_extra(md)?;
    }
    let msg_id = i64::from(m.id.0);
    let row = db::MessageRow {
        chat_key: ck,
        msg_id,
        random_id: m.random_id.or(fallback_rid).map(|r| r.0),
        sender_id: m.sender.map(|u| u.0),
        date: m.date.0,
        edit_date: m.edit_date.map(|t| t.0),
        is_out: m.outgoing,
        kind: if m.sticker.is_some() {
            db::MessageKind::Sticker
        } else if m.media.is_some() {
            db::MessageKind::Media
        } else {
            db::MessageKind::Text
        },
        text: m.text.clone(),
        entities: encode_entities(&m.entities),
        reply_to_id: m.reply_to.map(|r| i64::from(r.0)),
        fwd_from_name: m.forwarded_from.clone(),
        media_id: m.media.as_ref().map(|md| md.id),
        send_state: match m.state {
            core::SendState::Pending => db::SendState::Pending,
            core::SendState::Sent => db::SendState::Sent,
            core::SendState::Failed => db::SendState::Failed,
        },
        sticker_json: m.sticker.as_ref().map(encode_sticker),
        ..db::MessageRow::text(ck, msg_id, m.date.0, m.outgoing, "")
    };
    store.upsert_message(&row)?;
    // El upsert pudo renumerar un pendiente a su id definitivo: las reacciones cuelgan del id final.
    let rx: Vec<db::ReactionRow> = m
        .reactions
        .iter()
        .map(|r| db::ReactionRow { emoji: r.emoji.clone(), count: i64::from(r.count), chosen: r.mine })
        .collect();
    store.set_reactions(ck, msg_id, &rx)?;
    if m.state == core::SendState::Sent {
        store.bump_dialog_top(ck, msg_id, m.date.0)?;
    }
    Ok(())
}

pub(crate) fn load_message(store: &Store, peer: PeerRef, row: &db::MessageRow) -> Result<core::Message> {
    let media = match row.media_id {
        Some(id) => {
            let file = store.get_media_ref(id)?.map(file_ref_from);
            let extra = store.get_media_extra(id)?;
            store.get_media(id)?.map(|m| {
            let mut kind = media_kind_parse(&m.kind);
            let audio = crate::audio::apply_media_extra(&mut kind, extra);
            core::Media {
            id: m.id,
            kind,
            file_name: m.file_name,
            size_bytes: u64::try_from(m.size.unwrap_or(0)).unwrap_or(0),
            width: u32::try_from(m.width.unwrap_or(0)).unwrap_or(0),
            height: u32::try_from(m.height.unwrap_or(0)).unwrap_or(0),
            duration_secs: m.duration_ms.and_then(|ms| u32::try_from(ms / 1000).ok()),
            local_path: m.local_path,
            file,
            audio,
        }
        })
        }
        None => None,
    };
    let reactions = store
        .get_reactions(row.chat_key, row.msg_id)?
        .into_iter()
        .map(|r| core::ReactionCount { emoji: r.emoji, count: u32::try_from(r.count).unwrap_or(0), mine: r.chosen })
        .collect();
    Ok(core::Message {
        id: msg_id32(row.msg_id),
        peer,
        sender: row.sender_id.map(UserId),
        date: Timestamp(row.date),
        edit_date: row.edit_date.map(Timestamp),
        outgoing: row.is_out,
        text: row.text.clone(),
        entities: decode_entities(row.entities.as_deref()),
        reply_to: row.reply_to_id.map(msg_id32),
        forwarded_from: row.fwd_from_name.clone(),
        media,
        sticker: row.sticker_json.as_deref().and_then(decode_sticker),
        reactions,
        random_id: row.random_id.map(RandomId),
        state: match row.send_state {
            db::SendState::Pending => core::SendState::Pending,
            db::SendState::Sent => core::SendState::Sent,
            db::SendState::Failed => core::SendState::Failed,
        },
        ttl_secs: None,
    })
}

/// Página de historial desde la caché, con el diálogo al que pertenece (si lo hay).
pub(crate) fn cached_history(
    store: &Store,
    peer: PeerRef,
    anchor: core::Anchor,
    limit: usize,
) -> Result<(Vec<core::Message>, Option<db::DialogRow>)> {
    let (kind, id) = peer_parts(peer);
    let Some(d) = store.find_dialog(kind, id)? else { return Ok((Vec::new(), None)) };
    let rows = store.messages_page(d.chat_key, map_anchor(anchor), limit)?;
    let msgs = rows.iter().map(|r| load_message(store, peer, r)).collect::<Result<Vec<_>>>()?;
    Ok((msgs, Some(d)))
}

/// ¿Basta la caché para responder esta página sin esperar a la red?
pub(crate) fn history_is_servable(cached: &[core::Message], dialog: Option<&db::DialogRow>, anchor: core::Anchor, limit: usize) -> bool {
    let Some(d) = dialog else { return false };
    if limit == 0 {
        return true;
    }
    let enough = cached.len() >= limit || (d.history_complete && matches!(anchor, core::Anchor::Latest | core::Anchor::Before(_)));
    if !enough {
        return false;
    }
    match anchor {
        // Los últimos mensajes solo valen si llegan hasta el último conocido del diálogo.
        core::Anchor::Latest => match (cached.last(), d.top_message_id) {
            (Some(last), Some(top)) => i64::from(last.id.0) >= top,
            (None, _) => d.history_complete && d.top_message_id.is_none(),
            _ => false,
        },
        // Ventana alrededor del primer no leído o hacia lo más nuevo: la caché puede tener huecos
        // entre páginas sueltas que no sabemos detectar, así que manda la red (sin red, la caché).
        core::Anchor::Around(_) | core::Anchor::After(_) => false,
        core::Anchor::Before(_) => true,
    }
}

/// Guarda una página recibida del backend. Devuelve los eventos que describen lo que cambió respecto a
/// la caché (nuevos, editados, borrados en el servidor).
pub(crate) fn apply_history(
    store: &Store,
    peer: PeerRef,
    anchor: core::Anchor,
    limit: usize,
    fetched: &[core::Message],
) -> Result<Vec<core::BackendEvent>> {
    if is_secret(peer) {
        return Ok(Vec::new());
    }
    let ck = ensure_dialog(store, peer)?;
    let mut events = Vec::new();
    let ids: Vec<i64> = fetched.iter().map(|m| i64::from(m.id.0)).collect();
    if let (Some(&lo), Some(&hi)) = (ids.iter().min(), ids.iter().max()) {
        let stale: Vec<i64> = store.message_ids_between(ck, lo, hi)?.into_iter().filter(|i| !ids.contains(i)).collect();
        if !stale.is_empty() {
            store.delete_messages(ck, &stale)?;
            events.push(core::BackendEvent::MessagesDeleted { peer, ids: stale.iter().map(|&i| msg_id32(i)).collect() });
        }
    }
    for m in fetched {
        let before = match store.get_message(ck, i64::from(m.id.0))? {
            Some(r) => Some(load_message(store, peer, &r)?),
            None => None,
        };
        persist_message(store, m, None)?;
        let after = match store.get_message(ck, i64::from(m.id.0))? {
            Some(r) => Some(load_message(store, peer, &r)?),
            None => None,
        };
        match before {
            None if matches!(anchor, core::Anchor::Latest | core::Anchor::After(_)) => events.push(core::BackendEvent::MessageNew(m.clone())),
            Some(_) if before != after => events.push(core::BackendEvent::MessageEdited(m.clone())),
            _ => {}
        }
    }
    if matches!(anchor, core::Anchor::Latest | core::Anchor::Before(_)) && fetched.len() < limit {
        store.set_history_complete(ck, true)?;
    }
    Ok(events)
}

// ---- perfil ---------------------------------------------------------------------------------

fn presence(s: core::UserStatus) -> (db::PresenceKind, Option<i64>) {
    match s {
        core::UserStatus::Online => (db::PresenceKind::Online, None),
        core::UserStatus::LastSeen(t) => (db::PresenceKind::Offline, Some(t.0)),
        core::UserStatus::Recently => (db::PresenceKind::Recently, None),
        core::UserStatus::Offline => (db::PresenceKind::Offline, None),
    }
}

pub(crate) fn persist_user_status(store: &Store, id: i64, status: core::UserStatus) -> Result<()> {
    let Some(mut x) = store.get_user_extra(id)? else { return Ok(()) };
    (x.status, x.last_seen_at) = presence(status);
    store.set_user_extra(id, &x)
}

/// Guarda un usuario. `is_self = None` conserva la marca que ya tuviera.
pub(crate) fn persist_user(store: &Store, u: &core::User, is_self: Option<bool>) -> Result<()> {
    let old = store.get_user(u.id.0)?;
    store.upsert_user(&db::UserRow {
        id: u.id.0,
        access_hash: old.as_ref().and_then(|o| o.access_hash),
        first_name: u.first_name.clone(),
        last_name: u.last_name.clone(),
        username: u.username.clone(),
        phone: old.as_ref().and_then(|o| o.phone.clone()),
        is_self: is_self.unwrap_or_else(|| old.as_ref().is_some_and(|o| o.is_self)),
        is_contact: u.is_contact,
        updated_at: now(),
    })?;
    let (status, last_seen_at) = presence(u.status);
    store.set_user_extra(u.id.0, &db::UserExtra { is_bot: u.is_bot, status, last_seen_at })?;
    persist_photo(store, db::PeerKind::User, u.id.0, u.photo)
}

pub(crate) fn load_user(store: &Store, row: db::UserRow) -> Result<core::User> {
    let x = store.get_user_extra(row.id)?;
    let status = match x.as_ref().map(|x| (x.status, x.last_seen_at)) {
        Some((db::PresenceKind::Online, _)) => core::UserStatus::Online,
        Some((db::PresenceKind::Recently, _)) => core::UserStatus::Recently,
        Some((db::PresenceKind::Offline, Some(t))) => core::UserStatus::LastSeen(Timestamp(t)),
        _ => core::UserStatus::Offline,
    };
    Ok(core::User {
        id: UserId(row.id),
        first_name: row.first_name,
        last_name: row.last_name,
        username: row.username,
        status,
        is_contact: row.is_contact,
        is_bot: x.is_some_and(|x| x.is_bot),
        photo: load_photo(store, db::PeerKind::User, row.id)?,
    })
}

pub(crate) fn persist_group(store: &Store, g: &core::Group) -> Result<()> {
    let old = store.get_group(g.id.0)?;
    store.upsert_group(&db::GroupRow {
        id: g.id.0,
        kind: match g.kind {
            core::GroupKind::Basic => db::GroupKindRow::Group,
            core::GroupKind::Super => db::GroupKindRow::Supergroup,
            core::GroupKind::Channel => db::GroupKindRow::Channel,
        },
        access_hash: old.as_ref().and_then(|o| o.access_hash),
        title: g.title.clone(),
        about: old.as_ref().and_then(|o| o.about.clone()),
        username: g.username.clone(),
        participants_count: Some(i64::from(g.member_count)),
        my_role: old.as_ref().map_or(db::GroupRole::Member, |o| o.my_role),
        can_send: old.as_ref().is_none_or(|o| o.can_send),
        created_at: old.as_ref().and_then(|o| o.created_at),
        updated_at: now(),
    })?;
    store.set_group_forum(g.id.0, g.is_forum)?;
    persist_photo(store, db::PeerKind::Group, g.id.0, g.photo)
}

pub(crate) fn load_group(store: &Store, row: db::GroupRow) -> Result<core::Group> {
    let photo = load_photo(store, db::PeerKind::Group, row.id)?;
    Ok(core::Group {
        id: GroupId(row.id),
        kind: match row.kind {
            db::GroupKindRow::Group => core::GroupKind::Basic,
            db::GroupKindRow::Supergroup => core::GroupKind::Super,
            db::GroupKindRow::Channel => core::GroupKind::Channel,
        },
        title: row.title,
        username: row.username,
        member_count: row.participants_count.and_then(|n| u32::try_from(n).ok()).unwrap_or(0),
        photo,
        is_forum: store.group_is_forum(row.id)?,
    })
}

#[cfg(test)]
mod sticker_json_tests {
    use super::*;

    #[test]
    fn sticker_file_reference_survives_the_cache() {
        let file = core::FileRef {
            kind: core::FileKind::Document,
            id: 900,
            access_hash: -7,
            file_reference: vec![0, 1, 0xab, 0xff],
            dc_id: 4,
            size_bytes: 20_000,
            thumb: Some("m".into()),
            full: None,
        };
        let s = core::Sticker {
            id: StickerId(900),
            set_id: StickerSetId(314),
            emoji: "😺".into(),
            width: 512,
            height: 512,
            format: core::StickerFormat::Lottie,
            file: Some(file),
        };
        assert_eq!(decode_sticker(&encode_sticker(&s)), Some(s.clone()));
        // Filas antiguas, sin referencia: el sticker sigue leyéndose.
        let old = core::Sticker { file: None, ..s };
        assert_eq!(decode_sticker(&encode_sticker(&old)), Some(old));
    }
}
