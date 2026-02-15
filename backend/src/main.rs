use actix_cors::Cors;
use actix_files::{Files, NamedFile};
use actix_web::{middleware::Logger, web, App, HttpServer};
use sqlx::sqlite::SqlitePoolOptions;
use std::sync::Arc;

mod config;
mod db;
mod handlers;
mod middleware;
mod models;
mod services;

use config::Config;

async fn index(config: web::Data<models::AppState>) -> actix_web::Result<NamedFile> {
    let static_path = config.config.static_files_path.as_deref().unwrap_or("./static");
    Ok(NamedFile::open(format!("{}/index.html", static_path))?)
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    // Load .env file if present
    dotenvy::dotenv().ok();

    // Initialize logger
    env_logger::init_from_env(env_logger::Env::default().default_filter_or("info"));

    // Load configuration
    let config = Config::from_env().expect("Failed to load configuration");

    log::info!("Starting server at {}:{}", config.host, config.port);

    if let Some(ref path) = config.static_files_path {
        log::info!("Serving static files from: {}", path);
    }

    // Create database pool
    let pool = SqlitePoolOptions::new()
        .max_connections(5)
        .connect(&config.database_url)
        .await
        .expect("Failed to create database pool");

    // Run migrations
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("Failed to run migrations");

    log::info!("Database migrations completed");

    // Start background job scheduler
    let pool_for_scheduler = Arc::new(pool.clone());
    tokio::spawn(async move {
        services::background_jobs::start_scheduler(
            pool_for_scheduler,
            services::background_jobs::JobConfig::default(),
        )
        .await;
    });
    log::info!("Background job scheduler started");

    // Create WebSocket manager
    let ws_manager = services::websocket::WsManager::new();
    let ws_manager_data = web::Data::new(ws_manager);

    // Create rate limiter for login (5 attempts per 15 minutes)
    let login_rate_limiter = Arc::new(middleware::RateLimiter::new(5, 15 * 60));

    // Create app state
    let app_state = web::Data::new(models::AppState {
        db: pool.clone(),
        config: config.clone(),
        login_rate_limiter,
    });

    // Create pool and config data for WebSocket handler
    let pool_data = web::Data::new(pool);
    let config_data = web::Data::new(config.clone());

    let static_files_path = config.static_files_path.clone();

    // Start HTTP server
    HttpServer::new(move || {
        let ws_manager = ws_manager_data.clone();
        let pool = pool_data.clone();
        let config = config_data.clone();
        let allowed_origins = config.cors_origins.clone();
        let cors = Cors::default()
            .allowed_origin_fn(move |origin, _req_head| {
                let origin_str = origin.to_str().unwrap_or("");
                allowed_origins.iter().any(|allowed| {
                    origin_str.starts_with(allowed)
                })
            })
            .allowed_methods(vec!["GET", "POST", "PUT", "DELETE", "OPTIONS"])
            .allowed_headers(vec!["Authorization", "Content-Type"])
            .max_age(3600);

        let mut app = App::new()
            .app_data(app_state.clone())
            .app_data(ws_manager.clone())
            .app_data(pool.clone())
            .app_data(config.clone())
            .wrap(Logger::default())
            .wrap(cors)
            .configure(handlers::configure_routes)
            .configure(handlers::websocket::configure);

        // Serve static files if path is configured
        if let Some(ref path) = static_files_path {
            app = app
                .service(Files::new("/pkg", format!("{}/pkg", path)))
                .service(Files::new("/assets", format!("{}/assets", path)).show_files_listing())
                .default_service(web::route().to(index));
        }

        app
    })
    .bind((config.host.as_str(), config.port))?
    .run()
    .await
}
