//! Pestaña de GIFs (guardados, filtro al escribir y búsqueda en Telegram con Enter) y la burbuja de
//! un GIF en la conversación. Los GIF (MP4/H.264 o GIF real) se animan en bucle una vez
//! decodificados; mientras tanto, o si no se pueden decodificar, su miniatura con la insignia «GIF»
//! (ADR-0010).

use iced::widget::{Space, button, column, container, image, responsive, row, scrollable, stack, text_input};
use iced::{Alignment, Color, ContentFit, Element, Length, Padding};
use tc_core::{Gif, Media, Message, PeerRef};
use tc_theme::tokens::{layout, radius, space, text as ts};

use crate::app::{App, Msg};
use crate::gifs::{self, CELL_GAP, CELL_H, CELL_MIN_W, GifMsg};
use crate::icon::{Icon, icon};
use crate::media::ImageKey;
use crate::panel::{GIF_SEARCH_ID, PanelMsg};
use crate::style::{self, c, fade, with_weight};
use crate::widgets::t1;

fn gm(m: GifMsg) -> Msg {
    Msg::Panel(PanelMsg::Gif(m))
}

/// Insignia «GIF» sobre la esquina superior izquierda.
fn badge<'a>(w: f32, h: f32) -> Element<'a, Msg> {
    let pill = container(t1("GIF", with_weight(ts::CAPTION, 700), Color::WHITE))
        .padding([0, 5])
        .style(style::rounded_fill(Color { a: 0.55, ..Color::BLACK }, style::r(4.0)));
    container(pill).padding(6).width(Length::Fixed(w)).height(Length::Fixed(h)).into()
}

fn centered_text<'a>(app: &App, s: impl Into<String>) -> Element<'a, Msg> {
    container(t1(s.into(), ts::UI, c(app.brasa.p.text_muted))).center(Length::Fill).into()
}

pub fn view(app: &App) -> Element<'_, Msg> {
    let b = app.brasa;
    let g = &app.panel.gifs;
    let mut field = row![
        icon(Icon::Search, layout::ICON_SM, c(b.p.text_placeholder)),
        text_input("Buscar GIFs (Enter busca en Telegram)", &g.query)
            .id(GIF_SEARCH_ID)
            .on_input(|q| gm(GifMsg::Query(q)))
            .on_submit(gm(GifMsg::Search))
            .padding(0)
            .size(ts::UI.size)
            .font(style::font_of(ts::UI))
            .style(style::bare_input(b)),
    ]
    .spacing(space::S2)
    .align_y(Alignment::Center);
    if !g.query.trim().is_empty() && g.results().is_none() {
        field = field.push(button(t1("Buscar en Telegram", ts::CAPTION, c(b.p.accent_text))).padding([2, 8]).style(style::ghost(b, radius::SM)).on_press(gm(GifMsg::Search)));
    }
    let field = container(field)
        .padding(Padding::from([0.0, space::S3]))
        .height(Length::Fixed(32.0))
        .width(Length::Fill)
        .align_y(iced::alignment::Vertical::Center)
        .style(style::sunken_field(b, false, radius::SM));

    let shown = g.shown();
    let status: Option<String> = match g.results() {
        Some(s) if s.loading && s.gifs.is_empty() => Some("Buscando en Telegram…".into()),
        Some(s) => s.error.as_ref().map(|e| format!("No se pudo buscar en Telegram: {e}")),
        None => None,
    };
    let body: Element<'_, Msg> = if g.saved.is_none() || (g.saved_loading && shown.is_empty()) {
        centered_text(app, "Cargando GIFs…")
    } else if shown.is_empty() {
        if let Some(e) = &g.saved_error {
            container(
                column![
                    t1(format!("No se pudieron cargar los GIFs: {e}"), ts::UI, c(b.p.text_muted)),
                    button(t1("Reintentar", ts::UI_STRONG, c(b.p.accent_text))).padding([4, 12]).style(style::ghost(b, radius::SM)).on_press(gm(GifMsg::Retry)),
                ]
                .spacing(space::S2)
                .align_x(Alignment::Center),
            )
            .center(Length::Fill)
            .into()
        } else if g.query.trim().is_empty() {
            centered_text(app, "No tienes GIFs guardados. Escribe arriba para buscar.")
        } else if let Some(s) = status.clone() {
            centered_text(app, s)
        } else if g.results().is_some() {
            centered_text(app, "Ningún GIF coincide.")
        } else {
            centered_text(app, "Ningún GIF guardado coincide. Pulsa Enter para buscar en Telegram.")
        }
    } else {
        let ids: Vec<i64> = shown.iter().map(|g| g.id).collect();
        let more = g.results().is_some_and(|s| s.next.is_some());
        let loading_more = g.results().is_some_and(|s| s.loading);
        responsive(move |size| {
            let cols = (((size.width + CELL_GAP) / (CELL_MIN_W + CELL_GAP)).floor() as usize).max(1);
            app.panel.gifs.cols.set(cols);
            let cell_w = ((size.width - CELL_GAP * (cols as f32 - 1.0)) / cols as f32).floor().max(40.0);
            let mut col = column![].spacing(CELL_GAP);
            for (r, chunk) in ids.chunks(cols).enumerate() {
                let mut line = row![].spacing(CELL_GAP);
                for (k, id) in chunk.iter().enumerate() {
                    let i = r * cols + k;
                    if let Some(gif) = app.panel.gifs.find(*id) {
                        line = line.push(cell(app, gif, cell_w, app.panel.gifs.cursor.cursor == Some(i)));
                    }
                }
                col = col.push(line);
            }
            if more {
                let label = if loading_more { "Cargando…" } else { "Más resultados" };
                col = col.push(
                    container(button(t1(label, ts::UI_STRONG, c(b.p.accent_text))).padding([4, 12]).style(style::ghost(b, radius::SM)).on_press(gm(GifMsg::More)))
                        .center_x(Length::Fill),
                );
            }
            scrollable(col)
                .style(style::thin_scroll(b))
                .width(Length::Fill)
                .height(Length::Fill)
                .on_scroll(|v| gm(GifMsg::Scrolled { y: v.absolute_offset().y, h: v.bounds().height }))
                .into()
        })
        .into()
    };
    let mut col = column![field].spacing(space::S2);
    if let (Some(s), false) = (status, shown.is_empty()) {
        col = col.push(t1(s, ts::CAPTION, c(b.p.text_muted)));
    }
    col.push(body).into()
}

/// Celda de la rejilla: animación, miniatura con insignia (aún sin decodificar, fuera de la vista o
/// sin decodificador) o hueco con «GIF».
fn cell<'a>(app: &'a App, gif: &'a Gif, w: f32, focused: bool) -> Element<'a, Msg> {
    let b = app.brasa;
    let file = gif.file.as_ref().map(|f| f.id);
    let anim = file.and_then(|id| app.gif_frame(id));
    let animated = anim.is_some();
    let handle = anim.or_else(|| file.and_then(|id| app.gif_thumb(id)));
    let inner: Element<'a, Msg> = match handle {
        Some(h) => {
            let img = image(h).width(Length::Fixed(w)).height(Length::Fixed(CELL_H)).content_fit(ContentFit::Cover).border_radius(style::r(radius::SM));
            if animated { img.into() } else { stack![img, badge(w, CELL_H)].into() }
        }
        None => container(t1("GIF", with_weight(ts::UI_STRONG, 700), c(b.p.text_muted)))
            .center_x(Length::Fixed(w))
            .center_y(Length::Fixed(CELL_H))
            .style(style::rounded_fill(c(b.p.surface_hover), style::r(radius::SM)))
            .into(),
    };
    button(inner)
        .padding(0)
        .width(Length::Fixed(w))
        .height(Length::Fixed(CELL_H))
        .style(style::sticker_cell(b, focused))
        .on_press(gm(GifMsg::Send(gif.id)))
        .into()
}

/// GIF en una burbuja: animado en bucle si ya está decodificado; si no, la miniatura con la insignia
/// «GIF».
pub fn bubble<'a>(app: &'a App, peer: PeerRef, m: &'a Message, md: &'a Media, sz: iced::Size, opacity: f32, meta_c: Color) -> Element<'a, Msg> {
    let b = app.brasa;
    let file = md.file.as_ref().map(|f| f.id);
    let anim = file.filter(|_| gifs::animates(md)).and_then(|id| app.gif_frame(id));
    let animated = anim.is_some();
    let handle = anim
        .or_else(|| file.and_then(|id| app.images.handle(&ImageKey::Thumb { peer, msg: m.id, file: id })))
        .or_else(|| file.and_then(|id| app.gif_thumb(id)));
    match handle {
        Some(h) => {
            let img = image(h)
                .width(Length::Fixed(sz.width))
                .height(Length::Fixed(sz.height))
                .content_fit(ContentFit::Cover)
                .border_radius(style::r(radius::MD))
                .opacity(opacity);
            if animated { img.into() } else { stack![img, badge(sz.width, sz.height)].into() }
        }
        None => container(column![t1("GIF", with_weight(ts::UI_STRONG, 700), meta_c), Space::new().height(Length::Fixed(2.0))].align_x(Alignment::Center))
            .center_x(Length::Fixed(sz.width))
            .center_y(Length::Fixed(sz.height))
            .style(style::rounded_fill(fade(c(b.p.surface_hover), opacity), style::r(radius::MD)))
            .into(),
    }
}
