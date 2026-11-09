//! Fotos de perfil y medios: mapeo de referencias y descargas contra el transporte falso
//! (DC ajeno con copia de autorización, `FILE_MIGRATE`, referencia caducada, `FLOOD_WAIT`,
//! partes de 512 KiB y límite de descargas simultáneas).
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;

use async_trait::async_trait;
use grammers_client::InvocationError;
use grammers_session::Session;
use grammers_tl_types as tl;
use tc_core::*;

use super::fixtures::*;
use super::flows::restored;
use super::*;
use crate::convert::{DialogCtx, dialog_from_tl, media_from_tl, pick_sizes};
use crate::ids::channel_group;
use crate::media::PART_SIZE;
use crate::peers::{PeerStore, group_from_tl, user_from_tl};

fn with_photo(mut u: tl::types::User, id: i64, dc: i32) -> tl::types::User {
    u.photo = Some(tl::types::UserProfilePhoto { has_video: false, personal: false, photo_id: id, stripped_thumb: None, dc_id: dc }.into());
    u
}

fn channel_with_photo(id: i64, photo: i64, dc: i32) -> tl::enums::Chat {
    let tl::enums::Chat::Channel(mut c) = channel(id, "Noticias", true, 200) else { unreachable!() };
    c.photo = tl::types::ChatPhoto { has_video: false, photo_id: photo, stripped_thumb: None, dc_id: dc }.into();
    c.into()
}

fn size(t: &str, w: i32, h: i32, bytes: i32) -> tl::enums::PhotoSize {
    tl::types::PhotoSize { r#type: t.into(), w, h, size: bytes }.into()
}

fn photo(id: i64, file_reference: Vec<u8>, dc: i32) -> tl::types::Photo {
    tl::types::Photo {
        has_stickers: false,
        id,
        access_hash: 11,
        file_reference,
        date: 0,
        sizes: vec![
            tl::types::PhotoStrippedSize { r#type: "i".into(), bytes: vec![1, 2, 3] }.into(),
            size("m", 320, 240, 9_000),
            size("x", 800, 600, 50_000),
            tl::types::PhotoSizeProgressive { r#type: "y".into(), w: 1280, h: 960, sizes: vec![1000, 90_000] }.into(),
        ],
        video_sizes: None,
        dc_id: dc,
    }
}

fn photo_media(p: tl::types::Photo) -> tl::enums::MessageMedia {
    tl::types::MessageMediaPhoto { spoiler: false, live_photo: false, photo: Some(p.into()), ttl_seconds: None, video: None }.into()
}

fn file_part(bytes: Vec<u8>) -> Reply {
    ok(tl::enums::upload::File::from(tl::types::upload::File { r#type: tl::enums::storage::FileType::FileJpeg, mtime: 0, bytes }))
}

// ------------------------------------------------------------------ mapeo

#[test]
fn photo_sizes_pick_a_bubble_thumb_and_the_largest_full() {
    let p = photo(55, vec![9], 4);
    assert_eq!(pick_sizes(&p.sizes), (Some("x".into()), Some("y".into()), 90_000));
    // Todas mayores que la burbuja: la menor como vista previa.
    assert_eq!(pick_sizes(&[size("w", 2560, 1920, 9), size("y", 1280, 960, 5)]), (Some("y".into()), Some("w".into()), 9));
    assert_eq!(pick_sizes(&[]), (None, None, 0));

    let (m, _) = media_from_tl(&photo_media(p));
    let f = m.unwrap().file.unwrap();
    assert_eq!(
        f,
        FileRef {
            kind: FileKind::Photo,
            id: 55,
            access_hash: 11,
            file_reference: vec![9],
            dc_id: 4,
            size_bytes: 90_000,
            thumb: Some("x".into()),
            full: Some("y".into())
        }
    );
}

#[test]
fn documents_keep_their_thumbnail_as_preview() {
    let d = tl::types::Document {
        id: 900,
        access_hash: 3,
        file_reference: vec![4],
        date: 0,
        mime_type: "video/mp4".into(),
        size: 5_000_000,
        thumbs: Some(vec![size("m", 320, 180, 7_000)]),
        video_thumbs: None,
        dc_id: 5,
        attributes: vec![],
    };
    let media: tl::enums::MessageMedia = tl::types::MessageMediaDocument {
        nopremium: false,
        spoiler: false,
        video: true,
        round: false,
        voice: false,
        document: Some(d.into()),
        alt_documents: None,
        video_cover: None,
        video_timestamp: None,
        ttl_seconds: None,
    }
    .into();
    let m = media_from_tl(&media).0.unwrap();
    let f = m.file.unwrap();
    assert_eq!((m.kind, f.kind, f.thumb.as_deref(), f.full.as_deref(), f.dc_id, f.size_bytes), (MediaKind::Video, FileKind::Document, Some("m"), None, 5, 5_000_000));
    assert!(f.has_image(MediaVariant::Thumb) && !f.has_image(MediaVariant::Full));
}

#[test]
fn profile_photos_reach_users_groups_and_dialogs() {
    let u = user_from_tl(&with_photo(user(10, "Ana", 100, false), 501, 4));
    assert_eq!(u.photo, Some(ProfilePhoto { id: 501, dc_id: 4 }));
    assert_eq!(user_from_tl(&user(11, "Sin foto", 1, false)).photo, None);
    let g = group_from_tl(&channel_with_photo(20, 601, 2)).unwrap();
    assert_eq!(g.photo, Some(ProfilePhoto { id: 601, dc_id: 2 }));

    let mut p = PeerStore::default();
    p.ingest_users(&[user_enum(77, "Yo", 1, true), with_photo(user(10, "Ana", 100, false), 501, 4).into()]);
    p.ingest_chats(&[channel_with_photo(20, 601, 2)]);
    // Un usuario «min» sin `apply_min_photo` no borra la foto conocida.
    let mut min = user(10, "Ana", 0, false);
    min.min = true;
    min.access_hash = None;
    p.ingest_users(&[min.into()]);
    let ctx = DialogCtx { peers: &p, me: Some(UserId(77)), now: 0 };
    let d = dialog_from_tl(&dialog(peer_user(10), 1, 0, false, None), None, None, false, &ctx);
    assert_eq!(d.photo, Some(ProfilePhoto { id: 501, dc_id: 4 }));
    let d = dialog_from_tl(&dialog(peer_channel(20), 1, 0, false, None), None, None, false, &ctx);
    assert_eq!(d.photo, Some(ProfilePhoto { id: 601, dc_id: 2 }));
    // «Mensajes guardados» no usa mi foto.
    let d = dialog_from_tl(&dialog(peer_user(77), 1, 0, false, None), None, None, false, &ctx);
    assert_eq!(d.photo, None);
}

// ------------------------------------------------------------------ descargas

#[tokio::test]
async fn profile_photo_in_another_dc_copies_auth_and_joins_parts() {
    let home = Arc::new(AtomicUsize::new(0));
    let h = home.clone();
    let imported = Arc::new(AtomicUsize::new(0));
    let imp = imported.clone();
    let (b, fake, _) = restored(
        move |dc, id, body| match id {
            x if x == cid::<tl::functions::auth::ExportAuthorization>() => {
                h.store(dc as usize, Ordering::SeqCst);
                let req: tl::functions::auth::ExportAuthorization = decode(body);
                assert_eq!(req.dc_id, 4);
                ok(tl::enums::auth::ExportedAuthorization::from(tl::types::auth::ExportedAuthorization { id: 9, bytes: vec![7; 3] }))
            }
            x if x == cid::<tl::functions::auth::ImportAuthorization>() => {
                assert_eq!(dc, 4);
                let req: tl::functions::auth::ImportAuthorization = decode(body);
                assert_eq!((req.id, req.bytes), (9, vec![7; 3]));
                imp.fetch_add(1, Ordering::SeqCst);
                ok(authorization(user(ME_ID, "Yo", 1, true)))
            }
            x if x == cid::<tl::functions::upload::GetFile>() => {
                assert_eq!(dc, 4, "se descarga en el DC de la foto");
                if imp.load(Ordering::SeqCst) == 0 {
                    return rpc_err(401, "AUTH_KEY_UNREGISTERED", None);
                }
                let req: tl::functions::upload::GetFile = decode(body);
                let tl::enums::InputFileLocation::InputPeerPhotoFileLocation(loc) = req.location else { panic!("ubicación de foto de perfil") };
                assert_eq!(loc.photo_id, 501);
                assert!(matches!(loc.peer, tl::enums::InputPeer::User(u) if u.user_id == 10 && u.access_hash == 100));
                assert_eq!(req.limit, PART_SIZE as i32);
                match req.offset {
                    0 => file_part(vec![1; PART_SIZE]),
                    o if o == PART_SIZE as i64 => file_part(vec![2; 100]),
                    o => panic!("desplazamiento inesperado {o}"),
                }
            }
            other => panic!("llamada inesperada {}", tl::name_for_id(other)),
        },
        None,
    )
    .await;
    b.inner.ingest(&[with_photo(user(10, "Ana", 100, false), 501, 4).into()], &[]);
    let photo = b.user(UserId(10)).await.unwrap().photo.unwrap();
    let bytes = b.download_profile_photo(PeerRef::User(UserId(10)), photo, PhotoSize::Small).await.unwrap();
    assert_eq!(bytes.len(), PART_SIZE + 100);
    assert_eq!(home.load(Ordering::SeqCst) as i32, b.inner.net.session.home_dc_id().unwrap());
    assert_eq!(imported.load(Ordering::SeqCst), 1);
    // El AUTH_KEY_UNREGISTERED del DC ajeno no cierra la sesión.
    assert!(matches!(b.auth_state().await.unwrap(), AuthState::LoggedIn(_)));
    // Ya autorizado: la segunda descarga no vuelve a copiar la autorización.
    b.download_profile_photo(PeerRef::User(UserId(10)), photo, PhotoSize::Small).await.unwrap();
    assert_eq!(fake.count("auth.exportAuthorization"), 1);
    // Foto de un chat secreto: no hay chat → no encontrado (sin llamar a la red).
    let before = fake.calls.lock().unwrap().len();
    assert_eq!(b.download_profile_photo(PeerRef::Secret(SecretChatId(5)), photo, PhotoSize::Small).await, Err(BackendError::NotFound));
    assert!(fake.calls.lock().unwrap()[before..].iter().all(|c| c.1 != "upload.getFile"));
    let f = FileRef { kind: FileKind::Photo, id: 1, access_hash: 1, file_reference: vec![], dc_id: 2, size_bytes: 1, thumb: None, full: Some("y".into()) };
    assert!(matches!(b.download_media(PeerRef::Secret(SecretChatId(5)), MsgId(1), f, MediaVariant::Full).await, Err(BackendError::Forbidden(_))));
}

#[tokio::test]
async fn expired_reference_is_refreshed_and_file_migrate_followed() {
    let (b, fake, _) = restored(
        |dc, id, body| match id {
            x if x == cid::<tl::functions::upload::GetFile>() => {
                let req: tl::functions::upload::GetFile = decode(body);
                let tl::enums::InputFileLocation::InputPhotoFileLocation(loc) = req.location else { panic!("ubicación de foto") };
                assert_eq!((loc.id, loc.access_hash, loc.thumb_size.as_str()), (55, 11, "x"));
                if loc.file_reference == [1] {
                    return rpc_err(400, "FILE_REFERENCE_EXPIRED", None);
                }
                assert_eq!(loc.file_reference, [2]);
                if dc != 5 { rpc_err(303, "FILE_MIGRATE", Some(5)) } else { file_part(b"jpeg".to_vec()) }
            }
            x if x == cid::<tl::functions::messages::GetMessages>() => {
                let req: tl::functions::messages::GetMessages = decode(body);
                assert!(matches!(&req.id[..], [tl::enums::InputMessage::Id(m)] if m.id == 7));
                let mut m = message(7, peer_user(10), "", false, None, 100);
                m.media = Some(photo_media(photo(55, vec![2], 2)));
                ok(tl::enums::messages::Messages::from(tl::types::messages::Messages { messages: vec![m.into()], topics: vec![], chats: vec![], users: vec![] }))
            }
            other => panic!("llamada inesperada {}", tl::name_for_id(other)),
        },
        None,
    )
    .await;
    b.inner.ingest(&[user_enum(10, "Ana", 100, false)], &[]);
    let old = media_from_tl(&photo_media(photo(55, vec![1], 2))).0.unwrap().file.unwrap();
    let bytes = b.download_media(PeerRef::User(UserId(10)), MsgId(7), old, MediaVariant::Thumb).await.unwrap();
    assert_eq!(bytes, b"jpeg");
    assert_eq!(fake.count("messages.getMessages"), 1);
    let gets: Vec<i32> = fake.calls.lock().unwrap().iter().filter(|c| c.1 == "upload.getFile").map(|c| c.0).collect();
    assert_eq!(gets[1..], [2, 5], "tras refrescar: DC del archivo y luego el de FILE_MIGRATE");
}

#[tokio::test]
async fn channel_reference_refresh_uses_channels_get_messages_and_long_flood_is_reported() {
    let (b, fake, _) = restored(
        |_, id, _| match id {
            x if x == cid::<tl::functions::upload::GetFile>() => rpc_err(400, "FILE_REFERENCE_EXPIRED", None),
            x if x == cid::<tl::functions::channels::GetMessages>() => {
                let mut m = message(3, peer_channel(20), "", false, None, 100);
                m.media = Some(photo_media(photo(55, vec![2], 2)));
                ok(tl::enums::messages::Messages::from(tl::types::messages::Messages { messages: vec![m.into()], topics: vec![], chats: vec![], users: vec![] }))
            }
            other => panic!("llamada inesperada {}", tl::name_for_id(other)),
        },
        None,
    )
    .await;
    b.inner.ingest(&[], &[channel(20, "Noticias", true, 200)]);
    let peer = PeerRef::Group(channel_group(20));
    let f = media_from_tl(&photo_media(photo(55, vec![1], 2))).0.unwrap().file.unwrap();
    // Sigue caducada tras refrescar: un solo reintento y «no encontrado».
    assert_eq!(b.download_media(peer, MsgId(3), f.clone(), MediaVariant::Full).await, Err(BackendError::NotFound));
    assert_eq!((fake.count("channels.getMessages"), fake.count("upload.getFile")), (1, 2));

    let (b, _, _) = restored(|_, _, _| rpc_err(420, "FLOOD_WAIT", Some(600)), None).await;
    b.inner.ingest(&[user_enum(10, "Ana", 100, false)], &[]);
    assert_eq!(b.download_media(PeerRef::User(UserId(10)), MsgId(3), f, MediaVariant::Thumb).await, Err(BackendError::FloodWait(600)));
}

/// Transporte que tarda en responder `upload.getFile` y cuenta cuántas hay en vuelo.
struct Slow {
    inflight: AtomicUsize,
    peak: AtomicUsize,
}

#[async_trait]
impl Transport for Slow {
    async fn invoke_raw(&self, _dc: i32, body: Vec<u8>) -> Result<Vec<u8>, InvocationError> {
        let id = u32::from_le_bytes(body[..4].try_into().unwrap());
        if id == cid::<tl::functions::users::GetUsers>() {
            return ok(vec![user_enum(ME_ID, "Yo", 1, true)]);
        }
        assert_eq!(id, cid::<tl::functions::upload::GetFile>());
        let now = self.inflight.fetch_add(1, Ordering::SeqCst) + 1;
        self.peak.fetch_max(now, Ordering::SeqCst);
        tokio::time::sleep(Duration::from_millis(30)).await;
        self.inflight.fetch_sub(1, Ordering::SeqCst);
        file_part(vec![3; 10])
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn at_most_four_downloads_run_at_once() {
    let slow = Arc::new(Slow { inflight: AtomicUsize::new(0), peak: AtomicUsize::new(0) });
    let store = Arc::new(MemorySessionStore::default());
    let b = crate::GrammersBackend::with_transport(cfg(), store, logged_session(), slow.clone(), None);
    wait_auth(&b, |s| matches!(s, AuthState::LoggedIn(_))).await;
    b.inner.ingest(&[with_photo(user(10, "Ana", 100, false), 501, 2).into()], &[]);
    let photo = ProfilePhoto { id: 501, dc_id: 2 };
    let all = futures::future::join_all((0..10).map(|_| b.download_profile_photo(PeerRef::User(UserId(10)), photo, PhotoSize::Big))).await;
    assert!(all.iter().all(|r| r.as_deref() == Ok(&[3u8; 10][..])));
    let peak = slow.peak.load(Ordering::SeqCst);
    assert!((2..=crate::media::MAX_DOWNLOADS).contains(&peak), "pico de descargas simultáneas: {peak}");
}
