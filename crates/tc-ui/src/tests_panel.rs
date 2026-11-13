//! Pruebas de `update` del panel del compositor (Emojis | GIFs | Stickers) contra el `MockBackend`:
//! como en `tests.rs`, la prueba hace la llamada al backend y entrega el resultado como lo haría el
//! runtime de iced.

use std::sync::Arc;

use iced::widget::text_editor::{Action, Motion};
use tc_core::*;
use tc_mock::{BIG_CHAT, MockBackend, MockConfig};

use crate::app::{App, Backend, Flags, Msg};
use crate::emoji::{Category, Tone};
use crate::gifs::GifMsg;
use crate::panel::{PanelMsg, PanelTab};
use crate::stickers::Nav;

const NOW: i64 = 1_762_430_400;

fn pm(m: PanelMsg) -> Msg {
    Msg::Panel(m)
}

fn gm(m: GifMsg) -> Msg {
    Msg::Panel(PanelMsg::Gif(m))
}

async fn boot_in_chat() -> (App, Arc<MockBackend>, Backend) {
    let mock = Arc::new(MockBackend::new(MockConfig { now: Some(NOW), ..Default::default() }));
    let backend: Backend = mock.clone();
    let (mut app, _) = App::new(Flags::new(backend.clone()));
    let _ = app.update(Msg::Me(backend.me().await));
    let _ = app.update(Msg::DialogsLoaded(backend.dialogs(DialogsPage { offset: 0, limit: 500, archived: false }).await));
    let _ = app.update(Msg::StickerSets(backend.installed_sticker_sets().await));
    let _ = app.update(Msg::Open(BIG_CHAT));
    let r = backend.history(BIG_CHAT, Anchor::Latest, crate::conv::PAGE).await;
    let _ = app.update(Msg::History { peer: BIG_CHAT, older: false, result: r });
    (app, mock, backend)
}

#[tokio::test]
async fn pestanas_atajos_y_cierre() {
    let (mut app, _, _) = boot_in_chat().await;
    let _ = app.update(Msg::TogglePicker);
    assert!(app.picker.open);
    assert_eq!(app.panel.tab, PanelTab::Emoji, "primera vez: emojis");
    assert!(app.picker_tab_stickers().is_empty(), "sin stickers a la vista fuera de su pestaña");
    let _ = app.update(pm(PanelMsg::Tab(PanelTab::Gifs)));
    assert_eq!(app.panel.tab, PanelTab::Gifs);
    assert!(app.panel.gifs.saved_loading, "al entrar en GIFs se piden los guardados");
    // Esc cierra; Ctrl+E reabre en la última pestaña usada.
    let _ = app.update(Msg::Escape);
    assert!(!app.picker.open);
    let _ = app.update(Msg::TogglePicker);
    assert_eq!((app.picker.open, app.panel.tab), (true, PanelTab::Gifs));
    // Alt+3 → Stickers (atajo global con el compositor sin foco).
    let alt3 = iced::keyboard::Event::KeyPressed {
        key: iced::keyboard::Key::Character("3".into()),
        modified_key: iced::keyboard::Key::Character("3".into()),
        physical_key: iced::keyboard::key::Physical::Code(iced::keyboard::key::Code::Digit3),
        location: iced::keyboard::Location::Standard,
        modifiers: iced::keyboard::Modifiers::ALT,
        text: None,
        repeat: false,
    };
    let _ = app.update(Msg::Key(alt3));
    assert_eq!(app.panel.tab, PanelTab::Stickers);
    assert!(!app.picker_tab_stickers().is_empty());
    // Clic fuera (sobre la conversación) cierra.
    let _ = app.update(pm(PanelMsg::Close));
    assert!(!app.picker.open);
    assert_eq!(app.panel.tab, PanelTab::Stickers, "la pestaña se recuerda");
}

#[tokio::test]
async fn emoji_en_el_cursor_recientes_busqueda_y_tono() {
    let (mut app, _, backend) = boot_in_chat().await;
    app.composer = iced::widget::text_editor::Content::with_text("hola mundo");
    let _ = app.update(Msg::TogglePicker);
    let _ = app.update(pm(PanelMsg::Usage(backend.emoji_usage().await)));
    assert_eq!(app.panel.emoji.shown_category(), Category::Smileys, "sin recientes se ven las caras");

    // Cursor tras «hola»: el emoji se inserta ahí, no al final.
    let _ = app.update(Msg::Edit(Action::Move(Motion::DocumentStart)));
    for _ in 0..4 {
        let _ = app.update(Msg::Edit(Action::Move(Motion::Right)));
    }
    let _ = app.update(pm(PanelMsg::Insert("😀".into())));
    assert_eq!(app.composer.text().trim_end(), "hola😀 mundo");
    assert!(app.picker.open, "el panel sigue abierto tras insertar");
    let _ = app.update(pm(PanelMsg::Insert("😀".into())));
    let _ = app.update(pm(PanelMsg::Insert("🔥".into())));
    assert_eq!(app.composer.text().trim_end(), "hola😀😀🔥 mundo");
    assert_eq!(app.panel.emoji.usage.frequent(2), ["😀", "🔥"]);
    assert_eq!(app.panel.emoji.shown_category(), Category::Recent);

    // Búsqueda en español + Enter inserta el primero, con el tono elegido si lo admite.
    let _ = app.update(pm(PanelMsg::ToggleTones));
    let _ = app.update(pm(PanelMsg::Tone(Tone::Medium)));
    assert!(!app.panel.emoji.tones_open);
    let _ = app.update(pm(PanelMsg::EmojiQuery("pulgar arriba".into())));
    let _ = app.update(pm(PanelMsg::EmojiSubmit));
    assert!(app.composer.text().contains("👍🏽"), "{}", app.composer.text());

    // El uso se persiste en el backend (en la app, tc-store) y vuelve al recargar.
    for e in ["😀", "🔥"] {
        backend.record_emoji_use(e.into(), Timestamp(NOW)).await.unwrap();
    }
    let (mut fresh, _, _) = boot_in_chat().await;
    let _ = fresh.update(pm(PanelMsg::Usage(backend.emoji_usage().await)));
    assert_eq!(fresh.panel.emoji.usage.recent(5).len(), 2);
}

#[tokio::test]
async fn gifs_guardados_busqueda_envio_y_reintento() {
    let (mut app, mock, backend) = boot_in_chat().await;
    let before = app.conv.as_ref().unwrap().messages.len();
    let _ = app.update(pm(PanelMsg::Tab(PanelTab::Gifs)));
    let _ = app.update(Msg::TogglePicker);
    let _ = app.update(gm(GifMsg::Saved(backend.saved_gifs().await)));
    assert_eq!(app.panel.gifs.shown().len(), tc_mock::gifs::SAVED);
    // Pide las miniaturas del panel.
    assert!(!app.gif_requests().is_empty());

    // Filtrar lo guardado al escribir; Enter busca en la red.
    let _ = app.update(gm(GifMsg::Query("lluvia".into())));
    assert!(app.panel.gifs.shown().is_empty(), "«lluvia» no está entre los guardados");
    let _ = app.update(gm(GifMsg::Search));
    assert!(app.panel.gifs.results().is_some_and(|s| s.loading));
    let r = backend.search_gifs("lluvia".into(), String::new()).await;
    let _ = app.update(gm(GifMsg::Results { query: "lluvia".into(), offset: String::new(), result: r }));
    let shown: Vec<Gif> = app.panel.gifs.shown().into_iter().cloned().collect();
    assert_eq!(shown.len(), 1);
    assert!(shown[0].inline.is_some(), "resultado del bot");

    // Teclado: flecha coloca el cursor, Enter envía.
    let _ = app.update(Msg::PickerNav(Nav::Right));
    assert_eq!(app.panel.gifs.cursor.cursor, Some(0));
    let _ = app.update(Msg::PickerNav(Nav::Activate));
    assert!(!app.picker.open, "se cierra al enviar un GIF");
    let pending = app.conv.as_ref().unwrap().messages.last().unwrap().clone();
    assert_eq!(pending.state, SendState::Pending);
    assert_eq!(pending.media.as_ref().map(|m| (m.kind, m.file_name.clone())), Some((MediaKind::Gif, Some("lluvia.gif".into()))));
    let rid = pending.random_id.unwrap();

    // Falla sin red y se reintenta como GIF.
    mock.set_offline(true);
    let r = backend.send_gif(BIG_CHAT, shown[0].clone(), None, rid).await;
    let _ = app.update(Msg::Sent { peer: BIG_CHAT, random_id: rid, result: r });
    assert_eq!(app.conv.as_ref().unwrap().messages.last().unwrap().state, SendState::Failed);
    mock.set_offline(false);
    let _ = app.update(Msg::Retry(rid));
    assert_eq!(app.conv.as_ref().unwrap().messages.last().unwrap().state, SendState::Pending);
    let r = backend.send_gif(BIG_CHAT, shown[0].clone(), None, rid).await;
    let _ = app.update(Msg::Sent { peer: BIG_CHAT, random_id: rid, result: r });
    let last = app.conv.as_ref().unwrap().messages.last().unwrap();
    assert_eq!((last.state, last.media.as_ref().map(|m| m.kind)), (SendState::Sent, Some(MediaKind::Gif)));
    assert_eq!(app.conv.as_ref().unwrap().messages.len(), before + 1);
}

#[tokio::test]
async fn los_gif_reales_se_animan_con_el_reloj_compartido() {
    let (mut app, _, backend) = boot_in_chat().await;
    let _ = app.update(pm(PanelMsg::Tab(PanelTab::Gifs)));
    let _ = app.update(Msg::TogglePicker);
    let _ = app.update(gm(GifMsg::Saved(backend.saved_gifs().await)));
    let real = app.panel.gifs.shown().into_iter().find(|g| g.is_real_gif()).cloned().unwrap();
    let file = real.file.clone().unwrap();
    assert!(!app.gifs_animating(), "aún sin fotogramas");
    // Lo que haría `pump_gif_anims`: descargar y decodificar.
    let bytes = backend.download_media(BIG_CHAT, MsgId(0), file.clone(), MediaVariant::Full).await.unwrap();
    let frames = crate::gifs::decode(&bytes).map(Arc::new);
    assert!(frames.is_some());
    let _ = app.update(gm(GifMsg::AnimLoaded(file.id, frames)));
    assert!(app.gifs_animating());
    assert!(app.gif_frame(file.id).is_some());
    // Un MP4 no se anima (miniatura con insignia).
    let mp4 = app.panel.gifs.shown().into_iter().find(|g| !g.is_real_gif()).unwrap().file.clone().unwrap();
    assert!(app.gif_frame(mp4.id).is_none());
    // Movimiento reducido: primer fotograma y sin reloj.
    let _ = app.update(Msg::ToggleMotion);
    assert!(!app.gifs_animating());
    assert!(app.gif_frame(file.id).is_some());
    // Al cerrar el panel tampoco.
    let _ = app.update(Msg::ToggleMotion);
    let _ = app.update(Msg::Escape);
    assert!(!app.gifs_animating());
}
