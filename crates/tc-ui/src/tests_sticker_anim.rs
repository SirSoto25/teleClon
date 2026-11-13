//! Stickers TGS animados en la app (ADR-0008): reloj solo con algo visible, foco y movimiento
//! reducido, y el fotograma animado que llega a la vista. Las `Task` no se ejecutan: la prueba
//! entrega las cargas y los lotes como lo haría el runtime de iced.

use std::sync::Arc;
use std::time::{Duration, Instant};

use tc_core::*;
use tc_mock::{MockBackend, MockConfig};
use tc_stickers::anim::Tgs;
use tc_theme::ThemeKind;

use crate::app::{App, Flags, Msg};
use crate::conv::PAGE;
use crate::sticker_anim::{self, AnimMsg};

const NOW: i64 = 1_762_430_400;

/// Lottie mínimo animado (cuadrado que se desplaza; 60 fotogramas a 60 fps).
const MOVING: &str = r#"{"v":"5.5.2","fr":60,"ip":0,"op":60,"w":64,"h":64,"nm":"m","ddd":0,"assets":[],"layers":[{"ddd":0,"ind":1,"ty":4,"nm":"s","sr":1,"ks":{"o":{"a":0,"k":100},"r":{"a":0,"k":0},"p":{"a":1,"k":[{"t":0,"s":[16,32,0],"e":[48,32,0],"i":{"x":[1],"y":[1]},"o":{"x":[0],"y":[0]}},{"t":59}]},"a":{"a":0,"k":[0,0,0]},"s":{"a":0,"k":[100,100,100]}},"ao":0,"shapes":[{"ty":"gr","it":[{"ty":"rc","d":1,"s":{"a":0,"k":[20,20]},"p":{"a":0,"k":[0,0]},"r":{"a":0,"k":0}},{"ty":"fl","c":{"a":0,"k":[1,0,0,1]},"o":{"a":0,"k":100},"r":1},{"ty":"tr","p":{"a":0,"k":[0,0]},"a":{"a":0,"k":[0,0]},"s":{"a":0,"k":[100,100]},"r":{"a":0,"k":0},"o":{"a":0,"k":100}}]}],"ip":0,"op":60,"st":0,"bm":0}]}"#;

const FILE: i64 = 4242;

fn tgs_sticker() -> Sticker {
    Sticker {
        id: StickerId(FILE),
        set_id: StickerSetId(77),
        emoji: "😺".into(),
        width: 512,
        height: 512,
        format: StickerFormat::Lottie,
        file: Some(FileRef { kind: FileKind::Document, id: FILE, access_hash: 1, file_reference: vec![1], dc_id: 2, size_bytes: 9_000, thumb: Some("m".into()), full: None }),
    }
}

/// App con un chat abierto cuyo último mensaje es un TGS en la ventana visible.
async fn app_with_tgs_bubble() -> App {
    let mock = Arc::new(MockBackend::new(MockConfig { now: Some(NOW), ..Default::default() }));
    let backend: crate::app::Backend = mock;
    let (mut app, _) = App::new(Flags { theme: ThemeKind::Dark, ..Flags::new(backend.clone()) });
    let _ = app.update(Msg::Me(backend.me().await));
    let _ = app.update(Msg::DialogsLoaded(backend.dialogs(DialogsPage { offset: 0, limit: 500, archived: false }).await));
    let peer = app.dialogs.iter().find(|d| matches!(d.peer, PeerRef::User(_))).unwrap().peer;
    let mut page = backend.history(peer, Anchor::Latest, PAGE).await.unwrap();
    let mut m = page.last().cloned().expect("historial");
    m.id = MsgId(m.id.0 + 1000);
    m.text.clear();
    m.media = None;
    m.sticker = Some(tgs_sticker());
    page.push(m);
    let _ = app.update(Msg::Open(peer));
    let _ = app.update(Msg::History { peer, older: false, result: Ok(page) });
    // La vista marca qué mensajes están en pantalla: el último.
    let cv = app.conv.as_ref().unwrap();
    let n = cv.messages.len();
    cv.visible.set(Some((n - 1, n)));
    cv.anim_visible.set(false);
    app
}

#[tokio::test]
async fn el_reloj_solo_corre_con_un_tgs_visible_con_foco_y_sin_movimiento_reducido() {
    let mut app = app_with_tgs_bubble().await;
    let lottie = tc_stickers::tgs::lottie_enabled();
    assert_eq!(app.stickers_animating(), lottie, "TGS visible: reloj de 30 fps (si hay rlottie)");

    let _ = app.update(Msg::StickerAnim(AnimMsg::Focus(false)));
    assert!(!app.stickers_animating(), "sin foco no hay reloj");
    let _ = app.update(Msg::StickerAnim(AnimMsg::Focus(true)));
    assert_eq!(app.stickers_animating(), lottie);

    // Fuera de la vista: nada que animar.
    app.conv.as_ref().unwrap().visible.set(Some((0, 1)));
    assert!(!app.stickers_animating());
    let n = app.conv.as_ref().unwrap().messages.len();
    app.conv.as_ref().unwrap().visible.set(Some((n - 1, n)));

    let _ = app.update(Msg::ToggleMotion);
    assert!(!app.stickers_animating(), "movimiento reducido: primer fotograma, sin reloj");
}

#[tokio::test]
async fn el_fotograma_animado_llega_a_la_burbuja_y_se_congela_sin_foco() {
    let mut app = app_with_tgs_bubble().await;
    let s = tgs_sticker();
    let Ok(tgs) = Tgs::load(&tc_stickers::tgs::compress(MOVING.as_bytes()), "app-anim-test") else {
        assert!(!tc_stickers::tgs::lottie_enabled());
        assert!(app.sticker_anim_frame(&s, crate::conv::STICKER).is_none());
        return;
    };
    let tgs = Arc::new(tgs);
    // Primer tic: se pide el TGS (la descarga es una Task); mientras, imagen estática.
    let t0 = Instant::now();
    let _ = app.update(Msg::StickerAnim(AnimMsg::Tick(t0)));
    assert!(app.sticker_anim_frame(&s, crate::conv::STICKER).is_none());
    let _ = app.update(Msg::StickerAnim(AnimMsg::Loaded(FILE, Some(tgs.clone()))));
    // Llega el lote (el que toca y los siguientes) y el siguiente tic lo muestra.
    let frames: Vec<usize> = (0..60).step_by(2).collect();
    let _ = app.update(Msg::StickerAnim(AnimMsg::Frames(FILE, sticker_anim::render_handles(&tgs, &frames))));
    let _ = app.update(Msg::StickerAnim(AnimMsg::Tick(t0 + Duration::from_millis(100))));
    let shown = app.sticker_anim_frame(&s, crate::conv::STICKER).expect("fotograma animado en la burbuja");
    let _ = app.update(Msg::StickerAnim(AnimMsg::Tick(t0 + Duration::from_millis(400))));
    let later = app.sticker_anim_frame(&s, crate::conv::STICKER).unwrap();
    assert_ne!(shown.id(), later.id(), "avanza con el reloj");
    // En pequeño (selector) no se anima si no es el sticker activo.
    assert!(app.sticker_anim_frame(&s, 60.0).is_none());

    // Sin foco: se congela en el mismo fotograma aunque llegue otro tic.
    let _ = app.update(Msg::StickerAnim(AnimMsg::Focus(false)));
    let _ = app.update(Msg::StickerAnim(AnimMsg::Tick(t0 + Duration::from_millis(900))));
    assert_eq!(app.sticker_anim_frame(&s, crate::conv::STICKER).map(|h| h.id()), Some(later.id()));
    // Movimiento reducido: imagen estática (primer fotograma).
    let _ = app.update(Msg::StickerAnim(AnimMsg::Focus(true)));
    let _ = app.update(Msg::ToggleMotion);
    assert!(app.sticker_anim_frame(&s, crate::conv::STICKER).is_none());
}

#[tokio::test]
async fn en_el_selector_se_anima_el_sticker_bajo_el_raton() {
    let mut app = app_with_tgs_bubble().await;
    let s = tgs_sticker();
    let set = StickerSet { id: StickerSetId(77), short_name: "red".into(), title: "Gatos".into(), stickers: vec![s.clone()] };
    let _ = app.update(Msg::StickerSets(Ok(vec![set])));
    let _ = app.update(Msg::TogglePicker);
    // El panel abre en Emojis: se pasa a la pestaña de stickers.
    let _ = app.update(Msg::Panel(crate::panel::PanelMsg::Tab(crate::panel::PanelTab::Stickers)));
    let tab = app.picker_tabs().iter().position(|t| matches!(t, crate::stickers::Tab::Set(0))).unwrap();
    let _ = app.update(Msg::PickerTab(tab));
    assert_eq!(app.picker_active_sticker(), None);
    let _ = app.update(Msg::StickerAnim(AnimMsg::Hover(s.id)));
    assert_eq!(app.picker_active_sticker(), Some(s.id));
    let _ = app.update(Msg::StickerAnim(AnimMsg::Unhover(StickerId(1))));
    assert_eq!(app.picker_active_sticker(), Some(s.id), "salir de otra celda no lo quita");
    let _ = app.update(Msg::StickerAnim(AnimMsg::Unhover(s.id)));
    assert_eq!(app.picker_active_sticker(), None);
}
