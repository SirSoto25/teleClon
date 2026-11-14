//! Nombres de archivo seguros para guardar lo que llega de la red (ADR-0011).
//!
//! `sanitize_file_name` convierte un nombre hostil (el `file_name` de un documento lo pone quien lo
//! envía) en **un solo componente** de ruta válido en Linux, Windows y macOS. `unique_path` elige en un
//! directorio un nombre libre con sufijo « (1)», « (2)»… sin sobrescribir nada.
use std::path::{Path, PathBuf};

/// Nombre que se usa cuando del original no queda nada.
pub const DEFAULT_FILE_NAME: &str = "archivo";

/// Tope en bytes UTF-8 del nombre saneado. Por debajo de los 255 de ext4/NTFS para que quepan el
/// sufijo de `unique_path` (« (99999)») y el `.part` de la descarga en curso.
pub const MAX_FILE_NAME_BYTES: usize = 200;

/// Extensión más larga (sin el punto) que se conserva al recortar un nombre largo.
const MAX_EXT_BYTES: usize = 16;

/// Intentos de `unique_path` antes de rendirse (devuelve el último candidato).
const MAX_UNIQUE_TRIES: u32 = 99_999;

/// Caracteres prohibidos en Windows (más los separadores de ruta de todos los SO).
fn forbidden(c: char) -> bool {
    matches!(c, '/' | '\\' | '<' | '>' | ':' | '"' | '|' | '?' | '*') || c.is_control()
}

/// Controles de dirección bidi: permiten disfrazar la extensión (`fdp.exe` mostrado como `exe.pdf`).
fn bidi_control(c: char) -> bool {
    matches!(c, '\u{200E}' | '\u{200F}' | '\u{202A}'..='\u{202E}' | '\u{2066}'..='\u{2069}')
}

/// Nombres de dispositivo reservados de Windows (también con extensión: `con.txt`).
fn is_reserved(name: &str) -> bool {
    let stem = name.split('.').next().unwrap_or("").trim_end();
    let up = stem.to_ascii_uppercase();
    match up.as_str() {
        "CON" | "PRN" | "AUX" | "NUL" => true,
        _ => {
            let (head, tail) = (up.get(..3), up.get(3..));
            matches!(head, Some("COM" | "LPT"))
                && matches!(tail, Some(t) if t.chars().count() == 1 && t.chars().all(|c| c.is_ascii_digit() || matches!(c, '¹' | '²' | '³')))
        }
    }
}

/// Parte un nombre en (raíz, extensión con su punto). Sin extensión si no hay punto, si el punto es el
/// primer carácter o si la extensión es más larga que `MAX_EXT_BYTES`.
fn split_ext(name: &str) -> (&str, &str) {
    match name.rfind('.') {
        Some(p) if p > 0 && name.len() - p - 1 <= MAX_EXT_BYTES => name.split_at(p),
        _ => (name, ""),
    }
}

fn trim_end(s: &str) -> &str {
    s.trim_end_matches(|c: char| c.is_whitespace() || c == '.')
}

/// Recorta a `max` bytes conservando la extensión (si es corta) y sin partir un carácter.
fn truncate(name: &str, max: usize) -> String {
    if name.len() <= max {
        return name.to_string();
    }
    let (stem, ext) = split_ext(name);
    let mut keep = max.saturating_sub(ext.len());
    while !stem.is_char_boundary(keep) {
        keep -= 1;
    }
    format!("{}{ext}", trim_end(&stem[..keep]))
}

/// Nombre de archivo seguro a partir de uno cualquiera:
/// - sin separadores (`/`, `\`), sin `<>:"|?*` ni caracteres de control (pasan a `_`) y sin controles
///   bidi (se quitan);
/// - sin `..` (los puntos seguidos se juntan en uno) y sin puntos ni espacios al principio o al final
///   (nada de archivos ocultos, ni `.`/`..`, ni nombres que Windows recorta);
/// - los nombres reservados de Windows (`CON`, `nul.txt`, `COM1`…) llevan `_` delante;
/// - como mucho `MAX_FILE_NAME_BYTES` bytes, conservando la extensión;
/// - si no queda nada, `DEFAULT_FILE_NAME`.
///
/// Es idempotente y el resultado es siempre un único componente normal de ruta.
pub fn sanitize_file_name(name: &str) -> String {
    let mut mapped = String::with_capacity(name.len());
    let mut prev_dot = false;
    for c in name.chars() {
        if bidi_control(c) {
            continue;
        }
        let c = if forbidden(c) { '_' } else { c };
        if c == '.' && prev_dot {
            continue;
        }
        prev_dot = c == '.';
        mapped.push(c);
    }
    let trimmed = mapped.trim_matches(|c: char| c.is_whitespace() || c == '.');
    let mut out = truncate(trimmed, MAX_FILE_NAME_BYTES);
    if is_reserved(&out) {
        out = truncate(&format!("_{out}"), MAX_FILE_NAME_BYTES);
    }
    if out.is_empty() { DEFAULT_FILE_NAME.to_string() } else { out }
}

/// Ruta libre para guardar `name` (saneado) en `dir`: `dir/name` si no existe; si no, `dir/raíz (1).ext`,
/// `dir/raíz (2).ext`… Comprueba con el sistema de archivos (solo lectura). Entre la comprobación y la
/// creación otro proceso podría ocupar el nombre: quien escribe debe crear el archivo sin sobrescribir
/// (o aceptar el renombrado atómico de `download_media_to`, que sí reemplaza).
pub fn unique_path(dir: &Path, name: &str) -> PathBuf {
    unique_path_with(dir, name, |p| p.exists())
}

/// Como `unique_path`, pero `exists` decide qué rutas están ocupadas (sin tocar el disco; para pruebas
/// y para quien lleve su propia lista de nombres reservados).
pub fn unique_path_with(dir: &Path, name: &str, mut exists: impl FnMut(&Path) -> bool) -> PathBuf {
    let name = sanitize_file_name(name);
    let first = dir.join(&name);
    if !exists(&first) {
        return first;
    }
    let (stem, ext) = split_ext(&name);
    let mut cand = first;
    for n in 1..=MAX_UNIQUE_TRIES {
        cand = dir.join(format!("{stem} ({n}){ext}"));
        if !exists(&cand) {
            break;
        }
    }
    cand
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;
    use std::path::Component;

    /// Generador determinista (xorshift) para las pruebas de propiedades: sin dependencias.
    struct Rng(u64);
    impl Rng {
        fn next(&mut self) -> u64 {
            self.0 ^= self.0 << 13;
            self.0 ^= self.0 >> 7;
            self.0 ^= self.0 << 17;
            self.0
        }
        fn below(&mut self, n: usize) -> usize {
            (self.next() % n as u64) as usize
        }
    }

    /// Piezas hostiles y normales con las que se arman nombres al azar.
    const PIECES: &[&str] = &[
        "a", "Z", "informe", "ñandú", "😀", "文件", " ", ".", "..", "...", "/", "\\", ":", "*", "?", "\"", "<", ">", "|", "\0", "\n",
        "\t", "\u{7f}", "\u{202E}", "\u{2066}", "\u{200F}", "CON", "nul", "Com1", "LPT9", "aux", "COM¹", ".txt", ".pdf", ".tar.gz", "../",
        "..\\", "C:", "~", "-", "_", "(1)",
    ];

    fn random_name(rng: &mut Rng) -> String {
        let mut s = String::new();
        let parts = rng.below(12);
        for _ in 0..parts {
            s.push_str(PIECES[rng.below(PIECES.len())]);
        }
        // De vez en cuando, nombres larguísimos.
        if rng.below(8) == 0 {
            let piece = PIECES[rng.below(PIECES.len())];
            s.push_str(&piece.repeat(1 + rng.below(120)));
        }
        s
    }

    fn assert_safe(input: &str, s: &str) {
        let ctx = format!("{input:?} → {s:?}");
        assert!(!s.is_empty(), "{ctx}");
        assert!(s.len() <= MAX_FILE_NAME_BYTES, "{ctx}");
        assert!(!s.chars().any(forbidden), "{ctx}");
        assert!(!s.chars().any(bidi_control), "{ctx}");
        assert!(!s.contains(".."), "{ctx}");
        assert!(!s.starts_with('.') && !s.starts_with(char::is_whitespace), "{ctx}");
        assert!(!s.ends_with('.') && !s.ends_with(char::is_whitespace), "{ctx}");
        assert!(!is_reserved(s), "{ctx}");
        let comps: Vec<_> = Path::new(s).components().collect();
        assert_eq!(comps.len(), 1, "{ctx}");
        assert!(matches!(comps[0], Component::Normal(_)), "{ctx}");
        assert_eq!(sanitize_file_name(s), s, "no idempotente: {ctx}");
    }

    #[test]
    fn property_sanitized_names_are_safe_and_stable() {
        let mut rng = Rng(0x9E37_79B9_7F4A_7C15);
        for _ in 0..20_000 {
            let input = random_name(&mut rng);
            let s = sanitize_file_name(&input);
            assert_safe(&input, &s);
        }
    }

    #[test]
    fn examples() {
        let cases = [
            ("informe.pdf", "informe.pdf"),
            ("foto de Ana 😀.jpg", "foto de Ana 😀.jpg"),
            ("", DEFAULT_FILE_NAME),
            ("   ", DEFAULT_FILE_NAME),
            (".", DEFAULT_FILE_NAME),
            ("..", DEFAULT_FILE_NAME),
            ("../../etc/passwd", "_._etc_passwd"),
            ("..\\..\\Windows\\win.ini", "_._Windows_win.ini"),
            ("a/b\\c", "a_b_c"),
            ("CON", "_CON"),
            ("con.txt", "_con.txt"),
            ("Com1.log", "_Com1.log"),
            ("LPT9", "_LPT9"),
            ("CONSOLE.txt", "CONSOLE.txt"),
            ("COM10", "COM10"),
            (".bashrc", "bashrc"),
            ("nota.txt.", "nota.txt"),
            ("dos..puntos", "dos.puntos"),
            ("fdp\u{202E}fdp.exe", "fdpfdp.exe"),
            ("a\0b\nc", "a_b_c"),
            ("¿qué?: «sí»|*", "¿qué__ «sí»__"),
        ];
        for (input, want) in cases {
            assert_eq!(sanitize_file_name(input), want, "{input:?}");
        }
    }

    #[test]
    fn long_names_keep_their_extension_and_char_boundaries() {
        let long = format!("{}.pdf", "ñ".repeat(300));
        let s = sanitize_file_name(&long);
        assert!(s.len() <= MAX_FILE_NAME_BYTES);
        assert!(s.ends_with(".pdf"));
        assert!(s.starts_with('ñ'));
        // Extensión absurda: se recorta como parte del nombre.
        let s = sanitize_file_name(&format!("a.{}", "e".repeat(400)));
        assert_eq!(s.len(), MAX_FILE_NAME_BYTES);
        // Reservado al recortar: «CON» seguido de espacios que el recorte deja al final.
        let s = sanitize_file_name(&format!("CON{}x", " ".repeat(300)));
        assert_eq!(s, "_CON");
    }

    #[test]
    fn unique_path_adds_numbered_suffixes() {
        let dir = Path::new("/descargas");
        let mut taken: HashSet<PathBuf> = HashSet::new();
        let mut names = Vec::new();
        for _ in 0..5 {
            let p = unique_path_with(dir, "informe.pdf", |p| taken.contains(p));
            taken.insert(p.clone());
            names.push(p.file_name().unwrap().to_string_lossy().into_owned());
        }
        assert_eq!(names, ["informe.pdf", "informe (1).pdf", "informe (2).pdf", "informe (3).pdf", "informe (4).pdf"]);
        let p = unique_path_with(dir, "LEEME", |p| p == dir.join("LEEME"));
        assert_eq!(p, dir.join("LEEME (1)"));
        // El nombre se sanea antes: nunca sale del directorio.
        assert_eq!(unique_path_with(dir, "../../x", |_| false), dir.join("_._x"));
    }

    #[test]
    fn property_unique_path_never_reuses_and_stays_in_dir() {
        let mut rng = Rng(42);
        let dir = Path::new("/d");
        for _ in 0..300 {
            let name = random_name(&mut rng);
            let mut taken: HashSet<PathBuf> = HashSet::new();
            for _ in 0..(1 + rng.below(6)) {
                let p = unique_path_with(dir, &name, |p| taken.contains(p));
                assert_eq!(p.parent(), Some(dir), "{name:?}");
                let file = p.file_name().unwrap().to_str().unwrap();
                assert!(!taken.contains(&p), "{name:?} repetido");
                assert!(file.len() <= MAX_FILE_NAME_BYTES + " (99999)".len());
                assert!(!file.contains('/') && !file.contains('\\'));
                taken.insert(p);
            }
        }
    }

    #[test]
    fn unique_path_on_the_real_file_system() {
        let dir = std::env::temp_dir().join(format!("tc-core-unique-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let a = unique_path(&dir, "nota.txt");
        assert_eq!(a, dir.join("nota.txt"));
        std::fs::write(&a, b"1").unwrap();
        let b = unique_path(&dir, "nota.txt");
        assert_eq!(b, dir.join("nota (1).txt"));
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
