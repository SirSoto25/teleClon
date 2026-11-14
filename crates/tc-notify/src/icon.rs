//! Utilidades comunes a los backends: icono a ruta y escape de texto.

use std::{
    hash::{DefaultHasher, Hash, Hasher},
    path::PathBuf,
};

use crate::Icon;

/// Ruta en disco del icono. Los bytes se escriben una vez en `<tmp>/parlo-notify/<hash>.<ext>`
/// (el nombre depende del contenido, así el mismo avatar no se reescribe). `None` si no se pudo
/// escribir: el aviso sale sin imagen.
pub(crate) fn icon_path(icon: &Icon) -> Option<PathBuf> {
    match icon {
        Icon::Path(p) => Some(p.clone()),
        Icon::Bytes(bytes) => {
            let mut h = DefaultHasher::new();
            bytes.hash(&mut h);
            let dir = std::env::temp_dir().join("parlo-notify");
            let path = dir.join(format!("{:016x}.{}", h.finish(), extension(bytes)));
            if path.exists() {
                return Some(path);
            }
            let tmp = dir.join(format!("{:016x}.part", h.finish()));
            let res = std::fs::create_dir_all(&dir)
                .and_then(|()| std::fs::write(&tmp, bytes))
                .and_then(|()| std::fs::rename(&tmp, &path));
            match res {
                Ok(()) => Some(path),
                Err(e) => {
                    tracing::debug!("icono de aviso sin escribir: {e}");
                    None
                }
            }
        }
    }
}

fn extension(bytes: &[u8]) -> &'static str {
    if bytes.starts_with(&[0xFF, 0xD8, 0xFF]) {
        "jpg"
    } else {
        "png"
    }
}

/// Escapa `&`, `<`, `>`, `"` y `'` (cuerpo con marcado en D-Bus y XML de los toasts).
#[cfg_attr(target_os = "macos", allow(dead_code))]
pub(crate) fn escape_xml(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&apos;"),
            c => out.push(c),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn escapa_marcado() {
        assert_eq!(
            escape_xml(r#"<b>a & 'b' "c"</b>"#),
            "&lt;b&gt;a &amp; &apos;b&apos; &quot;c&quot;&lt;/b&gt;"
        );
        assert_eq!(escape_xml("ñandú 🙂"), "ñandú 🙂");
    }

    #[test]
    fn ruta_tal_cual() {
        let p = PathBuf::from("/x/y.png");
        assert_eq!(icon_path(&Icon::Path(p.clone())), Some(p));
    }

    #[test]
    fn bytes_a_cache_estable() {
        let png = b"\x89PNG\r\n\x1a\nprueba-tc-notify".to_vec();
        let a = icon_path(&Icon::Bytes(png.clone())).unwrap();
        let b = icon_path(&Icon::Bytes(png.clone())).unwrap();
        assert_eq!(a, b);
        assert_eq!(a.extension().unwrap(), "png");
        assert_eq!(std::fs::read(&a).unwrap(), png);
        let jpg = icon_path(&Icon::Bytes(vec![0xFF, 0xD8, 0xFF, 0xE0, 1, 2, 3])).unwrap();
        assert_eq!(jpg.extension().unwrap(), "jpg");
    }
}
