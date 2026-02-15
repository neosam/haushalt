use actix_web::web;

pub mod auth;
pub mod users;
pub mod households;
pub mod tasks;
pub mod rewards;
pub mod punishments;
pub mod point_conditions;

pub fn configure_routes(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/api")
            .configure(auth::configure)
            .configure(users::configure)
            .configure(households::configure)
    );
}
