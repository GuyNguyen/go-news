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
    pub channel_ids: Option<Vec<ChannelIdConfig>>,
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
    #[allow(dead_code)]
    pub channel_id: ChannelId,
    pub channel_ids: Vec<ChannelId>,
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
            info!("No config file loaded from candidate paths: {:?}. Ensure config.toml exists.", candidate_paths);
        }

        let discord_token = settings
            .discord_token
            .filter(|s| !s.trim().is_empty())
            .expect("Missing Discord token! Set 'discord_token' in config.toml.");

        let mut resolved_channel_ids: Vec<u64> = Vec::new();

        if let Some(cfg_ids) = settings.channel_ids {
            for cid in cfg_ids {
                if let Some(id) = cid.to_u64() {
                    if !resolved_channel_ids.contains(&id) {
                        resolved_channel_ids.push(id);
                    }
                }
            }
        }

        if resolved_channel_ids.is_empty() {
            if let Some(cid) = settings.channel_id.as_ref().and_then(|c| c.to_u64()) {
                resolved_channel_ids.push(cid);
            }
        }

        if resolved_channel_ids.is_empty() {
            info!("No static channel IDs configured in config.toml. Subscriptions will be managed dynamically via Discord slash commands (/subscribe).");
        }

        let channel_ids: Vec<ChannelId> = resolved_channel_ids
            .into_iter()
            .map(ChannelId::new)
            .collect();
        let primary_channel_id = channel_ids.first().copied().unwrap_or(ChannelId::new(1));

        Self {
            discord_token,
            channel_id: primary_channel_id,
            channel_ids,
            api_url: settings.api_url,
            interval_seconds: settings.check_interval_seconds,
        }
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
    fn test_parse_bot_settings_with_multiple_channels() {
        let toml_str = r#"
        [bot]
        discord_token = "my_token"
        channel_ids = [111111111, "222222222", 333333333]
        "#;
        let config: ConfigFile = toml::from_str(toml_str).expect("Failed to parse");
        let bot = config.bot.expect("bot section missing");
        let cids = bot.channel_ids.expect("channel_ids missing");
        assert_eq!(cids.len(), 3);
        assert_eq!(cids[0].to_u64(), Some(111111111));
        assert_eq!(cids[1].to_u64(), Some(222222222));
        assert_eq!(cids[2].to_u64(), Some(333333333));
    }

    #[test]
    fn test_parse_bot_settings_without_channels() {
        let toml_str = r#"
        [bot]
        discord_token = "my_token"
        "#;
        let config: ConfigFile = toml::from_str(toml_str).expect("Failed to parse");
        let bot = config.bot.expect("bot section missing");
        assert_eq!(bot.discord_token.as_deref(), Some("my_token"));
        assert!(bot.channel_ids.is_none());
        assert!(bot.channel_id.is_none());
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

