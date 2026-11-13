//! Panel junto al compositor: contenido de la pestaña (Emojis, GIFs o el selector de stickers de
//! siempre) y la barra de pestañas inferior «Emojis | GIFs | Stickers».

use iced::widget::{Space, button, column, container, row};
use iced::{Alignment, Element, Length, Padding};
use tc_theme::tokens::{layout, space, text as ts};

use crate::app::{App, Msg};
use crate::icon::{Icon, icon, icon_fill};
use crate::panel::{PanelMsg, PanelTab};
use crate::stickers::PANEL_H;
use crate::style::{self, c, with_weight};
use crate::widgets::t1;

/// Alto de la barra de pestañas inferior.
pub const TABS_H: f32 = 40.0;

pub fn view(app: &App) -> Element<'_, Msg> {
    let content = match app.panel.tab {
        PanelTab::Emoji => framed(app, super::emoji_panel::view(app)),
        PanelTab::Gifs => framed(app, super::gif_panel::view(app)),
        PanelTab::Stickers => super::stickers::picker(app),
    };
    column![content, tab_bar(app)].into()
}

/// Mismo marco que el selector de stickers: filete arriba y superficie de `PANEL_H`.
fn framed<'a>(app: &'a App, inner: Element<'a, Msg>) -> Element<'a, Msg> {
    let b = app.brasa;
    let panel = container(container(inner).max_width(layout::CHAT_COLUMN_MAX))
        .padding(Padding::from([space::S2, space::S4]))
        .height(Length::Fixed(PANEL_H))
        .center_x(Length::Fill)
        .style(style::surface(b));
    let line = container(Space::new().height(Length::Fixed(1.0)).width(Length::Fill)).style(style::fill(c(b.p.border)));
    column![line, panel].into()
}

fn tab_bar(app: &App) -> Element<'_, Msg> {
    let b = app.brasa;
    let mut r = row![].spacing(space::S2).align_y(Alignment::Center);
    for tab in PanelTab::ALL {
        let active = app.panel.tab == tab;
        let col = c(if active { b.p.accent_text } else { b.p.text_muted });
        let glyph: Element<'_, Msg> = match (tab, active) {
            (PanelTab::Emoji, true) => icon_fill(Icon::Smiley, layout::ICON_SM, col),
            (PanelTab::Emoji, false) => icon(Icon::Smiley, layout::ICON_SM, col),
            (PanelTab::Stickers, true) => icon_fill(Icon::Sticker, layout::ICON_SM, col),
            (PanelTab::Stickers, false) => icon(Icon::Sticker, layout::ICON_SM, col),
            (PanelTab::Gifs, _) => t1("GIF", with_weight(ts::CAPTION, 800), col).into(),
        };
        let label = t1(tab.label(), if active { ts::UI_STRONG } else { ts::UI }, col);
        r = r.push(
            button(row![glyph, label].spacing(space::S1).align_y(Alignment::Center))
                .padding([6, 14])
                .style(style::option_row(b, active))
                .on_press(Msg::Panel(PanelMsg::Tab(tab))),
        );
    }
    let hint = t1("Alt+1/2/3 cambia de pestaña · Esc cierra", ts::CAPTION, c(b.p.text_muted));
    let inner = container(row![r, Space::new().width(Length::Fill), hint].align_y(Alignment::Center)).max_width(layout::CHAT_COLUMN_MAX);
    container(inner)
        .padding(Padding::from([0.0, space::S4]))
        .height(Length::Fixed(TABS_H))
        .center_x(Length::Fill)
        .align_y(iced::alignment::Vertical::Center)
        .style(style::surface(b))
        .into()
}
