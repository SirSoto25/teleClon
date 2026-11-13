//! `CachedBackend` refleja en la caché las acciones de gestión de chats (menú contextual).
use crate::{CachedBackend, Store};
use futures::StreamExt;
use std::time::Duration;
use tc_core::*;
use tc_mock::{MockBackend, MockConfig};

fn cached() -> CachedBackend<MockBackend> {
    CachedBackend::new(MockBackend::new(MockConfig { now: Some(1_800_000_000), ..Default::default() }), Store::open_in_memory().unwrap())
}

fn all() -> DialogsPage {
    DialogsPage { offset: 0, limit: 1000, archived: false }
}

/// Lo que queda en la caché (sin red).
async fn offline_list(c: &CachedBackend<MockBackend>) -> Vec<Dialog> {
    c.inner().set_offline(true);
    let v = c.dialogs(all()).await.unwrap();
    c.inner().set_offline(false);
    v
}

fn a_user_chat(list: &[Dialog]) -> PeerRef {
    list.iter().find(|d| matches!(d.peer, PeerRef::User(_)) && d.last_message.is_some() && d.peer != tc_mock::BIG_CHAT).unwrap().peer
}

#[tokio::test]
async fn unread_mark_read_mute_and_pin_persist() {
    let c = cached();
    let list = c.dialogs(all()).await.unwrap();
    let peer = a_user_chat(&list);
    c.set_unread_mark(peer, true).await.unwrap();
    assert!(offline_list(&c).await.iter().any(|d| d.peer == peer && d.unread_mark));
    c.mark_dialog_read(peer).await.unwrap();
    assert!(offline_list(&c).await.iter().any(|d| d.peer == peer && !d.unread_mark && d.unread_count == 0));
    c.mute_until(peer, Some(Timestamp(i64::from(i32::MAX) - 1))).await.unwrap();
    assert!(offline_list(&c).await.iter().any(|d| d.peer == peer && d.muted));
    c.set_pinned(peer, true).await.unwrap();
    assert!(offline_list(&c).await.iter().any(|d| d.peer == peer && d.pinned_order.is_some()));
}

#[tokio::test]
async fn clear_and_delete_reach_the_cache() {
    let c = cached();
    let list = c.dialogs(all()).await.unwrap();
    let peer = a_user_chat(&list);
    assert!(!c.history(peer, Anchor::Latest, 50).await.unwrap().is_empty());
    c.clear_history(peer).await.unwrap();
    c.inner().set_offline(true);
    assert!(c.history(peer, Anchor::Latest, 50).await.unwrap_or_default().is_empty());
    c.inner().set_offline(false);
    assert!(offline_list(&c).await.iter().any(|d| d.peer == peer && d.last_message.is_none()));
    c.delete_dialog(peer, false).await.unwrap();
    assert!(offline_list(&c).await.iter().all(|d| d.peer != peer));
}

#[tokio::test]
async fn removal_events_are_persisted() {
    let c = cached();
    let list = c.dialogs(all()).await.unwrap();
    let group = list.iter().find(|d| matches!(d.peer, PeerRef::Group(_))).unwrap().peer;
    let mut ev = c.events();
    // Llamada directa al backend interno: la caché se entera solo por el evento.
    c.inner().leave_chat(group).await.unwrap();
    tokio::time::timeout(Duration::from_secs(5), async {
        while let Some(e) = ev.next().await {
            if e == (BackendEvent::DialogRemoved { peer: group }) {
                break;
            }
        }
    })
    .await
    .unwrap();
    assert!(offline_list(&c).await.iter().all(|d| d.peer != group));
}
