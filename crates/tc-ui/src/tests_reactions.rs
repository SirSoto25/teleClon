//! Reacciones (solo las del chat, optimistas y revertidas con aviso claro) y stickers de la red en
//! el selector y en las burbujas. Como en el resto de pruebas, las `Task` no se ejecutan: la prueba
//! entrega los resultados como lo haría el runtime de iced.

use std::sync::Arc;

use iced::widget::image::Handle;
use tc_core::*;
use tc_mock::{MockBackend, MockConfig};
use tc_theme::ThemeKind;

use crate::app::{App, Backend, Flags, Msg, apply_mine, reaction_error_text};
use crate::media::{Decoded, ImageKey};
use crate::stickers::Tab;

const NOW: i64 = 1_762_430_400;

async fn boot() -> (App, Backend) {
    let mock = Arc::new(MockBackend::new(MockConfig { now: Some(NOW), ..Default::default() }));
    let backend: Backend = mock;
    let (mut app, _) = App::new(Flags { theme: ThemeKind::Dark, ..Flags::new(backend.clone()) });
    let _ = app.update(Msg::Me(backend.me().await));
    let _ = app.update(Msg::DialogsLoaded(backend.dialogs(DialogsPage { offset: 0, limit: 500, archived: false }).await));
    (app, backend)
}

fn own_note(id: i32, peer: PeerRef, me: UserId) -> Message {
    Message {
        id: MsgId(id),
        peer,
        sender: Some(me),
        date: Timestamp(NOW),
        edit_date: None,
        outgoing: true,
        text: "nota".into(),
        entities: vec![],
        reply_to: None,
        forwarded_from: None,
        media: None,
        sticker: None,
        reactions: vec![],
        random_id: None,
        state: SendState::Sent,
        ttl_secs: None,
    }
}

#[test]
fn my_reaction_moves_like_the_server_does() {
    let mut l = vec![ReactionCount { emoji: "👍".into(), count: 2, mine: false }];
    apply_mine(&mut l, Some("❤"));
    assert_eq!(l, vec![ReactionCount { emoji: "👍".into(), count: 2, mine: false }, ReactionCount { emoji: "❤".into(), count: 1, mine: true }]);
    apply_mine(&mut l, Some("👍"));
    assert_eq!(l, vec![ReactionCount { emoji: "👍".into(), count: 3, mine: true }]);
    apply_mine(&mut l, None);
    assert_eq!(l, vec![ReactionCount { emoji: "👍".into(), count: 2, mine: false }]);
}

/// El caso real: reaccionar a un mensaje propio en «Mensajes guardados» sin Premium. La reacción se
/// ve en el acto, la red responde `PREMIUM_ACCOUNT_REQUIRED` (→ `PremiumRequired`), se deshace y
/// sale un aviso claro; al volver a preguntar, el chat no ofrece reacciones.
#[tokio::test]
async fn premium_required_reverts_the_optimistic_reaction_with_a_clear_toast() {
    let (mut app, _) = boot().await;
    let me = app.me.as_ref().expect("cuenta").id;
    let saved = PeerRef::User(me);
    let _ = app.update(Msg::Open(saved));
    assert_eq!(app.reaction_choices(saved), None, "se piden al abrir el chat");
    let _ = app.update(Msg::History { peer: saved, older: false, result: Ok(vec![own_note(10, saved, me)]) });
    // Supongamos que se ofrecieron (p. ej. datos viejos o un fallo al pedirlas).
    let _ = app.update(Msg::ReactionsLoaded(saved, Err(BackendError::Network)));
    assert_eq!(app.reaction_choices(saved).map(<[String]>::len), Some(DEFAULT_REACTIONS.len()));

    let _ = app.update(Msg::React(MsgId(10), Some("👍".into())));
    let m = app.conv.as_ref().unwrap().find(MsgId(10)).unwrap();
    assert_eq!(m.reactions, vec![ReactionCount { emoji: "👍".into(), count: 1, mine: true }], "optimista");

    let _ = app.update(Msg::Reacted { peer: saved, id: MsgId(10), prev: vec![], result: Err(BackendError::PremiumRequired) });
    assert!(app.conv.as_ref().unwrap().find(MsgId(10)).unwrap().reactions.is_empty(), "revertida");
    let toast = app.toasts.last().expect("aviso");
    assert!(toast.error && toast.text.contains("Premium") && toast.text.contains("Mensajes guardados"), "{}", toast.text);
    assert!(!toast.text.contains("PREMIUM_ACCOUNT_REQUIRED"));
    assert_eq!(app.reaction_choices(saved), None, "se volverán a pedir");

    // Lo que dice ahora el backend real para «Mensajes guardados» sin Premium: ninguna.
    let _ = app.update(Msg::ReactionsLoaded(saved, Ok(vec![])));
    assert!(!app.can_react(saved));

    // Una reacción que sí llega no se toca.
    let _ = app.update(Msg::ReactionsLoaded(saved, Ok(vec!["🔥".into()])));
    let _ = app.update(Msg::React(MsgId(10), Some("🔥".into())));
    let _ = app.update(Msg::Reacted { peer: saved, id: MsgId(10), prev: vec![], result: Ok(()) });
    assert_eq!(app.conv.as_ref().unwrap().find(MsgId(10)).unwrap().reactions[0].emoji, "🔥");

    assert_eq!(reaction_error_text(&BackendError::ReactionUnavailable, false), "Esa reacción no está disponible en este chat.");
    assert_eq!(reaction_error_text(&BackendError::PremiumRequired, false), "Esa reacción necesita Telegram Premium.");
}

#[tokio::test]
async fn the_mock_offers_its_reactions_through_the_backend() {
    let (mut app, backend) = boot().await;
    let peer = app.dialogs.iter().find(|d| matches!(d.peer, PeerRef::User(_))).unwrap().peer;
    let _ = app.update(Msg::Open(peer));
    let _ = app.update(Msg::ReactionsLoaded(peer, backend.available_reactions(peer).await));
    assert_eq!(app.reaction_choices(peer).unwrap(), DEFAULT_REACTIONS.map(String::from));
    assert!(!app.reaction_choices(peer).unwrap().iter().any(|e| e == "❤️"), "la red usa «❤» sin U+FE0F");
}

fn net_sticker(id: i64, format: StickerFormat) -> Sticker {
    Sticker {
        id: StickerId(id),
        set_id: StickerSetId(77),
        emoji: "😺".into(),
        width: 512,
        height: 512,
        format,
        file: Some(FileRef { kind: FileKind::Document, id, access_hash: 1, file_reference: vec![1], dc_id: 2, size_bytes: 9_000, thumb: Some("m".into()), full: None }),
    }
}

fn decoded() -> Option<Decoded> {
    Some(Decoded { handle: Handle::from_rgba(1, 1, vec![0u8; 4]), width: 1, height: 1 })
}

#[tokio::test]
async fn account_stickers_load_in_the_picker_and_in_bubbles() {
    let (mut app, _) = boot().await;
    let peer = app.dialogs.iter().find(|d| matches!(d.peer, PeerRef::User(_))).unwrap().peer;
    let _ = app.update(Msg::Open(peer));
    let set = StickerSet { id: StickerSetId(77), short_name: "red".into(), title: "Gatos".into(), stickers: vec![net_sticker(1, StickerFormat::Webp), net_sticker(2, StickerFormat::Lottie)] };
    // La cuenta real no trae el pack propio: se añade como pack local.
    let _ = app.update(Msg::StickerSets(Ok(vec![set])));
    assert_eq!(app.sticker_sets.len(), 2);
    assert_eq!(app.sticker_sets[1].id, tc_stickers::pack::SET_ID);
    assert!(app.sticker_lists.local_pack);
    let _ = app.update(Msg::NetStickers { recent: Ok(vec![net_sticker(3, StickerFormat::Webm)]), faved: Ok(vec![net_sticker(1, StickerFormat::Webp)]) });
    assert_eq!(app.picker_tabs(), vec![Tab::Recent, Tab::Faved, Tab::Set(0), Tab::Set(1)]);
    assert!(app.find_sticker(StickerId(3)).is_some(), "los recientes del servidor se pueden enviar");

    // Selector abierto en el pack de la red: se piden las miniaturas (sin mensaje: MsgId(0)).
    // El panel recuerda la pestaña: se elige Stickers antes de abrirlo (si no, abre en Emojis).
    let _ = app.update(Msg::Panel(crate::panel::PanelMsg::Tab(crate::panel::PanelTab::Stickers)));
    app.images.clear(); // libera los huecos que ocupan los avatares de la lista
    let _ = app.update(Msg::TogglePicker);
    let _ = app.update(Msg::PickerTab(2));
    let thumb = ImageKey::Sticker { file: 1, full: false };
    assert!(app.images.is_loading(&thumb), "miniatura pedida");
    let s1 = app.sticker_sets[0].stickers[0].clone();
    assert!(app.sticker_image(&s1, 60.0).is_none(), "emoji hasta que llegue");
    let _ = app.update(Msg::ImageLoaded(thumb, decoded()));
    assert!(app.sticker_image(&s1, 60.0).is_some());
    // En grande (burbuja) se usa la miniatura mientras llega el archivo entero.
    assert!(app.sticker_image(&s1, crate::conv::STICKER).is_some());

    // Un sticker recibido en un mensaje pide su archivo entero para la burbuja.
    let mut m = own_note(900, peer, UserId(1));
    m.outgoing = false;
    m.text.clear();
    m.sticker = Some(net_sticker(5, StickerFormat::Webp));
    let _ = app.update(Msg::TogglePicker);
    let _ = app.update(Msg::Backend(BackendEvent::MessageNew(m)));
    let full = ImageKey::Sticker { file: 5, full: true };
    for k in [1i64, 2, 3] {
        let _ = app.update(Msg::ImageLoaded(ImageKey::Sticker { file: k, full: false }, None));
    }
    assert!(app.images.is_loading(&full), "archivo del sticker recibido pedido");
}
