//! Login: teléfono → código → contraseña 2FA (SRP) y QR (`auth.exportLoginToken` en crudo).
use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use futures::stream::{self, BoxStream, StreamExt};
use grammers_client::InvocationError;
use grammers_crypto::two_factor_auth::{calculate_2fa, check_p_and_g};
use grammers_session::Session;
use grammers_session::types::{PeerAuth, PeerInfo, UpdateState, UpdatesState};
use grammers_tl_types as tl;
use tc_core::*;
use tokio::sync::broadcast::error::RecvError;

use crate::net::map_error;
use crate::peers::user_from_tl;
use crate::{GrammersBackend, Inner, now_secs};

#[derive(Default)]
pub(crate) struct AuthInner {
    pub(crate) state: Option<AuthState>,
    phone: Option<String>,
    code_hash: Option<String>,
    password: Option<tl::types::account::Password>,
    qr_running: bool,
}

pub(crate) fn code_kind(t: &tl::enums::auth::SentCodeType) -> (CodeKind, Option<u32>) {
    use tl::enums::auth::SentCodeType as T;
    match t {
        T::App(x) => (CodeKind::App, Some(x.length.max(0) as u32)),
        T::Sms(x) => (CodeKind::Sms, Some(x.length.max(0) as u32)),
        T::Call(x) => (CodeKind::Call, Some(x.length.max(0) as u32)),
        T::FlashCall(_) => (CodeKind::FlashCall, None),
        T::MissedCall(x) => (CodeKind::MissedCall, Some(x.length.max(0) as u32)),
        T::EmailCode(x) => (CodeKind::Email, Some(x.length.max(0) as u32)),
        T::FragmentSms(x) => (CodeKind::Sms, Some(x.length.max(0) as u32)),
        T::FirebaseSms(x) => (CodeKind::Sms, Some(x.length.max(0) as u32)),
        _ => (CodeKind::Other, None),
    }
}

fn next_kind(t: &tl::enums::auth::CodeType) -> CodeKind {
    use tl::enums::auth::CodeType as C;
    match t {
        C::Sms | C::FragmentSms => CodeKind::Sms,
        C::Call => CodeKind::Call,
        C::FlashCall => CodeKind::FlashCall,
        C::MissedCall => CodeKind::MissedCall,
    }
}

/// URL del QR: `tg://login?token=<base64url sin relleno>` (formato público de la API).
pub(crate) fn qr_url(token: &[u8]) -> String {
    const A: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_";
    let mut s = String::from("tg://login?token=");
    for chunk in token.chunks(3) {
        let b = [chunk[0], *chunk.get(1).unwrap_or(&0), *chunk.get(2).unwrap_or(&0)];
        let n = (u32::from(b[0]) << 16) | (u32::from(b[1]) << 8) | u32::from(b[2]);
        let chars = chunk.len() + 1;
        for i in 0..chars {
            s.push(A[((n >> (18 - 6 * i)) & 63) as usize] as char);
        }
    }
    s
}

/// Migración de DC durante el login (`PHONE_MIGRATE_X`, `NETWORK_MIGRATE_X`, `USER_MIGRATE_X`).
fn migrate_dc(e: &InvocationError) -> Option<i32> {
    match e {
        InvocationError::Rpc(r) if r.code == 303 => r.value.map(|v| v as i32),
        _ => None,
    }
}

fn valid_phone(p: &str) -> bool {
    let digits = p.strip_prefix('+').unwrap_or(p);
    (5..=20).contains(&digits.len()) && digits.chars().all(|c| c.is_ascii_digit())
}

impl Inner {
    pub(crate) fn set_auth(&self, st: AuthState) {
        self.auth.lock().unwrap_or_else(|e| e.into_inner()).state = Some(st.clone());
        let _ = self.auth_tx.send(st);
    }

    fn auth_lock(&self) -> std::sync::MutexGuard<'_, AuthInner> {
        self.auth.lock().unwrap_or_else(|e| e.into_inner())
    }

    /// Llamada de login: sigue una migración de DC una vez.
    async fn login_call<R: tl::RemoteCall>(&self, req: &R) -> Result<R::Return, InvocationError> {
        match self.net.invoke(req).await {
            Err(e) => match migrate_dc(&e) {
                Some(dc) => {
                    self.switch_home_dc(dc).await;
                    self.net.invoke(req).await
                }
                None => Err(e),
            },
            ok => ok,
        }
    }

    async fn switch_home_dc(&self, dc: i32) {
        let old = self.net.session.home_dc();
        if old != dc {
            self.net.transport.disconnect_from_dc(old);
            let _ = self.net.session.set_home_dc_id(dc).await;
        }
    }

    fn code_request(&self, phone: &str, sent: tl::types::auth::SentCode) -> CodeRequest {
        let (kind, length) = code_kind(&sent.r#type);
        let req = CodeRequest {
            phone: phone.to_string(),
            kind,
            length,
            next: sent.next_type.as_ref().map(next_kind),
            timeout_secs: sent.timeout.map(|t| t.max(0) as u32),
        };
        {
            let mut a = self.auth_lock();
            a.phone = Some(phone.to_string());
            a.code_hash = Some(sent.phone_code_hash);
            a.password = None;
        }
        self.set_auth(AuthState::WaitingCode(req.clone()));
        req
    }

    async fn sent_code(&self, phone: &str, r: tl::enums::auth::SentCode) -> BackendResult<CodeRequest> {
        match r {
            tl::enums::auth::SentCode::Code(c) => Ok(self.code_request(phone, c)),
            tl::enums::auth::SentCode::Success(s) => {
                // El servidor aceptó sin código (p. ej. token de sesión futura).
                let user = self.complete_login(s.authorization).await?;
                let _ = user;
                Err(BackendError::InvalidInput("already_logged_in"))
            }
            tl::enums::auth::SentCode::PaymentRequired(_) => {
                Err(BackendError::Forbidden("el servidor exige un pago para enviar el código a este número".into()))
            }
        }
    }

    /// Tras `auth.authorization`: guarda la cuenta propia, el estado de updates y la sesión.
    pub(crate) async fn complete_login(&self, a: tl::enums::auth::Authorization) -> BackendResult<SignInResult> {
        let a = match a {
            tl::enums::auth::Authorization::Authorization(a) => a,
            tl::enums::auth::Authorization::SignUpRequired(_) => {
                self.set_auth(AuthState::SignUpRequired);
                return Ok(SignInResult::SignUpNeeded);
            }
        };
        let tl::enums::User::User(u) = &a.user else { return Err(BackendError::Internal) };
        let me = user_from_tl(u);
        self.ingest(std::slice::from_ref(&a.user), &[]);
        let _ = self
            .net
            .session
            .cache_peer(PeerInfo::User { id: u.id, auth: u.access_hash.map(PeerAuth::from_hash), bot: Some(u.bot), is_self: Some(true) })
            .await;
        if let Ok(tl::enums::updates::State::State(s)) = self.net.invoke(&tl::functions::updates::GetState {}).await {
            let _ = self
                .net
                .session
                .set_update_state(UpdateState::All(UpdatesState { pts: s.pts, qts: s.qts, date: s.date, seq: s.seq, channels: vec![] }))
                .await;
        }
        self.cache().me = Some(me.id);
        {
            let mut st = self.auth_lock();
            st.phone = None;
            st.code_hash = None;
            st.password = None;
        }
        self.persist().await;
        self.logged_in.send_replace(true);
        self.set_auth(AuthState::LoggedIn(me.clone()));
        Ok(SignInResult::LoggedIn(me))
    }

    async fn fetch_password(&self) -> BackendResult<Option<String>> {
        let tl::enums::account::Password::Password(p) = self.rpc(&tl::functions::account::GetPassword {}).await?;
        let hint = p.hint.clone();
        self.auth_lock().password = Some(p);
        self.set_auth(AuthState::WaitingPassword { hint: hint.clone() });
        Ok(hint)
    }

    /// Sigue el login por QR: renueva el token al caducar y comprueba al recibir `updateLoginToken`.
    async fn qr_loop(self: Arc<Self>, mut expires: i64) {
        loop {
            let wait = Duration::from_secs((expires - now_secs()).clamp(1, 30) as u64);
            tokio::select! {
                _ = self.qr_wake.notified() => {}
                _ = tokio::time::sleep(wait) => {}
            }
            if !matches!(self.auth_lock().state, Some(AuthState::WaitingQr(_))) {
                break;
            }
            match self.export_qr().await {
                Ok(Some(t)) => expires = t.expires.0,
                Ok(None) => break,
                Err(e) => {
                    tracing::warn!(error = %e, "fallo al renovar el QR");
                    if !matches!(e, BackendError::Network | BackendError::FloodWait(_)) {
                        self.set_auth(AuthState::LoggedOut);
                        break;
                    }
                    expires = now_secs() + 5;
                }
            }
        }
        self.auth_lock().qr_running = false;
    }

    /// Exporta (o reexporta) el token. `Some(token)` si sigue esperando; `None` si terminó
    /// (sesión iniciada o hace falta la contraseña).
    pub(crate) async fn export_qr(&self) -> BackendResult<Option<QrToken>> {
        let cfg = &self.net.cfg;
        let req = tl::functions::auth::ExportLoginToken { api_id: cfg.api_id, api_hash: cfg.api_hash.expose().to_string(), except_ids: vec![] };
        let mut res = self.net.invoke(&req).await;
        loop {
            match res {
                Ok(tl::enums::auth::LoginToken::Token(t)) => {
                    let tok = QrToken { url: qr_url(&t.token), expires: Timestamp(t.expires.into()) };
                    self.set_auth(AuthState::WaitingQr(tok.clone()));
                    return Ok(Some(tok));
                }
                Ok(tl::enums::auth::LoginToken::MigrateTo(m)) => {
                    // La cuenta vive en otro DC: se cambia de casa y se importa allí.
                    self.switch_home_dc(m.dc_id).await;
                    res = self.net.invoke(&tl::functions::auth::ImportLoginToken { token: m.token }).await;
                }
                Ok(tl::enums::auth::LoginToken::Success(s)) => {
                    self.complete_login(s.authorization).await?;
                    return Ok(None);
                }
                Err(e) if e.is("SESSION_PASSWORD_NEEDED") => {
                    self.fetch_password().await?;
                    return Ok(None);
                }
                Err(e) => return Err(map_error(&e)),
            }
        }
    }
}

/// Parámetros SRP de la contraseña.
pub(crate) struct Srp {
    pub salt1: Vec<u8>,
    pub salt2: Vec<u8>,
    pub p: Vec<u8>,
    pub g: i32,
    pub srp_b: Vec<u8>,
    pub srp_id: i64,
}

pub(crate) fn srp_params(p: &tl::types::account::Password) -> Option<Srp> {
    let tl::enums::PasswordKdfAlgo::Sha256Sha256Pbkdf2Hmacsha512iter100000Sha256ModPow(a) = p.current_algo.as_ref()? else {
        return None;
    };
    Some(Srp { salt1: a.salt1.clone(), salt2: a.salt2.clone(), p: a.p.clone(), g: a.g, srp_b: p.srp_b.clone()?, srp_id: p.srp_id? })
}

#[async_trait]
impl AuthApi for GrammersBackend {
    async fn auth_state(&self) -> BackendResult<AuthState> {
        Ok(self.inner.auth_lock().state.clone().unwrap_or(AuthState::Checking))
    }

    async fn request_login_code(&self, phone: String) -> BackendResult<CodeRequest> {
        let phone: String = phone.chars().filter(|c| !c.is_whitespace() && *c != '-').collect();
        if !valid_phone(&phone) {
            return Err(BackendError::InvalidInput("phone"));
        }
        let cfg = &self.inner.net.cfg;
        let req = tl::functions::auth::SendCode {
            phone_number: phone.clone(),
            api_id: cfg.api_id,
            api_hash: cfg.api_hash.expose().to_string(),
            settings: tl::types::CodeSettings {
                allow_flashcall: false,
                current_number: false,
                allow_app_hash: false,
                allow_missed_call: false,
                allow_firebase: false,
                unknown_number: false,
                logout_tokens: None,
                token: None,
                app_sandbox: None,
            }
            .into(),
        };
        let r = self.inner.login_call(&req).await.map_err(|e| map_error(&e))?;
        self.inner.sent_code(&phone, r).await
    }

    async fn resend_code(&self) -> BackendResult<CodeRequest> {
        let (phone, hash) = {
            let a = self.inner.auth_lock();
            (a.phone.clone(), a.code_hash.clone())
        };
        let (Some(phone), Some(hash)) = (phone, hash) else { return Err(BackendError::InvalidInput("phone")) };
        let r = self
            .inner
            .rpc(&tl::functions::auth::ResendCode { phone_number: phone.clone(), phone_code_hash: hash, reason: None })
            .await?;
        self.inner.sent_code(&phone, r).await
    }

    async fn sign_in(&self, code: String) -> BackendResult<SignInResult> {
        let (phone, hash) = {
            let a = self.inner.auth_lock();
            (a.phone.clone(), a.code_hash.clone())
        };
        let (Some(phone), Some(hash)) = (phone, hash) else { return Err(BackendError::InvalidInput("phone")) };
        let code: String = code.chars().filter(|c| c.is_ascii_alphanumeric()).collect();
        if code.is_empty() {
            return Err(BackendError::InvalidInput("code"));
        }
        let req = tl::functions::auth::SignIn { phone_number: phone, phone_code_hash: hash, phone_code: Some(code), email_verification: None };
        match self.inner.net.invoke(&req).await {
            Ok(a) => self.inner.complete_login(a).await,
            Err(e) if e.is("SESSION_PASSWORD_NEEDED") => {
                let hint = self.inner.fetch_password().await?;
                Ok(SignInResult::PasswordNeeded { hint })
            }
            Err(e) => Err(map_error(&e)),
        }
    }

    async fn check_password(&self, password: String) -> BackendResult<User> {
        let mut info = self.inner.auth_lock().password.clone();
        if info.is_none() {
            self.inner.fetch_password().await?;
            info = self.inner.auth_lock().password.clone();
        }
        let info = info.ok_or(BackendError::InvalidInput("password"))?;
        let Srp { salt1, salt2, p, g, srp_b, srp_id } = srp_params(&info).ok_or_else(|| BackendError::Crypto("algoritmo de contraseña desconocido".into()))?;
        // `check_p_and_g` entra en pánico con g fuera de 2..=7: se filtra antes.
        if !(2..=7).contains(&g) || !check_p_and_g(&p, &g) {
            return Err(BackendError::Crypto("parámetros de la contraseña no válidos".into()));
        }
        let (m1, g_a) = calculate_2fa(&salt1, &salt2, &p, &g, srp_b, info.secure_random.clone(), password.as_bytes());
        drop(password);
        let req = tl::functions::auth::CheckPassword {
            password: tl::types::InputCheckPasswordSrp { srp_id, a: g_a.to_vec(), m1: m1.to_vec() }.into(),
        };
        match self.inner.net.invoke(&req).await {
            Ok(a) => match self.inner.complete_login(a).await? {
                SignInResult::LoggedIn(u) => Ok(u),
                _ => Err(BackendError::Internal),
            },
            Err(e) => {
                let err = map_error(&e);
                if err == BackendError::InvalidInput("password") {
                    // Los parámetros SRP son de un solo uso: pedir unos nuevos para el siguiente intento.
                    let _ = self.inner.fetch_password().await;
                }
                Err(err)
            }
        }
    }

    async fn qr_login_start(&self) -> BackendResult<QrToken> {
        let tok = self.inner.export_qr().await?.ok_or(BackendError::InvalidInput("already_logged_in"))?;
        let start = {
            let mut a = self.inner.auth_lock();
            !std::mem::replace(&mut a.qr_running, true)
        };
        if start {
            let inner = Arc::clone(&self.inner);
            let expires = tok.expires.0;
            tokio::spawn(inner.qr_loop(expires));
        }
        Ok(tok)
    }

    async fn log_out(&self) -> BackendResult<()> {
        // Aunque falle en el servidor (sin red), la sesión local se borra.
        let res = self.inner.net.invoke(&tl::functions::auth::LogOut {}).await;
        if let Err(e) = &res {
            tracing::warn!(error = %map_error(e), "auth.logOut falló; se borra la sesión local igualmente");
        }
        let home = self.inner.net.session.home_dc();
        self.inner.session_lost().await;
        self.inner.net.transport.disconnect_from_dc(home);
        Ok(())
    }

    fn auth_events(&self) -> BoxStream<'static, AuthState> {
        let rx = self.inner.auth_tx.subscribe();
        stream::unfold(rx, |mut rx| async move {
            loop {
                match rx.recv().await {
                    Ok(ev) => return Some((ev, rx)),
                    Err(RecvError::Lagged(_)) => continue,
                    Err(RecvError::Closed) => return None,
                }
            }
        })
        .boxed()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn qr_url_is_unpadded_base64url() {
        assert_eq!(qr_url(b""), "tg://login?token=");
        assert_eq!(qr_url(b"f"), "tg://login?token=Zg");
        assert_eq!(qr_url(b"fo"), "tg://login?token=Zm8");
        assert_eq!(qr_url(b"foo"), "tg://login?token=Zm9v");
        assert_eq!(qr_url(&[0xfb, 0xff]), "tg://login?token=-_8");
    }

    #[test]
    fn phone_validation() {
        assert!(valid_phone("+34600111222"));
        assert!(!valid_phone("+34 600"));
        assert!(!valid_phone("abc"));
    }

    #[test]
    fn code_types_map() {
        let t = tl::enums::auth::SentCodeType::App(tl::types::auth::SentCodeTypeApp { length: 5 });
        assert_eq!(code_kind(&t), (CodeKind::App, Some(5)));
        assert_eq!(next_kind(&tl::enums::auth::CodeType::Call), CodeKind::Call);
    }
}
