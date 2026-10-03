use go_news_shared::RssItem;
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
    let result = sqlx::query(
        "DELETE FROM items WHERE posted = 1 AND created_at < datetime('now', ? || ' days')",
    )
    .bind(format!("-{}", retention_days))
    .execute(pool)
    .await?;

    let pruned = result.rows_affected();
    if pruned > 0 {
        info!("Pruned {} posted items older than {} days.", pruned, retention_days);
    }
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
