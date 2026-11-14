//! Envío de archivos contra el mock (fase A2): selección → diálogo, envío con progreso, cancelar a
//! mitad, fallo y reintento, álbum, pie demasiado largo, temas de foro y chats de solo lectura.
//! Como en el resto de pruebas, las `Task` no se ejecutan: la prueba hace la llamada que haría la
//! tarea (con el `OutgoingMedia` que preparó la UI) y entrega el resultado como lo haría iced.

use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use futures::{FutureExt, StreamExt};
use iced::widget::text_editor;
use tc_core::*;
use tc_mock::{BIG_CHAT, FORUM_CHAT, FORUM_GROUP, MockBackend, MockConfig, TransferSim};
use tc_theme::ThemeKind;

use crate::app::forum::ForumMsg;
use crate::app::uploads::FilesMsg;
use crate::app::{App, Backend, Compose, Flags, Msg};
use crate::conv::{self, PAGE};
use crate::files::{FileClass, MAX_CAPTION_CHARS, Picked};

const NOW: i64 = 1_762_430_400;

async fn boot() -> (App, Arc<MockBackend>) {
    let mock = Arc::new(MockBackend::new(MockConfig { now: Some(NOW), ..Default::default() }));
    let backend: Backend = mock.clone();
    let (mut app, _) = App::new(Flags { theme: ThemeKind::Dark, ..Flags::new(backend.clone()) });
    let _ = app.update(Msg::Me(backend.me().await));
    let _ = app.update(Msg::DialogsLoaded(backend.dialogs(DialogsPage { offset: 0, limit: 500, archived: false }).await));
    let _ = app.update(Msg::GroupLoaded(backend.group(FORUM_GROUP).await));
    let _ = app.update(Msg::Open(BIG_CHAT));
    let r = backend.history(BIG_CHAT, Anchor::Latest, PAGE).await;
    let _ = app.update(Msg::History { peer: BIG_CHAT, older: false, result: r });
    (app, mock)
}

fn file(name: &str, size: u64) -> Picked {
    let class = crate::files::classify(name);
    let (w, h) = if class == FileClass::Image { (1280, 960) } else { (0, 0) };
    Picked { path: PathBuf::from(format!("/home/ana/{name}")), name: name.into(), size, class, width: w, height: h, preview: None }
}

fn pick(app: &mut App, files: Vec<Picked>) {
    let _ = app.update(Msg::Files(FilesMsg::Inspected { files, errors: vec![] }));
}

fn pump(app: &mut App, ev: &mut futures::stream::BoxStream<'static, BackendEvent>) -> Vec<BackendEvent> {
    let mut seen = Vec::new();
    while let Some(Some(e)) = ev.next().now_or_never() {
        seen.push(e.clone());
        let _ = app.update(Msg::Backend(e));
    }
    seen
}

fn pendings(app: &App) -> Vec<Message> {
    app.conv.as_ref().unwrap().messages.iter().filter(|m| conv::is_local(m)).cloned().collect()
}

/// El envío que acaba de preparar la UI (primer `random_id`, `OutgoingMedia`).
fn last_job(app: &App) -> (RandomId, OutgoingMedia) {
    let first = pendings(app).last().and_then(|m| m.random_id).map(|r| app.uploads.owner[&r]).expect("hay un envío");
    (first, app.uploads.jobs[&first].media.clone())
}

fn has_error_toast(app: &App, needle: &str) -> bool {
    app.toasts.iter().any(|t| t.error && t.text.contains(needle))
}

#[tokio::test]
async fn elegir_archivos_abre_el_dialogo_y_cerrar_devuelve_el_texto() {
    let (mut app, _mock) = boot().await;
    app.composer = text_editor::Content::with_text("mira esto");
    pick(&mut app, vec![file("playa.jpg", 2_000_000)]);
    let d = app.uploads.dialog.as_ref().expect("diálogo abierto");
    assert_eq!(d.peer, BIG_CHAT);
    assert!(d.compress, "las fotos se comprimen por defecto");
    assert_eq!(d.caption_text(), "mira esto", "lo escrito pasa al pie");
    assert!(app.composer.text().trim().is_empty());
    assert_eq!(crate::view::send_dialog::dialog_title(d), "Enviar una foto");

    // Más archivos con el diálogo abierto (Añadir, soltar otro): se suman sin repetir.
    pick(&mut app, vec![file("playa.jpg", 2_000_000), file("informe.pdf", 300_000)]);
    let d = app.uploads.dialog.as_ref().unwrap();
    assert_eq!(d.files.len(), 2);
    assert_eq!(crate::view::send_dialog::dialog_title(d), "Enviar 2 archivos");
    // Quitar y elegir «enviar como archivo».
    let _ = app.update(Msg::Files(FilesMsg::Remove(1)));
    let _ = app.update(Msg::Files(FilesMsg::Compress(false)));
    let d = app.uploads.dialog.as_ref().unwrap();
    assert_eq!(d.files.len(), 1);
    assert!(!d.compress);
    assert_eq!(d.files[0].send_kind(d.compress), MediaKind::File);

    // Esc cierra y el texto vuelve al compositor; no se envía nada.
    let _ = app.update(Msg::Escape);
    assert!(app.uploads.dialog.is_none());
    assert_eq!(app.composer.text().trim(), "mira esto");
    assert!(pendings(&app).is_empty());

    // Quitar el último archivo también cierra.
    pick(&mut app, vec![file("a.pdf", 10)]);
    let _ = app.update(Msg::Files(FilesMsg::Remove(0)));
    assert!(app.uploads.dialog.is_none());

    // Archivos ilegibles: aviso y sin diálogo.
    let _ = app.update(Msg::Files(FilesMsg::Inspected { files: vec![], errors: vec!["No se puede leer «x».".into()] }));
    assert!(app.uploads.dialog.is_none());
    assert!(has_error_toast(&app, "No se puede leer"));
}

#[tokio::test]
async fn inspeccion_real_de_un_archivo_soltado() {
    let (mut app, _mock) = boot().await;
    let dir = std::env::temp_dir().join(format!("parlo-drop-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let png = dir.join("captura.png");
    ::image::RgbaImage::from_pixel(64, 48, ::image::Rgba([10, 120, 200, 255])).save(&png).unwrap();
    let _ = app.update(Msg::Files(FilesMsg::Hover(true)));
    assert!(app.uploads.hover);
    let _ = app.update(Msg::Files(FilesMsg::Dropped(png.clone())));
    assert!(!app.uploads.hover, "al soltar desaparece la zona");
    // Lo que haría la tarea de inspección.
    let (files, errors) = crate::files::inspect(vec![png]).await;
    assert!(errors.is_empty());
    let _ = app.update(Msg::Files(FilesMsg::Inspected { files, errors }));
    let d = app.uploads.dialog.as_ref().unwrap();
    assert_eq!((d.files[0].width, d.files[0].height), (64, 48));
    assert!(d.files[0].preview.is_some());
    std::fs::remove_dir_all(&dir).ok();
}

#[tokio::test]
async fn pegar_texto_normal_va_al_compositor() {
    let (mut app, _mock) = boot().await;
    let _ = app.update(Msg::Files(FilesMsg::PasteText(Some("hola".into()))));
    assert_eq!(app.composer.text().trim(), "hola");
    assert!(app.uploads.dialog.is_none());
}

#[tokio::test(start_paused = true)]
async fn envio_con_progreso_y_confirmacion() {
    let (mut app, mock) = boot().await;
    let mut ev = mock.events();
    mock.set_transfer_sim(TransferSim { chunk: 100_000, delay: Duration::from_millis(100), fail_at: None });
    let _ = app.update(Msg::ReplyTo(app.conv.as_ref().unwrap().newest_real().unwrap()));
    pick(&mut app, vec![file("informe.pdf", 1_000_000)]);
    let _ = app.update(Msg::Files(FilesMsg::Send));
    assert!(app.uploads.dialog.is_none());
    assert_eq!(app.compose, Compose::Normal, "la respuesta se gasta en el envío");
    let p = pendings(&app);
    assert_eq!(p.len(), 1);
    assert_eq!(p[0].state, SendState::Pending);
    assert!(p[0].reply_to.is_some());
    let (first, media) = last_job(&app);
    assert_eq!(TransferId::upload_of(&media), Some(TransferId::Upload(first)));
    assert_eq!(media.items[0].kind, MediaKind::File);
    let job = app.upload_of(&p[0]).unwrap().1;
    assert_eq!(job.fraction(), None, "sin eventos aún: «Enviando…»");
    assert!(crate::view::send_dialog::status_label(job).starts_with("Enviando…"));

    let b = app.scoped_backend().unwrap();
    let send = tokio::spawn(async move { b.send_media(BIG_CHAT, media).await });
    tokio::time::sleep(Duration::from_millis(450)).await;
    pump(&mut app, &mut ev);
    let job = &app.uploads.jobs[&first];
    let f = job.fraction().unwrap();
    assert!(f > 0.2 && f < 0.8, "progreso a mitad: {f}");
    assert!(crate::view::send_dialog::status_label(job).contains(" % · "));

    let result = send.await.unwrap();
    let sent = result.clone().unwrap();
    pump(&mut app, &mut ev);
    let _ = app.update(Msg::Files(FilesMsg::Sent { peer: BIG_CHAT, first, result }));
    assert!(pendings(&app).is_empty());
    assert!(app.uploads.jobs.is_empty() && app.uploads.owner.is_empty());
    let c = app.conv.as_ref().unwrap();
    assert_eq!(c.messages.iter().filter(|m| m.id == sent[0].id).count(), 1, "sin duplicar");
    assert_eq!(c.find(sent[0].id).unwrap().media.as_ref().unwrap().file_name.as_deref(), Some("informe.pdf"));
}

#[tokio::test(start_paused = true)]
async fn cancelar_a_mitad_quita_la_burbuja_sin_marcar_fallo() {
    let (mut app, mock) = boot().await;
    let mut ev = mock.events();
    mock.set_transfer_sim(TransferSim { chunk: 100_000, delay: Duration::from_millis(100), fail_at: None });
    let n0 = app.conv.as_ref().unwrap().messages.len();
    pick(&mut app, vec![file("video.mp4", 1_000_000)]);
    let _ = app.update(Msg::Files(FilesMsg::Send));
    let (first, media) = last_job(&app);
    let b = app.scoped_backend().unwrap();
    let send = tokio::spawn(async move { b.send_media(BIG_CHAT, media).await });
    tokio::time::sleep(Duration::from_millis(350)).await;
    pump(&mut app, &mut ev);
    assert!(app.uploads.jobs[&first].done > 0);

    // Botón «Cancelar» de la burbuja → la tarea llama a `cancel_transfer`.
    let rid = pendings(&app)[0].random_id.unwrap();
    let _ = app.update(Msg::Files(FilesMsg::Cancel(rid)));
    assert!(app.uploads.jobs[&first].cancelling);
    assert_eq!(crate::view::send_dialog::status_label(&app.uploads.jobs[&first]), "Cancelando…");
    mock.cancel_transfer(TransferId::Upload(first)).await.unwrap();
    let result = send.await.unwrap();
    assert_eq!(result, Err(BackendError::Cancelled));
    let seen = pump(&mut app, &mut ev);
    assert!(seen.iter().any(|e| matches!(e, BackendEvent::TransferProgress(t) if t.state == TransferState::Cancelled)));
    let _ = app.update(Msg::Files(FilesMsg::Sent { peer: BIG_CHAT, first, result }));
    assert!(pendings(&app).is_empty(), "la burbuja se va");
    assert_eq!(app.conv.as_ref().unwrap().messages.len(), n0);
    assert!(app.uploads.jobs.is_empty());
    assert!(!app.toasts.iter().any(|t| t.error), "cancelar no es un error");
}

#[tokio::test(start_paused = true)]
async fn fallo_y_reintento() {
    let (mut app, mock) = boot().await;
    let mut ev = mock.events();
    mock.set_transfer_sim(TransferSim { chunk: 100_000, delay: Duration::from_millis(10), fail_at: Some((300_000, BackendError::Network)) });
    pick(&mut app, vec![file("tesis.pdf", 600_000)]);
    let _ = app.update(Msg::Files(FilesMsg::Send));
    let (first, media) = last_job(&app);
    let result = app.scoped_backend().unwrap().send_media(BIG_CHAT, media).await;
    assert_eq!(result, Err(BackendError::Network));
    pump(&mut app, &mut ev);
    let _ = app.update(Msg::Files(FilesMsg::Sent { peer: BIG_CHAT, first, result }));
    let p = pendings(&app);
    assert_eq!(p[0].state, SendState::Failed, "queda para reintentar");
    assert!(app.uploads.jobs[&first].failed);
    assert_eq!(crate::view::send_dialog::status_label(&app.uploads.jobs[&first]), "No se envió");
    assert!(has_error_toast(&app, "Sin conexión"));

    // «Reintentar» (el mismo mensaje que usan los textos fallidos).
    mock.set_transfer_sim(TransferSim::default());
    let _ = app.update(Msg::Retry(p[0].random_id.unwrap()));
    assert_eq!(pendings(&app)[0].state, SendState::Pending);
    let (first2, media) = last_job(&app);
    assert_eq!(first2, first);
    assert!(!app.uploads.jobs[&first].failed);
    let result = app.scoped_backend().unwrap().send_media(BIG_CHAT, media).await;
    assert!(result.is_ok());
    pump(&mut app, &mut ev);
    let _ = app.update(Msg::Files(FilesMsg::Sent { peer: BIG_CHAT, first, result }));
    assert!(pendings(&app).is_empty());
    assert!(app.uploads.jobs.is_empty());
}

#[tokio::test]
async fn varios_archivos_forman_un_album_con_el_pie_en_el_primero() {
    let (mut app, mock) = boot().await;
    let mut ev = mock.events();
    pick(&mut app, vec![file("1.jpg", 900_000), file("2.png", 800_000), file("3.mp4", 5_000_000)]);
    let d = app.uploads.dialog.as_mut().unwrap();
    d.caption = text_editor::Content::with_text("Vacaciones ☀️");
    assert_eq!(crate::view::send_dialog::dialog_title(d), "Enviar 3 fotos y vídeos");
    let _ = app.update(Msg::Files(FilesMsg::Send));
    let p = pendings(&app);
    assert_eq!(p.len(), 3, "una burbuja por elemento");
    assert_eq!(p[0].text, "Vacaciones ☀️");
    assert!(p[1].text.is_empty() && p[2].text.is_empty());
    assert_eq!(app.uploads.jobs.len(), 1, "un solo envío (un álbum)");
    let (first, media) = last_job(&app);
    assert_eq!(media.items.iter().map(|i| i.kind).collect::<Vec<_>>(), [MediaKind::Photo, MediaKind::Photo, MediaKind::Video]);
    assert_eq!(media.caption, "Vacaciones ☀️");
    assert_eq!(media.random_ids.len(), 3);
    // Las tres burbujas comparten el progreso (y el «Cancelar») del envío.
    assert!(p.iter().all(|m| app.upload_of(m).map(|(f, _)| f) == Some(first)));

    let result = app.scoped_backend().unwrap().send_media(BIG_CHAT, media).await;
    pump(&mut app, &mut ev);
    let _ = app.update(Msg::Files(FilesMsg::Sent { peer: BIG_CHAT, first, result }));
    assert!(pendings(&app).is_empty());
    let c = app.conv.as_ref().unwrap();
    let last3: Vec<&Message> = c.messages.iter().rev().take(3).collect();
    assert!(last3.iter().all(|m| m.outgoing && m.media.is_some()));

    // Fotos y un PDF: dos envíos (Telegram no mezcla documentos en un álbum de fotos).
    pick(&mut app, vec![file("a.jpg", 1000), file("b.pdf", 1000), file("c.jpg", 1000)]);
    let _ = app.update(Msg::Files(FilesMsg::Send));
    assert_eq!(app.uploads.jobs.len(), 2);
}

#[tokio::test]
async fn pie_demasiado_largo_no_se_envia() {
    let (mut app, _mock) = boot().await;
    pick(&mut app, vec![file("a.jpg", 1000)]);
    let long = "a".repeat(MAX_CAPTION_CHARS + 1);
    app.uploads.dialog.as_mut().unwrap().caption = text_editor::Content::with_text(&long);
    assert!(app.uploads.dialog.as_ref().unwrap().caption_too_long());
    let _ = app.update(Msg::Files(FilesMsg::Send));
    assert!(app.uploads.dialog.is_some(), "el diálogo sigue abierto para recortarlo");
    assert!(pendings(&app).is_empty());
    assert!(has_error_toast(&app, "1024"));
    // Justo en el límite, sí.
    app.uploads.dialog.as_mut().unwrap().caption = text_editor::Content::with_text(&"ñ".repeat(MAX_CAPTION_CHARS));
    let _ = app.update(Msg::Files(FilesMsg::Send));
    assert!(app.uploads.dialog.is_none());
    assert_eq!(pendings(&app).len(), 1);
}

#[tokio::test]
async fn enter_envia_y_los_atajos_esperan_con_el_dialogo_abierto() {
    let (mut app, _mock) = boot().await;
    pick(&mut app, vec![file("a.jpg", 1000)]);
    let enter = iced::keyboard::Event::KeyPressed {
        key: iced::keyboard::Key::Named(iced::keyboard::key::Named::Enter),
        modified_key: iced::keyboard::Key::Named(iced::keyboard::key::Named::Enter),
        physical_key: iced::keyboard::key::Physical::Code(iced::keyboard::key::Code::Enter),
        location: iced::keyboard::Location::Standard,
        modifiers: iced::keyboard::Modifiers::empty(),
        text: None,
        repeat: false,
    };
    let _ = app.update(Msg::Key(enter));
    assert!(app.uploads.dialog.is_none());
    assert_eq!(pendings(&app).len(), 1);
}

#[tokio::test]
async fn en_un_tema_del_foro_va_al_tema_y_respeta_el_tema_cerrado() {
    let (mut app, mock) = boot().await;
    let mut ev = mock.events();
    let _ = app.update(Msg::Open(FORUM_CHAT));
    let r = mock.forum_topics(FORUM_GROUP, TopicsQuery::first(crate::app::forum::TOPICS_PAGE)).await;
    let _ = app.update(Msg::Forum(ForumMsg::Loaded { group: FORUM_GROUP, append: false, query: None, result: r }));
    let t = app.forum.topics[&FORUM_GROUP].items.iter().find(|t| !t.closed && !t.is_general()).cloned().unwrap();
    let _ = app.update(Msg::Forum(ForumMsg::OpenTopic(t.thread())));
    let anchor = crate::reading::initial_anchor(app.reading_dialog(FORUM_CHAT).as_deref());
    let r = app.scoped_backend().unwrap().history(FORUM_CHAT, anchor, PAGE).await;
    let _ = app.update(Msg::History { peer: FORUM_CHAT, older: false, result: r });

    pick(&mut app, vec![file("plano.pdf", 4000)]);
    assert_eq!(app.uploads.dialog.as_ref().unwrap().thread, Some(t.thread()));
    let _ = app.update(Msg::Files(FilesMsg::Send));
    let (first, media) = last_job(&app);
    let result = app.scoped_backend().unwrap().send_media(FORUM_CHAT, media).await;
    let sent = result.clone().unwrap();
    pump(&mut app, &mut ev);
    let _ = app.update(Msg::Files(FilesMsg::Sent { peer: FORUM_CHAT, first, result }));
    let in_topic = mock.topic_history(t.thread(), Anchor::Latest, 5).await.unwrap();
    assert_eq!(in_topic.last().map(|m| m.id), Some(sent[0].id), "llega al tema, no al General");
    assert!(app.conv.as_ref().unwrap().find(sent[0].id).is_some());

    // Tema cerrado: ni diálogo ni clip; se explica por qué.
    let closed = app.forum.topics[&FORUM_GROUP].items.iter().find(|t| t.closed).cloned().unwrap();
    let _ = app.update(Msg::Forum(ForumMsg::OpenTopic(closed.thread())));
    assert!(app.attach_block().is_some());
    pick(&mut app, vec![file("x.pdf", 10)]);
    assert!(app.uploads.dialog.is_none());
    assert!(has_error_toast(&app, "cerrado"));
}

#[tokio::test]
async fn sin_chat_o_editando_no_se_adjunta() {
    let (mut app, _mock) = boot().await;
    let id = app.conv.as_ref().unwrap().last_own(app.me_id()).map(|m| m.id).unwrap();
    let _ = app.update(Msg::StartEdit(id));
    pick(&mut app, vec![file("x.pdf", 10)]);
    assert!(app.uploads.dialog.is_none());
    assert!(has_error_toast(&app, "Termina de editar"));
    let _ = app.update(Msg::CloseChat);
    assert_eq!(app.attach_block().as_deref(), Some("Abre un chat para enviar archivos."));
    let _ = app.update(Msg::Files(FilesMsg::Hover(true)));
    assert!(!app.uploads.hover, "sin chat no hay zona para soltar");
}
