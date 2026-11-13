//! Capturas sin ventana del panel del compositor (`iced_test`, renderizador headless) y
//! comprobación de que los emojis salen en color con la fuente del sistema.
//!
//! Ignorada por defecto (depende de las fuentes del sistema y escribe PNG):
//! `TC_PANEL_SHOTS=<dir> cargo test -p tc-ui capturas_del_panel -- --ignored`

use std::sync::Arc;

use iced::Settings;
use tc_core::*;
use tc_mock::{BIG_CHAT, MockBackend, MockConfig};

use crate::app::{App, Backend, Flags, Msg};
use crate::emoji::{Category, Tone};
use crate::gifs::GifMsg;
use crate::panel::{PanelMsg, PanelTab};

fn settings() -> Settings {
    Settings {
        fonts: tc_theme::fonts::BUNDLED.iter().map(|f| std::borrow::Cow::Borrowed(*f)).collect(),
        default_font: crate::style::ui_font(),
        ..Settings::default()
    }
}

/// Guarda la vista actual y devuelve los píxeles RGBA (ancho, alto, bytes).
fn shot(app: &App, dir: &std::path::Path, name: &str) -> image::RgbaImage {
    // `iced_test` añade el nombre del renderizador: `<nombre>-wgpu.png` o `<nombre>-tiny-skia.png`.
    let stem = name.trim_end_matches(".png");
    let ours = |e: &std::fs::DirEntry| e.file_name().to_string_lossy().starts_with(&format!("{stem}-"));
    for e in std::fs::read_dir(dir).unwrap().flatten().filter(ours) {
        let _ = std::fs::remove_file(e.path());
    }
    let mut sim = iced_test::Simulator::with_size(settings(), (1240.0, 800.0), app.view());
    let snap = sim.snapshot(&app.theme()).expect("captura");
    assert!(snap.matches_image(dir.join(name)).expect("PNG escrito"));
    let written = std::fs::read_dir(dir).unwrap().flatten().find(ours).expect("PNG escrito").path();
    image::open(written).expect("PNG legible").to_rgba8()
}

/// ¿Hay píxeles claramente saturados (color) en la franja `y0..y1`?
fn colorful(img: &image::RgbaImage, y0: u32, y1: u32) -> usize {
    img.enumerate_pixels()
        .filter(|(_, y, _)| (y0..y1).contains(y))
        .filter(|(_, _, p)| {
            let [r, g, b, _] = p.0.map(i32::from);
            let (mx, mn) = (r.max(g).max(b), r.min(g).min(b));
            mx > 120 && mx - mn > 90
        })
        .count()
}

#[tokio::test]
#[ignore = "escribe capturas; depende de las fuentes del sistema"]
async fn capturas_del_panel() {
    let dir = std::path::PathBuf::from(std::env::var("TC_PANEL_SHOTS").unwrap_or_else(|_| std::env::temp_dir().join("parlo-panel-shots").to_string_lossy().into_owned()));
    std::fs::create_dir_all(&dir).unwrap();
    let mock = Arc::new(MockBackend::new(MockConfig { now: Some(1_762_430_400), ..Default::default() }));
    let backend: Backend = mock.clone();
    let (mut app, _) = App::new(Flags::new(backend.clone()));
    let _ = app.update(Msg::Me(backend.me().await));
    let _ = app.update(Msg::Contacts(backend.contacts().await));
    let _ = app.update(Msg::DialogsLoaded(backend.dialogs(DialogsPage { offset: 0, limit: 500, archived: false }).await));
    let _ = app.update(Msg::StickerSets(backend.installed_sticker_sets().await));
    let _ = app.update(Msg::Open(BIG_CHAT));
    let r = backend.history(BIG_CHAT, Anchor::Latest, crate::conv::PAGE).await;
    let _ = app.update(Msg::History { peer: BIG_CHAT, older: false, result: r });
    let pm = |m| Msg::Panel(m);

    // Emojis: caras (la primera vez, sin recientes).
    let _ = app.update(Msg::TogglePicker);
    let _ = app.update(pm(PanelMsg::Usage(backend.emoji_usage().await)));
    let img = shot(&app, &dir, "panel-emojis.png");
    // El panel ocupa la parte baja (2× de escala): los emojis amarillos de las caras deben verse en color.
    let h = img.height();
    let n = colorful(&img, h / 2, h);
    println!("píxeles en color en el panel de emojis: {n}");
    assert!(n > 2000, "los emojis deberían dibujarse en color (Noto Color Emoji): {n}");

    for e in ["😀", "🔥", "👍", "❤️", "😂"] {
        let _ = app.update(pm(PanelMsg::Insert(e.into())));
    }
    let _ = app.update(pm(PanelMsg::Category(Category::Recent)));
    let _ = app.update(pm(PanelMsg::ToggleTones));
    shot(&app, &dir, "panel-emojis-recientes-tono.png");
    let _ = app.update(pm(PanelMsg::Tone(Tone::Medium)));
    let _ = app.update(pm(PanelMsg::EmojiQuery("mano".into())));
    shot(&app, &dir, "panel-emojis-busqueda-mano-tono.png");
    let _ = app.update(pm(PanelMsg::EmojiQuery(String::new())));
    let _ = app.update(pm(PanelMsg::Category(Category::Flags)));
    shot(&app, &dir, "panel-emojis-banderas.png");

    // GIFs: guardados con miniaturas y animaciones cargadas como lo haría el runtime.
    let _ = app.update(pm(PanelMsg::Tab(PanelTab::Gifs)));
    let _ = app.update(Msg::Panel(PanelMsg::Gif(GifMsg::Saved(backend.saved_gifs().await))));
    let peer = app.chat_peer_for_tests();
    for g in app.panel.gifs.shown().into_iter().cloned().collect::<Vec<_>>() {
        let f = g.file.clone().unwrap();
        let req = crate::media::ImageRequest::Media { peer, msg: MsgId(0), file: f.clone(), variant: MediaVariant::Thumb, aspect: None };
        let key = req.key();
        let img = crate::media::load(backend.clone(), req).await;
        let _ = app.update(Msg::ImageLoaded(key, img));
        // GIF reales y MP4/H.264: todos se animan (ADR-0010).
        let bytes = backend.download_media(BIG_CHAT, MsgId(0), f.clone(), MediaVariant::Full).await.unwrap();
        let _ = app.update(Msg::Panel(PanelMsg::Gif(GifMsg::AnimLoaded(f.id, crate::gifs::decode(&bytes).map(Arc::new)))));
    }
    shot(&app, &dir, "panel-gifs-guardados.png");
    let _ = app.update(Msg::Panel(PanelMsg::Gif(GifMsg::Query("lluvia".into()))));
    shot(&app, &dir, "panel-gifs-filtro.png");

    // Stickers: el selector de siempre como tercera pestaña.
    let _ = app.update(pm(PanelMsg::Tab(PanelTab::Stickers)));
    shot(&app, &dir, "panel-stickers.png");

    // Burbujas: un GIF real y un MP4/H.264, los dos animados (ya decodificados arriba: misma caché).
    let _ = app.update(pm(PanelMsg::Tab(PanelTab::Gifs)));
    let _ = app.update(Msg::Panel(PanelMsg::Gif(GifMsg::Query(String::new()))));
    let saved = backend.saved_gifs().await.unwrap();
    for g in [&saved[0], &saved[1]] {
        let _ = app.update(Msg::TogglePicker);
        if !app.picker.open {
            let _ = app.update(Msg::TogglePicker);
        }
        let _ = app.update(Msg::Panel(PanelMsg::Gif(GifMsg::Send(g.id))));
        let rid = app.conv.as_ref().unwrap().messages.last().unwrap().random_id.unwrap();
        let r = backend.send_gif(BIG_CHAT, g.clone(), None, rid).await;
        let _ = app.update(Msg::Sent { peer: BIG_CHAT, random_id: rid, result: r });
    }
    // Sin ventana no llegan frames: se dan a mano para que acaben los fundidos de entrada.
    let t0 = std::time::Instant::now();
    for i in 1..=120u64 {
        let _ = app.update(Msg::Frame(t0 + std::time::Duration::from_millis(16 * i)));
    }
    shot(&app, &dir, "conversacion-gifs.png");
}

impl App {
    /// Chat neutro con el que el panel pide las miniaturas (para las pruebas).
    fn chat_peer_for_tests(&self) -> PeerRef {
        PeerRef::User(self.me_id().unwrap_or(UserId(0)))
    }
}
