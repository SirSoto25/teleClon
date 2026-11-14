//! Capturas sin ventana de la descarga de archivos (`iced_test`, renderizador headless): tarjetas en
//! cada estado, menú contextual de un documento y de una foto, y el visor con «Guardar».
//!
//! Ignorada por defecto (depende de las fuentes del sistema y escribe PNG):
//! `TC_DL_SHOTS=$PWD/docs/design/screens cargo test -p tc-ui capturas_de_la_descarga -- --ignored`

use std::path::PathBuf;
use std::sync::Arc;

use iced::Settings;
use tc_core::*;
use tc_mock::{BIG_CHAT, MockBackend, MockConfig};

use crate::app::chat_actions::ChatMsg;
use crate::app::downloads::DlState;
use crate::app::{App, Backend, Flags, Msg};

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

#[tokio::test]
#[ignore = "escribe capturas; depende de las fuentes del sistema"]
async fn capturas_de_la_descarga() {
    let dir = PathBuf::from(std::env::var("TC_DL_SHOTS").unwrap_or_else(|_| std::env::temp_dir().join("parlo-dl-shots").to_string_lossy().into_owned()));
    std::fs::create_dir_all(&dir).unwrap();
    let mock = Arc::new(MockBackend::new(MockConfig { now: Some(1_762_430_400), ..Default::default() }));
    let backend: Backend = mock.clone();
    let (mut app, _) = App::new(Flags::new(backend.clone()));
    let _ = app.update(Msg::Me(backend.me().await));
    let _ = app.update(Msg::Contacts(backend.contacts().await));
    let _ = app.update(Msg::DialogsLoaded(backend.dialogs(DialogsPage { offset: 0, limit: 500, archived: false }).await));
    let _ = app.update(Msg::Open(BIG_CHAT));
    let r = backend.history(BIG_CHAT, Anchor::Latest, crate::conv::PAGE).await;
    let _ = app.update(Msg::History { peer: BIG_CHAT, older: false, result: r });
    let t0 = std::time::Instant::now();
    for i in 1..=120u64 {
        let _ = app.update(Msg::Frame(t0 + std::time::Duration::from_millis(16 * i)));
    }

    // Un chat de pocos mensajes: cuatro documentos (uno por estado) y una foto.
    let c = app.conv.as_mut().unwrap();
    let base = c.messages.iter().find(|m| m.media.as_ref().is_some_and(|md| md.kind == MediaKind::File)).cloned().expect("hay un documento de muestra");
    let docs: Vec<Message> = [("Presupuesto reforma 2026.pdf", 2_400_000), ("ruta_sierra.gpx", 3_100_000), ("apuntes_tema3.odt", 880_000), ("acta de la reunión de vecinos.docx", 52_000)]
        .iter()
        .enumerate()
        .map(|(i, (n, sz))| {
            let mut m = base.clone();
            m.id = MsgId(900_000 + i as i32);
            m.text.clear();
            let md = m.media.as_mut().unwrap();
            md.file_name = Some((*n).into());
            md.size_bytes = *sz;
            m
        })
        .collect();
    let photo = c.messages.iter().find(|m| m.media.as_ref().is_some_and(|md| md.kind == MediaKind::Photo)).cloned().unwrap();
    assert_eq!(docs.len(), 4, "los datos de muestra traen al menos cuatro documentos");
    c.messages = docs.clone();
    let home = PathBuf::from("/home/ana/Descargas");
    let peer = BIG_CHAT;
    app.dl.force(peer, docs[1].id, home.join("a"), DlState::Running { done: 1_300_000, total: 3_100_000 });
    app.dl.force(peer, docs[2].id, home.join("b"), DlState::Done);
    app.dl.force(peer, docs[3].id, home.join("c"), DlState::Failed("Sin conexión. Se reintentará al volver la red.".into()));
    shot(&app, &dir, "a3-tarjetas-estados.png");

    let _ = app.update(Msg::Chat(ChatMsg::SetCursor(700.0, 300.0)));
    let _ = app.update(Msg::Chat(ChatMsg::OpenMessageMenu(docs[0].id)));
    shot(&app, &dir, "a3-menu-documento.png");
    let _ = app.update(Msg::Chat(ChatMsg::CloseMenu));
    app.chat_actions.menu = None;

    app.conv.as_mut().unwrap().messages = vec![photo.clone()];
    let _ = app.update(Msg::Chat(ChatMsg::SetCursor(700.0, 300.0)));
    let _ = app.update(Msg::Chat(ChatMsg::OpenMessageMenu(photo.id)));
    shot(&app, &dir, "a3-menu-foto.png");
    app.chat_actions.menu = None;

    let _ = app.update(Msg::OpenPhoto { peer, msg: photo.id });
    for i in 121..=200u64 {
        let _ = app.update(Msg::Frame(t0 + std::time::Duration::from_millis(16 * i)));
    }
    shot(&app, &dir, "a3-visor-guardar.png");
}
