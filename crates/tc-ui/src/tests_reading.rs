//! Apertura en el primer no leído, paginación hacia lo nuevo, «ir al final» con contador y
//! marcado como leído progresivo. Como en el resto de pruebas, las `Task` no se ejecutan: la
//! prueba entrega los resultados como lo haría el runtime de iced.

use std::sync::Arc;
use std::time::{Duration, Instant};

use tc_core::*;
use tc_mock::{BIG_CHAT, MockBackend, MockConfig, UNREAD_CHAT};
use tc_theme::ThemeKind;
use tc_theme::tokens::space;

use crate::app::{App, Backend, Flags, Msg};
use crate::conv::{Item, PAGE};
use crate::reading::{self, ReadMsg};

const NOW: i64 = 1_762_430_400;

async fn boot() -> (App, Backend) {
    let mock = Arc::new(MockBackend::new(MockConfig { now: Some(NOW), ..Default::default() }));
    let backend: Backend = mock;
    let (mut app, _) = App::new(Flags { theme: ThemeKind::Dark, ..Flags::new(backend.clone()) });
    let _ = app.update(Msg::Me(backend.me().await));
    let _ = app.update(Msg::DialogsLoaded(backend.dialogs(DialogsPage { offset: 0, limit: 500, archived: false }).await));
    (app, backend)
}

/// Abre el chat y entrega la primera página con el ancla que eligió la UI.
async fn open(app: &mut App, backend: &Backend, peer: PeerRef) -> Anchor {
    let _ = app.update(Msg::Open(peer));
    let anchor = reading::initial_anchor(app.dialog(peer));
    let result = backend.history(peer, anchor, PAGE).await;
    let _ = app.update(Msg::History { peer, older: false, result });
    anchor
}

fn ids(app: &App) -> Vec<i32> {
    app.conv.as_ref().unwrap().messages.iter().map(|m| m.id.0).collect()
}

#[tokio::test]
async fn abre_en_el_primer_no_leido_con_contexto_y_sin_marcar_todo() {
    let (mut app, backend) = boot().await;
    let d = app.dialog(UNREAD_CHAT).cloned().expect("el mock trae el grupo con no leídos");
    assert!(d.unread_count > 40, "muchos no leídos: {}", d.unread_count);
    let read = d.read_inbox_max_id;
    assert!(read.0 > 0);

    let anchor = open(&mut app, &backend, UNREAD_CHAT).await;
    assert_eq!(anchor, Anchor::Around(read), "no se pide el final del chat");
    let c = app.conv.as_ref().unwrap();
    assert_eq!(c.reading.opened_at, Some(read));
    let first = c.unread_from.expect("hay marca «Sin leer desde aquí»");
    assert!(first > read);
    assert!(c.messages.iter().all(|m| m.outgoing || m.id <= read || m.id >= first), "la marca va en el primer entrante no leído");
    // Contexto ya leído por encima y no se cargó todo el historial.
    assert!(c.messages.iter().filter(|m| m.id <= read).count() >= 10);
    assert!(c.messages.len() <= PAGE);
    assert!(!c.reading.reached_end, "con tantos no leídos la ventana no llega al final");
    assert!(!c.reached_start);
    assert!(reading::show_jump(c), "«ir al final» visible");
    assert_eq!(app.jump_badge(c), d.unread_count, "con el contador de no leídos");
    // Nada marcado al abrir.
    assert_eq!(app.dialog(UNREAD_CHAT).unwrap().unread_count, d.unread_count);
    assert_eq!(c.reading.sent, MsgId(0));

    // Aún sin maqueta el salto espera; con medidas se coloca la marca arriba con contexto.
    assert!(c.reading.jump_pending);
    let (col, vh) = (600.0, 500.0);
    c.last_col.set(col);
    c.last_viewport.set(vh);
    let _ = app.resolve_unread_jump();
    let c = app.conv.as_ref().unwrap();
    assert!(!c.reading.jump_pending);
    let (items, _) = crate::view::conversation::build_items(&app, c, col);
    let acc: f32 = items.iter().map(Item::height).sum();
    let mark = items.iter().position(|i| matches!(i, Item::UnreadMark)).unwrap();
    let mark_top: f32 = items[..mark].iter().map(Item::height).sum();
    let view_top = acc + space::S2 - vh - c.from_bottom;
    assert!(c.from_bottom > 0.0, "no se queda al final");
    assert!((mark_top - view_top - reading::UNREAD_CONTEXT).abs() < 1.0, "marca a {} px del borde", mark_top - view_top);
}

#[tokio::test]
async fn lectura_progresiva_con_espera() {
    let (mut app, backend) = boot().await;
    open(&mut app, &backend, UNREAD_CHAT).await;
    let before = app.dialog(UNREAD_CHAT).unwrap().clone();
    // Mientras falta colocar la vista en la marca no se marca nada.
    let c = app.conv.as_ref().unwrap();
    c.reading.seen.set(c.newest_real());
    let _ = app.update(Msg::Reading(ReadMsg::Flush(Instant::now() + Duration::from_secs(1))));
    assert_eq!(app.dialog(UNREAD_CHAT).unwrap().unread_count, before.unread_count);
    let c = app.conv.as_ref().unwrap();
    c.last_col.set(600.0);
    c.last_viewport.set(500.0);
    let _ = app.resolve_unread_jump();
    let c = app.conv.as_ref().unwrap();
    let first = c.unread_from.unwrap();
    // Lo visible llega unos mensajes por debajo de la marca.
    let seen = c.messages.iter().filter(|m| m.id >= first && !m.outgoing).nth(4).unwrap().id;
    c.reading.seen.set(Some(seen));
    let _ = app.update(Msg::Reading(ReadMsg::Flush(Instant::now())));
    assert_eq!(app.dialog(UNREAD_CHAT).unwrap().unread_count, before.unread_count, "antes de la espera no se marca");

    let _ = app.update(Msg::Reading(ReadMsg::Flush(Instant::now() + Duration::from_secs(1))));
    let d = app.dialog(UNREAD_CHAT).unwrap();
    assert_eq!(d.read_inbox_max_id, seen, "se marca hasta lo visto");
    assert_eq!(d.unread_count, before.unread_count - 5, "solo lo visto deja de contar");
    assert_eq!(app.conv.as_ref().unwrap().reading.sent, seen);
    assert!(!app.read_flush_due());

    // La red responde con el diálogo (el mock recalcula igual) y el contador se mantiene.
    backend.mark_read(UNREAD_CHAT, seen).await.unwrap();
    let net = backend.dialogs(DialogsPage { offset: 0, limit: 500, archived: false }).await.unwrap();
    let net = net.into_iter().find(|x| x.peer == UNREAD_CHAT).unwrap();
    assert_eq!(net.unread_count, before.unread_count - 5);
    assert_eq!(net.read_inbox_max_id, seen);

    // Volver a ver lo mismo no repite la petición.
    let c = app.conv.as_mut().unwrap();
    c.reading.dirty = Some(Instant::now());
    let sent = c.reading.sent;
    let _ = app.update(Msg::Reading(ReadMsg::Flush(Instant::now() + Duration::from_secs(1))));
    assert_eq!(app.conv.as_ref().unwrap().reading.sent, sent);
}

#[tokio::test]
async fn pagina_hacia_lo_nuevo_sin_duplicados_y_llega_al_final() {
    let (mut app, backend) = boot().await;
    open(&mut app, &backend, UNREAD_CHAT).await;
    let last = app.dialog(UNREAD_CHAT).unwrap().last_message.as_ref().unwrap().id;
    // Mientras no se ha colocado la marca no se pagina; con medidas se coloca.
    let _ = app.reading_on_scroll(500.0);
    assert!(!app.conv.as_ref().unwrap().reading.loading_newer);
    let c = app.conv.as_ref().unwrap();
    c.last_col.set(600.0);
    c.last_viewport.set(500.0);
    let _ = app.resolve_unread_jump();
    let mut rounds = 0;
    while !app.conv.as_ref().unwrap().reading.reached_end {
        rounds += 1;
        assert!(rounds < 10);
        // Cerca del final de la ventana: se pide la página siguiente.
        let c = app.conv.as_mut().unwrap();
        c.from_bottom = 0.0;
        let _ = app.reading_on_scroll(500.0);
        let c = app.conv.as_ref().unwrap();
        assert!(c.reading.loading_newer);
        let newest = c.newest_real().unwrap();
        // Una petición a la vez.
        let _ = app.reading_on_scroll(500.0);
        let result = backend.history(UNREAD_CHAT, Anchor::After(newest), PAGE).await;
        // Página con solape: no debe duplicar.
        let result = result.map(|mut p| {
            p.insert(0, app.conv.as_ref().unwrap().messages.last().unwrap().clone());
            p
        });
        let _ = app.update(Msg::Reading(ReadMsg::Newer { peer: UNREAD_CHAT, result }));
    }
    let v = ids(&app);
    assert!(v.windows(2).all(|w| w[0] < w[1]), "ascendente y sin duplicados");
    assert_eq!(*v.last().unwrap(), last.0);
    assert!(!app.conv.as_ref().unwrap().reading.loading_newer);
}

#[tokio::test]
async fn ir_al_final_carga_los_ultimos_y_lo_nuevo_espera_fuera_de_la_ventana() {
    let (mut app, backend) = boot().await;
    open(&mut app, &backend, UNREAD_CHAT).await;
    let before = ids(&app);
    // Llega un mensaje con la ventana lejos del final: no se mete en ella (dejaría un hueco).
    let mut m = app.conv.as_ref().unwrap().messages.last().unwrap().clone();
    m.id = MsgId(1_000_000);
    m.outgoing = false;
    let _ = app.update(Msg::Backend(BackendEvent::MessageNew(m)));
    assert_eq!(ids(&app), before);
    assert_eq!(app.conv.as_ref().unwrap().new_below, 1);

    let _ = app.update(Msg::JumpToBottom);
    assert!(app.conv.as_ref().unwrap().reading.loading_newer, "pide los últimos");
    let result = backend.history(UNREAD_CHAT, Anchor::Latest, PAGE).await;
    let _ = app.update(Msg::Reading(ReadMsg::Latest { peer: UNREAD_CHAT, result }));
    let c = app.conv.as_ref().unwrap();
    assert!(c.reading.reached_end);
    assert_eq!((c.from_bottom, c.new_below), (0.0, 0));
    let last = app.dialog(UNREAD_CHAT).unwrap().last_message.as_ref().unwrap().id;
    assert_eq!(c.newest_real(), Some(last));
    assert!(!reading::show_jump(c));
    // Al final se marca todo como leído.
    assert_eq!(app.dialog(UNREAD_CHAT).unwrap().unread_count, 0);
}

#[tokio::test]
async fn sin_no_leidos_abre_al_final() {
    let (mut app, backend) = boot().await;
    assert_eq!(app.dialog(BIG_CHAT).unwrap().unread_count, 0);
    let anchor = open(&mut app, &backend, BIG_CHAT).await;
    assert_eq!(anchor, Anchor::Latest);
    let c = app.conv.as_ref().unwrap();
    assert!(c.unread_from.is_none());
    assert!(c.reading.reached_end);
    assert!(!c.reading.jump_pending);
    assert!(!reading::show_jump(c));
}
