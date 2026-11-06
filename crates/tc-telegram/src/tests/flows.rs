//! Flujos completos contra el transporte falso (sin red).
use std::collections::VecDeque;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;

use futures::StreamExt;
use grammers_client::InvocationError;
use grammers_session::Session;
use grammers_tl_types as tl;
use tc_core::*;

use super::fixtures::*;
use super::*;
use crate::ids::channel_group;
use crate::session::SessionStore;

fn sent_code(hash: &str) -> tl::enums::auth::SentCode {
    tl::types::auth::SentCode {
        r#type: tl::types::auth::SentCodeTypeApp { length: 5 }.into(),
        phone_code_hash: hash.into(),
        next_type: Some(tl::enums::auth::CodeType::Sms),
        timeout: Some(60),
    }
    .into()
}

#[tokio::test]
async fn phone_code_and_2fa_login_with_dc_migration() {
    use tl::functions::{account, auth, updates};
    let wrong_once = Arc::new(AtomicUsize::new(0));
    let w = wrong_once.clone();
    let fake = Fake::new(move |dc, id, body| match id {
        x if x == cid::<auth::SendCode>() => {
            let req: auth::SendCode = decode(body);
            assert_eq!(req.api_id, 12345);
            if dc == 2 { rpc_err(303, "PHONE_MIGRATE", Some(4)) } else { ok(sent_code("hash-1")) }
        }
        x if x == cid::<auth::SignIn>() => {
            let req: auth::SignIn = decode(body);
            assert_eq!(req.phone_code_hash, "hash-1");
            assert_eq!(req.phone_code.as_deref(), Some("12345"));
            rpc_err(401, "SESSION_PASSWORD_NEEDED", None)
        }
        x if x == cid::<account::GetPassword>() => ok(password("mi pista")),
        x if x == cid::<auth::CheckPassword>() => {
            let req: auth::CheckPassword = decode(body);
            let tl::enums::InputCheckPasswordSrp::Srp(srp) = req.password else { panic!("SRP esperado") };
            assert_eq!(srp.srp_id, 4242);
            assert_eq!((srp.a.len(), srp.m1.len()), (256, 32));
            if w.fetch_add(1, Ordering::SeqCst) == 0 {
                rpc_err(400, "PASSWORD_HASH_INVALID", None)
            } else {
                ok(authorization(user(ME_ID, "Yo", 1, true)))
            }
        }
        x if x == cid::<updates::GetState>() => ok(state()),
        other => panic!("llamada inesperada {}", tl::name_for_id(other)),
    });
    let (b, store) = backend(Arc::new(TcSession::default()), fake.clone(), None);
    assert_eq!(wait_auth(&b, |s| *s == AuthState::LoggedOut).await, AuthState::LoggedOut);
    let mut ev = b.auth_events();

    assert_eq!(b.request_login_code("12".into()).await, Err(BackendError::InvalidInput("phone")));
    let req = b.request_login_code("+34 600 000 000".into()).await.unwrap();
    assert_eq!((req.kind, req.length, req.next, req.timeout_secs), (CodeKind::App, Some(5), Some(CodeKind::Sms), Some(60)));
    assert_eq!(req.phone, "+34600000000");
    // Migración: segundo intento en el DC 4, que pasa a ser el de casa.
    assert_eq!(fake.calls.lock().unwrap()[..2], [(2, "auth.sendCode"), (4, "auth.sendCode")]);
    assert_eq!(*fake.disconnected.lock().unwrap(), vec![2]);
    assert_eq!(b.inner.net.session.home_dc(), 4);
    assert!(matches!(ev.next().await, Some(AuthState::WaitingCode(_))));

    let r = b.sign_in("12345".into()).await.unwrap();
    assert_eq!(r, SignInResult::PasswordNeeded { hint: Some("mi pista".into()) });
    assert_eq!(ev.next().await, Some(AuthState::WaitingPassword { hint: Some("mi pista".into()) }));

    // Contraseña mala: error y se piden parámetros SRP nuevos.
    assert_eq!(b.check_password("mala".into()).await, Err(BackendError::InvalidInput("password")));
    assert_eq!(fake.count("account.getPassword"), 2);
    let me = b.check_password("buena".into()).await.unwrap();
    assert_eq!(me.id, UserId(ME_ID));
    assert!(matches!(wait_auth(&b, |s| matches!(s, AuthState::LoggedIn(_))).await, AuthState::LoggedIn(u) if u.id == UserId(ME_ID)));
    // Sesión guardada con la cuenta propia y el DC nuevo.
    let saved = TcSession::from_bytes(&store.snapshot().expect("sesión guardada")).unwrap();
    assert_eq!((saved.self_user_id(), saved.home_dc()), (Some(ME_ID), 4));
    assert_eq!(b.me().await.unwrap().first_name, "Yo");
}

#[tokio::test]
async fn invalid_code_keeps_waiting() {
    use tl::functions::auth;
    let fake = Fake::new(move |_, id, _| match id {
        x if x == cid::<auth::SendCode>() => ok(sent_code("h")),
        x if x == cid::<auth::SignIn>() => rpc_err(400, "PHONE_CODE_INVALID", None),
        other => panic!("llamada inesperada {}", tl::name_for_id(other)),
    });
    let (b, _) = backend(Arc::new(TcSession::default()), fake, None);
    wait_auth(&b, |s| *s == AuthState::LoggedOut).await;
    b.request_login_code("+34600000000".into()).await.unwrap();
    assert_eq!(b.sign_in("99999".into()).await, Err(BackendError::InvalidInput("code")));
    assert!(matches!(b.auth_state().await.unwrap(), AuthState::WaitingCode(_)));
}

#[tokio::test]
async fn qr_login_renews_migrates_and_completes() {
    use tl::functions::{auth, updates};
    let exports = Arc::new(AtomicUsize::new(0));
    let e = exports.clone();
    let fake = Fake::new(move |dc, id, body| match id {
        x if x == cid::<auth::ExportLoginToken>() => {
            let req: auth::ExportLoginToken = decode(body);
            assert_eq!(req.api_hash, "0123456789abcdef0123456789abcdef");
            if e.fetch_add(1, Ordering::SeqCst) == 0 {
                ok(tl::enums::auth::LoginToken::from(tl::types::auth::LoginToken { expires: (crate::now_secs() + 30) as i32, token: b"abc".to_vec() }))
            } else {
                ok(tl::enums::auth::LoginToken::from(tl::types::auth::LoginTokenMigrateTo { dc_id: 5, token: b"xyz".to_vec() }))
            }
        }
        x if x == cid::<auth::ImportLoginToken>() => {
            assert_eq!(dc, 5);
            let req: auth::ImportLoginToken = decode(body);
            assert_eq!(req.token, b"xyz");
            ok(tl::enums::auth::LoginToken::from(tl::types::auth::LoginTokenSuccess { authorization: authorization(user(ME_ID, "Yo", 1, true)) }))
        }
        x if x == cid::<updates::GetState>() => ok(state()),
        other => panic!("llamada inesperada {}", tl::name_for_id(other)),
    });
    let (b, _) = backend(Arc::new(TcSession::default()), fake.clone(), None);
    wait_auth(&b, |s| *s == AuthState::LoggedOut).await;
    let tok = b.qr_login_start().await.unwrap();
    assert_eq!(tok.url, "tg://login?token=YWJj");
    assert!(matches!(b.auth_state().await.unwrap(), AuthState::WaitingQr(_)));
    // Llega `updateLoginToken` (lo haría el bombeo de updates): se reexporta.
    b.inner.qr_wake.notify_one();
    let st = wait_auth(&b, |s| matches!(s, AuthState::LoggedIn(_))).await;
    assert!(matches!(st, AuthState::LoggedIn(u) if u.id == UserId(ME_ID)));
    assert_eq!(b.inner.net.session.home_dc(), 5);
    assert!(fake.names().contains(&"auth.importLoginToken"));
}

/// Backend con sesión restaurada y la cuenta propia confirmada por `users.getUsers`.
async fn restored(handler: impl FnMut(i32, u32, &[u8]) -> Reply + Send + 'static, src: Option<Box<dyn UpdateSource>>) -> (GrammersBackend, Arc<Fake>, Arc<MemorySessionStore>) {
    let mut h = handler;
    let fake = Fake::new(move |dc, id, body| {
        if id == cid::<tl::functions::users::GetUsers>() {
            let req: tl::functions::users::GetUsers = decode(body);
            if matches!(req.id[..], [tl::enums::InputUser::UserSelf]) {
                return ok(vec![user_enum(ME_ID, "Yo", 1, true)]);
            }
        }
        h(dc, id, body)
    });
    let (b, store) = backend(logged_session(), fake.clone(), src);
    wait_auth(&b, |s| matches!(s, AuthState::LoggedIn(_))).await;
    (b, fake, store)
}

fn dialogs_page(from: i64, n: i64, complete: bool) -> tl::enums::messages::Dialogs {
    let mut dialogs = vec![];
    let mut messages = vec![];
    let mut users = vec![];
    for i in 0..n {
        let uid = from + i;
        let top = (1000 - uid) as i32;
        dialogs.push(tl::enums::Dialog::Dialog(dialog(peer_user(uid), top, i as i32, uid == from && from == 1000, if i == 1 { Some(i32::MAX) } else { None })));
        messages.push(msg_enum(top, peer_user(uid), &format!("msg {uid}"), false, 2_000_000 - uid as i32));
        users.push(user_enum(uid, &format!("U{uid}"), uid * 10, false));
    }
    if complete {
        tl::types::messages::Dialogs { dialogs, messages, chats: vec![], users }.into()
    } else {
        tl::types::messages::DialogsSlice { count: 999, dialogs, messages, chats: vec![], users }.into()
    }
}

#[tokio::test]
async fn dialogs_paginate_with_server_offsets() {
    let (b, fake, _) = restored(
        |_, id, body| {
            assert_eq!(id, cid::<tl::functions::messages::GetDialogs>());
            let req: tl::functions::messages::GetDialogs = decode(body);
            assert_eq!(req.folder_id, None);
            if req.offset_id == 0 {
                assert!(matches!(req.offset_peer, tl::enums::InputPeer::Empty));
                ok(dialogs_page(1000, req.limit as i64, false))
            } else {
                // Continúa tras el último de la página anterior (usuario 1024, mensaje 1000-1024).
                assert_eq!(req.offset_id, 1000 - 1024);
                assert!(matches!(req.offset_peer, tl::enums::InputPeer::User(u) if u.user_id == 1024 && u.access_hash == 10240));
                assert_eq!(req.offset_date, 2_000_000 - 1024);
                ok(dialogs_page(2000, 3, true))
            }
        },
        None,
    )
    .await;
    let first = b.dialogs(DialogsPage { offset: 0, limit: 25, archived: false }).await.unwrap();
    assert_eq!(first.len(), 25);
    assert_eq!(first[0].pinned_order, Some(0));
    assert_eq!(first[0].title, "U1000");
    assert_eq!(first[0].last_message.as_ref().unwrap().text, "msg 1000");
    assert!(first[1].muted);
    assert_eq!(first[1].unread_count, 1);
    let rest = b.dialogs(DialogsPage { offset: 25, limit: 10, archived: false }).await.unwrap();
    assert_eq!(rest.iter().map(|d| d.peer).collect::<Vec<_>>(), (2000..2003).map(|u| PeerRef::User(UserId(u))).collect::<Vec<_>>());
    assert_eq!(fake.count("messages.getDialogs"), 2);
    // Ya agotada: no se vuelve a pedir.
    assert!(b.dialogs(DialogsPage { offset: 40, limit: 10, archived: false }).await.unwrap().is_empty());
    assert_eq!(fake.count("messages.getDialogs"), 2);
}

#[tokio::test]
async fn history_anchors_and_ascending_order() {
    let (b, _, _) = restored(
        |_, id, body| {
            assert_eq!(id, cid::<tl::functions::messages::GetHistory>());
            let req: tl::functions::messages::GetHistory = decode(body);
            assert!(matches!(req.peer, tl::enums::InputPeer::User(ref u) if u.access_hash == 100));
            let ids: Vec<i32> = match (req.offset_id, req.add_offset, req.min_id) {
                (50, 0, 0) => vec![49, 48, 47],
                (11, -3, 10) => vec![13, 12, 11, 10],
                (0, 0, 0) => vec![90],
                (31, -1, 0) => vec![31, 30, 29],
                other => panic!("parámetros inesperados {other:?}"),
            };
            assert_eq!(req.limit, if req.offset_id == 0 { 1 } else { 3 });
            let msgs = ids.into_iter().map(|i| msg_enum(i, peer_user(10), "x", false, 1)).collect();
            ok(tl::enums::messages::Messages::from(tl::types::messages::Messages { messages: msgs, topics: vec![], chats: vec![], users: vec![] }))
        },
        None,
    )
    .await;
    b.inner.ingest(&[user_enum(10, "Ana", 100, false)], &[]);
    let peer = PeerRef::User(UserId(10));
    let ids = |v: Vec<Message>| v.into_iter().map(|m| m.id.0).collect::<Vec<_>>();
    assert_eq!(ids(b.history(peer, Anchor::Before(MsgId(50)), 3).await.unwrap()), vec![47, 48, 49]);
    // After es exclusivo aunque el servidor devuelva el propio id.
    assert_eq!(ids(b.history(peer, Anchor::After(MsgId(10)), 3).await.unwrap()), vec![11, 12, 13]);
    assert_eq!(ids(b.history(peer, Anchor::Latest, 1).await.unwrap()), vec![90]);
    assert_eq!(ids(b.history(peer, Anchor::Around(MsgId(30)), 3).await.unwrap()), vec![29, 30, 31]);
    assert_eq!(b.history(PeerRef::User(UserId(404)), Anchor::Latest, 3).await, Err(BackendError::NotFound));
}

#[tokio::test]
async fn send_text_confirms_and_is_idempotent() {
    use tl::functions::messages;
    let fake_state = Arc::new(AtomicUsize::new(0));
    let s = fake_state.clone();
    let (b, fake, _) = restored(
        move |_, id, body| {
            assert_eq!(id, cid::<messages::SendMessage>());
            let req: messages::SendMessage = decode(body);
            assert!(req.clear_draft);
            if s.fetch_add(1, Ordering::SeqCst) == 0 {
                assert_eq!(req.random_id, 111);
                assert!(matches!(req.reply_to, Some(tl::enums::InputReplyTo::Message(ref r)) if r.reply_to_msg_id == 3));
                assert!(req.entities.is_some());
                ok(tl::enums::Updates::from(tl::types::UpdateShortSentMessage {
                    out: true,
                    id: 500,
                    pts: 1,
                    pts_count: 1,
                    date: 1234,
                    media: None,
                    entities: None,
                    ttl_period: None,
                }))
            } else {
                assert_eq!(req.random_id, 222);
                ok(updates(
                    vec![
                        tl::types::UpdateMessageId { id: 501, random_id: 222 }.into(),
                        tl::types::UpdateNewMessage { message: msg_enum(501, peer_user(10), "dos", true, 1235), pts: 2, pts_count: 1 }.into(),
                    ],
                    vec![],
                    vec![],
                ))
            }
        },
        None,
    )
    .await;
    b.inner.ingest(&[user_enum(10, "Ana", 100, false)], &[]);
    let peer = PeerRef::User(UserId(10));
    let mut ev = b.events();
    let bold = vec![TextEntity { kind: EntityKind::Bold, offset: 0, length: 3 }];
    let out = OutgoingText { text: "uno".into(), entities: bold, reply_to: Some(MsgId(3)), random_id: RandomId(111) };
    let m = b.send_text(peer, out.clone()).await.unwrap();
    assert_eq!((m.id, m.random_id, m.sender, m.date, m.text.as_str()), (MsgId(500), Some(RandomId(111)), Some(UserId(ME_ID)), Timestamp(1234), "uno"));
    assert!(matches!(ev.next().await, Some(BackendEvent::MessageSent { random_id: RandomId(111), .. })));
    assert!(matches!(ev.next().await, Some(BackendEvent::DialogUpserted(d)) if d.last_message.as_ref().unwrap().id == MsgId(500)));
    // Reintento con el mismo random_id: no se vuelve a enviar.
    assert_eq!(b.send_text(peer, out).await.unwrap().id, MsgId(500));
    assert_eq!(fake.count("messages.sendMessage"), 1);
    let m2 = b.send_text(peer, OutgoingText { text: "dos".into(), entities: vec![], reply_to: None, random_id: RandomId(222) }).await.unwrap();
    assert_eq!((m2.id, m2.random_id), (MsgId(501), Some(RandomId(222))));
    assert_eq!(b.send_text(peer, OutgoingText { text: "  ".into(), entities: vec![], reply_to: None, random_id: RandomId(3) }).await, Err(BackendError::InvalidInput("text")));
    assert_eq!(b.send_text(PeerRef::Secret(SecretChatId(1)), OutgoingText { text: "x".into(), entities: vec![], reply_to: None, random_id: RandomId(4) }).await, Err(BackendError::NotFound));
}

#[tokio::test]
async fn edit_delete_react_and_read() {
    use tl::functions::{channels, messages};
    let (b, fake, _) = restored(
        |_, id, body| match id {
            x if x == cid::<messages::EditMessage>() => {
                let req: messages::EditMessage = decode(body);
                let mut m = message(req.id, peer_user(10), req.message.as_deref().unwrap(), true, None, 1);
                m.edit_date = Some(2);
                ok(updates(vec![tl::types::UpdateEditMessage { message: m.into(), pts: 1, pts_count: 1 }.into()], vec![], vec![]))
            }
            x if x == cid::<channels::DeleteMessages>() => {
                let req: channels::DeleteMessages = decode(body);
                assert!(matches!(req.channel, tl::enums::InputChannel::Channel(ref c) if c.channel_id == 20 && c.access_hash == 200));
                ok(tl::enums::messages::AffectedMessages::from(tl::types::messages::AffectedMessages { pts: 1, pts_count: 1 }))
            }
            x if x == cid::<messages::SendReaction>() => {
                let req: messages::SendReaction = decode(body);
                let r = req.reaction.unwrap();
                assert!(matches!(&r[..], [tl::enums::Reaction::Emoji(e)] if e.emoticon == "🔥"));
                ok(updates(
                    vec![tl::types::UpdateMessageReactions {
                        peer: peer_user(10),
                        msg_id: req.msg_id,
                        top_msg_id: None,
                        saved_peer_id: None,
                        reactions: tl::types::MessageReactions {
                            min: false,
                            can_see_list: false,
                            reactions_as_tags: false,
                            results: vec![tl::types::ReactionCount { chosen_order: Some(1), reaction: tl::types::ReactionEmoji { emoticon: "🔥".into() }.into(), count: 1 }.into()],
                            recent_reactions: None,
                            top_reactors: None,
                        }
                        .into(),
                    }
                    .into()],
                    vec![],
                    vec![],
                ))
            }
            x if x == cid::<messages::ReadHistory>() => ok(tl::enums::messages::AffectedMessages::from(tl::types::messages::AffectedMessages { pts: 1, pts_count: 1 })),
            other => panic!("llamada inesperada {}", tl::name_for_id(other)),
        },
        None,
    )
    .await;
    b.inner.ingest(&[user_enum(10, "Ana", 100, false)], &[channel(20, "Noticias", true, 200)]);
    let peer = PeerRef::User(UserId(10));
    let mut ev = b.events();
    let m = b.edit_message(peer, MsgId(7), "nuevo".into(), vec![]).await.unwrap();
    assert_eq!((m.text.as_str(), m.edit_date), ("nuevo", Some(Timestamp(2))));
    assert!(matches!(ev.next().await, Some(BackendEvent::MessageEdited(_))));
    let ch = PeerRef::Group(channel_group(20));
    b.delete_messages(ch, vec![MsgId(1), MsgId(2)], true).await.unwrap();
    assert_eq!(ev.next().await, Some(BackendEvent::MessagesDeleted { peer: ch, ids: vec![MsgId(1), MsgId(2)] }));
    b.send_reaction(peer, MsgId(7), Some("🔥".into())).await.unwrap();
    assert!(matches!(ev.next().await, Some(BackendEvent::Reactions { reactions, .. }) if reactions[0].mine));
    b.mark_read(peer, MsgId(7)).await.unwrap();
    assert_eq!(ev.next().await, Some(BackendEvent::ReadInbox { peer, max_id: MsgId(7) }));
    assert_eq!(fake.count("messages.readHistory"), 1);
}

#[tokio::test(start_paused = true)]
async fn flood_wait_short_is_slept_long_is_returned() {
    let n = Arc::new(AtomicUsize::new(0));
    let c = n.clone();
    let (b, _, _) = restored(
        move |_, _, body| {
            let req: tl::functions::messages::GetHistory = decode(body);
            match (req.offset_id, c.fetch_add(1, Ordering::SeqCst)) {
                (1, 0) => rpc_err(420, "FLOOD_WAIT", Some(3)),
                (1, _) => ok(tl::enums::messages::Messages::from(tl::types::messages::Messages { messages: vec![], topics: vec![], chats: vec![], users: vec![] })),
                _ => rpc_err(420, "FLOOD_WAIT", Some(600)),
            }
        },
        None,
    )
    .await;
    b.inner.ingest(&[user_enum(10, "Ana", 100, false)], &[]);
    let peer = PeerRef::User(UserId(10));
    let t0 = tokio::time::Instant::now();
    assert_eq!(b.history(peer, Anchor::Before(MsgId(1)), 5).await, Ok(vec![]));
    assert!(t0.elapsed() >= Duration::from_secs(3));
    // Mayor que el umbral (10 s): a la UI, para su cuenta atrás.
    assert_eq!(b.history(peer, Anchor::Before(MsgId(2)), 5).await, Err(BackendError::FloodWait(600)));
}

#[tokio::test(start_paused = true)]
async fn network_errors_retry_with_backoff_then_report_offline() {
    let fail = Arc::new(std::sync::atomic::AtomicBool::new(false));
    let f = fail.clone();
    let (b, fake, _) = restored(
        move |_, _, _| {
            if f.load(Ordering::SeqCst) {
                Err(InvocationError::Dropped("conexión cerrada"))
            } else {
                ok(tl::enums::messages::Messages::from(tl::types::messages::Messages { messages: vec![], topics: vec![], chats: vec![], users: vec![] }))
            }
        },
        None,
    )
    .await;
    b.inner.ingest(&[user_enum(10, "Ana", 100, false)], &[]);
    let mut ev = b.events();
    fail.store(true, Ordering::SeqCst);
    let peer = PeerRef::User(UserId(10));
    assert_eq!(b.history(peer, Anchor::Latest, 5).await, Err(BackendError::Network));
    // 1 intento + 4 reintentos (100, 200, 400, 800 ms).
    assert_eq!(fake.count("messages.getHistory"), 5);
    assert_eq!(ev.next().await, Some(BackendEvent::Connection(ConnectionState::Connecting)));
    assert_eq!(ev.next().await, Some(BackendEvent::Connection(ConnectionState::Offline)));
    // Vuelve la red: la siguiente llamada reconecta y avisa.
    fail.store(false, Ordering::SeqCst);
    assert_eq!(b.history(peer, Anchor::Latest, 5).await, Ok(vec![]));
    assert_eq!(ev.next().await, Some(BackendEvent::Connection(ConnectionState::Online)));
}

#[tokio::test(start_paused = true)]
async fn update_pump_maps_events_and_recovers_from_errors() {
    let batch = (
        vec![tl::types::UpdateNewMessage { message: msg_enum(70, peer_user(10), "en vivo", false, 5), pts: 1, pts_count: 1 }.into()],
        vec![user_enum(10, "Ana", 100, false)],
        vec![],
    );
    let src = ScriptSource(VecDeque::from(vec![Err(InvocationError::Io(std::io::Error::other("caída"))), Ok(batch)]));
    let fake_ready = |_: i32, id: u32, _: &[u8]| -> Reply { panic!("llamada inesperada {}", tl::name_for_id(id)) };
    // La fuente arranca tras el login; se suscribe antes de que lleguen eventos.
    let store = Arc::new(MemorySessionStore::default());
    let fake = Fake::new(move |dc, id, body| {
        if id == cid::<tl::functions::users::GetUsers>() {
            return ok(vec![user_enum(ME_ID, "Yo", 1, true)]);
        }
        fake_ready(dc, id, body)
    });
    let b = GrammersBackend::with_transport(cfg(), store, logged_session(), fake, Some(Box::new(src)));
    let mut ev = b.events();
    let mut got = vec![];
    while got.len() < 5 {
        got.push(ev.next().await.unwrap());
    }
    assert_eq!(got[0], BackendEvent::Connection(ConnectionState::Online));
    assert_eq!(got[1], BackendEvent::Connection(ConnectionState::Offline));
    assert_eq!(got[2], BackendEvent::Connection(ConnectionState::Connecting));
    assert_eq!(got[3], BackendEvent::Connection(ConnectionState::Online));
    assert!(matches!(&got[4], BackendEvent::MessageNew(m) if m.text == "en vivo" && m.sender == Some(UserId(10))));
    assert!(matches!(ev.next().await, Some(BackendEvent::DialogUpserted(d)) if d.title == "Ana" && d.unread_count == 1));
}

#[tokio::test]
async fn revoked_session_logs_out_and_clears_store() {
    let (b, _, store) = restored(|_, _, _| rpc_err(401, "AUTH_KEY_UNREGISTERED", None), None).await;
    b.inner.ingest(&[user_enum(10, "Ana", 100, false)], &[]);
    store.save(b"algo").unwrap();
    assert_eq!(b.history(PeerRef::User(UserId(10)), Anchor::Latest, 5).await, Err(BackendError::Unauthorized));
    assert_eq!(b.auth_state().await.unwrap(), AuthState::LoggedOut);
    assert!(store.snapshot().is_none());
    assert!(!b.inner.net.session.has_auth_key());
    assert_eq!(b.me().await, Err(BackendError::Unauthorized));
}

#[tokio::test]
async fn startup_without_session_is_logged_out_and_secret_chats_need_login() {
    let fake = Fake::new(|_, id, _| panic!("no debería llamar a {}", tl::name_for_id(id)));
    let (b, _) = backend(Arc::new(TcSession::default()), fake.clone(), None);
    wait_auth(&b, |s| *s == AuthState::LoggedOut).await;
    assert!(fake.names().is_empty());
    assert_eq!(b.secret_chats().await, Ok(vec![]));
    assert_eq!(b.request_secret_chat(UserId(1)).await, Err(BackendError::Unauthorized));
    assert_eq!(b.accept_secret_chat(SecretChatId(1)).await, Err(BackendError::NotFound));
    assert_eq!(b.dialogs(DialogsPage { offset: 0, limit: 10, archived: false }).await, Err(BackendError::Unauthorized));
    let _ = b.inner.net.session.home_dc_id();
}
