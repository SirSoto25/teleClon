//! Panel junto al compositor con pestañas inferiores «Emojis | GIFs | Stickers» (Ctrl+E). El panel
//! abierto es `App::picker.open` (el selector de stickers de siempre pasa a ser una pestaña); aquí
//! viven la pestaña elegida y el estado de las pestañas de emojis y GIFs. La lógica está en
//! `app/panel.rs` y las vistas en `view/panel.rs`, `view/emoji_panel.rs` y `view/gif_panel.rs`.

use tc_core::{BackendResult, EmojiUse};

use crate::emoji::{Category, Tone, Usage};
use crate::gifs::{GifMsg, GifPanel};

/// Pestaña del panel.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
pub enum PanelTab {
    #[default]
    Emoji,
    Gifs,
    Stickers,
}

impl PanelTab {
    pub const ALL: [PanelTab; 3] = [PanelTab::Emoji, PanelTab::Gifs, PanelTab::Stickers];

    pub fn label(self) -> &'static str {
        match self {
            PanelTab::Emoji => "Emojis",
            PanelTab::Gifs => "GIFs",
            PanelTab::Stickers => "Stickers",
        }
    }

    /// Atajo Alt+1/2/3.
    pub fn from_digit(d: &str) -> Option<PanelTab> {
        match d {
            "1" => Some(PanelTab::Emoji),
            "2" => Some(PanelTab::Gifs),
            "3" => Some(PanelTab::Stickers),
            _ => None,
        }
    }

    /// ¿Las flechas, Enter y Tab del compositor mueven el cursor de la rejilla? (en emojis no: el
    /// panel sigue abierto mientras se escribe y las flechas mueven el cursor del texto).
    pub fn grid_keys(self) -> bool {
        self != PanelTab::Emoji
    }
}

/// Estado de la pestaña de emojis.
#[derive(Debug, Default)]
pub struct EmojiPanel {
    pub category: Category,
    pub query: String,
    pub tone: Tone,
    /// Fila de tonos de piel desplegada.
    pub tones_open: bool,
    pub usage: Usage,
    /// Ya se pidió el uso guardado al backend.
    pub usage_requested: bool,
}

impl EmojiPanel {
    /// Categoría que se pinta: «Recientes» sin usos todavía muestra las caras.
    pub fn shown_category(&self) -> Category {
        if self.category == Category::Recent && self.usage.is_empty() { Category::Smileys } else { self.category }
    }
}

/// Estado del panel (además de `App::picker`).
#[derive(Debug, Default)]
pub struct Panel {
    /// Pestaña actual; se conserva al cerrar (Ctrl+E reabre en la última usada).
    pub tab: PanelTab,
    pub emoji: EmojiPanel,
    pub gifs: GifPanel,
}

/// Id del buscador de emojis.
pub const EMOJI_SEARCH_ID: &str = "emoji-search";
/// Id del buscador de GIFs.
pub const GIF_SEARCH_ID: &str = "gif-search";

#[derive(Debug, Clone)]
pub enum PanelMsg {
    Tab(PanelTab),
    /// Clic fuera del panel (sobre la conversación).
    Close,
    Category(crate::emoji::Category),
    EmojiQuery(String),
    /// Enter en el buscador de emojis: inserta el primero.
    EmojiSubmit,
    /// Inserta este emoji en el cursor del compositor.
    Insert(String),
    ToggleTones,
    Tone(Tone),
    /// Uso guardado de emojis.
    Usage(BackendResult<Vec<EmojiUse>>),
    /// Respuesta al guardar un uso (se ignora: no merece un aviso).
    Recorded(BackendResult<()>),
    Gif(GifMsg),
}
