//! GIFs (guardados de la cuenta y resultados de búsqueda) y uso local de emojis del panel del
//! compositor. Ver docs/adr/0009-emojis.md y docs/adr/0010-gifs.md.
use crate::ids::Timestamp;
use crate::model::FileRef;

/// MIME de un GIF «de verdad» (los de Telegram suelen ser `video/mp4` sin sonido).
pub const GIF_MIME: &str = "image/gif";

/// Resultado de un bot inline (en Telegram, el bot `@gif`): se envía con
/// `messages.sendInlineBotResult` y no como documento suelto.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct InlineResult {
    pub query_id: i64,
    pub result_id: String,
}

/// GIF que se puede mostrar en el panel y enviar.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Gif {
    /// Id del documento (estable entre sesiones; sirve de clave de caché).
    pub id: i64,
    /// Documento: `Thumb` = miniatura estática (JPEG/PNG), `Full` = el archivo (MP4 o GIF).
    pub file: Option<FileRef>,
    /// `video/mp4` (lo habitual) o `image/gif`.
    pub mime: String,
    pub width: u32,
    pub height: u32,
    pub duration_secs: Option<u32>,
    /// Texto para buscar entre los guardados (nombre de archivo o título del resultado).
    pub title: String,
    /// `Some` si viene de una búsqueda: se envía como resultado del bot.
    pub inline: Option<InlineResult>,
}

impl Gif {
    /// ¿Es un GIF animado real (se puede animar con un decodificador de GIF)?
    pub fn is_real_gif(&self) -> bool {
        self.mime.eq_ignore_ascii_case(GIF_MIME)
    }
}

/// Página de resultados de una búsqueda de GIFs.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct GifPage {
    pub gifs: Vec<Gif>,
    /// Desplazamiento para pedir la página siguiente (`None`: no hay más).
    pub next_offset: Option<String>,
}

/// Uso de un emoji en el panel (recientes y frecuentes), guardado en local.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EmojiUse {
    pub emoji: String,
    pub count: u32,
    pub last_used: Timestamp,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn solo_image_gif_es_gif_real() {
        let mut g = Gif { id: 1, file: None, mime: "video/mp4".into(), width: 1, height: 1, duration_secs: None, title: String::new(), inline: None };
        assert!(!g.is_real_gif());
        g.mime = "IMAGE/GIF".into();
        assert!(g.is_real_gif());
    }
}
