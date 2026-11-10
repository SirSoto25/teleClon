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

/// Ancho de una línea de texto sin envolver (cacheado: etiquetas cortas de la lista de chats).
pub fn text_width(s: &str, style: TextStyle) -> f32 {
    use std::cell::RefCell;
    use std::collections::HashMap;
    thread_local! {
        static CACHE: RefCell<HashMap<(String, u32, u16), f32>> = RefCell::new(HashMap::new());
    }
    let key = (s.to_string(), style.size.to_bits(), style.weight);
    if let Some(w) = CACHE.with(|c| c.borrow().get(&key).copied()) {
        return w;
    }
    let w = Paragraph::with_text(base(s, style, f32::INFINITY)).min_bounds().width;
    CACHE.with(|c| {
        let mut c = c.borrow_mut();
        if c.len() > 4096 {
            c.clear();
        }
        c.insert(key, w);
    });
    w
}

/// Recorta `s` (una línea) con «…» para que quepa en `max_w` px con el estilo dado, midiendo con
/// el mismo motor que pinta (iced 0.14 no tiene elipsis nativa). Las medidas se cachean: la lista
/// de chats lo llama en cada frame para las filas visibles.
pub fn ellipsize_to(s: &str, style: TextStyle, max_w: f32) -> String {
    use std::cell::RefCell;
    use std::collections::HashMap;
    type Key = (String, u32, u16, u32);
    thread_local! {
        static CACHE: RefCell<HashMap<Key, String>> = RefCell::new(HashMap::new());
    }
    let key: Key = (s.to_string(), style.size.to_bits(), style.weight, max_w.round() as u32);
    if let Some(hit) = CACHE.with(|c| c.borrow().get(&key).cloned()) {
        return hit;
    }
    let width = |t: &str| Paragraph::with_text(base(t, style, f32::INFINITY)).min_bounds().width;
    let out = if text_width(s, style) <= max_w {
        s.to_string()
    } else {
        // Búsqueda binaria del prefijo más largo que cabe con la elipsis.
        let ends: Vec<usize> = s.char_indices().map(|(i, _)| i).skip(1).chain([s.len()]).collect();
        let fits = |n: usize| {
            let mut t = s[..if n == 0 { 0 } else { ends[n - 1] }].trim_end().to_string();
            t.push('…');
            (width(&t) <= max_w).then_some(t)
        };
        let (mut lo, mut hi) = (0usize, ends.len());
        while lo < hi {
            let mid = (lo + hi).div_ceil(2);
            if fits(mid).is_some() { lo = mid } else { hi = mid - 1 }
        }
        fits(lo).unwrap_or_else(|| "…".to_string())
    };
    CACHE.with(|c| {
        let mut c = c.borrow_mut();
        if c.len() > 4096 {
            c.clear();
        }
        c.insert(key, out.clone());
    });
    out
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
