mod config;
mod db;
mod feed;
mod handlers;

use actix_web::{web, App, HttpServer};
use config::AppConfig;
use log::{info, warn};

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    dotenv::dotenv().ok();

    env_logger::Builder::from_env(
        env_logger::Env::default().default_filter_or("info,actix_web=info,sqlx=warn"),
    )
    .init();

    let config = AppConfig::from_env();
    info!("Connecting to SQLite database at {}...", config.database_url);

    let pool = db::create_pool(&config.database_url)
        .await
        .expect("Failed to initialize SQLite database pool");

    // Check if database is empty and pre-populate with posted=1 to prevent historical spam
    if let Err(e) = feed::initial_sync_if_empty(&pool, &config.feed_url, config.retention_days).await {
        warn!("Initial feed sync check encountered an issue: {}", e);
    }

    let db_pool = web::Data::new(pool);
    let config_data = web::Data::new(config.clone());
    info!("Successfully connected to SQLite database.");

    let background_pool = db_pool.clone();
    let bg_feed_url = config.feed_url.clone();
    let bg_interval = config.check_interval_seconds;
    let bg_retention = config.retention_days;
    tokio::spawn(async move {
        feed::run_periodic_checker(background_pool, bg_feed_url, bg_interval, bg_retention).await;
    });
    info!(
        "Periodic feed checker started in background (checking every {}s).",
        config.check_interval_seconds
    );

    let host = config.host.clone();
    let port = config.port;
    info!("Starting Actix web server at http://{}:{}", host, port);

    HttpServer::new(move || {
        App::new()
            .app_data(db_pool.clone())
            .app_data(config_data.clone())
            .configure(handlers::configure_routes)
    })
    .bind((host.as_str(), port))?
    .run()
    .await
}

#[cfg(test)]
mod tests {
    use super::*;
    use go_news_shared::RssItem;
    use rss::Channel;
    use std::time::Duration;

    const TEST_FEED_URL: &str = "https://www.igomely.com/feed";

    #[tokio::test]
    async fn test_fetch_igomely_feed() {
        let http_client = reqwest::Client::builder()
            .user_agent("go-news-backend/1.0")
            .timeout(Duration::from_secs(30))
            .build()
            .expect("Failed to build client");
        let content = http_client
            .get(TEST_FEED_URL)
            .send()
            .await
            .expect("Failed to fetch feed")
            .error_for_status()
            .expect("HTTP error response")
            .bytes()
            .await
            .expect("Failed to get bytes");
        let channel = Channel::read_from(&content[..]).expect("Failed to parse channel");
        assert_eq!(channel.title(), "igomely");
        assert!(!channel.items().is_empty());
    }

    #[test]
    fn test_parse_user_xml_snippet() {
        let xml = r#"<rss version="2.0">
            <channel>
            <title>igomely</title>
            <link>https://www.igomely.com/news</link>
            <description>Aggregated Go news from around the world.</description>
            <language>en</language>
            <pubDate>Sat, 05 Sep 2026 20:23:00 +0000</pubDate>
            <lastBuildDate>Sat, 05 Sep 2026 20:23:00 +0000</lastBuildDate>
            <image>
            <url>https://www.igomely.com/feed_image.png</url>
            <title>igomely</title>
            <link>https://www.igomely.com/news</link>
            <width>88</width>
            <height>80</height>
            </image>
            <item>
            <title>
            The 46th World Amateur Go Championship starts tomorrow in Mungyeong, Korea!
            </title>
            <link>https://www.eurogofed.org/wagc-2026-mungyeong/</link>
            <pubDate>Sat, 05 Sep 2026 20:23:00 +0000</pubDate>
            <dc:date>2026-09-05T20:23:00+00:00</dc:date>
            </item>
            <item>
            <title>September National Self-Pair Tournament Now Open</title>
            <link>
            https://www.usgo.org/content.aspx?page_id=5&amp;club_id=454497&amp;item_id=139639
            </link>
            <pubDate>Thu, 03 Sep 2026 00:00:00 +0000</pubDate>
            <dc:date>2026-09-03T00:00:00+00:00</dc:date>
            </item>
            <dc:date>2026-09-05T20:23:00+00:00</dc:date>
            </channel>
            </rss>"#;
        let channel = Channel::read_from(xml.as_bytes()).expect("Failed to parse channel");
        assert_eq!(channel.title(), "igomely");
        assert_eq!(channel.items().len(), 2);

        let item1 = &channel.items()[0];
        let title1 = item1.title().unwrap_or_default().trim();
        let link1 = item1.link().unwrap_or_default().trim();
        let pub_date1 = item1.pub_date().unwrap_or_default().trim();
        let description1 = item1.description().unwrap_or_default().trim();
        assert_eq!(
            title1,
            "The 46th World Amateur Go Championship starts tomorrow in Mungyeong, Korea!"
        );
        assert_eq!(link1, "https://www.eurogofed.org/wagc-2026-mungyeong/");
        assert_eq!(pub_date1, "Sat, 05 Sep 2026 20:23:00 +0000");
        assert_eq!(description1, "");

        let item2 = &channel.items()[1];
        let title2 = item2.title().unwrap_or_default().trim();
        let link2 = item2.link().unwrap_or_default().trim();
        assert_eq!(title2, "September National Self-Pair Tournament Now Open");
        assert_eq!(
            link2,
            "https://www.usgo.org/content.aspx?page_id=5&club_id=454497&item_id=139639"
        );
    }

    #[tokio::test]
    async fn test_sqlite_in_memory_db() {
        let pool = db::create_pool("sqlite::memory:")
            .await
            .expect("Failed to create in-memory pool");

        let item = RssItem {
            title: "Test Go News".to_string(),
            link: "https://example.com/test-news".to_string(),
            description: "Test description".to_string(),
            pub_date: "2026-10-02T10:00:00Z".to_string(),
            posted: false,
        };

        db::insert_item_if_new(
            &pool,
            &item.title,
            &item.link,
            &item.description,
            &item.pub_date,
            item.posted,
        )
        .await
        .expect("Failed to insert item");

        let items = db::get_unposted_items(&pool)
            .await
            .expect("Failed to fetch unposted");

        assert_eq!(items.len(), 1);
        assert_eq!(items[0].title, "Test Go News");
        assert_eq!(items[0].link, "https://example.com/test-news");
        assert!(!items[0].posted);
    }

    #[tokio::test]
    async fn test_initial_sync_empty_db() {
        let pool = db::create_pool("sqlite::memory:")
            .await
            .expect("Failed to create in-memory pool");

        let count = db::count_items(&pool).await.expect("Failed to count");
        assert_eq!(count, 0);

        // Run initial sync
        feed::initial_sync_if_empty(&pool, TEST_FEED_URL, 60)
            .await
            .expect("Initial sync failed");

        let total = db::count_items(&pool)
            .await
            .expect("Failed to count total");
        assert!(total > 0, "Feed items should have been loaded");

        // Verify that 0 items are unposted
        let unposted = db::count_unposted_items(&pool)
            .await
            .expect("Failed to count unposted");
        assert_eq!(unposted, 0, "No items should be unposted after initial sync!");
    }

    #[tokio::test]
    async fn test_pruning_old_items() {
        let pool = db::create_pool("sqlite::memory:")
            .await
            .expect("Failed to create in-memory pool");

        sqlx::query(
            "INSERT INTO items (title, link, description, pub_date, posted, created_at) VALUES (?, ?, ?, ?, 1, datetime('now', '-100 days'))",
        )
        .bind("Old Article")
        .bind("https://example.com/old")
        .bind("Old description")
        .bind("2026-06-01T00:00:00Z")
        .execute(&pool)
        .await
        .expect("Failed to insert old item");

        sqlx::query(
            "INSERT INTO items (title, link, description, pub_date, posted, created_at) VALUES (?, ?, ?, ?, 1, datetime('now', '-5 days'))",
        )
        .bind("Recent Article")
        .bind("https://example.com/recent")
        .bind("Recent description")
        .bind("2026-09-25T00:00:00Z")
        .execute(&pool)
        .await
        .expect("Failed to insert recent item");

        let pruned = db::prune_old_items(&pool, 60).await.expect("Failed to prune");
        assert_eq!(pruned, 1);

        let remaining = db::count_items(&pool).await.expect("Failed to count");
        assert_eq!(remaining, 1);
    }
}
