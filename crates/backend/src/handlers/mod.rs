pub mod health;
pub mod items;

use actix_web::web;

pub fn configure_routes(cfg: &mut web::ServiceConfig) {
    cfg.service(health::health_check)
        .service(items::force_check)
        .service(items::init_posted_items)
        .service(items::get_items)
        .service(items::get_unposted_items)
        .service(items::mark_items_posted);
}
