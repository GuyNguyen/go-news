use log::{error, info};
use serenity::prelude::Context;
use std::error::Error;
use std::time::Duration;

use crate::api::BackendApiClient;
use crate::config::BotConfig;
use crate::discord::send_news_item;

/// Periodic polling loop that wakes up on interval to check for new posts.
pub async fn run_checker(ctx: Context, config: BotConfig) {
    let api_client = BackendApiClient::new(config.api_url.clone());
    let mut interval = tokio::time::interval(Duration::from_secs(config.interval_seconds));

    info!(
        "Checker task started. Polling every {} seconds for channel {}.",
        config.interval_seconds, config.channel_id
    );

    loop {
        interval.tick().await;
        info!("Checking for new posts...");

        if let Err(e) = check_for_updates(&ctx, &api_client, &config).await {
            error!("Error during news check: {}", e);
        }
    }
}

/// Fetches unposted items, sends them to Discord, and marks them as posted.
pub async fn check_for_updates(
    ctx: &Context,
    api_client: &BackendApiClient,
    config: &BotConfig,
) -> Result<(), Box<dyn Error>> {
    let items = api_client.fetch_unposted_items().await?;

    if items.is_empty() {
        info!("No new items to post.");
        return Ok(());
    }

    info!("Found {} new item(s) to post.", items.len());

    let mut posted_links: Vec<String> = Vec::new();
    for item in items {
        info!("Posting to Discord: {}", item.title);

        match send_news_item(ctx, config.channel_id, &item).await {
            Ok(_) => {
                posted_links.push(item.link);
            }
            Err(e) => {
                error!("Failed to send message for [{}]: {}", item.title, e);
            }
        }

        // Small pause to respect Discord message send rate limits
        tokio::time::sleep(Duration::from_millis(500)).await;
    }

    if !posted_links.is_empty() {
        api_client.mark_items_posted(&posted_links).await?;
    }

    Ok(())
}
