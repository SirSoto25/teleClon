//! Conversación (S11–S13): cabecera, mensajes con scroll virtual y compositor.

use iced::alignment::{Horizontal, Vertical};
use iced::keyboard::{Key, key::Named};
use iced::widget::text::LineHeight;
use iced::widget::{Space, button, column, container, image, mouse_area, responsive, rich_text, row, scrollable, stack, text_editor};
use iced::{Alignment, Color, ContentFit, Element, Length, Padding, Pixels};
use tc_core::{GroupKind, Message, PeerRef, SecretChatState, SendState};
use tc_theme::tokens::{layout, radius, space, text as ts};
use tc_theme::{TextStyle, tokens};

use crate::app::{AnimKey, App, COMPOSER_ID, CONV_SCROLL_ID, Compose, Msg};
use crate::stickers::Nav;
use crate::conv::{self, Body, Conversation, Item, LayoutInput, MsgKey, MsgLayout};
use crate::format;
use crate::icon::{Icon, icon, icon_fill};
use crate::measure::{IcedMeasurer, to_spans};
use crate::media::ImageKey;
use crate::style::{self, Brasa, GroupPos, c, fade, with_weight};
use crate::widgets::{avatar_photo, contour_bg, hfill, hgap, icon_button, t, t1, vgap};

/// Ancho máximo de burbuja: 560 o 72 % de la columna (menos el avatar en grupos), lo menor.
pub fn bubble_max(col_w: f32, group: bool) -> f32 {
    let avail = if group { col_w - conv::GROUP_AVATAR_COL } else { col_w };
    layout::BUBBLE_MAX_WIDTH.min(avail * layout::BUBBLE_MAX_RATIO).max(160.0)
}

pub fn view(app: &App, narrow: bool) -> Element<'_, Msg> {
    let Some(cv) = &app.conv else { return empty(app) };
    let mut col = column![header(app, cv, narrow)];
    if matches!(cv.peer, PeerRef::Secret(_)) {
        col = col.push(experimental_banner(app));
    }
    col = col.push(body(app, cv));
    if app.picker.open {
        col = col.push(super::stickers::picker(app));
    }
    col.push(composer(app, cv)).height(Length::Fill).width(Length::Fill).into()
}

/// Columna de la conversación (§7): ancho máximo `CHAT_COLUMN_MAX` centrado, con margen lateral
/// `SIDE_PAD`. Cabecera, mensajes, compositor, selector de stickers y avisos comparten estos
/// bordes, así que en ventana ancha todo queda en una misma columna y en estrecha a 16 px del borde.
pub fn in_column<'a>(content: impl Into<Element<'a, Msg>>) -> container::Container<'a, Msg> {
    container(container(content).width(Length::Fill).max_width(layout::CHAT_COLUMN_MAX))
        .padding(Padding::from([0.0, conv::SIDE_PAD]))
        .center_x(Length::Fill)
}

/// Etiqueta «EXPERIMENTAL» de los chats secretos (texto, no solo color).
pub fn experimental_pill<'a>(b: Brasa) -> Element<'a, Msg> {
    container(t1("EXPERIMENTAL", with_weight(ts::CAPTION, 700), c(b.p.warning))).padding([1, 8]).style(style::experimental_pill(b)).into()
}

/// Aviso permanente bajo la cabecera de un chat secreto (ADR-0004: cripto propia sin auditoría).
fn experimental_banner(app: &App) -> Element<'_, Msg> {
    let b = app.brasa;
    let text = "Experimental: el cifrado de extremo a extremo de Parlo todavía no ha pasado una auditoría de seguridad. No lo uses para nada sensible.";
    let r = row![icon(Icon::Warning, layout::ICON_SM, c(b.p.warning)), t(text, ts::CAPTION, c(b.p.text))].spacing(space::S2).align_y(Alignment::Center);
    column![
        in_column(r).padding(Padding::from([space::S1_5, conv::SIDE_PAD])).style(style::warning_banner(b)),
        hairline(b)
    ]
    .into()
}

fn empty(app: &App) -> Element<'_, Msg> {
    let b = app.brasa;
    stack![
        contour_bg(&app.bg_cache, 0, false, b),
        container(
            column![
                t1("Elige una conversación", ts::DISPLAY, c(b.p.text)),
                t1("O busca a alguien con Ctrl+K.", ts::UI, c(b.p.text_muted)),
            ]
            .spacing(space::S2)
            .align_x(Alignment::Center)
        )
        .center(Length::Fill),
    ]
    .into()
}

// ------------------------------------------------------------------ cabecera

fn header<'a>(app: &'a App, cv: &'a Conversation, narrow: bool) -> Element<'a, Msg> {
    let b = app.brasa;
    let peer = cv.peer;
    let d = app.dialog(peer);
    let title = d.map_or("…", |d| d.title.as_str());
    let mut left = row![].spacing(space::S3).align_y(Alignment::Center);
    if narrow {
        left = left.push(icon_button(Icon::Back, layout::ICON_MD, c(b.p.text), Some(Msg::CloseChat), b));
    }
    left = left.push(avatar_photo(title, App::peer_seed(peer), layout::AVATAR_MD, None, app.avatar_handle(peer, d.and_then(|d| d.photo)), b));

    let mut name = row![].spacing(space::S1).align_y(Alignment::Center);
    if matches!(peer, PeerRef::Secret(_)) {
        name = name.push(icon_fill(Icon::Shield, layout::ICON_SM, c(b.p.success)));
    }
    name = name.push(t1(format::ellipsize(&format::plain_letters(title), 48), ts::TITLE, c(b.p.text)));
    if matches!(peer, PeerRef::Secret(_)) {
        name = name.push(experimental_pill(b));
    }
    if d.is_some_and(|d| d.muted) {
        name = name.push(icon(Icon::Muted, 14.0, c(b.p.text_muted)));
    }
    let (status, status_color) = header_status(app, peer);
    left = left.push(column![name, t1(status, ts::META, status_color)].spacing(0));

    let mut actions = row![].spacing(space::S1).align_y(Alignment::Center);
    match peer {
        PeerRef::Secret(s) => {
            let ttl_on = app.secrets.get(&s).is_some_and(|c| c.ttl_secs.is_some());
            let timer: Element<'_, Msg> =
                if ttl_on { icon_fill(Icon::Timer, layout::ICON_MD, c(b.p.accent_text)) } else { icon(Icon::Timer, layout::ICON_MD, c(b.p.text)) };
            actions = actions.push(button(timer).padding(8).style(style::ghost(b, radius::SM)).on_press(Msg::ShowTtl));
            actions = actions.push(icon_button(Icon::Fingerprint, layout::ICON_MD, c(b.p.text), Some(Msg::ShowKey), b));
        }
        PeerRef::User(u) if Some(u) != app.me_id() && !app.users.get(&u).is_some_and(|u| u.is_bot) => {
            actions = actions.push(icon_button(Icon::LockKey, layout::ICON_MD, c(b.p.text), Some(Msg::NewSecret(Some(u))), b));
        }
        _ => {}
    }
    let pinned = d.is_some_and(|d| d.pinned_order.is_some());
    let pin_icon: Element<'_, Msg> = if pinned {
        icon_fill(Icon::Pin, layout::ICON_MD, c(b.p.accent_text))
    } else {
        icon(Icon::Pin, layout::ICON_MD, c(b.p.text))
    };
    actions = actions.push(button(pin_icon).padding(8).style(style::ghost(b, radius::SM)).on_press(Msg::TogglePin));
    let muted = d.is_some_and(|d| d.muted);
    actions = actions.push(icon_button(if muted { Icon::Muted } else { Icon::Bell }, layout::ICON_MD, c(b.p.text), Some(Msg::ToggleMute), b));

    let bar = in_column(row![left, hfill(), actions].align_y(Alignment::Center))
        .height(Length::Fixed(layout::HEADER_HEIGHT))
        .align_y(Vertical::Center)
        .style(style::surface(b));
    column![bar, hairline(b)].into()
}

fn hairline<'a>(b: Brasa) -> Element<'a, Msg> {
    container(Space::new().height(Length::Fixed(1.0)).width(Length::Fill)).style(style::fill(c(b.p.border))).into()
}

fn header_status(app: &App, peer: PeerRef) -> (String, Color) {
    let b = app.brasa;
    let muted = c(b.p.text_muted);
    match app.connection {
        tc_core::ConnectionState::Online => {}
        tc_core::ConnectionState::Connecting | tc_core::ConnectionState::Offline => return ("Conectando…".into(), c(b.p.warning)),
        tc_core::ConnectionState::Updating => return ("Actualizando…".into(), c(b.p.warning)),
    }
    if let Some((user, _)) = app.typing.get(&peer) {
        let who = if matches!(peer, PeerRef::Group(_)) { format!("{} está ", app.user_name(*user)) } else { String::new() };
        let dots = if app.anim.pref.is_reduced() { "…".to_string() } else { ".".repeat(app.dots as usize + 1) };
        return (format!("{who}escribiendo{dots}"), c(b.p.accent_text));
    }
    let now = crate::app::now();
    match peer {
        PeerRef::User(u) => match app.users.get(&u) {
            Some(u) if u.status == tc_core::UserStatus::Online => ("en línea".into(), c(b.p.accent_text)),
            Some(u) => (format::user_status(u.status, now, &app.tz), muted),
            None => (String::new(), muted),
        },
        PeerRef::Group(g) => match app.groups.get(&g) {
            Some(g) if g.kind == GroupKind::Channel => (format!("{} suscriptores", g.member_count), muted),
            Some(g) => (format!("{} miembros", g.member_count), muted),
            None => (String::new(), muted),
        },
        PeerRef::Secret(s) => {
            let chat = app.secrets.get(&s);
            let state = match chat.map(|c| c.state) {
                Some(SecretChatState::Waiting) => "esperando aceptación".to_string(),
                Some(SecretChatState::Discarded) => "cerrado".to_string(),
                _ => "cifrado de extremo a extremo".to_string(),
            };
            match chat.and_then(|c| c.ttl_secs) {
                Some(ttl) => (format!("{state} · se borra tras {}", format::ttl(ttl)), muted),
                None => (state, muted),
            }
        }
    }
}

// ------------------------------------------------------------------ cuerpo (scroll virtual)

fn body<'a>(app: &'a App, cv: &'a Conversation) -> Element<'a, Msg> {
    let b = app.brasa;
    let secret = matches!(cv.peer, PeerRef::Secret(_));
    responsive(move |size| {
        let col_w = (size.width - 2.0 * conv::SIDE_PAD).clamp(200.0, layout::CHAT_COLUMN_MAX);
        cv.last_col.set(col_w);
        cv.last_viewport.set(size.height);
        let switch = app.anim.value(AnimKey::ChatSwitch, 1.0);
        let x_shift = if app.anim.pref.is_reduced() { 0.0 } else { (1.0 - switch) * 12.0 };

        let content: Element<'_, Msg> = if cv.loading {
            container(t1("Cargando mensajes…", ts::UI, c(b.p.text_muted))).center(Length::Fill).into()
        } else if let Some(e) = &cv.error {
            container(t(e.as_str(), ts::UI, c(b.p.danger))).center(Length::Fill).into()
        } else {
            messages(app, cv, col_w, size.height, x_shift)
        };

        let mut layers = vec![contour_bg(&app.bg_cache, App::peer_seed(cv.peer), secret, b), content];
        if cv.from_bottom > 320.0 || cv.new_below > 0 {
            layers.push(jump_button(app, cv.new_below));
        }
        if switch < 0.999 {
            // Entrada del chat nuevo: una capa del color de fondo que se desvanece (= opacidad 0 → 1).
            layers.push(container(Space::new()).width(Length::Fill).height(Length::Fill).style(style::fill(fade(c(b.p.bg), 1.0 - switch))).into());
        }
        stack(layers).width(Length::Fill).height(Length::Fill).into()
    })
    .into()
}

fn jump_button<'a>(app: &App, n: u32) -> Element<'a, Msg> {
    let b = app.brasa;
    let circle = button(container(icon(Icon::Down, layout::ICON_MD, c(b.p.text))).center(Length::Fixed(40.0)))
        .padding(0)
        .width(Length::Fixed(40.0))
        .height(Length::Fixed(40.0))
        .style(move |_, status| iced::widget::button::Style {
            background: Some(c(if status == iced::widget::button::Status::Hovered { b.p.surface_hover } else { b.p.surface_raised }).into()),
            text_color: c(b.p.text),
            border: iced::Border { radius: style::r(radius::PILL), width: 1.0, color: c(b.p.border) },
            shadow: style::shadow(b, 2),
            snap: true,
        })
        .on_press(Msg::JumpToBottom);
    let el: Element<'_, Msg> = if n > 0 {
        column![container(crate::widgets::badge(n, false, b)).width(Length::Fixed(40.0)).align_x(Horizontal::Center), circle]
            .spacing(-6.0)
            .into()
    } else {
        circle.into()
    };
    // Abajo a la derecha de la columna de mensajes (encima del botón enviar).
    in_column(container(el).width(Length::Fill).align_x(Horizontal::Right))
        .height(Length::Fill)
        .align_y(Vertical::Bottom)
        .padding(Padding::from([space::S4, conv::SIDE_PAD]))
        .into()
}

/// Construye los elementos (fechas, marca de no leídos, mensajes) con sus alturas medidas.
fn build_items(app: &App, cv: &Conversation, col_w: f32) -> (Vec<Item>, Vec<MsgLayout>) {
    let group = app.is_group_chat(cv.peer);
    let bmax = bubble_max(col_w, group);
    let mut cache = cv.cache.borrow_mut();
    cache.reset_if_width(col_w);
    let msgs = &cv.messages;
    let days: Vec<i32> = msgs.iter().map(|m| cache.day(m, || format::day_key(m.date, &app.tz))).collect();
    let grouped_prev: Vec<bool> = (0..msgs.len())
        .map(|i| i > 0 && Some(msgs[i].id) != cv.unread_from && conv::same_group(&msgs[i - 1], &msgs[i], days[i - 1], days[i]))
        .collect();
    let mut items = Vec::with_capacity(msgs.len() + 16);
    let mut layouts = Vec::with_capacity(msgs.len());
    items.push(Item::Edge { loading: !cv.reached_start });
    for (i, m) in msgs.iter().enumerate() {
        if i == 0 || days[i] != days[i - 1] {
            items.push(Item::Date { ts: m.date });
        }
        if Some(m.id) == cv.unread_from {
            items.push(Item::UnreadMark);
        }
        let gp = grouped_prev[i];
        let gn = grouped_prev.get(i + 1).copied().unwrap_or(false);
        let pos = match (gp, gn) {
            (false, false) => GroupPos::Single,
            (false, true) => GroupPos::First,
            (true, true) => GroupPos::Middle,
            (true, false) => GroupPos::Last,
        };
        let show_name = group && !m.outgoing && !gp;
        let sig = conv::signature(m, show_name);
        let l = cache.get_or(m, sig, || {
            let name = m.sender.map(|s| format::plain_letters(&app.user_name(s)).into_owned()).unwrap_or_default();
            let fwd = m.forwarded_from.as_ref().map(|f| format!("Reenviado de {f}"));
            let meta = meta_label(app, m);
            let input = LayoutInput { show_name, name: &name, forwarded: fwd.as_deref(), meta: &meta, bubble_max: bmax, day: days[i] };
            conv::layout_message(m, &input, &IcedMeasurer)
        });
        let gap = if gp { conv::GAP_IN_GROUP } else { conv::GAP_GROUP };
        let react = if m.reactions.is_empty() { 0.0 } else { conv::REACT_GAP + conv::REACT_H };
        items.push(Item::Msg { idx: i, pos, gap, avatar: group && !m.outgoing && !gn, height: gap + l.bubble.height + react });
        layouts.push(l);
    }
    (items, layouts)
}

fn meta_label(app: &App, m: &Message) -> String {
    let time = format::hhmm(m.date, &app.tz);
    if m.edit_date.is_some() { format!("editado {time}") } else { time }
}

fn messages<'a>(app: &'a App, cv: &'a Conversation, col_w: f32, vh: f32, x_shift: f32) -> Element<'a, Msg> {
    let b = app.brasa;
    let (items, layouts) = build_items(app, cv, col_w);
    let mut prefix = Vec::with_capacity(items.len() + 1);
    let mut acc = 0.0;
    prefix.push(0.0);
    for it in &items {
        acc += it.height();
        prefix.push(acc);
    }
    // Si el contenido no llena la vista, los mensajes se apoyan abajo.
    let filler = (vh - acc - space::S2).max(0.0);
    let win = conv::visible_window(&prefix, vh, cv.from_bottom, vh);

    let mut col = column![Space::new().height(Length::Fixed(win.top_space + filler))].width(Length::Fill).align_x(Alignment::Center);
    let mut li = items[..win.start].iter().filter(|i| matches!(i, Item::Msg { .. })).count();
    let idx = items[win.start..win.end].iter().filter_map(|it| match it {
        Item::Msg { idx, .. } => Some(*idx),
        _ => None,
    });
    cv.visible.set(idx.clone().min().zip(idx.max()).map(|(a, z)| (a, z + 1)));
    cv.anim_visible.set(items[win.start..win.end].iter().any(|it| match it {
        Item::Msg { idx, .. } => cv.messages[*idx].sticker.as_ref().is_some_and(crate::stickers::is_animated),
        _ => false,
    }));
    for it in &items[win.start..win.end] {
        let el: Element<'_, Msg> = match *it {
            Item::Edge { loading } => edge(app, cv, loading),
            Item::Date { ts } => date_pill(app, ts),
            Item::UnreadMark => unread_mark(b),
            Item::Msg { idx, pos, gap, avatar, height } => {
                let l = layouts[li];
                li += 1;
                message_row(app, cv, &cv.messages[idx], &l, pos, gap, avatar, height, col_w)
            }
        };
        col = col.push(container(el).width(Length::Fixed(col_w)).height(Length::Fixed(it.height())).clip(true));
    }
    col = col.push(Space::new().height(Length::Fixed(win.bottom_space + space::S2)));
    let inner = container(col).padding(Padding { left: x_shift, ..Padding::ZERO }).width(Length::Fill);
    scrollable(inner)
        .id(CONV_SCROLL_ID)
        // `direction` antes que `anchor_bottom`: el ancla vive en la barra y `direction` la reemplaza.
        .direction(scrollable::Direction::Vertical(scrollable::Scrollbar::new().width(6).scroller_width(6).margin(2)))
        .anchor_bottom()
        .on_scroll(Msg::ConvScrolled)
        .style(style::thin_scroll(b))
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
}

fn edge<'a>(app: &App, cv: &Conversation, loading: bool) -> Element<'a, Msg> {
    let b = app.brasa;
    let label = if loading {
        "Cargando mensajes anteriores…".to_string()
    } else if matches!(cv.peer, PeerRef::Secret(_)) {
        "Chat secreto: cifrado de extremo a extremo y solo en este dispositivo.".to_string()
    } else {
        "Aquí empieza la conversación".to_string()
    };
    container(container(t1(label, ts::CAPTION, c(b.p.text_muted))).padding([4, 12]).style(style::pill_raised(b))).center(Length::Fill).into()
}

fn date_pill<'a>(app: &App, ts_: tc_core::Timestamp) -> Element<'a, Msg> {
    let b = app.brasa;
    let label = format::date_separator(ts_, crate::app::now(), &app.tz);
    container(container(t1(label, ts::CAPTION, c(b.p.text_muted))).padding([4, 12]).style(style::pill_raised(b))).center(Length::Fill).into()
}

fn unread_mark<'a>(b: Brasa) -> Element<'a, Msg> {
    let line = || container(Space::new().height(Length::Fixed(1.0)).width(Length::Fill)).style(style::fill(c(b.p.accent)));
    container(row![line(), t1("Sin leer desde aquí", ts::CAPTION, c(b.p.accent_text)), line()].spacing(space::S2).align_y(Alignment::Center))
        .center_y(Length::Fill)
        .width(Length::Fill)
        .into()
}

#[allow(clippy::too_many_arguments)]
fn message_row<'a>(
    app: &'a App,
    cv: &'a Conversation,
    m: &'a Message,
    l: &MsgLayout,
    pos: GroupPos,
    gap: f32,
    show_avatar: bool,
    height: f32,
    col_w: f32,
) -> Element<'a, Msg> {
    let b = app.brasa;
    let key = MsgKey::of(m);
    let opacity = app.anim.value(AnimKey::BubbleOpacity(key), 1.0);
    let y = app.anim.value(AnimKey::BubbleY(key), 0.0).max(0.0);
    let group = app.is_group_chat(cv.peer);
    let hovered = app.hover == Some(m.id);
    let menu_open = app.reactions_for == Some(m.id);

    let bubble_el = bubble(app, cv, m, l, pos, opacity, menu_open);
    let extras = extras(app, m, hovered || menu_open);

    let mut line = row![].align_y(Alignment::End).spacing(space::S2);
    if m.outgoing {
        line = line.push(hfill()).push(extras).push(bubble_el);
    } else {
        if group {
            let av: Element<'_, Msg> = if show_avatar {
                let name = m.sender.map(|s| app.user_name(s)).unwrap_or_default();
                avatar_photo(&name, m.sender.map_or(0, |s| s.0), conv::GROUP_AVATAR, None, m.sender.and_then(|s| app.user_avatar(s)), b)
            } else {
                Space::new().width(Length::Fixed(conv::GROUP_AVATAR)).into()
            };
            line = line.push(av);
        }
        line = line.push(bubble_el).push(extras).push(hfill());
    }
    let line = mouse_area(container(line).width(Length::Fixed(col_w)))
        .on_enter(Msg::Hover(Some(m.id)))
        .on_exit(Msg::Hover(None))
        .on_right_press(if conv::is_local(m) { Msg::Hover(Some(m.id)) } else { Msg::ToggleReactions(m.id) });

    let mut col = column![Space::new().height(Length::Fixed(gap + y)), line].width(Length::Fixed(col_w)).height(Length::Fixed(height));
    if !m.reactions.is_empty() {
        let mut chips = row![].spacing(space::S1);
        for r in &m.reactions {
            let label = if r.count > 1 { format!("{} {}", r.emoji, r.count) } else { r.emoji.clone() };
            let next = if r.mine { None } else { Some(r.emoji.clone()) };
            chips = chips.push(
                button(container(t1(label, ts::META, c(b.p.text))).center_y(Length::Fill))
                    .height(Length::Fixed(conv::REACT_H))
                    .padding(Padding::from([0.0, space::S2]))
                    .style(style::reaction_chip(b, r.mine))
                    .on_press_maybe((!conv::is_local(m)).then_some(Msg::React(m.id, next))),
            );
        }
        let lead = if !m.outgoing && group { conv::GROUP_AVATAR_COL } else { 0.0 };
        let chips_row = if m.outgoing { row![hfill(), chips] } else { row![hgap(lead), chips] };
        col = col.push(vgap(conv::REACT_GAP)).push(chips_row.width(Length::Fixed(col_w)));
    }
    col.into()
}

/// Barra de acciones al pasar el ratón (o el selector de reacciones), y «Reintentar» si falló.
fn extras<'a>(app: &'a App, m: &'a Message, show: bool) -> Element<'a, Msg> {
    let b = app.brasa;
    if m.state == SendState::Failed {
        return row![
            icon(Icon::Warning, layout::ICON_MD, c(b.p.danger)),
            button(t1("Reintentar", ts::UI_STRONG, c(b.p.accent_text)))
                .padding([4, 8])
                .style(style::ghost(b, radius::SM))
                .on_press_maybe(m.random_id.map(Msg::Retry)),
        ]
        .align_y(Alignment::Center)
        .spacing(space::S1)
        .into();
    }
    if !show || conv::is_local(m) {
        return Space::new().width(Length::Shrink).into();
    }
    let blocked = app.conv.as_ref().and_then(|c| app.write_block(c.peer)).is_some();
    let bar: Element<'_, Msg> = if app.reactions_for == Some(m.id) {
        // Solo las que admite este chat (ver `app/reacts.rs`); mientras se piden, «…».
        let mut r = row![].spacing(space::S0_5).align_y(Alignment::Center);
        match app.reaction_choices(m.peer) {
            None => r = r.push(container(t1("…", ts::UI, c(b.p.text_muted))).center(Length::Fixed(28.0))),
            Some([]) => r = r.push(container(t1("No se puede reaccionar aquí", ts::CAPTION, c(b.p.text_muted))).padding([4, 8])),
            Some(list) => {
                for e in list.iter().take(crate::app::MAX_REACTION_CHOICES) {
                    let mine = m.reactions.iter().any(|x| x.mine && x.emoji == *e);
                    r = r.push(
                        button(container(t1(e.as_str(), TextStyle { size: 18.0, line: 22.0, ..ts::UI }, c(b.p.text))).center(Length::Fixed(28.0)))
                            .padding(2)
                            .style(style::ghost(b, radius::SM))
                            .on_press(Msg::React(m.id, if mine { None } else { Some(e.clone()) })),
                    );
                }
            }
        }
        r.into()
    } else {
        let ib = |i: Icon, msg: Msg| icon_button(i, 16.0, c(b.p.text_muted), Some(msg), b);
        let mut r = row![].spacing(0);
        if app.can_react(m.peer) {
            r = r.push(ib(Icon::Smiley, Msg::ToggleReactions(m.id)));
        }
        if !blocked {
            r = r.push(ib(Icon::Reply, Msg::ReplyTo(m.id)));
        }
        if !m.text.is_empty() {
            r = r.push(ib(Icon::Copy, Msg::Copy(m.id)));
        }
        if m.outgoing && !m.text.is_empty() && m.sticker.is_none() && !blocked {
            r = r.push(ib(Icon::Edit, Msg::StartEdit(m.id)));
        }
        r = r.push(icon_button(Icon::Trash, 16.0, c(b.p.danger), Some(Msg::AskDelete(m.id)), b));
        r.into()
    };
    container(bar).padding(2).style(style::raised(b, 2, radius::MD)).into()
}

fn meta_row<'a>(app: &App, m: &Message, color: Color) -> Element<'a, Msg> {
    let b = app.brasa;
    let mut r = row![].spacing(conv::META_ICON_GAP).align_y(Alignment::Center);
    if m.ttl_secs.is_some() {
        r = r.push(icon(Icon::Timer, conv::META_ICON, color));
    }
    r = r.push(t1(meta_label(app, m), ts::CAPTION, color));
    if m.outgoing {
        let read = app.conv.as_ref().and_then(|cv| app.dialog(cv.peer)).is_some_and(|d| m.id <= d.read_outbox_max_id) && !conv::is_local(m);
        let (ic, col) = match m.state {
            SendState::Pending => (Icon::Timer, color),
            SendState::Failed => (Icon::Warning, c(b.p.danger)),
            SendState::Sent if read => (Icon::Checks, c(b.p.tick_read)),
            SendState::Sent => (Icon::Check, color),
        };
        r = r.push(icon(ic, conv::META_ICON, col));
    }
    r.into()
}

fn bubble<'a>(app: &'a App, cv: &'a Conversation, m: &'a Message, l: &MsgLayout, pos: GroupPos, opacity: f32, menu_open: bool) -> Element<'a, Msg> {
    let b = app.brasa;
    let (fg, meta_c) = if m.outgoing { (b.p.on_bubble_out, b.p.bubble_out_meta) } else { (b.p.on_bubble_in, b.p.bubble_in_meta) };
    let (fg, meta_c) = (fade(c(fg), opacity), fade(c(meta_c), opacity));

    match l.body {
        Body::Sticker | Body::Emoji => {
            let big: Element<'_, Msg> = match (&m.sticker, l.body) {
                (Some(s), _) => container(super::stickers::art(app, s, conv::STICKER, m.id.0.unsigned_abs() as usize, fg))
                    .center(Length::Fixed(conv::STICKER))
                    .into(),
                _ => container(t(m.text.trim(), ts::EMOJI_ONLY, fg)).width(Length::Fixed(l.text.width + 1.0)).height(Length::Fixed(l.text.height)).into(),
            };
            let pill = container(meta_row(app, m, c(b.p.text_muted)))
                .height(Length::Fixed(conv::META_PILL_H))
                .padding(Padding::from([0.0, space::S2]))
                .align_y(Vertical::Center)
                .style(style::pill_raised(b));
            let align = if m.outgoing { Alignment::End } else { Alignment::Start };
            return column![big, vgap(conv::META_GAP), pill]
                .align_x(align)
                .width(Length::Fixed(l.bubble.width))
                .height(Length::Fixed(l.bubble.height))
                .into();
        }
        Body::Bubble => {}
    }

    let mut col = column![].spacing(0);
    if l.show_name {
        // Mismo texto que se midió en `build_items`.
        let name = m.sender.map(|s| format::plain_letters(&app.user_name(s)).into_owned()).unwrap_or_default();
        let nc = fade(b.name_color(m.sender.map_or(0, |s| s.0)), opacity);
        col = col.push(container(t1(name, with_weight(ts::META, 600), nc)).height(Length::Fixed(conv::NAME_H)));
    }
    if let Some(f) = &m.forwarded_from {
        col = col.push(container(t1(format!("Reenviado de {f}"), ts::META, meta_c)).height(Length::Fixed(conv::FWD_H)));
    }
    if let Some(rid) = m.reply_to {
        let (who, what, seed) = match cv.find(rid) {
            Some(r) => (
                r.sender.map(|s| app.user_name(s)).unwrap_or_else(|| app.dialog(cv.peer).map(|d| d.title.clone()).unwrap_or_default()),
                format::preview(r),
                r.sender.map_or(0, |s| s.0),
            ),
            None => ("Mensaje".to_string(), "No está cargado en esta vista".to_string(), 0),
        };
        let quote_w = (l.content_w - 3.0 - space::S2 - space::S2).max(40.0);
        let (who, what) = (
            crate::measure::ellipsize_to(&format::plain_letters(&who), with_weight(ts::CAPTION, 600), quote_w),
            crate::measure::ellipsize_to(&format::plain_letters(&what), ts::META, quote_w),
        );
        let nc = fade(b.name_color(seed), opacity);
        let bar = container(Space::new().width(Length::Fixed(3.0)).height(Length::Fill)).style(style::rounded_fill(nc, style::r(2.0)));
        let quote = container(
            row![bar, column![t1(who, with_weight(ts::CAPTION, 600), nc), t1(what, ts::META, fg)]]
                .spacing(space::S2)
                .height(Length::Fill),
        )
        .padding(Padding::from([space::S0_5, space::S2]).left(0.0))
        .width(Length::Fixed(l.content_w))
        .height(Length::Fixed(conv::REPLY_H))
        .clip(true)
        .style(style::quote(b));
        col = col.push(quote).push(vgap(conv::REPLY_GAP));
    }
    if let (Some(md), Some(sz)) = (&m.media, l.media) {
        col = col.push(media_box(app, cv.peer, m, md, sz, opacity, meta_c));
        if !m.text.trim().is_empty() {
            col = col.push(vgap(conv::MEDIA_GAP));
        }
    }
    if !m.text.trim().is_empty() {
        let link = if m.outgoing { None } else { Some(fade(c(b.p.accent_text), opacity)) };
        let body: Element<'_, Msg> = if m.entities.is_empty() {
            t(m.text.as_str(), ts::BODY, fg).into()
        } else {
            rich_text(to_spans(&conv::runs(m), ts::BODY, Some(fg), link))
                .size(Pixels(ts::BODY.size))
                .line_height(LineHeight::Absolute(Pixels(ts::BODY.line)))
                .font(style::font_of(ts::BODY))
                .wrapping(crate::measure::WRAP)
                .into()
        };
        let body = container(body).width(Length::Fixed(l.text.width + 1.0)).height(Length::Fixed(l.text.height));
        if l.meta_inline {
            // La hora (caption 12/16) sube META_BASELINE px para compartir línea de base con el
            // texto (body 15/22) en lugar de quedar por debajo.
            let meta = container(meta_row(app, m, meta_c)).padding(Padding { bottom: conv::META_BASELINE, ..Padding::ZERO });
            col = col.push(row![body, hfill(), meta].align_y(Alignment::End).width(Length::Fixed(l.content_w + 1.0)));
        } else {
            col = col.push(body);
        }
    }
    if !l.meta_inline {
        col = col.push(vgap(conv::META_GAP)).push(
            container(meta_row(app, m, meta_c)).height(Length::Fixed(conv::META_H)).width(Length::Fixed(l.content_w + 1.0)).align_x(Horizontal::Right),
        );
    }
    container(col)
        .padding(Padding::from([conv::PAD_Y, conv::PAD_X]))
        .width(Length::Fixed(l.bubble.width + 1.0))
        .height(Length::Fixed(l.bubble.height))
        .clip(true)
        .style(style::bubble(b, m.outgoing, pos, opacity, menu_open))
        .into()
}

/// Medio en la burbuja: la foto (o la miniatura del vídeo) si ya está cargada; si no, el
/// marcador de Brasa (icono y nombre o dimensiones) en el mismo hueco, para que nada salte al llegar.
/// Las fotos abren el visor al pulsarlas.
fn media_box<'a>(app: &'a App, peer: PeerRef, m: &'a Message, md: &'a tc_core::Media, sz: iced::Size, opacity: f32, meta_c: Color) -> Element<'a, Msg> {
    use tc_core::MediaKind;
    if crate::audio::is_audio(md) {
        return super::audio::bubble(app, peer, m, md, sz, opacity);
    }
    let b = app.brasa;
    // Los vídeos redondos se recortan en círculo.
    let rad = if md.kind == MediaKind::VideoNote { sz.width / 2.0 } else { radius::MD };
    let handle = md.file.as_ref().and_then(|f| app.images.handle(&ImageKey::Thumb { peer, msg: m.id, file: f.id }));
    let el: Element<'a, Msg> = match handle {
        Some(h) => {
            let img = image(h)
                .width(Length::Fixed(sz.width))
                .height(Length::Fixed(sz.height))
                .content_fit(ContentFit::Cover)
                .border_radius(style::r(rad))
                .opacity(opacity);
            if matches!(md.kind, MediaKind::Video | MediaKind::Gif | MediaKind::VideoNote) {
                let play = container(icon_fill(Icon::Play, 22.0, c(b.p.on_accent)))
                    .center(Length::Fixed(44.0))
                    .style(style::rounded_fill(Color { a: 0.55, ..Color::BLACK }, style::r(radius::PILL)));
                stack![img, container(play).center_x(Length::Fixed(sz.width)).center_y(Length::Fixed(sz.height))].into()
            } else {
                img.into()
            }
        }
        None => {
            let ic = match md.kind {
                MediaKind::Photo => Icon::Image,
                MediaKind::Video | MediaKind::Gif | MediaKind::VideoNote => Icon::Play,
                _ => Icon::File,
            };
            let label = md.file_name.clone().unwrap_or_else(|| if md.width > 0 { format!("{} × {}", md.width, md.height) } else { format::preview(m) });
            container(column![icon(ic, 32.0, meta_c), t1(label, ts::CAPTION, meta_c)].spacing(space::S1).align_x(Alignment::Center))
                .center_x(Length::Fixed(sz.width))
                .center_y(Length::Fixed(sz.height))
                .style(style::rounded_fill(fade(c(b.p.surface_hover), opacity), style::r(rad)))
                .into()
        }
    };
    if md.kind == MediaKind::Photo && md.file.is_some() && !conv::is_local(m) {
        mouse_area(el).interaction(iced::mouse::Interaction::Pointer).on_press(Msg::OpenPhoto { peer, msg: m.id }).into()
    } else {
        el
    }
}

// ------------------------------------------------------------------ compositor

fn composer<'a>(app: &'a App, cv: &'a Conversation) -> Element<'a, Msg> {
    let b = app.brasa;
    if let Some(chat) = app.secret_invite(cv.peer) {
        let who = app.user_name(chat.user);
        let bar = container(container(
            row![
                icon_fill(Icon::Shield, layout::ICON_MD, c(b.p.success)),
                t(format!("{who} te invita a un chat secreto (experimental)."), ts::UI, c(b.p.text)),
                hfill(),
                button(t1("Rechazar", ts::UI_STRONG, c(b.p.text))).padding([8, 16]).style(style::ghost(b, radius::MD)).on_press(Msg::DiscardSecret),
                button(t1("Aceptar", ts::UI_STRONG, c(b.p.on_accent))).padding([8, 16]).style(style::primary(b, radius::MD)).on_press(Msg::AcceptSecret),
            ]
            .spacing(space::S2)
            .align_y(Alignment::Center),
        )
        .max_width(layout::CHAT_COLUMN_MAX))
        .padding(Padding::from([space::S2, space::S4]))
        .center_x(Length::Fill)
        .height(Length::Fixed(layout::COMPOSER_MIN_HEIGHT + space::S2))
        .align_y(Vertical::Center)
        .style(style::surface(b));
        return column![hairline(b), bar].into();
    }
    if let Some(reason) = app.write_block(cv.peer) {
        let bar = container(t1(reason, ts::UI, c(b.p.text_muted)))
            .center_x(Length::Fill)
            .height(Length::Fixed(layout::COMPOSER_MIN_HEIGHT))
            .align_y(Vertical::Center)
            .style(style::surface(b));
        return column![hairline(b), bar].into();
    }
    let empty = app.composer.text().trim().is_empty();
    let editing = matches!(app.compose, Compose::Edit(_));
    let picker_open = app.picker.open;
    let editor = text_editor(&app.composer)
        .id(COMPOSER_ID)
        .placeholder(if editing { "Edita el mensaje…" } else { "Escribe un mensaje…" })
        .on_action(Msg::Edit)
        .font(style::font_of(ts::BODY))
        .size(Pixels(ts::BODY.size))
        .line_height(LineHeight::Absolute(Pixels(ts::BODY.line)))
        .padding(Padding::from([7.0, 14.0]))
        .height(Length::Shrink)
        .min_height(36.0)
        .max_height(ts::BODY.line * tokens::layout::COMPOSER_MAX_LINES as f32 + 14.0)
        .style(style::composer(b))
        .key_binding(move |kp| {
            if !matches!(kp.status, text_editor::Status::Focused { .. }) {
                return text_editor::Binding::from_key_press(kp);
            }
            let nav = |n: Nav| Some(text_editor::Binding::Custom(Msg::PickerNav(n)));
            match kp.key.as_ref() {
                Key::Character("e") if kp.modifiers.command() => Some(text_editor::Binding::Custom(Msg::TogglePicker)),
                // Con el selector abierto, las flechas, Enter y Tab mueven el cursor de la rejilla.
                Key::Named(Named::ArrowLeft) if picker_open => nav(Nav::Left),
                Key::Named(Named::ArrowRight) if picker_open => nav(Nav::Right),
                Key::Named(Named::ArrowUp) if picker_open => nav(Nav::Up),
                Key::Named(Named::ArrowDown) if picker_open => nav(Nav::Down),
                Key::Named(Named::Tab) if picker_open => nav(if kp.modifiers.shift() { Nav::PrevTab } else { Nav::NextTab }),
                Key::Named(Named::Enter) if picker_open && app.picker.cursor.is_some() => nav(Nav::Activate),
                Key::Named(Named::Enter) if !kp.modifiers.shift() => Some(text_editor::Binding::Custom(Msg::Send)),
                Key::Named(Named::Escape) => Some(text_editor::Binding::Custom(Msg::Escape)),
                Key::Named(Named::ArrowUp) if empty => Some(text_editor::Binding::Custom(Msg::EditLast)),
                _ => text_editor::Binding::from_key_press(kp),
            }
        });

    let scale = app.anim.value(AnimKey::SendButton, 1.0);
    let s = 36.0 * scale;
    let send_icon = if editing { Icon::Check } else { Icon::Send };
    let send = if empty {
        button(container(icon(send_icon, 18.0, c(b.p.text_muted))).center(Length::Fill)).style(style::send_disabled(b))
    } else {
        button(container(icon_fill(send_icon, 18.0 * scale, c(b.p.on_accent))).center(Length::Fill)).style(style::primary(b, radius::PILL)).on_press(Msg::Send)
    }
    .padding(0)
    .width(Length::Fixed(s))
    .height(Length::Fixed(s));
    let send = container(send).center(Length::Fixed(40.0));

    let mut col = column![];
    if let Some(strip) = compose_strip(app, cv) {
        col = col.push(strip);
    }
    let sticker_btn: Element<'_, Msg> = if editing {
        Space::new().width(Length::Fixed(0.0)).into()
    } else {
        let ic: Element<'_, Msg> = if picker_open {
            icon_fill(Icon::Sticker, layout::ICON_MD, c(b.p.accent_text))
        } else {
            icon(Icon::Sticker, layout::ICON_MD, c(b.p.text_muted))
        };
        container(button(ic).padding(8).style(style::ghost(b, radius::PILL)).on_press(Msg::TogglePicker)).center(Length::Fixed(40.0)).into()
    };
    col = col.push(row![sticker_btn, editor, send].spacing(space::S2).align_y(Alignment::End));
    let bar = container(col.spacing(space::S2).max_width(layout::CHAT_COLUMN_MAX))
        .padding(Padding::from([space::S2, space::S4]))
        .center_x(Length::Fill)
        .style(style::surface(b));
    column![hairline(b), bar].into()
}

/// Franja «respondiendo a…» / «editando».
fn compose_strip<'a>(app: &'a App, cv: &'a Conversation) -> Option<Element<'a, Msg>> {
    let b = app.brasa;
    let (ic, title, id) = match app.compose {
        Compose::Normal => return None,
        Compose::Reply(id) => {
            let who = cv.find(id).and_then(|m| m.sender).map(|s| format::plain_letters(&app.user_name(s)).into_owned()).unwrap_or_else(|| "mensaje".into());
            (Icon::Reply, format!("Respondiendo a {who}"), id)
        }
        Compose::Edit(id) => (Icon::Edit, "Editando el mensaje".to_string(), id),
    };
    let what = cv.find(id).map(|m| format::plain_letters(&format::preview(m)).into_owned()).unwrap_or_default();
    let bar = container(Space::new().width(Length::Fixed(3.0)).height(Length::Fixed(36.0))).style(style::rounded_fill(c(b.p.accent), style::r(2.0)));
    Some(
        row![
            bar,
            icon(ic, layout::ICON_MD, c(b.p.accent_text)),
            column![t1(title, with_weight(ts::CAPTION, 600), c(b.p.accent_text)), t1(format::ellipsize(&what, 70), ts::META, c(b.p.text_muted))],
            hfill(),
            icon_button(Icon::Close, layout::ICON_SM, c(b.p.text_muted), Some(Msg::CancelCompose), b),
        ]
        .spacing(space::S2)
        .align_y(Alignment::Center)
        .into(),
    )
}
