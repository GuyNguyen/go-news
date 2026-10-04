pub mod repository;

pub use repository::*;

use sqlx::sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions, SqliteSynchronous};
use sqlx::SqlitePool;
use std::str::FromStr;
use std::time::Duration;

pub async fn create_pool(database_url: &str) -> Result<SqlitePool, sqlx::Error> {
    if let Some(path_str) = database_url
        .strip_prefix("sqlite://")
        .or_else(|| database_url.strip_prefix("sqlite:"))
    {
        let clean_path = path_str.split('?').next().unwrap_or(path_str);
        if clean_path != ":memory:" {
            let path = std::path::Path::new(clean_path);
            if let Some(parent) = path.parent() {
                if !parent.as_os_str().is_empty() {
                    let _ = std::fs::create_dir_all(parent);
                }
            }
        }
    }

    let connection_options = SqliteConnectOptions::from_str(database_url)?
        .create_if_missing(true)
        .journal_mode(SqliteJournalMode::Wal)
        .synchronous(SqliteSynchronous::Normal)
        .busy_timeout(Duration::from_secs(5));

    let pool = SqlitePoolOptions::new()
        .max_connections(5)
        .connect_with(connection_options)
        .await?;

    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS items (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            title TEXT NOT NULL,
            link TEXT NOT NULL UNIQUE,
            description TEXT NOT NULL DEFAULT '',
            pub_date TEXT NOT NULL DEFAULT '',
            posted INTEGER NOT NULL DEFAULT 0,
            created_at DATETIME DEFAULT CURRENT_TIMESTAMP
        );
        CREATE INDEX IF NOT EXISTS idx_items_posted ON items(posted);
        CREATE UNIQUE INDEX IF NOT EXISTS idx_items_link ON items(link);

        CREATE TABLE IF NOT EXISTS subscriptions (
            channel_id TEXT PRIMARY KEY,
            guild_id TEXT,
            created_at DATETIME DEFAULT CURRENT_TIMESTAMP
        );

        CREATE TABLE IF NOT EXISTS item_deliveries (
            channel_id TEXT NOT NULL,
            item_link TEXT NOT NULL,
            delivered_at DATETIME DEFAULT CURRENT_TIMESTAMP,
            PRIMARY KEY (channel_id, item_link)
        );
        CREATE INDEX IF NOT EXISTS idx_item_deliveries_link ON item_deliveries(item_link);
        CREATE INDEX IF NOT EXISTS idx_item_deliveries_channel ON item_deliveries(channel_id);
        "#,
    )
    .execute(&pool)
    .await?;

    Ok(pool)
}
