mod api;
mod config;
mod discord;
mod poller;

use config::BotConfig;
use discord::BotHandler;
use log::{error, info};
use serenity::prelude::*;

#[tokio::main]
async fn main() {
    env_logger::Builder::from_env(
        env_logger::Env::default().default_filter_or("info,serenity=warn,tracing=warn"),
    )
    .init();

    // 1. Load validated configuration from TOML
    let config = BotConfig::load();

    // 2. Configure Gateway Intents (GUILDS is non-privileged, public-ready)
    let intents = GatewayIntents::GUILDS;

    // 3. Build Serenity client
    let api_client = api::BackendApiClient::new(config.api_url.clone());
    let handler = BotHandler {
        config: config.clone(),
        api_client,
    };
    let mut client = Client::builder(&config.discord_token, intents)
        .event_handler(handler)
        .await
        .expect("Error creating Discord client");

    info!("Starting go-news bot client...");
    if let Err(e) = client.start().await {
        error!("Client error: {:?}", e);
    }
}
