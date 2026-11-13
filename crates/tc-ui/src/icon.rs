//! Iconos Phosphor (MIT, assets/icons, ver NOTICE.md) incluidos en el binario y coloreados con el
//! tema. Peso *regular* por defecto, *fill* para el estado activo (docs/design/system.md §7).

use std::sync::LazyLock;

use iced::widget::svg;
use iced::{Color, Element, Length};

macro_rules! icons {
    ($($variant:ident => $file:literal),* $(,)?) => {
        /// Icono disponible.
        #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
        pub enum Icon { $($variant),* }

        impl Icon {
            pub const ALL: &'static [Icon] = &[$(Icon::$variant),*];

            fn bytes(self, filled: bool) -> &'static [u8] {
                match (self, filled) {
                    $(
                        (Icon::$variant, false) => include_bytes!(concat!("../../../assets/icons/regular/", $file, ".svg")),
                        (Icon::$variant, true) => include_bytes!(concat!("../../../assets/icons/fill/", $file, "-fill.svg")),
                    )*
                }
            }
        }
    };
}

icons! {
    Search => "magnifying-glass",
    Pin => "push-pin",
    Muted => "bell-slash",
    Bell => "bell",
    Check => "check",
    Checks => "checks",
    Timer => "timer",
    Warning => "warning-circle",
    Send => "paper-plane-tilt",
    Reply => "arrow-bend-up-left",
    Edit => "pencil-simple",
    Trash => "trash",
    Smiley => "smiley",
    Shield => "shield-check",
    LockKey => "lock-key",
    Close => "x",
    Sun => "sun",
    Moon => "moon",
    Fingerprint => "fingerprint",
    Back => "arrow-left",
    Down => "caret-down",
    CaretLeft => "caret-left",
    CaretRight => "caret-right",
    Chats => "chat-circle",
    Unread => "eye-slash",
    User => "user",
    Users => "users",
    Megaphone => "megaphone",
    Copy => "copy",
    Pause => "pause",
    Play => "play",
    Image => "image",
    File => "file",
    Sticker => "sticker",
    Download => "download-simple",
    Mic => "microphone",
    // Menús contextuales de chats y mensajes (`view/context_menu.rs`).
    Archive => "archive",
    More => "dots-three",
    Forward => "arrow-bend-up-right",
    Folder => "folder",
    // Grupos con temas (`view/topics.rs`); «hash» es un dibujo propio, no de Phosphor.
    Hash => "hash",
    Plus => "plus",
    Lock => "lock",
}

/// Handles creados una vez (el id del handle es un hash de los bytes; evitamos rehacerlo cada frame).
static HANDLES: LazyLock<Vec<(svg::Handle, svg::Handle)>> = LazyLock::new(|| {
    Icon::ALL
        .iter()
        .map(|i| (svg::Handle::from_memory(i.bytes(false)), svg::Handle::from_memory(i.bytes(true))))
        .collect()
});

fn handle(icon: Icon, filled: bool) -> svg::Handle {
    let idx = Icon::ALL.iter().position(|i| *i == icon).unwrap_or(0);
    let (regular, fill) = &HANDLES[idx];
    if filled { fill.clone() } else { regular.clone() }
}

/// Icono de `size` px en `color`.
pub fn icon<'a, M: 'a>(icon: Icon, size: f32, color: Color) -> Element<'a, M> {
    view(icon, false, size, color)
}

/// Variante *fill* (estado activo).
pub fn icon_fill<'a, M: 'a>(icon: Icon, size: f32, color: Color) -> Element<'a, M> {
    view(icon, true, size, color)
}

fn view<'a, M: 'a>(i: Icon, filled: bool, size: f32, color: Color) -> Element<'a, M> {
    svg(handle(i, filled))
        .width(Length::Fixed(size))
        .height(Length::Fixed(size))
        .style(move |_, _| svg::Style { color: Some(color) })
        .into()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn todos_los_iconos_son_svg_en_ambos_pesos() {
        for i in Icon::ALL {
            for filled in [false, true] {
                let b = i.bytes(filled);
                let s = std::str::from_utf8(b).expect("utf-8");
                assert!(s.contains("<svg") && s.contains("viewBox=\"0 0 256 256\""), "{i:?} {filled}");
            }
        }
    }
}
