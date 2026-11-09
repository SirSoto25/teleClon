//! Binario de Parlo: lee la configuración del entorno y de un `.env` propio (nunca del repo ni
//! de los registros), elige el backend (`TC_BACKEND`: `mock` por defecto o `telegram`), registra las
//! fuentes de `tc-theme` y arranca la UI de iced. El runtime tokio lo crea iced (feature `tokio`);
//! no se crea un segundo runtime: la red real se construye dentro de él con un `Connector`.
//! Ver docs/architecture.md §10.
//!
//! Variables: `TC_BACKEND`, `TC_ENV_FILE` (ruta del `.env`; por defecto `./.env` y, si no existe,
//! `<datos>/.env`, necesario al abrir la app empaquetada desde el menú/Finder, ADR-0006),
//! `TC_ACCOUNT` (cuenta de la red real, `[A-Za-z0-9_-]`, por defecto `default`: servicio
//! `parlo/<cuenta>` del llavero y base `<datos>/accounts/<cuenta>/cache.sqlite3`),
//! `TC_SESSION_FILE` (respaldo en claro 0600 de la sesión cuando no hay llavero, y origen de la
//! migración única al llavero), `TC_THEME`, `TC_REDUCED_MOTION`,
//! `TC_MOCK_LATENCY_MS`, `TC_MOCK_SCRIPT_SECS`, `TC_MOCK_LOGIN` (mock sin sesión: pantalla de
//! login), `TC_MOCK_QR_SECS`, `TC_DEMO_SHOTS=<dir>` (guion de capturas).
#![forbid(unsafe_code)]
// En Windows, el binario de release no abre una consola junto a la ventana (en debug sí, para ver los registros).
#![cfg_attr(all(windows, not(debug_assertions)), windows_subsystem = "windows")]

mod envfile;
mod select;
mod stores;

use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant};

use iced::widget::text_editor::{Action, Edit};
use tc_core::{ChatBackend, GroupId, PeerRef, StickerId};
use tc_mock::{ANIMATED_SET_ID, BIG_CHAT, MOCK_2FA_PASSWORD, MOCK_LOGIN_CODE, MockBackend, MockConfig, SECRET_ID};
use tc_telegram::{GrammersBackend, TelegramConfig};
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
    let candidates = match std::env::var_os("TC_ENV_FILE") {
        Some(p) => vec![PathBuf::from(p)],
        // Al abrir la app desde el menú, el Explorador o el Finder el directorio actual no es el del
        // usuario: también se busca en el directorio de datos (%APPDATA%\Parlo, Application Support...).
        None => std::iter::once(PathBuf::from(".env")).chain(tc_store::default_data_dir().ok().map(|d| d.join(".env"))).collect(),
    };
    for path in candidates {
        match EnvFile::load(&path) {
            Ok(Some((f, bad))) => {
                tracing::info!(variables = f.len(), "leído {}", path.display());
                if bad > 0 {
                    tracing::warn!(lineas = bad, "{} tiene líneas que no son asignaciones CLAVE=valor; se ignoran", path.display());
                }
                return f;
            }
            Ok(None) => {}
            Err(e) => {
                tracing::warn!("no se pudo leer {}: {}", path.display(), e.kind());
                return EnvFile::default();
            }
        }
    }
    EnvFile::default()
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
    tc_store::default_data_dir()
        .unwrap_or_else(|_| PathBuf::from("parlo-data"))
        .join("telegram")
        .join("session.json")
}

/// Cuenta de la red real (`TC_ACCOUNT`, entorno o `.env`; por defecto `default`).
fn account_id(file: &EnvFile) -> String {
    std::env::var("TC_ACCOUNT")
        .ok()
        .or_else(|| file.get("TC_ACCOUNT").map(str::to_string))
        .map(|a| a.trim().to_string())
        .filter(|a| !a.is_empty())
        .unwrap_or_else(|| "default".to_string())
}

/// Dónde guarda la red real su estado.
#[derive(Clone)]
struct TelegramPaths {
    account: String,
    session_file: PathBuf,
    data_dir: Option<PathBuf>,
}

/// Crea el `GrammersBackend` dentro del runtime de iced (lanza sus tareas al construirse). La sesión va
/// al llavero del sistema (respaldo: archivo 0600) y el estado de los chats secretos, cifrado, a SQLite.
fn telegram_connector(cfg: TelegramConfig, paths: TelegramPaths) -> Connector {
    Arc::new(move || {
        let (cfg, paths) = (cfg.clone(), paths.clone());
        Box::pin(async move {
            // Llavero (D-Bus) y SQLite son síncronos: fuera del hilo del runtime.
            let stores = tokio::task::spawn_blocking(move || stores::telegram_stores(&paths.account, &paths.session_file, paths.data_dir.as_deref()))
                .await
                .map_err(|_| "No se pudo preparar el almacenamiento local.".to_string())?;
            match GrammersBackend::connect(cfg, stores.session).await {
                Ok(b) => {
                    if let Some(secrets) = stores.secrets {
                        b.set_secret_store(secrets).await;
                    }
                    let b = Arc::new(b);
                    // Las descargas (fotos de perfil y de mensajes) pasan por la caché de disco de la cuenta.
                    let chat: Arc<dyn ChatBackend> = match stores.media {
                        Some(media) => Arc::new(tc_store::MediaCacheLayer::new(b.clone(), media)),
                        None => b.clone(),
                    };
                    Ok(Session { chat, auth: Some(b) })
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
            let paths = TelegramPaths {
                account: account_id(&file),
                session_file: session_path(&file),
                data_dir: tc_store::default_data_dir().ok(),
            };
            tracing::info!(cuenta = %paths.account, "backend: red real (sesión en el llavero; respaldo {})", paths.session_file.display());
            tc_ui::Flags { theme, reduced_motion, on_screenshot: Some(on_screenshot), ..tc_ui::Flags::with_connector(telegram_connector(cfg, paths)) }
        }
        Choice::Mock { notice } => {
            if let Some(n) = &notice {
                tracing::warn!("{n}");
                eprintln!("Parlo: {n}");
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
        .window(window_settings())
        .antialiasing(true);
    for font in tc_theme::fonts::BUNDLED {
        app = app.font(font);
    }
    app.run()
}

/// Icono PROVISIONAL de la ventana (tools/packaging/make_icons.py; ADR-0006).
const WINDOW_ICON: &[u8] = include_bytes!("../../../packaging/icons/parlo-256.png");

fn window_settings() -> iced::window::Settings {
    iced::window::Settings {
        size: iced::Size::new(1240.0, 800.0),
        icon: iced::window::icon::from_file_data(WINDOW_ICON, Some(image::ImageFormat::Png)).ok(),
        // Wayland/X11: el id coincide con packaging/linux/parlo.desktop para que el escritorio asocie icono y nombre.
        #[cfg(target_os = "linux")]
        platform_specific: iced::window::settings::PlatformSpecific { application_id: "parlo".into(), ..Default::default() },
        ..Default::default()
    }
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
