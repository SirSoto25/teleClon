//! Gestión de chats (menú contextual) contra el servidor falso: peticiones TL exactas, repetición de
//! `deleteHistory` por tandas, eventos emitidos y errores de la red.
use std::sync::{Arc, Mutex};

use futures::StreamExt;
use grammers_tl_types as tl;
use tc_core::*;

use super::fixtures::*;
use super::flows::restored;
use super::*;
use crate::ids::{channel_group, chat_group};

fn affected_history(offset: i32) -> Reply {
    ok(tl::enums::messages::AffectedHistory::from(tl::types::messages::AffectedHistory { pts: 1, pts_count: 1, offset }))
}

fn affected_messages() -> Reply {
    ok(tl::enums::messages::AffectedMessages::from(tl::types::messages::AffectedMessages { pts: 1, pts_count: 1 }))
}

fn is_user(p: &tl::enums::InputPeer, id: i64, hash: i64) -> bool {
    matches!(p, tl::enums::InputPeer::User(u) if u.user_id == id && u.access_hash == hash)
}

/// Backend con Ana (usuario 10), un supergrupo (20), un canal (21) y un grupo básico (30) en caché,
/// todos con su fila en la lista. `log` recoge lo que el manejador quiera anotar.
async fn setup(handler: impl FnMut(u32, &[u8]) -> Reply + Send + 'static) -> (GrammersBackend, Arc<Fake>) {
    let mut h = handler;
    let (b, fake, _) = restored(move |_, id, body| h(id, body), None).await;
    b.inner.ingest(&[user_enum(10, "Ana", 100, false)], &[channel(20, "Charla", false, 200), channel(21, "Noticias", true, 210), chat(30, "Familia")]);
    {
        let mut c = b.inner.cache();
        for peer in [PeerRef::User(UserId(10)), PeerRef::Group(channel_group(20)), PeerRef::Group(channel_group(21)), PeerRef::Group(chat_group(30))] {
            let mut d = crate::convert::dialog_stub(peer, &c.ctx(0));
            d.unread_count = 3;
            c.dialogs.insert(peer, d);
            c.main.order.push(peer);
        }
    }
    (b, fake)
}

#[tokio::test]
async fn unread_mark_then_read_all_clears_it() {
    use tl::functions::messages;
    let marks = Arc::new(Mutex::new(Vec::new()));
    let m2 = marks.clone();
    let (b, fake) = setup(move |id, body| match id {
        x if x == cid::<messages::MarkDialogUnread>() => {
            let req: messages::MarkDialogUnread = decode(body);
            let tl::enums::InputDialogPeer::Peer(p) = &req.peer else { panic!("se esperaba un chat") };
            assert!(is_user(&p.peer, 10, 100) && req.parent_peer.is_none());
            m2.lock().unwrap().push(req.unread);
            ok(true)
        }
        x if x == cid::<messages::ReadHistory>() => affected_messages(),
        other => panic!("llamada inesperada {}", tl::name_for_id(other)),
    })
    .await;
    let peer = PeerRef::User(UserId(10));
    let mut ev = b.events();
    b.set_unread_mark(peer, true).await.unwrap();
    assert!(matches!(ev.next().await, Some(BackendEvent::DialogUpserted(d)) if d.unread_mark));
    b.mark_dialog_read(peer).await.unwrap();
    assert_eq!(*marks.lock().unwrap(), vec![true, false]);
    assert_eq!(fake.count("messages.readHistory"), 1);
    let d = b.inner.cache().dialogs[&peer].clone();
    assert!(!d.unread_mark && d.unread_count == 0);
    // Los chats secretos no tienen marca en el servidor.
    assert!(b.set_unread_mark(PeerRef::Secret(SecretChatId(1)), true).await.is_err());
}

#[tokio::test]
async fn pin_limit_and_mute_until() {
    use tl::functions::{account, messages};
    let (b, _) = setup(|id, body| match id {
        x if x == cid::<messages::ToggleDialogPin>() => rpc_err(400, "PINNED_DIALOGS_TOO_MUCH", None),
        x if x == cid::<account::UpdateNotifySettings>() => {
            let req: account::UpdateNotifySettings = decode(body);
            let tl::enums::InputPeerNotifySettings::Settings(s) = req.settings;
            assert_eq!(s.mute_until, Some(2_000_000_000));
            ok(true)
        }
        other => panic!("llamada inesperada {}", tl::name_for_id(other)),
    })
    .await;
    let peer = PeerRef::User(UserId(10));
    assert_eq!(b.set_pinned(peer, true).await.unwrap_err(), BackendError::Forbidden("PINNED_DIALOGS_TOO_MUCH".into()));
    b.mute_until(peer, Some(Timestamp(2_000_000_000))).await.unwrap();
    assert!(b.inner.cache().dialogs[&peer].muted);
    // Un plazo ya pasado no se manda.
    assert_eq!(b.mute_until(peer, Some(Timestamp(10))).await.unwrap_err(), BackendError::InvalidInput("mute_until"));
}

#[tokio::test]
async fn clear_and_delete_private_chat_repeat_until_done() {
    use tl::functions::messages;
    let seen = Arc::new(Mutex::new(Vec::new()));
    let s2 = seen.clone();
    let (b, fake) = setup(move |id, body| match id {
        x if x == cid::<messages::DeleteHistory>() => {
            let req: messages::DeleteHistory = decode(body);
            assert!(is_user(&req.peer, 10, 100) && req.max_id == 0);
            let mut s = s2.lock().unwrap();
            s.push((req.just_clear, req.revoke));
            // La primera llamada de cada operación deja una tanda pendiente.
            affected_history(if s.len() % 2 == 1 { 50 } else { 0 })
        }
        other => panic!("llamada inesperada {}", tl::name_for_id(other)),
    })
    .await;
    let peer = PeerRef::User(UserId(10));
    let mut ev = b.events();
    b.clear_history(peer).await.unwrap();
    assert!(matches!(ev.next().await, Some(BackendEvent::DialogUpserted(d)) if d.last_message.is_none() && d.unread_count == 0));
    assert_eq!(ev.next().await, Some(BackendEvent::HistoryCleared { peer }));
    b.delete_dialog(peer, true).await.unwrap();
    assert_eq!(ev.next().await, Some(BackendEvent::DialogRemoved { peer }));
    assert_eq!(*seen.lock().unwrap(), vec![(true, false), (true, false), (false, true), (false, true)]);
    assert_eq!(fake.count("messages.deleteHistory"), 4);
    assert!(!b.inner.cache().dialogs.contains_key(&peer) && !b.inner.cache().main.order.contains(&peer));
}

#[tokio::test]
async fn leave_and_delete_groups_and_channels() {
    use tl::functions::{channels, messages};
    let (b, fake) = setup(|id, body| match id {
        x if x == cid::<channels::LeaveChannel>() => {
            let req: channels::LeaveChannel = decode(body);
            assert!(matches!(req.channel, tl::enums::InputChannel::Channel(ref c) if (c.channel_id, c.access_hash) == (21, 210) || (c.channel_id, c.access_hash) == (20, 200)));
            ok(updates(vec![], vec![], vec![]))
        }
        x if x == cid::<channels::DeleteHistory>() => {
            let req: channels::DeleteHistory = decode(body);
            assert!(!req.for_everyone && req.max_id == i32::MAX);
            assert!(matches!(req.channel, tl::enums::InputChannel::Channel(ref c) if c.channel_id == 20));
            ok(updates(vec![], vec![], vec![]))
        }
        x if x == cid::<messages::DeleteChatUser>() => {
            let req: messages::DeleteChatUser = decode(body);
            assert_eq!(req.chat_id, 30);
            assert!(matches!(req.user_id, tl::enums::InputUser::UserSelf));
            assert!(req.revoke_history);
            // Ya no era miembro: el borrado sigue igualmente.
            rpc_err(400, "USER_NOT_PARTICIPANT", None)
        }
        x if x == cid::<messages::DeleteHistory>() => {
            let req: messages::DeleteHistory = decode(body);
            assert!(matches!(req.peer, tl::enums::InputPeer::Chat(ref c) if c.chat_id == 30) && !req.just_clear);
            affected_history(0)
        }
        other => panic!("llamada inesperada {}", tl::name_for_id(other)),
    })
    .await;
    let (sup, chan, basic) = (PeerRef::Group(channel_group(20)), PeerRef::Group(channel_group(21)), PeerRef::Group(chat_group(30)));
    // Vaciar un canal de difusión no está permitido; en un supergrupo, solo para mí.
    assert_eq!(b.clear_history(chan).await.unwrap_err(), BackendError::Forbidden("CHAT_ADMIN_REQUIRED".into()));
    b.clear_history(sup).await.unwrap();
    b.leave_chat(chan).await.unwrap();
    b.delete_dialog(sup, false).await.unwrap();
    b.delete_dialog(basic, false).await.unwrap();
    assert_eq!(fake.count("channels.leaveChannel"), 2);
    assert_eq!(fake.count("messages.deleteChatUser"), 1);
    assert_eq!(fake.count("messages.deleteHistory"), 1);
    assert!([sup, chan, basic].iter().all(|p| !b.inner.cache().dialogs.contains_key(p)));
    // Salir de un chat privado no tiene sentido.
    assert_eq!(b.leave_chat(PeerRef::User(UserId(10))).await.unwrap_err(), BackendError::InvalidInput("peer"));
}

#[tokio::test]
async fn block_pin_message_and_info_errors() {
    use tl::functions::{contacts, messages, users};
    let (b, fake) = setup(|id, body| match id {
        x if x == cid::<contacts::Block>() => {
            let req: contacts::Block = decode(body);
            assert!(is_user(&req.id, 10, 100));
            ok(true)
        }
        x if x == cid::<contacts::Unblock>() => ok(true),
        x if x == cid::<messages::UpdatePinnedMessage>() => {
            let req: messages::UpdatePinnedMessage = decode(body);
            assert!(is_user(&req.peer, 10, 100) && req.id == 7 && req.silent);
            ok(updates(vec![], vec![], vec![]))
        }
        x if x == cid::<users::GetFullUser>() => {
            let req: users::GetFullUser = decode(body);
            assert!(matches!(req.id, tl::enums::InputUser::User(ref u) if u.user_id == 10 && u.access_hash == 100));
            rpc_err(400, "USER_ID_INVALID", None)
        }
        x if x == cid::<messages::GetFullChat>() => rpc_err(403, "CHANNEL_PRIVATE", None),
        other => panic!("llamada inesperada {}", tl::name_for_id(other)),
    })
    .await;
    let ana = PeerRef::User(UserId(10));
    b.set_blocked(UserId(10), true).await.unwrap();
    b.set_blocked(UserId(10), false).await.unwrap();
    assert_eq!(b.set_blocked(UserId(ME_ID), true).await.unwrap_err(), BackendError::InvalidInput("user"));
    b.pin_message(ana, MsgId(7), true).await.unwrap();
    b.pin_message(ana, MsgId(7), false).await.unwrap();
    assert_eq!(fake.count("messages.updatePinnedMessage"), 2);
    assert_eq!(b.chat_info(ana).await.unwrap_err(), BackendError::NotFound);
    assert_eq!(b.chat_info(PeerRef::Group(chat_group(30))).await.unwrap_err(), BackendError::Forbidden("CHANNEL_PRIVATE".into()));
    assert_eq!((fake.count("contacts.block"), fake.count("contacts.unblock")), (1, 1));
}
