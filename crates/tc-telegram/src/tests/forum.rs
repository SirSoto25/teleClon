//! Grupos con temas contra el servidor falso: peticiones TL exactas (`getForumTopics`, `getReplies`,
//! `reply_to` con `top_msg_id`, `readDiscussion`) y enrutado de los updates en vivo a su tema.
use std::collections::VecDeque;
use std::sync::{Arc, Mutex};

use futures::StreamExt;
use grammers_tl_types as tl;
use tc_core::*;

use super::fixtures::*;
use super::flows::restored;
use super::*;
use crate::events::{Cache, apply_batch};
use crate::ids::channel_group;

const CH: i64 = 20;

fn forum_channel() -> tl::enums::Chat {
    let mut c = channel(CH, "Huertos", false, 200);
    if let tl::enums::Chat::Channel(ch) = &mut c {
        ch.forum = true;
    }
    c
}

fn group() -> GroupId {
    channel_group(CH)
}

fn header(msg: Option<i32>, top: Option<i32>) -> tl::enums::MessageReplyHeader {
    tl::types::MessageReplyHeader {
        reply_to_scheduled: false,
        forum_topic: true,
        quote: false,
        reply_to_ephemeral: false,
        reply_to_msg_id: msg,
        reply_to_peer_id: None,
        reply_from: None,
        reply_media: None,
        reply_to_top_id: top,
        quote_text: None,
        quote_entities: None,
        quote_offset: None,
        todo_item_id: None,
        poll_option: None,
    }
    .into()
}

/// Mensaje del foro en el tema `topic` (`reply` = respuesta a otro mensaje del tema).
fn topic_msg(id: i32, topic: i32, reply: Option<i32>, text: &str, out: bool) -> tl::enums::Message {
    let mut m = message(id, peer_channel(CH), text, out, Some(peer_user(if out { ME_ID } else { 10 })), 1_000 + id);
    m.reply_to = Some(match reply {
        Some(r) => header(Some(r), Some(topic)),
        None => header(Some(topic), None),
    });
    m.into()
}

fn tl_topic(id: i32, title: &str, top: i32, unread: i32, read_in: i32, pinned: bool) -> tl::enums::ForumTopic {
    tl::types::ForumTopic {
        my: false,
        closed: false,
        pinned,
        short: false,
        hidden: false,
        title_missing: false,
        id,
        date: 500,
        peer: peer_channel(CH),
        title: title.into(),
        icon_color: 0x6FB9F0,
        icon_emoji_id: None,
        top_message: top,
        read_inbox_max_id: read_in,
        read_outbox_max_id: 0,
        unread_count: unread,
        unread_mentions_count: 0,
        unread_reactions_count: 0,
        unread_poll_votes_count: 0,
        from_id: peer_user(10),
        notify_settings: notify(None),
        draft: None,
    }
    .into()
}

fn is_channel(p: &tl::enums::InputPeer) -> bool {
    matches!(p, tl::enums::InputPeer::Channel(c) if c.channel_id == CH && c.access_hash == 200)
}

async fn setup(handler: impl FnMut(u32, &[u8]) -> Reply + Send + 'static) -> (GrammersBackend, Arc<Fake>) {
    let mut h = handler;
    let (b, fake, _) = restored(move |_, id, body| h(id, body), None).await;
    b.inner.ingest(&[user_enum(10, "Ana", 100, false)], &[forum_channel()]);
    (b, fake)
}

#[tokio::test]
async fn el_canal_con_forum_es_un_foro_y_las_respuestas_al_tema_no_son_respuestas() {
    let (b, _) = setup(|id, _| panic!("llamada inesperada {}", tl::name_for_id(id))).await;
    assert!(b.group(group()).await.unwrap().is_forum);
    let c = b.inner.cache();
    let suelto = crate::convert::message_from_tl(&topic_msg(30, 10, None, "hola", false), c.me, &c.peers).unwrap();
    let respuesta = crate::convert::message_from_tl(&topic_msg(31, 10, Some(30), "¿qué tal?", false), c.me, &c.peers).unwrap();
    assert_eq!((suelto.reply_to, respuesta.reply_to), (None, Some(MsgId(30))));
    assert_eq!(crate::forum::topic_of(&topic_msg(31, 10, Some(30), "x", false)), MsgId(10));
    assert_eq!(crate::forum::topic_of(&msg_enum(40, peer_channel(CH), "general", false, 1)), GENERAL_TOPIC);
}

#[tokio::test]
async fn lista_de_temas_paginada_e_historial_con_get_replies() {
    use tl::functions::messages;
    let seen = Arc::new(Mutex::new(Vec::new()));
    let s2 = seen.clone();
    let (b, fake) = setup(move |id, body| match id {
        x if x == cid::<messages::GetForumTopics>() => {
            let req: messages::GetForumTopics = decode(body);
            assert!(is_channel(&req.peer));
            s2.lock().unwrap().push((req.q.clone(), req.offset_date, req.offset_id, req.offset_topic, req.limit));
            ok(tl::enums::messages::ForumTopics::from(tl::types::messages::ForumTopics {
                order_by_create_date: false,
                count: 7,
                topics: vec![tl_topic(1, "General", 41, 0, 41, false), tl_topic(10, "Riego", 33, 2, 31, true)],
                messages: vec![msg_enum(41, peer_channel(CH), "último del general", false, 1_400), topic_msg(33, 10, None, "último de riego", false)],
                chats: vec![forum_channel()],
                users: vec![user_enum(10, "Ana", 100, false)],
                pts: 1,
            }))
        }
        x if x == cid::<messages::GetReplies>() => {
            let req: messages::GetReplies = decode(body);
            assert!(is_channel(&req.peer));
            assert_eq!((req.msg_id, req.limit), (10, 50));
            ok(tl::enums::messages::Messages::from(tl::types::messages::ChannelMessages {
                inexact: false,
                pts: 1,
                count: 3,
                offset_id_offset: None,
                messages: vec![topic_msg(33, 10, Some(31), "respuesta", false), topic_msg(31, 10, None, "suelto", false), msg_enum(10, peer_channel(CH), "creación", false, 900)],
                topics: vec![],
                chats: vec![forum_channel()],
                users: vec![user_enum(10, "Ana", 100, false)],
            }))
        }
        other => panic!("llamada inesperada {}", tl::name_for_id(other)),
    })
    .await;
    let page = b.forum_topics(group(), TopicsQuery::first(2)).await.unwrap();
    assert_eq!((page.topics.len(), page.total), (2, 7));
    let riego = &page.topics[1];
    assert_eq!((riego.id, riego.title.as_str(), riego.unread_count, riego.read_inbox_max_id, riego.pinned), (MsgId(10), "Riego", 2, MsgId(31), true));
    assert_eq!(riego.icon.color, 0x6FB9F0);
    assert_eq!(riego.last_message.as_ref().map(|m| (m.id, m.reply_to)), Some((MsgId(33), None)));
    let next = page.next.expect("página llena: hay más");
    assert_eq!((next.topic, next.msg_id), (MsgId(10), MsgId(33)));
    b.forum_topics(group(), TopicsQuery { query: Some("rie".into()), offset: Some(next), limit: 2 }).await.unwrap();
    assert_eq!(seen.lock().unwrap()[1], (Some("rie".into()), 1_033, 33, 10, 2));

    let h = b.topic_history(riego.thread(), Anchor::Latest, 50).await.unwrap();
    assert_eq!(h.iter().map(|m| m.id.0).collect::<Vec<_>>(), [10, 31, 33]);
    assert_eq!(h[2].reply_to, Some(MsgId(31)));
    assert_eq!(fake.count("messages.getReplies"), 1);
}

#[tokio::test]
async fn envio_en_tema_lleva_top_msg_id_y_actualiza_el_tema() {
    use tl::functions::messages;
    let replies = Arc::new(Mutex::new(Vec::new()));
    let r2 = replies.clone();
    let (b, fake) = setup(move |id, body| match id {
        x if x == cid::<messages::SendMessage>() => {
            let req: messages::SendMessage = decode(body);
            let rt = req.reply_to.map(|r| match r {
                tl::enums::InputReplyTo::Message(m) => (m.reply_to_msg_id, m.top_msg_id),
                _ => panic!("se esperaba respuesta a mensaje"),
            });
            r2.lock().unwrap().push(rt);
            let id = 50 + r2.lock().unwrap().len() as i32;
            let topic = rt.map_or(1, |(_, top)| top.unwrap_or(1));
            let mut m = topic_msg(id, topic, rt.and_then(|(r, _)| (r != topic).then_some(r)), &req.message, true);
            if topic == 1 && let tl::enums::Message::Message(mm) = &mut m {
                mm.reply_to = None;
            }
            ok(updates(
                vec![
                    tl::types::UpdateMessageId { id, random_id: req.random_id }.into(),
                    tl::types::UpdateNewChannelMessage { message: m, pts: 2, pts_count: 1 }.into(),
                ],
                vec![],
                vec![forum_channel()],
            ))
        }
        x if x == cid::<messages::ReadDiscussion>() => {
            let req: messages::ReadDiscussion = decode(body);
            assert!(is_channel(&req.peer));
            assert_eq!((req.msg_id, req.read_max_id), (10, 51));
            ok(true)
        }
        other => panic!("llamada inesperada {}", tl::name_for_id(other)),
    })
    .await;
    // Tema conocido en caché (como tras `forum_topics`).
    let mut riego = ForumTopic::new(group(), MsgId(10), "Riego", 0);
    riego.unread_count = 3;
    b.inner.cache().forums.entry(group()).or_default().insert(10, riego.clone());
    let th = riego.thread();
    let mut ev = b.events();
    let out = |t: &str, reply: Option<MsgId>, rid| OutgoingText { text: t.into(), entities: vec![], reply_to: reply, random_id: RandomId(rid) };
    let m = b.send_topic_text(th, out("hola", None, 1)).await.unwrap();
    assert_eq!((m.id, m.reply_to, m.outgoing), (MsgId(51), None, true));
    b.send_topic_text(th, out("re", Some(MsgId(51)), 2)).await.unwrap();
    b.send_topic_text(ThreadRef { group: group(), topic: GENERAL_TOPIC }, out("al general", None, 3)).await.unwrap();
    // Fuera de un tema, el `reply_to` vuelve a ser el normal.
    assert_eq!(*replies.lock().unwrap(), vec![Some((10, Some(10))), Some((51, Some(10))), None]);
    assert!(matches!(ev.next().await, Some(BackendEvent::MessageSent { msg, .. }) if msg.id == MsgId(51)));
    assert!(matches!(ev.next().await, Some(BackendEvent::ForumTopicUpserted(t)) if t.id == MsgId(10) && t.last_message.as_ref().map(|l| l.id) == Some(MsgId(51))));
    b.mark_topic_read(th, MsgId(51)).await.unwrap();
    let t = b.inner.cache().forums[&group()][&10].clone();
    assert_eq!((t.unread_count, t.read_inbox_max_id), (0, MsgId(52)), "lo enviado por mí cuenta como leído");
    assert_eq!(fake.count("messages.readDiscussion"), 1);
}

#[test]
fn updates_en_vivo_van_a_su_tema() {
    let mut cache = Cache::default();
    let users = [user_enum(10, "Ana", 100, false)];
    let chats = [forum_channel()];
    let _ = apply_batch(&mut cache, &[], &users, &chats, 0);
    let mut riego = ForumTopic::new(group(), MsgId(10), "Riego", 0);
    riego.read_inbox_max_id = MsgId(20);
    cache.forums.entry(group()).or_default().insert(10, riego);
    let new = |m| -> tl::enums::Update { tl::types::UpdateNewChannelMessage { message: m, pts: 1, pts_count: 1 }.into() };
    let (a, _) = apply_batch(&mut cache, &[new(topic_msg(60, 10, Some(55), "en riego", false))], &users, &chats, 0);
    assert!(matches!(&a.events[0], BackendEvent::TopicMessageNew { topic, msg } if *topic == MsgId(10) && msg.reply_to == Some(MsgId(55))));
    assert!(matches!(&a.events[1], BackendEvent::ForumTopicUpserted(t) if t.unread_count == 1 && t.last_message.as_ref().is_some_and(|m| m.id == MsgId(60))));
    assert!(matches!(&a.events[2], BackendEvent::DialogUpserted(_)));
    // Tema desconocido: se pide recargar la lista.
    let (a, _) = apply_batch(&mut cache, &[new(topic_msg(61, 77, None, "otro", false))], &users, &chats, 0);
    assert_eq!(a.events[1], BackendEvent::ForumTopicsChanged { group: group() });
    // Sin cabecera: el General.
    let (a, _) = apply_batch(&mut cache, &[new(msg_enum(62, peer_channel(CH), "general", false, 1))], &users, &chats, 0);
    assert!(matches!(&a.events[0], BackendEvent::TopicMessageNew { topic, .. } if *topic == GENERAL_TOPIC));
    // Lectura y fijado del tema.
    let read: tl::enums::Update =
        tl::types::UpdateReadChannelDiscussionInbox { channel_id: CH, top_msg_id: 10, read_max_id: 60, broadcast_id: None, broadcast_post: None }.into();
    let pin: tl::enums::Update = tl::types::UpdatePinnedForumTopic { pinned: true, peer: peer_channel(CH), topic_id: 10 }.into();
    let (a, _) = apply_batch(&mut cache, &[read, pin], &users, &chats, 0);
    assert!(matches!(&a.events[0], BackendEvent::ForumTopicUpserted(t) if t.unread_count == 0 && t.read_inbox_max_id == MsgId(60)));
    assert!(matches!(&a.events[1], BackendEvent::ForumTopicUpserted(t) if t.pinned));
    // Un supergrupo normal sigue dando `MessageNew`.
    let normal = [channel(21, "Charla", false, 210)];
    let (a, _) = apply_batch(&mut cache, &[new(msg_enum(5, peer_channel(21), "hola", false, 1))], &users, &normal, 0);
    assert!(matches!(&a.events[0], BackendEvent::MessageNew(_)));
}

#[tokio::test(start_paused = true)]
async fn la_fuente_de_updates_entrega_topic_message_new() {
    let batch = (vec![tl::types::UpdateNewChannelMessage { message: topic_msg(70, 10, None, "en vivo", false), pts: 1, pts_count: 1 }.into()], vec![user_enum(10, "Ana", 100, false)], vec![forum_channel()]);
    let src = ScriptSource(VecDeque::from(vec![Ok(batch)]));
    let store = Arc::new(MemorySessionStore::default());
    let fake = Fake::new(move |_, id, _| {
        if id == cid::<tl::functions::users::GetUsers>() {
            return ok(vec![user_enum(ME_ID, "Yo", 1, true)]);
        }
        panic!("llamada inesperada {}", tl::name_for_id(id))
    });
    let b = GrammersBackend::with_transport(cfg(), store, logged_session(), fake, Some(Box::new(src)));
    let mut ev = b.events();
    loop {
        match ev.next().await.unwrap() {
            BackendEvent::TopicMessageNew { topic, msg } => {
                assert_eq!((topic, msg.text.as_str(), msg.reply_to), (MsgId(10), "en vivo", None));
                break;
            }
            BackendEvent::MessageNew(_) => panic!("en un foro no debe llegar `MessageNew`"),
            _ => {}
        }
    }
}
