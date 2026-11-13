//! Teclado de emojis: catálogo Unicode completo (crate `emojis`, datos CLDR empaquetados, sin red),
//! categorías, búsqueda (palabras en español de `emoji_es.rs` + nombres CLDR en inglés + atajos de
//! GitHub), tono de piel y recientes/frecuentes. Ver docs/adr/0009-emojis.md.
//!
//! Los emojis se dibujan como texto: cosmic-text (el motor de texto de iced) recurre a la fuente de
//! color del sistema (Noto Color Emoji en Linux, Segoe UI Emoji en Windows, Apple Color Emoji en
//! macOS). Para no pintar «tofu» con fuentes algo antiguas se ofrecen solo los de Emoji ≤ 15.0.

use std::collections::HashMap;
use std::sync::OnceLock;

use emojis::{Emoji, EmojiVersion, Group, SkinTone};
use tc_core::EmojiUse;

use crate::format::fold;

/// Versión de Emoji más alta que se ofrece (Windows 11 y Noto la cubren desde 2023).
pub const MAX_VERSION: EmojiVersion = EmojiVersion::new(15, 0);
/// Recientes que se muestran en «Recientes».
pub const RECENT_SHOWN: usize = 24;
/// Frecuentes que se muestran debajo de los recientes.
pub const FREQUENT_SHOWN: usize = 32;
/// Tope de resultados de búsqueda.
pub const SEARCH_MAX: usize = 160;

/// Categoría del teclado (las de CLDR más «Recientes»).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
pub enum Category {
    #[default]
    Recent,
    Smileys,
    People,
    Animals,
    Food,
    Travel,
    Activities,
    Objects,
    Symbols,
    Flags,
}

impl Category {
    pub const ALL: [Category; 10] = [
        Category::Recent,
        Category::Smileys,
        Category::People,
        Category::Animals,
        Category::Food,
        Category::Travel,
        Category::Activities,
        Category::Objects,
        Category::Symbols,
        Category::Flags,
    ];

    pub fn group(self) -> Option<Group> {
        Some(match self {
            Category::Recent => return None,
            Category::Smileys => Group::SmileysAndEmotion,
            Category::People => Group::PeopleAndBody,
            Category::Animals => Group::AnimalsAndNature,
            Category::Food => Group::FoodAndDrink,
            Category::Travel => Group::TravelAndPlaces,
            Category::Activities => Group::Activities,
            Category::Objects => Group::Objects,
            Category::Symbols => Group::Symbols,
            Category::Flags => Group::Flags,
        })
    }

    pub fn label(self) -> &'static str {
        match self {
            Category::Recent => "Recientes",
            Category::Smileys => "Caras y emociones",
            Category::People => "Personas",
            Category::Animals => "Animales y naturaleza",
            Category::Food => "Comida y bebida",
            Category::Travel => "Viajes y lugares",
            Category::Activities => "Actividades",
            Category::Objects => "Objetos",
            Category::Symbols => "Símbolos",
            Category::Flags => "Banderas",
        }
    }

    /// Emoji que hace de icono de la pestaña (`Recent` usa el icono del reloj).
    pub fn icon(self) -> &'static str {
        match self {
            Category::Recent => "🕘",
            Category::Smileys => "😀",
            Category::People => "👋",
            Category::Animals => "🐻",
            Category::Food => "🍔",
            Category::Travel => "✈️",
            Category::Activities => "⚽",
            Category::Objects => "💡",
            Category::Symbols => "🔣",
            Category::Flags => "🏁",
        }
    }

    /// Palabras (sin tildes) que encuentran el grupo entero al buscar.
    fn words(self) -> &'static str {
        match self {
            Category::Recent => "",
            Category::Smileys => "cara caras emocion emociones",
            Category::People => "persona personas mano manos gente cuerpo",
            Category::Animals => "animal animales naturaleza planta",
            Category::Food => "comida bebida alimento",
            Category::Travel => "viaje viajes lugar lugares transporte",
            Category::Activities => "actividad actividades deporte juego",
            Category::Objects => "objeto objetos",
            Category::Symbols => "simbolo simbolos signo",
            Category::Flags => "bandera banderas pais",
        }
    }

    fn of(g: Group) -> Category {
        Category::ALL.into_iter().find(|c| c.group() == Some(g)).unwrap_or(Category::Symbols)
    }
}

fn offered(e: &Emoji) -> bool {
    e.emoji_version() <= MAX_VERSION
}

/// Emojis de cada grupo (sin variantes de tono), en orden CLDR.
fn lists() -> &'static HashMap<Group, Vec<&'static Emoji>> {
    static L: OnceLock<HashMap<Group, Vec<&'static Emoji>>> = OnceLock::new();
    L.get_or_init(|| {
        let mut m: HashMap<Group, Vec<&'static Emoji>> = HashMap::new();
        for e in emojis::iter().filter(|e| offered(e)) {
            m.entry(e.group()).or_default().push(e);
        }
        m
    })
}

/// Emojis de una categoría (vacío para `Recent`, que sale del uso).
pub fn list(cat: Category) -> &'static [&'static Emoji] {
    cat.group().and_then(|g| lists().get(&g)).map_or(&[], Vec::as_slice)
}

/// Total del catálogo ofrecido (sin tonos).
pub fn catalog_len() -> usize {
    lists().values().map(Vec::len).sum()
}

/// Palabras en español por emoji (clave: la forma totalmente cualificada).
fn es_index() -> &'static HashMap<&'static str, &'static str> {
    static I: OnceLock<HashMap<&'static str, &'static str>> = OnceLock::new();
    I.get_or_init(|| crate::emoji_es::KEYWORDS.iter().filter_map(|(e, w)| emojis::get(e).map(|x| (x.as_str(), *w))).collect())
}

/// Palabras en español de un emoji (o vacío).
pub fn spanish_words(e: &Emoji) -> &'static str {
    es_index().get(e.as_str()).copied().unwrap_or("")
}

/// Nombre que se muestra al pasar por encima: la primera palabra en español o el nombre CLDR.
pub fn display_name(e: &Emoji) -> String {
    let es = spanish_words(e);
    if es.is_empty() { e.name().to_string() } else { es.split_whitespace().take(3).collect::<Vec<_>>().join(" ") }
}

/// ¿Alguna palabra de `hay` empieza por `w`? 0 = es la primera palabra, 1 = otra palabra exacta,
/// 2 = prefijo de alguna.
fn word_rank(hay: &str, w: &str) -> Option<u8> {
    hay.split(|c: char| !c.is_alphanumeric())
        .filter(|x| !x.is_empty())
        .enumerate()
        .filter_map(|(i, x)| if x == w { Some(u8::from(i > 0)) } else { x.starts_with(w).then_some(2) })
        .min()
}

/// Busca por nombre. Cada palabra de la consulta debe casar (prefijo) con alguna palabra del
/// emoji: primero las palabras en español, luego el nombre inglés y los atajos, luego el grupo.
pub fn search(query: &str) -> Vec<&'static Emoji> {
    let q = fold(query.trim());
    let words: Vec<&str> = q.split_whitespace().collect();
    if words.is_empty() {
        return Vec::new();
    }
    let mut hits: Vec<(u8, u8, usize, &'static Emoji)> = Vec::new();
    for (i, e) in emojis::iter().filter(|e| offered(e)).enumerate() {
        let es = spanish_words(e);
        let en = fold(&format!("{} {}", e.name(), e.shortcodes().collect::<Vec<_>>().join(" ")));
        let group = Category::of(e.group()).words();
        let mut worst = 0u8;
        let mut ok = true;
        for w in &words {
            let r = word_rank(es, w).or_else(|| word_rank(&en, w).map(|r| r + 3)).or_else(|| word_rank(group, w).map(|_| 6));
            match r {
                Some(r) => worst = worst.max(r),
                None => {
                    ok = false;
                    break;
                }
            }
        }
        if ok {
            // Con el mismo rango, antes el emoji cuyas palabras son justo la consulta («gato» → 🐱, no 😺).
            let exact = u8::from(es != q);
            hits.push((worst, exact, i, e));
        }
    }
    hits.sort_by_key(|(r, x, i, _)| (*r, *x, *i));
    hits.into_iter().take(SEARCH_MAX).map(|(_, _, _, e)| e).collect()
}

/// Tono de piel elegido para los emojis que lo admiten.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
pub enum Tone {
    #[default]
    Default,
    Light,
    MediumLight,
    Medium,
    MediumDark,
    Dark,
}

impl Tone {
    pub const ALL: [Tone; 6] = [Tone::Default, Tone::Light, Tone::MediumLight, Tone::Medium, Tone::MediumDark, Tone::Dark];

    fn skin(self) -> SkinTone {
        match self {
            Tone::Default => SkinTone::Default,
            Tone::Light => SkinTone::Light,
            Tone::MediumLight => SkinTone::MediumLight,
            Tone::Medium => SkinTone::Medium,
            Tone::MediumDark => SkinTone::MediumDark,
            Tone::Dark => SkinTone::Dark,
        }
    }

    /// Mano de muestra con este tono (botón del selector).
    pub fn sample(self) -> &'static str {
        apply(emojis::get("👋").expect("👋 está en el catálogo"), self)
    }

    pub fn label(self) -> &'static str {
        match self {
            Tone::Default => "Tono por defecto",
            Tone::Light => "Tono claro",
            Tone::MediumLight => "Tono medio claro",
            Tone::Medium => "Tono medio",
            Tone::MediumDark => "Tono medio oscuro",
            Tone::Dark => "Tono oscuro",
        }
    }
}

/// El emoji con el tono `tone` si lo admite (si no, tal cual).
pub fn apply(e: &'static Emoji, tone: Tone) -> &'static str {
    if tone == Tone::Default || e.skin_tone().is_none() {
        return e.as_str();
    }
    e.with_skin_tone(tone.skin()).map_or(e.as_str(), Emoji::as_str)
}

/// Uso de emojis de la cuenta (recientes por fecha, frecuentes por veces), persistido por el
/// backend (`StickersApi::emoji_usage`, en tc-store la tabla `emoji_usage`).
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Usage {
    map: HashMap<String, (u32, i64)>,
}

impl Usage {
    /// Suma lo guardado a lo que ya hubiera en memoria (usos de antes de que llegara la carga).
    pub fn merge(&mut self, list: Vec<EmojiUse>) {
        for u in list {
            let e = self.map.entry(u.emoji).or_insert((0, u.last_used.0));
            e.0 += u.count;
            e.1 = e.1.max(u.last_used.0);
        }
    }

    pub fn record(&mut self, emoji: &str, now: i64) {
        let e = self.map.entry(emoji.to_string()).or_insert((0, now));
        e.0 += 1;
        e.1 = e.1.max(now);
    }

    pub fn is_empty(&self) -> bool {
        self.map.is_empty()
    }

    /// Los `n` usados más recientemente.
    pub fn recent(&self, n: usize) -> Vec<&str> {
        let mut v: Vec<_> = self.map.iter().collect();
        v.sort_by(|a, b| b.1.1.cmp(&a.1.1).then(b.1.0.cmp(&a.1.0)).then(a.0.cmp(b.0)));
        v.into_iter().take(n).map(|(e, _)| e.as_str()).collect()
    }

    /// Los `n` más usados.
    pub fn frequent(&self, n: usize) -> Vec<&str> {
        let mut v: Vec<_> = self.map.iter().collect();
        v.sort_by(|a, b| b.1.0.cmp(&a.1.0).then(b.1.1.cmp(&a.1.1)).then(a.0.cmp(b.0)));
        v.into_iter().take(n).map(|(e, _)| e.as_str()).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tc_core::Timestamp;

    #[test]
    fn el_catalogo_es_completo_y_por_categorias() {
        assert!(catalog_len() > 1800, "catálogo Unicode completo: {}", catalog_len());
        assert!(list(Category::Recent).is_empty());
        for c in &Category::ALL[1..] {
            assert!(!list(*c).is_empty(), "{c:?}");
        }
        assert_eq!(list(Category::Smileys)[0].as_str(), "😀");
        assert!(list(Category::Flags).iter().any(|e| e.as_str() == "🇪🇸"));
        // Sin variantes de tono en las listas.
        assert!(list(Category::People).iter().all(|e| matches!(e.skin_tone(), None | Some(SkinTone::Default))));
        // Nada por encima de la versión ofrecida.
        assert!(lists().values().flatten().all(|e| e.emoji_version() <= MAX_VERSION));
    }

    #[test]
    fn todas_las_palabras_en_espanol_apuntan_a_emojis_reales() {
        for (e, w) in crate::emoji_es::KEYWORDS {
            assert!(emojis::get(e).is_some(), "{e} no está en el catálogo");
            assert_eq!(fold(w), *w, "palabras de {e} sin tildes ni mayúsculas");
        }
    }

    #[test]
    fn busqueda_en_espanol_ingles_y_por_grupo() {
        let first = |q: &str| search(q).first().map(|e| e.as_str());
        assert_eq!(first("corazón rojo"), Some("❤️"), "con tilde y en español");
        assert_eq!(first("pulgar"), Some("👍"));
        assert_eq!(first("GATO"), Some("🐱"));
        assert!(search("thumbs").iter().any(|e| e.as_str() == "👍"), "nombre inglés");
        assert!(search("joy").iter().any(|e| e.as_str() == "😂"), "atajo de GitHub");
        assert!(search("españa").iter().any(|e| e.as_str() == "🇪🇸"));
        let banderas = search("bandera");
        assert!(banderas.len() > 100 && banderas.iter().all(|e| e.group() == Group::Flags), "el grupo entero");
        assert!(search("   ").is_empty());
        assert!(search("zzqqxx").is_empty());
        // Todas las palabras deben casar.
        assert!(search("gato risa").iter().all(|e| ["😹"].contains(&e.as_str())));
    }

    #[test]
    fn tono_de_piel_solo_donde_aplica() {
        let hand = emojis::get("👍").unwrap();
        assert_eq!(apply(hand, Tone::Default), "👍");
        assert_eq!(apply(hand, Tone::Medium), "👍🏽");
        assert_eq!(apply(hand, Tone::Dark), "👍🏿");
        let cat = emojis::get("🐱").unwrap();
        assert_eq!(apply(cat, Tone::Dark), "🐱");
        assert_eq!(Tone::Light.sample(), "👋🏻");
    }

    #[test]
    fn recientes_por_fecha_y_frecuentes_por_veces() {
        let mut u = Usage::default();
        assert!(u.is_empty());
        u.record("😀", 10);
        u.record("😀", 11);
        u.record("😀", 12);
        u.record("🔥", 20);
        u.merge(vec![EmojiUse { emoji: "👍".into(), count: 2, last_used: Timestamp(5) }, EmojiUse { emoji: "🔥".into(), count: 1, last_used: Timestamp(3) }]);
        assert_eq!(u.recent(10), ["🔥", "😀", "👍"]);
        assert_eq!(u.frequent(10), ["😀", "🔥", "👍"], "empate a 2: el más reciente primero");
        assert_eq!(u.recent(1), ["🔥"]);
    }
}
