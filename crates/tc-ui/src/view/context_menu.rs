//! Vista de los menús contextuales de chats y mensajes (§8.8: ítems de 32 px sobre
//! `surface-raised`, elevación 2, radio `md`, destructivos en `danger`, atajo en `caption`),
//! las confirmaciones, «Ver información», el selector de «Reenviar», el aviso con «Deshacer», la
//! barra de selección de mensajes y la fila «Archivados». La lógica está en `app/chat_actions.rs`.

use std::cell::Cell;

use iced::advanced::widget::{Operation, Tree};
use iced::advanced::{Clipboard, Layout, Shell, Widget, layout, mouse as amouse, overlay, renderer};
use iced::alignment::{Horizontal, Vertical};
use iced::widget::{MouseArea, Space, button, checkbox, column, container, mouse_area, opaque, row, scrollable, stack, text_input};
use iced::{Alignment, Element, Event, Length, Padding, Point, Rectangle, Size, Vector, mouse};
use tc_core::{GroupKind, Message, MsgId, PeerRef};
use tc_theme::tokens::{layout as lay, radius, space, text as ts};

use crate::app::chat_actions::{ChatMsg, Confirm, ForwardPicker, InfoView, Menu, MenuTarget, MuteFor, message_link};
use crate::app::{App, Msg};
use crate::icon::{Icon, icon};
use crate::style::{self, Brasa, c};
use crate::widgets::{avatar_photo, hfill, t, t1};

const MENU_W: f32 = 248.0;
const ITEM_H: f32 = 32.0;
const SEP_H: f32 = 9.0;
const REACT_H: f32 = 40.0;

fn chat(m: ChatMsg) -> Msg {
    Msg::Chat(m)
}

// ------------------------------------------------------------------ sonda del cursor

/// Envuelve la vista y anota dónde se pulsa cualquier botón del ratón (coordenadas de la ventana),
/// sin emitir mensajes: así el menú se abre donde se hizo clic aunque la fila esté dentro de un
/// `scrollable` (que traslada el cursor de sus hijos).
pub struct CursorProbe<'a> {
    content: Element<'a, Msg>,
    cell: &'a Cell<Point>,
}

pub fn probe<'a>(app: &'a App, content: Element<'a, Msg>) -> Element<'a, Msg> {
    Element::new(CursorProbe { content, cell: &app.chat_actions.cursor })
}

impl Widget<Msg, iced::Theme, iced::Renderer> for CursorProbe<'_> {
    fn children(&self) -> Vec<Tree> {
        vec![Tree::new(&self.content)]
    }

    fn diff(&self, tree: &mut Tree) {
        tree.diff_children(std::slice::from_ref(&self.content));
    }

    fn size(&self) -> Size<Length> {
        self.content.as_widget().size()
    }

    fn layout(&mut self, tree: &mut Tree, renderer: &iced::Renderer, limits: &layout::Limits) -> layout::Node {
        self.content.as_widget_mut().layout(&mut tree.children[0], renderer, limits)
    }

    fn operate(&mut self, tree: &mut Tree, layout: Layout<'_>, renderer: &iced::Renderer, operation: &mut dyn Operation) {
        self.content.as_widget_mut().operate(&mut tree.children[0], layout, renderer, operation);
    }

    fn update(
        &mut self,
        tree: &mut Tree,
        event: &Event,
        layout: Layout<'_>,
        cursor: amouse::Cursor,
        renderer: &iced::Renderer,
        clipboard: &mut dyn Clipboard,
        shell: &mut Shell<'_, Msg>,
        viewport: &Rectangle,
    ) {
        if let Event::Mouse(mouse::Event::ButtonPressed(_)) = event
            && let Some(p) = cursor.position()
        {
            self.cell.set(p);
        }
        self.content.as_widget_mut().update(&mut tree.children[0], event, layout, cursor, renderer, clipboard, shell, viewport);
    }

    fn mouse_interaction(&self, tree: &Tree, layout: Layout<'_>, cursor: amouse::Cursor, viewport: &Rectangle, renderer: &iced::Renderer) -> amouse::Interaction {
        self.content.as_widget().mouse_interaction(&tree.children[0], layout, cursor, viewport, renderer)
    }

    fn draw(
        &self,
        tree: &Tree,
        renderer: &mut iced::Renderer,
        theme: &iced::Theme,
        style: &renderer::Style,
        layout: Layout<'_>,
        cursor: amouse::Cursor,
        viewport: &Rectangle,
    ) {
        self.content.as_widget().draw(&tree.children[0], renderer, theme, style, layout, cursor, viewport);
    }

    fn overlay<'b>(
        &'b mut self,
        tree: &'b mut Tree,
        layout: Layout<'b>,
        renderer: &iced::Renderer,
        viewport: &Rectangle,
        translation: Vector,
    ) -> Option<overlay::Element<'b, Msg, iced::Theme, iced::Renderer>> {
        self.content.as_widget_mut().overlay(&mut tree.children[0], layout, renderer, viewport, translation)
    }
}

// ------------------------------------------------------------------ ítems

/// Ítem de un menú contextual (datos puros: los usan la vista y las pruebas).
#[derive(Debug, Clone)]
pub enum Item {
    Act { icon: Icon, label: String, msg: Box<Msg>, danger: bool, hint: Option<&'static str> },
    Sep,
    /// Fila de reacciones del mensaje.
    Reactions(MsgId),
}

fn act(icon: Icon, label: impl Into<String>, msg: Msg) -> Item {
    Item::Act { icon, label: label.into(), msg: Box::new(msg), danger: false, hint: None }
}

fn danger(icon: Icon, label: impl Into<String>, msg: Msg, hint: Option<&'static str>) -> Item {
    Item::Act { icon, label: label.into(), msg: Box::new(msg), danger: true, hint }
}

impl Item {
    pub fn label(&self) -> Option<&str> {
        match self {
            Item::Act { label, .. } => Some(label),
            _ => None,
        }
    }
}

/// Ítems del menú de un chat de la lista, según su tipo.
pub fn chat_items(app: &App, peer: PeerRef, mute_sub: bool) -> Vec<Item> {
    let Some(d) = app.dialog(peer) else { return Vec::new() };
    if mute_sub {
        let mut v: Vec<Item> = MuteFor::ALL.iter().map(|f| act(Icon::Muted, f.label(), chat(ChatMsg::Mute(peer, Some(*f))))).collect();
        v.push(Item::Sep);
        v.push(act(Icon::CaretLeft, "Atrás", chat(ChatMsg::MuteSubmenu)));
        return v;
    }
    let secret = matches!(peer, PeerRef::Secret(_));
    let saved = app.is_saved(peer);
    let kind = app.group_kind(peer);
    let mut v = Vec::new();
    if d.looks_unread() {
        v.push(act(Icon::Checks, "Marcar como leído", chat(ChatMsg::MarkRead(peer))));
    } else if !secret {
        v.push(act(Icon::Unread, "Marcar como no leído", chat(ChatMsg::MarkUnread(peer))));
    }
    if d.pinned_order.is_some() {
        v.push(act(Icon::Pin, "Desfijar", chat(ChatMsg::Pin(peer, false))));
    } else {
        v.push(act(Icon::Pin, "Fijar arriba", chat(ChatMsg::Pin(peer, true))));
    }
    if d.muted {
        v.push(act(Icon::Bell, "Activar sonido", chat(ChatMsg::Mute(peer, None))));
    } else {
        v.push(act(Icon::Muted, "Silenciar…", chat(ChatMsg::MuteSubmenu)));
    }
    if !secret && !saved {
        if d.archived {
            v.push(act(Icon::Archive, "Desarchivar", chat(ChatMsg::Archive(peer, false))));
        } else {
            v.push(act(Icon::Archive, "Archivar", chat(ChatMsg::Archive(peer, true))));
        }
    }
    let info_icon = match kind {
        Some(GroupKind::Channel) => Icon::Megaphone,
        Some(_) => Icon::Users,
        None if matches!(peer, PeerRef::Group(_)) => Icon::Users,
        None => Icon::User,
    };
    v.push(act(info_icon, "Ver información", chat(ChatMsg::ShowInfo(peer))));
    v.push(Item::Sep);
    match peer {
        PeerRef::User(u) => {
            v.push(act(Icon::Trash, "Vaciar historial", chat(ChatMsg::AskClear(peer))));
            if !saved {
                if app.chat_actions.blocked.get(&u) == Some(&true) {
                    v.push(act(Icon::Close, "Desbloquear", chat(ChatMsg::Unblock(peer))));
                } else {
                    let label = if app.users.get(&u).is_some_and(|u| u.is_bot) { "Bloquear bot" } else { "Bloquear usuario" };
                    v.push(danger(Icon::Close, label, chat(ChatMsg::AskBlock(peer)), None));
                }
            }
            v.push(danger(Icon::Trash, "Borrar chat", chat(ChatMsg::AskDeleteChat(peer)), Some("Supr")));
        }
        PeerRef::Secret(_) => v.push(danger(Icon::Trash, "Borrar chat secreto", chat(ChatMsg::AskDeleteChat(peer)), Some("Supr"))),
        PeerRef::Group(_) => {
            if kind != Some(GroupKind::Channel) {
                v.push(act(Icon::Trash, "Vaciar historial", chat(ChatMsg::AskClear(peer))));
            }
            let leave = if kind == Some(GroupKind::Channel) { "Salir del canal" } else { "Salir del grupo" };
            v.push(danger(Icon::Back, leave, chat(ChatMsg::AskLeave(peer)), Some("Supr")));
            if kind == Some(GroupKind::Basic) {
                v.push(danger(Icon::Trash, "Borrar y salir", chat(ChatMsg::AskDeleteChat(peer)), None));
            }
        }
    }
    v
}

/// Ítems del menú de un mensaje de la conversación abierta.
pub fn message_items(app: &App, id: MsgId) -> Vec<Item> {
    let Some(m) = app.conv.as_ref().and_then(|c| c.find(id)) else { return Vec::new() };
    let peer = m.peer;
    let secret = matches!(peer, PeerRef::Secret(_));
    let blocked = app.write_block(peer).is_some();
    let mut v = Vec::new();
    if app.can_react(peer) {
        v.push(Item::Reactions(id));
    }
    if !blocked {
        v.push(act(Icon::Reply, "Responder", Msg::ReplyTo(id)));
    }
    if !m.text.is_empty() {
        v.push(act(Icon::Copy, "Copiar texto", Msg::Copy(id)));
    }
    if let PeerRef::Group(g) = peer
        && app.groups.get(&g).and_then(|g| message_link(g, id)).is_some()
    {
        v.push(act(Icon::Copy, "Copiar enlace", chat(ChatMsg::CopyLink(id))));
    }
    if !secret {
        v.push(act(Icon::Forward, "Reenviar", chat(ChatMsg::Forward(vec![id]))));
    }
    if app.can_edit(m) {
        v.push(act(Icon::Edit, "Editar", Msg::StartEdit(id)));
    }
    if !secret {
        let pinned = app.chat_actions.msg_pinned(peer, id);
        v.push(act(Icon::Pin, if pinned { "Desfijar mensaje" } else { "Fijar mensaje" }, chat(ChatMsg::PinMessage(id, !pinned))));
    }
    v.push(act(Icon::Check, "Seleccionar", chat(ChatMsg::StartSelect(id))));
    v.push(Item::Sep);
    v.push(danger(Icon::Trash, "Borrar", Msg::AskDelete(id), None));
    v
}

fn items_height(items: &[Item]) -> f32 {
    items
        .iter()
        .map(|i| match i {
            Item::Act { .. } => ITEM_H,
            Item::Sep => SEP_H,
            Item::Reactions(_) => REACT_H,
        })
        .sum::<f32>()
        + 2.0 * space::S1
}

fn render_item<'a>(app: &'a App, item: &Item) -> Element<'a, Msg> {
    let b = app.brasa;
    match item {
        Item::Sep => container(container(Space::new().height(Length::Fixed(1.0)).width(Length::Fill)).style(style::fill(c(b.p.border))))
            .height(Length::Fixed(SEP_H))
            .center_y(Length::Fixed(SEP_H))
            .into(),
        Item::Reactions(id) => {
            let mut r = row![].spacing(space::S0_5).align_y(Alignment::Center);
            let mine = |e: &str| app.conv.as_ref().and_then(|cv| cv.find(*id)).is_some_and(|m| m.reactions.iter().any(|x| x.mine && x.emoji == e));
            match app.reaction_choices(app.conv.as_ref().map_or(PeerRef::User(tc_core::UserId(0)), |cv| cv.peer)) {
                None => r = r.push(t1("…", ts::UI, c(b.p.text_muted))),
                Some(list) => {
                    for e in list.iter().take(6) {
                        let next = if mine(e) { None } else { Some(e.clone()) };
                        r = r.push(
                            button(container(t1(e.as_str(), tc_theme::tokens::TextStyle { size: 18.0, line: 22.0, ..ts::UI }, c(b.p.text))).center(Length::Fixed(30.0)))
                                .padding(1)
                                .style(style::ghost(b, radius::SM))
                                .on_press(Msg::React(*id, next)),
                        );
                    }
                }
            }
            container(r).height(Length::Fixed(REACT_H)).center_y(Length::Fixed(REACT_H)).padding(Padding::from([0.0, space::S1])).into()
        }
        Item::Act { icon: ic, label, msg, danger, hint } => {
            let color = c(if *danger { b.p.danger } else { b.p.text });
            let mut r = row![icon(*ic, lay::ICON_MD, if *danger { color } else { c(b.p.text_muted) }), t1(label.clone(), ts::UI, color)]
                .spacing(space::S3)
                .align_y(Alignment::Center);
            if let Some(h) = hint {
                r = r.push(hfill()).push(t1(*h, ts::CAPTION, c(b.p.text_muted)));
            }
            button(container(r).center_y(Length::Fill))
                .width(Length::Fill)
                .height(Length::Fixed(ITEM_H))
                .padding(Padding::from([0.0, space::S3]))
                .style(style::option_row(b, false))
                .on_press((**msg).clone())
                .into()
        }
    }
}

fn menu_layer<'a>(app: &'a App, menu: &Menu, size: Size) -> Element<'a, Msg> {
    let b = app.brasa;
    let items = match menu.target {
        MenuTarget::Chat(peer) => chat_items(app, peer, menu.mute_sub),
        MenuTarget::Message(id) => message_items(app, id),
    };
    let h = items_height(&items);
    let col = column(items.iter().map(|i| render_item(app, i)));
    let k = app.anim.value(crate::app::AnimKey::Overlay, 1.0);
    let w = if app.anim.pref.is_reduced() { MENU_W } else { MENU_W * (0.96 + 0.04 * k) };
    let panel = container(col).padding(space::S1).width(Length::Fixed(w)).style(style::raised(b, 2, radius::MD));
    let x = menu.at.x.min(size.width - MENU_W - space::S2).max(space::S2);
    let y = menu.at.y.min(size.height - h - space::S2).max(space::S2);
    let catcher = mouse_area(container(Space::new()).width(Length::Fill).height(Length::Fill)).on_press(chat(ChatMsg::CloseMenu)).on_right_press(chat(ChatMsg::CloseMenu));
    opaque(stack![catcher, container(opaque(panel)).padding(Padding { left: x, top: y, ..Padding::ZERO })])
}

// ------------------------------------------------------------------ diálogos

fn dialog_frame<'a>(app: &'a App, body: Element<'a, Msg>, on_close: Msg, width: f32) -> Element<'a, Msg> {
    let b = app.brasa;
    let k = app.anim.value(crate::app::AnimKey::Overlay, 1.0);
    let panel = container(body).padding(space::S6).width(Length::Fixed(width)).style(style::raised(b, 3, radius::XL));
    let scrim = mouse_area(container(Space::new()).width(Length::Fill).height(Length::Fill).style(style::scrim(b, k))).on_press(on_close);
    opaque(stack![scrim, container(opaque(panel)).center(Length::Fill)])
}

fn buttons<'a>(b: Brasa, cancel: Msg, ok_label: &'a str, ok: Msg, destructive: bool) -> Element<'a, Msg> {
    let ok_btn = if destructive {
        button(t1(ok_label, ts::UI_STRONG, c(b.p.on_danger))).padding([8, 16]).style(style::danger_solid(b)).on_press(ok)
    } else {
        button(t1(ok_label, ts::UI_STRONG, c(b.p.on_accent))).padding([8, 16]).style(style::primary(b, radius::MD)).on_press(ok)
    };
    row![hfill(), button(t1("Cancelar", ts::UI_STRONG, c(b.p.text))).padding([8, 16]).style(style::ghost(b, radius::MD)).on_press(cancel), ok_btn]
        .spacing(space::S2)
        .into()
}

fn confirm_panel<'a>(app: &'a App, cf: &'a Confirm) -> Element<'a, Msg> {
    let b = app.brasa;
    let title_of = |p: PeerRef| app.dialog(p).map(|d| d.title.clone()).unwrap_or_default();
    let revoke_box = |revoke: bool, who: String| {
        checkbox(revoke)
            .label(format!("Borrar también para {who}"))
            .on_toggle(|v| chat(ChatMsg::SetRevoke(v)))
            .font(style::font_of(ts::UI))
            .text_size(ts::UI.size)
            .style(style::check(b))
    };
    let (title, body, ok, check): (String, String, &str, Option<Element<'a, Msg>>) = match cf {
        Confirm::DeleteChat { peer, revoke, can_revoke } => {
            let who = title_of(*peer);
            let (title, body) = match peer {
                PeerRef::Group(_) => (format!("¿Borrar «{who}» y salir?"), "Saldrás del grupo y su historial desaparecerá de tus dispositivos.".to_string()),
                PeerRef::Secret(_) => (format!("¿Borrar el chat secreto con {who}?"), "Se cerrará el chat y su historial desaparecerá de este dispositivo.".to_string()),
                PeerRef::User(_) => (format!("¿Borrar el chat con {who}?"), "Todo el historial desaparecerá de tus dispositivos. No se puede deshacer.".to_string()),
            };
            (title, body, "Borrar", can_revoke.then(|| revoke_box(*revoke, who).into()))
        }
        Confirm::ClearHistory { peer } => (
            format!("¿Vaciar el historial de {}?", title_of(*peer)),
            "Se borrarán todos los mensajes para ti. El chat seguirá en la lista.".into(),
            "Vaciar",
            None,
        ),
        Confirm::Leave { peer } => {
            let channel = app.is_channel(*peer);
            (
                format!("¿Salir de «{}»?", title_of(*peer)),
                if channel { "Dejarás de recibir sus publicaciones y desaparecerá de la lista." } else { "Dejarás de recibir sus mensajes y desaparecerá de la lista." }.into(),
                "Salir",
                None,
            )
        }
        Confirm::Block { peer, user } => {
            let who = app.users.get(user).map_or_else(|| title_of(*peer), tc_core::User::display_name);
            (format!("¿Bloquear a {who}?"), "No podrá escribirte ni llamarte. Puedes desbloquearlo cuando quieras.".into(), "Bloquear", None)
        }
        Confirm::DeleteMessages { peer, ids, revoke, can_revoke } => {
            let n = ids.len();
            let title = if n == 1 { "¿Borrar este mensaje?".to_string() } else { format!("¿Borrar {n} mensajes?") };
            (title, "Desaparecerán de esta conversación. No se puede deshacer.".into(), "Borrar", can_revoke.then(|| revoke_box(*revoke, title_of(*peer)).into()))
        }
    };
    let mut col = column![t1(title, ts::TITLE, c(b.p.text)), t(body, ts::UI, c(b.p.text_muted))].spacing(space::S3);
    if let Some(ch) = check {
        col = col.push(ch);
    }
    col.push(buttons(b, chat(ChatMsg::CancelConfirm), ok, chat(ChatMsg::Confirm), true)).into()
}

fn info_panel<'a>(app: &'a App, v: &'a InfoView) -> Element<'a, Msg> {
    let b = app.brasa;
    let peer = v.peer;
    let d = app.dialog(peer);
    let title = d.map_or_else(String::new, |d| d.title.clone());
    let kind = match peer {
        PeerRef::Secret(_) => "Chat secreto",
        PeerRef::User(_) if app.is_saved(peer) => "Mensajes guardados",
        PeerRef::User(u) if app.users.get(&u).is_some_and(|u| u.is_bot) => "Bot",
        PeerRef::User(_) => "Usuario",
        PeerRef::Group(_) => match app.group_kind(peer) {
            Some(GroupKind::Channel) => "Canal",
            Some(GroupKind::Super) => "Supergrupo",
            _ => "Grupo",
        },
    };
    let head = row![
        avatar_photo(&title, App::peer_seed(peer), lay::AVATAR_LG, None, app.avatar_handle(peer, d.and_then(|d| d.photo)), b),
        column![t1(crate::format::ellipsize(&title, 40), ts::TITLE, c(b.p.text)), t1(kind, ts::META, c(b.p.text_muted))]
    ]
    .spacing(space::S3)
    .align_y(Alignment::Center);
    let mut col = column![head].spacing(space::S3);
    let field = |label: &'static str, value: String| column![t1(label, ts::CAPTION, c(b.p.text_muted)), t(value, ts::UI, c(b.p.text))].spacing(0);
    let local_members = match peer {
        PeerRef::Group(g) => app.groups.get(&g).map(|g| g.member_count),
        _ => None,
    };
    match &v.info {
        None => col = col.push(t1("Cargando…", ts::UI, c(b.p.text_muted))),
        Some(r) => {
            let info = r.as_ref().ok();
            let username = info.and_then(|i| i.username.clone()).or_else(|| match peer {
                PeerRef::User(u) => app.users.get(&u).and_then(|u| u.username.clone()),
                PeerRef::Group(g) => app.groups.get(&g).and_then(|g| g.username.clone()),
                PeerRef::Secret(_) => None,
            });
            if let Some(u) = username {
                col = col.push(field("Nombre de usuario", format!("@{u}")));
            }
            if let Some(a) = info.and_then(|i| i.about.clone()) {
                col = col.push(field(if matches!(peer, PeerRef::Group(_)) { "Descripción" } else { "Biografía" }, a));
            }
            if let Some(n) = info.and_then(|i| i.members).or(local_members) {
                col = col.push(field(if app.is_channel(peer) { "Suscriptores" } else { "Miembros" }, n.to_string()));
            }
            if info.is_some_and(|i| i.blocked) {
                col = col.push(t1("Has bloqueado a este usuario.", ts::UI, c(b.p.danger)));
            }
            if let Err(e) = r
                && !e.is_empty()
            {
                col = col.push(t(format!("No se pudieron cargar más datos: {e}"), ts::CAPTION, c(b.p.warning)));
            }
        }
    }
    col.push(row![hfill(), button(t1("Cerrar", ts::UI_STRONG, c(b.p.on_accent))).padding([8, 16]).style(style::primary(b, radius::MD)).on_press(chat(ChatMsg::CloseInfo))])
        .into()
}

fn forward_panel<'a>(app: &'a App, f: &'a ForwardPicker) -> Element<'a, Msg> {
    let b = app.brasa;
    let n = f.ids.len();
    let title = if n == 1 { "Reenviar mensaje".to_string() } else { format!("Reenviar {n} mensajes") };
    let mut rows = column![].spacing(space::S0_5);
    let list = app.forward_targets(&f.query);
    for d in list.iter().take(60) {
        let r = row![
            avatar_photo(&d.title, App::peer_seed(d.peer), lay::AVATAR_MD, None, app.avatar_handle(d.peer, d.photo), b),
            t1(crate::format::ellipsize(&d.title, 40), ts::UI_STRONG, c(b.p.text))
        ]
        .spacing(space::S3)
        .align_y(Alignment::Center);
        rows = rows.push(button(r).width(Length::Fill).padding([6, 8]).style(style::option_row(b, false)).on_press_maybe((!f.busy).then(|| chat(ChatMsg::ForwardTo(d.peer)))));
    }
    if list.is_empty() {
        rows = rows.push(t1("Ningún chat coincide.", ts::UI, c(b.p.text_muted)));
    }
    let filter = text_input("Buscar chat", &f.query)
        .id("forward-filter")
        .on_input(|q| chat(ChatMsg::ForwardQuery(q)))
        .padding(Padding::from([10.0, 12.0]))
        .size(ts::UI.size)
        .font(style::font_of(ts::UI))
        .style(style::field_input(b, false));
    let status = if f.busy { "Reenviando…" } else { "Elige a quién se lo mandas." };
    column![
        row![icon(Icon::Forward, lay::ICON_LG, c(b.p.accent_text)), t1(title, ts::TITLE, c(b.p.text))].spacing(space::S2).align_y(Alignment::Center),
        t1(status, ts::UI, c(b.p.text_muted)),
        filter,
        scrollable(rows).height(Length::Fixed(280.0)).style(style::thin_scroll(b)),
        row![hfill(), button(t1("Cancelar", ts::UI_STRONG, c(b.p.text))).padding([8, 16]).style(style::ghost(b, radius::MD)).on_press(chat(ChatMsg::CloseForward))],
    ]
    .spacing(space::S3)
    .into()
}

fn undo_toast(app: &App) -> Option<Element<'_, Msg>> {
    let b = app.brasa;
    let u = app.chat_actions.undo.as_ref()?;
    let r = row![
        icon(Icon::Check, lay::ICON_MD, c(b.p.success)),
        t(u.text.as_str(), ts::UI, c(b.p.text)),
        button(t1("Deshacer", ts::UI_STRONG, c(b.p.accent_text))).padding([4, 8]).style(style::ghost(b, radius::SM)).on_press(chat(ChatMsg::Undo)),
    ]
    .spacing(space::S2)
    .align_y(Alignment::Center);
    let toast = container(r).padding(Padding::from([space::S1, space::S2])).max_width(440.0).style(style::raised(b, 2, radius::MD));
    let bottom = 84.0 + app.toasts.len() as f32 * 44.0;
    Some(
        container(toast)
            .width(Length::Fill)
            .height(Length::Fill)
            .align_x(Horizontal::Center)
            .align_y(Vertical::Bottom)
            .padding(Padding { bottom, ..Padding::ZERO })
            .into(),
    )
}

/// Capas que se apilan sobre la vista principal (menú, diálogos y aviso con «Deshacer»).
pub fn layers(app: &App, size: Size) -> Vec<Element<'_, Msg>> {
    let a = &app.chat_actions;
    let mut out = Vec::new();
    if let Some(u) = undo_toast(app) {
        out.push(u);
    }
    if let Some(cf) = &a.confirm {
        out.push(dialog_frame(app, confirm_panel(app, cf), chat(ChatMsg::CancelConfirm), 460.0));
    } else if let Some(v) = &a.info {
        out.push(dialog_frame(app, info_panel(app, v), chat(ChatMsg::CloseInfo), 420.0));
    } else if let Some(f) = &a.forward {
        out.push(dialog_frame(app, forward_panel(app, f), chat(ChatMsg::CloseForward), 440.0));
    }
    if let Some(m) = &a.menu {
        out.push(menu_layer(app, m, size));
    }
    out
}

// ------------------------------------------------------------------ piezas de la lista

/// Botón «⋯» de la fila bajo el ratón.
pub fn more_button<'a>(b: Brasa, peer: PeerRef) -> Element<'a, Msg> {
    button(icon(Icon::More, 16.0, c(b.p.text_muted))).padding(Padding::from([0.0, 4.0])).style(style::ghost(b, radius::SM)).on_press(chat(ChatMsg::OpenChatMenu(peer))).into()
}

/// Punto de «marcado como no leído» (sin contador).
pub fn unread_dot<'a>(b: Brasa, muted: bool) -> Element<'a, Msg> {
    container(Space::new().width(Length::Fixed(10.0)).height(Length::Fixed(10.0))).style(style::badge(b, muted)).into()
}

/// Fila «Archivados» al principio de la lista (o cabecera de vuelta si se está dentro).
pub fn archive_row(app: &App) -> Option<Element<'_, Msg>> {
    let b = app.brasa;
    let (n, unread) = app.archived_counts();
    let inside = app.chat_actions.show_archived;
    if !inside && (n == 0 || !app.search.trim().is_empty()) {
        return None;
    }
    let mut r = row![
        icon(if inside { Icon::Back } else { Icon::Archive }, lay::ICON_MD, c(b.p.text_muted)),
        t1(if inside { "Volver a los chats" } else { "Archivados" }, ts::UI_STRONG, c(b.p.text)),
        hfill(),
    ]
    .spacing(space::S3)
    .align_y(Alignment::Center);
    if inside {
        r = r.push(t1(format!("{n} archivados"), ts::CAPTION, c(b.p.text_muted)));
    } else if unread > 0 {
        r = r.push(crate::widgets::badge(unread as u32, true, b));
    }
    Some(
        button(container(r).center_y(Length::Fill))
            .width(Length::Fill)
            .height(Length::Fixed(44.0))
            .padding(Padding::from([0.0, space::S4]))
            .style(style::option_row(b, false))
            .on_press(chat(ChatMsg::ToggleArchived))
            .into(),
    )
}

// ------------------------------------------------------------------ piezas de la conversación

/// Fila de un mensaje: clic derecho abre su menú; en modo selección, el clic marca/desmarca y la
/// fila se resalta con una casilla en el lado libre.
pub fn message_line<'a>(app: &'a App, m: &'a Message, area: MouseArea<'a, Msg>) -> Element<'a, Msg> {
    let b = app.brasa;
    let local = crate::conv::is_local(m);
    let selecting = app.chat_actions.selection.is_some();
    if local {
        return area.on_right_press(Msg::Hover(Some(m.id))).into();
    }
    if !selecting {
        return area.on_right_press(chat(ChatMsg::OpenMessageMenu(m.id))).into();
    }
    let sel = app.chat_actions.is_selected(m.id);
    let area = area.on_press(chat(ChatMsg::ToggleSelect(m.id))).on_right_press(chat(ChatMsg::ToggleSelect(m.id)));
    let mark = container(if sel { icon(Icon::Check, 14.0, c(b.p.on_accent)) } else { Space::new().into() })
        .center(Length::Fixed(20.0))
        .style(if sel { style::rounded_fill(c(b.p.accent), style::r(radius::PILL)) } else { style::rounded_fill(style::TRANSPARENT, style::r(radius::PILL)) });
    let mark = container(container(mark).style(move |_| container::Style {
        border: iced::Border { radius: style::r(radius::PILL), width: 1.5, color: c(if sel { b.p.accent } else { b.p.border_strong }) },
        ..container::Style::default()
    }))
    .height(Length::Fill)
    .width(Length::Fill)
    .align_y(Vertical::Center)
    .align_x(if m.outgoing { Horizontal::Left } else { Horizontal::Right })
    .padding(Padding::from([0.0, space::S2]));
    let bg = container(Space::new())
        .width(Length::Fill)
        .height(Length::Fill)
        .style(style::rounded_fill(if sel { style::fade(c(b.p.accent), 0.12) } else { style::TRANSPARENT }, style::r(radius::MD)));
    stack![bg, mark, area].into()
}

/// Barra de selección que sustituye al compositor mientras hay mensajes elegidos.
pub fn selection_bar(app: &App) -> Option<Element<'_, Msg>> {
    let b = app.brasa;
    let sel = app.chat_actions.selection.as_ref()?;
    let peer = app.conv.as_ref()?.peer;
    let n = sel.len();
    let label = if n == 1 { "1 mensaje seleccionado".to_string() } else { format!("{n} mensajes seleccionados") };
    let mut r = row![
        crate::widgets::icon_button(Icon::Close, lay::ICON_MD, c(b.p.text_muted), Some(chat(ChatMsg::CancelSelect)), b),
        t1(label, ts::UI_STRONG, c(b.p.text)),
        hfill(),
    ]
    .spacing(space::S2)
    .align_y(Alignment::Center);
    if !matches!(peer, PeerRef::Secret(_)) {
        r = r.push(
            button(row![icon(Icon::Forward, lay::ICON_MD, c(b.p.accent_text)), t1("Reenviar", ts::UI_STRONG, c(b.p.accent_text))].spacing(space::S1).align_y(Alignment::Center))
                .padding([8, 12])
                .style(style::ghost(b, radius::MD))
                .on_press(chat(ChatMsg::ForwardSelected)),
        );
    }
    r = r.push(
        button(row![icon(Icon::Trash, lay::ICON_MD, c(b.p.on_danger)), t1("Borrar", ts::UI_STRONG, c(b.p.on_danger))].spacing(space::S1).align_y(Alignment::Center))
            .padding([8, 12])
            .style(style::danger_solid(b))
            .on_press(chat(ChatMsg::AskDeleteSelected)),
    );
    Some(
        super::conversation::in_column(r)
            .height(Length::Fixed(lay::COMPOSER_MIN_HEIGHT + space::S4))
            .align_y(Vertical::Center)
            .style(style::surface(b))
            .into(),
    )
}
