//! Autenticación (flujos F01 teléfono → código → 2FA y F02 QR).
//!
//! `AuthApi` es un sub-trait independiente: de momento NO es supertrait de `ChatBackend`
//! para no romper otras implementaciones de `ChatBackend` (p. ej. el decorador de caché).
//! Quien necesite ambos usa `Arc<dyn ChatBackend>` y `Arc<dyn AuthApi>` sobre el mismo objeto.
use async_trait::async_trait;
use futures::stream::BoxStream;

use crate::{error::BackendResult, ids::Timestamp, model::User};

/// Por dónde llega el código de inicio de sesión.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CodeKind {
    /// Mensaje dentro de otra sesión ya abierta de la cuenta.
    App,
    Sms,
    Call,
    FlashCall,
    MissedCall,
    Email,
    Other,
}

/// Código solicitado: cómo llegó, cómo llegará si se reenvía y cuándo se puede reenviar.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CodeRequest {
    pub phone: String,
    pub kind: CodeKind,
    /// Longitud esperada del código, si el servidor la indica.
    pub length: Option<u32>,
    /// Vía del siguiente envío (`resend_code`), si la hay.
    pub next: Option<CodeKind>,
    /// Segundos hasta poder pedir el reenvío.
    pub timeout_secs: Option<u32>,
}

/// Token de inicio de sesión por QR: la UI dibuja `url` como código QR.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QrToken {
    pub url: String,
    /// Instante (UTC) en que caduca; el backend emite uno nuevo antes.
    pub expires: Timestamp,
}

/// Estado de la sesión. Se consulta con `auth_state` y llega por `auth_events` al cambiar.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AuthState {
    /// Comprobando la sesión guardada (aún sin respuesta de la red).
    Checking,
    /// Sin sesión: hay que pedir teléfono o QR.
    LoggedOut,
    WaitingCode(CodeRequest),
    /// La cuenta tiene verificación en dos pasos.
    WaitingPassword { hint: Option<String> },
    WaitingQr(QrToken),
    /// El número no tiene cuenta; este cliente no registra cuentas nuevas.
    SignUpRequired,
    LoggedIn(User),
}

/// Resultado de verificar el código.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SignInResult {
    LoggedIn(User),
    PasswordNeeded { hint: Option<String> },
    SignUpNeeded,
}

#[async_trait]
pub trait AuthApi: Send + Sync {
    async fn auth_state(&self) -> BackendResult<AuthState>;
    /// Pide el código para `phone` (formato internacional, p. ej. `+34…`).
    async fn request_login_code(&self, phone: String) -> BackendResult<CodeRequest>;
    /// Reenvía el código por la vía `next` del último `CodeRequest`.
    async fn resend_code(&self) -> BackendResult<CodeRequest>;
    /// Código incorrecto → `InvalidInput("code")`; caducado → `InvalidInput("code_expired")`.
    async fn sign_in(&self, code: String) -> BackendResult<SignInResult>;
    /// Contraseña de 2FA (SRP; nunca se envía en claro). Incorrecta → `InvalidInput("password")`.
    async fn check_password(&self, password: String) -> BackendResult<User>;
    /// Empieza (o reinicia) el login por QR. Los tokens renovados, la aceptación
    /// (`LoggedIn`) o la petición de 2FA (`WaitingPassword`) llegan por `auth_events`.
    async fn qr_login_start(&self) -> BackendResult<QrToken>;
    /// Cierra la sesión en el servidor y borra la sesión guardada.
    async fn log_out(&self) -> BackendResult<()>;
    /// Cambios de `AuthState`, en orden. Cada llamada crea un suscriptor nuevo.
    fn auth_events(&self) -> BoxStream<'static, AuthState>;
}
