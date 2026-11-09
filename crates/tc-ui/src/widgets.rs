//! Primitivas compartidas (§8.8): texto con estilo de token, avatar cuadrado redondeado, badge,
//! botón icono y fondo de curvas de nivel.

use iced::alignment::{Horizontal, Vertical};
use iced::widget::canvas::{self, Path, Stroke};
use iced::widget::text::{IntoFragment, LineHeight};
use iced::widget::{Space, button, container, image, stack, text};
use iced::{Border, Color, ContentFit, Element, Length, Pixels, Rectangle, Renderer, Theme, mouse};
use tc_theme::TextStyle;
use tc_theme::pattern::{ContourParams, contours};
use tc_theme::tokens::{radius, text as ts};

use crate::app::Msg;
use crate::icon::{Icon, icon};
use crate::measure::{SHAPING, WRAP};
use crate::style::{self, Brasa, avatar_radius, font_of};

/// Texto con un estilo tipográfico de `tokens`.
pub fn t<'a>(content: impl IntoFragment<'a>, style: TextStyle, color: Color) -> text::Text<'a> {
    text(content)
        .size(Pixels(style.size))
        .line_height(LineHeight::Absolute(Pixels(style.line)))
        .font(font_of(style))
        .color(color)
        .shaping(SHAPING)
        .wrapping(WRAP)
}

/// Texto de una línea (sin envolver): listas y cabeceras.
pub fn t1<'a>(content: impl IntoFragment<'a>, style: TextStyle, color: Color) -> text::Text<'a> {
    t(content, style, color).wrapping(text::Wrapping::None)
}

pub fn vgap(h: f32) -> Space {
    Space::new().height(Length::Fixed(h))
}

pub fn hgap(w: f32) -> Space {
    Space::new().width(Length::Fixed(w))
}

pub fn hfill() -> Space {
    Space::new().width(Length::Fill)
}

/// Avatar sin foto: cuadrado redondeado (radio 0.32 × lado), color `name-n` e iniciales.
/// `online_ring` dibuja el punto «en línea» con un aro del color de fondo de la fila.
pub fn avatar<'a>(name: &str, seed: i64, size: f32, online_ring: Option<Color>, b: Brasa) -> Element<'a, Msg> {
    avatar_photo(name, seed, size, online_ring, None, b)
}

/// Avatar con la foto de perfil ya decodificada (recortada al mismo cuadrado redondeado); sin
/// foto (o mientras carga), el generado de [`avatar`].
pub fn avatar_photo<'a>(name: &str, seed: i64, size: f32, online_ring: Option<Color>, photo: Option<image::Handle>, b: Brasa) -> Element<'a, Msg> {
    let square: Element<'a, Msg> = match photo {
        Some(h) => image(h)
            .width(Length::Fixed(size))
            .height(Length::Fixed(size))
            .content_fit(ContentFit::Cover)
            .border_radius(avatar_radius(size))
            .into(),
        None => {
            let ink = style::avatar_ink();
            let initials = crate::format::initials(name);
            let font_size = (size * 0.38).round();
            let label = t1(initials, TextStyle { size: font_size, line: font_size * 1.2, ..style::with_weight(ts::UI, 600) }, ink);
            container(label)
                .width(Length::Fixed(size))
                .height(Length::Fixed(size))
                .align_x(Horizontal::Center)
                .align_y(Vertical::Center)
                .style(style::rounded_fill(Brasa::avatar_color(seed), avatar_radius(size)))
                .into()
        }
    };
    match online_ring {
        None => square,
        Some(ring) => {
            let dot = container(Space::new().width(Length::Fixed(10.0)).height(Length::Fixed(10.0))).style(move |_| {
                container::Style {
                    background: Some(style::c(b.p.success).into()),
                    border: Border { radius: style::r(radius::PILL), width: 2.0, color: ring },
                    ..Default::default()
                }
            });
            let side = size + 2.0;
            stack![
                square,
                container(dot)
                    .width(Length::Fixed(side))
                    .height(Length::Fixed(side))
                    .align_x(Horizontal::Right)
                    .align_y(Vertical::Bottom)
            ]
            .width(Length::Fixed(side))
            .height(Length::Fixed(side))
            .into()
        }
    }
}

/// Badge de no leídos (pill, mín. 20 px, «99+» por encima de 99).
pub fn badge<'a>(count: u32, muted: bool, b: Brasa) -> Element<'a, Msg> {
    let label = if count > 99 { "99+".to_string() } else { count.to_string() };
    let fg = style::c(if muted { b.p.text } else { b.p.on_accent });
    container(t1(label, ts::CAPTION, fg))
        .padding([2, 6])
        .height(Length::Fixed(20.0))
        .width(Length::Shrink)
        .align_y(Vertical::Center)
        .align_x(Horizontal::Center)
        .style(style::badge(b, muted))
        .into()
}

/// Botón de icono (área mínima 32 px).
pub fn icon_button<'a>(i: Icon, size: f32, color: Color, on: Option<Msg>, b: Brasa) -> Element<'a, Msg> {
    let pad = ((size + 12.0).max(32.0) - size) / 2.0;
    button(icon(i, size, color)).padding(pad).style(style::ghost(b, radius::SM)).on_press_maybe(on).into()
}

/// Fondo del chat: curvas de nivel deterministas por chat, con caché (se regenera al cambiar de
/// tamaño; la app limpia la caché al cambiar de chat o de tema).
pub struct ContourBg<'a> {
    pub cache: &'a canvas::Cache,
    pub seed: i64,
    pub secret: bool,
    pub color: Color,
}

impl<Message> canvas::Program<Message> for ContourBg<'_> {
    type State = ();

    fn draw(&self, _: &(), renderer: &Renderer, _: &Theme, bounds: Rectangle, _: mouse::Cursor) -> Vec<canvas::Geometry> {
        let geom = self.cache.draw(renderer, bounds.size(), |frame| {
            let params = if self.secret { ContourParams::SECRET } else { ContourParams::DEFAULT };
            let segs = contours(self.seed, bounds.width, bounds.height, params);
            let path = Path::new(|p| {
                for s in &segs {
                    p.move_to(iced::Point::new(s.a.0, s.a.1));
                    p.line_to(iced::Point::new(s.b.0, s.b.1));
                }
            });
            frame.stroke(&path, Stroke::default().with_color(self.color).with_width(params.stroke));
        });
        vec![geom]
    }
}

pub fn contour_bg<'a>(cache: &'a canvas::Cache, seed: i64, secret: bool, b: Brasa) -> Element<'a, Msg> {
    iced::widget::canvas(ContourBg { cache, seed, secret, color: style::c(b.p.pattern_line) })
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
}

