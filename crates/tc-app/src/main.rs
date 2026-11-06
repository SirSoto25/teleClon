//! Binario de teleClon: lee la configuración del entorno (nunca del repo), crea el backend
//! (por ahora siempre `MockBackend`; `TC_BACKEND=telegram` llegará en M2.1), registra las fuentes
//! de `tc-theme` y arranca la UI de iced. El runtime tokio lo crea iced (feature `tokio`); no se
//! crea un segundo runtime. Ver docs/architecture.md §10.
#![forbid(unsafe_code)]

use std::sync::Arc;
use std::time::Duration;

use iced::widget::text_editor::{Action, Edit};
use tc_core::{ChatBackend, GroupId, PeerRef};
use tc_mock::{BIG_CHAT, MockBackend, MockConfig, SECRET_ID};
use tc_theme::ThemeKind;
use tc_ui::{DemoStep, Msg};
use tracing_subscriber::EnvFilter;

fn env_u64(name: &str, default: u64) -> u64 {
    std::env::var(name).ok().and_then(|v| v.trim().parse().ok()).unwrap_or(default)
}

fn env_flag(name: &str) -> bool {
    std::env::var(name).is_ok_and(|v| matches!(v.trim(), "1" | "true" | "si" | "sí" | "yes"))
}

fn main() -> iced::Result {
    tracing_subscriber::fmt().with_env_filter(EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("warn"))).init();

    match std::env::var("TC_BACKEND").as_deref() {
        Ok("telegram") => tracing::warn!("TC_BACKEND=telegram aún no está disponible (hito M2.1); se usa el backend de pruebas"),
        Ok("mock") | Err(_) => {}
        Ok(other) => tracing::warn!("TC_BACKEND={other} desconocido; se usa el backend de pruebas"),
    }

    let latency = Duration::from_millis(env_u64("TC_MOCK_LATENCY_MS", 80));
    let script = Duration::from_secs(env_u64("TC_MOCK_SCRIPT_SECS", 8).max(1));
    let theme = match std::env::var("TC_THEME").as_deref() {
        Ok("light" | "claro") => ThemeKind::Light,
        _ => ThemeKind::Dark,
    };
    let reduced_motion = env_flag("TC_REDUCED_MOTION");

    let mock = Arc::new(MockBackend::new(MockConfig { latency, ..MockConfig::default() }));
    let backend: Arc<dyn ChatBackend> = mock.clone();
    // Se ejecuta dentro del runtime tokio de iced (necesario para `tokio::spawn` del guion).
    let on_start: Arc<dyn Fn() + Send + Sync> = Arc::new(move || mock.start_script(script));
    let demo = std::env::var("TC_DEMO_SHOTS").ok().map(|dir| demo_script(&dir)).unwrap_or_default();
    let on_screenshot: tc_ui::app::SaveShot = Arc::new(save_png);
    let flags = tc_ui::Flags { backend, on_start: Some(on_start), theme, reduced_motion, demo, on_screenshot: Some(on_screenshot) };

    let mut app = iced::application(move || tc_ui::App::new(flags.clone()), tc_ui::App::update, tc_ui::App::view)
        .title(tc_ui::App::title)
        .theme(tc_ui::App::theme)
        .style(tc_ui::App::style)
        .subscription(tc_ui::App::subscription)
        .default_font(tc_ui::style::ui_font())
        .window_size((1240.0, 800.0))
        .antialiasing(true);
    for font in tc_theme::fonts::BUNDLED {
        app = app.font(font);
    }
    app.run()
}

fn save_png(path: &str, w: u32, h: u32, rgba: &[u8]) {
    match image::save_buffer(path, rgba, w, h, image::ExtendedColorType::Rgba8) {
        Ok(()) => tracing::info!("captura guardada en {path} ({w}×{h})"),
        Err(e) => tracing::error!("no se pudo guardar la captura {path}: {e}"),
    }
}

/// Guion de capturas (`TC_DEMO_SHOTS=<directorio>`): recorre chat largo, grupo, tema claro y chat
/// secreto con su huella, guarda una captura en cada estado y cierra la app.
fn demo_script(dir: &str) -> Vec<(Duration, DemoStep)> {
    let at = |ms: u64, step: DemoStep| (Duration::from_millis(ms), step);
    let shot = |name: &str| DemoStep::Shot(format!("{dir}/{name}"));
    let paste = |s: &str| DemoStep::Do(Box::new(Msg::Edit(Action::Edit(Edit::Paste(Arc::new(s.to_string()))))));
    vec![
        at(1500, DemoStep::Do(Box::new(Msg::Open(BIG_CHAT)))),
        at(2500, paste("Probando el compositor: Enter envía, Shift+Enter salta de línea.")),
        at(2700, DemoStep::Do(Box::new(Msg::Send))),
        at(4000, shot("m1-oscuro-chat-largo.png")),
        // Subir mucho por el historial de 5000 mensajes: pagina hacia atrás y solo pinta lo visible.
        at(4200, DemoStep::ScrollUp(3000.0)),
        at(4700, DemoStep::ScrollUp(3000.0)),
        at(5200, DemoStep::ScrollUp(3000.0)),
        at(5700, DemoStep::ScrollUp(3000.0)),
        at(6200, DemoStep::ScrollUp(3000.0)),
        at(6700, DemoStep::ScrollUp(3000.0)),
        at(7500, shot("m1-oscuro-chat-largo-historial.png")),
        at(7800, DemoStep::Do(Box::new(Msg::Open(PeerRef::Group(GroupId(5000)))))),
        at(9800, shot("m1-oscuro-grupo.png")),
        at(10100, DemoStep::Do(Box::new(Msg::ToggleTheme))),
        at(11300, shot("m1-claro-grupo.png")),
        at(11600, DemoStep::Do(Box::new(Msg::Open(PeerRef::Secret(SECRET_ID))))),
        at(12800, DemoStep::Do(Box::new(Msg::ShowKey))),
        at(14300, shot("m1-claro-secreto-huella.png")),
        at(14600, DemoStep::Do(Box::new(Msg::CloseOverlay))),
        at(14800, DemoStep::Do(Box::new(Msg::ToggleTheme))),
        at(15000, DemoStep::Do(Box::new(Msg::SearchChanged("ca".into())))),
        at(16300, shot("m1-oscuro-secreto-busqueda.png")),
        at(16800, DemoStep::Exit),
    ]
}
