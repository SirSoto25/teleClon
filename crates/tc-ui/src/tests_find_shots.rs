//! Capturas sin ventana de la búsqueda en el chat (`iced_test`, renderizador headless): barra con
//! contador y resaltado en las burbujas, filtro activo y «Sin resultados».
//!
//! Ignorada por defecto (depende de las fuentes del sistema y escribe PNG):
//! `TC_FIND_SHOTS=$PWD/docs/design/screens cargo test -p tc-ui capturas_de_la_busqueda -- --ignored`

use std::path::PathBuf;
use std::sync::Arc;

use iced::Settings;
use tc_core::*;
use tc_mock::{BIG_CHAT, MockBackend, MockConfig};
use tc_theme::ThemeKind;

use crate::app::find::FindMsg;
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

async fn deliver(app: &mut App) {
    let f = app.find_active().unwrap();
    let (seq, peer, q) = (f.seq(), f.peer, f.query());
    let result = app.scoped_backend().unwrap().search_in_chat(peer, q).await;
    let _ = app.update(Msg::Find(FindMsg::Results { seq, append: false, result }));
    // Sin runtime no se ejecuta el desplazamiento: la captura se toma al final del chat.
    app.conv.as_mut().unwrap().from_bottom = 0.0;
}

fn frames(app: &mut App, from: u64, to: u64) {
    let t0 = std::time::Instant::now();
    for i in from..=to {
        let _ = app.update(Msg::Frame(t0 + std::time::Duration::from_millis(16 * i)));
    }
}

#[tokio::test]
#[ignore = "escribe capturas; depende de las fuentes del sistema"]
async fn capturas_de_la_busqueda() {
    let dir = PathBuf::from(std::env::var("TC_FIND_SHOTS").unwrap_or_else(|_| std::env::temp_dir().join("parlo-find-shots").to_string_lossy().into_owned()));
    std::fs::create_dir_all(&dir).unwrap();
    for theme in [ThemeKind::Dark, ThemeKind::Light] {
        let mock = Arc::new(MockBackend::new(MockConfig { now: Some(1_762_430_400), ..Default::default() }));
        let backend: Backend = mock.clone();
        let (mut app, _) = App::new(Flags { theme, ..Flags::new(backend.clone()) });
        let _ = app.update(Msg::Me(backend.me().await));
        let _ = app.update(Msg::Contacts(backend.contacts().await));
        let _ = app.update(Msg::DialogsLoaded(backend.dialogs(DialogsPage { offset: 0, limit: 500, archived: false }).await));
        // Unos mensajes recientes que hablan de la cafetería (entrantes y propios).
        let say = |t: &str, r: i64| OutgoingText { text: t.into(), entities: vec![], reply_to: None, random_id: RandomId(r) };
        mock.inject_incoming(BIG_CHAT, "¿Nos vemos en la cafetería de la esquina antes del cine?").unwrap();
        backend.send_text(BIG_CHAT, say("Vale, pero la Cafetería Central cierra a las ocho.", 1)).await.unwrap();
        mock.inject_incoming(BIG_CHAT, "Pues a las siete. Yo llevo las entradas.").unwrap();
        backend.send_text(BIG_CHAT, say("Perfecto. Te espero dentro, al fondo de la cafeteria.", 2)).await.unwrap();
        let _ = app.update(Msg::Open(BIG_CHAT));
        let r = backend.history(BIG_CHAT, Anchor::Latest, crate::conv::PAGE).await;
        let _ = app.update(Msg::History { peer: BIG_CHAT, older: false, result: r });
        frames(&mut app, 1, 120);

        let suffix = if theme == ThemeKind::Dark { "" } else { "-claro" };
        let _ = app.update(Msg::Find(FindMsg::Open));
        let _ = app.update(Msg::Find(FindMsg::Input("cafeteria".into())));
        let _ = app.update(Msg::Find(FindMsg::Tick(std::time::Instant::now() + std::time::Duration::from_secs(1))));
        deliver(&mut app).await;
        // El segundo más reciente seleccionado: banda sobre una burbuja entrante… o propia.
        let _ = app.update(Msg::Find(FindMsg::Older));
        app.conv.as_mut().unwrap().from_bottom = 0.0;
        shot(&app, &dir, &format!("b2-busqueda-resaltado{suffix}.png"));

        if theme == ThemeKind::Dark {
            let _ = app.update(Msg::Find(FindMsg::Input("xilofono".into())));
            let _ = app.update(Msg::Find(FindMsg::Tick(std::time::Instant::now() + std::time::Duration::from_secs(1))));
            deliver(&mut app).await;
            shot(&app, &dir, "b2-busqueda-sin-resultados.png");

            let _ = app.update(Msg::Find(FindMsg::Input(String::new())));
            let _ = app.update(Msg::Find(FindMsg::Filter(SearchFilter::Photos)));
            deliver(&mut app).await;
            shot(&app, &dir, "b2-busqueda-filtro-fotos.png");
        }
    }
}
