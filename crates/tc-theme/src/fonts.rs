//! Fuentes incluidas en el binario (assets/fonts, todas OFL-1.1) y nombres de familia.
//! Son fuentes variables; el peso se elige con `TextStyle::weight`.

use crate::tokens::FontRole;

pub const UI_FAMILY: &str = "Atkinson Hyperlegible Next";
pub const UI_FALLBACK: [&str; 3] = ["IBM Plex Sans", "Noto Sans", "Cantarell"];
pub const DISPLAY_FAMILY: &str = "Bricolage Grotesque";
pub const MONO_FAMILY: &str = "JetBrains Mono";
pub const MONO_FALLBACK: [&str; 2] = ["Noto Sans Mono", "DejaVu Sans Mono"];
/// No se empaqueta: se usa la del sistema.
pub const EMOJI_FAMILY: &str = "Noto Color Emoji";

pub const UI_REGULAR: &[u8] =
    include_bytes!("../../../assets/fonts/atkinson-hyperlegible-next/AtkinsonHyperlegibleNext[wght].ttf");
pub const UI_ITALIC: &[u8] =
    include_bytes!("../../../assets/fonts/atkinson-hyperlegible-next/AtkinsonHyperlegibleNext-Italic[wght].ttf");
pub const DISPLAY: &[u8] =
    include_bytes!("../../../assets/fonts/bricolage-grotesque/BricolageGrotesque[opsz,wdth,wght].ttf");
pub const MONO: &[u8] = include_bytes!("../../../assets/fonts/jetbrains-mono/JetBrainsMono[wght].ttf");

/// Todos los ficheros de fuente empaquetados, para registrarlos en el motor de texto.
pub const BUNDLED: [&[u8]; 4] = [UI_REGULAR, UI_ITALIC, DISPLAY, MONO];

/// Nombre de familia principal de un rol.
pub const fn family(role: FontRole) -> &'static str {
    match role {
        FontRole::Ui => UI_FAMILY,
        FontRole::Display => DISPLAY_FAMILY,
        FontRole::Mono => MONO_FAMILY,
        FontRole::Emoji => EMOJI_FAMILY,
    }
}
