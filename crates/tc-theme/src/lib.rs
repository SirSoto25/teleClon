//! Tokens de diseño «Brasa» como datos puros (sin dependencia de iced). Fuente de verdad:
//! docs/design/tokens.json (ADR-0002, docs/design/system.md). `tc-ui` los traduce a estilos de iced.
//!
//! - [`palette`]: colores sRGB oscuro/claro por rol y capas translúcidas.
//! - [`tokens`]: espaciado, radios, bordes, elevación, tipografía y medidas.
//! - [`fonts`]: fuentes OFL incluidas con `include_bytes!` y nombres de familia.
//! - [`motion`]: muelles, curvas de respaldo y movimiento reducido.
//! - [`pattern`]: curvas de nivel del fondo del chat, deterministas por id de chat.
#![forbid(unsafe_code)]

pub mod color;
pub mod fonts;
pub mod tokens;
pub mod motion;
pub mod palette;
pub mod pattern;

pub use color::{AA_NORMAL, AA_UI, Alpha, Alphas, Rgb};
pub use tokens::{FontRole, TextStyle};
pub use motion::{MotionPreference, Spring};
pub use palette::Palette;

/// Tema resuelto (oscuro o claro). `system` se resuelve en `tc-ui` leyendo el portal XDG.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
pub enum ThemeKind {
    #[default]
    Dark,
    Light,
}

impl ThemeKind {
    pub const fn palette(self) -> &'static Palette {
        match self {
            Self::Dark => &palette::DARK,
            Self::Light => &palette::LIGHT,
        }
    }

    pub const fn alphas(self) -> &'static Alphas {
        match self {
            Self::Dark => &palette::DARK_ALPHAS,
            Self::Light => &palette::LIGHT_ALPHAS,
        }
    }
}

/// Degradado vertical de la burbuja saliente (ángulo 180°): de `bubble_out` a `bubble_out_end`.
pub const fn bubble_out_gradient(kind: ThemeKind) -> (Rgb, Rgb) {
    let p = kind.palette();
    (p.bubble_out, p.bubble_out_end)
}

#[cfg(test)]
mod tests;
