use super::*;
use futures::StreamExt;

const NOW: i64 = 1_800_000_000;

fn mock() -> MockBackend {
    MockBackend::new(MockConfig { now: Some(NOW), secret_accept_delay: Duration::from_millis(20), ..Default::default() })
}

fn page(archived: bool) -> DialogsPage {
    DialogsPage { offset: 0, limit: 1000, archived }
}

fn text(t: &str, rid: i64) -> OutgoingText {
    OutgoingText { text: t.into(), entities: vec![], reply_to: None, random_id: RandomId(rid) }
}

fn user_peer(i: i64) -> PeerRef {
    PeerRef::User(UserId(1000 + i))
}

#[tokio::test]
async fn about_thirty_chats_sorted_with_pinned_first() {
    let b = mock();
    let mut all = b.dialogs(page(false)).await.unwrap();
    all.extend(b.dialogs(page(true)).await.unwrap());
    // 29 de `data.rs` + el grupo con temas de `forum.rs`.
    assert_eq!(all.len(), 30);
    let main = b.dialogs(page(false)).await.unwrap();
    let pinned = main.iter().take_while(|d| d.pinned_order.is_some()).count();
    assert_eq!(pinned, 3);
    assert!(main[pinned..].windows(2).all(|w| w[0].activity >= w[1].activity));
    assert!(main.iter().any(|d| matches!(d.peer, PeerRef::Secret(_))));
    assert!(main.iter().all(|d| d.last_message.is_some()));
}

#[tokio::test]
async fn pagination_of_dialogs() {
    let b = mock();
    let a = b.dialogs(DialogsPage { offset: 0, limit: 5, archived: false }).await.unwrap();
    let c = b.dialogs(DialogsPage { offset: 5, limit: 5, archived: false }).await.unwrap();
    assert_eq!((a.len(), c.len()), (5, 5));
    assert_ne!(a[0].peer, c[0].peer);
}

#[tokio::test]
async fn data_is_deterministic() {
    let (a, b) = (mock(), mock());
    let ha = a.history(BIG_CHAT, Anchor::Latest, 50).await.unwrap();
    let hb = b.history(BIG_CHAT, Anchor::Latest, 50).await.unwrap();
    assert_eq!(ha, hb);
}

#[tokio::test]
async fn long_history_paging_covers_everything_without_gaps() {
    let b = mock();
    let mut got = b.history(BIG_CHAT, Anchor::Latest, 200).await.unwrap();
    assert_eq!(got.len(), 200);
    assert_eq!(got.last().unwrap().id, MsgId(BIG_CHAT_LEN as i32));
    while got[0].id.0 > 1 {
        let mut older = b.history(BIG_CHAT, Anchor::Before(got[0].id), 200).await.unwrap();
        older.extend(got);
        got = older;
    }
    assert_eq!(got.len(), BIG_CHAT_LEN);
    assert!(got.windows(2).all(|w| w[1].id.0 == w[0].id.0 + 1 && w[1].date >= w[0].date));
}

#[tokio::test]
async fn history_after_and_around() {
    let b = mock();
    let after = b.history(BIG_CHAT, Anchor::After(MsgId(100)), 10).await.unwrap();
    assert_eq!(after.first().unwrap().id, MsgId(101));
    assert_eq!(after.len(), 10);
    let around = b.history(BIG_CHAT, Anchor::Around(MsgId(2500)), 20).await.unwrap();
    assert!(around.iter().any(|m| m.id == MsgId(2500)));
    assert_eq!(around.len(), 20);
    assert!(b.history(BIG_CHAT, Anchor::After(MsgId(BIG_CHAT_LEN as i32)), 10).await.unwrap().is_empty());
}

#[tokio::test]
async fn send_emits_events_and_moves_chat_to_top() {
    let b = mock();
    let mut ev = b.events();
    let peer = user_peer(15);
    let m = b.send_text(peer, text("Hola, ¿qué tal?", 1)).await.unwrap();
    assert!(m.outgoing && m.text == "Hola, ¿qué tal?");
    assert!(matches!(ev.next().await, Some(BackendEvent::MessageSent { random_id: RandomId(1), .. })));
    assert!(matches!(ev.next().await, Some(BackendEvent::DialogUpserted(d)) if d.peer == peer));
    let list = b.dialogs(page(false)).await.unwrap();
    let first_unpinned = list.iter().find(|d| d.pinned_order.is_none()).unwrap();
    assert_eq!(first_unpinned.peer, peer);
}

#[tokio::test]
async fn send_is_idempotent_by_random_id() {
    let b = mock();
    let peer = user_peer(3);
    let a = b.send_text(peer, text("uno", 77)).await.unwrap();
    let c = b.send_text(peer, text("uno", 77)).await.unwrap();
    assert_eq!(a.id, c.id);
    let last = b.history(peer, Anchor::Latest, 5).await.unwrap();
    assert_eq!(last.iter().filter(|m| m.random_id == Some(RandomId(77))).count(), 1);
}

#[tokio::test]
async fn send_validates_input_and_peer() {
    let b = mock();
    assert_eq!(b.send_text(user_peer(1), text("   ", 1)).await, Err(BackendError::InvalidInput("text")));
    assert_eq!(b.send_text(PeerRef::User(UserId(42)), text("x", 2)).await, Err(BackendError::NotFound));
    // Canal: solo lectura.
    let channel = PeerRef::Group(GroupId(5005));
    assert!(matches!(b.send_text(channel, text("x", 3)).await, Err(BackendError::Forbidden(_))));
}

#[tokio::test]
async fn reply_must_reference_existing_message() {
    let b = mock();
    let peer = user_peer(2);
    let target = b.history(peer, Anchor::Latest, 1).await.unwrap()[0].id;
    let mut t = text("respuesta", 5);
    t.reply_to = Some(target);
    assert_eq!(b.send_text(peer, t).await.unwrap().reply_to, Some(target));
    let mut bad = text("respuesta", 6);
    bad.reply_to = Some(MsgId(999_999));
    assert_eq!(b.send_text(peer, bad).await, Err(BackendError::NotFound));
}

#[tokio::test]
async fn edit_only_own_messages() {
    let b = mock();
    let peer = user_peer(4);
    let mine = b.send_text(peer, text("original", 9)).await.unwrap();
    let ed = b.edit_message(peer, mine.id, "corregido".into(), vec![]).await.unwrap();
    assert_eq!(ed.text, "corregido");
    assert!(ed.edit_date.is_some());
    let d = b.dialogs(page(false)).await.unwrap().into_iter().find(|d| d.peer == peer).unwrap();
    assert_eq!(d.last_message.unwrap().text, "corregido");
    let theirs = b.history(peer, Anchor::Latest, 100).await.unwrap().into_iter().find(|m| !m.outgoing).unwrap();
    assert!(matches!(b.edit_message(peer, theirs.id, "x".into(), vec![]).await, Err(BackendError::Forbidden(_))));
}

#[tokio::test]
async fn delete_updates_last_message_and_emits() {
    let b = mock();
    let peer = user_peer(6);
    let m = b.send_text(peer, text("borrame", 11)).await.unwrap();
    let mut ev = b.events();
    b.delete_messages(peer, vec![m.id], true).await.unwrap();
    assert!(matches!(ev.next().await, Some(BackendEvent::MessagesDeleted { ids, .. }) if ids == vec![m.id]));
    let hist = b.history(peer, Anchor::Latest, 1000).await.unwrap();
    assert!(hist.iter().all(|x| x.id != m.id));
    let d = b.dialogs(page(false)).await.unwrap().into_iter().find(|d| d.peer == peer).unwrap();
    assert_ne!(d.last_message.unwrap().id, m.id);
}

#[tokio::test]
async fn reactions_toggle_and_replace() {
    let b = mock();
    let peer = user_peer(7);
    let m = b.send_text(peer, text("reacciona", 12)).await.unwrap();
    let mut ev = b.events();
    b.send_reaction(peer, m.id, Some("👍".into())).await.unwrap();
    assert!(matches!(ev.next().await, Some(BackendEvent::Reactions { reactions, .. }) if reactions == vec![ReactionCount { emoji: "👍".into(), count: 1, mine: true }]));
    b.send_reaction(peer, m.id, Some("❤️".into())).await.unwrap();
    let r = b.history(peer, Anchor::Latest, 1).await.unwrap()[0].reactions.clone();
    assert_eq!(r.len(), 1);
    assert_eq!(r[0].emoji, "❤️");
    b.send_reaction(peer, m.id, None).await.unwrap();
    assert!(b.history(peer, Anchor::Latest, 1).await.unwrap()[0].reactions.is_empty());
}

#[tokio::test]
async fn mark_read_clears_unread() {
    let b = mock();
    let peer = user_peer(8);
    b.inject_incoming(peer, "nuevo").unwrap();
    let before = b.dialogs(page(false)).await.unwrap().into_iter().find(|d| d.peer == peer).unwrap();
    assert!(before.unread_count >= 1);
    let last = before.last_message.unwrap().id;
    b.mark_read(peer, last).await.unwrap();
    let after = b.dialogs(page(false)).await.unwrap().into_iter().find(|d| d.peer == peer).unwrap();
    assert_eq!(after.unread_count, 0);
}

#[tokio::test]
async fn pin_archive_mute_draft() {
    let b = mock();
    let peer = user_peer(9);
    b.set_pinned(peer, true).await.unwrap();
    b.set_muted(peer, true).await.unwrap();
    b.save_draft(peer, Some(Draft { text: "pendiente".into(), reply_to: None })).await.unwrap();
    let d = b.dialogs(page(false)).await.unwrap().into_iter().find(|d| d.peer == peer).unwrap();
    assert!(d.pinned_order.is_some() && d.muted);
    assert_eq!(d.draft.unwrap().text, "pendiente");
    b.set_archived(peer, true).await.unwrap();
    assert!(b.dialogs(page(true)).await.unwrap().iter().any(|d| d.peer == peer && d.pinned_order.is_none()));
    assert!(!b.dialogs(page(false)).await.unwrap().iter().any(|d| d.peer == peer));
}

#[tokio::test]
async fn forward_creates_messages_with_origin() {
    let b = mock();
    let from = user_peer(10);
    let src = b.history(from, Anchor::Latest, 1).await.unwrap()[0].id;
    let to = user_peer(11);
    b.forward_messages(from, vec![src], vec![to], false).await.unwrap();
    let m = b.history(to, Anchor::Latest, 1).await.unwrap().remove(0);
    assert!(m.outgoing && m.forwarded_from.is_some());
    b.forward_messages(from, vec![src], vec![to], true).await.unwrap();
    assert!(b.history(to, Anchor::Latest, 1).await.unwrap()[0].forwarded_from.is_none());
}

#[tokio::test]
async fn media_and_stickers() {
    let b = mock();
    let peer = user_peer(12);
    let item = OutgoingMediaItem { kind: MediaKind::Photo, path: "/tmp/a.jpg".into(), file_name: "a.jpg".into(), size_bytes: 1000, width: 800, height: 600 };
    let out = b
        .send_media(peer, OutgoingMedia { items: vec![item.clone(), item], caption: "Mirad".into(), reply_to: None, random_ids: vec![RandomId(21), RandomId(22)] })
        .await
        .unwrap();
    assert_eq!(out.len(), 2);
    assert_eq!(out[0].text, "Mirad");
    assert!(out[1].text.is_empty());
    let sets = b.installed_sticker_sets().await.unwrap();
    let st = sets[0].stickers[0].id;
    let m = b.send_sticker(peer, st, None, RandomId(23)).await.unwrap();
    assert_eq!(m.sticker.unwrap().id, st);
    assert_eq!(b.send_sticker(peer, StickerId(1), None, RandomId(24)).await, Err(BackendError::NotFound));
    // Pack propio de tc-stickers + su variante animada.
    assert_eq!(sets[0].id, tc_stickers::pack::SET_ID);
    assert_eq!(sets[1].id, crate::ANIMATED_SET_ID);
    assert!(sets[1].stickers.iter().all(|s| s.format == StickerFormat::Lottie));
    let anim = sets[1].stickers[3].id;
    assert_eq!(b.send_sticker(peer, anim, None, RandomId(25)).await.unwrap().sticker.unwrap().id, anim);
    assert_eq!(b.send_media(peer, OutgoingMedia { items: vec![], caption: String::new(), reply_to: None, random_ids: vec![] }).await, Err(BackendError::InvalidInput("items")));
}

#[tokio::test]
async fn offline_mode_fails_with_network() {
    let b = mock();
    let mut ev = b.events();
    b.set_offline(true);
    assert!(matches!(ev.next().await, Some(BackendEvent::Connection(ConnectionState::Offline))));
    assert_eq!(b.dialogs(page(false)).await, Err(BackendError::Network));
    b.set_offline(false);
    assert!(b.dialogs(page(false)).await.is_ok());
}

#[tokio::test]
async fn secret_chat_seed_and_flow() {
    let b = mock();
    let chats = b.secret_chats().await.unwrap();
    assert_eq!(chats.len(), 1);
    assert_eq!(chats[0].state, SecretChatState::Accepted);
    let peer = PeerRef::Secret(SECRET_ID);
    assert!(b.history(peer, Anchor::Latest, 5).await.unwrap().iter().all(|m| m.ttl_secs == Some(60)));
    b.set_secret_ttl(SECRET_ID, Some(5)).await.unwrap();
    assert_eq!(b.send_text(peer, text("secreto", 31)).await.unwrap().ttl_secs, Some(5));
    let kv = b.secret_key_visual(SECRET_ID).await.unwrap();
    assert_eq!(kv.emoji.len(), 4);

    let mut ev = b.events();
    let new = b.request_secret_chat(UserId(1010)).await.unwrap();
    assert_eq!(new.state, SecretChatState::Waiting);
    assert!(matches!(b.send_text(PeerRef::Secret(new.id), text("aún no", 32)).await, Err(BackendError::Forbidden(_))));
    // Tras el retardo llega la aceptación.
    loop {
        if let Some(BackendEvent::SecretChat(c)) = ev.next().await
            && c.id == new.id
            && c.state == SecretChatState::Accepted
        {
            break;
        }
    }
    assert!(b.send_text(PeerRef::Secret(new.id), text("ya sí", 33)).await.is_ok());
    b.discard_secret_chat(new.id).await.unwrap();
    assert!(matches!(b.send_text(PeerRef::Secret(new.id), text("no", 34)).await, Err(BackendError::Forbidden(_))));
    assert!(matches!(b.accept_secret_chat(new.id).await, Err(BackendError::Forbidden(_))));
}

#[tokio::test(start_paused = true)]
async fn script_delivers_incoming_messages() {
    let b = mock();
    let mut ev = b.events();
    b.start_script(Duration::from_secs(4));
    let mut got_msg = false;
    for _ in 0..8 {
        if let Some(BackendEvent::MessageNew(m)) = ev.next().await {
            assert!(!m.outgoing);
            got_msg = true;
            break;
        }
    }
    assert!(got_msg);
    b.stop_script();
}

#[tokio::test(start_paused = true)]
async fn latency_is_applied() {
    let b = MockBackend::new(MockConfig { latency: Duration::from_millis(80), now: Some(NOW), ..Default::default() });
    let t0 = tokio::time::Instant::now();
    b.me().await.unwrap();
    assert!(t0.elapsed() >= Duration::from_millis(80));
}

#[tokio::test]
async fn profile_and_contacts() {
    let b = mock();
    assert_eq!(b.me().await.unwrap().id, ME);
    assert_eq!(b.contacts().await.unwrap().len(), 20);
    assert_eq!(b.user(UserId(99)).await, Err(BackendError::NotFound));
    assert_eq!(b.group(GroupId(5000)).await.unwrap().title, "Cena de los viernes");
}

#[tokio::test]
async fn simulated_login_phone_code_and_2fa() {
    let b = MockBackend::new(MockConfig { now: Some(NOW), logged_in: false, ..Default::default() });
    assert_eq!(b.auth_state().await.unwrap(), AuthState::LoggedOut);
    let mut ev = b.auth_events();
    assert_eq!(b.request_login_code("abc".into()).await, Err(BackendError::InvalidInput("phone")));
    assert_eq!(b.request_login_code("+34600000009".into()).await, Err(BackendError::FloodWait(crate::MOCK_FLOOD_SECS)));
    let req = b.request_login_code("+34600000002".into()).await.unwrap();
    assert_eq!(req.kind, CodeKind::App);
    assert!(matches!(ev.next().await, Some(AuthState::WaitingCode(_))));
    assert_eq!(b.sign_in("00000".into()).await, Err(BackendError::InvalidInput("code")));
    let r = b.sign_in(crate::MOCK_LOGIN_CODE.into()).await.unwrap();
    assert!(matches!(r, SignInResult::PasswordNeeded { .. }));
    assert_eq!(b.check_password("mala".into()).await, Err(BackendError::InvalidInput("password")));
    let me = b.check_password(crate::MOCK_2FA_PASSWORD.into()).await.unwrap();
    assert_eq!(me.id, ME);
    assert!(matches!(b.auth_state().await.unwrap(), AuthState::LoggedIn(u) if u.id == ME));
    b.log_out().await.unwrap();
    assert_eq!(b.auth_state().await.unwrap(), AuthState::LoggedOut);
}

#[tokio::test(start_paused = true)]
async fn simulated_qr_login_is_accepted_later() {
    let b = MockBackend::new(MockConfig { now: Some(NOW), logged_in: false, ..Default::default() });
    let mut ev = b.auth_events();
    let tok = b.qr_login_start().await.unwrap();
    assert!(tok.url.starts_with("tg://login?token="));
    assert!(matches!(ev.next().await, Some(AuthState::WaitingQr(_))));
    assert!(matches!(ev.next().await, Some(AuthState::LoggedIn(u)) if u.id == ME));
}

#[tokio::test]
async fn grupo_con_muchos_no_leidos_y_lectura_parcial() {
    let m = mock();
    let d = m.dialogs(page(false)).await.unwrap().into_iter().find(|d| d.peer == UNREAD_CHAT).expect("visible en la lista");
    assert!(d.unread_count as usize > crate::data::UNREAD_CHAT_COUNT / 2);
    // `read_inbox_max_id` coherente con el contador: tantos entrantes por encima como no leídos.
    let all = m.history(UNREAD_CHAT, Anchor::Latest, 10_000).await.unwrap();
    let above: Vec<&Message> = all.iter().filter(|x| !x.outgoing && x.id > d.read_inbox_max_id).collect();
    assert_eq!(above.len() as u32, d.unread_count);
    // La ventana alrededor del último leído trae contexto y los primeros no leídos.
    let w = m.history(UNREAD_CHAT, Anchor::Around(d.read_inbox_max_id), 60).await.unwrap();
    assert!(w.iter().any(|x| x.id < d.read_inbox_max_id) && w.iter().any(|x| x.id > d.read_inbox_max_id));
    // Leer a medias deja el resto sin leer.
    m.mark_read(UNREAD_CHAT, above[9].id).await.unwrap();
    let d2 = m.dialogs(page(false)).await.unwrap().into_iter().find(|d| d.peer == UNREAD_CHAT).unwrap();
    assert_eq!((d2.unread_count, d2.read_inbox_max_id), (d.unread_count - 10, above[9].id));
}
