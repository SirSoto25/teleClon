//! Binario de teleClon: lee la configuración del entorno y de un `.env` propio (nunca del repo ni
//! de los registros), elige el backend (`TC_BACKEND`: `mock` por defecto o `telegram`), registra las
//! fuentes de `tc-theme` y arranca la UI de iced. El runtime tokio lo crea iced (feature `tokio`);
//! no se crea un segundo runtime: la red real se construye dentro de él con un `Connector`.
//! Ver docs/architecture.md §10.
//!
//! Variables: `TC_BACKEND`, `TC_ENV_FILE` (ruta del `.env`, por defecto `./.env`),
//! `TC_SESSION_FILE` (sesión de la red real), `TC_THEME`, `TC_REDUCED_MOTION`,
//! `TC_MOCK_LATENCY_MS`, `TC_MOCK_SCRIPT_SECS`, `TC_MOCK_LOGIN` (mock sin sesión: pantalla de
//! login), `TC_MOCK_QR_SECS`, `TC_DEMO_SHOTS=<dir>` (guion de capturas).
#![forbid(unsafe_code)]

mod envfile;
mod select;

use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant};

use iced::widget::text_editor::{Action, Edit};
use tc_core::{ChatBackend, GroupId, PeerRef, StickerId};
use tc_mock::{ANIMATED_SET_ID, BIG_CHAT, MOCK_2FA_PASSWORD, MOCK_LOGIN_CODE, MockBackend, MockConfig, SECRET_ID};
use tc_telegram::{FileSessionStore, GrammersBackend, SessionStore, TelegramConfig};
use tc_theme::ThemeKind;
use tc_ui::{Connector, DemoStep, LoginMsg, Msg, Session};
use tracing_subscriber::EnvFilter;

use crate::envfile::EnvFile;
use crate::select::Choice;

fn env_u64(name: &str, default: u64) -> u64 {
    std::env::var(name).ok().and_then(|v| v.trim().parse().ok()).unwrap_or(default)
}

fn env_flag(name: &str) -> bool {
    std::env::var(name).is_ok_and(|v| matches!(v.trim(), "1" | "true" | "si" | "sí" | "yes"))
}

/// Carga el `.env` (si existe). Solo se registra cuántas variables tiene, nunca cuáles valen qué.
fn load_env_file() -> EnvFile {
    let path = std::env::var_os("TC_ENV_FILE").map_or_else(|| PathBuf::from(".env"), PathBuf::from);
    match EnvFile::load(&path) {
        Ok(Some((f, bad))) => {
            tracing::info!(variables = f.len(), "leído {}", path.display());
            if bad > 0 {
                tracing::warn!(lineas = bad, "{} tiene líneas que no son asignaciones CLAVE=valor; se ignoran", path.display());
            }
            f
        }
        Ok(None) => EnvFile::default(),
        Err(e) => {
            tracing::warn!("no se pudo leer {}: {}", path.display(), e.kind());
            EnvFile::default()
        }
    }
}

/// Configuración de la red real: entorno del proceso (`from_env`) y, si falta algo, el `.env`.
fn telegram_config(file: &EnvFile) -> Result<TelegramConfig, tc_telegram::ConfigError> {
    TelegramConfig::from_env().or_else(|_| TelegramConfig::from_lookup(envfile::lookup(file, |k| std::env::var(k).ok())))
}

fn session_path(file: &EnvFile) -> PathBuf {
    if let Some(p) = std::env::var_os("TC_SESSION_FILE") {
        return PathBuf::from(p);
    }
    if let Some(p) = file.get("TC_SESSION_FILE").filter(|p| !p.is_empty()) {
        return PathBuf::from(p);
    }
    directories::ProjectDirs::from("", "", "teleclon")
        .map_or_else(|| PathBuf::from("teleclon-data"), |d| d.data_dir().to_path_buf())
        .join("telegram")
        .join("session.json")
}

/// Crea el `GrammersBackend` dentro del runtime de iced (lanza sus tareas al construirse).
fn telegram_connector(cfg: TelegramConfig, path: PathBuf) -> Connector {
    Arc::new(move || {
        let (cfg, path) = (cfg.clone(), path.clone());
        Box::pin(async move {
            let store: Arc<dyn SessionStore> = Arc::new(FileSessionStore::new(path));
            match GrammersBackend::connect(cfg, store).await {
                Ok(b) => {
                    let b = Arc::new(b);
                    Ok(Session { chat: b.clone(), auth: Some(b) })
                }
                Err(e) => Err(format!("No se pudo preparar la conexión con la red: {}", tc_ui::app::error_text(&e))),
            }
        })
    })
}

fn main() -> iced::Result {
    tracing_subscriber::fmt().with_env_filter(EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("warn"))).init();

    let file = load_env_file();
    let backend_var = std::env::var("TC_BACKEND").ok().or_else(|| file.get("TC_BACKEND").map(str::to_string));
    let choice = select::choose(backend_var.as_deref(), || telegram_config(&file));

    let theme = match std::env::var("TC_THEME").as_deref() {
        Ok("light" | "claro") => ThemeKind::Light,
        _ => ThemeKind::Dark,
    };
    let reduced_motion = env_flag("TC_REDUCED_MOTION");
    let demo_dir = std::env::var("TC_DEMO_SHOTS").ok();
    let on_screenshot: tc_ui::app::SaveShot = Arc::new(save_png);

    let flags = match choice {
        Choice::Telegram(cfg) => {
            let path = session_path(&file);
            tracing::info!("backend: red real (sesión en {})", path.display());
            tc_ui::Flags { theme, reduced_motion, on_screenshot: Some(on_screenshot), ..tc_ui::Flags::with_connector(telegram_connector(cfg, path)) }
        }
        Choice::Mock { notice } => {
            if let Some(n) = &notice {
                tracing::warn!("{n}");
                eprintln!("teleClon: {n}");
            }
            let demo = demo_dir.is_some();
            let latency = Duration::from_millis(env_u64("TC_MOCK_LATENCY_MS", 80));
            let script = Duration::from_secs(env_u64("TC_MOCK_SCRIPT_SECS", 8).max(1));
            // En el guion de capturas el QR no se «escanea» solo, para poder fotografiarlo.
            let qr = Duration::from_secs(env_u64("TC_MOCK_QR_SECS", if demo { 3600 } else { 4 }));
            let logged_in = !(env_flag("TC_MOCK_LOGIN") || demo);
            let mock = Arc::new(MockBackend::new(MockConfig { latency, logged_in, qr_accept_delay: qr, ..MockConfig::default() }));
            let backend: Arc<dyn ChatBackend> = mock.clone();
            // Se ejecuta dentro del runtime tokio de iced (necesario para `tokio::spawn` del guion).
            let on_start: Arc<dyn Fn() + Send + Sync> = Arc::new({
                let mock = mock.clone();
                move || mock.start_script(script)
            });
            tc_ui::Flags {
                auth: Some(mock),
                on_start: Some(on_start),
                theme,
                reduced_motion,
                demo: demo_dir.as_deref().map(demo_script).unwrap_or_default(),
                on_screenshot: Some(on_screenshot),
                simulated: true,
                login_hint: Some(format!(
                    "Backend de pruebas: el código es {MOCK_LOGIN_CODE}. Los números que acaban en 2 piden la contraseña «{MOCK_2FA_PASSWORD}», \
                     en 0 no tienen cuenta y en 9 simulan el límite de intentos."
                )),
                notice,
                ..tc_ui::Flags::new(backend)
            }
        }
    };

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

/// Guion de capturas (`TC_DEMO_SHOTS=<directorio>`): login (teléfono, espera por límite, QR, código
/// erróneo, 2FA), selector de stickers (estáticos y animados) y stickers en la conversación, el
/// recorrido de M1 (chat largo, grupo, tema claro, huella, búsqueda) y los chats secretos (aviso
/// EXPERIMENTAL, autodestrucción, nuevo chat secreto). Guarda una captura en cada estado y cierra.
fn demo_script(dir: &str) -> Vec<(Duration, DemoStep)> {
    let at = |ms: u64, step: DemoStep| (Duration::from_millis(ms), step);
    let shot = |name: &str| DemoStep::Shot(format!("{dir}/{name}"));
    let msg = |m: Msg| DemoStep::Do(Box::new(m));
    let login = |m: LoginMsg| DemoStep::Do(Box::new(Msg::Login(m)));
    let paste = |s: &str| DemoStep::Do(Box::new(Msg::Edit(Action::Edit(Edit::Paste(Arc::new(s.to_string()))))));
    // La espera simulada dura 45 s: un tic «adelantado» la da por cumplida para seguir el guion.
    let past_flood = Instant::now() + Duration::from_secs(55);
    let static_sticker = |i: i64| StickerId(tc_stickers::pack::SET_ID.0 * 100 + i);
    let animated_sticker = |i: i64| StickerId(ANIMATED_SET_ID.0 * 100 + i);
    // La captura se resuelve de forma asíncrona: tras cada `shot` se deja 1 s antes del paso siguiente.
    let m = 12_000;
    vec![
        at(1500, shot("m2-login-telefono.png")),
        at(2500, login(LoginMsg::PhoneChanged("+34 600 000 009".into()))),
        at(2600, login(LoginMsg::SubmitPhone)),
        at(3500, shot("m2-login-espera.png")),
        at(4500, login(LoginMsg::Tick(past_flood))),
        at(4700, login(LoginMsg::UseQr)),
        at(5700, shot("m2-login-qr.png")),
        at(6700, login(LoginMsg::UsePhone)),
        at(6800, login(LoginMsg::PhoneChanged("+34 600 000 002".into()))),
        at(6900, login(LoginMsg::SubmitPhone)),
        at(7700, login(LoginMsg::CodeChanged("11111".into()))),
        at(8600, shot("m2-login-codigo-error.png")),
        at(9600, login(LoginMsg::CodeChanged(MOCK_LOGIN_CODE.into()))),
        at(10400, login(LoginMsg::PasswordChanged(MOCK_2FA_PASSWORD.into()))),
        at(10800, shot("m2-login-2fa.png")),
        at(11800, login(LoginMsg::SubmitPassword)),
        // Stickers.
        at(m + 1000, msg(Msg::Open(BIG_CHAT))),
        at(m + 2000, msg(Msg::TogglePicker)),
        at(m + 2200, msg(Msg::PickerNav(tc_ui::stickers::Nav::Right))),
        at(m + 2300, msg(Msg::PickerNav(tc_ui::stickers::Nav::Right))),
        at(m + 3200, shot("m2-selector-stickers.png")),
        at(m + 4200, msg(Msg::PickerTab(1))),
        at(m + 5200, shot("m2-selector-animados.png")),
        at(m + 6200, msg(Msg::SendSticker(animated_sticker(1)))),
        at(m + 6500, msg(Msg::SendSticker(static_sticker(6)))),
        at(m + 7500, shot("m2-conversacion-stickers.png")),
        // Recorrido de M1.
        at(m + 8500, paste("Probando el compositor: Enter envía, Shift+Enter salta de línea.")),
        at(m + 8700, msg(Msg::Send)),
        at(m + 9800, shot("m1-oscuro-chat-largo.png")),
        // Subir mucho por el historial de 5000 mensajes: pagina hacia atrás y solo pinta lo visible.
        at(m + 10800, DemoStep::ScrollUp(3000.0)),
        at(m + 11300, DemoStep::ScrollUp(3000.0)),
        at(m + 11800, DemoStep::ScrollUp(3000.0)),
        at(m + 12300, DemoStep::ScrollUp(3000.0)),
        at(m + 12800, DemoStep::ScrollUp(3000.0)),
        at(m + 13300, DemoStep::ScrollUp(3000.0)),
        at(m + 14100, shot("m1-oscuro-chat-largo-historial.png")),
        at(m + 15100, msg(Msg::Open(PeerRef::Group(GroupId(5000))))),
        at(m + 17100, shot("m1-oscuro-grupo.png")),
        at(m + 18100, msg(Msg::ToggleTheme)),
        at(m + 19300, shot("m1-claro-grupo.png")),
        at(m + 20300, msg(Msg::Open(PeerRef::Secret(SECRET_ID)))),
        at(m + 21500, msg(Msg::ShowKey)),
        at(m + 23000, shot("m1-claro-secreto-huella.png")),
        at(m + 24000, msg(Msg::CloseOverlay)),
        at(m + 24200, msg(Msg::ToggleTheme)),
        at(m + 25400, shot("m2-secreto-experimental.png")),
        at(m + 26400, msg(Msg::ShowTtl)),
        at(m + 27500, shot("m2-secreto-ttl.png")),
        at(m + 28500, msg(Msg::CloseOverlay)),
        at(m + 28700, msg(Msg::NewSecret(None))),
        at(m + 29800, shot("m2-nuevo-secreto.png")),
        at(m + 30800, msg(Msg::CloseOverlay)),
        at(m + 31000, msg(Msg::SearchChanged("ca".into()))),
        at(m + 32300, shot("m1-oscuro-secreto-busqueda.png")),
        at(m + 33300, DemoStep::Exit),
    ]
}
