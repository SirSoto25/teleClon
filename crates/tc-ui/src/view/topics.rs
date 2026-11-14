//! Grupos con temas (docs/design/system.md §8.10): lista de temas en el panel de la conversación
//! (en ventana estrecha, a pantalla completa), cabecera «Grupo › Tema» de la conversación de un tema
//! y menú contextual de un tema. La lógica está en `app/forum.rs`.

use iced::alignment::{Horizontal, Vertical};
use iced::widget::{Space, button, column, container, mouse_area, opaque, responsive, row, scrollable, stack, text_input};
use iced::{Alignment, Color, Element, Length, Padding, Size};
use tc_core::{ForumTopic, GroupId, ThreadRef};
use tc_theme::TextStyle;
use tc_theme::tokens::{layout, radius, space, text as ts};

use crate::app::forum::{ForumMsg, TOPIC_NEW_ID, TOPIC_SEARCH_ID, TOPICS_SCROLL_ID, topic_initial};
use crate::app::{App, LoadState, Msg};
use crate::conv::Conversation;
use crate::format::{self, plain_letters, preview};
use crate::icon::{Icon, icon, icon_fill};
use crate::measure::{ellipsize_to, text_width};
use crate::style::{self, Brasa, c, with_weight};
use crate::widgets::{avatar_photo, badge, hfill, hgap, icon_button, t, t1};

use super::conversation::in_column;

/// Alto de una fila de tema (como las de la lista de chats).
pub const TOPIC_ROW_H: f32 = layout::CHAT_ITEM_HEIGHT_COMFORTABLE;
/// Lado del icono del tema en la lista y en la cabecera.
const ICON_LIST: f32 = 40.0;
const ICON_HEAD: f32 = layout::AVATAR_MD;
const MENU_W: f32 = 232.0;
const ITEM_H: f32 = 32.0;

fn forum(m: ForumMsg) -> Msg {
    Msg::Forum(m)
}

fn rgb(c: u32) -> Color {
    Color::from_rgb8(((c >> 16) & 0xFF) as u8, ((c >> 8) & 0xFF) as u8, (c & 0xFF) as u8)
}

fn hairline<'a>(b: Brasa) -> Element<'a, Msg> {
    container(Space::new().height(Length::Fixed(1.0)).width(Length::Fill)).style(style::fill(c(b.p.border))).into()
}

/// Icono de un tema: círculo de su color con su emoji o su inicial; el General, «#» sobre neutro.
pub fn topic_icon<'a>(t: &ForumTopic, size: f32, b: Brasa) -> Element<'a, Msg> {
    let glyph = TextStyle { size: (size * 0.46).round(), line: (size * 0.56).round(), ..with_weight(ts::UI, 700) };
    let (bg, inner): (Color, Element<'a, Msg>) = if t.is_general() {
        (c(b.p.surface_hover), icon(Icon::Hash, (size * 0.5).round(), c(b.p.text_muted)))
    } else {
        match &t.icon.emoji {
            Some(e) => (Color { a: 0.22, ..rgb(t.icon.color) }, t1(e.clone(), glyph, c(b.p.text)).into()),
            None => (rgb(t.icon.color), t1(topic_initial(&t.title), glyph, style::avatar_ink()).into()),
        }
    };
    container(inner)
        .width(Length::Fixed(size))
        .height(Length::Fixed(size))
        .align_x(Horizontal::Center)
        .align_y(Vertical::Center)
        .style(style::rounded_fill(bg, style::r(size / 2.0)))
        .into()
}

/// Lista de temas del foro abierto (o `None` si no hay ninguna a la vista).
pub fn view(app: &App, narrow: bool) -> Option<Element<'_, Msg>> {
    let g = app.topics_shown()?;
    let b = app.brasa;
    let mut col = column![header(app, g, narrow), hairline(b), search(app)];
    if app.forum.new_title.is_some() {
        col = col.push(new_topic(app));
    }
    col = col.push(body(app, g));
    Some(container(col.height(Length::Fill).width(Length::Fill)).style(style::surface(b)).into())
}

fn header(app: &App, g: GroupId, narrow: bool) -> Element<'_, Msg> {
    let b = app.brasa;
    let peer = tc_core::PeerRef::Group(g);
    let d = app.dialog(peer);
    let title = d.map_or_else(|| app.groups.get(&g).map(|g| g.title.clone()).unwrap_or_default(), |d| d.title.clone());
    let mut left = row![].spacing(space::S3).align_y(Alignment::Center);
    if narrow {
        left = left.push(icon_button(Icon::Back, layout::ICON_MD, c(b.p.text), Some(forum(ForumMsg::CloseList)), b));
    }
    left = left.push(avatar_photo(&title, App::peer_seed(peer), layout::AVATAR_MD, None, app.avatar_handle(peer, d.and_then(|d| d.photo)), b));
    let list = app.forum.topics.get(&g);
    let n = list.map_or(0, |l| l.total.max(l.items.len() as u32));
    let members = app.groups.get(&g).map_or(0, |g| g.member_count);
    let mut status = if n == 1 { "1 tema".to_string() } else { format!("{n} temas") };
    if members > 0 {
        status.push_str(&format!(" · {members} miembros"));
    }
    let name = row![icon(Icon::Hash, layout::ICON_SM, c(b.p.text_muted)), t1(format::ellipsize(&plain_letters(&title), 44), ts::TITLE, c(b.p.text))]
        .spacing(space::S1)
        .align_y(Alignment::Center);
    left = left.push(column![name, t1(status, ts::META, c(b.p.text_muted))].spacing(0));
    let new = icon_button(Icon::Plus, layout::ICON_MD, c(b.p.text), Some(forum(ForumMsg::StartNew)), b);
    in_column(row![left, hfill(), new].align_y(Alignment::Center))
        .height(Length::Fixed(layout::HEADER_HEIGHT))
        .align_y(Vertical::Center)
        .style(style::surface(b))
        .into()
}

fn search(app: &App) -> Element<'_, Msg> {
    let b = app.brasa;
    let mut field = row![icon(Icon::Search, layout::ICON_SM, c(b.p.text_placeholder))].spacing(space::S2).align_y(Alignment::Center).push(
        text_input("Buscar temas", &app.forum.query)
            .id(TOPIC_SEARCH_ID)
            .on_input(|q| forum(ForumMsg::Query(q)))
            .padding(0)
            .size(ts::UI.size)
            .font(style::font_of(ts::UI))
            .style(style::bare_input(b)),
    );
    if !app.forum.query.is_empty() {
        field = field.push(icon_button(Icon::Close, layout::ICON_SM, c(b.p.text_muted), Some(forum(ForumMsg::ClearQuery)), b));
    }
    let boxed = container(field).padding(Padding::from([0.0, space::S3])).height(Length::Fixed(40.0)).align_y(Vertical::Center).style(style::sunken_field(b, false, radius::SM));
    in_column(boxed).padding(Padding::from([space::S3, crate::conv::SIDE_PAD])).into()
}

fn new_topic(app: &App) -> Element<'_, Msg> {
    let b = app.brasa;
    let value = app.forum.new_title.as_deref().unwrap_or_default();
    let input = text_input("Nombre del tema nuevo", value)
        .id(TOPIC_NEW_ID)
        .on_input(|s| forum(ForumMsg::NewTitle(s)))
        .on_submit(forum(ForumMsg::Create))
        .padding(Padding::from([10.0, 12.0]))
        .size(ts::UI.size)
        .font(style::font_of(ts::UI))
        .style(style::field_input(b, false));
    let ready = !value.trim().is_empty() && !app.forum.creating;
    let create = button(t1(if app.forum.creating { "Creando…" } else { "Crear tema" }, ts::UI_STRONG, c(b.p.on_accent)))
        .padding([8, 16])
        .style(style::primary(b, radius::MD))
        .on_press_maybe(ready.then_some(forum(ForumMsg::Create)));
    let cancel = button(t1("Cancelar", ts::UI_STRONG, c(b.p.text))).padding([8, 16]).style(style::ghost(b, radius::MD)).on_press(forum(ForumMsg::StartNew));
    in_column(row![input, cancel, create].spacing(space::S2).align_y(Alignment::Center)).padding(Padding { bottom: space::S3, ..Padding::from([0.0, crate::conv::SIDE_PAD]) }).into()
}

fn body(app: &App, g: GroupId) -> Element<'_, Msg> {
    let b = app.brasa;
    let state = app.forum.topics.get(&g).map_or(LoadState::Loading, |l| l.state.clone());
    let centered = |e: Element<'static, Msg>| -> Element<'_, Msg> { container(e).padding(space::S6).center_x(Length::Fill).into() };
    let list = app.visible_topics(g);
    match state {
        LoadState::Loading if list.is_empty() => centered(t1("Cargando temas…", ts::UI, c(b.p.text_muted)).into()),
        LoadState::Failed(e) if list.is_empty() => centered(
            column![
                t(e, ts::UI, c(b.p.danger)),
                button(t1("Reintentar", ts::UI_STRONG, c(b.p.on_accent))).style(style::primary(b, radius::MD)).padding([8, 16]).on_press(forum(ForumMsg::Reload)),
            ]
            .spacing(space::S3)
            .align_x(Alignment::Center)
            .into(),
        ),
        _ if list.is_empty() => {
            let q = app.forum.query.trim();
            let msg = if q.is_empty() { "Este grupo todavía no tiene temas.".to_string() } else { format!("Ningún tema coincide con «{q}».") };
            centered(t(msg, ts::UI, c(b.p.text_muted)).into())
        }
        _ => responsive(move |size| {
            let col_w = size.width.min(layout::CHAT_COLUMN_MAX + 2.0 * crate::conv::SIDE_PAD) - 2.0 * crate::conv::SIDE_PAD;
            let mut rows = column![];
            for tp in app.visible_topics(g) {
                rows = rows.push(topic_row(app, tp, col_w));
            }
            if app.forum.topics.get(&g).is_some_and(|l| l.loading_more) {
                rows = rows.push(container(t1("Cargando más temas…", ts::META, c(b.p.text_muted))).padding(space::S3).center_x(Length::Fill));
            }
            scrollable(in_column(rows))
                .id(TOPICS_SCROLL_ID)
                .on_scroll(|v| forum(ForumMsg::Scrolled(v)))
                .direction(scrollable::Direction::Vertical(scrollable::Scrollbar::new().width(6).scroller_width(6).margin(2)))
                .style(style::thin_scroll(b))
                .height(Length::Fill)
                .into()
        })
        .into(),
    }
}

/// Columna derecha (hora y no leídos), como en la lista de chats.
const RIGHT_W: f32 = 60.0;
const LINE_H: f32 = 20.0;

fn topic_row<'a>(app: &'a App, tp: &'a ForumTopic, col_w: f32) -> Element<'a, Msg> {
    let b = app.brasa;
    let th = tp.thread();
    let unread = tp.unread_count > 0;
    let name_style = if unread { with_weight(ts::UI_STRONG, 700) } else { ts::UI_STRONG };
    // Ancho del texto: la columna (como mucho CHAT_COLUMN_MAX) menos icono, huecos y columna derecha.
    let text_w = (col_w - ICON_LIST - 2.0 * space::S3 - space::S3 - space::S2 - RIGHT_W).max(80.0);
    let gap = space::S1;

    let mut line1 = row![].spacing(gap).align_y(Alignment::Center);
    let mut name_w = text_w;
    if tp.closed {
        line1 = line1.push(icon(Icon::Lock, layout::ICON_SM, c(b.p.text_muted)));
        name_w -= layout::ICON_SM + gap;
    }
    line1 = line1.push(t1(ellipsize_to(&plain_letters(&tp.title), name_style, name_w), name_style, c(b.p.text)));

    let mut line2 = row![].spacing(gap).align_y(Alignment::Center);
    match &tp.last_message {
        Some(m) => {
            let who = match m.sender {
                _ if m.outgoing => "Tú".to_string(),
                Some(s) => app.users.get(&s).map(|u| plain_letters(&u.first_name).into_owned()).unwrap_or_default(),
                None => String::new(),
            };
            let body = preview(m);
            if who.is_empty() {
                line2 = line2.push(t1(ellipsize_to(&plain_letters(&body), ts::META, text_w), ts::META, c(b.p.text_muted)));
            } else {
                let prefix = format!("{who}:");
                let pw = text_width(&prefix, ts::META);
                line2 = line2
                    .push(t1(prefix, ts::META, c(b.p.text)))
                    .push(t1(ellipsize_to(&plain_letters(&body), ts::META, (text_w - pw - gap).max(0.0)), ts::META, c(b.p.text_muted)));
            }
        }
        None => line2 = line2.push(t1("Sin mensajes todavía", ts::META, c(b.p.text_muted))),
    }

    let mut top = row![].spacing(gap).align_y(Alignment::Center);
    if let Some(m) = &tp.last_message {
        top = top.push(t1(format::list_time(m.date, crate::app::now(), &app.tz), ts::CAPTION, c(b.p.text_muted)));
    }
    let mut bottom = row![].spacing(gap).align_y(Alignment::Center);
    if unread {
        bottom = bottom.push(badge(tp.unread_count, false, b));
    } else if tp.pinned {
        bottom = bottom.push(icon(Icon::Pin, layout::ICON_SM, c(b.p.text_muted)));
    }
    let cell = |r: iced::widget::Row<'a, Msg>| container(r).width(Length::Fill).height(Length::Fixed(LINE_H)).align_x(Horizontal::Right).align_y(Vertical::Center);
    let right = column![cell(top), cell(bottom)].spacing(space::S0_5).width(Length::Fixed(RIGHT_W));
    let line = |r: iced::widget::Row<'a, Msg>| container(r).width(Length::Fill).height(Length::Fixed(LINE_H)).align_y(Vertical::Center).clip(true);
    let texts = column![line(line1), line(line2)].spacing(space::S0_5).width(Length::Fill);
    let content = row![topic_icon(tp, ICON_LIST, b), hgap(space::S3), texts, hgap(space::S2), right]
        .align_y(Alignment::Center)
        .padding(Padding::from([space::S2, space::S3]))
        .height(Length::Fixed(TOPIC_ROW_H));
    let menu_open = app.forum.menu.is_some_and(|(m, _)| m == th);
    mouse_area(button(content).padding(0).width(Length::Fill).style(style::chat_row(b, menu_open)).on_press(forum(ForumMsg::OpenTopic(th))))
        .on_right_press(forum(ForumMsg::OpenMenu(th)))
        .into()
}

/// Cabecera de la conversación de un tema: volver a los temas, icono, «Grupo › Tema» y estado.
pub fn topic_header<'a>(app: &'a App, cv: &'a Conversation, _narrow: bool) -> Option<Element<'a, Msg>> {
    let th = app.open_thread().filter(|t| t.peer() == cv.peer)?;
    let b = app.brasa;
    let group = app.dialog(cv.peer).map(|d| d.title.clone()).or_else(|| app.groups.get(&th.group).map(|g| g.title.clone())).unwrap_or_default();
    let tp = app.topic(th);
    let title = tp.map_or("Tema", |t| t.title.as_str());
    let mut left = row![icon_button(Icon::Back, layout::ICON_MD, c(b.p.text), Some(forum(ForumMsg::BackToTopics)), b)].spacing(space::S3).align_y(Alignment::Center);
    if let Some(tp) = tp {
        left = left.push(topic_icon(tp, ICON_HEAD, b));
    }
    let crumb = row![
        t1(format::ellipsize(&plain_letters(&group), 28), ts::UI, c(b.p.text_muted)),
        icon(Icon::CaretRight, layout::ICON_SM, c(b.p.text_muted)),
        t1(format::ellipsize(&plain_letters(title), 40), ts::TITLE, c(b.p.text)),
    ]
    .spacing(space::S1)
    .align_y(Alignment::Center);
    let (status, color) = match app.typing.get(&cv.peer) {
        Some((user, _)) => (format!("{} está escribiendo…", app.user_name(*user)), c(b.p.accent_text)),
        None => match tp {
            Some(t) if t.closed => ("tema cerrado".to_string(), c(b.p.warning)),
            Some(t) if t.unread_count > 0 => (format!("{} sin leer", t.unread_count), c(b.p.text_muted)),
            _ => (app.groups.get(&th.group).map_or_else(String::new, |g| format!("{} miembros", g.member_count)), c(b.p.text_muted)),
        },
    };
    left = left.push(column![crumb, t1(status, ts::META, color)].spacing(0));
    let mut actions = row![super::conversation::find_button(app)].spacing(space::S1).align_y(Alignment::Center);
    if let Some(t) = tp {
        let pin: Element<'_, Msg> = if t.pinned { icon_fill(Icon::Pin, layout::ICON_MD, c(b.p.accent_text)) } else { icon(Icon::Pin, layout::ICON_MD, c(b.p.text)) };
        actions = actions.push(button(pin).padding(8).style(style::ghost(b, radius::SM)).on_press(forum(ForumMsg::Pin(th, !t.pinned))));
    }
    let bar = in_column(row![left, hfill(), actions].align_y(Alignment::Center))
        .height(Length::Fixed(layout::HEADER_HEIGHT))
        .align_y(Vertical::Center)
        .style(style::surface(b));
    Some(column![bar, hairline(b)].into())
}

/// Menú contextual de un tema (mismo aspecto que el de la lista de chats).
pub fn layers(app: &App, size: Size) -> Vec<Element<'_, Msg>> {
    let Some((th, at)) = app.forum.menu else { return Vec::new() };
    let Some(tp) = app.topic(th) else { return Vec::new() };
    let b = app.brasa;
    let item = |ic: Icon, label: &'static str, msg: Option<Msg>| -> Element<'_, Msg> {
        let color = c(if msg.is_some() { b.p.text } else { b.p.text_muted });
        button(container(row![icon(ic, layout::ICON_MD, c(b.p.text_muted)), t1(label, ts::UI, color)].spacing(space::S3).align_y(Alignment::Center)).center_y(Length::Fill))
            .width(Length::Fill)
            .height(Length::Fixed(ITEM_H))
            .padding(Padding::from([0.0, space::S3]))
            .style(style::option_row(b, false))
            .on_press_maybe(msg)
            .into()
    };
    let read = (tp.unread_count > 0 || tp.unread_mentions > 0).then_some(forum(ForumMsg::MarkRead(th)));
    let items = vec![
        item(Icon::Chats, "Abrir tema", Some(forum(ForumMsg::OpenTopic(th)))),
        item(Icon::Checks, "Marcar como leído", read),
        item(Icon::Pin, if tp.pinned { "Desfijar tema" } else { "Fijar tema" }, Some(forum(ForumMsg::Pin(th, !tp.pinned)))),
        item(Icon::Lock, if tp.closed { "Reabrir tema" } else { "Cerrar tema" }, Some(forum(ForumMsg::Close(th, !tp.closed)))),
    ];
    let h = items.len() as f32 * ITEM_H + 2.0 * space::S1;
    let panel = container(column(items)).padding(space::S1).width(Length::Fixed(MENU_W)).style(style::raised(b, 2, radius::MD));
    let x = at.x.min(size.width - MENU_W - space::S2).max(space::S2);
    let y = at.y.min(size.height - h - space::S2).max(space::S2);
    let catcher = mouse_area(container(Space::new()).width(Length::Fill).height(Length::Fill)).on_press(forum(ForumMsg::CloseMenu)).on_right_press(forum(ForumMsg::CloseMenu));
    vec![opaque(stack![catcher, container(opaque(panel)).padding(Padding { left: x, top: y, ..Padding::ZERO })])]
}

/// Para pruebas: hilo de la fila `i` de la lista visible.
pub fn row_thread(app: &App, g: GroupId, i: usize) -> Option<ThreadRef> {
    app.visible_topics(g).get(i).map(|t| t.thread())
}
