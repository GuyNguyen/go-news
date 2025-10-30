use serenity::async_trait;
use serenity::builder::{CreateEmbed, CreateMessage};
use serenity::model::gateway::Ready;
use serenity::model::id::ChannelId;
use serenity::model::Timestamp;
use serenity::prelude::*;
use std::env;
use std::error::Error;
use std::time::Duration;

use dotenv::dotenv;
use log::{error, info};
use reqwest::Client as HttpClient;
use serde::{Deserialize, Serialize};

// --- Configuration ---
// These are loaded from environment variables
// 1. DISCORD_TOKEN
// 2. CHANNEL_ID (the channel where posts will be sent)
// 3. BACKEND_API_URL (e.g., "http://127.0.0.1:8080")
// 4. CHECK_INTERVAL_SECONDS (e.g., "60" for one minute)

// --- Data Structures for API ---
// These structs must match the ones in your backend API

#[derive(Debug, Serialize, Deserialize)]
struct RssItem {
    title: String,
    link: String,
    description: String,
    pub_date: String,
    posted: bool,
}

#[derive(Debug, Serialize, Deserialize)]
struct MarkPostedRequest {
    links: Vec<String>,
}

// --- Bot Event Handler ---

struct Handler;

#[async_trait]
impl EventHandler for Handler {
    /// This event fires once the bot is connected and ready.
    async fn ready(&self, ctx: Context, ready: Ready) {
        info!("Bot is connected and ready as {}!", ready.user.name);

        // Spawn a new task that runs the periodic checker
        let ctx = ctx.clone();
        tokio::spawn(async move {
            run_checker(ctx).await;
        });
    }
}

/// The main logic for periodically checking the backend for new posts.
async fn run_checker(ctx: Context) {
    // Load configuration from environment
    let channel_id = env::var("CHANNEL_ID")
        .expect("Expected CHANNEL_ID in environment")
        .parse::<u64>()
        .expect("CHANNEL_ID must be a valid number");
    let channel_id = ChannelId::new(channel_id);

    let api_url =
        env::var("BACKEND_API_URL").expect("Expected BACKEND_API_URL in environment");

    let interval_seconds = env::var("CHECK_INTERVAL_SECONDS")
        .unwrap_or("60".to_string())
        .parse::<u64>()
        .expect("CHECK_INTERVAL_SECONDS must be a valid number");

    let http_client = HttpClient::new();
    let mut interval = tokio::time::interval(Duration::from_secs(interval_seconds));

    info!(
        "Checker task started. Checking every {} seconds.",
        interval_seconds
    );

    // This loop runs forever
    loop {
        interval.tick().await;
        info!("Checking for new posts...");

        match check_for_updates(&ctx, &http_client, &api_url, channel_id).await {
            Ok(_) => info!("Check completed successfully."),
            Err(e) => error!("Error during check: {}", e),
        }
    }
}

/// This function performs the actual work:
/// 1. Calls GET /items/unposted
/// 2. Posts new items to Discord
/// 3. Calls POST /items/mark-posted
async fn check_for_updates(
    ctx: &Context,
    http_client: &HttpClient,
    api_url: &str,
    channel_id: ChannelId,
) -> Result<(), Box<dyn Error>> {
    let get_url = format!("{}/items/unposted", api_url);
    let response = http_client.get(&get_url).send().await?;

    if !response.status().is_success() {
        return Err(format!("Failed to get unposted items: {}", response.status()).into());
    }

    let items: Vec<RssItem> = response.json().await?;

    if items.is_empty() {
        info!("No new items to post.");
        return Ok(());
    }

    info!("Found {} new items to post.", items.len());

    // 2. Post new items to Discord
    let mut posted_links: Vec<String> = Vec::new();
    for item in items {
        info!("Posting: {}", item.title);

        // Parse the pub_date string into a Timestamp for the embed
        let timestamp =
            match chrono::DateTime::parse_from_rfc2822(&item.pub_date) {
                // Use `from_unix_timestamp` which takes an i64 and returns a Result
                Ok(ts) => Timestamp::from_unix_timestamp(ts.timestamp())
                                .unwrap_or_else(|_| Timestamp::now()), // Handle the Result
                Err(_) => Timestamp::now(), // Fallback to current time
            };

        let embed = CreateEmbed::new()
            .title(&item.title)
            .url(&item.link)
            .description(&item.description)
            .timestamp(timestamp)
            .color(0x00_FF_00); // Green


        let message = CreateMessage::new().embed(embed);

        let send_result = channel_id
            .send_message(&ctx.http, message)
            .await;

        match send_result {
            Ok(_) => {
                posted_links.push(item.link);
            }
            Err(e) => {
                error!("Failed to send message for [{}]: {}", item.title, e);
            }
        }

        tokio::time::sleep(Duration::from_secs(1)).await;
    }

    if !posted_links.is_empty() {
        let post_url = format!("{}/items/mark-posted", api_url);
        let payload = MarkPostedRequest {
            links: posted_links.clone(),
        };

        let post_response = http_client.post(&post_url).json(&payload).send().await?;

        if post_response.status().is_success() {
            info!("Successfully marked {} items as posted.", posted_links.len());
        } else {
            error!(
                "Failed to mark items as posted. Status: {}",
                post_response.status()
            );
        }
    }

    Ok(())
}

// --- Main Function ---

#[tokio::main]
async fn main() {
    // Load environment variables from .env file
    dotenv().ok();

    // Initialize logger
    env_logger::Builder::from_env(
        // Use RUST_LOG environment variable if set,
        // otherwise default to this filter string.
        env_logger::Env::default()
            .default_filter_or("info,serenity=warn,tracing=warn")
    )
    .init();

    // 1. Get Discord token from environment
    let token = env::var("DISCORD_TOKEN").expect("Expected DISCORD_TOKEN in environment");

    // 2. Define bot intents
    let intents = GatewayIntents::GUILD_MESSAGES | GatewayIntents::MESSAGE_CONTENT;

    // 3. Build and start the client
    let mut client = Client::builder(&token, intents)
        .event_handler(Handler)
        .await
        .expect("Error creating client");

    info!("Starting bot...");
    if let Err(e) = client.start().await {
        error!("Client error: {:?}", e);
    }
}
