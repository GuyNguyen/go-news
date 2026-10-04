use go_news_shared::{ItemDeliveryStatus, RssItem, SubscriptionItem};
use log::info;
use sqlx::SqlitePool;

pub async fn get_all_items(pool: &SqlitePool) -> Result<Vec<RssItem>, sqlx::Error> {
    sqlx::query_as::<_, RssItem>(
        "SELECT title, link, description, pub_date, posted FROM items ORDER BY id DESC",
    )
    .fetch_all(pool)
    .await
}

pub async fn get_unposted_items(pool: &SqlitePool) -> Result<Vec<RssItem>, sqlx::Error> {
    sqlx::query_as::<_, RssItem>(
        "SELECT title, link, description, pub_date, posted FROM items WHERE posted = 0 ORDER BY id ASC",
    )
    .fetch_all(pool)
    .await
}

pub async fn get_unposted_items_for_channel(
    pool: &SqlitePool,
    channel_id: &str,
) -> Result<Vec<RssItem>, sqlx::Error> {
    sqlx::query_as::<_, RssItem>(
        r#"
        SELECT title, link, description, pub_date, posted
        FROM items
        WHERE posted = 0
          AND NOT EXISTS (
              SELECT 1 FROM item_deliveries
              WHERE item_deliveries.channel_id = ?
                AND item_deliveries.item_link = items.link
          )
        ORDER BY id ASC
        "#,
    )
    .bind(channel_id)
    .fetch_all(pool)
    .await
}

pub async fn mark_items_posted(pool: &SqlitePool, links: &[String]) -> Result<u64, sqlx::Error> {
    if links.is_empty() {
        return Ok(0);
    }

    let mut tx = pool.begin().await?;
    let mut updated_count: u64 = 0;

    for link in links {
        let result = sqlx::query("UPDATE items SET posted = 1 WHERE link = ? AND posted = 0")
            .bind(link)
            .execute(&mut *tx)
            .await?;
        updated_count += result.rows_affected();
    }

    tx.commit().await?;
    Ok(updated_count)
}

pub async fn mark_items_posted_for_channel(
    pool: &SqlitePool,
    links: &[String],
    channel_id: &str,
) -> Result<u64, sqlx::Error> {
    if links.is_empty() {
        return Ok(0);
    }

    let mut tx = pool.begin().await?;

    // Ensure subscription exists for this channel
    sqlx::query("INSERT OR IGNORE INTO subscriptions (channel_id) VALUES (?)")
        .bind(channel_id)
        .execute(&mut *tx)
        .await?;

    let mut updated_count: u64 = 0;
    for link in links {
        let result = sqlx::query(
            "INSERT OR IGNORE INTO item_deliveries (channel_id, item_link) VALUES (?, ?)",
        )
        .bind(channel_id)
        .bind(link)
        .execute(&mut *tx)
        .await?;
        updated_count += result.rows_affected();
    }

    tx.commit().await?;
    Ok(updated_count)
}

pub async fn init_posted_items_for_channel(
    pool: &SqlitePool,
    channel_id: &str,
) -> Result<u64, sqlx::Error> {
    let mut tx = pool.begin().await?;

    sqlx::query("INSERT OR IGNORE INTO subscriptions (channel_id) VALUES (?)")
        .bind(channel_id)
        .execute(&mut *tx)
        .await?;

    let result = sqlx::query(
        "INSERT OR IGNORE INTO item_deliveries (channel_id, item_link) SELECT ?, link FROM items",
    )
    .bind(channel_id)
    .execute(&mut *tx)
    .await?;

    let count = result.rows_affected();
    tx.commit().await?;
    Ok(count)
}

pub async fn add_subscription(
    pool: &SqlitePool,
    channel_id: &str,
    guild_id: Option<&str>,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT INTO subscriptions (channel_id, guild_id) VALUES (?, ?) ON CONFLICT(channel_id) DO UPDATE SET guild_id = coalesce(excluded.guild_id, subscriptions.guild_id)",
    )
    .bind(channel_id)
    .bind(guild_id)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn remove_subscription(
    pool: &SqlitePool,
    channel_id: &str,
) -> Result<bool, sqlx::Error> {
    let mut tx = pool.begin().await?;
    sqlx::query("DELETE FROM item_deliveries WHERE channel_id = ?")
        .bind(channel_id)
        .execute(&mut *tx)
        .await?;
    let result = sqlx::query("DELETE FROM subscriptions WHERE channel_id = ?")
        .bind(channel_id)
        .execute(&mut *tx)
        .await?;
    tx.commit().await?;
    Ok(result.rows_affected() > 0)
}

pub async fn get_subscriptions(
    pool: &SqlitePool,
) -> Result<Vec<SubscriptionItem>, sqlx::Error> {
    sqlx::query_as::<_, SubscriptionItem>(
        r#"
        SELECT s.channel_id, s.guild_id, s.created_at,
               COUNT(d.item_link) as delivered_count,
               MAX(d.delivered_at) as last_delivered_at
        FROM subscriptions s
        LEFT JOIN item_deliveries d ON s.channel_id = d.channel_id
        GROUP BY s.channel_id
        ORDER BY s.created_at DESC
        "#,
    )
    .fetch_all(pool)
    .await
}

pub async fn get_delivery_statuses(
    pool: &SqlitePool,
    limit: i64,
) -> Result<Vec<ItemDeliveryStatus>, sqlx::Error> {
    let all_channels: Vec<String> = sqlx::query_scalar(
        "SELECT channel_id FROM subscriptions ORDER BY channel_id ASC",
    )
    .fetch_all(pool)
    .await?;

    let items = sqlx::query_as::<_, RssItem>(
        "SELECT title, link, description, pub_date, posted FROM items ORDER BY id DESC LIMIT ?",
    )
    .bind(limit)
    .fetch_all(pool)
    .await?;

    let mut statuses = Vec::with_capacity(items.len());
    for item in items {
        let delivered_channels: Vec<String> = sqlx::query_scalar(
            "SELECT channel_id FROM item_deliveries WHERE item_link = ? ORDER BY channel_id ASC",
        )
        .bind(&item.link)
        .fetch_all(pool)
        .await?;

        let pending_channels: Vec<String> = all_channels
            .iter()
            .filter(|cid| !delivered_channels.contains(cid))
            .cloned()
            .collect();

        statuses.push(ItemDeliveryStatus {
            title: item.title,
            link: item.link,
            pub_date: item.pub_date,
            delivered_channels,
            pending_channels,
        });
    }

    Ok(statuses)
}

pub async fn insert_item_if_new(
    pool: &SqlitePool,
    title: &str,
    link: &str,
    description: &str,
    pub_date: &str,
    posted: bool,
) -> Result<bool, sqlx::Error> {
    let posted_val = if posted { 1 } else { 0 };
    let result = sqlx::query(
        "INSERT OR IGNORE INTO items (title, link, description, pub_date, posted) VALUES (?, ?, ?, ?, ?)",
    )
    .bind(title)
    .bind(link)
    .bind(description)
    .bind(pub_date)
    .bind(posted_val)
    .execute(pool)
    .await?;

    Ok(result.rows_affected() > 0)
}

pub async fn prune_old_items(pool: &SqlitePool, retention_days: i64) -> Result<u64, sqlx::Error> {
    let mut tx = pool.begin().await?;
    let result = sqlx::query(
        "DELETE FROM items WHERE (posted = 1 OR EXISTS (SELECT 1 FROM item_deliveries WHERE item_deliveries.item_link = items.link)) AND created_at < datetime('now', ? || ' days')",
    )
    .bind(format!("-{}", retention_days))
    .execute(&mut *tx)
    .await?;

    let pruned = result.rows_affected();
    if pruned > 0 {
        sqlx::query("DELETE FROM item_deliveries WHERE item_link NOT IN (SELECT link FROM items)")
            .execute(&mut *tx)
            .await?;
        info!("Pruned {} posted items older than {} days.", pruned, retention_days);
    }
    tx.commit().await?;
    Ok(pruned)
}

pub async fn count_items(pool: &SqlitePool) -> Result<i64, sqlx::Error> {
    let count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM items")
        .fetch_one(pool)
        .await?;
    Ok(count.0)
}

#[allow(dead_code)]
pub async fn count_unposted_items(pool: &SqlitePool) -> Result<i64, sqlx::Error> {
    let count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM items WHERE posted = 0")
        .fetch_one(pool)
        .await?;
    Ok(count.0)
}
