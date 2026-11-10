//! Mapeo TL → dominio (funciones puras).
use grammers_tl_types as tl;
use tc_core::*;

use super::fixtures::*;
use crate::convert::*;
use crate::events::{Cache, apply_batch};
use crate::ids::{channel_group, chat_group};
use crate::peers::{PeerStore, group_from_tl, user_from_tl};

const ME: UserId = UserId(77);

fn store() -> PeerStore {
    let mut p = PeerStore::default();
    p.ingest_users(&[user_enum(77, "Yo", 1, true), user_enum(10, "Ana", 100, false)]);
    p.ingest_chats(&[channel(20, "Noticias", true, 200), chat(30, "Amigos")]);
    p
}

#[test]
fn entities_convert_between_utf16_and_chars() {
    // «a😀b hola»: el emoji ocupa 2 unidades UTF-16 y 1 carácter.
    let text = "a😀b hola";
    let tl_ents: Vec<tl::enums::MessageEntity> = vec![
        tl::types::MessageEntityBold { offset: 3, length: 1 }.into(),
        tl::types::MessageEntityTextUrl { offset: 5, length: 4, url: "https://ejemplo.org".into() }.into(),
        tl::types::MessageEntityCustomEmoji { offset: 1, length: 2, document_id: 1 }.into(),
        tl::types::MessageEntityItalic { offset: 50, length: 2 }.into(),
    ];
    let ents = entities_from_tl(text, Some(&tl_ents));
    assert_eq!(
        ents,
        vec![
            TextEntity { kind: EntityKind::Bold, offset: 2, length: 1 },
            TextEntity { kind: EntityKind::Url, offset: 4, length: 4 },
        ]
    );
    let back = entities_to_tl(text, &ents).unwrap();
    assert!(matches!(&back[0], tl::enums::MessageEntity::Bold(b) if b.offset == 3 && b.length == 1));
    assert!(matches!(&back[1], tl::enums::MessageEntity::Url(u) if u.offset == 5 && u.length == 4));
    assert_eq!(entities_to_tl(text, &[]), None);
    // Fuera de rango: se descarta.
    assert_eq!(entities_to_tl(text, &[TextEntity { kind: EntityKind::Code, offset: 7, length: 9 }]), None);
}

#[test]
fn users_and_groups_map() {
    let mut u = user(5, "", 9, false);
    u.deleted = true;
    let d = user_from_tl(&u);
    assert_eq!(d.first_name, "Cuenta eliminada");
    assert_eq!(d.status, UserStatus::LastSeen(Timestamp(1_700_000_000)));
    let ch = group_from_tl(&channel(20, "Noticias", true, 1)).unwrap();
    assert_eq!((ch.id, ch.kind, ch.member_count), (channel_group(20), GroupKind::Channel, 321));
    assert_eq!(group_from_tl(&channel(21, "Super", false, 1)).unwrap().kind, GroupKind::Super);
    assert_eq!(group_from_tl(&chat(30, "Amigos")).unwrap().id, GroupId(-30));
    let p = store();
    assert_eq!(p.title(PeerRef::User(ME), Some(ME)), "Mensajes guardados");
    assert_eq!(p.title(PeerRef::User(UserId(10)), Some(ME)), "Ana");
    assert_eq!(p.title(PeerRef::Group(chat_group(30)), Some(ME)), "Amigos");
}

#[test]
fn input_peers_need_known_hashes() {
    let p = store();
    let s = crate::TcSession::default();
    assert!(matches!(p.input_peer(PeerRef::User(UserId(10)), Some(ME), &s).unwrap(), tl::enums::InputPeer::User(u) if u.access_hash == 100));
    assert!(matches!(p.input_peer(PeerRef::User(ME), Some(ME), &s).unwrap(), tl::enums::InputPeer::PeerSelf));
    assert!(matches!(p.input_peer(PeerRef::Group(channel_group(20)), Some(ME), &s).unwrap(), tl::enums::InputPeer::Channel(c) if c.access_hash == 200));
    assert!(matches!(p.input_peer(PeerRef::Group(chat_group(30)), Some(ME), &s).unwrap(), tl::enums::InputPeer::Chat(c) if c.chat_id == 30));
    assert_eq!(p.input_peer(PeerRef::User(UserId(999)), Some(ME), &s), Err(BackendError::NotFound));
    assert!(matches!(p.input_peer(PeerRef::Secret(SecretChatId(1)), Some(ME), &s), Err(BackendError::Forbidden(_))));
}

#[test]
fn messages_map_sender_reply_forward_and_reactions() {
    let p = store();
    // Entrante en privado sin from_id: remitente = el otro.
    let m = message_from_tl(&msg_enum(5, peer_user(10), "hola", false, 100), Some(ME), &p).unwrap();
    assert_eq!((m.id, m.peer, m.sender, m.outgoing, m.date), (MsgId(5), PeerRef::User(UserId(10)), Some(UserId(10)), false, Timestamp(100)));
    // Saliente: remitente = yo.
    let m = message_from_tl(&msg_enum(6, peer_user(10), "adiós", true, 101), Some(ME), &p).unwrap();
    assert_eq!(m.sender, Some(ME));
    // Canal sin from_id: sin usuario.
    let m = message_from_tl(&msg_enum(7, peer_channel(20), "post", false, 102), Some(ME), &p).unwrap();
    assert_eq!((m.peer, m.sender), (PeerRef::Group(channel_group(20)), None));
    // Respuesta, reenvío y reacciones.
    let mut raw = message(8, peer_chat(30), "re", false, Some(peer_user(10)), 103);
    raw.reply_to = Some(
        tl::types::MessageReplyHeader {
            reply_to_scheduled: false,
            forum_topic: false,
            quote: false,
            reply_to_ephemeral: false,
            reply_to_msg_id: Some(4),
            reply_to_peer_id: None,
            reply_from: None,
            reply_media: None,
            reply_to_top_id: None,
            quote_text: None,
            quote_entities: None,
            quote_offset: None,
            todo_item_id: None,
            poll_option: None,
        }
        .into(),
    );
    raw.fwd_from = Some(
        tl::types::MessageFwdHeader {
            imported: false,
            saved_out: false,
            from_id: Some(peer_channel(20)),
            from_name: None,
            date: 0,
            channel_post: None,
            post_author: None,
            saved_from_peer: None,
            saved_from_msg_id: None,
            saved_from_id: None,
            saved_from_name: None,
            saved_date: None,
            psa_type: None,
        }
        .into(),
    );
    raw.reactions = Some(
        tl::types::MessageReactions {
            min: false,
            can_see_list: false,
            reactions_as_tags: false,
            results: vec![
                tl::types::ReactionCount { chosen_order: Some(0), reaction: tl::types::ReactionEmoji { emoticon: "👍".into() }.into(), count: 3 }.into(),
                tl::types::ReactionCount { chosen_order: None, reaction: tl::enums::Reaction::Paid, count: 1 }.into(),
            ],
            recent_reactions: None,
            top_reactors: None,
        }
        .into(),
    );
    raw.edit_date = Some(200);
    let m = message_from_tl(&raw.into(), Some(ME), &p).unwrap();
    assert_eq!(m.sender, Some(UserId(10)));
    assert_eq!(m.reply_to, Some(MsgId(4)));
    assert_eq!(m.forwarded_from.as_deref(), Some("Noticias"));
    assert_eq!(m.reactions, vec![ReactionCount { emoji: "👍".into(), count: 3, mine: true }]);
    assert_eq!(m.edit_date, Some(Timestamp(200)));
    assert!(message_from_tl(&tl::types::MessageEmpty { id: 1, peer_id: None }.into(), Some(ME), &p).is_none());
}

fn document(mime: &str, attributes: Vec<tl::enums::DocumentAttribute>) -> tl::types::Document {
    tl::types::Document {
        id: 900,
        access_hash: 1,
        file_reference: vec![],
        date: 0,
        mime_type: mime.into(),
        size: 2048,
        thumbs: None,
        video_thumbs: None,
        dc_id: 2,
        attributes,
    }
}

fn doc_media(d: tl::types::Document, voice: bool) -> tl::enums::MessageMedia {
    tl::types::MessageMediaDocument {
        nopremium: false,
        spoiler: false,
        video: false,
        round: false,
        voice,
        document: Some(d.into()),
        alt_documents: None,
        video_cover: None,
        video_timestamp: None,
        ttl_seconds: None,
    }
    .into()
}

#[test]
fn media_photos_documents_and_stickers() {
    let photo: tl::enums::MessageMedia = tl::types::MessageMediaPhoto {
        spoiler: false,
        live_photo: false,
        photo: Some(
            tl::types::Photo {
                has_stickers: false,
                id: 55,
                access_hash: 1,
                file_reference: vec![],
                date: 0,
                sizes: vec![
                    tl::types::PhotoSize { r#type: "m".into(), w: 320, h: 240, size: 10_000 }.into(),
                    tl::types::PhotoSizeProgressive { r#type: "y".into(), w: 1280, h: 960, sizes: vec![1000, 90_000] }.into(),
                ],
                video_sizes: None,
                dc_id: 2,
            }
            .into(),
        ),
        ttl_seconds: None,
        video: None,
    }
    .into();
    let (m, s) = media_from_tl(&photo);
    let m = m.unwrap();
    assert!(s.is_none());
    assert_eq!((m.id, m.kind, m.width, m.height, m.size_bytes), (55, MediaKind::Photo, 1280, 960, 90_000));

    let file = document("application/pdf", vec![tl::types::DocumentAttributeFilename { file_name: "informe.pdf".into() }.into()]);
    let (m, _) = media_from_tl(&doc_media(file, false));
    let m = m.unwrap();
    assert_eq!((m.kind, m.file_name.as_deref(), m.size_bytes), (MediaKind::File, Some("informe.pdf"), 2048));

    let voice = document("audio/ogg", vec![tl::types::DocumentAttributeAudio { voice: true, duration: 7, title: None, performer: None, waveform: None }.into()]);
    let (m, _) = media_from_tl(&doc_media(voice, true));
    assert_eq!(m.map(|m| (m.kind, m.duration_secs)), Some((MediaKind::Voice, Some(7))));

    let sticker = document(
        "image/webp",
        vec![
            tl::types::DocumentAttributeSticker {
                mask: false,
                alt: "😺".into(),
                stickerset: tl::types::InputStickerSetId { id: 314, access_hash: 2 }.into(),
                mask_coords: None,
            }
            .into(),
            tl::types::DocumentAttributeImageSize { w: 512, h: 500 }.into(),
        ],
    );
    let (m, s) = media_from_tl(&doc_media(sticker, false));
    assert!(m.is_none());
    let s = s.unwrap();
    let file = s.file.clone().expect("los stickers de la red se pueden descargar");
    assert_eq!((file.kind, file.id), (FileKind::Document, 900));
    assert_eq!(
        s,
        Sticker { id: StickerId(900), set_id: StickerSetId(314), emoji: "😺".into(), width: 512, height: 500, format: StickerFormat::Webp, file: Some(file) }
    );
}

#[test]
fn dialogs_map_pinned_muted_and_drafts() {
    let p = store();
    let ctx = DialogCtx { peers: &p, me: Some(ME), now: 1_000 };
    let last = message_from_tl(&msg_enum(9, peer_user(10), "último", false, 900), Some(ME), &p);
    let mut d = dialog(peer_user(10), 9, 2, true, Some(5_000));
    d.draft = Some(
        tl::types::DraftMessage {
            no_webpage: false,
            invert_media: false,
            reply_to: None,
            message: "borrador".into(),
            entities: None,
            media: None,
            date: 950,
            effect: None,
            suggested_post: None,
            rich_message: None,
        }
        .into(),
    );
    let row = dialog_from_tl(&d, last, Some(0), false, &ctx);
    assert_eq!(row.title, "Ana");
    assert_eq!((row.unread_count, row.pinned_order, row.muted), (2, Some(0), true));
    assert_eq!(row.draft, Some(Draft { text: "borrador".into(), reply_to: None }));
    assert_eq!(row.activity, Timestamp(950));
    assert_eq!(row.read_outbox_max_id, MsgId(3));
    // Silencio caducado y no fijado.
    let row = dialog_from_tl(&dialog(peer_channel(20), 1, 0, false, Some(10)), None, Some(3), true, &ctx);
    assert_eq!((row.muted, row.pinned_order, row.archived), (false, None, true));
}

fn cache() -> Cache {
    let mut c = Cache::default();
    c.peers = store();
    c.me = Some(ME);
    c
}

#[test]
fn updates_new_edit_delete_read() {
    let mut c = cache();
    let new: tl::enums::Update = tl::types::UpdateNewMessage { message: msg_enum(40, peer_user(10), "hola", false, 500), pts: 1, pts_count: 1 }.into();
    let (a, _) = apply_batch(&mut c, &[new], &[], &[], 0);
    assert!(matches!(&a.events[0], BackendEvent::MessageNew(m) if m.id == MsgId(40)));
    assert!(matches!(&a.events[1], BackendEvent::DialogUpserted(d) if d.unread_count == 1 && d.title == "Ana"));

    let mut edited = message(40, peer_user(10), "hola editado", false, None, 500);
    edited.edit_date = Some(510);
    let edit: tl::enums::Update = tl::types::UpdateEditMessage { message: edited.into(), pts: 2, pts_count: 1 }.into();
    let (a, _) = apply_batch(&mut c, &[edit], &[], &[], 0);
    assert!(matches!(&a.events[0], BackendEvent::MessageEdited(m) if m.text == "hola editado"));
    assert!(matches!(&a.events[1], BackendEvent::DialogUpserted(d) if d.last_message.as_ref().unwrap().text == "hola editado"));

    let read: tl::enums::Update = tl::types::UpdateReadHistoryInbox {
        folder_id: None,
        peer: peer_user(10),
        top_msg_id: None,
        max_id: 40,
        still_unread_count: 0,
        pts: 3,
        pts_count: 1,
    }
    .into();
    let (a, _) = apply_batch(&mut c, &[read], &[], &[], 0);
    assert_eq!(a.events[0], BackendEvent::ReadInbox { peer: PeerRef::User(UserId(10)), max_id: MsgId(40) });
    assert!(matches!(&a.events[1], BackendEvent::DialogUpserted(d) if d.unread_count == 0));

    // updateDeleteMessages no trae el chat: se resuelve con los ids recordados.
    let del: tl::enums::Update = tl::types::UpdateDeleteMessages { messages: vec![40, 9999], pts: 4, pts_count: 2 }.into();
    let (a, _) = apply_batch(&mut c, &[del], &[], &[], 0);
    assert_eq!(a.events[0], BackendEvent::MessagesDeleted { peer: PeerRef::User(UserId(10)), ids: vec![MsgId(40)] });
    assert!(matches!(&a.events[1], BackendEvent::DialogUpserted(d) if d.last_message.is_none()));

    let del_ch: tl::enums::Update = tl::types::UpdateDeleteChannelMessages { channel_id: 20, messages: vec![3], pts: 1, pts_count: 1 }.into();
    let (a, _) = apply_batch(&mut c, &[del_ch], &[], &[], 0);
    assert_eq!(a.events, vec![BackendEvent::MessagesDeleted { peer: PeerRef::Group(channel_group(20)), ids: vec![MsgId(3)] }]);
}

#[test]
fn updates_typing_status_reactions_and_login_token() {
    let mut c = cache();
    let ups: Vec<tl::enums::Update> = vec![
        tl::types::UpdateUserTyping { user_id: 10, top_msg_id: None, action: tl::enums::SendMessageAction::SendMessageTypingAction }.into(),
        tl::types::UpdateChatUserTyping { chat_id: 30, from_id: peer_user(10), action: tl::enums::SendMessageAction::SendMessageCancelAction }.into(),
        tl::types::UpdateUserStatus { user_id: 10, status: tl::types::UserStatusOnline { expires: 0 }.into() }.into(),
        tl::enums::Update::LoginToken,
    ];
    let (a, _) = apply_batch(&mut c, &ups, &[], &[], 0);
    assert!(a.login_token);
    assert_eq!(
        a.events,
        vec![
            BackendEvent::Typing { peer: PeerRef::User(UserId(10)), user: UserId(10), typing: true },
            BackendEvent::Typing { peer: PeerRef::Group(chat_group(30)), user: UserId(10), typing: false },
            BackendEvent::UserStatus { user: UserId(10), status: UserStatus::Online },
        ]
    );
    assert_eq!(c.peers.users[&UserId(10)].status, UserStatus::Online);
}

#[test]
fn own_send_confirmed_via_update_message_id() {
    let mut c = cache();
    c.pending.insert(777, PeerRef::User(UserId(10)));
    let ups: Vec<tl::enums::Update> = vec![
        tl::types::UpdateMessageId { id: 41, random_id: 777 }.into(),
        tl::types::UpdateNewMessage { message: msg_enum(41, peer_user(10), "mío", true, 600), pts: 1, pts_count: 1 }.into(),
    ];
    let (a, _) = apply_batch(&mut c, &ups, &[], &[], 0);
    assert!(matches!(&a.events[0], BackendEvent::MessageSent { random_id: RandomId(777), msg } if msg.id == MsgId(41) && msg.random_id == Some(RandomId(777))));
    assert!(c.pending.is_empty());
    assert!(c.sent.contains_key(&777));
}

#[test]
fn saved_messages_without_out_flag_are_outgoing() {
    // «Mensajes guardados»: chat conmigo mismo, Telegram puede mandar out=false.
    let p = store();
    let m = message_from_tl(&msg_enum(8, peer_user(77), "nota", false, 103), Some(ME), &p).unwrap();
    assert!(m.outgoing);
    assert_eq!(m.sender, Some(ME));
    // Un privado con otra persona sigue siendo entrante.
    let m = message_from_tl(&msg_enum(9, peer_user(10), "hola", false, 104), Some(ME), &p).unwrap();
    assert!(!m.outgoing);
}
