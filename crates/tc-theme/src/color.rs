//! Color sRGB sin dependencias y utilidades de contraste WCAG 2.x.

/// Color sRGB opaco de 8 bits por canal.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Rgb {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

impl Rgb {
    /// Desde `0xRRGGBB`.
    pub const fn hex(v: u32) -> Self {
        Self { r: (v >> 16) as u8, g: (v >> 8) as u8, b: v as u8 }
    }

    /// Canales como `f32` en 0..=1 (para convertir a `iced::Color` en `tc-ui`).
    pub fn to_f32(self) -> [f32; 3] {
        [self.r as f32 / 255.0, self.g as f32 / 255.0, self.b as f32 / 255.0]
    }

    /// Parsea `#rrggbb` (o `rrggbb`).
    pub fn parse(s: &str) -> Option<Self> {
        let h = s.strip_prefix('#').unwrap_or(s);
        if h.len() != 6 || !h.is_ascii() {
            return None;
        }
        u32::from_str_radix(h, 16).ok().map(Self::hex)
    }

    /// Luminancia relativa WCAG (0 negro, 1 blanco).
    pub fn luminance(self) -> f64 {
        fn lin(c: u8) -> f64 {
            let c = c as f64 / 255.0;
            if c <= 0.03928 { c / 12.92 } else { ((c + 0.055) / 1.055).powf(2.4) }
        }
        0.2126 * lin(self.r) + 0.7152 * lin(self.g) + 0.0722 * lin(self.b)
    }

    /// Razón de contraste WCAG con otro color (1.0 a 21.0).
    pub fn contrast(self, other: Rgb) -> f64 {
        let (a, b) = (self.luminance(), other.luminance());
        let (hi, lo) = if a >= b { (a, b) } else { (b, a) };
        (hi + 0.05) / (lo + 0.05)
    }
}

/// Color con opacidad (scrim, sombra, resaltados).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Alpha {
    pub rgb: Rgb,
    pub opacity: f32,
}

impl Alpha {
    pub const fn new(hex: u32, opacity: f32) -> Self {
        Self { rgb: Rgb::hex(hex), opacity }
    }
}

/// Capas translúcidas de un tema.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Alphas {
    pub scrim: Alpha,
    pub shadow: Alpha,
    pub highlight: Alpha,
    pub selection: Alpha,
    pub pressed: Alpha,
}

/// Mínimos WCAG 2.x nivel AA.
pub const AA_NORMAL: f64 = 4.5;
pub const AA_UI: f64 = 3.0;
