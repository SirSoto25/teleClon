//! Vista raíz: disposición responsiva (raíl + lista + conversación), diálogos y toasts.

pub mod conversation;
pub mod sidebar;

use iced::alignment::{Horizontal, Vertical};
use iced::widget::{Space, button, checkbox, column, container, mouse_area, opaque, responsive, row, stack};
use iced::{Alignment, Element, Length, Padding};
use tc_core::PeerRef;
use tc_theme::tokens::{layout, radius, space, text as ts};

use crate::app::{AnimKey, App, Msg, Overlay};
use crate::format;
use crate::icon::{Icon, icon};
use crate::style::{self, c};
use crate::widgets::{hfill, t, t1, vgap};

impl App {
    pub fn view(&self) -> Element<'_, Msg> {
        responsive(move |size| {
            let narrow = size.width < layout::BREAKPOINT_SPLIT;
            let main: Element<'_, Msg> = if narrow {
                if self.conv.is_some() {
                    conversation::view(self, true)
                } else {
                    row![sidebar::rail(self), sidebar::list(self, Length::Fill)].into()
                }
            } else {
                row![
                    sidebar::rail(self),
                    sidebar::list(self, Length::Fixed(layout::LIST_WIDTH_DEFAULT)),
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
            stack(layers).width(Length::Fill).height(Length::Fill).into()
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
    };
    let panel = container(panel).padding(space::S6).width(Length::Fixed(width)).style(style::raised(b, 3, radius::XL));
    let scrim = mouse_area(container(Space::new()).width(Length::Fill).height(Length::Fill).style(style::scrim(b, k))).on_press(Msg::CloseOverlay);
    opaque(stack![scrim, container(opaque(panel)).center(Length::Fill)])
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
                .push(t1(kv.emoji.join("  "), ts::TITLE, c(b.p.text)))
                .push(t("Backend de pruebas: esta huella es ilustrativa, no sale de una clave real.", ts::CAPTION, c(b.p.warning)));
        }
    }
    col.push(row![hfill(), button(t1("Cerrar", ts::UI_STRONG, c(b.p.on_accent))).padding([8, 16]).style(style::primary(b, radius::MD)).on_press(Msg::CloseOverlay)])
        .into()
}
