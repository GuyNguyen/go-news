mod api;
mod config;
mod discord;
mod poller;

use config::BotConfig;
use discord::BotHandler;
use dotenv::dotenv;
use log::{error, info};
use serenity::prelude::*;

#[tokio::main]
async fn main() {
    dotenv().ok();

    env_logger::Builder::from_env(
        env_logger::Env::default().default_filter_or("info,serenity=warn,tracing=warn"),
    )
    .init();

    // 1. Load validated configuration
    let config = BotConfig::from_env();

    // 2. Configure Gateway Intents (GUILDS is non-privileged, public-ready)
    let intents = GatewayIntents::GUILDS;

    // 3. Build Serenity client
    let handler = BotHandler { config: config.clone() };
    let mut client = Client::builder(&config.discord_token, intents)
        .event_handler(handler)
        .await
        .expect("Error creating Discord client");

    info!("Starting go-news bot client...");
    if let Err(e) = client.start().await {
        error!("Client error: {:?}", e);
    }
}
