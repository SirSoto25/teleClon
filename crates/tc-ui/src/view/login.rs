//! Inicio de sesión (§8 «Inicio de sesión», S01–S06): tarjeta de 400 px centrada sobre `bg` con el
//! patrón de curvas de nivel; título `display`, campos de 40 px, botón primario de 44 a todo el ancho.
//! Teclado: Enter envía, Tab cambia de campo, Esc vuelve al teléfono.

use iced::alignment::{Horizontal, Vertical};
use iced::widget::{Space, button, column, container, qr_code, row, scrollable, stack, text_input};
use iced::{Alignment, Element, Length, Padding};
use tc_theme::tokens::{layout, radius, space, text as ts};

use crate::app::{App, Msg};
use crate::icon::{Icon, icon};
use crate::login::{self, CODE_ID, Field, LoginMsg, PASSWORD_ID, PHONE_ID, Step};
use crate::style::{self, Brasa, c, with_weight};
use crate::widgets::{contour_bg, hfill, t, t1, vgap};

const CARD_W: f32 = 400.0;

fn lm(m: LoginMsg) -> Msg {
    Msg::Login(m)
}

/// Pantalla de inicio de sesión.
pub fn view(app: &App) -> Element<'_, Msg> {
    let b = app.brasa;
    let l = &app.login;
    let body: Vec<Element<'_, Msg>> = match &l.step {
        Step::Checking => vec![
            title(b, "Comprobando la sesión…"),
            t("Un momento: miramos si ya habías entrado en este equipo.", ts::UI, c(b.p.text_muted)).into(),
        ],
        Step::Phone => phone(app),
        Step::Code(req) => code(app, req),
        Step::Password { hint } => password(app, hint.as_deref()),
        Step::Qr(tok) => qr(app, tok),
        Step::SignUp => vec![
            title(b, "Ese número no tiene cuenta"),
            t(
                "Parlo no crea cuentas nuevas. Regístrate desde la app de tu teléfono y vuelve a entrar aquí con el mismo número.",
                ts::UI,
                c(b.p.text_muted),
            )
            .into(),
            vgap(space::S2).into(),
            primary(b, "Usar otro número", false, Some(lm(LoginMsg::UsePhone))),
        ],
    };
    card_screen(app, body, true)
}

/// Pantalla mientras se crea el backend (red real) o si falló.
pub fn connecting(app: &App) -> Element<'_, Msg> {
    let b = app.brasa;
    let body = match &app.connect_error {
        None => vec![title(b, "Conectando…"), t("Preparando la conexión segura con la red.", ts::UI, c(b.p.text_muted)).into()],
        Some(e) => vec![
            title(b, "No se pudo conectar"),
            notice(b, Icon::Warning, e.clone(), b.p.danger),
            vgap(space::S2).into(),
            primary(b, "Reintentar", false, Some(Msg::RetryConnect)),
        ],
    };
    card_screen(app, body, false)
}

fn card_screen<'a>(app: &'a App, body: Vec<Element<'a, Msg>>, footer: bool) -> Element<'a, Msg> {
    let b = app.brasa;
    let mark = container(t1("tc", with_weight(ts::TITLE, 700), c(b.p.on_accent)))
        .width(Length::Fixed(40.0))
        .height(Length::Fixed(40.0))
        .align_x(Horizontal::Center)
        .align_y(Vertical::Center)
        .style(style::rounded_fill(c(b.p.accent), style::avatar_radius(40.0)));
    let mut col = column![mark, vgap(space::S2)].spacing(space::S3);
    for el in body {
        col = col.push(el);
    }
    let card = container(col).padding(space::S8).width(Length::Fixed(CARD_W)).style(style::raised(b, 3, radius::XL));
    let mut outer = column![card].spacing(space::S4).align_x(Alignment::Center);
    if footer {
        outer = outer.push(t1("Enter continúa · Tab cambia de campo · Esc vuelve atrás", ts::CAPTION, c(b.p.text_muted)));
        if let Some(h) = &app.login_hint {
            outer = outer.push(container(t(h.as_str(), ts::CAPTION, c(b.p.text_muted))).max_width(CARD_W));
        }
    }
    let centered = container(outer).width(Length::Fill).align_x(Horizontal::Center).padding(Padding::from([space::S8, space::S4]));
    stack![
        contour_bg(&app.bg_cache, 7, false, b),
        scrollable(container(centered).center_y(Length::Shrink).width(Length::Fill)).height(Length::Fill).style(style::thin_scroll(b)),
    ]
    .width(Length::Fill)
    .height(Length::Fill)
    .into()
}

fn title<'a>(b: Brasa, s: &'a str) -> Element<'a, Msg> {
    t(s, ts::DISPLAY, c(b.p.text)).into()
}

fn subtitle<'a>(b: Brasa, s: impl Into<String>) -> Element<'a, Msg> {
    t(s.into(), ts::UI, c(b.p.text_muted)).into()
}

fn label<'a>(b: Brasa, s: &'a str) -> Element<'a, Msg> {
    t1(s, ts::CAPTION, c(b.p.text_muted)).into()
}

/// Aviso con icono (espera por límite de intentos, error general).
fn notice<'a>(_b: Brasa, ic: Icon, text: String, color: tc_theme::Rgb) -> Element<'a, Msg> {
    row![icon(ic, layout::ICON_MD, c(color)), t(text, ts::UI, c(color))].spacing(space::S2).align_y(Alignment::Start).into()
}

/// Botón primario grande (44 px) a todo el ancho; deshabilitado (45 %) si `on` es `None`.
fn primary<'a>(b: Brasa, label: &'a str, busy: bool, on: Option<Msg>) -> Element<'a, Msg> {
    let text = if busy { "Un momento…" } else { label };
    button(container(t1(text, ts::UI_STRONG, c(b.p.on_accent))).center_x(Length::Fill).center_y(Length::Fill))
        .width(Length::Fill)
        .height(Length::Fixed(44.0))
        .padding(0)
        .style(style::primary(b, radius::MD))
        .on_press_maybe(on)
        .into()
}

/// Enlace secundario (botón fantasma con icono opcional).
fn ghost<'a>(b: Brasa, ic: Option<Icon>, label: &'a str, on: Option<Msg>) -> Element<'a, Msg> {
    // Deshabilitado (p. ej. durante la espera por límite de intentos): `text-placeholder`.
    let color = c(if on.is_some() { b.p.accent_text } else { b.p.text_placeholder });
    let mut r = row![].spacing(space::S2).align_y(Alignment::Center);
    if let Some(i) = ic {
        r = r.push(icon(i, layout::ICON_MD, color));
    }
    r = r.push(t1(label, ts::UI_STRONG, color));
    button(container(r).center_x(Length::Fill)).width(Length::Fill).padding([10, 12]).style(style::ghost(b, radius::MD)).on_press_maybe(on).into()
}

fn field<'a>(b: Brasa, id: &'static str, placeholder: &'a str, value: &'a str, error: bool) -> text_input::TextInput<'a, Msg> {
    text_input(placeholder, value)
        .id(id)
        .padding(Padding::from([10.0, 12.0]))
        .size(ts::BODY.size)
        .font(style::font_of(ts::BODY))
        .width(Length::Fill)
        .style(style::field_input(b, error))
}

fn field_error<'a>(app: &App, f: Field) -> Option<Element<'a, Msg>> {
    app.login.field_error(f).map(|e| t(e.to_string(), ts::CAPTION, c(app.brasa.p.danger)).into())
}

/// Espera por límite de intentos y error general, si los hay.
fn status<'a>(app: &App) -> Vec<Element<'a, Msg>> {
    let b = app.brasa;
    let mut v = Vec::new();
    if let Some(s) = app.login.flood_left() {
        v.push(notice(b, Icon::Timer, format!("Demasiados intentos seguidos. Podrás volver a probar en {}.", login::countdown(s)), b.p.warning));
    }
    if let Some(e) = app.login.general_error() {
        v.push(notice(b, Icon::Warning, e.to_string(), b.p.danger));
    }
    v
}

fn submit_if(app: &App, m: LoginMsg) -> Option<Msg> {
    app.login.can_submit().then_some(Msg::Login(m))
}

fn phone(app: &App) -> Vec<Element<'_, Msg>> {
    let b = app.brasa;
    let l = &app.login;
    let err = l.field_error(Field::Phone).is_some();
    let mut v = vec![
        title(b, "Entra en tu cuenta"),
        subtitle(b, "Escribe tu número de teléfono con el prefijo del país. Te enviaremos un código para confirmar que es tuyo."),
        vgap(space::S1).into(),
        label(b, "Número de teléfono"),
        field(b, PHONE_ID, "+34 600 000 000", &l.phone, err)
            .on_input(|s| lm(LoginMsg::PhoneChanged(s)))
            .on_submit(lm(LoginMsg::SubmitPhone))
            .into(),
    ];
    v.extend(field_error(app, Field::Phone));
    v.extend(status(app));
    v.push(vgap(space::S1).into());
    v.push(primary(b, "Continuar", l.busy, submit_if(app, LoginMsg::SubmitPhone)));
    v.push(divider(b));
    let qr_on = (!l.busy && l.flood_left().is_none()).then_some(lm(LoginMsg::UseQr));
    v.push(ghost(b, None, "Entrar con un código QR", qr_on));
    v
}

fn divider<'a>(b: Brasa) -> Element<'a, Msg> {
    let line = || container(Space::new().height(Length::Fixed(1.0)).width(Length::Fill)).style(style::fill(c(b.p.border)));
    row![line(), t1("o", ts::CAPTION, c(b.p.text_muted)), line()].spacing(space::S3).align_y(Alignment::Center).into()
}

fn code<'a>(app: &'a App, req: &'a tc_core::CodeRequest) -> Vec<Element<'a, Msg>> {
    let b = app.brasa;
    let l = &app.login;
    let err = l.field_error(Field::Code).is_some();
    let placeholder = req.length.map_or_else(|| "Código".to_string(), |n| "·".repeat(n as usize));
    let mut v = vec![
        title(b, "Escribe el código"),
        subtitle(b, login::code_where(req)),
        vgap(space::S1).into(),
        label(b, "Código"),
        text_input(&placeholder, &l.code)
            .id(CODE_ID)
            .padding(Padding::from([8.0, 12.0]))
            .size(ts::HEADING.size)
            .font(style::font_of(ts::CODE))
            .width(Length::Fill)
            .style(style::field_input(b, err))
            .on_input(|s| lm(LoginMsg::CodeChanged(s)))
            .on_submit(lm(LoginMsg::SubmitCode))
            .into(),
    ];
    v.extend(field_error(app, Field::Code));
    v.extend(status(app));
    v.push(vgap(space::S1).into());
    v.push(primary(b, "Entrar", l.busy, submit_if(app, LoginMsg::SubmitCode)));
    match (l.resend_left(), req.next) {
        (Some(s), Some(_)) => {
            v.push(container(t1(format!("Podrás pedir otro código en {}", login::countdown(s)), ts::META, c(b.p.text_muted))).center_x(Length::Fill).into())
        }
        (None, Some(next)) => v.push(ghost(b, None, login::resend_label(next), (!l.busy).then_some(lm(LoginMsg::Resend)))),
        _ => {}
    }
    v.push(ghost(b, Some(Icon::Back), "Cambiar de número", (!l.busy).then_some(lm(LoginMsg::UsePhone))));
    v
}

fn password<'a>(app: &'a App, hint: Option<&'a str>) -> Vec<Element<'a, Msg>> {
    let b = app.brasa;
    let l = &app.login;
    let err = l.field_error(Field::Password).is_some();
    let mut v = vec![
        title(b, "Verificación en dos pasos"),
        subtitle(b, "Tu cuenta tiene una contraseña adicional. Se comprueba sin enviarla en claro."),
        vgap(space::S1).into(),
    ];
    let mut head = row![label(b, "Contraseña"), hfill()].align_y(Alignment::Center);
    if let Some(h) = hint {
        head = head.push(t1(format!("Pista: {h}"), ts::CAPTION, c(b.p.text_muted)));
    }
    v.push(head.into());
    let input = field(b, PASSWORD_ID, "Contraseña", &l.password, err)
        .secure(!l.show_password)
        .on_input(|s| lm(LoginMsg::PasswordChanged(s)))
        .on_submit(lm(LoginMsg::SubmitPassword));
    let toggle = button(t1(if l.show_password { "Ocultar" } else { "Mostrar" }, ts::UI_STRONG, c(b.p.accent_text)))
        .padding([10, 12])
        .style(style::ghost(b, radius::MD))
        .on_press(lm(LoginMsg::TogglePassword));
    v.push(row![input, toggle].spacing(space::S2).align_y(Alignment::Center).into());
    v.extend(field_error(app, Field::Password));
    v.extend(status(app));
    v.push(vgap(space::S1).into());
    v.push(primary(b, "Entrar", l.busy, submit_if(app, LoginMsg::SubmitPassword)));
    v.push(ghost(b, Some(Icon::Back), "Cambiar de número", (!l.busy).then_some(lm(LoginMsg::UsePhone))));
    v
}

fn qr<'a>(app: &'a App, tok: &'a tc_core::QrToken) -> Vec<Element<'a, Msg>> {
    let b = app.brasa;
    let steps = column![
        t("1. Abre la app en el teléfono donde ya tienes la sesión iniciada.", ts::UI, c(b.p.text)),
        t("2. Entra en Ajustes › Dispositivos › Vincular un dispositivo.", ts::UI, c(b.p.text)),
        t("3. Apunta la cámara a este código.", ts::UI, c(b.p.text)),
    ]
    .spacing(space::S1);
    let code: Element<'a, Msg> = match &app.qr {
        Some(data) => container(qr_code(data).cell_size(5).style(|_| qr_code::Style { cell: style::QR_INK, background: style::QR_PAPER }))
            .padding(space::S4)
            .style(style::rounded_fill(style::QR_PAPER, style::r(radius::MD)))
            .into(),
        None => container(t1("Preparando el código…", ts::UI, c(b.p.text_muted))).padding(space::S8).into(),
    };
    let left = (tok.expires.0 - crate::app::now().0).max(0) as u64;
    let mut v = vec![
        title(b, "Entra con un código QR"),
        steps.into(),
        container(code).center_x(Length::Fill).into(),
        container(t1(
            if left > 0 { format!("El código se renueva solo dentro de {}.", login::countdown(left)) } else { "Renovando el código…".to_string() },
            ts::META,
            c(b.p.text_muted),
        ))
        .center_x(Length::Fill)
        .into(),
    ];
    v.extend(status(app));
    v.push(ghost(b, Some(Icon::Back), "Entrar con el número de teléfono", Some(lm(LoginMsg::UsePhone))));
    v
}
