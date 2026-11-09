//! Pruebas de `update` del inicio de sesión, los stickers y los chats secretos contra el
//! `MockBackend` (sin ventana): como en `tests.rs`, la prueba hace la llamada al backend y entrega
//! el resultado como lo haría el runtime de iced.

use std::sync::Arc;
use std::time::Duration;

use tc_core::*;
use tc_mock::{ANIMATED_SET_ID, BIG_CHAT, ME, MOCK_2FA_PASSWORD, MOCK_FLOOD_SECS, MOCK_LOGIN_CODE, MockBackend, MockConfig};

use crate::app::{App, Auth, Backend, Connector, Flags, Msg, Overlay, Phase, Session};
use crate::login::{Field, LoginMsg, Step};
use crate::stickers::{self, Nav, Tab};

const NOW: i64 = 1_762_430_400;

fn lm(m: LoginMsg) -> Msg {
    Msg::Login(m)
}

/// App en la pantalla de login sobre un mock sin sesión.
async fn boot_logged_out() -> (App, Auth) {
    let mock = Arc::new(MockBackend::new(MockConfig { now: Some(NOW), logged_in: false, ..Default::default() }));
    let backend: Backend = mock.clone();
    let auth: Auth = mock;
    let (mut app, _) = App::new(Flags { auth: Some(auth.clone()), ..Flags::new(backend) });
    assert_eq!(app.phase, Phase::Auth);
    assert_eq!(app.login.step, Step::Checking);
    let _ = app.update(lm(LoginMsg::State(auth.auth_state().await)));
    assert_eq!(app.login.step, Step::Phone);
    (app, auth)
}

/// App ya dentro, con diálogos, contactos, secretos y stickers cargados.
async fn boot_main() -> (App, Arc<MockBackend>, Backend) {
    let mock = Arc::new(MockBackend::new(MockConfig { now: Some(NOW), ..Default::default() }));
    let backend: Backend = mock.clone();
    let (mut app, _) = App::new(Flags::new(backend.clone()));
    assert_eq!(app.phase, Phase::Main);
    let _ = app.update(Msg::Me(backend.me().await));
    let _ = app.update(Msg::Contacts(backend.contacts().await));
    let _ = app.update(Msg::Secrets(backend.secret_chats().await));
    let _ = app.update(Msg::DialogsLoaded(backend.dialogs(DialogsPage { offset: 0, limit: 500, archived: false }).await));
    let _ = app.update(Msg::StickerSets(backend.installed_sticker_sets().await));
    for g in 5000..5008 {
        let _ = app.update(Msg::GroupLoaded(backend.group(GroupId(g)).await));
    }
    (app, mock, backend)
}

async fn open(app: &mut App, backend: &Backend, peer: PeerRef) {
    let _ = app.update(Msg::Open(peer));
    let r = backend.history(peer, Anchor::Latest, crate::conv::PAGE).await;
    let _ = app.update(Msg::History { peer, older: false, result: r });
}

// ------------------------------------------------------------------ inicio de sesión

#[tokio::test]
async fn login_telefono_espera_codigo_y_2fa() {
    let (mut app, auth) = boot_logged_out().await;

    // Número sin prefijo: se avisa en el campo sin llamar al backend.
    let _ = app.update(lm(LoginMsg::PhoneChanged("600 000".into())));
    let _ = app.update(lm(LoginMsg::SubmitPhone));
    assert!(app.login.field_error(Field::Phone).is_some());
    assert!(!app.login.busy);
    let _ = app.update(lm(LoginMsg::PhoneChanged("600 000 1".into())));
    assert!(app.login.field_error(Field::Phone).is_none(), "escribir borra el error");

    // Límite de intentos: cuenta atrás y botón bloqueado hasta que pase.
    let _ = app.update(lm(LoginMsg::PhoneChanged("+34 600 000 009".into())));
    let _ = app.update(lm(LoginMsg::SubmitPhone));
    assert!(app.login.busy);
    let _ = app.update(lm(LoginMsg::CodeSent(auth.request_login_code("+34600000009".into()).await)));
    assert_eq!(app.login.flood_left(), Some(u64::from(MOCK_FLOOD_SECS)));
    assert!(!app.login.busy && !app.login.can_submit());
    let _ = app.update(lm(LoginMsg::SubmitPhone));
    assert!(!app.login.busy, "no se envía durante la espera");
    let later = app.login.now + Duration::from_secs(u64::from(MOCK_FLOOD_SECS) + 1);
    let _ = app.update(lm(LoginMsg::Tick(later)));
    assert_eq!(app.login.flood_left(), None);
    assert!(app.login.flood_until.is_none());

    // Teléfono con 2FA (termina en 2).
    let _ = app.update(lm(LoginMsg::PhoneChanged("+34 600 000 002".into())));
    let _ = app.update(lm(LoginMsg::SubmitPhone));
    let _ = app.update(lm(LoginMsg::CodeSent(auth.request_login_code("+34600000002".into()).await)));
    let Step::Code(req) = app.login.step.clone() else { panic!("paso del código") };
    assert_eq!(req.length, Some(5));
    assert_eq!(app.login.resend_left(), Some(60));
    // El mismo estado por `auth_events` no borra lo ya escrito.
    let _ = app.update(lm(LoginMsg::CodeChanged("9a9".into())));
    assert_eq!(app.login.code, "99");
    let _ = app.update(lm(LoginMsg::Event(AuthState::WaitingCode(req.clone()))));
    assert_eq!(app.login.code, "99");

    // Código incorrecto: error en el campo y campo vacío.
    let _ = app.update(lm(LoginMsg::SubmitCode));
    assert!(app.login.busy);
    let _ = app.update(lm(LoginMsg::SignedIn(auth.sign_in("99".into()).await)));
    assert!(app.login.field_error(Field::Code).is_some());
    assert!(app.login.code.is_empty() && !app.login.busy);

    // Con la longitud conocida, el código completo se envía solo.
    let _ = app.update(lm(LoginMsg::CodeChanged(MOCK_LOGIN_CODE.into())));
    assert!(app.login.busy, "envío automático");
    let _ = app.update(lm(LoginMsg::SignedIn(auth.sign_in(MOCK_LOGIN_CODE.into()).await)));
    assert!(matches!(&app.login.step, Step::Password { hint: Some(_) }));

    // Contraseña vacía: no se envía. Incorrecta: error. Correcta: dentro.
    let _ = app.update(lm(LoginMsg::SubmitPassword));
    assert!(!app.login.busy);
    let _ = app.update(lm(LoginMsg::PasswordChanged("mala".into())));
    let _ = app.update(lm(LoginMsg::TogglePassword));
    assert!(app.login.show_password);
    let _ = app.update(lm(LoginMsg::SubmitPassword));
    let _ = app.update(lm(LoginMsg::PasswordChecked(auth.check_password("mala".into()).await)));
    assert!(app.login.field_error(Field::Password).is_some());
    assert!(app.login.password.is_empty());
    let _ = app.update(lm(LoginMsg::PasswordChanged(MOCK_2FA_PASSWORD.into())));
    let _ = app.update(lm(LoginMsg::SubmitPassword));
    let _ = app.update(lm(LoginMsg::PasswordChecked(auth.check_password(MOCK_2FA_PASSWORD.into()).await)));
    assert_eq!(app.phase, Phase::Main);
    assert_eq!(app.me_id(), Some(ME));
    assert!(app.login.password.is_empty(), "la contraseña no se queda en memoria");
}

#[tokio::test]
async fn login_por_qr_numero_sin_cuenta_y_escape() {
    let (mut app, auth) = boot_logged_out().await;

    // Número sin cuenta (termina en 0) → aviso de registro; Esc vuelve al teléfono.
    let _ = app.update(lm(LoginMsg::PhoneChanged("+34600000010".into())));
    let _ = app.update(lm(LoginMsg::SubmitPhone));
    let _ = app.update(lm(LoginMsg::CodeSent(auth.request_login_code("+34600000010".into()).await)));
    let _ = app.update(lm(LoginMsg::CodeChanged(MOCK_LOGIN_CODE.into())));
    let _ = app.update(lm(LoginMsg::SignedIn(auth.sign_in(MOCK_LOGIN_CODE.into()).await)));
    assert_eq!(app.login.step, Step::SignUp);
    let _ = app.update(Msg::Escape);
    assert_eq!(app.login.step, Step::Phone);
    assert_eq!(app.login.phone, "+34600000010", "se conserva el número");

    // QR: se dibuja el token y la aceptación llega por `auth_events`.
    let _ = app.update(lm(LoginMsg::UseQr));
    assert!(app.login.busy);
    let _ = app.update(lm(LoginMsg::QrReady(auth.qr_login_start().await)));
    let Step::Qr(tok) = app.login.step.clone() else { panic!("paso QR") };
    assert!(app.qr.is_some());
    assert_eq!(app.qr_url, tok.url);
    assert!(app.login.needs_tick(), "el QR muestra su caducidad");
    let me = auth.auth_state().await.ok();
    assert!(matches!(me, Some(AuthState::WaitingQr(_))));
    let user = tc_core::User {
        id: ME,
        first_name: "Yo".into(),
        last_name: String::new(),
        username: None,
        status: UserStatus::Online,
        is_contact: false,
        is_bot: false,
        photo: None,
    };
    let _ = app.update(lm(LoginMsg::Event(AuthState::LoggedIn(user))));
    assert_eq!(app.phase, Phase::Main);

    // Sesión revocada desde fuera: vuelta al login con el estado limpio.
    let _ = app.update(lm(LoginMsg::Event(AuthState::LoggedOut)));
    assert_eq!(app.phase, Phase::Auth);
    assert_eq!(app.login.step, Step::Phone);
    assert!(app.conv.is_none() && app.dialogs.is_empty());
}

#[tokio::test]
async fn sin_login_entra_directo_y_conector_asincrono() {
    // Sin `AuthApi`: directamente a la app.
    let (app, _, _) = boot_main().await;
    assert_eq!(app.phase, Phase::Main);

    // Con conector: fase «conectando», error con reintento y, al conectar, a la app.
    let mock = Arc::new(MockBackend::new(MockConfig { now: Some(NOW), ..Default::default() }));
    let chat: Backend = mock;
    let ok = chat.clone();
    let connect: Connector = Arc::new(move || {
        let chat = ok.clone();
        Box::pin(async move { Ok(Session { chat, auth: None }) })
    });
    let (mut app, _) = App::new(Flags::with_connector(connect));
    assert_eq!(app.phase, Phase::Connecting);
    let _ = app.update(Msg::Connected(Err("No se pudo abrir la sesión guardada.".into())));
    assert_eq!(app.phase, Phase::Connecting);
    assert!(app.connect_error.is_some());
    let _ = app.update(Msg::RetryConnect);
    assert!(app.connect_error.is_none());
    let _ = app.update(Msg::Connected(Ok(Session { chat, auth: None })));
    assert_eq!(app.phase, Phase::Main);
}

// ------------------------------------------------------------------ stickers

#[tokio::test]
async fn todos_los_stickers_del_mock_tienen_dibujo_propio() {
    let (app, _, _) = boot_main().await;
    assert_eq!(app.sticker_sets.len(), 2);
    assert_eq!(app.sticker_sets[1].id, ANIMATED_SET_ID);
    assert_eq!(stickers::ANIMATED_SET_ID, ANIMATED_SET_ID, "convenio compartido con tc-mock");
    for s in app.sticker_sets.iter().flat_map(|s| s.stickers.iter()) {
        assert!(stickers::art_index(s).is_some(), "{s:?}");
    }
}

#[tokio::test]
async fn selector_de_stickers_teclado_envio_y_recientes() {
    let (mut app, _, backend) = boot_main().await;
    open(&mut app, &backend, BIG_CHAT).await;
    let before = app.conv.as_ref().unwrap().messages.len();

    let _ = app.update(Msg::TogglePicker);
    assert!(app.picker.open);
    assert_eq!(app.picker_tabs(), vec![Tab::Set(0), Tab::Set(1)], "sin recientes todavía");
    let _ = app.update(Msg::PickerNav(Nav::Right));
    let _ = app.update(Msg::PickerNav(Nav::Right));
    assert_eq!(app.picker.cursor, Some(1));
    let id = app.picker_tab_stickers()[1].id;
    let _ = app.update(Msg::PickerNav(Nav::Activate));
    assert!(!app.picker.open, "se cierra al enviar");
    assert_eq!(app.recents.ids(), &[id]);
    let c = app.conv.as_ref().unwrap();
    assert_eq!(c.messages.len(), before + 1);
    let pending = c.messages.last().unwrap();
    assert_eq!(pending.state, SendState::Pending);
    assert_eq!(pending.sticker.as_ref().map(|s| s.id), Some(id));
    let rid = pending.random_id.unwrap();
    let r = backend.send_sticker(BIG_CHAT, id, None, rid).await;
    let _ = app.update(Msg::Sent { peer: BIG_CHAT, random_id: rid, result: r });
    let last = app.conv.as_ref().unwrap().messages.last().unwrap().clone();
    assert_eq!((last.state, last.sticker.map(|s| s.id)), (SendState::Sent, Some(id)));
    assert_eq!(app.conv.as_ref().unwrap().messages.len(), before + 1, "el confirmado sustituye al pendiente");

    // Reabrir: «Recientes» primero; Tab cambia de pack; el animado pide frames salvo con movimiento reducido.
    let _ = app.update(Msg::TogglePicker);
    assert_eq!(app.picker_tabs(), vec![Tab::Recent, Tab::Set(0), Tab::Set(1)]);
    assert_eq!(app.picker_tab_stickers().len(), 1);
    assert!(!app.stickers_animating());
    let _ = app.update(Msg::PickerNav(Nav::PrevTab));
    assert_eq!(app.picker.tab, 2);
    assert!(app.stickers_animating());
    let _ = app.update(Msg::ToggleMotion);
    assert!(!app.stickers_animating(), "movimiento reducido: primer fotograma, sin frames");
    assert_eq!(app.sticker_phase(3), 0.0);
    let _ = app.update(Msg::Escape);
    assert!(!app.picker.open, "Esc cierra el selector");

    // En un canal (sin permiso de escritura) no se abre.
    let channel = app.dialogs.iter().find(|d| app.is_channel(d.peer)).unwrap().peer;
    open(&mut app, &backend, channel).await;
    let _ = app.update(Msg::TogglePicker);
    assert!(!app.picker.open);
}

#[tokio::test]
async fn sticker_fallido_se_reintenta_como_sticker() {
    let (mut app, mock, backend) = boot_main().await;
    open(&mut app, &backend, BIG_CHAT).await;
    let id = app.sticker_sets[0].stickers[0].id;
    let _ = app.update(Msg::SendSticker(id));
    let rid = app.conv.as_ref().unwrap().messages.last().unwrap().random_id.unwrap();
    mock.set_offline(true);
    let r = backend.send_sticker(BIG_CHAT, id, None, rid).await;
    let _ = app.update(Msg::Sent { peer: BIG_CHAT, random_id: rid, result: r });
    assert_eq!(app.conv.as_ref().unwrap().messages.last().unwrap().state, SendState::Failed);
    mock.set_offline(false);
    let _ = app.update(Msg::Retry(rid));
    assert_eq!(app.conv.as_ref().unwrap().messages.last().unwrap().state, SendState::Pending);
    let r = backend.send_sticker(BIG_CHAT, id, None, rid).await;
    let _ = app.update(Msg::Sent { peer: BIG_CHAT, random_id: rid, result: r });
    let last = app.conv.as_ref().unwrap().messages.last().unwrap();
    assert_eq!(last.state, SendState::Sent);
    assert!(last.sticker.is_some() && last.text.is_empty());
}

// ------------------------------------------------------------------ chats secretos

#[tokio::test]
async fn crear_chat_secreto_ttl_y_cerrar() {
    let (mut app, _, backend) = boot_main().await;

    let _ = app.update(Msg::NewSecret(None));
    assert!(matches!(app.overlay, Some(Overlay::NewSecret { user: None, busy: false, .. })));
    let _ = app.update(Msg::SecretQuery("zzzz".into()));
    let _ = app.update(Msg::ConfirmNewSecret);
    assert!(matches!(app.overlay, Some(Overlay::NewSecret { busy: false, .. })), "sin contacto no se crea");

    // Un filtro que deja un único contacto: Enter lo elige y lanza la petición.
    let target = app.secret_candidates("")[0].clone();
    let _ = app.update(Msg::SecretQuery(target.display_name()));
    assert_eq!(app.secret_candidates(&target.display_name()).len(), 1);
    let _ = app.update(Msg::ConfirmNewSecret);
    assert!(matches!(app.overlay, Some(Overlay::NewSecret { user: Some(u), busy: true, .. }) if u == target.id));
    let chat = backend.request_secret_chat(target.id).await;
    let _ = app.update(Msg::SecretCreated(chat.clone()));
    let chat = chat.unwrap();
    assert!(app.overlay.is_none());
    let peer = PeerRef::Secret(chat.id);
    assert_eq!(app.conv.as_ref().map(|c| c.peer), Some(peer));
    assert!(app.write_block(peer).unwrap().contains("Esperando"), "hasta que acepte, no se escribe");
    let _ = app.update(Msg::Backend(BackendEvent::SecretChat(SecretChat { state: SecretChatState::Accepted, ..chat.clone() })));
    assert!(app.write_block(peer).is_none());

    // TTL: diálogo, opción optimista y confirmación por evento.
    let _ = app.update(Msg::ShowTtl);
    assert_eq!(app.overlay, Some(Overlay::Ttl { confirm_discard: false }));
    let _ = app.update(Msg::SetTtl(Some(30)));
    assert!(app.overlay.is_none());
    assert_eq!(app.secrets[&chat.id].ttl_secs, Some(30));
    backend.set_secret_ttl(chat.id, Some(30)).await.unwrap();

    // Cerrar pide confirmación dentro del diálogo.
    let _ = app.update(Msg::ShowTtl);
    let _ = app.update(Msg::AskDiscardSecret);
    assert_eq!(app.overlay, Some(Overlay::Ttl { confirm_discard: true }));
    let _ = app.update(Msg::DiscardSecret);
    assert!(app.overlay.is_none());
    backend.discard_secret_chat(chat.id).await.unwrap();
    let closed = backend.secret_chats().await.unwrap().into_iter().find(|c| c.id == chat.id).unwrap();
    let _ = app.update(Msg::Backend(BackendEvent::SecretChat(closed)));
    assert!(app.write_block(peer).unwrap().contains("cerró"));

    // Desde la cabecera de un chat privado el contacto llega ya elegido.
    let _ = app.update(Msg::NewSecret(Some(target.id)));
    assert!(matches!(app.overlay, Some(Overlay::NewSecret { user: Some(u), .. }) if u == target.id));
}

#[tokio::test]
async fn invitacion_entrante_se_acepta() {
    let (mut app, _, backend) = boot_main().await;
    let user = app.secret_candidates("")[1].id;
    let invite = SecretChat { id: SecretChatId(77), user, state: SecretChatState::Waiting, outgoing: false, ttl_secs: None };
    let _ = app.update(Msg::Backend(BackendEvent::SecretChat(invite.clone())));
    let peer = PeerRef::Secret(invite.id);
    let _ = app.update(Msg::Open(peer));
    assert!(app.secret_invite(peer).is_some());
    assert!(app.write_block(peer).unwrap().contains("invita"));
    let _ = app.update(Msg::AcceptSecret);
    let _ = app.update(Msg::Backend(BackendEvent::SecretChat(SecretChat { state: SecretChatState::Accepted, ..invite })));
    assert!(app.secret_invite(peer).is_none() && app.write_block(peer).is_none());
    // El picker de stickers funciona también en chats secretos aceptados.
    let r = backend.history(peer, Anchor::Latest, crate::conv::PAGE).await;
    let _ = app.update(Msg::History { peer, older: false, result: r });
    let _ = app.update(Msg::TogglePicker);
    assert!(app.picker.open);
}
