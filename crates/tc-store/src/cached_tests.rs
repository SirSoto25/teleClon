//! Pruebas de `CachedBackend` con `MockBackend` como backend interno.
use crate::{CachedBackend, Store};
use futures::StreamExt;
use std::time::Duration;
use tc_core::*;
use tc_mock::{BIG_CHAT, MockBackend, MockConfig, SECRET_ID};

const NOW: i64 = 1_800_000_000;

fn mock() -> MockBackend {
    MockBackend::new(MockConfig { now: Some(NOW), ..Default::default() })
}

fn cached() -> CachedBackend<MockBackend> {
    CachedBackend::new(mock(), Store::open_in_memory().unwrap())
}

fn all(archived: bool) -> DialogsPage {
    DialogsPage { offset: 0, limit: 1000, archived }
}

fn text(t: &str, rid: i64) -> OutgoingText {
    OutgoingText { text: t.into(), entities: vec![], reply_to: None, random_id: RandomId(rid) }
}

async fn next_event<S: futures::Stream<Item = BackendEvent> + Unpin>(s: &mut S, mut pred: impl FnMut(&BackendEvent) -> bool) -> BackendEvent {
    tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            let e = s.next().await.expect("stream terminado");
            if pred(&e) {
                return e;
            }
        }
    })
    .await
    .expect("evento esperado no llegó")
}

fn sorted(mut v: Vec<Dialog>) -> Vec<Dialog> {
    v.sort_by_key(|d| format!("{:?}", d.peer));
    v
}

#[tokio::test]
async fn dialogs_roundtrip_exactly_and_survive_offline() {
    let c = cached();
    let fresh = c.dialogs(all(false)).await.unwrap(); // frío: de la red
    assert!(fresh.len() > 20);
    c.inner().set_offline(true);
    let cache = c.dialogs(all(false)).await.unwrap(); // sin red: de la caché
    // Los secretos se guardan sin último mensaje, borrador ni foto (privacidad); el resto, idéntico.
    let strip = |d: &Dialog| Dialog { last_message: None, draft: None, photo: None, ..d.clone() };
    let norm = |v: Vec<Dialog>| -> Vec<Dialog> { v.into_iter().map(|d| if matches!(d.peer, PeerRef::Secret(_)) { strip(&d) } else { d }).collect() };
    assert_eq!(sorted(norm(fresh.clone())), sorted(cache.clone()));
    // Mismo orden que el backend (fijados primero, luego actividad).
    let pins = |v: &[Dialog]| v.iter().take_while(|d| d.pinned_order.is_some()).count();
    assert_eq!(pins(&fresh), pins(&cache));
    assert!(cache[pins(&cache)..].windows(2).all(|w| w[0].activity >= w[1].activity));
    // Archivados: no se mezclan.
    c.inner().set_offline(false);
    let arch = c.dialogs(all(true)).await.unwrap();
    assert!(arch.iter().all(|d| d.archived) && !arch.is_empty());
    assert!(c.dialogs(all(false)).await.unwrap().iter().all(|d| !d.archived));
}

#[tokio::test]
async fn cold_cache_offline_is_an_error_and_pagination_works() {
    let c = cached();
    c.inner().set_offline(true);
    assert_eq!(c.dialogs(all(false)).await.unwrap_err(), BackendError::Network);
    c.inner().set_offline(false);
    let p1 = c.dialogs(DialogsPage { offset: 0, limit: 5, archived: false }).await.unwrap();
    let p2 = c.dialogs(DialogsPage { offset: 5, limit: 5, archived: false }).await.unwrap();
    assert_eq!((p1.len(), p2.len()), (5, 5));
    c.inner().set_offline(true);
    // Páginas ya vistas se sirven sin red, y sin solaparse.
    let q1 = c.dialogs(DialogsPage { offset: 0, limit: 5, archived: false }).await.unwrap();
    let q2 = c.dialogs(DialogsPage { offset: 5, limit: 5, archived: false }).await.unwrap();
    assert_eq!((q1.len(), q2.len()), (5, 5));
    assert!(q1.iter().all(|a| q2.iter().all(|b| a.peer != b.peer)));
}

#[tokio::test]
async fn warm_dialogs_are_served_from_cache_then_refreshed_via_events() {
    let c = cached();
    let before = c.dialogs(all(false)).await.unwrap();
    let target = before.iter().find(|d| !d.muted && !matches!(d.peer, PeerRef::Secret(_))).unwrap().peer;
    c.inner().set_muted(target, true).await.unwrap(); // cambio remoto que la caché aún no conoce
    let mut ev = c.events();
    let stale = c.dialogs(all(false)).await.unwrap();
    assert!(!stale.iter().find(|d| d.peer == target).unwrap().muted, "se sirve lo cacheado al instante");
    let e = next_event(&mut ev, |e| matches!(e, BackendEvent::DialogUpserted(d) if d.peer == target && d.muted)).await;
    assert!(matches!(e, BackendEvent::DialogUpserted(_)));
    c.inner().set_offline(true);
    assert!(c.dialogs(all(false)).await.unwrap().iter().find(|d| d.peer == target).unwrap().muted, "el refresco quedó en la base");
}

#[tokio::test]
async fn history_is_lossless_paginates_and_serves_offline() {
    let c = cached();
    let latest = c.history(BIG_CHAT, Anchor::Latest, 50).await.unwrap();
    assert_eq!(latest.len(), 50);
    let direct = c.inner().history(BIG_CHAT, Anchor::Latest, 50).await.unwrap();
    assert_eq!(latest, direct);
    c.inner().set_offline(true);
    // Sin red: la caché devuelve exactamente lo mismo, sin pérdida de campos.
    assert_eq!(c.history(BIG_CHAT, Anchor::Latest, 50).await.unwrap(), direct);
    let older = c.history(BIG_CHAT, Anchor::Before(latest[0].id), 50).await;
    assert_eq!(older.unwrap_err(), BackendError::Network, "lo no cacheado necesita red");
    c.inner().set_offline(false);
    let older = c.history(BIG_CHAT, Anchor::Before(latest[0].id), 50).await.unwrap();
    assert_eq!(older, c.inner().history(BIG_CHAT, Anchor::Before(latest[0].id), 50).await.unwrap());
    c.inner().set_offline(true);
    assert_eq!(c.history(BIG_CHAT, Anchor::Before(latest[0].id), 50).await.unwrap(), older);
    // Las dos páginas son contiguas.
    assert!(older.last().unwrap().id < latest[0].id);
}

#[tokio::test]
async fn history_roundtrips_media_stickers_reactions_and_entities() {
    let c = cached();
    let peers: Vec<PeerRef> = c.dialogs(all(false)).await.unwrap().into_iter().map(|d| d.peer).filter(|p| !matches!(p, PeerRef::Secret(_))).collect();
    let mut fetched = Vec::new();
    for &p in &peers {
        fetched.push((p, c.history(p, Anchor::Latest, 200).await.unwrap()));
    }
    // El mock reutiliza ids de medio con metadatos distintos entre chats; la tabla `media` va por id de
    // red (único en Telegram), así que en esos casos solo se exige conservar el id.
    let mut by_id: std::collections::HashMap<i64, Media> = std::collections::HashMap::new();
    let mut clash = std::collections::HashSet::new();
    for m in fetched.iter().flat_map(|(_, h)| h).filter_map(|m| m.media.as_ref()) {
        if by_id.insert(m.id, m.clone()).is_some_and(|prev| prev != *m) {
            clash.insert(m.id);
        }
    }
    let norm = |v: Vec<Message>| -> Vec<Message> {
        v.into_iter()
            .map(|mut m| {
                if let Some(md) = m.media.as_mut().filter(|md| clash.contains(&md.id)) {
                    *md = Media { id: md.id, kind: md.kind, file_name: None, size_bytes: 0, width: 0, height: 0, duration_secs: None, local_path: None, file: None, audio: None };
                }
                m
            })
            .collect()
    };
    c.inner().set_offline(true);
    let mut seen = (false, false, false, false);
    for (p, h) in fetched {
        let cache = c.history(p, Anchor::Latest, 200).await.unwrap();
        assert_eq!(norm(cache), norm(h.clone()), "{p:?}");
        for m in &h {
            seen = (seen.0 || m.media.is_some(), seen.1 || m.sticker.is_some(), seen.2 || !m.reactions.is_empty(), seen.3 || !m.entities.is_empty());
        }
    }
    assert!(seen.0 || seen.1 || seen.2 || seen.3, "los datos de muestra deberían ejercitar algún campo rico: {seen:?}");
}

#[tokio::test]
async fn send_text_persists_is_idempotent_and_failures_stay_visible() {
    let c = cached();
    let peer = PeerRef::User(UserId(1001));
    c.dialogs(all(false)).await.unwrap();
    let m1 = c.send_text(peer, text("hola mundo cálido", 7001)).await.unwrap();
    let m2 = c.send_text(peer, text("hola mundo cálido", 7001)).await.unwrap();
    assert_eq!(m1.id, m2.id, "idempotente por random_id");
    c.inner().set_offline(true);
    let cached_h = c.history(peer, Anchor::Latest, 5).await.unwrap();
    assert_eq!(cached_h.iter().filter(|m| m.text == "hola mundo cálido").count(), 1);
    assert!(c.unsent_messages(peer).await.unwrap().is_empty());

    // Sin red: el envío falla y queda un pendiente marcado como fallido.
    assert_eq!(c.send_text(peer, text("no llegará aún", 7002)).await.unwrap_err(), BackendError::Network);
    let unsent = c.unsent_messages(peer).await.unwrap();
    assert_eq!((unsent.len(), unsent[0].state, unsent[0].random_id), (1, SendState::Failed, Some(RandomId(7002))));
    assert!(unsent[0].id.0 < 0 && unsent[0].outgoing);
    // El pendiente no contamina el historial.
    assert!(c.history(peer, Anchor::Latest, 50).await.unwrap().iter().all(|m| m.text != "no llegará aún"));

    // Reintento con el mismo random_id: el pendiente pasa a confirmado, sin duplicar.
    c.inner().set_offline(false);
    let ok = c.send_text(peer, text("no llegará aún", 7002)).await.unwrap();
    assert!(ok.id.0 > 0);
    assert!(c.unsent_messages(peer).await.unwrap().is_empty());
    c.inner().set_offline(true);
    let h = c.history(peer, Anchor::Latest, 50).await.unwrap();
    assert_eq!(h.iter().filter(|m| m.text == "no llegará aún").count(), 1);
}

#[tokio::test]
async fn events_are_persisted_and_searchable_locally() {
    let c = cached();
    c.dialogs(all(false)).await.unwrap();
    let peer = PeerRef::User(UserId(1002));
    let mut ev = c.events();
    let m = c.inner().inject_incoming(peer, "Reunión mañana en la CAFETERÍA").unwrap();
    next_event(&mut ev, |e| matches!(e, BackendEvent::MessageNew(x) if x.id == m.id)).await;
    c.inner().set_offline(true);
    let hits = c.search_local("cafeteria", None, 10).await.unwrap();
    assert_eq!(hits.len(), 1, "sin acentos ni mayúsculas");
    assert_eq!((hits[0].peer, hits[0].msg_id), (peer, m.id));
    assert!(hits[0].snippet.contains('«'));
    assert_eq!(c.search_local("reun", Some(peer), 10).await.unwrap().len(), 1, "prefijo");
    assert!(c.search_local("cafeteria", Some(PeerRef::User(UserId(1003))), 10).await.unwrap().is_empty());
    assert!(c.search_local("cafeteria", Some(PeerRef::User(UserId(424242))), 10).await.unwrap().is_empty());
    // También lo sirve el historial sin red.
    assert!(c.history(peer, Anchor::Latest, 3).await.is_ok());
}

#[tokio::test]
async fn edits_deletes_reactions_read_and_failures_write_through() {
    let c = cached();
    c.dialogs(all(false)).await.unwrap();
    let peer = PeerRef::User(UserId(1001));
    let m = c.send_text(peer, text("original", 8001)).await.unwrap();
    let e = c.edit_message(peer, m.id, "editado ñandú".into(), vec![]).await.unwrap();
    assert!(e.edit_date.is_some());
    assert_eq!(c.search_local("nandu", None, 5).await.unwrap().len(), 1);
    assert!(c.search_local("original", None, 5).await.unwrap().is_empty(), "el FTS sigue a la edición");

    c.send_reaction(peer, m.id, Some("👍".into())).await.unwrap();
    c.inner().set_offline(true);
    let h = c.history(peer, Anchor::Latest, 50).await.unwrap();
    let got = h.iter().find(|x| x.id == m.id).unwrap();
    assert_eq!(got.reactions, [ReactionCount { emoji: "👍".into(), count: 1, mine: true }]);
    c.inner().set_offline(false);

    c.send_reaction(peer, m.id, None).await.unwrap();
    c.mark_read(peer, m.id).await.unwrap();
    c.delete_messages(peer, vec![m.id], true).await.unwrap();
    assert!(c.search_local("nandu", None, 5).await.unwrap().is_empty());
    c.inner().set_offline(true);
    assert!(c.history(peer, Anchor::Latest, 50).await.unwrap().iter().all(|x| x.id != m.id));
    // Una escritura rechazada por el backend no toca la caché.
    assert!(c.edit_message(peer, m.id, "x".into(), vec![]).await.is_err());
}

#[tokio::test]
async fn pin_archive_mute_and_draft_are_mirrored() {
    let c = cached();
    let peer = PeerRef::User(UserId(1005));
    let before = c.dialogs(all(false)).await.unwrap();
    assert!(before.iter().find(|d| d.peer == peer).is_some_and(|d| d.pinned_order.is_none() && !d.muted));
    c.set_pinned(peer, true).await.unwrap();
    c.set_muted(peer, true).await.unwrap();
    c.save_draft(peer, Some(Draft { text: "borrador".into(), reply_to: None })).await.unwrap();
    c.inner().set_offline(true);
    let d = c.dialogs(all(false)).await.unwrap().into_iter().find(|d| d.peer == peer).unwrap();
    assert!(d.pinned_order.is_some() && d.muted);
    assert_eq!(d.draft.as_ref().map(|x| x.text.as_str()), Some("borrador"));
    // Sin red el borrador igualmente queda guardado en local (y el error sube).
    assert!(c.save_draft(peer, Some(Draft { text: "offline".into(), reply_to: None })).await.is_err());
    let d = c.dialogs(all(false)).await.unwrap().into_iter().find(|d| d.peer == peer).unwrap();
    assert_eq!(d.draft.unwrap().text, "offline");
    c.inner().set_offline(false);
    c.set_archived(peer, true).await.unwrap();
    c.inner().set_offline(true);
    assert!(c.dialogs(all(true)).await.unwrap().iter().any(|d| d.peer == peer && d.pinned_order.is_none()));
    assert!(c.dialogs(all(false)).await.unwrap().iter().all(|d| d.peer != peer));
}

#[tokio::test]
async fn secret_chats_never_reach_plaintext_tables() {
    let c = cached();
    c.dialogs(all(false)).await.unwrap();
    let peer = PeerRef::Secret(SECRET_ID);
    let sent = c.send_text(peer, text("clave: tomate-secreto", 9001)).await.unwrap();
    assert_eq!(sent.peer, peer);
    let h = c.history(peer, Anchor::Latest, 50).await.unwrap();
    assert!(h.iter().any(|m| m.text.contains("tomate")), "el backend sí lo sirve");
    c.save_draft(peer, Some(Draft { text: "borrador-secreto".into(), reply_to: None })).await.unwrap();
    c.send_reaction(peer, sent.id, Some("👍".into())).await.unwrap();
    let mut ev = c.events();
    c.inner().inject_incoming(peer, "respuesta-secreta").unwrap();
    next_event(&mut ev, |e| matches!(e, BackendEvent::MessageNew(m) if m.peer == peer)).await;
    // Ni mensajes, ni FTS, ni borradores, ni último mensaje de ese diálogo en la base.
    let counts = c
        .with_store(|s| {
            let n_msgs: i64 = s.conn.query_row("SELECT COUNT(*) FROM messages m JOIN dialogs d USING(chat_key) WHERE d.peer_kind='secret'", [], |r| r.get(0))?;
            let n_fts: i64 = s.conn.query_row("SELECT COUNT(*) FROM messages_fts WHERE messages_fts MATCH 'tomate OR secreta OR secreto'", [], |r| r.get(0))?;
            let n_drafts: i64 = s.conn.query_row("SELECT COUNT(*) FROM dialogs WHERE peer_kind='secret' AND draft_text IS NOT NULL", [], |r| r.get(0))?;
            Ok((n_msgs, n_fts, n_drafts))
        })
        .await
        .unwrap();
    assert_eq!(counts, (0, 0, 0));
    assert!(c.search_local("tomate", None, 5).await.unwrap().is_empty());
    assert!(c.search_local("tomate", Some(peer), 5).await.unwrap().is_empty());
}

#[tokio::test]
async fn profile_reads_write_through_and_fall_back_offline() {
    let c = cached();
    let me = c.me().await.unwrap();
    let contacts = c.contacts().await.unwrap();
    let u = c.user(UserId(1003)).await.unwrap();
    let dialogs = c.dialogs(all(false)).await.unwrap();
    let g = dialogs.iter().find_map(|d| if let PeerRef::Group(g) = d.peer { Some(g) } else { None }).unwrap();
    let group = c.group(g).await.unwrap();
    c.inner().set_offline(true);
    assert_eq!(c.me().await.unwrap(), me);
    assert_eq!(c.user(UserId(1003)).await.unwrap(), u);
    assert_eq!(c.group(g).await.unwrap(), group);
    let mut a = c.contacts().await.unwrap();
    let mut b = contacts;
    a.sort_by_key(|u| u.id);
    b.sort_by_key(|u| u.id);
    assert_eq!(a, b);
    assert_eq!(c.user(UserId(99_999)).await.unwrap_err(), BackendError::Network, "lo desconocido sigue fallando");
}

#[tokio::test]
async fn cache_persists_across_restarts_on_disk() {
    let dir = std::env::temp_dir().join(format!("tc-store-cached-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let first = {
        let c = CachedBackend::new(mock(), Store::open_account(&dir, "u1").unwrap());
        c.dialogs(all(false)).await.unwrap();
        c.history(BIG_CHAT, Anchor::Latest, 30).await.unwrap()
    };
    let offline = mock();
    offline.set_offline(true);
    let c = CachedBackend::new(offline, Store::open_account(&dir, "u1").unwrap());
    assert!(!c.dialogs(all(false)).await.unwrap().is_empty());
    assert_eq!(c.history(BIG_CHAT, Anchor::Latest, 30).await.unwrap(), first);
    let _ = std::fs::remove_dir_all(&dir);
}

#[tokio::test]
async fn remote_deletions_in_refreshed_range_are_removed_and_announced() {
    let c = cached();
    let peer = PeerRef::User(UserId(1001));
    let m = c.send_text(peer, text("se borrará", 5501)).await.unwrap();
    let last = c.send_text(peer, text("se queda", 5502)).await.unwrap();
    let h = c.history(peer, Anchor::Latest, 20).await.unwrap();
    assert!(h.iter().any(|x| x.id == m.id));
    c.inner().delete_messages(peer, vec![m.id], true).await.unwrap(); // borrado remoto
    let mut ev = c.events();
    let stale = c.history(peer, Anchor::Latest, 20).await.unwrap();
    assert!(stale.iter().any(|x| x.id == m.id), "se sirve primero lo cacheado");
    next_event(&mut ev, |e| matches!(e, BackendEvent::MessagesDeleted { ids, .. } if ids == &[m.id])).await;
    c.inner().set_offline(true);
    let h = c.history(peer, Anchor::Latest, 20).await.unwrap();
    assert!(h.iter().all(|x| x.id != m.id) && h.iter().any(|x| x.id == last.id));
}

#[tokio::test]
async fn events_stream_ends_when_the_decorator_and_backend_are_gone() {
    let c = cached();
    let mut ev = c.events();
    drop(c);
    let end = tokio::time::timeout(Duration::from_secs(5), async { while ev.next().await.is_some() {} }).await;
    assert!(end.is_ok(), "el stream debe terminar al soltar el decorador");
}

fn tmp_dir(tag: &str) -> std::path::PathBuf {
    let p = std::env::temp_dir().join(format!("tc-cached-{tag}-{}-{:?}", std::process::id(), std::thread::current().id()));
    let _ = std::fs::remove_dir_all(&p);
    p
}

#[tokio::test]
async fn downloads_are_cached_on_disk_but_never_for_secret_chats() {
    let dir = tmp_dir("media");
    let media = std::sync::Arc::new(crate::MediaCache::open(dir.join("media"), Store::open_in_memory().unwrap(), 64 << 20).unwrap());
    let c = CachedBackend::with_media_cache(mock(), Store::open_in_memory().unwrap(), media.clone());
    let dialogs = c.dialogs(all(false)).await.unwrap();
    // Las fotos de perfil sobreviven a la caché SQLite (y la del chat secreto, solo en memoria).
    let user = dialogs.iter().find(|d| matches!(d.peer, PeerRef::User(_)) && d.photo.is_some()).expect("algún usuario con foto");
    let photo = user.photo.unwrap();
    c.inner().set_offline(true);
    let offline = c.dialogs(all(false)).await.unwrap();
    assert_eq!(offline.iter().find(|d| d.peer == user.peer).unwrap().photo, Some(photo));
    assert!(offline.iter().filter(|d| matches!(d.peer, PeerRef::Secret(_))).all(|d| d.photo.is_none()));
    // Sin red y sin caché: error; con red, una sola descarga y luego desde el disco.
    assert_eq!(c.download_profile_photo(user.peer, photo, PhotoSize::Small).await, Err(BackendError::Network));
    c.inner().set_offline(false);
    let png = c.download_profile_photo(user.peer, photo, PhotoSize::Small).await.unwrap();
    assert_eq!(&png[1..4], b"PNG");
    c.inner().set_offline(true);
    assert_eq!(c.download_profile_photo(user.peer, photo, PhotoSize::Small).await.unwrap(), png);
    assert!(c.download_profile_photo(user.peer, photo, PhotoSize::Big).await.is_err(), "otro tamaño, otra clave");
    c.inner().set_offline(false);

    // Medios: la referencia de descarga se guarda con el mensaje y la miniatura queda en disco.
    let mut found = None;
    for d in &dialogs {
        if matches!(d.peer, PeerRef::Secret(_)) {
            continue;
        }
        let h = c.history(d.peer, Anchor::Latest, 200).await.unwrap();
        if let Some(m) = h.iter().find(|m| m.media.as_ref().is_some_and(|md| md.kind == MediaKind::Photo)) {
            found = Some((d.peer, m.clone()));
            break;
        }
    }
    let (peer, m) = found.expect("alguna foto en el historial del mock");
    let file = m.media.as_ref().unwrap().file.clone().expect("referencia de descarga");
    c.inner().set_offline(true);
    let again = c.history(peer, Anchor::Latest, 200).await.unwrap();
    assert_eq!(again.iter().find(|x| x.id == m.id).and_then(|x| x.media.as_ref()).and_then(|md| md.file.clone()), Some(file.clone()));
    c.inner().set_offline(false);
    let used = media.usage().unwrap();
    let thumb = c.download_media(peer, m.id, file.clone(), MediaVariant::Thumb).await.unwrap();
    assert!(media.usage().unwrap() > used);
    c.inner().set_offline(true);
    assert_eq!(c.download_media(peer, m.id, file.clone(), MediaVariant::Thumb).await.unwrap(), thumb);
    c.inner().set_offline(false);

    // Chat secreto: la foto del otro se descarga pero no se escribe nada en disco.
    let secret = dialogs.iter().find(|d| d.peer == PeerRef::Secret(SECRET_ID)).unwrap();
    if let Some(p) = secret.photo {
        let before = media.usage().unwrap();
        c.download_profile_photo(secret.peer, p, PhotoSize::Big).await.unwrap();
        assert_eq!(media.usage().unwrap(), before);
    }
    let _ = std::fs::remove_dir_all(&dir);
}

#[tokio::test]
async fn ventana_del_primer_no_leido_va_a_la_red_y_sin_red_a_la_cache() {
    let c = cached();
    let d = c.dialogs(all(false)).await.unwrap().into_iter().find(|d| d.peer == tc_mock::UNREAD_CHAT).expect("grupo con no leídos");
    assert!(d.read_inbox_max_id.0 > 0, "el último leído llega de la red");
    let anchor = Anchor::Around(d.read_inbox_max_id);
    let w = c.history(tc_mock::UNREAD_CHAT, anchor, 60).await.unwrap();
    assert_eq!(w, c.inner().history(tc_mock::UNREAD_CHAT, anchor, 60).await.unwrap());
    // Hacia lo nuevo también desde la red (la caché puede tener huecos entre páginas sueltas).
    let newer = c.history(tc_mock::UNREAD_CHAT, Anchor::After(w.last().unwrap().id), 60).await.unwrap();
    assert!(newer.first().is_none_or(|m| m.id > w.last().unwrap().id));
    c.inner().set_offline(true);
    // Sin red se sirve lo guardado alrededor del mismo id (el reparto antes/después puede variar).
    let off = c.history(tc_mock::UNREAD_CHAT, anchor, 60).await.unwrap();
    assert!(off.iter().any(|m| m.id == d.read_inbox_max_id));
    assert!(off.iter().any(|m| m.id > d.read_inbox_max_id));
    assert!(off.iter().all(|m| w.contains(m) || newer.contains(m)), "sin pérdida de campos");
    // El último leído sobrevive en la base.
    let d2 = c.dialogs(all(false)).await.unwrap().into_iter().find(|x| x.peer == tc_mock::UNREAD_CHAT).unwrap();
    assert_eq!(d2.read_inbox_max_id, d.read_inbox_max_id);
}

/// ADR-0011: `CachedBackend` deja pasar las transferencias (descarga a disco, cancelación y el
/// progreso de `send_media`) y sus eventos llegan por su propio flujo.
#[tokio::test]
async fn transfers_pass_through_with_their_events() {
    let m = mock();
    m.set_transfer_sim(tc_mock::TransferSim { chunk: 100, ..Default::default() });
    let c = CachedBackend::new(m, Store::open_in_memory().unwrap());
    let peer = PeerRef::User(UserId(1001));
    let mut ev = c.events();
    let item = OutgoingMediaItem { kind: MediaKind::File, path: "/tmp/a.pdf".into(), file_name: "a.pdf".into(), size_bytes: 500, width: 0, height: 0 };
    let media = OutgoingMedia { items: vec![item], caption: String::new(), reply_to: None, random_ids: vec![RandomId(4040)] };
    let sent = c.send_media(peer, media).await.unwrap().remove(0);
    let up = TransferId::Upload(RandomId(4040));
    let done = next_event(&mut ev, |e| matches!(e, BackendEvent::TransferProgress(t) if t.id == up && t.state.is_finished())).await;
    assert!(matches!(done, BackendEvent::TransferProgress(t) if t.state == TransferState::Done && t.done == 500));

    let dir = std::env::temp_dir().join(format!("tc-store-transfer-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let dest = dir.join("a.pdf");
    let file = sent.media.unwrap().file.unwrap();
    let id = c.download_media_to(peer, sent.id, file, MediaVariant::Full, dest.clone()).await.unwrap();
    let end = next_event(&mut ev, |e| matches!(e, BackendEvent::TransferProgress(t) if t.id == id && t.state.is_finished())).await;
    assert!(matches!(end, BackendEvent::TransferProgress(t) if t.state == TransferState::Done));
    assert_eq!(std::fs::read(&dest).unwrap().len(), 500);
    c.cancel_transfer(id).await.unwrap();
    std::fs::remove_dir_all(&dir).unwrap();
}

// ---- búsqueda en el chat (ADR-0012) ---------------------------------------------------------

fn q(t: &str) -> SearchQuery {
    SearchQuery::new(t, 50)
}

fn ids(p: &SearchPage) -> Vec<MsgId> {
    p.hits.iter().map(|m| m.id).collect()
}

#[tokio::test]
async fn la_busqueda_fusiona_servidor_y_local_sin_duplicados_y_sin_red_usa_lo_local() {
    let c = cached();
    c.dialogs(all(false)).await.unwrap();
    let peer = PeerRef::User(UserId(1012));
    c.history(peer, Anchor::Latest, 20).await.unwrap();
    // Dos solo en el servidor y uno enviado por la caché (está en las dos partes).
    let s1 = c.inner().send_text(peer, text("zanahoria del servidor", 1)).await.unwrap();
    let mine = c.send_text(peer, text("Zanahoria enviada aquí", 2)).await.unwrap();
    let s2 = c.inner().send_text(peer, text("otra ZANAHORIA", 3)).await.unwrap();
    let page = c.search_in_chat(peer, q("zanahoria")).await.unwrap();
    assert_eq!(ids(&page), [s2.id, mine.id, s1.id], "más reciente primero y sin repetir el que está en las dos partes");
    assert_eq!(page.total, Some(3));
    assert_eq!(page.next, None);
    // Paginación: el servidor manda y lo local se funde sin repetir.
    let mut seen = Vec::new();
    let mut query = SearchQuery::new("zanahoria", 2);
    loop {
        let p = c.search_in_chat(peer, query.clone()).await.unwrap();
        seen.extend(ids(&p));
        match p.next {
            Some(n) => query = query.after(n),
            None => break,
        }
    }
    assert_eq!(seen, [s2.id, mine.id, s1.id]);
    // Sin red: solo lo que hay guardado.
    c.inner().set_offline(true);
    let off = c.search_in_chat(peer, q("zanahoria")).await.unwrap();
    assert_eq!(ids(&off), [mine.id]);
    assert_eq!(off.total, Some(1));
    let acc = c.search_in_chat(peer, q("ZANAHORIA ENVIADA aqui")).await.unwrap();
    assert_eq!(ids(&acc), [mine.id], "acentos y mayúsculas locales");
}

#[tokio::test]
async fn con_el_historial_completo_se_responde_en_local_sin_preguntar_al_servidor() {
    let c = cached();
    c.dialogs(all(false)).await.unwrap();
    let peer = PeerRef::User(UserId(1012));
    let first = c.history(peer, Anchor::Latest, 30).await.unwrap();
    let mine = c.send_text(peer, text("alcachofa local", 1)).await.unwrap();
    c.with_store(move |s| {
        let ck = crate::convert::find_chat_key(s, peer)?.unwrap();
        s.set_history_complete(ck, true)
    })
    .await
    .unwrap();
    assert!(!first.is_empty());
    // Un mensaje que solo conoce el servidor no aparece: no se le ha preguntado.
    c.inner().send_text(peer, text("alcachofa remota", 2)).await.unwrap();
    let page = c.search_in_chat(peer, q("alcachofa")).await.unwrap();
    assert_eq!(ids(&page), [mine.id]);
    // Y un mensaje borrado desaparece de los resultados.
    c.delete_messages(peer, vec![mine.id], true).await.unwrap();
    assert!(c.search_in_chat(peer, q("alcachofa")).await.unwrap().hits.is_empty());
    assert!(c.search_in_chat(peer, q("xyzqw")).await.unwrap().hits.is_empty());
    assert_eq!(c.search_in_chat(peer, q("  ")).await.unwrap(), SearchPage::default());
}

#[tokio::test]
async fn la_busqueda_filtra_por_tipo_en_la_cache() {
    let c = cached();
    c.dialogs(all(false)).await.unwrap();
    let peer = PeerRef::User(UserId(1012));
    let item = |kind, n: &str| OutgoingMediaItem { kind, path: "/tmp/x".into(), file_name: n.into(), size_bytes: 10, width: 4, height: 4 };
    let sent = c
        .send_media(
            peer,
            OutgoingMedia { items: vec![item(MediaKind::Photo, "a.jpg"), item(MediaKind::File, "b.pdf")], caption: "adjuntos".into(), reply_to: None, random_ids: vec![RandomId(5), RandomId(6)] },
        )
        .await
        .unwrap();
    let link = c.send_text(peer, text("mira https://example.org/x", 7)).await.unwrap();
    c.inner().set_offline(true);
    let f = |flt| SearchQuery::new("", 50).with_filter(flt);
    let has = |p: &SearchPage, id| p.hits.iter().any(|m| m.id == id);
    let photos = c.search_in_chat(peer, f(SearchFilter::Photos)).await.unwrap();
    assert!(has(&photos, sent[0].id) && !has(&photos, sent[1].id));
    let files = c.search_in_chat(peer, f(SearchFilter::Files)).await.unwrap();
    assert!(has(&files, sent[1].id) && !has(&files, sent[0].id));
    assert_eq!(ids(&c.search_in_chat(peer, SearchQuery::new("example", 5).with_filter(SearchFilter::Links)).await.unwrap()), [link.id]);
}

#[tokio::test]
async fn la_busqueda_en_un_tema_usa_el_servidor_y_sin_red_lo_guardado_con_su_tema() {
    use tc_mock::FORUM_GROUP;
    let c = cached();
    c.dialogs(all(false)).await.unwrap();
    let topics = c.forum_topics(FORUM_GROUP, TopicsQuery::first(50)).await.unwrap().topics;
    let open: Vec<_> = topics.iter().filter(|t| !t.closed).collect();
    let (t1, t2) = (open[0].thread(), open[1].thread());
    let m1 = c.send_topic_text(t1, text("pelota roja", 1)).await.unwrap();
    let m2 = c.send_topic_text(t2, text("pelota azul", 2)).await.unwrap();
    // Solo en el servidor.
    let only = c.inner().send_topic_text(t1, text("pelota verde", 3)).await.unwrap();
    let online = c.search_in_topic(t1, q("pelota")).await.unwrap();
    assert_eq!(ids(&online), [only.id, m1.id]);
    let scope = ThreadScope::new(std::sync::Arc::new(c.clone()), t2, std::sync::Arc::new(std::sync::atomic::AtomicBool::new(true)));
    assert_eq!(ids(&scope.search_in_chat(PeerRef::Group(FORUM_GROUP), q("pelota")).await.unwrap()), [m2.id]);
    c.inner().set_offline(true);
    assert_eq!(ids(&c.search_in_topic(t1, q("pelota")).await.unwrap()), [m1.id], "sin red: lo guardado de ese tema");
}

#[tokio::test]
async fn chats_secretos_se_buscan_en_el_backend_sin_tocar_la_base() {
    let c = cached();
    c.dialogs(all(false)).await.unwrap();
    let peer = PeerRef::Secret(SECRET_ID);
    let direct = c.inner().search_in_chat(peer, q("a")).await;
    assert_eq!(c.search_in_chat(peer, q("a")).await, direct);
    assert!(c.search_local("a", Some(peer), 5).await.unwrap().is_empty());
}
