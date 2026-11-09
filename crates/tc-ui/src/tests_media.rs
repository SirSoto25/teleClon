//! Fotos de perfil y de mensajes contra el `MockBackend` (imágenes sintéticas): qué se pide, que la
//! carga real (descarga + decodificado fuera del hilo de la UI) llega a la vista y el visor.
//! Como en el resto de pruebas, las `Task` no se ejecutan: la prueba hace la carga y entrega el
//! resultado como lo haría el runtime de iced.

use std::sync::Arc;

use tc_core::*;
use tc_mock::{MockBackend, MockConfig};
use tc_theme::ThemeKind;

use crate::app::{App, Backend, Flags, Msg, Overlay};
use crate::conv::PAGE;
use crate::media::{self, ImageKey, ImageRequest};

const NOW: i64 = 1_762_430_400;

async fn boot() -> (App, Backend) {
    let mock = Arc::new(MockBackend::new(MockConfig { now: Some(NOW), ..Default::default() }));
    let backend: Backend = mock;
    let (mut app, _) = App::new(Flags { theme: ThemeKind::Dark, ..Flags::new(backend.clone()) });
    let _ = app.update(Msg::Me(backend.me().await));
    let _ = app.update(Msg::Contacts(backend.contacts().await));
    let _ = app.update(Msg::DialogsLoaded(backend.dialogs(DialogsPage { offset: 0, limit: 500, archived: false }).await));
    (app, backend)
}

async fn deliver(app: &mut App, backend: &Backend, req: ImageRequest) -> ImageKey {
    let key = req.key();
    let img = media::load(backend.clone(), req).await;
    let _ = app.update(Msg::ImageLoaded(key, img));
    key
}

#[tokio::test]
async fn la_lista_pide_solo_avatares_visibles_y_los_dibuja_al_llegar() {
    let (mut app, backend) = boot().await;
    // Tras cargar la lista ya hay cargas en curso, sin pasar del tope.
    assert!(app.images.inflight() > 0);
    assert!(app.images.inflight() <= media::MAX_INFLIGHT);
    let d = app.visible_dialogs().into_iter().find(|d| matches!(d.peer, PeerRef::User(_)) && d.photo.is_some()).cloned().expect("chat con foto");
    let (peer, photo) = (d.peer, d.photo.unwrap());
    assert!(app.avatar_handle(peer, Some(photo)).is_none(), "aún el avatar generado");
    let key = deliver(&mut app, &backend, ImageRequest::Avatar { peer, photo }).await;
    assert_eq!(key, ImageKey::Avatar { peer, photo: photo.id });
    let img = app.images.get(&key).expect("decodificada");
    assert_eq!((img.width, img.height), (media::AVATAR_PX, media::AVATAR_PX));
    assert!(app.avatar_handle(peer, Some(photo)).is_some());
    // El mismo usuario como remitente comparte la imagen.
    if let PeerRef::User(u) = peer {
        assert!(app.user_avatar(u).is_some());
    }
    // Una foto que ya no es la vigente falla y se queda con el avatar generado.
    let stale = ProfilePhoto { id: photo.id + 1, dc_id: photo.dc_id };
    let key = deliver(&mut app, &backend, ImageRequest::Avatar { peer, photo: stale }).await;
    assert!(app.images.failed(&key));
    assert!(app.avatar_handle(peer, Some(stale)).is_none());
}

#[tokio::test]
async fn fotos_de_la_conversacion_y_visor() {
    let (mut app, backend) = boot().await;
    // Un chat con alguna foto entre sus últimos mensajes.
    let mut chosen = None;
    for d in app.dialogs.clone() {
        if matches!(d.peer, PeerRef::Secret(_)) {
            continue;
        }
        let h = backend.history(d.peer, Anchor::Latest, PAGE).await.unwrap();
        let tail = h.len().saturating_sub(20);
        if let Some(m) = h[tail..].iter().rev().find(|m| m.media.as_ref().is_some_and(|md| md.kind == MediaKind::Photo)) {
            chosen = Some((d.peer, m.clone(), h));
            break;
        }
    }
    let (peer, m, page) = chosen.expect("alguna foto reciente en el mock");
    let _ = app.update(Msg::Open(peer));
    let _ = app.update(Msg::History { peer, older: false, result: Ok(page) });
    let cv = app.conv.as_ref().unwrap();
    let (a, z) = cv.image_range();
    assert_eq!(z, cv.messages.len());
    assert!(cv.messages[a..z].iter().any(|x| x.id == m.id), "la foto está en la ventana que se carga");

    let req = App::media_request(peer, &m, MediaVariant::Thumb).expect("foto descargable");
    let key = deliver(&mut app, &backend, req).await;
    let md = m.media.as_ref().unwrap();
    let thumb = app.images.get(&key).cloned().expect("vista previa decodificada");
    // Recortada a la proporción de la burbuja.
    let want = media::bubble_aspect(md.width, md.height).unwrap();
    assert!((thumb.height as f32 / thumb.width as f32 - want).abs() < 0.01);
    assert!(thumb.width <= media::BUBBLE_PX);

    // Visor: abre, pide la completa y se cierra con Escape.
    let _ = app.update(Msg::OpenPhoto { peer, msg: m.id });
    assert_eq!(app.overlay, Some(Overlay::Photo { peer, msg: m.id }));
    let full = App::media_request(peer, &m, MediaVariant::Full).unwrap();
    let fkey = full.key();
    assert!(app.images.is_loading(&fkey) || app.images.get(&fkey).is_some(), "el visor pide la foto completa");
    deliver(&mut app, &backend, full).await;
    let f = app.images.get(&fkey).unwrap();
    assert!(f.width.max(f.height) <= media::VIEWER_PX && f.width >= thumb.width);
    let _ = app.update(Msg::Escape);
    assert_eq!(app.overlay, None);

    // Fotos de otro chat no abren el visor; los documentos no tienen vista previa.
    let _ = app.update(Msg::OpenPhoto { peer: PeerRef::User(UserId(-1)), msg: m.id });
    assert_eq!(app.overlay, None);
    let mut doc = m.clone();
    if let Some(md) = doc.media.as_mut() {
        md.kind = MediaKind::File;
    }
    assert!(App::media_request(peer, &doc, MediaVariant::Thumb).is_none());
}

#[tokio::test]
async fn el_progreso_de_descarga_se_registra_y_se_limpia() {
    let (mut app, _) = boot().await;
    let _ = app.update(Msg::Backend(BackendEvent::Download { file_id: 7, done: 10, total: 40 }));
    assert_eq!(app.downloads.get(&7), Some(&(10, 40)));
    let key = ImageKey::Full { peer: PeerRef::User(UserId(1)), msg: MsgId(1), file: 7 };
    let _ = app.update(Msg::ImageLoaded(key, None));
    assert!(app.downloads.is_empty());
}
