use log::{error, info, warn};
use serenity::model::id::ChannelId;
use serenity::prelude::Context;
use std::error::Error;
use std::time::Duration;

use crate::api::BackendApiClient;
use crate::config::BotConfig;
use crate::discord::send_news_item;

/// Periodic polling loop that wakes up on interval to check for new posts across all configured channels.
pub async fn run_checker(ctx: Context, config: BotConfig) {
    let api_client = BackendApiClient::new(config.api_url.clone());
    let mut interval = tokio::time::interval(Duration::from_secs(config.interval_seconds));

    // Register all configured channels with the backend
    for cid in &config.channel_ids {
        if let Err(e) = api_client.register_subscription(&cid.to_string(), None).await {
            warn!("Could not pre-register subscription for channel {}: {}", cid, e);
        }
    }

    info!(
        "Checker task started. Polling every {} seconds for {} channel(s): {:?}.",
        config.interval_seconds,
        config.channel_ids.len(),
        config.channel_ids
    );

    loop {
        interval.tick().await;
        info!(
            "Checking for new posts across {} channel(s)...",
            config.channel_ids.len()
        );

        for channel_id in &config.channel_ids {
            if let Err(e) = check_for_channel_updates(&ctx, &api_client, *channel_id).await {
                error!("Error during news check for channel {}: {}", channel_id, e);
            }
        }
    }
}

/// Fetches unposted items for a specific channel, sends them to Discord, and marks them as posted for that channel.
pub async fn check_for_channel_updates(
    ctx: &Context,
    api_client: &BackendApiClient,
    channel_id: ChannelId,
) -> Result<(), Box<dyn Error>> {
    let channel_str = channel_id.to_string();
    let items = api_client.fetch_unposted_items(Some(&channel_str)).await?;

    if items.is_empty() {
        return Ok(());
    }

    info!(
        "Found {} new item(s) to post to channel {}.",
        items.len(),
        channel_id
    );

    let mut posted_links: Vec<String> = Vec::new();
    for item in items {
        info!("Posting to Discord channel {}: {}", channel_id, item.title);

        match send_news_item(ctx, channel_id, &item).await {
            Ok(_) => {
                posted_links.push(item.link);
            }
            Err(e) => {
                error!(
                    "Failed to send message to channel {} for [{}]: {}",
                    channel_id, item.title, e
                );
            }
        }

        // Small pause to respect Discord message send rate limits
        tokio::time::sleep(Duration::from_millis(500)).await;
    }

    if !posted_links.is_empty() {
        api_client
            .mark_items_posted(&posted_links, Some(&channel_str))
            .await?;
    }

    Ok(())
}

/// Backwards compatibility helper checking all configured channels.
#[allow(dead_code)]
pub async fn check_for_updates(
    ctx: &Context,
    api_client: &BackendApiClient,
    config: &BotConfig,
) -> Result<(), Box<dyn Error>> {
    for channel_id in &config.channel_ids {
        check_for_channel_updates(ctx, api_client, *channel_id).await?;
    }
    Ok(())
}
