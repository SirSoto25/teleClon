//! Pruebas de `update` contra el `MockBackend` real (sin ventana): las `Task` que devuelve
//! `update` no se ejecutan aquí; la prueba llama al backend y entrega el resultado como lo haría
//! el runtime de iced.

use std::sync::Arc;

use iced::widget::text_editor;
use tc_core::*;
use tc_mock::{BIG_CHAT, BIG_CHAT_LEN, MockBackend, MockConfig, SECRET_ID};
use tc_theme::ThemeKind;

use crate::app::{App, Backend, Compose, Flags, Folder, Msg, Overlay};
use crate::conv::PAGE as PAGE_FOR_TESTS;
use crate::conv;

const NOW: i64 = 1_762_430_400;

async fn boot() -> (App, Arc<MockBackend>, Backend) {
    let mock = Arc::new(MockBackend::new(MockConfig { now: Some(NOW), ..Default::default() }));
    let backend: Backend = mock.clone();
    let (mut app, _) = App::new(Flags { backend: backend.clone(), on_start: None, theme: ThemeKind::Dark, reduced_motion: false, demo: vec![], on_screenshot: None });
    let _ = app.update(Msg::Me(backend.me().await));
    let _ = app.update(Msg::Contacts(backend.contacts().await));
    let _ = app.update(Msg::Secrets(backend.secret_chats().await));
    let _ = app.update(Msg::DialogsLoaded(backend.dialogs(DialogsPage { offset: 0, limit: 500, archived: false }).await));
    for g in 5000..5008 {
        let _ = app.update(Msg::GroupLoaded(backend.group(GroupId(g)).await));
    }
    (app, mock, backend)
}

async fn open(app: &mut App, backend: &Backend, peer: PeerRef) {
    let _ = app.update(Msg::Open(peer));
    let r = backend.history(peer, Anchor::Latest, PAGE_FOR_TESTS).await;
    let _ = app.update(Msg::History { peer, older: false, result: r });
}

#[tokio::test]
async fn lista_fijados_primero_busqueda_y_carpetas() {
    let (mut app, _, _) = boot().await;
    assert!(app.dialogs.len() >= 20);
    let pinned: Vec<_> = app.dialogs.iter().take(3).map(|d| d.pinned_order).collect();
    assert_eq!(pinned, vec![Some(0), Some(1), Some(2)]);
    assert_eq!(app.dialogs[0].peer, BIG_CHAT);
    // Actividad descendente tras los fijados.
    let rest: Vec<_> = app.dialogs.iter().skip(3).map(|d| d.activity).collect();
    assert!(rest.windows(2).all(|w| w[0] >= w[1]));

    let _ = app.update(Msg::SearchChanged("LUCIA".into()));
    let v = app.visible_dialogs();
    assert_eq!(v.len(), 1);
    assert!(v[0].title.starts_with("Lucía"));
    let _ = app.update(Msg::SearchChanged("zzzz".into()));
    assert!(app.visible_dialogs().is_empty());
    let _ = app.update(Msg::ClearSearch);

    let _ = app.update(Msg::SetFolder(Folder::Channels));
    assert!(!app.visible_dialogs().is_empty());
    assert!(app.visible_dialogs().iter().all(|d| app.is_channel(d.peer)));
    let _ = app.update(Msg::SetFolder(Folder::Personal));
    assert!(app.visible_dialogs().iter().all(|d| matches!(d.peer, PeerRef::User(_) | PeerRef::Secret(_))));
    let _ = app.update(Msg::SetFolder(Folder::Unread));
    assert!(app.visible_dialogs().iter().all(|d| d.unread_count > 0));
}

#[tokio::test]
async fn entrante_en_vivo_reordena_la_lista() {
    let (mut app, mock, _) = boot().await;
    let last = app.dialogs.last().unwrap().peer;
    let m = mock.inject_incoming(last, "¿Sigues por ahí?").unwrap();
    let _ = app.update(Msg::Backend(BackendEvent::MessageNew(m.clone())));
    let d = mock.dialogs(DialogsPage { offset: 0, limit: 500, archived: false }).await.unwrap();
    let up = d.into_iter().find(|d| d.peer == last).unwrap();
    let _ = app.update(Msg::Backend(BackendEvent::DialogUpserted(up)));
    // Fijados intactos y el chat con el mensaje nuevo justo debajo.
    assert_eq!(app.dialogs[3].peer, last);
    assert!(app.dialogs[3].unread_count >= 1);
}

#[tokio::test]
async fn historial_paginado_carga_los_5000_sin_huecos() {
    let (mut app, _, backend) = boot().await;
    open(&mut app, &backend, BIG_CHAT).await;
    assert_eq!(app.conv.as_ref().unwrap().messages.len(), PAGE_FOR_TESTS);
    let mut guard = 0;
    while !app.conv.as_ref().unwrap().reached_start {
        let oldest = app.conv.as_ref().unwrap().oldest_real().unwrap();
        let r = backend.history(BIG_CHAT, Anchor::Before(oldest), PAGE_FOR_TESTS).await;
        let _ = app.update(Msg::History { peer: BIG_CHAT, older: true, result: r });
        guard += 1;
        assert!(guard < 200);
    }
    let msgs = &app.conv.as_ref().unwrap().messages;
    assert_eq!(msgs.len(), BIG_CHAT_LEN);
    assert!(msgs.windows(2).all(|w| w[0].id < w[1].id));
    // La ventana virtual solo construye una fracción de 5000 filas de ~60 px.
    let prefix: Vec<f32> = (0..=msgs.len()).map(|i| i as f32 * 60.0).collect();
    let w = conv::visible_window(&prefix, 800.0, 0.0, 800.0);
    assert!(w.end - w.start < 50, "{:?}", w);
}

#[tokio::test]
async fn enviar_optimista_confirmar_editar_reaccionar_borrar() {
    let (mut app, _, backend) = boot().await;
    let peer = PeerRef::User(UserId(1002));
    open(&mut app, &backend, peer).await;
    let before = app.conv.as_ref().unwrap().messages.len();

    app.composer = text_editor::Content::with_text("Hola desde la prueba");
    let _ = app.update(Msg::Send);
    let pending = app.conv.as_ref().unwrap().messages.last().unwrap().clone();
    assert!(conv::is_local(&pending));
    assert_eq!(pending.state, SendState::Pending);
    assert!(app.composer.text().trim().is_empty(), "el compositor se vacía");
    let rid = pending.random_id.unwrap();

    let out = OutgoingText { text: pending.text.clone(), entities: vec![], reply_to: None, random_id: rid };
    let r = backend.send_text(peer, out).await;
    let _ = app.update(Msg::Sent { peer, random_id: rid, result: r });
    let sent = app.conv.as_ref().unwrap().messages.last().unwrap().clone();
    assert!(!conv::is_local(&sent) && sent.state == SendState::Sent);
    assert_eq!(app.conv.as_ref().unwrap().messages.len(), before + 1);
    // El evento duplicado del backend no crea otra fila.
    let _ = app.update(Msg::Backend(BackendEvent::MessageSent { random_id: rid, msg: sent.clone() }));
    assert_eq!(app.conv.as_ref().unwrap().messages.len(), before + 1);

    // Editar: ↑ con el compositor vacío toma el último propio.
    let _ = app.update(Msg::EditLast);
    assert_eq!(app.compose, Compose::Edit(sent.id));
    assert_eq!(app.composer.text().trim(), "Hola desde la prueba");
    let edited = backend.edit_message(peer, sent.id, "Hola (editado)".into(), vec![]).await.unwrap();
    let _ = app.update(Msg::Backend(BackendEvent::MessageEdited(edited)));
    let _ = app.update(Msg::CancelCompose);
    assert_eq!(app.compose, Compose::Normal);
    assert!(app.conv.as_ref().unwrap().find(sent.id).unwrap().edit_date.is_some());

    // Reaccionar.
    backend.send_reaction(peer, sent.id, Some("🔥".into())).await.unwrap();
    let reactions = vec![ReactionCount { emoji: "🔥".into(), count: 1, mine: true }];
    let _ = app.update(Msg::Backend(BackendEvent::Reactions { peer, msg_id: sent.id, reactions }));
    assert!(app.conv.as_ref().unwrap().find(sent.id).unwrap().reactions[0].mine);

    // Responder.
    let _ = app.update(Msg::ReplyTo(sent.id));
    assert_eq!(app.compose, Compose::Reply(sent.id));
    let _ = app.update(Msg::Escape);
    assert_eq!(app.compose, Compose::Normal);

    // Borrar con diálogo.
    let _ = app.update(Msg::AskDelete(sent.id));
    assert!(matches!(app.overlay, Some(Overlay::Delete { revoke: true, .. })));
    let _ = app.update(Msg::SetRevoke(false));
    let _ = app.update(Msg::ConfirmDelete);
    assert!(app.overlay.is_none());
    backend.delete_messages(peer, vec![sent.id], false).await.unwrap();
    let _ = app.update(Msg::Backend(BackendEvent::MessagesDeleted { peer, ids: vec![sent.id] }));
    assert!(app.conv.as_ref().unwrap().find(sent.id).is_none());
}

#[tokio::test]
async fn fallo_de_envio_y_reintento() {
    let (mut app, mock, backend) = boot().await;
    let peer = PeerRef::User(UserId(1003));
    open(&mut app, &backend, peer).await;
    app.composer = text_editor::Content::with_text("sin red");
    let _ = app.update(Msg::Send);
    let rid = app.conv.as_ref().unwrap().messages.last().unwrap().random_id.unwrap();
    mock.set_offline(true);
    let out = OutgoingText { text: "sin red".into(), entities: vec![], reply_to: None, random_id: rid };
    let r = backend.send_text(peer, out.clone()).await;
    assert_eq!(r, Err(BackendError::Network));
    let _ = app.update(Msg::Sent { peer, random_id: rid, result: r });
    assert_eq!(app.conv.as_ref().unwrap().messages.last().unwrap().state, SendState::Failed);
    assert!(!app.toasts.is_empty(), "aviso de error");
    mock.set_offline(false);
    let _ = app.update(Msg::Retry(rid));
    assert_eq!(app.conv.as_ref().unwrap().messages.last().unwrap().state, SendState::Pending);
    let r = backend.send_text(peer, out).await;
    let _ = app.update(Msg::Sent { peer, random_id: rid, result: r });
    assert_eq!(app.conv.as_ref().unwrap().messages.last().unwrap().state, SendState::Sent);
}

#[tokio::test]
async fn chat_secreto_huella_y_canal_de_solo_lectura() {
    let (mut app, _, backend) = boot().await;
    let peer = PeerRef::Secret(SECRET_ID);
    open(&mut app, &backend, peer).await;
    assert!(app.write_block(peer).is_none(), "el secreto sembrado está aceptado");
    let _ = app.update(Msg::ShowKey);
    assert_eq!(app.overlay, Some(Overlay::Key(None)));
    let _ = app.update(Msg::KeyLoaded(backend.secret_key_visual(SECRET_ID).await));
    let Some(Overlay::Key(Some(kv))) = &app.overlay else { panic!("huella cargada") };
    let (cells, groups) = crate::format::key_grid(&kv.fingerprint);
    assert!(cells.iter().all(|c| *c < 8) && groups.len() == 4);
    let _ = app.update(Msg::Escape);
    assert!(app.overlay.is_none());

    let channel = app.dialogs.iter().find(|d| app.is_channel(d.peer)).unwrap().peer;
    assert!(app.write_block(channel).is_some());
}

#[tokio::test]
async fn tema_y_movimiento_conmutables() {
    let (mut app, _, _) = boot().await;
    assert_eq!(app.brasa.kind, ThemeKind::Dark);
    let _ = app.update(Msg::ToggleTheme);
    assert_eq!(app.brasa.kind, ThemeKind::Light);
    assert!(!app.anim.pref.is_reduced());
    let _ = app.update(Msg::ToggleMotion);
    assert!(app.anim.pref.is_reduced());
    // Con movimiento reducido, abrir un chat no anima posición (solo opacidad ≤ 100 ms).
    let _ = app.update(Msg::Open(BIG_CHAT));
    assert!(app.anim.is_active());
}
