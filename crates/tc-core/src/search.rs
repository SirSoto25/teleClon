//! Búsqueda dentro de un chat (ADR-0012): consulta, filtros por tipo, página de resultados y la
//! regla de coincidencia de texto que comparten el mock y las pruebas (la misma que el FTS5 local).
//!
//! El orden de los resultados es **del más reciente al más antiguo** (id descendente). Se pagina
//! con `offset`: «solo mensajes con id menor que este»; `SearchPage::next` es el `offset` de la
//! página siguiente (`None` = no hay más).
use crate::ids::MsgId;
use crate::model::{EntityKind, MediaKind, Message};

/// Tipo de mensaje que se busca (los chips de la barra de búsqueda).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum SearchFilter {
    /// Cualquier mensaje con el texto (o, sin texto, no devuelve nada).
    #[default]
    All,
    Photos,
    Videos,
    /// Documentos y archivos (no audio ni voz).
    Files,
    /// Mensajes con enlaces.
    Links,
    /// Notas de voz.
    Voice,
}

impl SearchFilter {
    /// ¿Es un filtro por tipo que admite buscar sin texto (lista de todas las fotos, etc.)?
    pub fn lists_without_text(self) -> bool {
        self != SearchFilter::All
    }

    /// ¿Cumple el mensaje este filtro? (`All` siempre.) Misma definición que el SQL de `tc-store`.
    pub fn accepts(self, m: &Message) -> bool {
        let kind = m.media.as_ref().map(|md| md.kind);
        match self {
            SearchFilter::All => true,
            SearchFilter::Photos => kind == Some(MediaKind::Photo),
            SearchFilter::Videos => kind == Some(MediaKind::Video),
            SearchFilter::Files => kind == Some(MediaKind::File),
            SearchFilter::Voice => kind == Some(MediaKind::Voice),
            SearchFilter::Links => has_link(m),
        }
    }
}

fn has_link(m: &Message) -> bool {
    m.entities.iter().any(|e| e.kind == EntityKind::Url) || ["http://", "https://", "www."].iter().any(|p| m.text.to_lowercase().contains(p))
}

/// Qué buscar.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SearchQuery {
    /// Texto; vacío solo es válido con un filtro distinto de `All`.
    pub text: String,
    pub filter: SearchFilter,
    /// Solo mensajes con id **menor** que este (página siguiente); `None` = desde el más reciente.
    pub offset: Option<MsgId>,
    /// Máximo de resultados por página.
    pub limit: usize,
}

impl SearchQuery {
    pub fn new(text: impl Into<String>, limit: usize) -> Self {
        Self { text: text.into(), filter: SearchFilter::All, offset: None, limit }
    }
    pub fn with_filter(mut self, filter: SearchFilter) -> Self {
        self.filter = filter;
        self
    }
    pub fn after(mut self, offset: MsgId) -> Self {
        self.offset = Some(offset);
        self
    }
    /// ¿Hay algo que buscar? (texto no vacío, o un filtro que lista por tipo).
    pub fn is_empty(&self) -> bool {
        self.text.split_whitespace().next().is_none() && !self.filter.lists_without_text()
    }
}

/// Una página de resultados, del más reciente al más antiguo.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct SearchPage {
    pub hits: Vec<Message>,
    /// Total de coincidencias del chat, si se conoce (el servidor lo da; en local es el recuento).
    pub total: Option<u32>,
    /// `offset` de la página siguiente; `None` si no hay más.
    pub next: Option<MsgId>,
}

/// Minúsculas y sin acentos (letras latinas), como el tokenizador `unicode61 remove_diacritics`.
pub fn fold(s: &str) -> String {
    s.chars()
        .flat_map(char::to_lowercase)
        .map(|c| match c {
            'á' | 'à' | 'â' | 'ä' | 'ã' | 'å' => 'a',
            'é' | 'è' | 'ê' | 'ë' => 'e',
            'í' | 'ì' | 'î' | 'ï' => 'i',
            'ó' | 'ò' | 'ô' | 'ö' | 'õ' => 'o',
            'ú' | 'ù' | 'û' | 'ü' => 'u',
            'ñ' => 'n',
            'ç' => 'c',
            c => c,
        })
        .collect()
}

fn tokens(s: &str) -> Vec<String> {
    fold(s).split(|c: char| !c.is_alphanumeric()).filter(|t| !t.is_empty()).map(str::to_owned).collect()
}

/// ¿Coincide `text` con la consulta? Todas las palabras (AND), sin acentos ni mayúsculas, y la
/// última como prefijo: igual que la búsqueda FTS5 del almacén local. Consulta vacía: no coincide.
pub fn text_matches(query: &str, text: &str) -> bool {
    let words: Vec<&str> = query.split_whitespace().collect();
    let Some((last, init)) = words.split_last() else { return false };
    let toks = tokens(text);
    let ok = |w: &str, prefix: bool| {
        let ws = tokens(w);
        // Una «palabra» con signos se parte como en el índice: todas sus partes deben aparecer.
        ws.iter().all(|w| toks.iter().any(|t| if prefix { t.starts_with(w.as_str()) } else { t == w }))
    };
    init.iter().all(|w| ok(w, false)) && ok(last, true)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn text_rule_matches_fts() {
        assert!(text_matches("cafeteria", "Reunión mañana en la CAFETERÍA"));
        assert!(text_matches("reun", "Reunión mañana"));
        assert!(text_matches("reunion MAN", "Reunión mañana"));
        assert!(!text_matches("union", "Reunión"), "no es prefijo");
        assert!(!text_matches("reu mañ", "Reunión mañana"), "solo la última es prefijo");
        assert!(!text_matches("   ", "hola"));
    }

    #[test]
    fn empty_query_rules() {
        assert!(SearchQuery::new("  ", 10).is_empty());
        assert!(!SearchQuery::new("", 10).with_filter(SearchFilter::Photos).is_empty());
    }
}
