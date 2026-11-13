//! Stickers: dibujo (estático o animado) y selector junto al compositor (§8 «Sticker»: rejilla de
//! 72 px con pestañas de pack de 28 px, hover `surface-hover`, foco con anillo).

use iced::alignment::{Horizontal, Vertical};
use iced::widget::{Space, button, column, container, image, mouse_area, responsive, row, scrollable, stack};
use iced::{Alignment, Color, Element, Length, Padding, Radians, Rotation};
use tc_core::Sticker;
use tc_theme::TextStyle;
use tc_theme::tokens::{layout, space, text as ts};

use crate::app::{App, Msg};
use crate::icon::{Icon, icon, icon_fill};
use crate::sticker_anim::AnimMsg;
use crate::stickers::{self, CELL, CELL_ART, CELL_GAP, PANEL_H, Tab};
use crate::style::{self, c};
use crate::widgets::{hfill, icon_button, t, t1};

/// Sticker de `size` px. Los animados se mueven con el reloj de la app salvo con movimiento
/// reducido (primer fotograma). Los de la red se dibujan con su imagen descargada (ver
/// `sticker_img.rs`); sin dibujo ni imagen, el emoji hace de respaldo.
pub fn art<'a>(app: &'a App, s: &'a Sticker, size: f32, seed: usize, fallback: Color) -> Element<'a, Msg> {
    match (app.art.handle(s), stickers::art_index(s)) {
        (Some(h), Some(idx)) => {
            let (scale, rot) = if stickers::is_animated(s) { stickers::motion(idx, app.sticker_phase(seed)) } else { (1.0, 0.0) };
            image(h)
                .width(Length::Fixed(size))
                .height(Length::Fixed(size))
                .scale(scale)
                .rotation(Rotation::Floating(Radians(rot)))
                .into()
        }
        // TGS animado (fotograma actual) o, si no, la imagen estática del sticker de la red.
        _ => match app.sticker_anim_frame(s, size).or_else(|| app.sticker_image(s, size)) {
            Some(h) => image(h).width(Length::Fixed(size)).height(Length::Fixed(size)).content_fit(iced::ContentFit::Contain).into(),
            None => {
                let fs = (size * 0.66).round();
                container(t1(s.emoji.as_str(), TextStyle { size: fs, line: (fs * 1.15).round(), ..ts::EMOJI_ONLY }, fallback)).center(Length::Fixed(size)).into()
            }
        },
    }
}

/// Panel del selector (entre la conversación y el compositor).
pub fn picker(app: &App) -> Element<'_, Msg> {
    let b = app.brasa;
    let tabs = app.picker_tabs();
    let mut tab_row = row![].spacing(space::S1).align_y(Alignment::Center);
    for (i, tab) in tabs.iter().enumerate() {
        let active = i == app.picker.tab;
        let content: Element<'_, Msg> = match tab {
            Tab::Recent => icon(Icon::Timer, 24.0, c(if active { b.p.accent_text } else { b.p.text_muted })),
            Tab::Faved => t1("★", TextStyle { size: 20.0, line: 24.0, ..ts::UI }, c(if active { b.p.accent_text } else { b.p.text_muted })).into(),
            Tab::Set(k) => match app.sticker_sets.get(*k).and_then(|s| s.stickers.first()) {
                // Los packs animados llevan una marca de «reproducir» (no solo el movimiento).
                Some(first) if stickers::is_animated(first) => stack![
                    art(app, first, 28.0, 0, c(b.p.text)),
                    container(icon_fill(Icon::Play, 12.0, c(b.p.accent_text)))
                        .width(Length::Fixed(32.0))
                        .height(Length::Fixed(32.0))
                        .align_x(Horizontal::Right)
                        .align_y(Vertical::Bottom),
                ]
                .into(),
                Some(first) => art(app, first, 28.0, 0, c(b.p.text)),
                None => icon(Icon::Sticker, 24.0, c(b.p.text_muted)),
            },
        };
        tab_row = tab_row.push(
            button(container(content).center(Length::Fixed(36.0))).padding(0).style(style::option_row(b, active)).on_press(Msg::PickerTab(i)),
        );
    }
    let tab_title = match tabs.get(app.picker.tab) {
        Some(Tab::Recent) => "Recientes".to_string(),
        Some(Tab::Faved) => "Favoritos".to_string(),
        Some(Tab::Set(k)) => app.sticker_sets.get(*k).map(|s| s.title.clone()).unwrap_or_default(),
        None => String::new(),
    };
    let head = row![
        tab_row,
        hfill(),
        t1(tab_title, ts::UI_STRONG, c(b.p.text)),
        icon_button(Icon::Close, layout::ICON_SM, c(b.p.text_muted), Some(Msg::TogglePicker), b),
    ]
    .spacing(space::S3)
    .align_y(Alignment::Center);

    let list = app.picker_tab_stickers();
    let grid: Element<'_, Msg> = if app.sticker_sets.is_empty() {
        container(t1("Cargando stickers…", ts::UI, c(b.p.text_muted))).center(Length::Fill).into()
    } else if list.is_empty() {
        container(t1("Este pack está vacío.", ts::UI, c(b.p.text_muted))).center(Length::Fill).into()
    } else {
        responsive(move |size| {
            let cols = (((size.width + CELL_GAP) / (CELL + CELL_GAP)).floor() as usize).max(1);
            app.picker_cols.set(cols);
            let list = app.picker_tab_stickers();
            let mut col = column![].spacing(CELL_GAP);
            for (r, chunk) in list.chunks(cols).enumerate() {
                let mut line = row![].spacing(CELL_GAP);
                for (k, s) in chunk.iter().enumerate() {
                    let i = r * cols + k;
                    let focused = app.picker.cursor == Some(i);
                    let cell = button(container(art(app, s, CELL_ART, i, c(b.p.text))).center(Length::Fixed(CELL)))
                        .padding(0)
                        .width(Length::Fixed(CELL))
                        .height(Length::Fixed(CELL))
                        .style(style::sticker_cell(b, focused))
                        .on_press(Msg::SendSticker(s.id));
                    // El TGS bajo el ratón se anima (ver `sticker_anim.rs`).
                    line = line.push(
                        mouse_area(cell)
                            .on_enter(Msg::StickerAnim(AnimMsg::Hover(s.id)))
                            .on_exit(Msg::StickerAnim(AnimMsg::Unhover(s.id))),
                    );
                }
                col = col.push(line);
            }
            scrollable(col).style(style::thin_scroll(b)).width(Length::Fill).height(Length::Fill).into()
        })
        .into()
    };
    let hint = t("Flechas para elegir · Enter envía · Tab cambia de pack · Esc cierra · Ctrl+E abre y cierra", ts::CAPTION, c(b.p.text_muted));
    let inner = column![head, grid, hint].spacing(space::S2).max_width(layout::CHAT_COLUMN_MAX);
    let panel = container(inner)
        .padding(Padding::from([space::S2, space::S4]))
        .height(Length::Fixed(PANEL_H))
        .center_x(Length::Fill)
        .style(style::surface(b));
    let line = container(Space::new().height(Length::Fixed(1.0)).width(Length::Fill)).style(style::fill(c(b.p.border)));
    column![line, panel].into()
}
