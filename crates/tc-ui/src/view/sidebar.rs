//! Raíl de carpetas (§8.1) y lista de chats (§8.2, S07–S08) con búsqueda local y virtualización
//! por índice directo (filas de altura fija).

use iced::alignment::{Horizontal, Vertical};
use iced::widget::{Space, button, column, container, mouse_area, row, scrollable, stack, text_input};
use iced::{Alignment, Element, Length, Padding};
use tc_core::{Dialog, GroupKind, PeerRef, SendState, UserStatus};
use tc_theme::ThemeKind;
use tc_theme::tokens::{layout, radius, space, text as ts};

use crate::app::{App, Folder, LIST_SCROLL_ID, LoadState, Msg, SEARCH_ID};
use crate::icon::{Icon, icon, icon_fill};
use crate::style::{self, c, with_weight};
use crate::widgets::{avatar, badge, hfill, hgap, t, t1, vgap};

const ROW_H: f32 = layout::CHAT_ITEM_HEIGHT_COMFORTABLE;

fn folder_icon(f: Folder) -> Icon {
    match f {
        Folder::All => Icon::Chats,
        Folder::Unread => Icon::Unread,
        Folder::Personal => Icon::User,
        Folder::Groups => Icon::Users,
        Folder::Channels => Icon::Megaphone,
    }
}

/// Raíl de 72 px: marca, carpetas con contador y conmutadores de tema y movimiento.
pub fn rail(app: &App) -> Element<'_, Msg> {
    let b = app.brasa;
    let mark = container(t1("tc", with_weight(ts::TITLE, 700), c(b.p.on_accent)))
        .width(Length::Fixed(40.0))
        .height(Length::Fixed(40.0))
        .align_x(Horizontal::Center)
        .align_y(Vertical::Center)
        .style(style::rounded_fill(c(b.p.accent), style::avatar_radius(40.0)));

    let mut col = column![vgap(space::S3), mark, vgap(space::S3)].align_x(Alignment::Center).width(Length::Fixed(layout::RAIL_WIDTH));
    for f in Folder::ALL {
        let active = app.folder == f;
        let ic = if active {
            icon_fill(folder_icon(f), layout::ICON_LG, c(b.p.accent_text))
        } else {
            icon(folder_icon(f), layout::ICON_LG, c(b.p.text_muted))
        };
        let n = app.folder_unread(f) as u32;
        let ic: Element<'_, Msg> = if n > 0 && f != Folder::All {
            stack![
                container(ic).width(Length::Fixed(40.0)).height(Length::Fixed(28.0)).align_x(Horizontal::Center),
                container(badge(n, false, b)).width(Length::Fixed(40.0)).align_x(Horizontal::Right),
            ]
            .into()
        } else {
            container(ic).width(Length::Fixed(40.0)).height(Length::Fixed(28.0)).align_x(Horizontal::Center).into()
        };
        let label = t1(f.label(), ts::CAPTION, c(if active { b.p.text } else { b.p.text_muted }));
        let cell = button(column![ic, label].align_x(Alignment::Center).spacing(space::S0_5))
            .width(Length::Fixed(64.0))
            .height(Length::Fixed(64.0))
            .padding(Padding::from([space::S2, 0.0]))
            .style(style::rail_cell(b))
            .on_press(Msg::SetFolder(f));
        let pill = container(Space::new().width(Length::Fixed(4.0)).height(Length::Fixed(24.0)))
            .style(style::rounded_fill(if active { c(b.p.accent) } else { style::TRANSPARENT }, style::r(radius::PILL)));
        col = col.push(row![pill, hgap(space::S0_5), cell].align_y(Alignment::Center));
    }
    let (theme_icon, theme_label) = match b.kind {
        ThemeKind::Dark => (Icon::Sun, "Claro"),
        ThemeKind::Light => (Icon::Moon, "Oscuro"),
    };
    let reduced = app.anim.pref.is_reduced();
    let toggle = |i: Icon, label: &'static str, on: bool, msg: Msg| {
        let ic = if on { icon_fill(i, layout::ICON_MD, c(b.p.accent_text)) } else { icon(i, layout::ICON_MD, c(b.p.text_muted)) };
        button(column![ic, t1(label, ts::CAPTION, c(b.p.text_muted))].align_x(Alignment::Center).spacing(space::S0_5))
            .width(Length::Fixed(64.0))
            .padding(Padding::from([space::S2, 0.0]))
            .style(style::rail_cell(b))
            .on_press(msg)
    };
    col = col
        .push(Space::new().height(Length::Fill))
        .push(toggle(theme_icon, theme_label, false, Msg::ToggleTheme))
        .push(toggle(if reduced { Icon::Pause } else { Icon::Play }, "Animar", reduced, Msg::ToggleMotion))
        .push(vgap(space::S3));
    with_border_right(container(col).height(Length::Fill).style(style::surface(b)).into(), app)
}

fn with_border_right<'a>(inner: Element<'a, Msg>, app: &App) -> Element<'a, Msg> {
    container(inner).padding(Padding { right: 1.0, ..Padding::ZERO }).height(Length::Fill).style(style::border_fill(app.brasa)).into()
}

/// Lista de chats con buscador.
pub fn list(app: &App, width: Length) -> Element<'_, Msg> {
    let b = app.brasa;
    let mut field = row![icon(Icon::Search, layout::ICON_SM, c(b.p.text_placeholder))]
        .spacing(space::S2)
        .align_y(Alignment::Center)
        .push(
            text_input("Buscar chats (Ctrl+K)", &app.search)
                .id(SEARCH_ID)
                .on_input(Msg::SearchChanged)
                .padding(0)
                .size(ts::UI.size)
                .font(style::font_of(ts::UI))
                .style(style::bare_input(b)),
        );
    if !app.search.is_empty() {
        field = field.push(crate::widgets::icon_button(Icon::Close, layout::ICON_SM, c(b.p.text_muted), Some(Msg::ClearSearch), b));
    }
    let search = container(field)
        .padding(Padding::from([0.0, space::S3]))
        .height(Length::Fixed(40.0))
        .align_y(Vertical::Center)
        .style(style::sunken_field(b, false, radius::SM));
    let header = container(search).padding(space::S3);

    let body: Element<'_, Msg> = match &app.dialogs_state {
        LoadState::Loading => skeleton(app),
        LoadState::Failed(e) => container(
            column![
                t(e.as_str(), ts::UI, c(b.p.danger)),
                button(t1("Reintentar", ts::UI_STRONG, c(b.p.on_accent))).style(style::primary(b, radius::MD)).padding([8, 16]).on_press(Msg::RetryDialogs),
            ]
            .spacing(space::S3),
        )
        .padding(space::S4)
        .into(),
        LoadState::Ready => rows(app),
    };
    let col = column![header, body].width(width).height(Length::Fill);
    with_border_right(container(col).style(style::surface(b)).height(Length::Fill).into(), app)
}

fn skeleton(app: &App) -> Element<'_, Msg> {
    let b = app.brasa;
    let block = |w: f32, h: f32| container(Space::new().width(Length::Fixed(w)).height(Length::Fixed(h))).style(style::rounded_fill(c(b.p.surface_hover), style::r(radius::XS)));
    let mut col = column![];
    for i in 0..8 {
        col = col.push(
            row![
                container(Space::new().width(Length::Fixed(44.0)).height(Length::Fixed(44.0)))
                    .style(style::rounded_fill(c(b.p.surface_hover), style::avatar_radius(44.0))),
                column![block(120.0 + (i % 3) as f32 * 30.0, 12.0), block(180.0, 10.0)].spacing(space::S2),
            ]
            .spacing(space::S3)
            .align_y(Alignment::Center)
            .padding(Padding::from([space::S2, space::S3]))
            .height(Length::Fixed(ROW_H)),
        );
    }
    col.into()
}

fn rows(app: &App) -> Element<'_, Msg> {
    let b = app.brasa;
    let list = app.visible_dialogs();
    if list.is_empty() {
        let msg = if app.search.trim().is_empty() {
            "No hay chats en esta carpeta.".to_string()
        } else {
            format!("Ningún chat coincide con «{}». La búsqueda en mensajes llegará en un hito posterior.", app.search.trim())
        };
        return container(t(msg, ts::UI, c(b.p.text_muted))).padding(space::S4).into();
    }
    // Virtualización por índice directo: filas fijas de 68 px.
    let first = ((app.list_from_top / ROW_H).floor() as usize).saturating_sub(4).min(list.len());
    let count = (app.list_viewport / ROW_H).ceil() as usize + 8;
    let last = (first + count).min(list.len());
    let mut col = column![Space::new().height(Length::Fixed(first as f32 * ROW_H))];
    for d in &list[first..last] {
        col = col.push(chat_row(app, d));
    }
    col = col.push(Space::new().height(Length::Fixed((list.len() - last) as f32 * ROW_H)));
    scrollable(col)
        .id(LIST_SCROLL_ID)
        .on_scroll(Msg::ListScrolled)
        .direction(scrollable::Direction::Vertical(scrollable::Scrollbar::new().width(6).scroller_width(6).margin(2)))
        .style(style::thin_scroll(b))
        .height(Length::Fill)
        .into()
}

/// Caracteres que caben en la vista previa (la lista no tiene elipsis nativa en iced 0.14).
const PREVIEW_CHARS: usize = 34;
const NAME_CHARS: usize = 26;

fn chat_row<'a>(app: &'a App, d: &'a Dialog) -> Element<'a, Msg> {
    let b = app.brasa;
    let selected = app.conv.as_ref().is_some_and(|cv| cv.peer == d.peer);
    let row_bg = c(if selected { b.p.surface_selected } else { b.p.surface });
    let online = match d.peer {
        PeerRef::User(u) => app.users.get(&u).is_some_and(|u| u.status == UserStatus::Online),
        _ => false,
    };
    let av = avatar(&d.title, App::peer_seed(d.peer), layout::AVATAR_LG, online.then_some(row_bg), b);

    // Línea 1: [escudo] nombre · hora
    let unread = d.unread_count > 0;
    let name_style = if unread { with_weight(ts::UI_STRONG, 700) } else { ts::UI_STRONG };
    let mut line1 = row![].spacing(space::S1).align_y(Alignment::Center);
    if matches!(d.peer, PeerRef::Secret(_)) {
        line1 = line1.push(icon(Icon::Shield, layout::ICON_SM, c(b.p.success)));
    }
    if app.group_kind(d.peer) == Some(GroupKind::Channel) {
        line1 = line1.push(icon(Icon::Megaphone, layout::ICON_SM, c(b.p.text_muted)));
    }
    line1 = line1.push(t1(crate::format::ellipsize(&d.title, NAME_CHARS), name_style, c(b.p.text)));
    if d.muted {
        line1 = line1.push(icon(Icon::Muted, 14.0, c(b.p.text_muted)));
    }
    line1 = line1.push(hfill());
    if let Some(m) = &d.last_message {
        if m.outgoing {
            let read = m.id <= d.read_outbox_max_id;
            let (ic, col) = match m.state {
                SendState::Pending => (Icon::Timer, b.p.text_muted),
                SendState::Failed => (Icon::Warning, b.p.danger),
                SendState::Sent if read => (Icon::Checks, b.p.accent_text),
                SendState::Sent => (Icon::Check, b.p.text_muted),
            };
            line1 = line1.push(icon(ic, layout::ICON_SM, c(col)));
        }
        line1 = line1.push(t1(crate::format::list_time(m.date, crate::app::now(), &app.tz), ts::CAPTION, c(b.p.text_muted)));
    }

    // Línea 2: vista previa · badge / chincheta
    let typing = app.typing.contains_key(&d.peer);
    let mut line2 = row![].spacing(space::S1).align_y(Alignment::Center);
    if typing {
        let dots = if app.anim.pref.is_reduced() { "…".to_string() } else { ".".repeat(app.dots as usize + 1) };
        line2 = line2.push(t1(format!("escribiendo{dots}"), ts::META, c(b.p.accent_text)));
    } else if let Some(dr) = &d.draft {
        line2 = line2
            .push(t1("Borrador:", ts::META, c(b.p.danger)))
            .push(t1(crate::format::ellipsize(&dr.text, PREVIEW_CHARS - 10), ts::META, c(b.p.text_muted)));
    } else if let Some(m) = &d.last_message {
        let mut budget = PREVIEW_CHARS;
        if !m.outgoing
            && app.is_group_chat(d.peer)
            && let Some(s) = m.sender
        {
            {
                let who = app.users.get(&s).map_or_else(String::new, |u| u.first_name.clone());
                if !who.is_empty() {
                    budget = budget.saturating_sub(who.chars().count() + 2);
                    line2 = line2.push(t1(format!("{who}:"), ts::META, c(b.p.text)));
                }
            }
        }
        line2 = line2.push(t1(crate::format::ellipsize(&crate::format::preview(m), budget), ts::META, c(b.p.text_muted)));
    } else if matches!(d.peer, PeerRef::Secret(_)) {
        line2 = line2.push(t1("Chat secreto listo. Los mensajes no salen de este dispositivo.", ts::META, c(b.p.text_muted)));
    }
    line2 = line2.push(hfill());
    if d.unread_mentions > 0 {
        line2 = line2.push(badge_at(b));
    }
    if unread {
        line2 = line2.push(badge(d.unread_count, d.muted, b));
    } else if d.pinned_order.is_some() {
        line2 = line2.push(icon(Icon::Pin, layout::ICON_SM, c(b.p.text_muted)));
    }

    let texts = column![line1, line2].spacing(space::S0_5).width(Length::Fill).clip(true);
    let content = row![av, texts].spacing(space::S3).align_y(Alignment::Center).padding(Padding::from([space::S2, space::S3]));
    let bar = container(Space::new().width(Length::Fixed(3.0)).height(Length::Fill))
        .style(style::fill(if selected { c(b.p.accent) } else { style::TRANSPARENT }));
    let inner = row![bar, content].height(Length::Fixed(ROW_H));
    mouse_area(
        button(inner)
            .padding(0)
            .width(Length::Fill)
            .height(Length::Fixed(ROW_H))
            .style(style::chat_row(b, selected))
            .on_press(Msg::Open(d.peer)),
    )
    .into()
}

fn badge_at<'a>(b: crate::style::Brasa) -> Element<'a, Msg> {
    container(t1("@", ts::CAPTION, c(b.p.on_accent)))
        .width(Length::Fixed(20.0))
        .height(Length::Fixed(20.0))
        .align_x(Horizontal::Center)
        .align_y(Vertical::Center)
        .style(style::badge(b, false))
        .into()
}
