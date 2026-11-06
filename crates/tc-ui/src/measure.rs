//! Medición de texto con el `Paragraph` de iced (cosmic-text, el mismo motor que el render) y
//! construcción de tramos con formato compartida entre medir y pintar.

use iced::advanced::text::{self, Paragraph as _};
use iced::alignment::Vertical;
use iced::font::Style as FontStyle;
use iced::{Color, Font, Pixels, Size};
use tc_theme::TextStyle;
use tc_theme::tokens::FontRole;

use crate::conv::{Measurer, Run};
use crate::style::font_of;

type Paragraph = <iced::Renderer as text::Renderer>::Paragraph;
type Font_ = <iced::Renderer as text::Renderer>::Font;

/// Envoltura por palabra con corte de glifo si una palabra no cabe (URLs largas).
pub const WRAP: text::Wrapping = text::Wrapping::WordOrGlyph;
pub const SHAPING: text::Shaping = text::Shaping::Advanced;

fn base<C>(content: C, style: TextStyle, max_w: f32) -> text::Text<C, Font_> {
    text::Text {
        content,
        bounds: Size::new(max_w, f32::INFINITY),
        size: Pixels(style.size),
        line_height: text::LineHeight::Absolute(Pixels(style.line)),
        font: font_of(style),
        align_x: text::Alignment::Default,
        align_y: Vertical::Top,
        shaping: SHAPING,
        wrapping: WRAP,
    }
}

/// Medidor real.
#[derive(Clone, Copy, Debug, Default)]
pub struct IcedMeasurer;

impl Measurer for IcedMeasurer {
    fn text(&self, content: &str, style: TextStyle, max_w: f32) -> Size {
        Paragraph::with_text(base(content, style, max_w)).min_bounds()
    }

    fn rich(&self, spans: &[Run<'_>], style: TextStyle, max_w: f32) -> Size {
        let spans = to_spans(spans, style, None, None);
        Paragraph::with_spans::<()>(base(spans.as_slice(), style, max_w)).min_bounds()
    }
}

/// Tramos de iced para un mensaje. `color`/`link_color` solo afectan al pintado.
pub fn to_spans<'a>(runs: &[Run<'a>], style: TextStyle, color: Option<Color>, link_color: Option<Color>) -> Vec<text::Span<'a, (), Font>> {
    runs.iter()
        .map(|r| {
            let mut f = if r.mono {
                font_of(TextStyle { font: FontRole::Mono, ..style })
            } else {
                font_of(if r.bold { crate::style::with_weight(style, 700) } else { style })
            };
            if r.italic {
                f.style = FontStyle::Italic;
            }
            let c = if r.link { link_color.or(color) } else { color };
            text::Span::new(r.text)
                .font(f)
                .size(Pixels(style.size))
                .line_height(text::LineHeight::Absolute(Pixels(style.line)))
                .color_maybe(c)
                .underline(r.underline || (r.link && link_color.is_none() && color.is_some()))
                .strikethrough(r.strike)
        })
        .collect()
}
