//! GIFs contra el transporte falso: guardados con `hash`, búsqueda con el bot `@gif` (resuelto una
//! vez), envío como documento (con referencia caducada) y como resultado del bot (con caída al
//! documento si la consulta caducó), y refresco de la miniatura de un GIF guardado.
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use grammers_tl_types as tl;
use tc_core::*;

use super::fixtures::*;
use super::flows::restored;
use super::*;

const BOT: i64 = 140;

fn gif_doc(id: i64, mime: &str, file_reference: Vec<u8>) -> tl::types::Document {
    tl::types::Document {
        id,
        access_hash: id * 10,
        file_reference,
        date: 0,
        mime_type: mime.into(),
        size: 90_000,
        thumbs: Some(vec![tl::types::PhotoSize { r#type: "m".into(), w: 320, h: 180, size: 9_000 }.into()]),
        video_thumbs: None,
        dc_id: 2,
        attributes: vec![
            tl::types::DocumentAttributeVideo {
                round_message: false,
                supports_streaming: true,
                nosound: true,
                duration: 2.4,
                w: 320,
                h: 180,
                preload_prefix_size: None,
                video_start_ts: None,
                video_codec: None,
            }
            .into(),
            tl::enums::DocumentAttribute::Animated,
            tl::types::DocumentAttributeFilename { file_name: "giphy.mp4".into() }.into(),
        ],
    }
}

fn media_result(id: &str, doc: tl::types::Document) -> tl::enums::BotInlineResult {
    tl::types::BotInlineMediaResult {
        id: id.into(),
        r#type: "gif".into(),
        photo: None,
        document: Some(doc.into()),
        title: None,
        description: None,
        send_message: tl::types::BotInlineMessageMediaAuto { invert_media: false, message: String::new(), entities: None, reply_markup: None }.into(),
    }
    .into()
}

fn sent(req_random: i64, id: i32, doc: tl::types::Document) -> Reply {
    let mut m = message(id, peer_user(10), "", true, None, 50);
    m.media = Some(
        tl::types::MessageMediaDocument {
            nopremium: false,
            spoiler: false,
            video: true,
            round: false,
            voice: false,
            document: Some(doc.into()),
            alt_documents: None,
            video_cover: None,
            video_timestamp: None,
            ttl_seconds: None,
        }
        .into(),
    );
    ok(updates(
        vec![tl::types::UpdateMessageId { id, random_id: req_random }.into(), tl::types::UpdateNewMessage { message: m.into(), pts: 1, pts_count: 1 }.into()],
        vec![],
        vec![],
    ))
}

#[tokio::test]
async fn saved_gifs_use_hash_send_as_document_and_refresh_references() {
    use tl::functions::{messages, upload};
    let saved_calls = Arc::new(AtomicUsize::new(0));
    let sc = saved_calls.clone();
    let (b, fake, _) = restored(
        move |_, id, body| match id {
            x if x == cid::<messages::GetSavedGifs>() => {
                let req: messages::GetSavedGifs = decode(body);
                let n = sc.fetch_add(1, Ordering::SeqCst);
                match (n, req.hash) {
                    (0, 0) => ok(tl::enums::messages::SavedGifs::from(tl::types::messages::SavedGifs {
                        hash: 55,
                        gifs: vec![gif_doc(700, "video/mp4", vec![1]).into(), gif_doc(701, "image/gif", vec![1]).into()],
                    })),
                    (1, 55) => ok(tl::enums::messages::SavedGifs::NotModified),
                    // Refresco (sin hash): referencias nuevas.
                    (_, 0) => ok(tl::enums::messages::SavedGifs::from(tl::types::messages::SavedGifs {
                        hash: 56,
                        gifs: vec![gif_doc(700, "video/mp4", vec![2]).into(), gif_doc(701, "image/gif", vec![2]).into()],
                    })),
                    other => panic!("getSavedGifs inesperado {other:?}"),
                }
            }
            x if x == cid::<messages::SendMedia>() => {
                let req: messages::SendMedia = decode(body);
                let tl::enums::InputMedia::Document(d) = req.media else { panic!("GIF como documento") };
                let tl::enums::InputDocument::Document(doc) = d.id else { panic!("documento") };
                assert_eq!((doc.id, doc.access_hash), (700, 7000));
                if doc.file_reference == [1] {
                    return rpc_err(400, "FILE_REFERENCE_EXPIRED", None);
                }
                sent(req.random_id, 610, gif_doc(700, "video/mp4", vec![2]))
            }
            x if x == cid::<upload::GetFile>() => {
                let req: upload::GetFile = decode(body);
                let tl::enums::InputFileLocation::InputDocumentFileLocation(loc) = req.location else { panic!("documento") };
                assert_eq!((loc.id, loc.thumb_size.as_str()), (701, "m"));
                if loc.file_reference == [1] {
                    return rpc_err(400, "FILE_REFERENCE_EXPIRED", None);
                }
                ok(tl::enums::upload::File::from(tl::types::upload::File { r#type: tl::enums::storage::FileType::FileJpeg, mtime: 0, bytes: b"jpeg".to_vec() }))
            }
            other => panic!("llamada inesperada {}", tl::name_for_id(other)),
        },
        None,
    )
    .await;
    b.inner.ingest(&[user_enum(10, "Ana", 100, false)], &[]);

    let gifs = b.saved_gifs().await.unwrap();
    assert_eq!(gifs.iter().map(|g| (g.id, g.mime.as_str(), g.is_real_gif())).collect::<Vec<_>>(), [(700, "video/mp4", false), (701, "image/gif", true)]);
    assert_eq!((gifs[0].width, gifs[0].height, gifs[0].duration_secs, gifs[0].title.as_str()), (320, 180, Some(2), "giphy.mp4"));
    assert_eq!(gifs[0].file.as_ref().and_then(|f| f.thumb.as_deref()), Some("m"));
    assert!(gifs[0].inline.is_none());
    // Con el mismo hash no se baja nada y se devuelve lo que había.
    assert_eq!(b.saved_gifs().await.unwrap(), gifs);

    // Miniatura del panel (sin mensaje) con referencia caducada: se piden los guardados sin hash.
    let peer = PeerRef::User(UserId(10));
    let thumb = b.download_media(peer, MsgId(0), gifs[1].file.clone().unwrap(), MediaVariant::Thumb).await.unwrap();
    assert_eq!(thumb, b"jpeg");
    assert_eq!(saved_calls.load(Ordering::SeqCst), 3);

    // El refresco ya dejó la referencia nueva del 700: se envía a la primera, como documento.
    let m = b.send_gif(peer, gifs[0].clone(), None, RandomId(9)).await.unwrap();
    assert_eq!(m.id, MsgId(610));
    let md = m.media.unwrap();
    assert_eq!((md.kind, md.width), (MediaKind::Gif, 320));
    assert_eq!(fake.count("messages.sendMedia"), 1);
}

#[tokio::test]
async fn stale_gif_reference_on_send_is_refreshed_once() {
    use tl::functions::messages;
    let calls = Arc::new(AtomicUsize::new(0));
    let c2 = calls.clone();
    let (b, fake, _) = restored(
        move |_, id, body| match id {
            x if x == cid::<messages::GetSavedGifs>() => {
                // La primera vez, la referencia vieja; después, la nueva.
                let fr = if c2.fetch_add(1, Ordering::SeqCst) == 0 { vec![1] } else { vec![2] };
                ok(tl::enums::messages::SavedGifs::from(tl::types::messages::SavedGifs { hash: 0, gifs: vec![gif_doc(700, "video/mp4", fr).into()] }))
            }
            x if x == cid::<messages::SendMedia>() => {
                let req: messages::SendMedia = decode(body);
                let tl::enums::InputMedia::Document(d) = req.media else { panic!() };
                let tl::enums::InputDocument::Document(doc) = d.id else { panic!() };
                if doc.file_reference == [1] {
                    return rpc_err(400, "FILE_REFERENCE_EXPIRED", None);
                }
                sent(req.random_id, 611, gif_doc(700, "video/mp4", vec![2]))
            }
            other => panic!("llamada inesperada {}", tl::name_for_id(other)),
        },
        None,
    )
    .await;
    b.inner.ingest(&[user_enum(10, "Ana", 100, false)], &[]);
    let g = b.saved_gifs().await.unwrap().remove(0);
    let m = b.send_gif(PeerRef::User(UserId(10)), g, None, RandomId(10)).await.unwrap();
    assert_eq!(m.id, MsgId(611));
    assert_eq!(fake.count("messages.sendMedia"), 2);
    assert_eq!(fake.count("messages.getSavedGifs"), 2);
    assert_eq!(calls.load(Ordering::SeqCst), 2);
    // Un GIF del que no se sabe nada no se puede enviar.
    let unknown = Gif { id: 1, file: None, mime: "video/mp4".into(), width: 0, height: 0, duration_secs: None, title: String::new(), inline: None };
    assert_eq!(b.send_gif(PeerRef::User(UserId(10)), unknown, None, RandomId(11)).await, Err(BackendError::NotFound));
}

#[tokio::test]
async fn search_uses_the_gif_bot_and_sends_inline_results() {
    use tl::functions::{contacts, messages};
    let (b, fake, _) = restored(
        move |_, id, body| match id {
            x if x == cid::<contacts::ResolveUsername>() => {
                let req: contacts::ResolveUsername = decode(body);
                assert_eq!(req.username, "gif");
                let mut bot = user(BOT, "GIF Search", 4444, false);
                bot.bot = true;
                bot.bot_info_version = Some(1); // comparte bit de flags con `bot`
                ok(tl::enums::contacts::ResolvedPeer::from(tl::types::contacts::ResolvedPeer { peer: peer_user(BOT), chats: vec![], users: vec![bot.into()] }))
            }
            x if x == cid::<messages::GetInlineBotResults>() => {
                let req: messages::GetInlineBotResults = decode(body);
                let tl::enums::InputUser::User(u) = req.bot else { panic!("bot por id") };
                assert_eq!((u.user_id, u.access_hash), (BOT, 4444));
                assert_eq!(req.query, "gato");
                let page2 = req.offset == "6";
                let external = tl::types::BotInlineResult {
                    id: "url".into(),
                    r#type: "gif".into(),
                    title: None,
                    description: None,
                    url: None,
                    thumb: None,
                    content: None,
                    send_message: tl::types::BotInlineMessageMediaAuto { invert_media: false, message: String::new(), entities: None, reply_markup: None }.into(),
                };
                ok(tl::enums::messages::BotResults::from(tl::types::messages::BotResults {
                    gallery: true,
                    query_id: 31337,
                    next_offset: if page2 { None } else { Some("6".into()) },
                    switch_pm: None,
                    switch_webview: None,
                    results: vec![media_result("a1", gif_doc(800, "video/mp4", vec![1])), external.into(), media_result("a2", gif_doc(801, "image/gif", vec![1]))],
                    cache_time: 300,
                    users: vec![],
                }))
            }
            x if x == cid::<messages::SendInlineBotResult>() => {
                let req: messages::SendInlineBotResult = decode(body);
                assert!(req.hide_via);
                assert_eq!(req.query_id, 31337);
                if req.id == "a2" {
                    return rpc_err(400, "QUERY_ID_INVALID", None);
                }
                assert_eq!(req.id, "a1");
                sent(req.random_id, 620, gif_doc(800, "video/mp4", vec![1]))
            }
            x if x == cid::<messages::SendMedia>() => {
                let req: messages::SendMedia = decode(body);
                let tl::enums::InputMedia::Document(d) = req.media else { panic!() };
                let tl::enums::InputDocument::Document(doc) = d.id else { panic!() };
                assert_eq!(doc.id, 801, "consulta caducada: se envía el documento");
                sent(req.random_id, 621, gif_doc(801, "image/gif", vec![1]))
            }
            other => panic!("llamada inesperada {}", tl::name_for_id(other)),
        },
        None,
    )
    .await;
    b.inner.ingest(&[user_enum(10, "Ana", 100, false)], &[]);
    let peer = PeerRef::User(UserId(10));

    let p = b.search_gifs("gato".into(), String::new()).await.unwrap();
    assert_eq!(p.next_offset.as_deref(), Some("6"));
    assert_eq!(p.gifs.iter().map(|g| g.id).collect::<Vec<_>>(), [800, 801], "el resultado sin documento se omite");
    assert_eq!(p.gifs[0].inline, Some(InlineResult { query_id: 31337, result_id: "a1".into() }));
    let p2 = b.search_gifs("gato".into(), "6".into()).await.unwrap();
    assert_eq!(p2.next_offset, None);
    assert_eq!(fake.count("contacts.resolveUsername"), 1, "el bot se resuelve una vez");

    let m = b.send_gif(peer, p.gifs[0].clone(), None, RandomId(20)).await.unwrap();
    assert_eq!((m.id, m.media.map(|md| md.kind)), (MsgId(620), Some(MediaKind::Gif)));
    let m = b.send_gif(peer, p.gifs[1].clone(), None, RandomId(21)).await.unwrap();
    assert_eq!(m.id, MsgId(621));
    assert_eq!((fake.count("messages.sendInlineBotResult"), fake.count("messages.sendMedia")), (2, 1));
}
