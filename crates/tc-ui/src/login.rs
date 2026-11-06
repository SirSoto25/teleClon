//! Inicio de sesión (S01–S06): teléfono → código → contraseña 2FA, y QR como alternativa.
//! Solo habla con `tc_core::AuthApi`. El estado (`Login`) y las reglas puras (normalizar el
//! teléfono, filtrar el código, cuentas atrás) se prueban sin ventana; `App::on_login` lanza las
//! llamadas al backend como `Task`.

use std::mem::discriminant;
use std::time::{Duration, Instant};

use iced::Task;
use iced::widget::{Id, operation, qr_code};
use tc_core::{AuthState, BackendError, BackendResult, CodeKind, CodeRequest, QrToken, SignInResult, User};

use crate::app::{App, Msg, Phase};

pub const PHONE_ID: &str = "login-phone";
pub const CODE_ID: &str = "login-code";
pub const PASSWORD_ID: &str = "login-password";

/// Paso visible del inicio de sesión.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Step {
    /// Comprobando la sesión guardada.
    Checking,
    Phone,
    Code(CodeRequest),
    Password { hint: Option<String> },
    Qr(QrToken),
    /// El número no tiene cuenta: este cliente no registra cuentas.
    SignUp,
}

/// Campo al que se refiere un error (para pintarlo en rojo junto al campo).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Field {
    Phone,
    Code,
    Password,
}

#[derive(Debug, Clone)]
pub enum LoginMsg {
    /// Respuesta de `auth_state` al arrancar.
    State(BackendResult<AuthState>),
    /// Cambio de estado que llega por `auth_events`.
    Event(AuthState),
    RetryCheck,
    PhoneChanged(String),
    SubmitPhone,
    CodeSent(BackendResult<CodeRequest>),
    CodeChanged(String),
    SubmitCode,
    SignedIn(BackendResult<SignInResult>),
    PasswordChanged(String),
    TogglePassword,
    SubmitPassword,
    PasswordChecked(BackendResult<User>),
    Resend,
    UseQr,
    QrReady(BackendResult<QrToken>),
    /// Volver al paso del teléfono (cambiar número, salir del QR o del registro).
    UsePhone,
    Tick(Instant),
}

#[derive(Debug, Clone)]
pub struct Login {
    pub step: Step,
    pub phone: String,
    pub code: String,
    pub password: String,
    pub show_password: bool,
    /// Hay una petición en curso (botones en «cargando», sin envíos dobles).
    pub busy: bool,
    pub error: Option<(Option<Field>, String)>,
    /// Fin de la espera por límite de intentos (`FloodWait`).
    pub flood_until: Option<Instant>,
    /// Desde cuándo se puede pedir otro código.
    pub resend_at: Option<Instant>,
    /// Instante del último tic (las cuentas atrás se pintan respecto a él).
    pub now: Instant,
}

impl Default for Login {
    fn default() -> Self {
        Self::new(Instant::now())
    }
}

/// Teléfono en formato internacional sin separadores (`+34600000000`), o `None` si no lo parece:
/// tiene que empezar por `+` y llevar de 7 a 15 cifras (E.164).
pub fn normalize_phone(raw: &str) -> Option<String> {
    let s: String = raw.chars().filter(|c| !matches!(c, ' ' | '-' | '.' | '(' | ')' | '\u{a0}')).collect();
    let digits = s.strip_prefix('+')?;
    (digits.chars().all(|c| c.is_ascii_digit()) && (7..=15).contains(&digits.len())).then(|| format!("+{digits}"))
}

/// Deja solo cifras y recorta a la longitud esperada del código, si se conoce.
pub fn clean_code(raw: &str, max: Option<u32>) -> String {
    let max = max.map_or(12, |m| m as usize);
    raw.chars().filter(char::is_ascii_digit).take(max).collect()
}

/// `m:ss` de una cuenta atrás.
pub fn countdown(secs: u64) -> String {
    format!("{}:{:02}", secs / 60, secs % 60)
}

fn left(until: Option<Instant>, now: Instant) -> Option<u64> {
    let d = until?.checked_duration_since(now)?;
    (!d.is_zero()).then(|| d.as_secs() + u64::from(d.subsec_nanos() > 0))
}

/// Texto propio para cada error del inicio de sesión.
pub fn login_error(e: &BackendError) -> String {
    match e {
        BackendError::InvalidInput("phone") => "Ese número no parece válido. Revísalo, con el prefijo del país.".into(),
        BackendError::InvalidInput("code") => "El código no es correcto. Revísalo e inténtalo otra vez.".into(),
        BackendError::InvalidInput("code_expired") => "El código ha caducado. Pide uno nuevo.".into(),
        BackendError::InvalidInput("password") => "La contraseña no es correcta.".into(),
        BackendError::FloodWait(s) => format!("Demasiados intentos. Vuelve a probar en {}.", countdown(u64::from(*s))),
        BackendError::Network => "Sin conexión con la red. Comprueba tu conexión y vuelve a intentarlo.".into(),
        other => crate::app::error_text(other),
    }
}

/// Dónde llegó el código, en una frase.
pub fn code_where(req: &CodeRequest) -> String {
    match req.kind {
        CodeKind::App => "Te hemos enviado el código a la app, en otro dispositivo donde ya tengas la sesión abierta.".into(),
        CodeKind::Sms => format!("Te hemos enviado un SMS con el código al {}.", req.phone),
        CodeKind::Call => format!("Vas a recibir una llamada al {} que te dictará el código.", req.phone),
        CodeKind::FlashCall | CodeKind::MissedCall => format!("Vas a recibir una llamada perdida al {}: el código son las últimas cifras del número.", req.phone),
        CodeKind::Email => "Te hemos enviado el código por correo electrónico.".into(),
        CodeKind::Other => format!("Te hemos enviado el código para el {}.", req.phone),
    }
}

/// Cómo llegará el código si se pide otra vez.
pub fn resend_label(kind: CodeKind) -> &'static str {
    match kind {
        CodeKind::Sms => "Enviar el código por SMS",
        CodeKind::Call | CodeKind::FlashCall | CodeKind::MissedCall => "Recibir el código por llamada",
        CodeKind::Email => "Enviar el código por correo",
        CodeKind::App | CodeKind::Other => "Enviar el código otra vez",
    }
}

impl Login {
    pub fn new(now: Instant) -> Self {
        Self {
            step: Step::Checking,
            phone: String::new(),
            code: String::new(),
            password: String::new(),
            show_password: false,
            busy: false,
            error: None,
            flood_until: None,
            resend_at: None,
            now,
        }
    }

    pub fn flood_left(&self) -> Option<u64> {
        left(self.flood_until, self.now)
    }

    pub fn resend_left(&self) -> Option<u64> {
        left(self.resend_at, self.now)
    }

    /// ¿Hace falta un tic por segundo (cuenta atrás o QR con caducidad)?
    pub fn needs_tick(&self) -> bool {
        self.flood_left().is_some() || self.resend_left().is_some() || matches!(self.step, Step::Qr(_))
    }

    pub fn field_error(&self, f: Field) -> Option<&str> {
        self.error.as_ref().filter(|(x, _)| *x == Some(f)).map(|(_, t)| t.as_str())
    }

    pub fn general_error(&self) -> Option<&str> {
        self.error.as_ref().filter(|(x, _)| x.is_none()).map(|(_, t)| t.as_str())
    }

    /// ¿Se puede pulsar el botón principal del paso actual?
    pub fn can_submit(&self) -> bool {
        if self.busy || self.flood_left().is_some() {
            return false;
        }
        match &self.step {
            Step::Phone => !self.phone.trim().is_empty(),
            Step::Code(_) => !self.code.is_empty(),
            Step::Password { .. } => !self.password.is_empty(),
            _ => false,
        }
    }

    fn fail(&mut self, field: Option<Field>, e: &BackendError) {
        self.busy = false;
        match e {
            BackendError::FloodWait(s) => {
                // Las cuentas atrás se miden desde la respuesta, no desde el último tic.
                self.now = Instant::now();
                self.flood_until = Some(self.now + Duration::from_secs(u64::from(*s)));
                self.error = None;
            }
            other => self.error = Some((field, login_error(other))),
        }
    }

    /// Aplica un estado del backend. Devuelve el usuario si la sesión quedó iniciada.
    pub fn apply(&mut self, st: AuthState) -> Option<User> {
        self.busy = false;
        match st {
            AuthState::Checking => self.step = Step::Checking,
            AuthState::LoggedOut => {
                // Un `LoggedOut` repetido no debe borrar lo que se está escribiendo.
                if !matches!(self.step, Step::Phone) {
                    self.step = Step::Phone;
                    self.code.clear();
                    self.password.clear();
                }
            }
            AuthState::WaitingCode(req) => {
                if !matches!(&self.step, Step::Code(r) if *r == req) {
                    self.now = Instant::now();
                    self.resend_at = req.timeout_secs.map(|s| self.now + Duration::from_secs(u64::from(s)));
                    self.code.clear();
                    self.error = None;
                }
                self.step = Step::Code(req);
            }
            AuthState::WaitingPassword { hint } => {
                if !matches!(self.step, Step::Password { .. }) {
                    self.password.clear();
                    self.error = None;
                }
                self.step = Step::Password { hint };
            }
            AuthState::WaitingQr(tok) => {
                self.error = None;
                self.step = Step::Qr(tok);
            }
            AuthState::SignUpRequired => {
                self.error = None;
                self.step = Step::SignUp;
            }
            AuthState::LoggedIn(u) => {
                self.error = None;
                self.code.clear();
                self.password.clear();
                return Some(u);
            }
        }
        None
    }

    /// Vuelve al paso del teléfono conservando el número escrito.
    pub fn back_to_phone(&mut self) {
        self.step = Step::Phone;
        self.busy = false;
        self.error = None;
        self.code.clear();
        self.password.clear();
        self.resend_at = None;
    }
}

fn focus_for(step: &Step) -> Task<Msg> {
    match step {
        Step::Phone => operation::focus(Id::new(PHONE_ID)),
        Step::Code(_) => operation::focus(Id::new(CODE_ID)),
        Step::Password { .. } => operation::focus(Id::new(PASSWORD_ID)),
        _ => Task::none(),
    }
}

fn wrap(m: LoginMsg) -> Msg {
    Msg::Login(m)
}

impl App {
    /// Aplica un estado de sesión: actualiza el paso, el QR y, al entrar, pasa a la app.
    pub(crate) fn apply_auth(&mut self, st: AuthState) -> Task<Msg> {
        if self.phase != Phase::Auth {
            // Ya dentro: solo importa que la sesión se cierre o se revoque.
            if matches!(st, AuthState::LoggedOut) && self.phase == Phase::Main {
                return self.back_to_login();
            }
            return Task::none();
        }
        let before = discriminant(&self.login.step);
        if let AuthState::WaitingQr(tok) = &st
            && self.qr_url != tok.url
        {
            self.qr = qr_code::Data::new(tok.url.as_bytes()).ok();
            self.qr_url.clone_from(&tok.url);
        }
        if let Some(u) = self.login.apply(st) {
            self.me = Some(u);
            return self.enter_main();
        }
        if discriminant(&self.login.step) == before { Task::none() } else { focus_for(&self.login.step) }
    }

    pub(crate) fn on_login(&mut self, m: LoginMsg) -> Task<Msg> {
        match m {
            LoginMsg::State(Ok(st)) | LoginMsg::Event(st) => self.apply_auth(st),
            LoginMsg::State(Err(e)) => {
                self.login.step = Step::Phone;
                self.login.fail(None, &e);
                Task::none()
            }
            LoginMsg::RetryCheck => {
                self.login.error = None;
                self.login.step = Step::Checking;
                self.call_auth(|a| async move { a.auth_state().await }, |r| wrap(LoginMsg::State(r)))
            }
            LoginMsg::PhoneChanged(s) => {
                self.login.phone = s;
                if self.login.field_error(Field::Phone).is_some() {
                    self.login.error = None;
                }
                Task::none()
            }
            LoginMsg::SubmitPhone => {
                if !self.login.can_submit() || self.login.step != Step::Phone {
                    return Task::none();
                }
                let Some(phone) = normalize_phone(&self.login.phone) else {
                    self.login.error =
                        Some((Some(Field::Phone), "Escribe el número completo con el prefijo del país, por ejemplo +34 600 000 000.".into()));
                    return Task::none();
                };
                self.login.busy = true;
                self.login.error = None;
                self.call_auth(move |a| async move { a.request_login_code(phone).await }, |r| wrap(LoginMsg::CodeSent(r)))
            }
            LoginMsg::CodeSent(r) => match r {
                Ok(req) => self.apply_auth(AuthState::WaitingCode(req)),
                Err(e) => {
                    let field = if matches!(self.login.step, Step::Code(_)) { Field::Code } else { Field::Phone };
                    self.login.fail(Some(field), &e);
                    Task::none()
                }
            },
            LoginMsg::CodeChanged(s) => {
                let len = match &self.login.step {
                    Step::Code(r) => r.length,
                    _ => None,
                };
                self.login.code = clean_code(&s, len);
                if self.login.field_error(Field::Code).is_some() {
                    self.login.error = None;
                }
                // Con la longitud conocida, el código completo se envía solo.
                if len.is_some_and(|l| self.login.code.len() == l as usize) {
                    return self.on_login(LoginMsg::SubmitCode);
                }
                Task::none()
            }
            LoginMsg::SubmitCode => {
                if !self.login.can_submit() || !matches!(self.login.step, Step::Code(_)) {
                    return Task::none();
                }
                self.login.busy = true;
                self.login.error = None;
                let code = self.login.code.clone();
                self.call_auth(move |a| async move { a.sign_in(code).await }, |r| wrap(LoginMsg::SignedIn(r)))
            }
            LoginMsg::SignedIn(r) => match r {
                Ok(SignInResult::LoggedIn(u)) => self.apply_auth(AuthState::LoggedIn(u)),
                Ok(SignInResult::PasswordNeeded { hint }) => self.apply_auth(AuthState::WaitingPassword { hint }),
                Ok(SignInResult::SignUpNeeded) => self.apply_auth(AuthState::SignUpRequired),
                Err(e) => {
                    self.login.fail(Some(Field::Code), &e);
                    if matches!(e, BackendError::InvalidInput(_)) {
                        self.login.code.clear();
                    }
                    operation::focus(Id::new(CODE_ID))
                }
            },
            LoginMsg::PasswordChanged(s) => {
                self.login.password = s;
                if self.login.field_error(Field::Password).is_some() {
                    self.login.error = None;
                }
                Task::none()
            }
            LoginMsg::TogglePassword => {
                self.login.show_password = !self.login.show_password;
                operation::focus(Id::new(PASSWORD_ID))
            }
            LoginMsg::SubmitPassword => {
                if !self.login.can_submit() || !matches!(self.login.step, Step::Password { .. }) {
                    return Task::none();
                }
                self.login.busy = true;
                self.login.error = None;
                let pw = self.login.password.clone();
                self.call_auth(move |a| async move { a.check_password(pw).await }, |r| wrap(LoginMsg::PasswordChecked(r)))
            }
            LoginMsg::PasswordChecked(r) => match r {
                Ok(u) => self.apply_auth(AuthState::LoggedIn(u)),
                Err(e) => {
                    self.login.fail(Some(Field::Password), &e);
                    if matches!(e, BackendError::InvalidInput(_)) {
                        self.login.password.clear();
                    }
                    operation::focus(Id::new(PASSWORD_ID))
                }
            },
            LoginMsg::Resend => {
                if self.login.busy || self.login.resend_left().is_some() || self.login.flood_left().is_some() {
                    return Task::none();
                }
                self.login.busy = true;
                self.call_auth(|a| async move { a.resend_code().await }, |r| wrap(LoginMsg::CodeSent(r)))
            }
            LoginMsg::UseQr => {
                if self.login.busy || self.login.flood_left().is_some() {
                    return Task::none();
                }
                self.login.busy = true;
                self.login.error = None;
                self.call_auth(|a| async move { a.qr_login_start().await }, |r| wrap(LoginMsg::QrReady(r)))
            }
            LoginMsg::QrReady(r) => match r {
                Ok(tok) => self.apply_auth(AuthState::WaitingQr(tok)),
                Err(e) => {
                    self.login.fail(None, &e);
                    Task::none()
                }
            },
            LoginMsg::UsePhone => {
                self.login.back_to_phone();
                operation::focus(Id::new(PHONE_ID))
            }
            LoginMsg::Tick(now) => {
                self.login.now = now;
                if self.login.flood_left().is_none() {
                    self.login.flood_until = None;
                }
                Task::none()
            }
        }
    }

    /// `Esc` en el inicio de sesión: vuelve al teléfono desde código, contraseña, QR o registro.
    pub(crate) fn login_escape(&mut self) -> Task<Msg> {
        match self.login.step {
            Step::Code(_) | Step::Password { .. } | Step::Qr(_) | Step::SignUp if !self.login.busy => self.on_login(LoginMsg::UsePhone),
            _ => Task::none(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn telefono_en_formato_internacional() {
        assert_eq!(normalize_phone("+34 600 00-00 02"), Some("+34600000002".into()));
        assert_eq!(normalize_phone(" +1 (555) 010.9999 "), Some("+15550109999".into()));
        assert_eq!(normalize_phone("600000002"), None, "sin prefijo");
        assert_eq!(normalize_phone("+34"), None, "corto");
        assert_eq!(normalize_phone("+34 6OO"), None, "letras");
        assert_eq!(normalize_phone("+1234567890123456"), None, "más de 15 cifras");
    }

    #[test]
    fn codigo_solo_cifras_y_recortado() {
        assert_eq!(clean_code("1a2 3-4", None), "1234");
        assert_eq!(clean_code("123456789", Some(5)), "12345");
    }

    #[test]
    fn cuentas_atras() {
        assert_eq!(countdown(45), "0:45");
        assert_eq!(countdown(65), "1:05");
        let mut l = Login::new(Instant::now());
        l.fail(None, &BackendError::FloodWait(45));
        assert_eq!(l.flood_left(), Some(45));
        assert!(l.error.is_none(), "la espera se pinta aparte, no como error");
        let end = l.flood_until.unwrap();
        l.now = end - Duration::from_millis(800);
        assert_eq!(l.flood_left(), Some(1), "redondea hacia arriba");
        l.now = end;
        assert_eq!(l.flood_left(), None);
    }

    #[test]
    fn errores_con_texto_propio() {
        assert!(login_error(&BackendError::InvalidInput("code")).contains("código"));
        assert!(login_error(&BackendError::InvalidInput("password")).contains("contraseña"));
        assert!(login_error(&BackendError::FloodWait(90)).contains("1:30"));
        assert!(login_error(&BackendError::Network).contains("conexión"));
    }

    #[test]
    fn un_logged_out_repetido_no_borra_lo_escrito() {
        let mut l = Login::new(Instant::now());
        l.apply(AuthState::LoggedOut);
        l.phone = "+34 6".into();
        l.apply(AuthState::LoggedOut);
        assert_eq!(l.phone, "+34 6");
        assert_eq!(l.step, Step::Phone);
    }
}
