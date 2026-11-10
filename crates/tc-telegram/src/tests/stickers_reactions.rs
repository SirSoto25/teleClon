//! Stickers reales (packs con `hash`, recientes, referencias caducadas al enviar y al descargar) y
//! reacciones disponibles (el `PREMIUM_ACCOUNT_REQUIRED` visto con la red real).
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use grammers_tl_types as tl;
use tc_core::*;

use super::fixtures::*;
use super::flows::restored;
use super::*;

const SET_A: i64 = 314;
const SET_B: i64 = 315;

fn sticker_doc(id: i64, set: i64, mime: &str, file_reference: Vec<u8>) -> tl::enums::Document {
    tl::types::Document {
        id,
        access_hash: id * 10,
        file_reference,
        date: 0,
        mime_type: mime.into(),
        size: 20_000,
        thumbs: Some(vec![tl::types::PhotoSize { r#type: "m".into(), w: 128, h: 128, size: 3_000 }.into()]),
        video_thumbs: None,
        dc_id: 2,
        attributes: vec![
            tl::types::DocumentAttributeSticker {
                mask: false,
                alt: "😺".into(),
                stickerset: tl::types::InputStickerSetId { id: set, access_hash: set * 2 }.into(),
                mask_coords: None,
            }
            .into(),
            tl::types::DocumentAttributeImageSize { w: 512, h: 512 }.into(),
        ],
    }
    .into()
}

fn set_info(id: i64, hash: i32) -> tl::enums::StickerSet {
    tl::types::StickerSet {
        archived: false,
        official: false,
        masks: false,
        emojis: false,
        text_color: false,
        channel_emoji_status: false,
        creator: false,
        installed_date: Some(1),
        id,
        access_hash: id * 2,
        title: format!("Pack {id}"),
        short_name: format!("pack{id}"),
        thumbs: None,
        thumb_dc_id: None,
        thumb_version: None,
        thumb_document_id: None,
        count: 2,
        hash,
    }
    .into()
}

fn full_set(id: i64, docs: Vec<tl::enums::Document>) -> tl::enums::messages::StickerSet {
    tl::types::messages::StickerSet { set: set_info(id, 1), packs: vec![], keywords: vec![], documents: docs }.into()
}

#[tokio::test]
async fn sticker_sets_use_hashes_skip_broken_packs_and_refresh_references() {
    use tl::functions::{messages, upload};
    let all_calls = Arc::new(AtomicUsize::new(0));
    let set_a_calls = Arc::new(AtomicUsize::new(0));
    let (all, set_a) = (all_calls.clone(), set_a_calls.clone());
    let (b, fake, _) = restored(
        move |_, id, body| match id {
            x if x == cid::<messages::GetAllStickers>() => {
                let req: messages::GetAllStickers = decode(body);
                all.fetch_add(1, Ordering::SeqCst);
                assert_eq!(req.hash, 0, "la carga fue parcial (pack roto): no se guarda el hash");
                ok(tl::enums::messages::AllStickers::from(tl::types::messages::AllStickers { hash: 99, sets: vec![set_info(SET_A, 1), set_info(SET_B, 1)] }))
            }
            x if x == cid::<messages::GetStickerSet>() => {
                let req: messages::GetStickerSet = decode(body);
                let tl::enums::InputStickerSet::Id(s) = req.stickerset else { panic!("pack por id") };
                assert_eq!(s.access_hash, s.id * 2);
                if s.id == SET_B {
                    return rpc_err(400, "STICKERSET_INVALID", None);
                }
                // La segunda vez (refresco) las referencias son nuevas.
                let fr = if set_a.fetch_add(1, Ordering::SeqCst) == 0 { vec![1] } else { vec![2] };
                ok(full_set(SET_A, vec![sticker_doc(900, SET_A, "image/webp", fr.clone()), sticker_doc(901, SET_A, "application/x-tgsticker", fr)]))
            }
            x if x == cid::<messages::SendMedia>() => {
                let req: messages::SendMedia = decode(body);
                let tl::enums::InputMedia::Document(d) = req.media else { panic!("sticker como documento") };
                let tl::enums::InputDocument::Document(doc) = d.id else { panic!("documento") };
                assert_eq!((doc.id, doc.access_hash), (900, 9000));
                if doc.file_reference == [1] {
                    return rpc_err(400, "FILE_REFERENCE_EXPIRED", None);
                }
                assert_eq!(doc.file_reference, [2]);
                let mut m = message(600, peer_user(10), "", true, None, 50);
                m.media = Some(
                    tl::types::MessageMediaDocument {
                        nopremium: false,
                        spoiler: false,
                        video: false,
                        round: false,
                        voice: false,
                        document: Some(sticker_doc(900, SET_A, "image/webp", vec![2])),
                        alt_documents: None,
                        video_cover: None,
                        video_timestamp: None,
                        ttl_seconds: None,
                    }
                    .into(),
                );
                ok(updates(
                    vec![
                        tl::types::UpdateMessageId { id: 600, random_id: req.random_id }.into(),
                        tl::types::UpdateNewMessage { message: m.into(), pts: 1, pts_count: 1 }.into(),
                    ],
                    vec![],
                    vec![],
                ))
            }
            x if x == cid::<upload::GetFile>() => {
                let req: upload::GetFile = decode(body);
                let tl::enums::InputFileLocation::InputDocumentFileLocation(loc) = req.location else { panic!("documento") };
                assert_eq!((loc.id, loc.thumb_size.as_str()), (901, "m"));
                if loc.file_reference == [1] {
                    return rpc_err(400, "FILE_REFERENCE_EXPIRED", None);
                }
                ok(tl::enums::upload::File::from(tl::types::upload::File { r#type: tl::enums::storage::FileType::FileWebp, mtime: 0, bytes: b"webp".to_vec() }))
            }
            x if x == cid::<messages::GetRecentStickers>() => {
                let req: messages::GetRecentStickers = decode(body);
                if req.hash == 0 {
                    ok(tl::enums::messages::RecentStickers::from(tl::types::messages::RecentStickers {
                        hash: 7,
                        packs: vec![],
                        stickers: vec![sticker_doc(901, SET_A, "application/x-tgsticker", vec![2])],
                        dates: vec![1],
                    }))
                } else {
                    assert_eq!(req.hash, 7);
                    ok(tl::enums::messages::RecentStickers::NotModified)
                }
            }
            other => panic!("llamada inesperada {}", tl::name_for_id(other)),
        },
        None,
    )
    .await;
    b.inner.ingest(&[user_enum(10, "Ana", 100, false)], &[]);

    // El pack roto no tumba la lista; el bueno trae sus documentos descargables.
    let sets = b.installed_sticker_sets().await.unwrap();
    assert_eq!(sets.len(), 1);
    assert_eq!((sets[0].id, sets[0].title.as_str(), sets[0].stickers.len()), (StickerSetId(SET_A), "Pack 314", 2));
    let tgs = sets[0].stickers[1].clone();
    assert_eq!(tgs.format, StickerFormat::Lottie);
    let file = tgs.file.clone().expect("referencia del documento");
    assert_eq!((file.kind, file.id, file.thumb.as_deref()), (FileKind::Document, 901, Some("m")));

    // Segunda carga: el pack bueno no se vuelve a pedir (mismo hash del pack); el roto sí.
    let again = b.installed_sticker_sets().await.unwrap();
    assert_eq!(again.len(), 1);
    assert_eq!(set_a_calls.load(Ordering::SeqCst), 1);
    assert_eq!(fake.count("messages.getStickerSet"), 3);

    // Miniatura de un sticker del selector (sin mensaje): referencia caducada → se pide el pack.
    let peer = PeerRef::User(UserId(10));
    assert_eq!(b.download_media(peer, MsgId(0), file, MediaVariant::Thumb).await.unwrap(), b"webp");
    assert_eq!(set_a_calls.load(Ordering::SeqCst), 2);

    // Enviar con referencia caducada: se refresca el pack y se reintenta.
    let set_a_before = set_a_calls.load(Ordering::SeqCst);
    let m = b.send_sticker(peer, StickerId(900), None, RandomId(5)).await;
    // El refresco anterior ya dejó la referencia nueva: se envía a la primera.
    let m = m.unwrap();
    assert_eq!(m.sticker.as_ref().map(|s| s.id), Some(StickerId(900)));
    assert!(m.sticker.unwrap().file.is_some());
    assert_eq!(set_a_calls.load(Ordering::SeqCst), set_a_before);
    assert_eq!(fake.count("messages.sendMedia"), 1);

    // Recientes con hash.
    let r = b.recent_stickers().await.unwrap();
    assert_eq!(r.iter().map(|s| (s.id, s.set_id)).collect::<Vec<_>>(), [(StickerId(901), StickerSetId(SET_A))]);
    assert_eq!(b.recent_stickers().await.unwrap(), r);
    assert_eq!(all_calls.load(Ordering::SeqCst), 2);
}

#[tokio::test]
async fn stale_sticker_reference_on_send_is_refreshed_once() {
    use tl::functions::messages;
    let set_calls = Arc::new(AtomicUsize::new(0));
    let sc = set_calls.clone();
    let (b, fake, _) = restored(
        move |_, id, body| match id {
            x if x == cid::<messages::GetAllStickers>() => {
                ok(tl::enums::messages::AllStickers::from(tl::types::messages::AllStickers { hash: 5, sets: vec![set_info(SET_A, 1)] }))
            }
            x if x == cid::<messages::GetStickerSet>() => {
                let fr = if sc.fetch_add(1, Ordering::SeqCst) == 0 { vec![1] } else { vec![2] };
                ok(full_set(SET_A, vec![sticker_doc(900, SET_A, "image/webp", fr)]))
            }
            x if x == cid::<messages::SendMedia>() => {
                let req: messages::SendMedia = decode(body);
                let tl::enums::InputMedia::Document(d) = req.media else { panic!() };
                let tl::enums::InputDocument::Document(doc) = d.id else { panic!() };
                if doc.file_reference == [1] {
                    return rpc_err(400, "FILE_REFERENCE_EXPIRED", None);
                }
                ok(updates(
                    vec![
                        tl::types::UpdateMessageId { id: 601, random_id: req.random_id }.into(),
                        tl::types::UpdateNewMessage { message: msg_enum(601, peer_user(10), "", true, 50), pts: 1, pts_count: 1 }.into(),
                    ],
                    vec![],
                    vec![],
                ))
            }
            other => panic!("llamada inesperada {}", tl::name_for_id(other)),
        },
        None,
    )
    .await;
    b.inner.ingest(&[user_enum(10, "Ana", 100, false)], &[]);
    b.installed_sticker_sets().await.unwrap();
    let m = b.send_sticker(PeerRef::User(UserId(10)), StickerId(900), None, RandomId(6)).await.unwrap();
    assert_eq!(m.id, MsgId(601));
    assert_eq!(fake.count("messages.sendMedia"), 2);
    assert_eq!(set_calls.load(Ordering::SeqCst), 2);
    // Un sticker que nunca se ha visto no se puede enviar.
    assert_eq!(b.send_sticker(PeerRef::User(UserId(10)), StickerId(1), None, RandomId(7)).await, Err(BackendError::NotFound));
}

fn available(e: &str, premium: bool, inactive: bool) -> tl::enums::AvailableReaction {
    let doc = || tl::enums::Document::Empty(tl::types::DocumentEmpty { id: 0 });
    tl::types::AvailableReaction {
        inactive,
        premium,
        reaction: e.into(),
        title: e.into(),
        static_icon: doc(),
        appear_animation: doc(),
        select_animation: doc(),
        activate_animation: doc(),
        effect_animation: doc(),
        around_animation: None,
        center_icon: None,
    }
    .into()
}

/// Reproduce lo visto con la red real: reaccionar a un mensaje propio en «Mensajes guardados» sin
/// Premium daba `PREMIUM_ACCOUNT_REQUIRED`. Ahora ahí no se ofrece ninguna reacción, el error se
/// traduce a `PremiumRequired` y en los demás chats solo se ofrecen las gratuitas activas.
#[tokio::test]
async fn reactions_offered_exclude_premium_and_saved_messages() {
    use tl::functions::messages;
    let (b, fake, _) = restored(
        |_, id, body| match id {
            x if x == cid::<messages::SendReaction>() => {
                let req: messages::SendReaction = decode(body);
                assert!(matches!(req.peer, tl::enums::InputPeer::PeerSelf));
                rpc_err(403, "PREMIUM_ACCOUNT_REQUIRED", None)
            }
            x if x == cid::<messages::GetAvailableReactions>() => ok(tl::enums::messages::AvailableReactions::from(tl::types::messages::AvailableReactions {
                hash: 3,
                reactions: vec![available("👍", false, false), available("❤", false, false), available("🤩", true, false), available("🫥", false, true), available("🔥", false, false)],
            })),
            x if x == cid::<messages::GetTopReactions>() => ok(tl::enums::messages::Reactions::from(tl::types::messages::Reactions {
                hash: 1,
                reactions: vec![tl::types::ReactionEmoji { emoticon: "🔥".into() }.into(), tl::types::ReactionCustomEmoji { document_id: 5 }.into()],
            })),
            x if x == cid::<messages::GetRecentReactions>() => ok(tl::enums::messages::Reactions::NotModified),
            other => panic!("llamada inesperada {}", tl::name_for_id(other)),
        },
        None,
    )
    .await;
    b.inner.ingest(&[user_enum(10, "Ana", 100, false)], &[]);
    let saved = PeerRef::User(UserId(ME_ID));
    assert_eq!(b.available_reactions(saved).await.unwrap(), Vec::<String>::new());
    assert_eq!(fake.count("messages.getAvailableReactions"), 0, "en guardados ni se pregunta el catálogo");
    assert_eq!(b.send_reaction(saved, MsgId(5), Some("👍".into())).await, Err(BackendError::PremiumRequired));

    let list = b.available_reactions(PeerRef::User(UserId(10))).await.unwrap();
    assert_eq!(list, ["🔥", "👍", "❤"], "más usadas primero; sin premium, inactivas ni personalizadas");
    assert!(b.available_reactions(PeerRef::Secret(SecretChatId(1))).await.unwrap().is_empty());
    // La cuenta solo se consulta una vez.
    assert_eq!(fake.count("users.getUsers"), 2, "una al restaurar la sesión y otra para saber si es Premium");
    b.available_reactions(PeerRef::User(UserId(10))).await.unwrap();
    assert_eq!(fake.count("users.getUsers"), 2);
}
