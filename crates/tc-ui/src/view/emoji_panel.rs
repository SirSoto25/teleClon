//! Pestaña de emojis: buscador, tono de piel, categorías con icono y rejilla. Pulsar un emoji lo
//! inserta en el cursor del compositor y el panel sigue abierto.

use iced::widget::{Space, button, column, container, responsive, row, scrollable, text_input};
use iced::{Alignment, Element, Length, Padding};
use tc_theme::TextStyle;
use tc_theme::tokens::{layout, radius, space, text as ts};

use crate::app::{App, Msg};
use crate::emoji::{self, Category, Tone};
use crate::icon::{Icon, icon, icon_fill};
use crate::panel::{EMOJI_SEARCH_ID, PanelMsg};
use crate::style::{self, c};
use crate::widgets::{t1, vgap};

/// Id de la rejilla (para volver arriba al cambiar de categoría).
pub const GRID_ID: &str = "emoji-grid";
/// Celda de la rejilla y tamaño del emoji dentro.
pub const CELL: f32 = 40.0;
pub const GLYPH: f32 = 26.0;

fn glyph<'a>(e: impl Into<String>, size: f32, app: &App) -> iced::widget::Text<'a> {
    t1(e.into(), TextStyle { size, line: (size * 1.2).round(), ..ts::EMOJI_ONLY }, c(app.brasa.p.text))
}

fn pm(m: PanelMsg) -> Msg {
    Msg::Panel(m)
}

pub fn view(app: &App) -> Element<'_, Msg> {
    let b = app.brasa;
    let p = &app.panel.emoji;
    let field = container(
        row![
            icon(Icon::Search, layout::ICON_SM, c(b.p.text_placeholder)),
            text_input("Buscar emoji (español o inglés)", &p.query)
                .id(EMOJI_SEARCH_ID)
                .on_input(|q| pm(PanelMsg::EmojiQuery(q)))
                .on_submit(pm(PanelMsg::EmojiSubmit))
                .padding(0)
                .size(ts::UI.size)
                .font(style::font_of(ts::UI))
                .style(style::bare_input(b)),
        ]
        .spacing(space::S2)
        .align_y(Alignment::Center),
    )
    .padding(Padding::from([0.0, space::S3]))
    .height(Length::Fixed(32.0))
    .width(Length::Fill)
    .align_y(iced::alignment::Vertical::Center)
    .style(style::sunken_field(b, false, radius::SM));
    let tone_btn = button(container(glyph(p.tone.sample(), 20.0, app)).center(Length::Fixed(28.0)))
        .padding(2)
        .style(style::option_row(b, p.tones_open))
        .on_press(pm(PanelMsg::ToggleTones));
    let mut head = column![row![field, tone_btn].spacing(space::S2).align_y(Alignment::Center)].spacing(space::S1);
    if p.tones_open {
        let mut tones = row![t1("Tono de piel:", ts::CAPTION, c(b.p.text_muted))].spacing(space::S1).align_y(Alignment::Center);
        for t in Tone::ALL {
            tones = tones.push(
                button(container(glyph(t.sample(), 20.0, app)).center(Length::Fixed(28.0)))
                    .padding(2)
                    .style(style::option_row(b, t == p.tone))
                    .on_press(pm(PanelMsg::Tone(t))),
            );
        }
        head = head.push(tones);
    }

    // Categorías (solo sin búsqueda).
    let searching = !p.query.trim().is_empty();
    if !searching {
        let shown = p.shown_category();
        let mut cats = row![].spacing(2).align_y(Alignment::Center);
        for cat in Category::ALL {
            let active = cat == shown || (cat == Category::Recent && p.category == Category::Recent);
            let inner: Element<'_, Msg> = if cat == Category::Recent {
                if active { icon_fill(Icon::Timer, 18.0, c(b.p.accent_text)) } else { icon(Icon::Timer, 18.0, c(b.p.text_muted)) }
            } else {
                glyph(cat.icon(), 18.0, app).into()
            };
            cats = cats.push(
                iced::widget::tooltip(
                    button(container(inner).center(Length::Fixed(28.0))).padding(0).style(style::option_row(b, active)).on_press(pm(PanelMsg::Category(cat))),
                    container(t1(cat.label(), ts::CAPTION, c(b.p.text))).padding([2, 6]).style(style::raised(b, 2, radius::SM)),
                    iced::widget::tooltip::Position::Top,
                ),
            );
        }
        head = head.push(cats);
    }

    // Secciones: (título, emojis ya con el tono).
    let tone = p.tone;
    let sections: Vec<(String, Vec<String>)> = if searching {
        let hits: Vec<String> = emoji::search(&p.query).into_iter().map(|e| emoji::apply(e, tone).to_string()).collect();
        vec![(if hits.is_empty() { "Sin resultados".into() } else { "Resultados".into() }, hits)]
    } else {
        match p.shown_category() {
            Category::Recent => [
                ("Recientes".to_string(), p.usage.recent(emoji::RECENT_SHOWN)),
                ("Frecuentes".to_string(), p.usage.frequent(emoji::FREQUENT_SHOWN)),
            ]
            .into_iter()
            .filter(|(_, l)| !l.is_empty())
            .map(|(t, l)| (t, l.into_iter().map(String::from).collect()))
            .collect(),
            cat => vec![(cat.label().to_string(), emoji::list(cat).iter().map(|e| emoji::apply(e, tone).to_string()).collect())],
        }
    };
    let grid = responsive(move |size| {
        let cols = ((size.width / CELL).floor() as usize).max(1);
        let mut col = column![].spacing(0);
        for (title, list) in &sections {
            col = col.push(container(t1(title.clone(), ts::CAPTION, c(b.p.text_muted))).padding(Padding::from([space::S1, 4.0])));
            for chunk in list.chunks(cols) {
                let mut line = row![].spacing(0);
                for e in chunk {
                    line = line.push(
                        button(container(glyph(e.clone(), GLYPH, app)).center(Length::Fixed(CELL)))
                            .padding(0)
                            .width(Length::Fixed(CELL))
                            .height(Length::Fixed(CELL))
                            .style(style::sticker_cell(b, false))
                            .on_press(pm(PanelMsg::Insert(e.clone()))),
                    );
                }
                col = col.push(line);
            }
            col = col.push(vgap(space::S1));
        }
        scrollable(col.push(Space::new().height(Length::Fixed(space::S2))))
            .id(GRID_ID)
            .style(style::thin_scroll(b))
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
    });
    column![head, grid].spacing(space::S1).into()
}
