//! Búsqueda dentro del chat (fase B2) contra el mock: espera al teclear, acentos, filtros,
//! navegación con contador, paginación, salto a un mensaje no cargado (`Anchor::Around`), borrado
//! de un resultado, tema de foro, chats secretos y `Esc` que restaura la posición. Como en el resto
//! de pruebas, las `Task` no se ejecutan: la prueba entrega los resultados como lo haría iced.

use std::sync::Arc;
use std::time::{Duration, Instant};

use iced::keyboard::{self, Key, Modifiers, key::Named};
use tc_core::*;
use tc_mock::{BIG_CHAT, FORUM_CHAT, FORUM_GROUP, MockBackend, MockConfig, SECRET_ID};
use tc_theme::ThemeKind;

use crate::app::find::{FIND_PAGE, FindMsg, FindState, highlight_ranges};
use crate::app::forum::ForumMsg;
use crate::app::{App, Backend, Flags, Msg};
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
    let _ = app.update(Msg::Secrets(backend.secret_chats().await));
    (app, mock)
}

/// Abre un chat con su primera página y medidas de una vista de 600 × 500.
async fn open(app: &mut App, peer: PeerRef) {
    let _ = app.update(Msg::Open(peer));
    let anchor = reading::initial_anchor(app.reading_dialog(peer).as_deref());
    let result = app.scoped_backend().unwrap().history(peer, anchor, PAGE).await;
    let _ = app.update(Msg::History { peer, older: false, result });
    measure(app);
}

fn measure(app: &App) {
    let c = app.conv.as_ref().unwrap();
    c.last_col.set(600.0);
    c.last_viewport.set(500.0);
}

fn find(m: FindMsg) -> Msg {
    Msg::Find(m)
}

/// Escribe en el campo y deja vencer la espera.
fn type_query(app: &mut App, text: &str) {
    let _ = app.update(find(FindMsg::Input(text.into())));
    let _ = app.update(find(FindMsg::Tick(Instant::now() + Duration::from_secs(2))));
}

/// Responde a la búsqueda en curso (primera página o la siguiente) por el backend que usa la UI.
async fn deliver(app: &mut App, append: bool) {
    let f = app.find_active().expect("barra abierta");
    let (seq, peer) = (f.seq(), f.peer);
    let mut q = f.query();
    if append {
        q = q.after(f.next.expect("hay página siguiente"));
    }
    let result = app.scoped_backend().unwrap().search_in_chat(peer, q).await;
    let _ = app.update(find(FindMsg::Results { seq, append, result }));
}

/// Entrega la ventana alrededor del resultado al que se está saltando.
async fn deliver_window(app: &mut App) {
    let f = app.find_active().unwrap();
    let (peer, target) = (f.peer, f.jumping.expect("salto en curso"));
    let result = app.scoped_backend().unwrap().history(peer, Anchor::Around(target), PAGE).await;
    let _ = app.update(find(FindMsg::Window { peer, target, result }));
}

fn key(k: Key, modifiers: Modifiers) -> Msg {
    Msg::Key(keyboard::Event::KeyPressed {
        key: k.clone(),
        modified_key: k,
        physical_key: keyboard::key::Physical::Unidentified(keyboard::key::NativeCode::Unidentified),
        location: keyboard::Location::Standard,
        modifiers,
        text: None,
        repeat: false,
    })
}

fn counter(app: &App) -> Option<String> {
    app.find_active().and_then(|f| f.counter())
}

#[test]
fn resaltado_sin_acentos_y_con_prefijo() {
    let t = "Reunión mañana en la CAFETERÍA, ¿vale?";
    let words = |q: &str| highlight_ranges(q, t).into_iter().map(|r| &t[r]).collect::<Vec<_>>();
    assert_eq!(words("cafeteria"), ["CAFETERÍA"]);
    assert_eq!(words("reunion MAÑ"), ["Reunión", "mañ"], "la última palabra resalta solo el prefijo");
    assert_eq!(words("cafe"), ["CAFE"]);
    assert!(words("   ").is_empty());
    assert!(words("union").is_empty(), "no resalta en medio de una palabra");
}

#[test]
fn con_varias_palabras_resalta_la_frase_y_no_cada_palabra_suelta() {
    let t = "Ut enim ad minim veniam. Lorem ut labore. Ut enim ad minim otra vez, ut y enim.";
    let words = |q: &str| highlight_ranges(q, t).into_iter().map(|r| &t[r]).collect::<Vec<_>>();
    assert_eq!(words("ut enim ad minim"), ["Ut", "enim", "ad", "minim", "Ut", "enim", "ad", "minim"], "solo la frase, dos veces");
    assert_eq!(words("enim ad mi"), ["enim", "ad", "mi", "enim", "ad", "mi"], "la última, como prefijo");
    // Sin la frase seguida se resaltan las palabras sueltas (el servidor las busca todas).
    assert_eq!(words("minim lorem"), ["minim", "Lorem", "minim"]);
}

#[tokio::test]
async fn espera_al_teclear_acentos_y_salto_al_mas_reciente() {
    let (mut app, mock) = boot().await;
    let m = mock.inject_incoming(BIG_CHAT, "Reunión en la CAFETERÍA a las cinco").unwrap();
    open(&mut app, BIG_CHAT).await;
    let _ = app.update(key(Key::Character("f".into()), Modifiers::COMMAND));
    let f = app.find_active().expect("Ctrl+F abre la barra");
    assert_eq!(f.state, FindState::Idle);
    assert!(f.thread.is_none());

    // Mientras se teclea no se pregunta: solo al vencer la espera.
    let _ = app.update(find(FindMsg::Input("cafeteria".into())));
    assert!(app.find_active().unwrap().debouncing());
    let _ = app.update(find(FindMsg::Tick(Instant::now())));
    assert_eq!(app.find_active().unwrap().state, FindState::Idle, "aún no venció la espera");
    let _ = app.update(find(FindMsg::Tick(Instant::now() + Duration::from_secs(1))));
    let f = app.find_active().unwrap();
    assert_eq!(f.state, FindState::Loading);
    assert!(!f.debouncing());
    deliver(&mut app, false).await;
    let f = app.find_active().unwrap();
    assert_eq!(f.hits, [m.id], "sin acentos ni mayúsculas");
    assert_eq!(f.current_id(), Some(m.id), "salta al más reciente");
    assert_eq!(counter(&app).as_deref(), Some("1 de 1"));
    assert_eq!(f.terms(), Some("cafeteria"));
    assert!(!f.has_older() && !f.has_newer());

    // Una respuesta de una consulta ya sustituida se descarta.
    let seq = f.seq();
    type_query(&mut app, "xyzqw");
    let stale = app.scoped_backend().unwrap().search_in_chat(BIG_CHAT, SearchQuery::new("cafeteria", FIND_PAGE)).await;
    let _ = app.update(find(FindMsg::Results { seq, append: false, result: stale }));
    assert_eq!(app.find_active().unwrap().state, FindState::Loading);
    deliver(&mut app, false).await;
    let f = app.find_active().unwrap();
    assert_eq!((f.state.clone(), f.hits.len(), f.counter()), (FindState::Ready, 0, None), "«Sin resultados»");

    // Sin red: error con copy propio y «Reintentar».
    mock.set_offline(true);
    type_query(&mut app, "cafeteri");
    deliver(&mut app, false).await;
    assert!(matches!(&app.find_active().unwrap().state, FindState::Failed(e) if e.contains("Sin conexión")));
    mock.set_offline(false);
    let _ = app.update(find(FindMsg::Retry));
    deliver(&mut app, false).await;
    assert_eq!(app.find_active().unwrap().hits, [m.id]);
}

#[tokio::test]
async fn filtro_por_tipo_sin_texto() {
    let (mut app, _mock) = boot().await;
    open(&mut app, BIG_CHAT).await;
    let _ = app.update(find(FindMsg::Open));
    // El filtro busca en el acto (sin espera) y sin texto lista todas las fotos.
    let _ = app.update(find(FindMsg::Filter(SearchFilter::Photos)));
    assert_eq!(app.find_active().unwrap().state, FindState::Loading);
    deliver(&mut app, false).await;
    let f = app.find_active().unwrap();
    assert!(!f.hits.is_empty(), "el chat largo tiene fotos");
    assert!(f.terms().is_none(), "sin texto no hay nada que resaltar");
    let b = app.scoped_backend().unwrap();
    for id in f.hits.iter().take(5) {
        let around = b.history(BIG_CHAT, Anchor::Around(*id), 3).await.unwrap();
        let m = around.iter().find(|m| m.id == *id).unwrap();
        assert_eq!(m.media.as_ref().map(|md| md.kind), Some(MediaKind::Photo));
    }
    // Alt+1 vuelve a «Todo»: sin texto no hay nada que buscar.
    let _ = app.update(key(Key::Character("1".into()), Modifiers::ALT));
    let f = app.find_active().unwrap();
    assert_eq!((f.filter, f.state.clone()), (SearchFilter::All, FindState::Idle));
}

#[tokio::test]
async fn navegacion_con_teclado_paginacion_y_salto_a_un_mensaje_no_cargado() {
    let (mut app, _mock) = boot().await;
    open(&mut app, BIG_CHAT).await;
    let window: Vec<MsgId> = app.conv.as_ref().unwrap().messages.iter().map(|m| m.id).collect();
    let _ = app.update(find(FindMsg::Open));
    type_query(&mut app, "panaderia");
    deliver(&mut app, false).await;
    let f = app.find_active().unwrap();
    let total = f.total.unwrap();
    assert!(total as usize > FIND_PAGE, "más de una página: {total}");
    assert_eq!(f.hits.len(), FIND_PAGE);
    assert!(f.hits.windows(2).all(|w| w[0] > w[1]), "más reciente primero");
    assert_eq!(counter(&app), Some(format!("1 de {total}")));

    // ↑ y Enter van al anterior; ↓ y Mayús+Enter al siguiente.
    let _ = app.update(key(Key::Named(Named::ArrowUp), Modifiers::empty()));
    assert_eq!(counter(&app), Some(format!("2 de {total}")));
    let _ = app.update(key(Key::Named(Named::Enter), Modifiers::empty()));
    assert_eq!(counter(&app), Some(format!("3 de {total}")));
    let _ = app.update(key(Key::Named(Named::Enter), Modifiers::SHIFT));
    let _ = app.update(key(Key::Named(Named::ArrowDown), Modifiers::empty()));
    assert_eq!(counter(&app), Some(format!("1 de {total}")));
    let _ = app.update(key(Key::Named(Named::ArrowDown), Modifiers::empty()));
    assert_eq!(counter(&app), Some(format!("1 de {total}")), "no pasa del más reciente");

    // Un resultado fuera de la ventana: se pide la de alrededor con `Anchor::Around`.
    let far = app.find_active().unwrap().hits.iter().position(|id| !window.contains(id)).expect("alguno no está cargado");
    for _ in 0..far {
        let _ = app.update(find(FindMsg::Older));
    }
    let target = app.find_active().unwrap().current_id().unwrap();
    assert_eq!(app.find_active().unwrap().jumping, Some(target));
    deliver_window(&mut app).await;
    let c = app.conv.as_ref().unwrap();
    assert!(c.find(target).is_some(), "la ventana nueva contiene el resultado");
    assert!(!c.reading.reached_end, "lejos del final: «ir al final» y paginación hacia lo nuevo");
    assert!(reading::show_jump(c));
    assert!(c.from_bottom > 0.0, "la vista se coloca en el resultado");
    let ids: Vec<i32> = c.messages.iter().map(|m| m.id.0).collect();
    assert!(ids.windows(2).all(|w| w[0] < w[1]));
    assert!(app.find_active().unwrap().jumping.is_none());

    // Al llegar al último cargado se pide la página siguiente y se avanza al llegar.
    let last = FIND_PAGE - 1;
    while app.find_active().unwrap().current != Some(last) {
        let _ = app.update(find(FindMsg::Older));
        if app.find_active().unwrap().jumping.is_some() {
            deliver_window(&mut app).await;
        }
    }
    assert!(app.find_active().unwrap().loading_more, "se adelanta la página siguiente");
    let _ = app.update(find(FindMsg::Older));
    assert_eq!(app.find_active().unwrap().current, Some(last), "espera a la página");
    deliver(&mut app, true).await;
    let f = app.find_active().unwrap();
    assert!(!f.loading_more);
    assert!(f.hits.len() > FIND_PAGE);
    assert_eq!(f.current, Some(FIND_PAGE), "avanza al primero de la página nueva");
    assert_eq!(counter(&app), Some(format!("{} de {total}", FIND_PAGE + 1)));
    let mut seen = f.hits.clone();
    seen.dedup();
    assert_eq!(seen.len(), f.hits.len(), "sin repetidos entre páginas");
}

#[tokio::test]
async fn esc_cierra_y_restaura_la_ventana_y_la_posicion() {
    let (mut app, _mock) = boot().await;
    open(&mut app, BIG_CHAT).await;
    app.conv.as_mut().unwrap().from_bottom = 123.0;
    let before: Vec<MsgId> = app.conv.as_ref().unwrap().messages.iter().map(|m| m.id).collect();
    let _ = app.update(find(FindMsg::Open));
    type_query(&mut app, "panaderia");
    deliver(&mut app, false).await;
    // Saltar a uno no cargado sustituye la ventana.
    while app.find_active().unwrap().jumping.is_none() {
        let _ = app.update(find(FindMsg::Older));
    }
    deliver_window(&mut app).await;
    assert_ne!(app.conv.as_ref().unwrap().messages.iter().map(|m| m.id).collect::<Vec<_>>(), before);

    let _ = app.update(Msg::Escape);
    assert!(app.find.is_none(), "Esc cierra la barra");
    let c = app.conv.as_ref().unwrap();
    assert_eq!(c.messages.iter().map(|m| m.id).collect::<Vec<_>>(), before, "vuelve la ventana de antes");
    assert_eq!(c.from_bottom, 123.0, "y la posición");
    assert!(c.reading.reached_end);

    // Un `Esc` capturado por el campo de la barra también la cierra.
    let _ = app.update(find(FindMsg::Open));
    assert!(app.find_active().is_some());
    let _ = app.update(find(FindMsg::EscCaptured));
    assert!(app.find.is_none());
    // Sin barra, un `Esc` capturado no hace nada.
    let _ = app.update(find(FindMsg::EscCaptured));
    assert!(app.conv.is_some());
}

#[tokio::test]
async fn un_resultado_borrado_sale_de_la_lista() {
    let (mut app, _mock) = boot().await;
    open(&mut app, BIG_CHAT).await;
    let _ = app.update(find(FindMsg::Open));
    type_query(&mut app, "panaderia");
    deliver(&mut app, false).await;
    for _ in 0..3 {
        let _ = app.update(find(FindMsg::Older));
    }
    let f = app.find_active().unwrap();
    let (total, hits, cur) = (f.total.unwrap(), f.hits.clone(), f.current_id().unwrap());
    assert_eq!(f.current, Some(3));
    // Se borra uno anterior al seleccionado: el seleccionado sigue siendo el mismo mensaje.
    let _ = app.update(Msg::Backend(BackendEvent::MessagesDeleted { peer: BIG_CHAT, ids: vec![hits[1]] }));
    let f = app.find_active().unwrap();
    assert!(!f.hits.contains(&hits[1]));
    assert_eq!(f.total, Some(total - 1));
    assert_eq!((f.current, f.current_id()), (Some(2), Some(cur)));
    assert_eq!(counter(&app), Some(format!("3 de {}", total - 1)));
    // Se borra el seleccionado: queda el siguiente más antiguo.
    let _ = app.update(Msg::Backend(BackendEvent::MessagesDeleted { peer: BIG_CHAT, ids: vec![cur] }));
    let f = app.find_active().unwrap();
    assert_eq!(f.current_id(), Some(hits[4]));
    // Otro chat no afecta.
    let _ = app.update(Msg::Backend(BackendEvent::MessagesDeleted { peer: FORUM_CHAT, ids: vec![hits[5]] }));
    assert!(app.find_active().unwrap().hits.contains(&hits[5]));
}

#[tokio::test]
async fn en_un_tema_de_foro_busca_solo_en_el_tema() {
    let (mut app, mock) = boot().await;
    let topics = mock.forum_topics(FORUM_GROUP, TopicsQuery::first(50)).await.unwrap().topics;
    let open_t: Vec<_> = topics.iter().filter(|t| !t.closed).collect();
    let (t1, t2) = (open_t[0].thread(), open_t[1].thread());
    let out = |t: &str, r: i64| OutgoingText { text: t.into(), entities: vec![], reply_to: None, random_id: RandomId(r) };
    let m1 = mock.send_topic_text(t1, out("pelota roja", 1)).await.unwrap();
    mock.send_topic_text(t2, out("pelota azul", 2)).await.unwrap();
    let _ = app.update(Msg::Open(FORUM_CHAT));
    let r = mock.forum_topics(FORUM_GROUP, TopicsQuery::first(crate::app::forum::TOPICS_PAGE)).await;
    let _ = app.update(Msg::Forum(ForumMsg::Loaded { group: FORUM_GROUP, append: false, query: None, result: r }));
    let _ = app.update(Msg::Forum(ForumMsg::OpenTopic(t1)));
    let anchor = reading::initial_anchor(app.reading_dialog(t1.peer()).as_deref());
    let result = app.scoped_backend().unwrap().history(t1.peer(), anchor, PAGE).await;
    let _ = app.update(Msg::History { peer: t1.peer(), older: false, result });
    measure(&app);

    let _ = app.update(find(FindMsg::Open));
    assert_eq!(app.find_active().unwrap().thread, Some(t1));
    type_query(&mut app, "pelota");
    deliver(&mut app, false).await;
    assert_eq!(app.find_active().unwrap().hits, [m1.id], "solo el mensaje de este tema");

    // Al cambiar de tema la barra no sigue abierta.
    let _ = app.update(Msg::Forum(ForumMsg::OpenTopic(t2)));
    assert!(app.find_active().is_none());
}

#[tokio::test]
async fn en_un_chat_secreto_no_se_busca() {
    let (mut app, _mock) = boot().await;
    let peer = PeerRef::Secret(SECRET_ID);
    let _ = app.update(Msg::Open(peer));
    assert!(!app.can_find(peer));
    let _ = app.update(key(Key::Character("f".into()), Modifiers::COMMAND));
    assert!(app.find.is_none());
    assert!(app.toasts.iter().any(|t| t.text.contains("secretos")), "se explica por qué");
}
