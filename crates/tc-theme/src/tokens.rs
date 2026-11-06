//! Espaciado, radios, bordes, elevación, tipografía y medidas de layout (px lógicos).

/// Escala de espaciado (`space/*`).
pub mod space {
    pub const S0: f32 = 0.0;
    pub const S0_5: f32 = 2.0;
    pub const S1: f32 = 4.0;
    pub const S1_5: f32 = 6.0;
    pub const S2: f32 = 8.0;
    pub const S3: f32 = 12.0;
    pub const S4: f32 = 16.0;
    pub const S5: f32 = 20.0;
    pub const S6: f32 = 24.0;
    pub const S8: f32 = 32.0;
    pub const S10: f32 = 40.0;
    pub const S12: f32 = 48.0;
    pub const S16: f32 = 64.0;
}

/// Radios (`radius/*`).
pub mod radius {
    pub const XS: f32 = 4.0;
    pub const SM: f32 = 8.0;
    pub const MD: f32 = 12.0;
    pub const LG: f32 = 18.0;
    pub const XL: f32 = 24.0;
    pub const PILL: f32 = 999.0;
    /// Radio del avatar = lado * ratio (cuadrado redondeado).
    pub const AVATAR_RATIO: f32 = 0.32;
    pub const BUBBLE: f32 = 18.0;
    /// Esquina de unión de burbujas consecutivas.
    pub const BUBBLE_JOINT: f32 = 6.0;
}

/// Anchos de borde (`border_width/*`).
pub mod border_width {
    pub const HAIRLINE: f32 = 1.0;
    pub const INPUT: f32 = 1.0;
    pub const FOCUS: f32 = 2.0;
    pub const FOCUS_OFFSET: f32 = 2.0;
    pub const SELECTED_BAR: f32 = 3.0;
}

/// Sombra de una elevación. Opacidad final = `alphas.shadow.opacity * alpha_scale`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ShadowSpec {
    pub y: f32,
    pub blur: f32,
    pub alpha_scale: f32,
}

/// Nivel de elevación: rol de superficie (`bg`, `surface`, `surface-raised`) y sombra.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Elevation {
    pub surface: &'static str,
    pub shadow: Option<ShadowSpec>,
}

pub const ELEVATION: [Elevation; 4] = [
    Elevation { surface: "bg", shadow: None },
    Elevation { surface: "surface", shadow: Some(ShadowSpec { y: 1.0, blur: 2.0, alpha_scale: 0.5 }) },
    Elevation { surface: "surface-raised", shadow: Some(ShadowSpec { y: 4.0, blur: 12.0, alpha_scale: 0.8 }) },
    Elevation { surface: "surface-raised", shadow: Some(ShadowSpec { y: 12.0, blur: 32.0, alpha_scale: 1.0 }) },
];

/// Familia tipográfica a usar en un estilo.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FontRole {
    Ui,
    Display,
    Mono,
    Emoji,
}

/// Estilo tipográfico (`type/*`). `tracking` en px.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TextStyle {
    pub size: f32,
    pub line: f32,
    pub weight: u16,
    pub tracking: f32,
    pub font: FontRole,
}

const fn ts(size: f32, line: f32, weight: u16, tracking: f32, font: FontRole) -> TextStyle {
    TextStyle { size, line, weight, tracking, font }
}

pub mod text {
    use super::{FontRole::*, TextStyle, ts};
    pub const CAPTION: TextStyle = ts(12.0, 16.0, 500, 0.2, Ui);
    pub const META: TextStyle = ts(13.0, 18.0, 400, 0.0, Ui);
    pub const UI: TextStyle = ts(14.0, 20.0, 400, 0.0, Ui);
    pub const UI_STRONG: TextStyle = ts(14.0, 20.0, 600, 0.0, Ui);
    pub const BODY: TextStyle = ts(15.0, 22.0, 400, 0.0, Ui);
    pub const TITLE: TextStyle = ts(16.0, 22.0, 600, -0.1, Ui);
    pub const HEADING: TextStyle = ts(20.0, 26.0, 700, -0.2, Ui);
    pub const DISPLAY: TextStyle = ts(28.0, 34.0, 700, -0.4, Display);
    pub const EMOJI_ONLY: TextStyle = ts(40.0, 46.0, 400, 0.0, Emoji);
    pub const CODE: TextStyle = ts(13.0, 20.0, 400, 0.0, Mono);
}

/// Escala de texto elegida por el usuario (`type_scale_user`).
pub mod type_scale_user {
    pub const MIN: f32 = 0.875;
    pub const MAX: f32 = 1.25;
    pub const STEP: f32 = 0.125;
    pub const DEFAULT: f32 = 1.0;

    /// Limita un factor al rango permitido.
    pub fn clamp(v: f32) -> f32 {
        v.clamp(MIN, MAX)
    }
}

/// Medidas de layout (`layout/*`).
pub mod layout {
    pub const RAIL_WIDTH: f32 = 72.0;
    pub const LIST_WIDTH_DEFAULT: f32 = 340.0;
    pub const LIST_WIDTH_MIN: f32 = 280.0;
    pub const LIST_WIDTH_MAX: f32 = 440.0;
    pub const INFO_PANEL_WIDTH: f32 = 360.0;
    pub const HEADER_HEIGHT: f32 = 56.0;
    pub const COMPOSER_MIN_HEIGHT: f32 = 52.0;
    pub const COMPOSER_MAX_LINES: u32 = 10;
    pub const BUBBLE_MAX_WIDTH: f32 = 560.0;
    pub const BUBBLE_MAX_RATIO: f32 = 0.72;
    pub const CHAT_COLUMN_MAX: f32 = 820.0;
    pub const BREAKPOINT_SPLIT: f32 = 720.0;
    pub const BREAKPOINT_TRIPLE: f32 = 1180.0;
    pub const CHAT_ITEM_HEIGHT_COMFORTABLE: f32 = 68.0;
    pub const CHAT_ITEM_HEIGHT_COMPACT: f32 = 52.0;
    pub const AVATAR_XS: f32 = 20.0;
    pub const AVATAR_SM: f32 = 28.0;
    pub const AVATAR_MD: f32 = 36.0;
    pub const AVATAR_LG: f32 = 44.0;
    pub const AVATAR_XL: f32 = 88.0;
    pub const ICON_SM: f32 = 16.0;
    pub const ICON_MD: f32 = 20.0;
    pub const ICON_LG: f32 = 24.0;
    pub const HIT_TARGET_MIN: f32 = 32.0;
}
