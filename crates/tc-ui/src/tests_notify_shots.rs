//! Capturas sin ventana del diálogo «Notificaciones» (S28, `iced_test`, renderizador headless):
//! valores por defecto en oscuro y ajustes cambiados (con «No molestar» hasta una hora) en claro.
//!
//! Ignorada por defecto (depende de las fuentes del sistema y escribe PNG):
//! `TC_NOTIFY_SHOTS=$PWD/docs/design/screens cargo test -p tc-ui capturas_de_notificaciones -- --ignored`

use std::path::PathBuf;
use std::sync::Arc;

use iced::Settings;
use tc_core::*;
use tc_mock::{BIG_CHAT, MockBackend, MockConfig};
use tc_notify::rules::Preview;
use tc_theme::ThemeKind;

use crate::app::notify::{DndChoice, NotifyMsg, Toggle};
use crate::app::{App, Backend, Flags, Msg};

const NOW: i64 = 1_762_430_400;

fn settings() -> Settings {
    Settings {
        fonts: tc_theme::fonts::BUNDLED.iter().map(|f| std::borrow::Cow::Borrowed(*f)).collect(),
        default_font: crate::style::ui_font(),
        ..Settings::default()
    }
}

fn shot(app: &App, dir: &std::path::Path, name: &str) {
    let stem = name.trim_end_matches(".png");
    let ours = |e: &std::fs::DirEntry| e.file_name().to_string_lossy().starts_with(&format!("{stem}-"));
    for e in std::fs::read_dir(dir).unwrap().flatten().filter(ours) {
        let _ = std::fs::remove_file(e.path());
    }
    let mut sim = iced_test::Simulator::with_size(settings(), (1240.0, 800.0), app.view());
    let snap = sim.snapshot(&app.theme()).expect("captura");
    assert!(snap.matches_image(dir.join(name)).expect("PNG escrito"));
    let written = std::fs::read_dir(dir).unwrap().flatten().find(ours).expect("PNG escrito").path();
    std::fs::rename(written, dir.join(name)).unwrap();
}

async fn app(theme: ThemeKind) -> App {
    let mock = Arc::new(MockBackend::new(MockConfig { now: Some(NOW), ..Default::default() }));
    let backend: Backend = mock.clone();
    let (mut app, _) = App::new(Flags { theme, ..Flags::new(backend.clone()) });
    app.notifs.clock = Some(NOW);
    app.tz = jiff::tz::TimeZone::UTC;
    let _ = app.update(Msg::Me(backend.me().await));
    let _ = app.update(Msg::Contacts(backend.contacts().await));
    let _ = app.update(Msg::DialogsLoaded(backend.dialogs(DialogsPage { offset: 0, limit: 500, archived: false }).await));
    let _ = app.update(Msg::Open(BIG_CHAT));
    let r = backend.history(BIG_CHAT, Anchor::Latest, crate::conv::PAGE).await;
    let _ = app.update(Msg::History { peer: BIG_CHAT, older: false, result: r });
    let _ = app.update(Msg::Notify(NotifyMsg::Settings));
    let t0 = std::time::Instant::now();
    for i in 1..=120u64 {
        let _ = app.update(Msg::Frame(t0 + std::time::Duration::from_millis(16 * i)));
    }
    app
}

#[tokio::test]
#[ignore = "escribe capturas; depende de las fuentes del sistema"]
async fn capturas_de_notificaciones() {
    let dir = PathBuf::from(std::env::var("TC_NOTIFY_SHOTS").unwrap_or_else(|_| std::env::temp_dir().join("parlo-notify-shots").to_string_lossy().into_owned()));
    std::fs::create_dir_all(&dir).unwrap();

    let a = app(ThemeKind::Dark).await;
    shot(&a, &dir, "c2-ajustes-avisos.png");

    let mut a = app(ThemeKind::Light).await;
    for (t, v) in [(Toggle::Channels, false), (Toggle::Sound, false), (Toggle::IgnoreJoined, false)] {
        let _ = a.update(Msg::Notify(NotifyMsg::Set(t, v)));
    }
    let _ = a.update(Msg::Notify(NotifyMsg::Preview(Preview::NameOnly)));
    let _ = a.update(Msg::Notify(NotifyMsg::Dnd(DndChoice::Hour)));
    shot(&a, &dir, "c2-ajustes-avisos-claro-no-molestar.png");
}
