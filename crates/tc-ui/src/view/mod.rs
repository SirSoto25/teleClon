//! Vista raíz: disposición responsiva (raíl + lista + conversación), diálogos y toasts.

pub mod audio;
pub mod context_menu;
pub mod conversation;
pub mod emoji_panel;
pub mod gif_panel;
pub mod login;
pub mod panel;
pub mod sidebar;
pub mod stickers;
/// Grupos con temas: lista de temas, cabecera de un tema y su menú.
pub mod topics;

use iced::alignment::{Horizontal, Vertical};
use iced::widget::{Space, button, checkbox, column, container, image, mouse_area, opaque, responsive, row, scrollable, stack, text_input, tooltip};
use iced::{Alignment, ContentFit, Element, Length, Padding};
use tc_core::PeerRef;
use tc_theme::tokens::{layout, radius, space, text as ts};

use crate::app::{AnimKey, App, Msg, Overlay, Phase, TTL_OPTIONS};
use crate::format;
use crate::icon::{Icon, icon, icon_fill};
use crate::media::ImageKey;
use crate::style::{self, c};
use crate::widgets::{hfill, t, t1, vgap};

impl App {
    pub fn view(&self) -> Element<'_, Msg> {
        if self.phase != Phase::Main {
            let screen = if self.phase == Phase::Auth { login::view(self) } else { login::connecting(self) };
            let mut layers = vec![screen];
            if !self.toasts.is_empty() {
                layers.push(toasts(self));
            }
            return stack(layers).width(Length::Fill).height(Length::Fill).into();
        }
        responsive(move |size| {
            let narrow = size.width < layout::BREAKPOINT_SPLIT;
            let main: Element<'_, Msg> = if narrow {
                // La lista de temas de un foro ocupa la pantalla como una conversación (`topics.rs`).
                if self.conv.is_some() || self.topics_shown().is_some() {
                    conversation::view(self, true)
                } else {
                    row![sidebar::rail(self), sidebar::list(self, (size.width - layout::RAIL_WIDTH - 2.0).max(200.0))].into()
                }
            } else {
                row![
                    sidebar::rail(self),
                    sidebar::list(self, layout::LIST_WIDTH_DEFAULT),
                    conversation::view(self, false)
                ]
                .into()
            };
            let mut layers = vec![main];
            if !self.toasts.is_empty() {
                layers.push(toasts(self));
            }
            if let Some(o) = &self.overlay {
                layers.push(overlay(self, o));
            }
            // Menús contextuales, confirmaciones y «Deshacer» (ver `context_menu.rs`).
            layers.extend(context_menu::layers(self, size));
            layers.extend(topics::layers(self, size));
            context_menu::probe(self, stack(layers).width(Length::Fill).height(Length::Fill).into())
        })
        .into()
    }
}

fn toasts(app: &App) -> Element<'_, Msg> {
    let b = app.brasa;
    let mut col = column![].spacing(space::S2).align_x(Alignment::Center);
    for tst in &app.toasts {
        let (ic, color) = if tst.error { (Icon::Warning, b.p.danger) } else { (Icon::Check, b.p.success) };
        col = col.push(
            container(row![icon(ic, layout::ICON_MD, c(color)), t(tst.text.as_str(), ts::UI, c(b.p.text))].spacing(space::S2).align_y(Alignment::Center))
                .padding(Padding::from([space::S2, space::S4]))
                .max_width(420.0)
                .style(style::raised(b, 2, radius::MD)),
        );
    }
    container(col)
        .width(Length::Fill)
        .height(Length::Fill)
        .align_x(Horizontal::Center)
        .align_y(Vertical::Bottom)
        .padding(Padding { bottom: 84.0, ..Padding::ZERO })
        .into()
}

fn overlay<'a>(app: &'a App, o: &'a Overlay) -> Element<'a, Msg> {
    let b = app.brasa;
    let k = app.anim.value(AnimKey::Overlay, 1.0);
    if let Overlay::Photo { peer, msg } = o {
        return photo_viewer(app, *peer, *msg, k);
    }
    let scale = if app.anim.pref.is_reduced() { 1.0 } else { 0.97 + 0.03 * k };
    let width = 480.0 * scale;
    let panel: Element<'a, Msg> = match o {
        Overlay::Delete { revoke, outgoing, .. } => {
            let who = app.conv.as_ref().and_then(|cv| app.dialog(cv.peer)).map(|d| d.title.clone()).unwrap_or_default();
            let mut col = column![
                t1("¿Eliminar este mensaje?", ts::TITLE, c(b.p.text)),
                t("Desaparecerá de esta conversación. No se puede deshacer.", ts::UI, c(b.p.text_muted)),
            ]
            .spacing(space::S3);
            if *outgoing {
                col = col.push(
                    checkbox(*revoke)
                        .label(format!("Eliminar también para {who}"))
                        .on_toggle(Msg::SetRevoke)
                        .font(style::font_of(ts::UI))
                        .text_size(ts::UI.size)
                        .style(style::check(b)),
                );
            }
            col = col.push(vgap(space::S2)).push(
                row![
                    hfill(),
                    button(t1("Cancelar", ts::UI_STRONG, c(b.p.text))).padding([8, 16]).style(style::ghost(b, radius::MD)).on_press(Msg::CloseOverlay),
                    button(t1("Eliminar", ts::UI_STRONG, c(b.p.on_danger))).padding([8, 16]).style(style::danger_solid(b)).on_press(Msg::ConfirmDelete),
                ]
                .spacing(space::S2),
            );
            col.into()
        }
        Overlay::Key(kv) => key_panel(app, kv.as_ref()),
        Overlay::NewSecret { query, user, busy } => new_secret_panel(app, query, *user, *busy),
        Overlay::Ttl { confirm_discard } => ttl_panel(app, *confirm_discard),
        Overlay::Photo { .. } => unreachable!("el visor se dibuja aparte"),
    };
    let panel = container(panel).padding(space::S6).width(Length::Fixed(width)).style(style::raised(b, 3, radius::XL));
    let scrim = mouse_area(container(Space::new()).width(Length::Fill).height(Length::Fill).style(style::scrim(b, k))).on_press(Msg::CloseOverlay);
    opaque(stack![scrim, container(opaque(panel)).center(Length::Fill)])
}

/// Visor de foto a pantalla completa: la versión completa en cuanto llega (antes, la de la burbuja
/// ampliada), con el progreso de la descarga si es larga. Se cierra con Escape, la X, «volver» o pulsando fuera de la imagen; flechas y botones laterales
/// cambian de foto; rueda o doble clic alternan ajuste y 100 %.
fn photo_viewer<'a>(app: &'a App, peer: PeerRef, msg: tc_core::MsgId, k: f32) -> Element<'a, Msg> {
    let b = app.brasa;
    let file = app.conv.as_ref().filter(|c| c.peer == peer).and_then(|c| c.find(msg)).and_then(|m| m.media.as_ref()).and_then(|md| md.file.as_ref());
    let full_key = file.map(|f| ImageKey::Full { peer, msg, file: f.id });
    let thumb_key = file.map(|f| ImageKey::Thumb { peer, msg, file: f.id });
    let shown = full_key.and_then(|key| app.images.get(&key)).or_else(|| thumb_key.and_then(|key| app.images.get(&key)));
    let status = match (file, full_key) {
        (None, _) => Some("Esta foto ya no está disponible.".to_string()),
        (Some(_), Some(key)) if app.images.failed(&key) => Some("No se pudo cargar la foto completa.".to_string()),
        (Some(f), Some(key)) if app.images.get(&key).is_none() => Some(match app.downloads.get(&f.id) {
            Some((done, total)) if *total > 0 => format!("Cargando la foto… {} %", done * 100 / total),
            _ => "Cargando la foto…".to_string(),
        }),
        _ => None,
    };
    let zoom = app.photo_zoom;
    let picture: Element<'a, Msg> = match shown {
        Some(d) => {
            let (handle, iw, ih) = (d.handle.clone(), d.width.max(1) as f32, d.height.max(1) as f32);
            responsive(move |size| {
                let (aw, ah) = ((size.width - 2.0 * space::S8).max(1.0), (size.height - 2.0 * space::S8).max(1.0));
                let (w, h) = if zoom {
                    (iw, ih)
                } else {
                    let f = (aw / iw).min(ah / ih);
                    (iw * f, ih * f)
                };
                let pic = image(handle.clone()).content_fit(ContentFit::Fill).width(Length::Fixed(w)).height(Length::Fixed(h)).opacity(k);
                let hit = mouse_area(pic)
                    .on_double_click(Msg::PhotoZoom(None))
                    .on_scroll(|d| {
                        let y = match d {
                            iced::mouse::ScrollDelta::Lines { y, .. } | iced::mouse::ScrollDelta::Pixels { y, .. } => y,
                        };
                        Msg::PhotoZoom(Some(y > 0.0))
                    })
                    .interaction(iced::mouse::Interaction::ZoomIn);
                container(opaque(hit)).width(Length::Fill).height(Length::Fill).center(Length::Fill).clip(true).into()
            })
            .into()
        }
        None => Space::new().into(),
    };
    let ink = iced::Color::WHITE;
    let chip = |i: Icon, tip: &'static str, msg: Option<Msg>| -> Element<'a, Msg> {
        let btn = button(container(icon(i, layout::ICON_MD, ink)).center(Length::Fixed(40.0)))
            .padding(0)
            .style(style::ghost(b, radius::PILL))
            .on_press_maybe(msg);
        tooltip(
            container(btn).style(style::rounded_fill(iced::Color { a: 0.55, ..iced::Color::BLACK }, style::r(radius::PILL))),
            container(t1(tip, ts::CAPTION, ink)).padding([4, 8]).style(style::rounded_fill(iced::Color { a: 0.85, ..iced::Color::BLACK }, style::r(radius::SM))),
            tooltip::Position::Bottom,
        )
        .into()
    };
    let (prev, next) = (app.photo_neighbor(peer, msg, -1), app.photo_neighbor(peer, msg, 1));
    let side = |i: Icon, tip: &'static str, target: Option<tc_core::MsgId>, dir: i32, right: bool| -> Element<'a, Msg> {
        let Some(_) = target else { return Space::new().into() };
        container(chip(i, tip, Some(Msg::PhotoStep(dir))))
            .width(Length::Fill)
            .height(Length::Fill)
            .align_x(if right { Horizontal::Right } else { Horizontal::Left })
            .align_y(Vertical::Center)
            .padding(space::S3)
            .into()
    };
    let mut layers = vec![
        mouse_area(container(Space::new()).width(Length::Fill).height(Length::Fill).style(style::fill(iced::Color { a: 0.88 * k, ..iced::Color::BLACK })))
            .on_press(Msg::CloseOverlay)
            .into(),
        container(picture).width(Length::Fill).height(Length::Fill).into(),
        side(Icon::CaretLeft, "Foto anterior (←)", prev, -1, false),
        side(Icon::CaretRight, "Foto siguiente (→)", next, 1, true),
        container(chip(Icon::Back, "Volver al chat (Esc)", Some(Msg::CloseOverlay))).width(Length::Fill).align_x(Horizontal::Left).padding(space::S3).into(),
        container(chip(Icon::Close, "Cerrar (Esc)", Some(Msg::CloseOverlay))).width(Length::Fill).align_x(Horizontal::Right).padding(space::S3).into(),
    ];
    if let Some(s) = status {
        layers.push(
            container(container(t1(s, ts::UI, ink)).padding([6, 12]).style(style::rounded_fill(iced::Color { a: 0.6, ..iced::Color::BLACK }, style::r(radius::PILL))))
                .width(Length::Fill)
                .height(Length::Fill)
                .align_x(Horizontal::Center)
                .align_y(Vertical::Bottom)
                .padding(space::S6)
                .into(),
        );
    }
    opaque(stack(layers).width(Length::Fill).height(Length::Fill))
}

/// Huella de clave (§8.9): cuadrícula 4×4 con colores `name-n` y 4 grupos de 5 cifras.
fn key_panel<'a>(app: &'a App, kv: Option<&'a tc_core::KeyVisual>) -> Element<'a, Msg> {
    let b = app.brasa;
    let who = match app.conv.as_ref().map(|c| c.peer) {
        Some(PeerRef::Secret(s)) => app.secrets.get(&s).map(|c| app.user_name(c.user)).unwrap_or_default(),
        _ => String::new(),
    };
    let mut col = column![
        t1("Huella de la clave", ts::TITLE, c(b.p.text)),
        t(format!("Compara este dibujo y las cifras con los que ve {who}. Si coinciden, nadie se ha colado en medio."), ts::UI, c(b.p.text_muted)),
    ]
    .spacing(space::S3);
    match kv {
        None => col = col.push(t1("Calculando…", ts::UI, c(b.p.text_muted))),
        Some(kv) => {
            let (cells, groups) = format::key_grid(&kv.fingerprint);
            let names = [b.p.name_1, b.p.name_2, b.p.name_3, b.p.name_4, b.p.name_5, b.p.name_6, b.p.name_7, b.p.name_8];
            let mut grid = column![].spacing(space::S1);
            for r in 0..4 {
                let mut line = row![].spacing(space::S1);
                for k in 0..4 {
                    let color = c(names[cells[r * 4 + k] as usize]);
                    line = line.push(
                        container(Space::new().width(Length::Fixed(20.0)).height(Length::Fixed(20.0))).style(style::rounded_fill(color, style::r(radius::XS))),
                    );
                }
                grid = grid.push(line);
            }
            let digits = column(groups.iter().map(|g| t1(g.clone(), ts::CODE, c(b.p.text)).into())).spacing(space::S1);
            col = col
                .push(row![grid, digits].spacing(space::S6).align_y(Alignment::Center))
                .push(t1(kv.emoji.join("  "), ts::TITLE, c(b.p.text)));
            if app.simulated {
                col = col.push(t("Backend de pruebas: esta huella es ilustrativa, no sale de una clave real.", ts::CAPTION, c(b.p.warning)));
            }
        }
    }
    col = col.push(experimental_note(app));
    col.push(row![hfill(), button(t1("Cerrar", ts::UI_STRONG, c(b.p.on_accent))).padding([8, 16]).style(style::primary(b, radius::MD)).on_press(Msg::CloseOverlay)])
        .into()
}

/// Recordatorio EXPERIMENTAL de los diálogos de chats secretos.
fn experimental_note(app: &App) -> Element<'_, Msg> {
    let b = app.brasa;
    row![
        conversation::experimental_pill(b),
        t("Cifrado propio sin auditoría de seguridad: no lo uses para información sensible.", ts::CAPTION, c(b.p.text_muted)),
    ]
    .spacing(space::S2)
    .align_y(Alignment::Center)
    .into()
}

/// Crear un chat secreto: filtro + lista de contactos + confirmación (Enter confirma).
fn new_secret_panel<'a>(app: &'a App, query: &'a str, user: Option<tc_core::UserId>, busy: bool) -> Element<'a, Msg> {
    let b = app.brasa;
    let list = app.secret_candidates(query);
    let chosen = user.or(match list.as_slice() {
        [u] => Some(u.id),
        _ => None,
    });
    let mut rows = column![].spacing(space::S0_5);
    for u in &list {
        let active = chosen == Some(u.id);
        let name = u.display_name();
        let mut r = row![crate::widgets::avatar_photo(&name, u.id.0, layout::AVATAR_MD, None, app.avatar_handle(PeerRef::User(u.id), u.photo), b), t1(format::ellipsize(&name, 36), ts::UI_STRONG, c(b.p.text))]
            .spacing(space::S3)
            .align_y(Alignment::Center);
        if active {
            r = r.push(hfill()).push(icon(Icon::Check, layout::ICON_MD, c(b.p.accent_text)));
        }
        rows = rows.push(button(r).width(Length::Fill).padding([6, 8]).style(style::option_row(b, active)).on_press(Msg::SecretPick(u.id)));
    }
    if list.is_empty() {
        rows = rows.push(t1("Ningún contacto coincide.", ts::UI, c(b.p.text_muted)));
    }
    let filter = text_input("Buscar contacto", query)
        .id("new-secret-filter")
        .on_input(Msg::SecretQuery)
        .on_submit(Msg::ConfirmNewSecret)
        .padding(Padding::from([10.0, 12.0]))
        .size(ts::UI.size)
        .font(style::font_of(ts::UI))
        .style(style::field_input(b, false));
    let confirm = (!busy && chosen.is_some()).then_some(Msg::ConfirmNewSecret);
    column![
        row![icon_fill(Icon::Shield, layout::ICON_LG, c(b.p.success)), t1("Nuevo chat secreto", ts::TITLE, c(b.p.text))].spacing(space::S2).align_y(Alignment::Center),
        t(
            "Los mensajes se cifran de extremo a extremo entre este dispositivo y el de la otra persona: no se guardan en la nube ni se pueden reenviar.",
            ts::UI,
            c(b.p.text_muted)
        ),
        experimental_note(app),
        filter,
        scrollable(rows).height(Length::Fixed(240.0)).style(style::thin_scroll(b)),
        row![
            hfill(),
            button(t1("Cancelar", ts::UI_STRONG, c(b.p.text))).padding([8, 16]).style(style::ghost(b, radius::MD)).on_press(Msg::CloseOverlay),
            button(t1(if busy { "Creando…" } else { "Crear chat secreto" }, ts::UI_STRONG, c(b.p.on_accent)))
                .padding([8, 16])
                .style(style::primary(b, radius::MD))
                .on_press_maybe(confirm),
        ]
        .spacing(space::S2),
    ]
    .spacing(space::S3)
    .into()
}

/// Autodestrucción del chat secreto abierto y opción de cerrarlo (con confirmación).
fn ttl_panel(app: &App, confirm_discard: bool) -> Element<'_, Msg> {
    let b = app.brasa;
    let current = match app.conv.as_ref().map(|c| c.peer) {
        Some(PeerRef::Secret(s)) => app.secrets.get(&s).and_then(|c| c.ttl_secs),
        _ => None,
    };
    let mut opts = column![].spacing(space::S0_5);
    for o in TTL_OPTIONS {
        let active = o == current;
        let label = o.map_or_else(|| "Desactivada".to_string(), |s| format!("Tras {}", format::ttl(s)));
        let mut r = row![t1(label, ts::UI, c(b.p.text))].align_y(Alignment::Center);
        if active {
            r = r.push(hfill()).push(icon(Icon::Check, layout::ICON_MD, c(b.p.accent_text)));
        }
        opts = opts.push(button(r).width(Length::Fill).padding([8, 12]).style(style::option_row(b, active)).on_press(Msg::SetTtl(o)));
    }
    let discard: Element<'_, Msg> = if confirm_discard {
        row![
            t("Se borrará el historial en este dispositivo. ¿Cerrar el chat secreto?", ts::UI, c(b.p.danger)),
            hfill(),
            button(t1("Cerrar chat", ts::UI_STRONG, c(b.p.on_danger))).padding([8, 16]).style(style::danger_solid(b)).on_press(Msg::DiscardSecret),
        ]
        .spacing(space::S2)
        .align_y(Alignment::Center)
        .into()
    } else {
        button(row![icon(Icon::Trash, layout::ICON_MD, c(b.p.danger)), t1("Cerrar chat secreto", ts::UI_STRONG, c(b.p.danger))].spacing(space::S2))
            .padding([8, 12])
            .style(style::ghost(b, radius::MD))
            .on_press(Msg::AskDiscardSecret)
            .into()
    };
    column![
        row![icon(Icon::Timer, layout::ICON_LG, c(b.p.accent_text)), t1("Autodestrucción", ts::TITLE, c(b.p.text))].spacing(space::S2).align_y(Alignment::Center),
        t("Los mensajes nuevos se borrarán en los dos dispositivos cuando pase este tiempo desde que se leen.", ts::UI, c(b.p.text_muted)),
        opts,
        experimental_note(app),
        discard,
        row![hfill(), button(t1("Listo", ts::UI_STRONG, c(b.p.text))).padding([8, 16]).style(style::ghost(b, radius::MD)).on_press(Msg::CloseOverlay)],
    ]
    .spacing(space::S3)
    .into()
}
