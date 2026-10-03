use go_news_shared::{parse_rss_date, RssItem};
use log::info;
use serenity::async_trait;
use serenity::builder::{CreateEmbed, CreateMessage};
use serenity::model::gateway::Ready;
use serenity::model::id::ChannelId;
use serenity::model::Timestamp;
use serenity::prelude::*;
use std::error::Error;

use crate::config::BotConfig;
use crate::poller::run_checker;

pub struct BotHandler {
    pub config: BotConfig,
}

#[async_trait]
impl EventHandler for BotHandler {
    /// Fires once the bot connects and is ready.
    async fn ready(&self, ctx: Context, ready: Ready) {
        info!("Bot connected and ready as {}!", ready.user.name);

        let ctx = ctx.clone();
        let config = self.config.clone();
        tokio::spawn(async move {
            run_checker(ctx, config).await;
        });
    }
}

/// Builds a styled Discord embed for a given news item.
pub fn build_news_embed(item: &RssItem) -> CreateEmbed {
    let timestamp = parse_rss_date(&item.pub_date)
        .and_then(|dt| Timestamp::from_unix_timestamp(dt.timestamp()).ok())
        .unwrap_or_else(Timestamp::now);

    CreateEmbed::new()
        .title(&item.title)
        .url(&item.link)
        .description(&item.description)
        .timestamp(timestamp)
        .color(0x00_FF_00) // Go green
}

/// Dispatches a single news item as an embed message to the target Discord channel.
pub async fn send_news_item(
    ctx: &Context,
    channel_id: ChannelId,
    item: &RssItem,
) -> Result<(), Box<dyn Error>> {
    let embed = build_news_embed(item);
    let message = CreateMessage::new().embed(embed);
    channel_id.send_message(&ctx.http, message).await?;
    Ok(())
}
