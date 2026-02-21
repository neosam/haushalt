use actix_web::web;

pub mod auth;
pub mod users;
pub mod households;
pub mod tasks;
pub mod task_categories;
pub mod rewards;
pub mod punishments;
pub mod point_conditions;
pub mod invitations;
pub mod activity_logs;
pub mod chat;
pub mod websocket;
pub mod notes;
pub mod journal;
pub mod announcements;
pub mod dashboard;
pub mod statistics;

pub fn configure_routes(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/api")
            .configure(auth::configure)
            .configure(users::configure)
            .configure(households::configure)
            .configure(invitations::configure)
            .configure(dashboard::configure)
    );
}
