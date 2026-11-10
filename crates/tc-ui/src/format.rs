//! Textos derivados: horas en zona local (jiff, con DST correcto), estados de usuario, búsqueda
//! sin acentos, vista previa de mensajes y huella de clave. Todo el texto es propio.

use jiff::Timestamp as JTs;
use jiff::civil::Date;
use jiff::tz::TimeZone;
use tc_core::{Message, MediaKind, Timestamp, UserStatus};

fn zoned(ts: Timestamp, tz: &TimeZone) -> Option<jiff::Zoned> {
    JTs::from_second(ts.0).ok().map(|t| t.to_zoned(tz.clone()))
}

/// Fecha civil local (para separadores y agrupación por día).
pub fn local_date(ts: Timestamp, tz: &TimeZone) -> Date {
    zoned(ts, tz).map_or(Date::constant(1970, 1, 1), |z| z.date())
}

/// Número de día local (clave compacta de agrupación).
pub fn day_key(ts: Timestamp, tz: &TimeZone) -> i32 {
    let d = local_date(ts, tz);
    i32::from(d.year()) * 400 + i32::from(d.month()) * 32 + i32::from(d.day())
}

/// «14:02».
pub fn hhmm(ts: Timestamp, tz: &TimeZone) -> String {
    zoned(ts, tz).map_or_else(String::new, |z| format!("{:02}:{:02}", z.hour(), z.minute()))
}

const WEEKDAYS: [&str; 7] = ["lun", "mar", "mié", "jue", "vie", "sáb", "dom"];
const MONTHS: [&str; 12] = [
    "enero", "febrero", "marzo", "abril", "mayo", "junio", "julio", "agosto", "septiembre", "octubre", "noviembre",
    "diciembre",
];

/// Hora en la lista de chats: hoy «14:02», última semana «mié», antes «03/09/26».
pub fn list_time(ts: Timestamp, now: Timestamp, tz: &TimeZone) -> String {
    let (d, today) = (local_date(ts, tz), local_date(now, tz));
    let days = (today - d).get_days();
    if d == today {
        hhmm(ts, tz)
    } else if (1..7).contains(&days) {
        WEEKDAYS[d.weekday().to_monday_zero_offset() as usize].to_string()
    } else {
        format!("{:02}/{:02}/{:02}", d.day(), d.month(), d.year() % 100)
    }
}

/// Separador de fecha en la conversación: «Hoy», «Ayer», «12 de marzo», «12 de marzo de 2025».
pub fn date_separator(ts: Timestamp, now: Timestamp, tz: &TimeZone) -> String {
    let (d, today) = (local_date(ts, tz), local_date(now, tz));
    let days = (today - d).get_days();
    let month = MONTHS[(d.month() - 1) as usize];
    match days {
        0 => "Hoy".into(),
        1 => "Ayer".into(),
        _ if d.year() == today.year() => format!("{} de {month}", d.day()),
        _ => format!("{} de {month} de {}", d.day(), d.year()),
    }
}

/// Estado bajo el nombre en la cabecera.
pub fn user_status(status: UserStatus, now: Timestamp, tz: &TimeZone) -> String {
    match status {
        UserStatus::Online => "en línea".into(),
        UserStatus::Recently => "activo hace poco".into(),
        UserStatus::Offline => "sin actividad reciente".into(),
        UserStatus::LastSeen(t) => {
            let mins = (now.0 - t.0).max(0) / 60;
            if mins < 1 {
                "activo hace un momento".into()
            } else if mins < 60 {
                format!("activo hace {mins} min")
            } else if local_date(t, tz) == local_date(now, tz) {
                format!("activo hoy a las {}", hhmm(t, tz))
            } else {
                format!("activo el {}", list_time(t, now, tz))
            }
        }
    }
}

/// Letra «de estilo» Unicode → letra normal: alfanuméricos matemáticos (U+1D400–U+1D7FF:
/// 𝒞𝓇𝒾𝓈𝓉𝒾𝓃𝒶, 𝐁𝐨𝐥𝐝, 𝕯…), los símbolos tipo letra que rellenan sus huecos (ℬ, ℯ, ℝ…) y las formas
/// de ancho completo (Ａ, １). Es el subconjunto de NFKC que se usa para «decorar» nombres; ninguna
/// fuente de UI lo cubre y el respaldo de fuentes los pinta con otra tipografía a saltos.
/// `None` si el carácter no es de esos.
fn plain_char(c: char) -> Option<char> {
    let cp = c as u32;
    let from = |base: u32| char::from_u32(base);
    match cp {
        // 13 alfabetos latinos de 52 letras (A–Z, a–z); los huecos no asignados nunca llegan.
        0x1D400..=0x1D6A3 => {
            let i = (cp - 0x1D400) % 52;
            from(if i < 26 { 'A' as u32 + i } else { 'a' as u32 + i - 26 })
        }
        0x1D6A4 => Some('i'),
        0x1D6A5 => Some('j'),
        // 5 alfabetos griegos de 58: 25 mayúsculas (ϴ en el hueco 17), ∇, 25 minúsculas, ∂ y 6 variantes.
        0x1D6A8..=0x1D7C9 => {
            let i = (cp - 0x1D6A8) % 58;
            match i {
                17 => Some('ϴ'),
                0..=24 => from(0x391 + i),
                25 => Some('∇'),
                26..=50 => from(0x3B1 + i - 26),
                51 => Some('∂'),
                _ => ['ϵ', 'ϑ', 'ϰ', 'ϕ', 'ϱ', 'ϖ'].get((i - 52) as usize).copied(),
            }
        }
        0x1D7CA => Some('Ϝ'),
        0x1D7CB => Some('ϝ'),
        // 5 juegos de cifras.
        0x1D7CE..=0x1D7FF => from('0' as u32 + (cp - 0x1D7CE) % 10),
        // Ancho completo (！ … ～).
        0xFF01..=0xFF5E => from(cp - 0xFF01 + 0x21),
        _ => match c {
            'ℂ' => Some('C'),
            'ℊ' => Some('g'),
            'ℋ' | 'ℌ' | 'ℍ' => Some('H'),
            'ℎ' => Some('h'),
            'ℐ' | 'ℑ' => Some('I'),
            'ℒ' => Some('L'),
            'ℕ' => Some('N'),
            'ℙ' => Some('P'),
            'ℚ' => Some('Q'),
            'ℛ' | 'ℜ' | 'ℝ' => Some('R'),
            'ℤ' | 'ℨ' => Some('Z'),
            'ℬ' => Some('B'),
            'ℭ' => Some('C'),
            'ℯ' => Some('e'),
            'ℰ' => Some('E'),
            'ℱ' => Some('F'),
            'ℳ' => Some('M'),
            'ℴ' => Some('o'),
            _ => None,
        },
    }
}

/// Nombres y vistas previas en la tipografía de la UI: cambia las letras «de estilo» por las
/// normales (ver [`plain_char`]). Solo para nombres, vistas previas y citas; el cuerpo del mensaje
/// se pinta tal cual.
pub fn plain_letters(s: &str) -> std::borrow::Cow<'_, str> {
    if s.chars().all(|c| plain_char(c).is_none()) {
        return std::borrow::Cow::Borrowed(s);
    }
    std::borrow::Cow::Owned(s.chars().map(|c| plain_char(c).unwrap_or(c)).collect())
}

/// Plegado para búsqueda local: minúsculas y sin diacríticos del español/catalán/euskera (y sin
/// letras «de estilo», para que «cris» encuentre a 𝒞𝓇𝒾𝓈𝓉𝒾𝓃𝒶).
pub fn fold(s: &str) -> String {
    s.chars()
        .map(|c| plain_char(c).unwrap_or(c))
        .flat_map(char::to_lowercase)
        .map(|c| match c {
            'á' | 'à' | 'â' | 'ä' => 'a',
            'é' | 'è' | 'ê' | 'ë' => 'e',
            'í' | 'ì' | 'î' | 'ï' => 'i',
            'ó' | 'ò' | 'ô' | 'ö' => 'o',
            'ú' | 'ù' | 'û' | 'ü' => 'u',
            'ñ' => 'n',
            'ç' => 'c',
            other => other,
        })
        .collect()
}

/// ¿Coincide `title` con la consulta ya plegada? Vacío coincide con todo.
pub fn matches(title: &str, folded_query: &str) -> bool {
    folded_query.is_empty() || fold(title).contains(folded_query)
}

/// Resumen de una línea del contenido de un mensaje (lista de chats, citas de respuesta).
pub fn preview(m: &Message) -> String {
    let body = m.text.split('\n').next().unwrap_or_default().trim();
    let media = m.media.as_ref().map(|md| match md.kind {
        MediaKind::Photo => "Foto",
        MediaKind::Video => "Vídeo",
        MediaKind::Gif => "GIF",
        MediaKind::Voice => "Nota de voz",
        MediaKind::File => "Archivo",
        MediaKind::Audio => "Audio",
        MediaKind::VideoNote => "Videomensaje",
    });
    match (m.sticker.as_ref(), media) {
        (Some(s), _) => format!("{} Sticker", s.emoji),
        (None, Some(kind)) if body.is_empty() => kind.to_string(),
        (None, Some(kind)) => format!("{kind} · {body}"),
        (None, None) => body.to_string(),
    }
}

/// Recorta a `max` caracteres con elipsis (la lista no tiene elipsis nativa en iced 0.14).
pub fn ellipsize(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        s.to_string()
    } else {
        let mut out: String = s.chars().take(max.saturating_sub(1)).collect();
        out.push('…');
        out
    }
}

/// Iniciales para el avatar sin foto («Lucía Barrientos» → «LB»).
pub fn initials(name: &str) -> String {
    let name = plain_letters(name);
    let mut it = name.split_whitespace().filter(|w| w.chars().next().is_some_and(char::is_alphanumeric));
    let a = it.next().and_then(|w| w.chars().next());
    let b = it.next().and_then(|w| w.chars().next());
    [a, b].into_iter().flatten().flat_map(char::to_uppercase).collect()
}

/// «1 min», «1 h», «30 s» (temporizador de autodestrucción).
pub fn ttl(secs: u32) -> String {
    match secs {
        0..=59 => format!("{secs} s"),
        60..=3599 => format!("{} min", secs / 60),
        3600..=86_399 => format!("{} h", secs / 3600),
        _ => format!("{} d", secs / 86_400),
    }
}

/// Huella de clave en formato propio (§8.9): 16 celdas con índice de color 0..8 (3 bits cada
/// una) y 4 grupos de 5 cifras, derivados de los bytes de la huella hexadecimal.
pub fn key_grid(fingerprint: &str) -> ([u8; 16], [String; 4]) {
    let bytes: Vec<u8> = fingerprint
        .chars()
        .filter_map(|c| c.to_digit(16))
        .collect::<Vec<u32>>()
        .chunks(2)
        .map(|p| (p[0] * 16 + p.get(1).copied().unwrap_or(0)) as u8)
        .collect();
    let mut bits: u128 = 0;
    for (i, b) in bytes.iter().take(16).enumerate() {
        bits |= u128::from(*b) << (8 * i);
    }
    let mut cells = [0u8; 16];
    for (i, cell) in cells.iter_mut().enumerate() {
        *cell = ((bits >> (3 * i)) & 0b111) as u8;
    }
    let group = |i: usize| {
        let w = u32::from(bytes.get(2 * i).copied().unwrap_or(0)) << 8 | u32::from(bytes.get(2 * i + 1).copied().unwrap_or(0));
        let w2 = u32::from(bytes.get(8 + 2 * i).copied().unwrap_or(0));
        format!("{:05}", (w * 256 + w2) % 100_000)
    };
    (cells, [group(0), group(1), group(2), group(3)])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plegado_sin_acentos() {
        assert_eq!(fold("Íñigo Peñalver"), "inigo penalver");
        assert!(matches("Club de lectura «Tinta y Sal»", &fold("LECTURA")));
        assert!(matches("Joaquín", &fold("joaquin")));
        assert!(!matches("Mateo", "zz"));
        assert!(matches("lo que sea", ""));
    }

    #[test]
    fn letras_de_estilo_a_normales() {
        assert_eq!(plain_letters("𝒞𝓇𝒾𝓈𝓉𝒾𝓃𝒶"), "Cristina");
        assert_eq!(plain_letters("𝐁𝐨𝐥𝐝 𝕯𝖆𝖗𝖎𝖔 ℬℯ𝓁𝓁𝒶 Ａｎａ １２"), "Bold Dario Bella Ana 12");
        assert_eq!(plain_letters("𝛂𝛃 𝟗"), "αβ 9");
        assert!(matches!(plain_letters("Lucía 👍 ½"), std::borrow::Cow::Borrowed("Lucía 👍 ½")));
        assert_eq!(initials("𝒞𝓇𝒾𝓈𝓉𝒾𝓃𝒶 Iglesias"), "CI");
        assert!(matches("𝒞𝓇𝒾𝓈𝓉𝒾𝓃𝒶", &fold("cris")));
    }

    #[test]
    fn iniciales_y_elipsis() {
        assert_eq!(initials("Lucía Barrientos"), "LB");
        assert_eq!(initials("Club de lectura «Tinta y Sal»"), "CD");
        assert_eq!(initials("Tú"), "T");
        assert_eq!(ellipsize("abcdef", 4), "abc…");
        assert_eq!(ellipsize("ñu", 4), "ñu");
    }

    #[test]
    fn horas_y_separadores_en_utc() {
        let tz = TimeZone::UTC;
        let now = Timestamp(1_762_430_400); // 2025-11-06 12:00 UTC (jueves)
        assert_eq!(hhmm(now, &tz), "12:00");
        assert_eq!(list_time(now, now, &tz), "12:00");
        assert_eq!(list_time(Timestamp(now.0 - 86_400), now, &tz), "mié");
        assert_eq!(list_time(Timestamp(now.0 - 30 * 86_400), now, &tz), "07/10/25");
        assert_eq!(date_separator(now, now, &tz), "Hoy");
        assert_eq!(date_separator(Timestamp(now.0 - 86_400), now, &tz), "Ayer");
        assert_eq!(date_separator(Timestamp(now.0 - 30 * 86_400), now, &tz), "7 de octubre");
        assert_ne!(day_key(now, &tz), day_key(Timestamp(now.0 - 86_400), &tz));
        assert_eq!(user_status(UserStatus::LastSeen(Timestamp(now.0 - 600)), now, &tz), "activo hace 10 min");
    }

    #[test]
    fn huella_determinista_y_en_rango() {
        let fp = "a1b2 c3d4 e5f6 0718 293a 4b5c 6d7e 8f90";
        let (cells, groups) = key_grid(fp);
        assert_eq!(key_grid(fp).0, cells);
        assert!(cells.iter().all(|c| *c < 8));
        assert!(groups.iter().all(|g| g.len() == 5 && g.chars().all(|c| c.is_ascii_digit())));
        assert_ne!(key_grid("0000 0000 0000 0000 0000 0000 0000 0001").0, cells);
        assert_eq!(ttl(60), "1 min");
    }
}
