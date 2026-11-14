//! Capturas sin ventana del envío de archivos (`iced_test`, renderizador headless): diálogo con una
//! foto, diálogo con varios archivos (pie demasiado largo), burbujas subiéndose y zona de soltar.
//!
//! Ignorada por defecto (depende de las fuentes del sistema y escribe PNG):
//! `TC_SEND_SHOTS=$PWD/docs/design/screens cargo test -p tc-ui capturas_del_envio -- --ignored`

use std::sync::Arc;

use iced::Settings;
use iced::widget::text_editor;
use tc_core::*;
use tc_mock::{BIG_CHAT, MockBackend, MockConfig};

use crate::app::uploads::FilesMsg;
use crate::app::{App, Backend, Flags, Msg};
use crate::files::{self, Picked};

fn settings() -> Settings {
    Settings {
        fonts: tc_theme::fonts::BUNDLED.iter().map(|f| std::borrow::Cow::Borrowed(*f)).collect(),
        default_font: crate::style::ui_font(),
        ..Settings::default()
    }
}

fn shot(app: &App, dir: &std::path::Path, name: &str) {
    // `iced_test` añade el nombre del renderizador (`-wgpu` o `-tiny-skia`): se renombra al final.
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

/// Foto de muestra: degradado cálido con un «sol» (para que la vista previa se distinga).
fn sample_photo(dir: &std::path::Path, name: &str, w: u32, h: u32) -> std::path::PathBuf {
    let img = ::image::RgbaImage::from_fn(w, h, |x, y| {
        let (fx, fy) = (x as f32 / w as f32, y as f32 / h as f32);
        let (dx, dy) = (fx - 0.7, fy - 0.35);
        let sun = (dx * dx + dy * dy).sqrt() < 0.12;
        if sun {
            ::image::Rgba([255, 214, 120, 255])
        } else if fy > 0.62 {
            ::image::Rgba([(40.0 + 60.0 * fx) as u8, (90.0 + 40.0 * fy) as u8, (140.0 + 40.0 * fx) as u8, 255])
        } else {
            ::image::Rgba([(230.0 - 40.0 * fy) as u8, (120.0 + 60.0 * fy) as u8, (70.0 + 90.0 * fy) as u8, 255])
        }
    });
    let p = dir.join(name);
    // JPEG no admite canal alfa.
    ::image::DynamicImage::ImageRgba8(img).to_rgb8().save(&p).unwrap();
    p
}

#[tokio::test]
#[ignore = "escribe capturas; depende de las fuentes del sistema"]
async fn capturas_del_envio() {
    let dir = std::path::PathBuf::from(std::env::var("TC_SEND_SHOTS").unwrap_or_else(|_| std::env::temp_dir().join("parlo-send-shots").to_string_lossy().into_owned()));
    std::fs::create_dir_all(&dir).unwrap();
    let work = std::env::temp_dir().join(format!("parlo-send-shots-src-{}", std::process::id()));
    std::fs::create_dir_all(&work).unwrap();
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
    let settle = |app: &mut App| {
        for i in 1..=120u64 {
            let _ = app.update(Msg::Frame(t0 + std::time::Duration::from_millis(16 * i)));
        }
    };

    // 1. Una foto: vista previa grande, elegir compresión y pie.
    let photo = sample_photo(&work, "atardecer-en-la-playa.jpg", 1600, 1000);
    let (files, _) = files::inspect(vec![photo.clone()]).await;
    app.composer = text_editor::Content::with_text("Así estaba ayer la playa 🌅");
    let _ = app.update(Msg::Files(FilesMsg::Inspected { files, errors: vec![] }));
    settle(&mut app);
    shot(&app, &dir, "a2-envio-foto.png");

    // 2. Varios archivos: lista con miniaturas e iconos; pie demasiado largo.
    let more: Vec<Picked> = {
        let a = sample_photo(&work, "plano-cocina.png", 900, 1200);
        let pdf = work.join("Presupuesto reforma 2026.pdf");
        std::fs::write(&pdf, vec![0u8; 2_400_000]).unwrap();
        let song = work.join("maqueta.mp3");
        std::fs::write(&song, vec![0u8; 5_300_000]).unwrap();
        files::inspect(vec![a, pdf, song]).await.0
    };
    let _ = app.update(Msg::Files(FilesMsg::Inspected { files: more, errors: vec![] }));
    app.uploads.dialog.as_mut().unwrap().caption = text_editor::Content::with_text(&"Te paso todo lo de la reforma. ".repeat(34));
    shot(&app, &dir, "a2-envio-varios-pie-largo.png");

    // 3. Burbujas subiéndose: el álbum a medias y el documento esperando.
    app.uploads.dialog.as_mut().unwrap().caption = text_editor::Content::with_text("Lo de la reforma");
    let _ = app.update(Msg::Files(FilesMsg::Send));
    let firsts: Vec<RandomId> = app.uploads.jobs.keys().copied().collect();
    for (i, f) in firsts.iter().enumerate() {
        let job = app.uploads.jobs.get_mut(f).unwrap();
        if i == 0 || job.media.items.len() > 1 {
            job.done = job.total * 2 / 5;
            job.progress_known = true;
        }
    }
    settle(&mut app);
    shot(&app, &dir, "a2-subida-progreso.png");

    // 4. Arrastrando archivos sobre la ventana.
    let _ = app.update(Msg::Files(FilesMsg::Hover(true)));
    shot(&app, &dir, "a2-soltar.png");
    std::fs::remove_dir_all(&work).ok();
}
