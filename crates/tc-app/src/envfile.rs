//! Lector mínimo de archivos `.env` (sin dependencias). Solo produce un mapa clave → valor
//! para consultarlo junto al entorno del proceso: NO modifica el entorno (`set_var` es `unsafe` en
//! la edición 2024 y el binario prohíbe `unsafe`) y NUNCA escribe valores en el registro.
//!
//! Formato admitido: `CLAVE=valor` por línea, `export CLAVE=valor`, comentarios con `#` al
//! principio de línea, líneas vacías, valores entre comillas simples o dobles (sin escapes) y
//! comentario final ` # …` en valores sin comillas.

use std::collections::HashMap;
use std::path::Path;

/// Variables leídas de un `.env`.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct EnvFile {
    vars: HashMap<String, String>,
}

impl EnvFile {
    /// Interpreta el contenido de un `.env`. Las líneas que no tienen forma de asignación se
    /// ignoran (y se cuentan, para avisar sin enseñar su contenido).
    pub fn parse(content: &str) -> (Self, usize) {
        let mut vars = HashMap::new();
        let mut bad = 0;
        for raw in content.lines() {
            let line = raw.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            let line = line.strip_prefix("export ").map_or(line, str::trim_start);
            let Some((k, v)) = line.split_once('=') else {
                bad += 1;
                continue;
            };
            let k = k.trim();
            if k.is_empty() || !k.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') || k.starts_with(|c: char| c.is_ascii_digit()) {
                bad += 1;
                continue;
            }
            vars.insert(k.to_string(), unquote(v.trim()));
        }
        (Self { vars }, bad)
    }

    /// Lee el archivo si existe. `Ok(None)` si no existe; el error no incluye el contenido.
    pub fn load(path: &Path) -> std::io::Result<Option<(Self, usize)>> {
        match std::fs::read_to_string(path) {
            Ok(s) => Ok(Some(Self::parse(&s))),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(e) => Err(e),
        }
    }

    pub fn get(&self, key: &str) -> Option<&str> {
        self.vars.get(key).map(String::as_str)
    }

    pub fn len(&self) -> usize {
        self.vars.len()
    }
}

fn unquote(v: &str) -> String {
    for q in ['"', '\''] {
        if let Some(inner) = v.strip_prefix(q)
            && let Some(end) = inner.find(q)
        {
            return inner[..end].to_string();
        }
    }
    // Sin comillas: un ` #` abre comentario.
    match v.find(" #") {
        Some(i) => v[..i].trim_end().to_string(),
        None => v.to_string(),
    }
}

/// Busca primero en el entorno del proceso (manda sobre el archivo) y luego en el `.env`.
pub fn lookup<'a>(file: &'a EnvFile, env: impl Fn(&str) -> Option<String> + 'a) -> impl Fn(&str) -> Option<String> + 'a {
    move |k| env(k).filter(|v| !v.trim().is_empty()).or_else(|| file.get(k).map(str::to_string))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn asignaciones_comillas_y_comentarios() {
        let src = "# comentario\n\nTC_A=1\nexport TC_B = dos \nTC_C=\"con # almohadilla\"\nTC_D='simple'\nTC_E=valor # nota\nTC_F=\nsin_igual\n9MAL=x\nTC-G=x\n";
        let (f, bad) = EnvFile::parse(src);
        assert_eq!(f.get("TC_A"), Some("1"));
        assert_eq!(f.get("TC_B"), Some("dos"));
        assert_eq!(f.get("TC_C"), Some("con # almohadilla"));
        assert_eq!(f.get("TC_D"), Some("simple"));
        assert_eq!(f.get("TC_E"), Some("valor"));
        assert_eq!(f.get("TC_F"), Some(""));
        assert_eq!(f.len(), 6);
        assert_eq!(bad, 3);
    }

    #[test]
    fn el_entorno_manda_sobre_el_archivo_salvo_si_esta_vacio() {
        let (f, _) = EnvFile::parse("TC_X=archivo\nTC_Y=archivo\n");
        let env = |k: &str| match k {
            "TC_X" => Some("proceso".to_string()),
            "TC_Y" => Some("  ".to_string()),
            _ => None,
        };
        let get = lookup(&f, env);
        assert_eq!(get("TC_X").as_deref(), Some("proceso"));
        assert_eq!(get("TC_Y").as_deref(), Some("archivo"));
        assert_eq!(get("TC_Z"), None);
    }

    #[test]
    fn archivo_inexistente_no_es_error() {
        assert_eq!(EnvFile::load(Path::new("/nonexistent/teleclon/.env")).unwrap(), None);
    }
}
