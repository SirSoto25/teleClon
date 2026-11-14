//! Diálogo «Notificaciones» (S28, fase C2): activar o desactivar los avisos, por tipo de chat, qué
//! enseñan (vista previa y chats secretos), sonido, archivados, «se unió a Telegram» y «No molestar».
//! Lógica y persistencia en `app/notify.rs`.

use iced::widget::{Column, button, checkbox, column, container, row};
use iced::{Alignment, Element, Length};
use tc_core::Timestamp;
use tc_notify::rules::{DoNotDisturb, Preview};
use tc_theme::tokens::{layout, radius, space, text as ts};

use crate::app::notify::{DndChoice, NotifyMsg, Toggle};
use crate::app::{App, Msg};
use crate::format;
use crate::icon::{Icon, icon};
use crate::style::{self, c};
use crate::widgets::{hfill, t, t1};

fn nm(m: NotifyMsg) -> Msg {
    Msg::Notify(m)
}

/// Opciones de vista previa, en el orden del diálogo.
pub const PREVIEWS: [(Preview, &str); 3] = [(Preview::Full, "Nombre y texto"), (Preview::NameOnly, "Solo el nombre"), (Preview::Hidden, "Nada")];

fn check<'a>(app: &App, label: &'a str, value: bool, on: Option<fn(bool) -> Msg>) -> Element<'a, Msg> {
    let b = app.brasa;
    let mut cb = checkbox(value).label(label).font(style::font_of(ts::UI)).text_size(ts::UI.size).style(style::check(b));
    if let Some(f) = on {
        cb = cb.on_toggle(f);
    }
    cb.into()
}

fn chip<'a>(app: &App, label: &'a str, active: bool, msg: Option<Msg>) -> Element<'a, Msg> {
    let b = app.brasa;
    let mut inner = row![].spacing(space::S1).align_y(Alignment::Center);
    if active {
        inner = inner.push(icon(Icon::Check, layout::ICON_SM, c(b.p.accent_text)));
    }
    inner = inner.push(t1(label, ts::UI, c(if msg.is_some() || active { b.p.text } else { b.p.text_muted })));
    button(container(inner).center_y(Length::Fixed(28.0))).padding([0, 12]).style(style::filter_chip(b, active)).on_press_maybe(msg).into()
}

fn section<'a>(app: &App, title: &'a str) -> Element<'a, Msg> {
    t1(title, ts::UI_STRONG, c(app.brasa.p.text)).into()
}

/// Opción de «No molestar» que corresponde al estado actual (un plazo vencido cuenta como «No»).
pub fn dnd_choice(d: DoNotDisturb, now: i64) -> Option<DndChoice> {
    match d {
        DoNotDisturb::Off => Some(DndChoice::Off),
        DoNotDisturb::On => Some(DndChoice::On),
        DoNotDisturb::Until(t) if t <= now => Some(DndChoice::Off),
        DoNotDisturb::Until(_) => None,
    }
}

/// Texto de estado de «No molestar».
pub fn dnd_status(app: &App, d: DoNotDisturb, now: i64) -> Option<String> {
    match d {
        DoNotDisturb::On => Some("No molestar activado: no saldrá ningún aviso hasta que lo quites.".into()),
        DoNotDisturb::Until(t) if t > now => {
            let when = format::hhmm(Timestamp(t), &app.tz);
            let day = if format::day_key(Timestamp(t), &app.tz) != format::day_key(Timestamp(now), &app.tz) { " de mañana" } else { "" };
            Some(format!("No molestar hasta las {when}{day}."))
        }
        _ => None,
    }
}

pub fn settings_panel(app: &App) -> Element<'_, Msg> {
    let b = app.brasa;
    let s = app.notifs.settings();
    let on = s.enabled;
    let now = app.notifs.now();
    let muted = c(b.p.text_muted);
    // Con los avisos apagados, el resto se ve pero no se toca.
    let gate = |f: fn(bool) -> Msg| on.then_some(f);

    let kinds = row![
        check(app, "Chats privados", s.private, gate(|v| nm(NotifyMsg::Set(Toggle::Private, v)))),
        check(app, "Grupos", s.groups, gate(|v| nm(NotifyMsg::Set(Toggle::Groups, v)))),
        check(app, "Canales", s.channels, gate(|v| nm(NotifyMsg::Set(Toggle::Channels, v)))),
    ]
    .spacing(space::S5);

    let mut previews = row![].spacing(space::S2);
    for (p, label) in PREVIEWS {
        previews = previews.push(chip(app, label, s.preview == p, on.then_some(nm(NotifyMsg::Preview(p)))));
    }
    let secret_text = s.secret_preview == Preview::Full;

    let current = dnd_choice(s.dnd, now);
    let mut dnd = row![].spacing(space::S2);
    for ch in DndChoice::ALL {
        dnd = dnd.push(chip(app, ch.label(), current == Some(ch), Some(nm(NotifyMsg::Dnd(ch)))));
    }

    let mut col: Column<'_, Msg> = column![
        row![icon(Icon::Bell, layout::ICON_LG, c(b.p.accent_text)), t1("Notificaciones", ts::TITLE, c(b.p.text))].spacing(space::S2).align_y(Alignment::Center),
        t("Parlo te avisa con una notificación del sistema cuando llega un mensaje y no estás mirando ese chat.", ts::UI, muted),
        check(app, "Avisar de mensajes nuevos", on, Some(|v| nm(NotifyMsg::Set(Toggle::Enabled, v)))),
        section(app, "Avisar de"),
        kinds,
        section(app, "Qué se ve en el aviso"),
        previews,
        check(app, "Mostrar también el texto de los chats secretos", secret_text, gate(|v| nm(NotifyMsg::Set(Toggle::SecretText, v)))),
        t("Sin marcar, un chat secreto solo enseña su nombre, ni siquiera en la pantalla de bloqueo.", ts::CAPTION, muted),
        section(app, "Más opciones"),
        check(app, "Sonido", s.sound, gate(|v| nm(NotifyMsg::Set(Toggle::Sound, v)))),
        check(app, "Avisar también de chats archivados (si no están silenciados)", s.archived, gate(|v| nm(NotifyMsg::Set(Toggle::Archived, v)))),
        check(app, "Avisar cuando un contacto se une a Telegram", !s.ignore_contact_joined, gate(|v| nm(NotifyMsg::Set(Toggle::IgnoreJoined, !v)))),
        section(app, "No molestar"),
        dnd,
    ]
    .spacing(space::S3);
    if let Some(st) = dnd_status(app, s.dnd, now) {
        col = col.push(t(st, ts::CAPTION, c(b.p.warning)));
    }
    col.push(row![
        hfill(),
        button(t1("Cerrar", ts::UI_STRONG, c(b.p.on_accent))).padding([8, 16]).style(style::primary(b, radius::MD)).on_press(Msg::CloseOverlay),
    ])
    .into()
}
