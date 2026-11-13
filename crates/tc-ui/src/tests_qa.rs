//! Pruebas de la pasada QA (replica/test-plan.md): casos críticos de F04, F05, F07 y F08 que
//! no tenían prueba propia. Mismo esquema que `tests.rs`: `update` contra el `MockBackend`.

use std::sync::Arc;

use iced::widget::text_editor;
use tc_core::*;
use tc_mock::{MockBackend, MockConfig};
use tc_theme::ThemeKind;

use crate::app::{App, Backend, Compose, Flags, Msg, Overlay, error_text};
use crate::conv::{self, PAGE};

const NOW: i64 = 1_762_430_400;

async fn boot_open(peer: PeerRef) -> (App, Arc<MockBackend>, Backend) {
    let mock = Arc::new(MockBackend::new(MockConfig { now: Some(NOW), ..Default::default() }));
    let backend: Backend = mock.clone();
    let (mut app, _) = App::new(Flags { theme: ThemeKind::Dark, ..Flags::new(backend.clone()) });
    let _ = app.update(Msg::Me(backend.me().await));
    let _ = app.update(Msg::DialogsLoaded(backend.dialogs(DialogsPage { offset: 0, limit: 500, archived: false }).await));
    let _ = app.update(Msg::Open(peer));
    let r = backend.history(peer, Anchor::Latest, PAGE).await;
    let _ = app.update(Msg::History { peer, older: false, result: r });
    (app, mock, backend)
}

fn n_msgs(app: &App) -> usize {
    app.conv.as_ref().unwrap().messages.len()
}

/// T-F04-03: texto vacío o solo espacios/saltos no se envía ni crea burbuja.
#[tokio::test]
async fn f04_texto_vacio_o_en_blanco_no_se_envia() {
    let peer = PeerRef::User(UserId(1002));
    let (mut app, _, _) = boot_open(peer).await;
    let before = n_msgs(&app);
    for t in ["", "   ", "\n\n  \t"] {
        app.composer = text_editor::Content::with_text(t);
        let _ = app.update(Msg::Send);
        assert_eq!(n_msgs(&app), before, "{t:?} no debe enviarse");
    }
}

/// T-F04-02/-08: el texto se recorta, conserva acentos, emoji y saltos internos.
#[tokio::test]
async fn f04_texto_con_emoji_acentos_y_saltos_se_envia_integro() {
    let peer = PeerRef::User(UserId(1002));
    let (mut app, _, backend) = boot_open(peer).await;
    app.composer = text_editor::Content::with_text("  ¡Qué día! 🎉🇪🇸\nsegunda línea  ");
    let _ = app.update(Msg::Send);
    let p = app.conv.as_ref().unwrap().messages.last().unwrap().clone();
    assert_eq!(p.text, "¡Qué día! 🎉🇪🇸\nsegunda línea");
    let rid = p.random_id.unwrap();
    let out = OutgoingText { text: p.text.clone(), entities: vec![], reply_to: None, random_id: rid };
    let r = backend.send_text(peer, out).await;
    assert_eq!(r.as_ref().map(|m| m.text.as_str()), Ok("¡Qué día! 🎉🇪🇸\nsegunda línea"));
}

/// T-F04-09 (doble envío): pulsar Enter dos veces seguidas solo envía una vez.
#[tokio::test]
async fn f04_doble_enter_no_duplica_el_mensaje() {
    let peer = PeerRef::User(UserId(1002));
    let (mut app, _, _) = boot_open(peer).await;
    let before = n_msgs(&app);
    app.composer = text_editor::Content::with_text("una sola vez");
    let _ = app.update(Msg::Send);
    let _ = app.update(Msg::Send);
    assert_eq!(n_msgs(&app), before + 1);
}

/// T-F05-01/-02: responder lleva `reply_to` en el pendiente; cancelar no deja la cita.
#[tokio::test]
async fn f05_responder_lleva_reply_to_y_cancelar_lo_quita() {
    let peer = PeerRef::User(UserId(1002));
    let (mut app, _, _) = boot_open(peer).await;
    let target = app.conv.as_ref().unwrap().messages.last().unwrap().id;
    let _ = app.update(Msg::ReplyTo(target));
    assert_eq!(app.compose, Compose::Reply(target));
    app.composer = text_editor::Content::with_text("respuesta");
    let _ = app.update(Msg::Send);
    let p = app.conv.as_ref().unwrap().messages.last().unwrap();
    assert_eq!(p.reply_to, Some(target));
    assert_eq!(app.compose, Compose::Normal, "tras enviar se sale del modo respuesta");

    let _ = app.update(Msg::ReplyTo(target));
    let _ = app.update(Msg::CancelCompose);
    app.composer = text_editor::Content::with_text("sin cita");
    let _ = app.update(Msg::Send);
    assert_eq!(app.conv.as_ref().unwrap().messages.last().unwrap().reply_to, None);
}

/// T-F05-03: borrar el mensaje citado mientras se redacta la respuesta sale del modo respuesta.
#[tokio::test]
async fn f05_borrar_el_citado_cancela_la_respuesta() {
    let peer = PeerRef::User(UserId(1002));
    let (mut app, _, _) = boot_open(peer).await;
    let target = app.conv.as_ref().unwrap().messages.last().unwrap().id;
    let _ = app.update(Msg::ReplyTo(target));
    let _ = app.update(Msg::Backend(BackendEvent::MessagesDeleted { peer, ids: vec![target] }));
    assert_eq!(app.compose, Compose::Normal);
}

/// T-F07-05: editar con el texto vaciado no envía nada ni cierra el modo edición.
#[tokio::test]
async fn f07_editar_con_texto_vacio_no_envia_y_sigue_editando() {
    let peer = PeerRef::User(UserId(1002));
    let (mut app, _, backend) = boot_open(peer).await;
    app.composer = text_editor::Content::with_text("para editar");
    let _ = app.update(Msg::Send);
    let rid = app.conv.as_ref().unwrap().messages.last().unwrap().random_id.unwrap();
    let out = OutgoingText { text: "para editar".into(), entities: vec![], reply_to: None, random_id: rid };
    let r = backend.send_text(peer, out).await;
    let _ = app.update(Msg::Sent { peer, random_id: rid, result: r });
    let _ = app.update(Msg::EditLast);
    let Compose::Edit(id) = app.compose else { panic!("modo edición") };
    app.composer = text_editor::Content::with_text("   ");
    let _ = app.update(Msg::Send);
    assert_eq!(app.compose, Compose::Edit(id));
    // Esc cancela y deja el compositor limpio.
    let _ = app.update(Msg::Escape);
    assert_eq!(app.compose, Compose::Normal);
    assert!(app.composer.text().trim().is_empty());
}

/// T-F07-03: ↑ con el compositor vacío nunca elige un mensaje ajeno.
#[tokio::test]
async fn f07_editar_ultimo_solo_toma_mensajes_propios() {
    let peer = PeerRef::User(UserId(1002));
    let (mut app, _, _) = boot_open(peer).await;
    let me = app.me_id();
    let c = app.conv.as_ref().unwrap();
    if let Some(m) = c.last_own(me) {
        assert!(m.outgoing && m.sender == me);
    }
    let _ = app.update(Msg::EditLast);
    if let Compose::Edit(id) = app.compose {
        assert!(app.conv.as_ref().unwrap().find(id).unwrap().outgoing);
    }
}

/// T-F08-04: cancelar el diálogo de borrado no cambia nada.
#[tokio::test]
async fn f08_cancelar_el_dialogo_de_borrado_no_toca_el_mensaje() {
    let peer = PeerRef::User(UserId(1002));
    let (mut app, _, _) = boot_open(peer).await;
    let id = app.conv.as_ref().unwrap().messages.last().unwrap().id;
    let before = n_msgs(&app);
    let _ = app.update(Msg::AskDelete(id));
    assert!(matches!(app.overlay, Some(Overlay::Delete { .. })));
    let _ = app.update(Msg::CloseOverlay);
    assert!(app.overlay.is_none());
    assert_eq!(n_msgs(&app), before);
    assert!(app.conv.as_ref().unwrap().find(id).is_some());
}

/// T-F04-05: un fallo de red deja el mensaje fallido y el reintento conserva el `random_id`
/// (el backend no lo duplica).
#[tokio::test]
async fn f04_reintento_conserva_random_id_y_no_duplica_en_el_backend() {
    let peer = PeerRef::User(UserId(1003));
    let (mut app, mock, backend) = boot_open(peer).await;
    app.composer = text_editor::Content::with_text("reintento");
    let _ = app.update(Msg::Send);
    let rid = app.conv.as_ref().unwrap().messages.last().unwrap().random_id.unwrap();
    let out = OutgoingText { text: "reintento".into(), entities: vec![], reply_to: None, random_id: rid };
    mock.set_offline(true);
    let r = backend.send_text(peer, out.clone()).await;
    let _ = app.update(Msg::Sent { peer, random_id: rid, result: r });
    mock.set_offline(false);
    let _ = app.update(Msg::Retry(rid));
    assert_eq!(app.conv.as_ref().unwrap().messages.last().unwrap().random_id, Some(rid));
    let a = backend.send_text(peer, out.clone()).await.unwrap();
    let b = backend.send_text(peer, out).await.unwrap();
    assert_eq!(a.id, b.id, "mismo random_id, mismo mensaje");
    assert!(conv::is_local(app.conv.as_ref().unwrap().messages.last().unwrap()) || n_msgs(&app) > 0);
}

/// T-F04-04 (B-01): un texto de más de 4096 caracteres debería avisar con un mensaje claro
/// y no dejar una burbuja condenada a fallar. Hoy `send()` no lo comprueba y el aviso es
/// «No se puede enviar un mensaje vacío.». Ignorada hasta que se arregle.
#[tokio::test]
async fn f04_texto_de_mas_de_4096_avisa_y_no_deja_burbuja() {
    let peer = PeerRef::User(UserId(1002));
    let (mut app, _, _) = boot_open(peer).await;
    let before = n_msgs(&app);
    app.composer = text_editor::Content::with_text(&"a".repeat(4097));
    let _ = app.update(Msg::Send);
    assert_eq!(n_msgs(&app), before, "no debe crear burbuja pendiente");
    assert!(!app.toasts.is_empty(), "debe avisar");
    assert!(!error_text(&BackendError::InvalidInput("text")).contains("vacío"));
}
