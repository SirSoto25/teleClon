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

/// Plegado para búsqueda local: minúsculas y sin diacríticos del español/catalán/euskera.
pub fn fold(s: &str) -> String {
    s.chars()
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
