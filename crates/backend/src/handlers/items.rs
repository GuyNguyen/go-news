use actix_web::{get, post, web, HttpResponse, Responder};
use go_news_shared::MarkPostedRequest;
use log::{error, info};
use sqlx::SqlitePool;

use crate::config::AppConfig;
use crate::db;
use crate::feed;

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
) -> impl Responder {
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
pub async fn get_unposted_items(db_pool: web::Data<SqlitePool>) -> impl Responder {
    info!("GET /items/unposted endpoint called.");
    match db::get_unposted_items(db_pool.get_ref()).await {
        Ok(items) => HttpResponse::Ok().json(items),
        Err(e) => {
            error!("Failed to fetch unposted items from database: {}", e);
            HttpResponse::InternalServerError().finish()
        }
    }
}

#[post("/items/mark-posted")]
pub async fn mark_items_posted(
    db_pool: web::Data<SqlitePool>,
    req: web::Json<MarkPostedRequest>,
) -> impl Responder {
    info!(
        "POST /items/mark-posted endpoint called for {} links.",
        req.links.len()
    );

    if req.links.is_empty() {
        return HttpResponse::Ok().json(serde_json::json!({
            "message": "Update successful",
            "items_updated": 0
        }));
    }

    match db::mark_items_posted(db_pool.get_ref(), &req.links).await {
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
