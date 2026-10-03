use log::{info, warn};
use serde::Deserialize;
use serenity::model::id::ChannelId;
use std::env;
use std::fs;
use std::path::Path;

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
        let config_path = env::var("CONFIG_PATH").unwrap_or_else(|_| "config.toml".to_string());

        let mut settings = if Path::new(&config_path).exists() {
            info!("Loading bot configuration from TOML file: {}", config_path);
            match fs::read_to_string(&config_path) {
                Ok(content) => match toml::from_str::<ConfigFile>(&content) {
                    Ok(parsed) => parsed.bot.unwrap_or_default(),
                    Err(e) => {
                        warn!(
                            "Failed to parse {}: {}. Falling back to default settings / env.",
                            config_path, e
                        );
                        BotSettings::default()
                    }
                },
                Err(e) => {
                    warn!(
                        "Failed to read {}: {}. Falling back to default settings / env.",
                        config_path, e
                    );
                    BotSettings::default()
                }
            }
        } else {
            BotSettings::default()
        };

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
}

