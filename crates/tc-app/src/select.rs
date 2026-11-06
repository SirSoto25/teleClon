//! Elección del backend según `TC_BACKEND` (por defecto, el mock). Con `telegram` hacen falta
//! `TC_API_ID`/`TC_API_HASH` (entorno o `.env`); si faltan o no son válidos se vuelve al mock con un
//! aviso que nombra las variables, nunca sus valores.

use tc_telegram::{ConfigError, TelegramConfig};

#[derive(Debug)]
pub enum Choice {
    /// Backend de pruebas; `notice` explica por qué si se pidió otro.
    Mock { notice: Option<String> },
    Telegram(TelegramConfig),
}

/// `backend` = valor de `TC_BACKEND`; `config` solo se evalúa si se pide la red real.
pub fn choose(backend: Option<&str>, config: impl FnOnce() -> Result<TelegramConfig, ConfigError>) -> Choice {
    match backend.map(str::trim) {
        None | Some("" | "mock") => Choice::Mock { notice: None },
        Some("telegram") => match config() {
            Ok(cfg) => Choice::Telegram(cfg),
            Err(e) => Choice::Mock { notice: Some(missing_credentials(&e)) },
        },
        Some(other) => Choice::Mock {
            notice: Some(format!("TC_BACKEND={other} no existe (usa «mock» o «telegram»). Se usa el backend de pruebas.")),
        },
    }
}

/// Mensaje para el usuario cuando faltan o no valen las credenciales (sin valores).
pub fn missing_credentials(e: &ConfigError) -> String {
    format!(
        "TC_BACKEND=telegram: {e}. Copia .env.example a .env y pon tu TC_API_ID y TC_API_HASH (de my.telegram.org). \
         Mientras tanto se usa el backend de pruebas."
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    const FAKE_HASH: &str = "0123456789abcdef0123456789abcdef";

    #[test]
    fn por_defecto_el_mock_sin_mirar_credenciales() {
        for v in [None, Some(""), Some("mock"), Some(" mock ")] {
            let c = choose(v, || panic!("no debe leer la configuración"));
            assert!(matches!(c, Choice::Mock { notice: None }), "{v:?}");
        }
    }

    #[test]
    fn telegram_con_credenciales() {
        let c = choose(Some("telegram"), || Ok(TelegramConfig::new(12345, FAKE_HASH.into())));
        assert!(matches!(c, Choice::Telegram(cfg) if cfg.api_id == 12345));
    }

    #[test]
    fn sin_credenciales_vuelve_al_mock_con_aviso_que_no_ensena_valores() {
        let lookup = |k: &str| (k == "TC_API_HASH").then(|| "secreto-no-hex".to_string());
        let c = choose(Some("telegram"), || TelegramConfig::from_lookup(lookup));
        let Choice::Mock { notice: Some(n) } = c else { panic!("vuelta al mock") };
        assert!(n.contains("TC_API_ID") && n.contains(".env"));
        let lookup = |k: &str| match k {
            "TC_API_ID" => Some("77".to_string()),
            "TC_API_HASH" => Some("secreto-no-hex".to_string()),
            _ => None,
        };
        let Choice::Mock { notice: Some(n) } = choose(Some("telegram"), || TelegramConfig::from_lookup(lookup)) else { panic!() };
        assert!(n.contains("TC_API_HASH"));
        assert!(!n.contains("secreto-no-hex") && !n.contains("77"));
    }

    #[test]
    fn valor_desconocido_avisa() {
        let Choice::Mock { notice: Some(n) } = choose(Some("matrix"), || panic!()) else { panic!() };
        assert!(n.contains("matrix"));
    }
}
