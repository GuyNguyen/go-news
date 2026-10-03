use actix_web::{get, HttpResponse, Responder};
use log::info;

#[get("/health")]
pub async fn health_check() -> impl Responder {
    info!("GET /health endpoint called.");
    HttpResponse::Ok().body("Service is running")
}
