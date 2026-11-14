//! Envío de archivos (fase A2): diálogo de envío, zona para soltar archivos y estado de subida en
//! las burbujas pendientes (barra de progreso, cancelar, «No se envió»). Lógica en `app/uploads.rs`.

use iced::alignment::Vertical;
use iced::keyboard::{Key, key::Named};
use iced::widget::text::LineHeight;
use iced::widget::{Space, button, column, container, image, mouse_area, opaque, row, scrollable, stack, text_editor, tooltip};
use iced::{Alignment, Color, ContentFit, Element, Length, Padding, Pixels, Size};
use tc_core::{MediaKind, Message, RandomId};
use tc_theme::tokens::{layout, radius, space, text as ts};

use crate::app::downloads::{DlMsg, DlState};
use crate::app::uploads::{FilesMsg, SendDialog, Upload};
use crate::app::{AnimKey, App, Msg};
use crate::files::{self, CAPTION_COUNTER_FROM, CAPTION_ID, FILE_WARN_BYTES, FileClass, MAX_CAPTION_CHARS, Picked};
use crate::format;
use crate::icon::{Icon, icon, icon_fill};
use crate::style::{self, Brasa, c, fade, with_weight};
use crate::widgets::{hfill, t, t1, vgap};

/// Ancho del diálogo de envío.
pub const DIALOG_W: f32 = 480.0;
/// Alto máximo de la vista previa de una sola imagen.
const PREVIEW_H: f32 = 300.0;
/// Lado de las miniaturas de la lista (varios archivos).
const THUMB: f32 = 48.0;
/// Grosor de la barra de progreso de una subida.
const BAR_H: f32 = 4.0;

fn files_msg(m: FilesMsg) -> Msg {
    Msg::Files(m)
}

/// Capas del envío de archivos por encima de todo: la zona de soltar y el diálogo.
pub fn layers(app: &App) -> Vec<Element<'_, Msg>> {
    let mut v = Vec::new();
    if app.uploads.hover && app.uploads.dialog.is_none() {
        v.push(drop_zone(app));
    }
    if let Some(d) = &app.uploads.dialog {
        v.push(dialog(app, d));
    }
    v
}

/// Mientras se arrastran archivos sobre la ventana: a dónde irán (o por qué no se puede).
fn drop_zone(app: &App) -> Element<'_, Msg> {
    let b = app.brasa;
    let (ic, title, sub, color) = match app.attach_block() {
        Some(r) => (Icon::Warning, "No se pueden enviar archivos aquí".to_string(), r, b.p.danger),
        None => {
            let who = app.conv.as_ref().and_then(|cv| app.dialog(cv.peer)).map(|d| format::plain_letters(&d.title).into_owned()).unwrap_or_default();
            (Icon::Paperclip, "Suelta para enviar".to_string(), format!("Los archivos irán a {}", format::ellipsize(&who, 40)), b.p.accent_text)
        }
    };
    let card = container(
        column![icon(ic, 40.0, c(color)), t1(title, ts::TITLE, c(b.p.text)), t(sub, ts::UI, c(b.p.text_muted))]
            .spacing(space::S2)
            .align_x(Alignment::Center),
    )
    .padding(space::S8)
    .center_x(Length::Fixed(DIALOG_W))
    .style(move |_| iced::widget::container::Style {
        background: Some(c(b.p.surface_raised).into()),
        border: iced::Border { radius: style::r(radius::XL), width: 2.0, color: c(color) },
        shadow: style::shadow(b, 3),
        ..Default::default()
    });
    stack![
        container(Space::new()).width(Length::Fill).height(Length::Fill).style(style::scrim(b, 1.0)),
        container(card).center(Length::Fill),
    ]
    .into()
}

/// Título del diálogo según lo que se va a enviar.
pub fn dialog_title(d: &SendDialog) -> String {
    let n = d.files.len();
    let photos = d.files.iter().filter(|f| f.send_kind(d.compress) == MediaKind::Photo).count();
    let videos = d.files.iter().filter(|f| f.send_kind(d.compress) == MediaKind::Video).count();
    match (n, photos, videos) {
        (1, 1, _) => "Enviar una foto".into(),
        (1, _, 1) => "Enviar un vídeo".into(),
        (1, _, _) => "Enviar un archivo".into(),
        (n, p, _) if p == n => format!("Enviar {n} fotos"),
        (n, p, v) if p + v == n => format!("Enviar {n} fotos y vídeos"),
        (n, ..) => format!("Enviar {n} archivos"),
    }
}

/// Avisos del diálogo (no impiden enviar, salvo el del pie).
pub fn dialog_notes(d: &SendDialog) -> Vec<String> {
    let mut v = Vec::new();
    let big = d.files.iter().filter(|f| f.size > FILE_WARN_BYTES).count();
    if big > 0 {
        v.push("Telegram admite hasta 2 GB por archivo (4 GB con Premium): puede que no se envíe.".to_string());
    }
    if d.compress && d.files.iter().any(|f| f.class == FileClass::Image && !f.can_be_photo()) {
        v.push("Las imágenes de más de 10 MB o muy alargadas se envían como archivo.".to_string());
    }
    let groups = files::plan(&d.files, d.compress).len();
    if groups > 1 {
        v.push(format!("Se enviará en {groups} mensajes agrupados (hasta {} por grupo; fotos y documentos van aparte).", files::ALBUM_MAX));
    }
    v
}

fn dialog<'a>(app: &'a App, d: &'a SendDialog) -> Element<'a, Msg> {
    let b = app.brasa;
    let k = app.anim.value(AnimKey::Overlay, 1.0);
    let scale = if app.anim.pref.is_reduced() { 1.0 } else { 0.97 + 0.03 * k };
    let mut col = column![t1(dialog_title(d), ts::TITLE, c(b.p.text))].spacing(space::S3);

    // Una sola imagen: vista previa grande. Si no, la lista con miniatura, nombre, tamaño y quitar.
    match d.files.as_slice() {
        [one] if one.preview.is_some() && one.send_kind(d.compress) == MediaKind::Photo => col = col.push(big_preview(b, one)),
        files => {
            let mut list = column![].spacing(space::S1);
            for (i, f) in files.iter().enumerate() {
                list = list.push(file_row(b, f, i, d.compress));
            }
            let h = (files.len() as f32 * (THUMB + space::S1 + 2.0 * space::S1)).min(280.0);
            col = col.push(scrollable(list).height(Length::Fixed(h)).style(style::thin_scroll(b)));
        }
    }

    if d.has_photos() {
        col = col.push(compress_choice(b, d.compress));
    }

    let editor = text_editor(&d.caption)
        .id(CAPTION_ID)
        .placeholder("Añade un pie de foto…")
        .on_action(|a| files_msg(FilesMsg::Caption(a)))
        .font(style::font_of(ts::BODY))
        .size(Pixels(ts::BODY.size))
        .line_height(LineHeight::Absolute(Pixels(ts::BODY.line)))
        .padding(Padding::from([7.0, 12.0]))
        .height(Length::Shrink)
        .min_height(36.0)
        .max_height(ts::BODY.line * 5.0 + 14.0)
        .style(style::composer(b))
        .key_binding(|kp| {
            if !matches!(kp.status, text_editor::Status::Focused { .. }) {
                return text_editor::Binding::from_key_press(kp);
            }
            match kp.key.as_ref() {
                // Enter envía (Mayús+Enter, salto de línea) y Esc cierra.
                Key::Named(Named::Enter) if !kp.modifiers.shift() => Some(text_editor::Binding::Custom(files_msg(FilesMsg::Send))),
                Key::Named(Named::Escape) => Some(text_editor::Binding::Custom(files_msg(FilesMsg::Close))),
                _ => text_editor::Binding::from_key_press(kp),
            }
        });
    col = col.push(editor);

    let len = files::caption_len(&d.caption.text());
    let too_long = len > MAX_CAPTION_CHARS;
    if too_long {
        col = col.push(
            row![
                icon(Icon::Warning, layout::ICON_SM, c(b.p.danger)),
                t(format!("El pie es demasiado largo: {len} de {MAX_CAPTION_CHARS} caracteres."), ts::CAPTION, c(b.p.danger)),
            ]
            .spacing(space::S1)
            .align_y(Alignment::Center),
        );
    } else if len >= CAPTION_COUNTER_FROM {
        col = col.push(row![hfill(), t1(format!("{len} / {MAX_CAPTION_CHARS}"), ts::CAPTION, c(b.p.text_muted))]);
    }
    for n in dialog_notes(d) {
        col = col.push(row![icon(Icon::Warning, layout::ICON_SM, c(b.p.warning)), t(n, ts::CAPTION, c(b.p.text_muted))].spacing(space::S1).align_y(Alignment::Center));
    }

    let send_label = if d.files.len() > 1 { format!("Enviar {}", d.files.len()) } else { "Enviar".to_string() };
    col = col.push(vgap(space::S1)).push(
        row![
            button(row![icon(Icon::Plus, layout::ICON_SM, c(b.p.accent_text)), t1("Añadir", ts::UI_STRONG, c(b.p.accent_text))].spacing(space::S1).align_y(Alignment::Center))
                .padding([8, 12])
                .style(style::ghost(b, radius::MD))
                .on_press(files_msg(FilesMsg::Pick)),
            hfill(),
            button(t1("Cancelar", ts::UI_STRONG, c(b.p.text))).padding([8, 16]).style(style::ghost(b, radius::MD)).on_press(files_msg(FilesMsg::Close)),
            send_button(b, send_label, too_long),
        ]
        .spacing(space::S2)
        .align_y(Alignment::Center),
    );
    col = col.push(t1("Enter envía · Mayús+Enter, nueva línea · Esc cierra", ts::CAPTION, c(b.p.text_muted)));

    let panel = container(col).padding(space::S6).width(Length::Fixed(DIALOG_W * scale)).style(style::raised(b, 3, radius::XL));
    let scrim = mouse_area(container(Space::new()).width(Length::Fill).height(Length::Fill).style(style::scrim(b, k))).on_press(files_msg(FilesMsg::Close));
    opaque(stack![scrim, container(opaque(panel)).center(Length::Fill)])
}

/// «Enviar»: apagado (sin acción) con el pie demasiado largo.
fn send_button<'a>(b: Brasa, label: String, disabled: bool) -> Element<'a, Msg> {
    if disabled {
        return button(t1(label, ts::UI_STRONG, c(b.p.text_muted))).padding([8, 16]).style(style::send_disabled(b)).into();
    }
    button(t1(label, ts::UI_STRONG, c(b.p.on_accent))).padding([8, 16]).style(style::primary(b, radius::MD)).on_press(files_msg(FilesMsg::Send)).into()
}

fn big_preview<'a>(b: Brasa, f: &'a Picked) -> Element<'a, Msg> {
    let Some(h) = f.preview.clone() else { return Space::new().into() };
    let inner = DIALOG_W - 2.0 * space::S6;
    let (w, ht) = (f.width.max(1) as f32, f.height.max(1) as f32);
    let k = (inner / w).min(PREVIEW_H / ht);
    let pic = image(h).width(Length::Fixed(w * k)).height(Length::Fixed(ht * k)).content_fit(ContentFit::Contain).border_radius(style::r(radius::MD));
    column![
        container(pic).center_x(Length::Fill).style(style::rounded_fill(c(b.p.surface_sunken), style::r(radius::MD))),
        t1(format!("{} · {}", format::ellipsize(&f.name, 48), files::size_label(f.size)), ts::CAPTION, c(b.p.text_muted)),
    ]
    .spacing(space::S1)
    .into()
}

/// Icono de un archivo según lo que es.
fn kind_icon(class: FileClass) -> Icon {
    match class {
        FileClass::Image => Icon::Image,
        FileClass::Video => Icon::Play,
        FileClass::Audio => Icon::Mic,
        FileClass::Other => Icon::File,
    }
}

fn file_row<'a>(b: Brasa, f: &'a Picked, i: usize, compress: bool) -> Element<'a, Msg> {
    let thumb: Element<'a, Msg> = match &f.preview {
        Some(h) => image(h.clone())
            .width(Length::Fixed(THUMB))
            .height(Length::Fixed(THUMB))
            .content_fit(ContentFit::Cover)
            .border_radius(style::r(radius::SM))
            .into(),
        None => container(icon(kind_icon(f.class), layout::ICON_LG, c(b.p.accent_text)))
            .center(Length::Fixed(THUMB))
            .style(style::rounded_fill(c(b.p.surface_sunken), style::r(radius::SM)))
            .into(),
    };
    let how = match f.send_kind(compress) {
        MediaKind::Photo => "foto",
        MediaKind::Video => "vídeo",
        MediaKind::Audio => "audio",
        _ => "archivo",
    };
    let remove = tooltip(
        button(icon(Icon::Close, layout::ICON_SM, c(b.p.text_muted))).padding(6).style(style::ghost(b, radius::SM)).on_press(files_msg(FilesMsg::Remove(i))),
        container(t1("Quitar", ts::CAPTION, c(b.p.text))).padding([4, 8]).style(style::raised(b, 2, radius::SM)),
        tooltip::Position::Left,
    );
    row![
        thumb,
        column![
            t1(format::ellipsize(&f.name, 44), ts::UI_STRONG, c(b.p.text)),
            t1(format!("{} · {how}", files::size_label(f.size)), ts::META, c(b.p.text_muted)),
        ]
        .spacing(2),
        hfill(),
        remove,
    ]
    .spacing(space::S3)
    .padding(space::S1)
    .align_y(Alignment::Center)
    .into()
}

/// «Comprimir fotos» | «Enviar como archivo»: dos opciones, la elegida marcada (texto, no solo color).
fn compress_choice<'a>(b: Brasa, compress: bool) -> Element<'a, Msg> {
    let opt = |label: &'static str, hint: &'static str, on: bool, value: bool| -> Element<'a, Msg> {
        let mark: Element<'a, Msg> =
            if on { icon_fill(Icon::Check, layout::ICON_SM, c(b.p.accent_text)) } else { Space::new().width(Length::Fixed(layout::ICON_SM)).into() };
        button(
            row![mark, column![t1(label, ts::UI_STRONG, c(b.p.text)), t1(hint, ts::CAPTION, c(b.p.text_muted))]]
                .spacing(space::S2)
                .align_y(Alignment::Center),
        )
        .width(Length::Fill)
        .padding([6, 10])
        .style(style::option_row(b, on))
        .on_press(files_msg(FilesMsg::Compress(value)))
        .into()
    };
    row![
        opt("Comprimir foto", "Más ligera, se ve al momento", compress, true),
        opt("Enviar como archivo", "Calidad original", !compress, false),
    ]
    .spacing(space::S2)
    .into()
}

// ------------------------------------------------------------------ burbujas

/// Medio de una burbuja pendiente de subida: la vista previa local (fotos) o la tarjeta del archivo,
/// con la barra de progreso y «Cancelar» (o «No se envió» si falló; «Reintentar» va al lado).
pub fn upload_box<'a>(app: &'a App, m: &'a Message, md: &'a tc_core::Media, sz: Size, opacity: f32, meta_c: Color) -> Option<Element<'a, Msg>> {
    let (first, job) = app.upload_of(m)?;
    let b = app.brasa;
    let rid = m.random_id?;
    let preview = app.uploads.previews.get(&rid).cloned();
    let card = sz.height < 100.0 || preview.is_none() && md.kind == MediaKind::File;
    if card {
        return Some(file_card(app, m, md, sz, opacity, meta_c, Some((first, job))));
    }
    let base: Element<'a, Msg> = match preview {
        Some(h) => image(h)
            .width(Length::Fixed(sz.width))
            .height(Length::Fixed(sz.height))
            .content_fit(ContentFit::Cover)
            .border_radius(style::r(radius::MD))
            .opacity(opacity)
            .into(),
        None => container(
            column![
                icon(kind_icon(files::classify(md.file_name.as_deref().unwrap_or_default())), 32.0, meta_c),
                t1(format::ellipsize(md.file_name.as_deref().unwrap_or_default(), 32), ts::CAPTION, meta_c)
            ]
            .spacing(space::S1)
            .align_x(Alignment::Center),
        )
        .center_x(Length::Fixed(sz.width))
        .center_y(Length::Fixed(sz.height))
        .style(style::rounded_fill(fade(c(b.p.surface_hover), opacity), style::r(radius::MD)))
        .into(),
    };
    // Sobre la foto: velo y un chip abajo con el estado y el botón de cancelar.
    let ink = Color::WHITE;
    let chip = container(
        column![
            row![t1(status_label(job), ts::CAPTION, ink), hfill(), cancel_button(b, first, job, ink)].align_y(Alignment::Center).spacing(space::S1),
            progress_bar(job, fade(ink, 0.3), ink, sz.width - 2.0 * space::S2 - 2.0 * space::S2),
        ]
        .spacing(space::S1),
    )
    .padding(Padding::from([space::S1, space::S2]))
    .style(style::rounded_fill(Color { a: 0.6, ..Color::BLACK }, style::r(radius::SM)));
    Some(
        stack![
            base,
            container(chip).width(Length::Fixed(sz.width)).height(Length::Fixed(sz.height)).align_y(Vertical::Bottom).padding(space::S2),
        ]
        .into(),
    )
}

/// Tarjeta de documento: icono, nombre y tamaño (y, si se está subiendo, progreso y cancelar).
#[allow(clippy::too_many_arguments)]
pub fn file_card<'a>(
    app: &'a App,
    m: &Message,
    md: &'a tc_core::Media,
    sz: Size,
    opacity: f32,
    meta_c: Color,
    upload: Option<(RandomId, &'a Upload)>,
) -> Element<'a, Msg> {
    let b = app.brasa;
    let fg = fade(c(if m.outgoing { b.p.on_bubble_out } else { b.p.on_bubble_in }), opacity);
    let name = md.file_name.as_deref().unwrap_or("Archivo");
    let class = files::classify(name);
    let ic = container(icon_fill(kind_icon(class), layout::ICON_MD, c(b.p.on_accent)))
        .center(Length::Fixed(40.0))
        .style(style::rounded_fill(fade(c(b.p.accent), opacity), style::r(radius::PILL)));
    let dl = if upload.is_none() && app.can_save(m) { Some(app.dl.get(m.peer, m.id)) } else { None };
    // Ancho que se lleva lo de la derecha (cancelar, guardar como…, reintentar).
    let trailing = match (&upload, dl.flatten().map(|d| &d.state)) {
        (Some(_), _) => 32.0 + space::S2,
        (None, Some(DlState::Failed(_))) => 76.0 + space::S2,
        (None, Some(DlState::Done)) => 0.0,
        (None, _) if dl.is_some() => 32.0 + space::S2,
        _ => 0.0,
    };
    let text_w = (sz.width - 40.0 - space::S3 - trailing).max(40.0);
    let name_el = t1(crate::measure::ellipsize_to(name, with_weight(ts::UI, 600), text_w), with_weight(ts::UI, 600), fg);
    let mut info = column![name_el].spacing(2).width(Length::Fixed(text_w));
    let mut line = row![ic].spacing(space::S3).align_y(Alignment::Center);
    match (upload, dl) {
        (Some((first, job)), _) => {
            info = info.push(t1(status_label(job), ts::CAPTION, meta_c)).push(progress_bar(job, fade(meta_c, 0.25), c(b.p.accent), text_w));
            line = line.push(info).push(cancel_button(b, first, job, meta_c));
        }
        (None, Some(d)) => {
            let (info, trail) = super::downloads::card_body(app, m, md, d, info, text_w, meta_c);
            line = line.push(info);
            if let Some(t) = trail {
                line = line.push(t);
            }
            // Toda la tarjeta es un botón: descargar (o abrir, si ya está).
            let id = m.id;
            return mouse_area(container(line).width(Length::Fixed(sz.width)).height(Length::Fixed(sz.height)).align_y(Vertical::Center))
                .interaction(iced::mouse::Interaction::Pointer)
                .on_press(Msg::Dl(DlMsg::Start(id)))
                .into();
        }
        (None, None) => {
            info = info.push(t1(files::size_label(md.size_bytes), ts::CAPTION, meta_c));
            line = line.push(info);
        }
    }
    container(line).width(Length::Fixed(sz.width)).height(Length::Fixed(sz.height)).align_y(Vertical::Center).into()
}

/// Texto del estado de una subida.
pub fn status_label(job: &Upload) -> String {
    if job.failed {
        return "No se envió".into();
    }
    if job.cancelling {
        return "Cancelando…".into();
    }
    match job.fraction() {
        Some(f) => format!("{} % · {}", (f * 100.0).floor() as u32, files::progress_label(job.done, job.total)),
        None => format!("Enviando… · {}", files::size_label(job.total)),
    }
}

fn progress_bar<'a>(job: &Upload, track: Color, fill: Color, w: f32) -> Element<'a, Msg> {
    progress_bar_of(job.fraction(), job.failed, track, fill, w)
}

/// Barra de progreso de `w` px. Sin progreso conocido, un tramo fijo al principio (indeterminado).
pub(super) fn progress_bar_of<'a>(fraction: Option<f32>, failed: bool, track: Color, fill: Color, w: f32) -> Element<'a, Msg> {
    let w = w.max(8.0);
    let frac = if failed { 0.0 } else { fraction.unwrap_or(0.12) };
    let filled = container(Space::new().width(Length::Fixed((w * frac).max(if frac > 0.0 { BAR_H } else { 0.0 }))).height(Length::Fixed(BAR_H)))
        .style(style::rounded_fill(fill, style::r(BAR_H / 2.0)));
    container(filled).width(Length::Fixed(w)).height(Length::Fixed(BAR_H)).style(style::rounded_fill(track, style::r(BAR_H / 2.0))).into()
}

/// Texto del estado de una descarga.
pub fn status_text(d: &crate::app::downloads::Dl) -> String {
    use crate::app::downloads::DlState;
    match d.state {
        DlState::Cancelling => "Cancelando…".into(),
        DlState::Running { done, total } if total > 0 => format!("{} % · {}", (d.fraction().unwrap_or(0.0) * 100.0).floor() as u32, files::progress_label(done, total)),
        DlState::Running { done, .. } => format!("Descargando… · {}", files::size_label(done)),
        _ => "Descargando…".into(),
    }
}

fn cancel_button<'a>(b: Brasa, first: RandomId, job: &Upload, ink: Color) -> Element<'a, Msg> {
    if job.failed {
        return icon(Icon::Warning, layout::ICON_MD, c(b.p.danger));
    }
    let msg = (!job.cancelling).then_some(files_msg(FilesMsg::Cancel(first)));
    tooltip(
        button(container(icon(Icon::Close, layout::ICON_SM, ink)).center(Length::Fixed(28.0))).padding(0).style(style::ghost(b, radius::PILL)).on_press_maybe(msg),
        container(t1("Cancelar el envío", ts::CAPTION, c(b.p.text))).padding([4, 8]).style(style::raised(b, 2, radius::SM)),
        tooltip::Position::Top,
    )
    .into()
}
