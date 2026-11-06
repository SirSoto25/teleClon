//! Configuración de la conexión. `api_id`/`api_hash` se leen SOLO de variables de entorno
//! (`TC_API_ID`, `TC_API_HASH`); la app las habrá cargado antes desde su `.env`.
//! Ni el `Debug` ni los errores muestran nunca los valores.
use std::fmt;
use std::time::Duration;

pub const ENV_API_ID: &str = "TC_API_ID";
pub const ENV_API_HASH: &str = "TC_API_HASH";

/// `api_hash` envuelto para que no se imprima por accidente.
#[derive(Clone, PartialEq, Eq)]
pub struct ApiHash(String);

impl ApiHash {
    pub fn expose(&self) -> &str {
        &self.0
    }
}

impl fmt::Debug for ApiHash {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("ApiHash(<oculto>)")
    }
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ConfigError {
    #[error("falta la variable de entorno {0}")]
    Missing(&'static str),
    #[error("la variable de entorno {0} no tiene un formato válido")]
    Invalid(&'static str),
}

#[derive(Debug, Clone)]
pub struct TelegramConfig {
    pub api_id: i32,
    pub api_hash: ApiHash,
    /// Esperas de `FLOOD_WAIT` de hasta este tiempo se duermen y reintentan solas;
    /// las mayores se devuelven como `BackendError::FloodWait` para que la UI muestre la cuenta atrás.
    pub flood_sleep_threshold: Duration,
    /// Reintentos ante cortes de red antes de devolver `BackendError::Network`.
    pub network_retries: u32,
    /// Primera espera entre reintentos de red (se duplica en cada intento, con tope de 30 s).
    pub network_backoff: Duration,
    /// Versión de la app que se anuncia al servidor en `initConnection`.
    pub app_version: String,
}

impl TelegramConfig {
    /// Lee `TC_API_ID` y `TC_API_HASH` del entorno del proceso.
    pub fn from_env() -> Result<Self, ConfigError> {
        Self::from_lookup(|k| std::env::var(k).ok())
    }

    /// Igual que `from_env` pero con una función de búsqueda (para pruebas).
    pub fn from_lookup(get: impl Fn(&str) -> Option<String>) -> Result<Self, ConfigError> {
        let id = get(ENV_API_ID).map(|v| v.trim().to_string()).filter(|v| !v.is_empty()).ok_or(ConfigError::Missing(ENV_API_ID))?;
        let api_id: i32 = id.parse().ok().filter(|v: &i32| *v > 0).ok_or(ConfigError::Invalid(ENV_API_ID))?;
        let hash = get(ENV_API_HASH).map(|v| v.trim().to_string()).filter(|v| !v.is_empty()).ok_or(ConfigError::Missing(ENV_API_HASH))?;
        // El api_hash es una cadena hexadecimal de 32 caracteres.
        if hash.len() != 32 || !hash.chars().all(|c| c.is_ascii_hexdigit()) {
            return Err(ConfigError::Invalid(ENV_API_HASH));
        }
        Ok(Self::new(api_id, hash))
    }

    pub fn new(api_id: i32, api_hash: String) -> Self {
        Self {
            api_id,
            api_hash: ApiHash(api_hash),
            flood_sleep_threshold: Duration::from_secs(10),
            network_retries: 4,
            network_backoff: Duration::from_millis(500),
            app_version: env!("CARGO_PKG_VERSION").to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const FAKE_HASH: &str = "0123456789abcdef0123456789abcdef";

    fn lookup<'a>(pairs: &'a [(&'a str, &'a str)]) -> impl Fn(&str) -> Option<String> + 'a {
        move |k| pairs.iter().find(|(n, _)| *n == k).map(|(_, v)| v.to_string())
    }

    #[test]
    fn reads_both_variables() {
        let c = TelegramConfig::from_lookup(lookup(&[(ENV_API_ID, " 12345 "), (ENV_API_HASH, FAKE_HASH)])).unwrap();
        assert_eq!(c.api_id, 12345);
        assert_eq!(c.api_hash.expose(), FAKE_HASH);
    }

    #[test]
    fn missing_and_invalid_values_never_echo_the_value() {
        assert_eq!(TelegramConfig::from_lookup(lookup(&[])).unwrap_err(), ConfigError::Missing(ENV_API_ID));
        assert_eq!(TelegramConfig::from_lookup(lookup(&[(ENV_API_ID, "1")])).unwrap_err(), ConfigError::Missing(ENV_API_HASH));
        assert_eq!(TelegramConfig::from_lookup(lookup(&[(ENV_API_ID, "x1"), (ENV_API_HASH, FAKE_HASH)])).unwrap_err(), ConfigError::Invalid(ENV_API_ID));
        let e = TelegramConfig::from_lookup(lookup(&[(ENV_API_ID, "1"), (ENV_API_HASH, "zz-no-hex")])).unwrap_err();
        assert_eq!(e, ConfigError::Invalid(ENV_API_HASH));
        assert!(!e.to_string().contains("zz-no-hex"));
    }

    #[test]
    fn debug_hides_the_hash() {
        let c = TelegramConfig::new(1, FAKE_HASH.into());
        let dbg = format!("{c:?}");
        assert!(!dbg.contains(FAKE_HASH));
        assert!(dbg.contains("<oculto>"));
    }
}
