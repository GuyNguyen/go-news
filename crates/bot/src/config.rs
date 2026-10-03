use log::{info, warn};
use serde::Deserialize;
use serenity::model::id::ChannelId;
use std::env;
use std::fs;

#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
pub enum ChannelIdConfig {
    Int(u64),
    Str(String),
}

impl ChannelIdConfig {
    pub fn to_u64(&self) -> Option<u64> {
        match self {
            ChannelIdConfig::Int(id) => Some(*id),
            ChannelIdConfig::Str(s) => s.trim().parse::<u64>().ok(),
        }
    }
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct BotSettings {
    pub discord_token: Option<String>,
    pub channel_id: Option<ChannelIdConfig>,
    #[serde(default = "default_api_url")]
    pub api_url: String,
    #[serde(default = "default_interval_seconds")]
    pub check_interval_seconds: u64,
}

fn default_api_url() -> String {
    "http://backend:8080".to_string()
}

fn default_interval_seconds() -> u64 {
    60
}

#[derive(Debug, Deserialize, Default)]
struct ConfigFile {
    #[serde(default)]
    bot: Option<BotSettings>,
}

#[derive(Clone, Debug)]
pub struct BotConfig {
    pub discord_token: String,
    pub channel_id: ChannelId,
    pub api_url: String,
    pub interval_seconds: u64,
}

impl BotConfig {
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

        let mut settings = BotSettings::default();
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
                            info!("Loaded bot configuration from {:?}", path);
                            if let Some(bot_cfg) = parsed.bot {
                                settings = bot_cfg;
                            } else {
                                warn!("Parsed {:?} successfully, but no '[bot]' section was found.", path);
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

        // Environment overrides
        let discord_token = env::var("DISCORD_TOKEN")
            .ok()
            .filter(|s| !s.trim().is_empty())
            .or(settings.discord_token)
            .expect(
                "Missing Discord token! Set 'discord_token' in config.toml or provide DISCORD_TOKEN in environment.",
            );

        let channel_id_raw = env::var("CHANNEL_ID")
            .ok()
            .and_then(|s| s.parse::<u64>().ok())
            .or_else(|| settings.channel_id.as_ref().and_then(|c| c.to_u64()))
            .expect(
                "Missing or invalid Discord channel ID! Set 'channel_id' in config.toml or provide numeric CHANNEL_ID in environment.",
            );

        if let Ok(url) = env::var("BACKEND_API_URL") {
            settings.api_url = url;
        }

        if let Some(interval) = env::var("CHECK_INTERVAL_SECONDS")
            .ok()
            .and_then(|s| s.parse::<u64>().ok())
        {
            settings.check_interval_seconds = interval;
        }

        Self {
            discord_token,
            channel_id: ChannelId::new(channel_id_raw),
            api_url: settings.api_url,
            interval_seconds: settings.check_interval_seconds,
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
    fn test_parse_bot_settings_with_int_channel() {
        let toml_str = r#"
        [bot]
        api_url = "http://localhost:8080"
        check_interval_seconds = 45
        discord_token = "my_token"
        channel_id = 123456789
        "#;
        let config: ConfigFile = toml::from_str(toml_str).expect("Failed to parse");
        let bot = config.bot.expect("bot section missing");
        assert_eq!(bot.discord_token.as_deref(), Some("my_token"));
        assert_eq!(bot.channel_id.unwrap().to_u64(), Some(123456789));
        assert_eq!(bot.api_url, "http://localhost:8080");
        assert_eq!(bot.check_interval_seconds, 45);
    }

    #[test]
    fn test_parse_bot_settings_with_string_channel() {
        let toml_str = r#"
        [bot]
        discord_token = "my_token"
        channel_id = "9876543210123"
        "#;
        let config: ConfigFile = toml::from_str(toml_str).expect("Failed to parse");
        let bot = config.bot.expect("bot section missing");
        assert_eq!(bot.channel_id.unwrap().to_u64(), Some(9876543210123));
        assert_eq!(bot.api_url, "http://backend:8080");
        assert_eq!(bot.check_interval_seconds, 60);
    }

    #[test]
    fn test_parse_full_config_file() {
        let content = std::fs::read_to_string("../../config.toml").expect("Failed to read config.toml");
        let parsed: Result<ConfigFile, _> = toml::from_str(&content);
        match parsed {
            Ok(cfg) => {
                let bot = cfg.bot.expect("bot section missing");
                assert!(bot.discord_token.is_some());
            }
            Err(e) => {
                panic!("Failed to parse config.toml: {}", e);
            }
        }
    }
}

