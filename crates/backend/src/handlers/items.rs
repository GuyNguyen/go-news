use actix_web::{delete, get, post, web, HttpResponse, Responder};
use go_news_shared::{AddSubscriptionRequest, MarkPostedRequest};
use log::{error, info};
use serde::Deserialize;
use sqlx::SqlitePool;

use crate::config::AppConfig;
use crate::db;
use crate::feed;

#[derive(Debug, Deserialize)]
pub struct ChannelFilterQuery {
    pub channel_id: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct LimitQuery {
    pub limit: Option<i64>,
}

#[post("/force-check")]
pub async fn force_check(
    db_pool: web::Data<SqlitePool>,
    config: web::Data<AppConfig>,
) -> impl Responder {
    info!("POST /force-check endpoint called.");
    match feed::fetch_and_store_feed(db_pool.get_ref(), &config.feed_url, config.retention_days).await {
        Ok(count) => HttpResponse::Ok().body(format!("Feed check completed. {} new items.", count)),
        Err(e) => {
            error!("Manual check failed: {}", e);
            HttpResponse::InternalServerError().body(format!("Failed to check feed: {}", e))
        }
    }
}

#[post("/items/init-posted")]
pub async fn init_posted_items(
    db_pool: web::Data<SqlitePool>,
    config: web::Data<AppConfig>,
    query: web::Query<ChannelFilterQuery>,
) -> impl Responder {
    if let Some(channel_id) = &query.channel_id {
        info!("POST /items/init-posted called for channel {}", channel_id);
        match db::init_posted_items_for_channel(db_pool.get_ref(), channel_id).await {
            Ok(count) => HttpResponse::Ok().json(serde_json::json!({
                "message": format!("Initialized feed items as delivered for channel {}", channel_id),
                "channel_id": channel_id,
                "items_initialized": count
            })),
            Err(e) => {
                error!("Failed to initialize feed items for channel {}: {}", channel_id, e);
                HttpResponse::InternalServerError().body(format!("Failed to initialize: {}", e))
            }
        }
    } else {
        info!("POST /items/init-posted called: syncing current RSS feed and marking all as posted.");
        match feed::fetch_and_store_feed_internal(
            db_pool.get_ref(),
            &config.feed_url,
            true,
            config.retention_days,
        )
        .await
        {
            Ok(count) => HttpResponse::Ok().json(serde_json::json!({
                "message": "Initialized feed items as posted",
                "items_initialized": count
            })),
            Err(e) => {
                error!("Failed to initialize feed items as posted: {}", e);
                HttpResponse::InternalServerError().body(format!("Failed to initialize: {}", e))
            }
        }
    }
}

#[get("/items")]
pub async fn get_items(db_pool: web::Data<SqlitePool>) -> impl Responder {
    info!("GET /items endpoint called.");
    match db::get_all_items(db_pool.get_ref()).await {
        Ok(items) => HttpResponse::Ok().json(items),
        Err(e) => {
            error!("Failed to fetch items from database: {}", e);
            HttpResponse::InternalServerError().finish()
        }
    }
}

#[get("/items/unposted")]
pub async fn get_unposted_items(
    db_pool: web::Data<SqlitePool>,
    query: web::Query<ChannelFilterQuery>,
) -> impl Responder {
    if let Some(channel_id) = &query.channel_id {
        info!("GET /items/unposted endpoint called for channel {}.", channel_id);
        match db::get_unposted_items_for_channel(db_pool.get_ref(), channel_id).await {
            Ok(items) => HttpResponse::Ok().json(items),
            Err(e) => {
                error!("Failed to fetch unposted items for channel {}: {}", channel_id, e);
                HttpResponse::InternalServerError().finish()
            }
        }
    } else {
        info!("GET /items/unposted endpoint called (global).");
        match db::get_unposted_items(db_pool.get_ref()).await {
            Ok(items) => HttpResponse::Ok().json(items),
            Err(e) => {
                error!("Failed to fetch unposted items from database: {}", e);
                HttpResponse::InternalServerError().finish()
            }
        }
    }
}

#[post("/items/mark-posted")]
pub async fn mark_items_posted(
    db_pool: web::Data<SqlitePool>,
    req: web::Json<MarkPostedRequest>,
) -> impl Responder {
    info!(
        "POST /items/mark-posted endpoint called for {} links (channel: {:?}).",
        req.links.len(),
        req.channel_id
    );

    if req.links.is_empty() {
        return HttpResponse::Ok().json(serde_json::json!({
            "message": "Update successful",
            "items_updated": 0
        }));
    }

    let res = if let Some(channel_id) = &req.channel_id {
        db::mark_items_posted_for_channel(db_pool.get_ref(), &req.links, channel_id).await
    } else {
        db::mark_items_posted(db_pool.get_ref(), &req.links).await
    };

    match res {
        Ok(updated_count) => {
            info!("Successfully marked {} items as posted.", updated_count);
            HttpResponse::Ok().json(serde_json::json!({
                "message": "Update successful",
                "items_updated": updated_count
            }))
        }
        Err(e) => {
            error!("Failed to mark items as posted: {}", e);
            HttpResponse::InternalServerError().body("Failed to update items")
        }
    }
}

#[get("/items/delivery-status")]
pub async fn get_delivery_status(
    db_pool: web::Data<SqlitePool>,
    query: web::Query<LimitQuery>,
) -> impl Responder {
    let limit = query.limit.unwrap_or(50);
    info!("GET /items/delivery-status called (limit: {}).", limit);
    match db::get_delivery_statuses(db_pool.get_ref(), limit).await {
        Ok(statuses) => HttpResponse::Ok().json(statuses),
        Err(e) => {
            error!("Failed to fetch delivery status: {}", e);
            HttpResponse::InternalServerError().finish()
        }
    }
}

#[get("/subscriptions")]
pub async fn get_subscriptions(db_pool: web::Data<SqlitePool>) -> impl Responder {
    info!("GET /subscriptions called.");
    match db::get_subscriptions(db_pool.get_ref()).await {
        Ok(subs) => HttpResponse::Ok().json(subs),
        Err(e) => {
            error!("Failed to fetch subscriptions: {}", e);
            HttpResponse::InternalServerError().finish()
        }
    }
}

#[post("/subscriptions")]
pub async fn add_subscription(
    db_pool: web::Data<SqlitePool>,
    req: web::Json<AddSubscriptionRequest>,
) -> impl Responder {
    info!("POST /subscriptions called for channel {}.", req.channel_id);
    match db::add_subscription(db_pool.get_ref(), &req.channel_id, req.guild_id.as_deref()).await {
        Ok(_) => HttpResponse::Ok().json(serde_json::json!({
            "message": "Subscription registered",
            "channel_id": req.channel_id,
            "guild_id": req.guild_id
        })),
        Err(e) => {
            error!("Failed to add subscription: {}", e);
            HttpResponse::InternalServerError().body(format!("Failed to add subscription: {}", e))
        }
    }
}

#[delete("/subscriptions/{channel_id}")]
pub async fn remove_subscription(
    db_pool: web::Data<SqlitePool>,
    path: web::Path<String>,
) -> impl Responder {
    let channel_id = path.into_inner();
    info!("DELETE /subscriptions/{} called.", channel_id);
    match db::remove_subscription(db_pool.get_ref(), &channel_id).await {
        Ok(removed) => HttpResponse::Ok().json(serde_json::json!({
            "message": if removed { "Subscription deleted" } else { "Subscription not found" },
            "deleted": removed
        })),
        Err(e) => {
            error!("Failed to remove subscription: {}", e);
            HttpResponse::InternalServerError().body(format!("Failed to remove subscription: {}", e))
        }
    }
}
