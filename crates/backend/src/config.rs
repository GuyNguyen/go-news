use log::{info, warn};
use serde::Deserialize;
use std::env;
use std::fs;

#[derive(Debug, Clone, Deserialize)]
pub struct BackendSettings {
    #[serde(default = "default_database_url")]
    pub database_url: String,

    #[serde(default = "default_feed_url")]
    pub feed_url: String,

    #[serde(default = "default_check_interval")]
    pub check_interval_seconds: u64,

    #[serde(default = "default_retention_days")]
    pub retention_days: i64,

    #[serde(default = "default_host")]
    pub host: String,

    #[serde(default = "default_port")]
    pub port: u16,
}

fn default_database_url() -> String {
    "sqlite://data/go_news.db?mode=rwc".to_string()
}

fn default_feed_url() -> String {
    "https://www.igomely.com/feed".to_string()
}

fn default_check_interval() -> u64 {
    1800
}

fn default_retention_days() -> i64 {
    60
}

fn default_host() -> String {
    "0.0.0.0".to_string()
}

fn default_port() -> u16 {
    8080
}

impl Default for BackendSettings {
    fn default() -> Self {
        Self {
            database_url: default_database_url(),
            feed_url: default_feed_url(),
            check_interval_seconds: default_check_interval(),
            retention_days: default_retention_days(),
            host: default_host(),
            port: default_port(),
        }
    }
}

#[derive(Debug, Deserialize, Default)]
struct ConfigFile {
    #[serde(default)]
    backend: Option<BackendSettings>,
}

#[derive(Clone, Debug)]
pub struct AppConfig {
    pub database_url: String,
    pub feed_url: String,
    pub check_interval_seconds: u64,
    pub retention_days: i64,
    pub host: String,
    pub port: u16,
}

impl AppConfig {
    /// Loads configuration from TOML file (default: config.toml or CONFIG_PATH),
    /// layered with environment variable overrides.
    pub fn load() -> Self {
        let candidate_paths = if let Ok(custom) = env::var("CONFIG_PATH") {
            vec![std::path::PathBuf::from(custom)]
        } else {
            vec![
                std::path::PathBuf::from("config.toml"),
                std::path::PathBuf::from("/app/config.toml"),
            ]
        };

        let mut settings = BackendSettings::default();
        let mut loaded = false;

        for path in &candidate_paths {
            if path.exists() {
                if path.is_dir() {
                    log::error!(
                        "Config path '{:?}' is a directory, not a file! (If mounted via Docker, recreate config.toml on the host as a file)",
                        path
                    );
                    continue;
                }

                match fs::read_to_string(path) {
                    Ok(content) => match toml::from_str::<ConfigFile>(&content) {
                        Ok(parsed) => {
                            info!("Loaded backend configuration from {:?}", path);
                            if let Some(backend_cfg) = parsed.backend {
                                settings = backend_cfg;
                            } else {
                                warn!("Parsed {:?} successfully, but no '[backend]' section was found.", path);
                            }
                            loaded = true;
                            break;
                        }
                        Err(e) => {
                            log::error!("Failed to parse TOML configuration from {:?}: {}", path, e);
                        }
                    },
                    Err(e) => {
                        log::error!(
                            "Found config file at {:?} but failed to read it: {}. (Check permissions, e.g. chmod 644)",
                            path, e
                        );
                    }
                }
            }
        }

        if !loaded {
            info!("No config file loaded. Relying on environment variables and defaults.");
        }

        // Environment variable overrides
        if let Ok(val) = env::var("DATABASE_URL") {
            settings.database_url = val;
        }
        if let Ok(val) = env::var("FEED_URL") {
            settings.feed_url = val;
        }
        if let Ok(val) = env::var("CHECK_INTERVAL_SECONDS") {
            if let Ok(num) = val.parse::<u64>() {
                settings.check_interval_seconds = num;
            }
        }
        if let Ok(val) = env::var("RETENTION_DAYS") {
            if let Ok(num) = val.parse::<i64>() {
                settings.retention_days = num;
            }
        }
        if let Ok(val) = env::var("HOST") {
            settings.host = val;
        }
        if let Ok(val) = env::var("PORT") {
            if let Ok(num) = val.parse::<u16>() {
                settings.port = num;
            }
        }

        Self {
            database_url: settings.database_url,
            feed_url: settings.feed_url,
            check_interval_seconds: settings.check_interval_seconds,
            retention_days: settings.retention_days,
            host: settings.host,
            port: settings.port,
        }
    }

    pub fn from_env() -> Self {
        Self::load()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_backend_settings_from_toml() {
        let toml_str = r#"
        [backend]
        database_url = "sqlite::memory:"
        feed_url = "https://custom.feed/rss"
        check_interval_seconds = 900
        retention_days = 30
        host = "127.0.0.1"
        port = 9090
        "#;
        let config: ConfigFile = toml::from_str(toml_str).expect("Failed to parse");
        let backend = config.backend.expect("backend section missing");
        assert_eq!(backend.database_url, "sqlite::memory:");
        assert_eq!(backend.feed_url, "https://custom.feed/rss");
        assert_eq!(backend.check_interval_seconds, 900);
        assert_eq!(backend.retention_days, 30);
        assert_eq!(backend.host, "127.0.0.1");
        assert_eq!(backend.port, 9090);
    }

    #[test]
    fn test_backend_settings_defaults() {
        let toml_str = r#"
        [backend]
        "#;
        let config: ConfigFile = toml::from_str(toml_str).expect("Failed to parse");
        let backend = config.backend.expect("backend section missing");
        assert_eq!(backend.database_url, "sqlite://data/go_news.db?mode=rwc");
        assert_eq!(backend.feed_url, "https://www.igomely.com/feed");
        assert_eq!(backend.check_interval_seconds, 1800);
        assert_eq!(backend.retention_days, 60);
        assert_eq!(backend.host, "0.0.0.0");
        assert_eq!(backend.port, 8080);
    }
}

