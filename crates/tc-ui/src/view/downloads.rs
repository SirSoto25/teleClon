//! Descarga de archivos (fase A3): lo que cambia en la tarjeta del documento según el estado de su
//! descarga (inicio, progreso con cancelar, «Abrir» y «Mostrar en carpeta», «Reintentar»), el botón
//! «Guardar como…» y el chip «Guardar» del visor. Lógica en `app/downloads.rs`.

use iced::widget::{Column, button, container, row, tooltip};
use iced::{Alignment, Color, Element, Length};
use tc_core::{MediaKind, Message};
use tc_theme::tokens::{layout, radius, space, text as ts};

use crate::app::downloads::{Dl, DlMsg, DlState};
use crate::app::{App, Msg};
use crate::files;
use crate::icon::{Icon, icon};
use crate::style::{self, c, fade, with_weight};
use crate::widgets::t1;

use super::send_dialog::{progress_bar_of, status_text};

fn dl(m: DlMsg) -> Msg {
    Msg::Dl(m)
}

/// Color de los textos pulsables dentro de la burbuja (sobre la propia o la ajena).
fn link_color(app: &App, m: &Message) -> Color {
    let p = &app.brasa.p;
    c(if m.outgoing { p.on_bubble_out } else { p.accent_text })
}

fn link<'a>(app: &App, m: &Message, label: &'static str, msg: DlMsg) -> Element<'a, Msg> {
    button(t1(label, with_weight(ts::CAPTION, 600), link_color(app, m)))
        .padding([2, 4])
        .style(style::ghost(app.brasa, radius::SM))
        .on_press(dl(msg))
        .into()
}

fn icon_button<'a>(app: &App, ic: Icon, tip: &'static str, ink: Color, msg: Option<DlMsg>) -> Element<'a, Msg> {
    let b = app.brasa;
    tooltip(
        button(container(icon(ic, layout::ICON_SM, ink)).center(Length::Fixed(28.0))).padding(0).style(style::ghost(b, radius::PILL)).on_press_maybe(msg.map(dl)),
        container(t1(tip, ts::CAPTION, c(b.p.text))).padding([4, 8]).style(style::raised(b, 2, radius::SM)),
        tooltip::Position::Top,
    )
    .into()
}

/// Cuerpo de la tarjeta de un documento descargable: sustituye al tamaño (o al progreso) y aporta el
/// botón de la derecha, si lo hay. `info` ya lleva el nombre.
pub fn card_body<'a>(
    app: &'a App,
    m: &Message,
    md: &'a tc_core::Media,
    d: Option<&'a Dl>,
    info: Column<'a, Msg>,
    text_w: f32,
    meta_c: Color,
) -> (Column<'a, Msg>, Option<Element<'a, Msg>>) {
    let b = app.brasa;
    let id = m.id;
    let size = || t1(files::size_label(md.size_bytes), ts::CAPTION, meta_c);
    match d.map(|d| (&d.state, d)) {
        None => (info.push(size()), Some(icon_button(app, Icon::Download, "Guardar como…", meta_c, Some(DlMsg::SaveAs(id))))),
        Some((DlState::Starting | DlState::Running { .. } | DlState::Cancelling, d)) => {
            let info = info.push(t1(status_text(d), ts::CAPTION, meta_c)).push(progress_bar_of(d.fraction(), false, fade(meta_c, 0.25), c(b.p.accent), text_w));
            let cancel = (!matches!(d.state, DlState::Cancelling)).then_some(DlMsg::Cancel(id));
            (info, Some(icon_button(app, Icon::Close, "Cancelar la descarga", meta_c, cancel)))
        }
        Some((DlState::Done, _)) => {
            let actions = row![link(app, m, "Abrir", DlMsg::Open(id)), link(app, m, "Mostrar en carpeta", DlMsg::Reveal(id))].spacing(space::S1).align_y(Alignment::Center);
            (info.push(actions), None)
        }
        Some((DlState::Failed(e), _)) => {
            let short = crate::measure::ellipsize_to(e, ts::CAPTION, text_w);
            let err: Element<'a, Msg> = tooltip(
                t1(short, ts::CAPTION, c(b.p.danger)),
                container(t1(e.as_str(), ts::CAPTION, c(b.p.text))).padding([4, 8]).max_width(320.0).style(style::raised(b, 2, radius::SM)),
                tooltip::Position::Top,
            )
            .into();
            (info.push(err), Some(link(app, m, "Reintentar", DlMsg::Start(id))))
        }
    }
}

/// ¿Se ofrece «Guardar» (descarga directa) para este medio? Documentos: lo hace el clic en la tarjeta.
pub fn offers_quick_save(m: &Message) -> bool {
    m.media.as_ref().is_some_and(|md| md.kind != MediaKind::File)
}

/// Diálogo «Carpeta de descargas» (§8.12): dónde caen las descargas, elegir otra o restablecer.
pub fn dir_panel(app: &App) -> Element<'_, Msg> {
    use crate::app::downloads::default_dir;
    use crate::widgets::{hfill, t, vgap};
    let b = app.brasa;
    let (shown, note) = match &app.dl.dir {
        Some(d) if !d.is_dir() => (d.clone(), Some("Esta carpeta ya no existe: se usará la predeterminada.")),
        Some(d) => (d.clone(), None),
        None => (default_dir().unwrap_or_default(), Some("Carpeta predeterminada. La de Telegram Desktop se usa si ya existe; si no, se crea «Telegram» al descargar la primera vez.")),
    };
    let warn = app.dl.dir.as_ref().is_some_and(|d| !d.is_dir());
    let mut col = Column::new()
        .spacing(space::S3)
        .push(t1("Carpeta de descargas", ts::TITLE, c(b.p.text)))
        .push(t("Los archivos que guardes con un clic van aquí. «Guardar como…» también empieza en esta carpeta.", ts::UI, c(b.p.text_muted)))
        .push(
            container(t1(shown.display().to_string(), ts::CODE, c(b.p.text)))
                .padding([8, 12])
                .width(Length::Fill)
                .style(style::rounded_fill(c(b.p.surface_hover), style::r(radius::MD))),
        );
    if let Some(n) = note {
        col = col.push(t(n, ts::CAPTION, c(if warn { b.p.warning } else { b.p.text_muted })));
    }
    col.push(vgap(space::S2))
        .push(
            row![
                hfill(),
                button(t1("Restablecer", ts::UI_STRONG, c(b.p.text)))
                    .padding([8, 16])
                    .style(style::ghost(b, radius::MD))
                    .on_press_maybe(app.dl.is_custom().then_some(dl(DlMsg::ResetDir))),
                button(t1("Elegir carpeta…", ts::UI_STRONG, c(b.p.text)))
                    .padding([8, 16])
                    .style(style::ghost(b, radius::MD))
                    .on_press(dl(DlMsg::ChooseDir)),
                button(t1("Cerrar", ts::UI_STRONG, c(b.p.on_accent))).padding([8, 16]).style(style::primary(b, radius::MD)).on_press(Msg::CloseOverlay),
            ]
            .spacing(space::S2),
        )
        .into()
}
