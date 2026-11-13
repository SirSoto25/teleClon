//! Grupos con temas contra el mock: abrir el foro → lista de temas, abrir un tema (en el primer no
//! leído), volver, enviar dentro del tema, no leídos por tema y filtro de eventos. Como en el resto
//! de pruebas, las `Task` no se ejecutan: la prueba entrega los resultados como lo haría iced.

use std::sync::Arc;

use futures::{FutureExt, StreamExt};
use tc_core::*;
use tc_mock::{BIG_CHAT, FORUM_CHAT, FORUM_GROUP, MockBackend, MockConfig};
use tc_theme::ThemeKind;

use crate::app::forum::ForumMsg;
use crate::app::{App, Backend, Flags, LoadState, Msg};
use crate::conv::PAGE;
use crate::reading;

const NOW: i64 = 1_762_430_400;

async fn boot() -> (App, Arc<MockBackend>) {
    let mock = Arc::new(MockBackend::new(MockConfig { now: Some(NOW), ..Default::default() }));
    let backend: Backend = mock.clone();
    let (mut app, _) = App::new(Flags { theme: ThemeKind::Dark, ..Flags::new(backend.clone()) });
    let _ = app.update(Msg::Me(backend.me().await));
    let _ = app.update(Msg::DialogsLoaded(backend.dialogs(DialogsPage { offset: 0, limit: 500, archived: false }).await));
    let _ = app.update(Msg::GroupLoaded(backend.group(FORUM_GROUP).await));
    (app, mock)
}

/// Abre el foro y entrega la primera página de temas.
async fn open_forum(app: &mut App, mock: &MockBackend) {
    let _ = app.update(Msg::Open(FORUM_CHAT));
    let result = mock.forum_topics(FORUM_GROUP, TopicsQuery::first(crate::app::forum::TOPICS_PAGE)).await;
    let _ = app.update(Msg::Forum(ForumMsg::Loaded { group: FORUM_GROUP, append: false, query: None, result }));
}

/// Abre un tema y entrega su primera página por el backend que usa la UI (atado al tema).
async fn open_topic(app: &mut App, th: ThreadRef) -> Anchor {
    let _ = app.update(Msg::Forum(ForumMsg::OpenTopic(th)));
    let anchor = reading::initial_anchor(app.reading_dialog(th.peer()).as_deref());
    let b = app.scoped_backend().expect("backend");
    let result = b.history(th.peer(), anchor, PAGE).await;
    let _ = app.update(Msg::History { peer: th.peer(), older: false, result });
    anchor
}

/// Entrega a la UI los eventos pendientes del backend.
fn pump(app: &mut App, ev: &mut futures::stream::BoxStream<'static, BackendEvent>) -> Vec<BackendEvent> {
    let mut seen = Vec::new();
    while let Some(Some(e)) = ev.next().now_or_never() {
        seen.push(e.clone());
        let _ = app.update(Msg::Backend(e));
    }
    seen
}

fn busy(app: &App) -> ForumTopic {
    app.forum.topics[&FORUM_GROUP].items.iter().max_by_key(|t| t.unread_count).cloned().unwrap()
}

#[tokio::test]
async fn abrir_un_foro_muestra_la_lista_de_temas() {
    let (mut app, mock) = boot().await;
    assert!(app.is_forum(FORUM_CHAT));
    let _ = app.update(Msg::Open(FORUM_CHAT));
    assert!(app.conv.is_none(), "no se abre como un chat plano");
    assert_eq!(app.topics_shown(), Some(FORUM_GROUP));
    assert_eq!(app.forum.topics[&FORUM_GROUP].state, LoadState::Loading);
    assert!(app.forum_selected(FORUM_CHAT));
    open_forum(&mut app, &mock).await;
    let list = &app.forum.topics[&FORUM_GROUP];
    assert_eq!(list.state, LoadState::Ready);
    assert_eq!(list.items.len(), 6);
    assert!(list.items[0].pinned, "fijados primero");
    assert!(list.items.iter().any(|t| t.is_general()));
    // La fila del foro en la lista de chats suma los no leídos de sus temas.
    let sum: u32 = list.items.iter().map(|t| t.unread_count).sum();
    let d = app.dialog(FORUM_CHAT).unwrap().clone();
    assert_eq!(app.dialog_unread(&d), sum);
    assert!(sum > 0);
    // Búsqueda de temas por nombre (la lista está entera: filtro local).
    let _ = app.update(Msg::Forum(ForumMsg::Query("riego".into())));
    assert_eq!(app.visible_topics(FORUM_GROUP).iter().map(|t| t.title.as_str()).collect::<Vec<_>>(), ["Riego automático"]);
    let _ = app.update(Msg::Forum(ForumMsg::ClearQuery));
    assert_eq!(app.visible_topics(FORUM_GROUP).len(), 6);
    // Otro chat cierra la lista del foro.
    let _ = app.update(Msg::Open(BIG_CHAT));
    assert_eq!(app.topics_shown(), None);
    assert_eq!(app.forum.list, None);
    assert_eq!(app.conv.as_ref().map(|c| c.peer), Some(BIG_CHAT));
}

#[tokio::test]
async fn abrir_un_tema_en_su_primer_no_leido_y_volver() {
    let (mut app, mock) = boot().await;
    open_forum(&mut app, &mock).await;
    let t = busy(&app);
    assert!(t.unread_count > 5);
    let anchor = open_topic(&mut app, t.thread()).await;
    assert_eq!(anchor, Anchor::Around(t.read_inbox_max_id), "abre en el primer no leído del tema");
    assert_eq!(app.open_thread(), Some(t.thread()));
    let c = app.conv.as_ref().unwrap();
    let all = mock.topic_history(t.thread(), Anchor::Latest, 500).await.unwrap();
    let topic_ids: Vec<MsgId> = all.iter().map(|m| m.id).collect();
    assert!(!c.messages.is_empty());
    assert!(c.messages.iter().all(|m| topic_ids.contains(&m.id)), "solo mensajes del tema");
    assert!(c.unread_from.is_some_and(|f| f > t.read_inbox_max_id));
    assert_eq!(app.jump_badge(c), t.unread_count, "el contador es el del tema");
    // El grupo no se marca con la lectura del tema.
    let before = app.dialog(FORUM_CHAT).unwrap().unread_count;
    let _ = app.update(Msg::Forum(ForumMsg::BackToTopics));
    assert!(app.conv.is_none());
    assert_eq!(app.topics_shown(), Some(FORUM_GROUP));
    assert_eq!(app.open_thread(), None);
    assert_eq!(app.dialog(FORUM_CHAT).unwrap().unread_count, before);
    // Volver a pulsar el foro en la lista de chats desde un tema también vuelve a los temas.
    open_topic(&mut app, t.thread()).await;
    let _ = app.update(Msg::Open(FORUM_CHAT));
    assert!(app.conv.is_none() && app.topics_shown() == Some(FORUM_GROUP));
}

#[tokio::test]
async fn enviar_dentro_del_tema_y_eventos_de_otros_temas() {
    let (mut app, mock) = boot().await;
    let mut ev = mock.events();
    open_forum(&mut app, &mock).await;
    let t = app.forum.topics[&FORUM_GROUP].items.iter().find(|t| !t.closed && !t.is_general()).cloned().unwrap();
    open_topic(&mut app, t.thread()).await;
    let _ = app.update(Msg::JumpToBottom);
    app.composer = iced::widget::text_editor::Content::with_text("¿Alguien riega mañana?");
    let _ = app.update(Msg::Send);
    let pending = app.conv.as_ref().unwrap().messages.last().cloned().unwrap();
    assert_eq!(pending.state, SendState::Pending);
    // La tarea de envío usa el backend atado al tema.
    let out = OutgoingText { text: pending.text.clone(), entities: vec![], reply_to: None, random_id: pending.random_id.unwrap() };
    let result = app.scoped_backend().unwrap().send_text(FORUM_CHAT, out).await;
    let sent = result.clone().unwrap();
    let _ = app.update(Msg::Sent { peer: FORUM_CHAT, random_id: pending.random_id.unwrap(), result });
    let in_topic = mock.topic_history(t.thread(), Anchor::Latest, 5).await.unwrap();
    assert_eq!(in_topic.last().map(|m| m.id), Some(sent.id), "llega al tema");
    let general = mock.topic_history(ThreadRef { group: FORUM_GROUP, topic: GENERAL_TOPIC }, Anchor::Latest, 500).await.unwrap();
    assert!(general.iter().all(|m| m.id != sent.id), "no al General");
    pump(&mut app, &mut ev);
    let c = app.conv.as_ref().unwrap();
    assert_eq!(c.messages.iter().filter(|m| m.id == sent.id).count(), 1, "sin duplicar");
    assert_eq!(app.topic(t.thread()).unwrap().last_message.as_ref().map(|m| m.id), Some(sent.id));

    // Un entrante en otro tema no entra en la conversación, pero sube los no leídos de ese tema.
    let general_th = ThreadRef { group: FORUM_GROUP, topic: GENERAL_TOPIC };
    let before = app.topic(general_th).unwrap().unread_count;
    let n = app.conv.as_ref().unwrap().messages.len();
    let other = mock.inject_topic_incoming(general_th, "en el general").unwrap();
    let seen = pump(&mut app, &mut ev);
    assert!(seen.iter().any(|e| matches!(e, BackendEvent::TopicMessageNew { topic, .. } if *topic == GENERAL_TOPIC)));
    assert_eq!(app.conv.as_ref().unwrap().messages.len(), n);
    assert!(app.conv.as_ref().unwrap().find(other.id).is_none());
    assert_eq!(app.topic(general_th).unwrap().unread_count, before + 1);
    // Uno en el tema abierto sí entra.
    let mine = mock.inject_topic_incoming(t.thread(), "yo también").unwrap();
    pump(&mut app, &mut ev);
    assert!(app.conv.as_ref().unwrap().find(mine.id).is_some());
}

#[tokio::test]
async fn marcar_leido_desde_el_menu_y_tema_cerrado() {
    let (mut app, mock) = boot().await;
    let mut ev = mock.events();
    open_forum(&mut app, &mock).await;
    let t = busy(&app);
    let _ = app.update(Msg::Forum(ForumMsg::OpenMenu(t.thread())));
    assert!(app.forum.menu.is_some());
    let _ = app.update(Msg::Forum(ForumMsg::MarkRead(t.thread())));
    assert!(app.forum.menu.is_none());
    assert_eq!(app.topic(t.thread()).unwrap().unread_count, 0, "optimista");
    // Lo que haría la tarea: la red confirma y el evento lo mantiene a 0.
    let max = t.last_message.as_ref().unwrap().id;
    app.scoped_backend().unwrap().mark_topic_read(t.thread(), max).await.unwrap();
    pump(&mut app, &mut ev);
    assert_eq!(app.topic(t.thread()).unwrap().unread_count, 0);
    let d = app.dialog(FORUM_CHAT).unwrap().clone();
    assert_eq!(app.dialog_unread(&d), app.forum.topics[&FORUM_GROUP].items.iter().map(|t| t.unread_count).sum::<u32>());

    // El tema cerrado no deja escribir.
    let closed = app.forum.topics[&FORUM_GROUP].items.iter().find(|t| t.closed).cloned().unwrap();
    open_topic(&mut app, closed.thread()).await;
    assert!(app.write_block(FORUM_CHAT).is_some_and(|r| r.contains("cerrado")));
}

#[tokio::test]
async fn crear_un_tema_lo_abre() {
    let (mut app, mock) = boot().await;
    open_forum(&mut app, &mock).await;
    let _ = app.update(Msg::Forum(ForumMsg::StartNew));
    let _ = app.update(Msg::Forum(ForumMsg::NewTitle("Compostaje".into())));
    let _ = app.update(Msg::Forum(ForumMsg::Create));
    assert!(app.forum.creating);
    let r = mock.create_topic(FORUM_GROUP, "Compostaje".into(), None, RandomId(5)).await;
    let id = r.as_ref().unwrap().id;
    let _ = app.update(Msg::Forum(ForumMsg::Created(Box::new(r))));
    assert!(!app.forum.creating && app.forum.new_title.is_none());
    assert_eq!(app.open_thread(), Some(ThreadRef { group: FORUM_GROUP, topic: id }));
}
