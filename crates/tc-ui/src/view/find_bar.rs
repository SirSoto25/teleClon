//! Barra de búsqueda dentro del chat (§8.3 «modo búsqueda», fase B2): sustituye a la cabecera
//! mientras está abierta. Campo con lupa, estado («Buscando…», «Sin resultados», «3 de 17» o el
//! error con «Reintentar»), flechas anterior/siguiente, cerrar y una fila de chips de tipo.

use iced::alignment::Vertical;
use iced::widget::{Space, button, column, container, row, text_input, tooltip};
use iced::{Alignment, Element, Length, Padding};
use tc_core::SearchFilter;
use tc_theme::tokens::{layout, radius, space, text as ts};

use crate::app::find::{ChatSearch, FIELD_ID, FILTERS, FindMsg, FindState, filter_label};
use crate::app::{App, Msg};
use crate::conv::Conversation;
use crate::icon::{Icon, icon, icon_turned};
use crate::style::{self, Brasa, c, with_weight};
use crate::widgets::{hfill, t1};

use super::conversation::in_column;

fn find(m: FindMsg) -> Msg {
    Msg::Find(m)
}

/// Altura de la fila de chips.
const CHIP_H: f32 = 28.0;

fn tip<'a>(el: impl Into<Element<'a, Msg>>, label: &'a str, b: Brasa) -> Element<'a, Msg> {
    let tip = container(t1(label, ts::CAPTION, c(b.p.text))).padding([4, 8]).style(style::raised(b, 2, radius::SM));
    tooltip(el, tip, tooltip::Position::Bottom).into()
}

/// Botón de flecha (`half_turns` 1 = arriba) apagado si no hay a dónde ir.
fn arrow<'a>(b: Brasa, half_turns: u8, on: Option<Msg>, label: &'a str) -> Element<'a, Msg> {
    let color = c(if on.is_some() { b.p.text } else { b.p.text_placeholder });
    let btn = button(icon_turned(Icon::Down, layout::ICON_MD, color, half_turns)).padding(8).style(style::ghost(b, radius::SM)).on_press_maybe(on);
    tip(btn, label, b)
}

fn status<'a>(f: &ChatSearch, b: Brasa) -> Element<'a, Msg> {
    let muted = |s: &'static str| -> Element<'a, Msg> { t1(s, ts::UI, c(b.p.text_muted)).into() };
    match &f.state {
        FindState::Idle => Space::new().width(Length::Shrink).into(),
        FindState::Loading => muted("Buscando…"),
        FindState::Failed(e) => row![
            t1(e.clone(), ts::UI, c(b.p.danger)),
            button(t1("Reintentar", ts::UI_STRONG, c(b.p.accent_text))).padding([4, 8]).style(style::ghost(b, radius::SM)).on_press(find(FindMsg::Retry)),
        ]
        .spacing(space::S1)
        .align_y(Alignment::Center)
        .into(),
        FindState::Ready if f.hits.is_empty() => muted("Sin resultados"),
        FindState::Ready => match f.counter() {
            Some(n) => t1(n, ts::UI_STRONG, c(b.p.text)).into(),
            None => Space::new().width(Length::Shrink).into(),
        },
    }
}

fn chip<'a>(f: &ChatSearch, filter: SearchFilter, i: usize, b: Brasa) -> Element<'a, Msg> {
    let active = f.filter == filter;
    let mut inner = row![].spacing(space::S1).align_y(Alignment::Center);
    if active {
        inner = inner.push(icon(Icon::Check, 14.0, c(b.p.accent_text)));
    }
    let weight = if active { with_weight(ts::CAPTION, 600) } else { ts::CAPTION };
    inner = inner.push(t1(filter_label(filter), weight, c(b.p.text)));
    let btn = button(container(inner).center_y(Length::Fill))
        .height(Length::Fixed(CHIP_H))
        .padding(Padding::from([0.0, space::S3]))
        .style(style::filter_chip(b, active))
        .on_press(find(FindMsg::Filter(filter)));
    let label: &'static str = ["Alt+1", "Alt+2", "Alt+3", "Alt+4", "Alt+5", "Alt+6"][i.min(5)];
    tip(btn, label, b)
}

/// Barra completa (en lugar de la cabecera), si la búsqueda está abierta en esta conversación.
pub fn view<'a>(app: &'a App, cv: &'a Conversation) -> Option<Element<'a, Msg>> {
    let f = app.find_active().filter(|f| f.peer == cv.peer)?;
    let b = app.brasa;
    let placeholder = if f.thread.is_some() { "Buscar en este tema" } else { "Buscar en este chat" };
    let input = text_input(placeholder, &f.text)
        .id(FIELD_ID)
        .on_input(|s| find(FindMsg::Input(s)))
        .padding(0)
        .size(ts::UI.size)
        .font(style::font_of(ts::UI))
        .style(style::bare_input(b));
    let field = container(row![icon(Icon::Search, layout::ICON_SM, c(b.p.text_placeholder)), input].spacing(space::S2).align_y(Alignment::Center))
        .padding(Padding::from([0.0, space::S3]))
        .height(Length::Fixed(36.0))
        .width(Length::Fill)
        .align_y(Vertical::Center)
        .style(style::sunken_field(b, true, radius::SM));
    let older = f.has_older().then_some(find(FindMsg::Older));
    let newer = f.has_newer().then_some(find(FindMsg::Newer));
    let close = button(icon(Icon::Close, layout::ICON_MD, c(b.p.text))).padding(8).style(style::ghost(b, radius::SM)).on_press(find(FindMsg::Close));
    let top = row![
        field,
        status(f, b),
        arrow(b, 1, older, "Anterior (↑ o Enter)"),
        arrow(b, 0, newer, "Siguiente (↓ o Mayús+Enter)"),
        tip(close, "Cerrar (Esc)", b),
    ]
    .spacing(space::S2)
    .align_y(Alignment::Center);
    let mut chips = row![].spacing(space::S2).align_y(Alignment::Center);
    for (i, filter) in FILTERS.into_iter().enumerate() {
        chips = chips.push(chip(f, filter, i, b));
    }
    let bar = column![
        in_column(top).height(Length::Fixed(layout::HEADER_HEIGHT)).align_y(Vertical::Center),
        in_column(row![chips, hfill()]).padding(Padding { top: 0.0, bottom: space::S2, left: crate::conv::SIDE_PAD, right: crate::conv::SIDE_PAD }),
    ];
    let hair = container(Space::new().height(Length::Fixed(1.0)).width(Length::Fill)).style(style::fill(c(b.p.border)));
    Some(column![container(bar).style(style::surface(b)), hair].into())
}
