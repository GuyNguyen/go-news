use go_news_shared::{parse_rss_date, RssItem};
use log::{error, info};
use serenity::async_trait;
use serenity::builder::{
    CreateCommand, CreateEmbed, CreateInteractionResponse, CreateInteractionResponseMessage,
    CreateMessage,
};
use serenity::model::application::{Command, Interaction};
use serenity::model::gateway::Ready;
use serenity::model::id::ChannelId;
use serenity::model::Permissions;
use serenity::model::Timestamp;
use serenity::prelude::*;
use std::error::Error;

use crate::api::BackendApiClient;
use crate::config::BotConfig;
use crate::poller::run_checker;

pub struct BotHandler {
    pub config: BotConfig,
    pub api_client: BackendApiClient,
}

#[async_trait]
impl EventHandler for BotHandler {
    /// Fires once the bot connects and is ready.
    async fn ready(&self, ctx: Context, ready: Ready) {
        info!("Bot connected and ready as {}!", ready.user.name);

        // Register global slash commands
        let commands = vec![
            CreateCommand::new("subscribe")
                .description("Subscribe this channel to receive new Go (Baduk/Weiqi) news updates")
                .default_member_permissions(Permissions::MANAGE_CHANNELS),
            CreateCommand::new("unsubscribe")
                .description("Unsubscribe this channel from Go news updates")
                .default_member_permissions(Permissions::MANAGE_CHANNELS),
            CreateCommand::new("news-status")
                .description("Check if this channel is currently subscribed to Go news updates"),
        ];

        match Command::set_global_commands(&ctx.http, commands).await {
            Ok(registered) => {
                info!(
                    "Successfully registered {} global slash command(s): {:?}",
                    registered.len(),
                    registered.iter().map(|c| &c.name).collect::<Vec<_>>()
                );
            }
            Err(e) => {
                error!("Failed to register global slash commands: {:?}", e);
            }
        }

        // Spawn periodic background news poller
        let ctx_clone = ctx.clone();
        let config_clone = self.config.clone();
        tokio::spawn(async move {
            run_checker(ctx_clone, config_clone).await;
        });
    }

    /// Handles slash command interactions
    async fn interaction_create(&self, ctx: Context, interaction: Interaction) {
        if let Interaction::Command(command) = interaction {
            let channel_id = command.channel_id;
            let guild_id = command.guild_id;

            let response_text = match command.data.name.as_str() {
                "subscribe" => {
                    let channel_str = channel_id.to_string();
                    let guild_str = guild_id.map(|g| g.to_string());
                    match self
                        .api_client
                        .register_subscription(&channel_str, guild_str.as_deref())
                        .await
                    {
                        Ok(_) => {
                            // Mark existing articles as delivered to avoid flood
                            let _ = self.api_client.init_posted_for_channel(&channel_str).await;
                            info!("Channel {} subscribed via /subscribe.", channel_id);
                            format!(
                                "✅ Successfully subscribed <#{}> to Go news updates! New articles will be posted here as they are published.",
                                channel_id
                            )
                        }
                        Err(e) => {
                            error!("Failed to register subscription for channel {}: {}", channel_id, e);
                            format!("❌ Failed to subscribe channel: {}. Please try again later.", e)
                        }
                    }
                }
                "unsubscribe" => {
                    let channel_str = channel_id.to_string();
                    match self.api_client.remove_subscription(&channel_str).await {
                        Ok(true) => {
                            info!("Channel {} unsubscribed via /unsubscribe.", channel_id);
                            format!(
                                "❌ Unsubscribed <#{}> from Go news updates. No further articles will be posted here.",
                                channel_id
                            )
                        }
                        Ok(false) => {
                            format!("ℹ️ Channel <#{}> is not currently subscribed.", channel_id)
                        }
                        Err(e) => {
                            error!("Failed to remove subscription for channel {}: {}", channel_id, e);
                            format!("❌ Failed to unsubscribe channel: {}. Please try again later.", e)
                        }
                    }
                }
                "news-status" => {
                    let channel_str = channel_id.to_string();
                    match self.api_client.fetch_subscriptions().await {
                        Ok(subs) => {
                            let is_sub = subs.iter().any(|s| s.channel_id == channel_str);
                            if is_sub {
                                format!(
                                    "📰 Channel <#{}> is currently **subscribed** to Go news updates.",
                                    channel_id
                                )
                            } else {
                                format!(
                                    "ℹ️ Channel <#{}> is **not** subscribed. Run `/subscribe` to receive Go news here.",
                                    channel_id
                                )
                            }
                        }
                        Err(e) => {
                            format!("❌ Could not query subscription status: {}", e)
                        }
                    }
                }
                _ => "Unknown command".to_string(),
            };

            let response_msg = CreateInteractionResponseMessage::new().content(response_text);
            let response = CreateInteractionResponse::Message(response_msg);

            if let Err(e) = command.create_response(&ctx.http, response).await {
                error!("Failed to respond to slash command: {:?}", e);
            }
        }
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
) -> Result<(), Box<dyn Error + Send + Sync>> {
    let embed = build_news_embed(item);
    let message = CreateMessage::new().embed(embed);
    channel_id.send_message(&ctx.http, message).await?;
    Ok(())
}
