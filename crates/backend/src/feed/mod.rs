use actix_web::web;
use chrono::Utc;
use go_news_shared::parse_rss_date;
use log::{error, info, warn};
use rss::Channel;
use sqlx::SqlitePool;
use std::error::Error;
use std::time::Duration;

use crate::db;

pub async fn fetch_and_store_feed_internal(
    pool: &SqlitePool,
    feed_url: &str,
    mark_as_posted: bool,
    retention_days: i64,
) -> Result<u64, Box<dyn Error + Send + Sync>> {
    info!(
        "Starting RSS feed fetch from {} (mark_as_posted = {})...",
        feed_url, mark_as_posted
    );

    let http_client = reqwest::Client::builder()
        .user_agent("go-news-backend/1.0")
        .timeout(Duration::from_secs(30))
        .build()?;

    let mut response = http_client.get(feed_url).send().await?.error_for_status()?;

    const MAX_FEED_BYTES: usize = 10 * 1024 * 1024; // 10 MB limit
    if let Some(len) = response.content_length() {
        if len > MAX_FEED_BYTES as u64 {
            return Err(format!(
                "Feed response exceeds maximum limit of {} bytes (got {})",
                MAX_FEED_BYTES, len
            )
            .into());
        }
    }

    let mut content = Vec::new();
    while let Some(chunk) = response.chunk().await? {
        if content.len() + chunk.len() > MAX_FEED_BYTES {
            return Err(format!(
                "Feed response exceeded maximum limit of {} bytes",
                MAX_FEED_BYTES
            )
            .into());
        }
        content.extend_from_slice(&chunk);
    }
    info!("Successfully fetched RSS feed ({} bytes).", content.len());

    let channel = Channel::read_from(&content[..])?;
    info!("Successfully parsed RSS channel: {}", channel.title());

    let mut new_items_count = 0;
    for item in channel.into_items() {
        let title = item.title().unwrap_or_default().trim().to_string();
        let link = item.link().unwrap_or_default().trim().to_string();
        let description = item.description().unwrap_or_default().trim().to_string();

        let raw_pub_date = item
            .pub_date()
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .or_else(|| {
                item.dublin_core_ext()
                    .and_then(|dc| dc.dates().first().map(|s| s.as_str()))
                    .map(str::trim)
                    .filter(|s| !s.is_empty())
            })
            .unwrap_or_default();

        let pub_date = parse_rss_date(raw_pub_date)
            .unwrap_or_else(Utc::now)
            .to_rfc3339();

        if link.is_empty() && title.is_empty() {
            continue;
        }

        let inserted = db::insert_item_if_new(
            pool,
            &title,
            &link,
            &description,
            &pub_date,
            mark_as_posted,
        )
        .await?;

        if inserted {
            info!(
                "Stored item: '{}' (posted = {})",
                title, mark_as_posted
            );
            new_items_count += 1;
        }
    }

    info!(
        "Feed processing complete. {} new item(s) stored.",
        new_items_count
    );

    // Prune old posted items to keep SQLite database lean
    if let Err(e) = db::prune_old_items(pool, retention_days).await {
        warn!("Failed to prune stale items: {}", e);
    }

    Ok(new_items_count)
}

pub async fn fetch_and_store_feed(
    pool: &SqlitePool,
    feed_url: &str,
    retention_days: i64,
) -> Result<u64, Box<dyn Error + Send + Sync>> {
    fetch_and_store_feed_internal(pool, feed_url, false, retention_days).await
}

/// If database is empty, performs an initial fetch marking all existing items as posted.
/// This prevents the bot from spamming historical feed items on initial launch.
pub async fn initial_sync_if_empty(
    pool: &SqlitePool,
    feed_url: &str,
    retention_days: i64,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    let count = db::count_items(pool).await?;

    if count == 0 {
        info!("Empty database detected on startup! Initializing with current RSS items marked as already posted...");
        let seeded = fetch_and_store_feed_internal(pool, feed_url, true, retention_days).await?;
        info!(
            "Initial sync complete: {} item(s) stored as already posted. The bot will only post new articles published after this point.",
            seeded
        );
    } else {
        info!("Database already contains {} items. Skipping initial seed.", count);
    }
    Ok(())
}

pub async fn run_periodic_checker(
    pool: web::Data<SqlitePool>,
    feed_url: String,
    interval_seconds: u64,
    retention_days: i64,
) {
    let mut interval = tokio::time::interval(Duration::from_secs(interval_seconds));
    loop {
        interval.tick().await;
        info!("Running periodic check for RSS feed updates...");
        if let Err(e) = fetch_and_store_feed(&pool, &feed_url, retention_days).await {
            error!("An error occurred during the periodic feed check: {}", e);
        }
    }
}
