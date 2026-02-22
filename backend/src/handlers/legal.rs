//! Handlers for legal pages (Impressum, Datenschutz, AGB)
//!
//! These endpoints serve Markdown content from files on the server.
//! The files are located in the directory specified by the LEGAL_DIR environment variable.

use actix_web::{web, HttpResponse, Responder};
use std::path::PathBuf;

use crate::config::Config;

/// Get Impressum content
pub async fn get_impressum(config: web::Data<Config>) -> impl Responder {
    get_legal_file(&config, "impressum.md").await
}

/// Get Datenschutz (Privacy Policy) content
pub async fn get_datenschutz(config: web::Data<Config>) -> impl Responder {
    get_legal_file(&config, "datenschutz.md").await
}

/// Get AGB (Terms of Service) content
pub async fn get_agb(config: web::Data<Config>) -> impl Responder {
    get_legal_file(&config, "agb.md").await
}

async fn get_legal_file(config: &Config, filename: &str) -> HttpResponse {
    let Some(legal_dir) = &config.legal_dir else {
        return HttpResponse::NotFound().json(serde_json::json!({
            "error": "Legal directory not configured"
        }));
    };

    let path: PathBuf = [legal_dir, filename].iter().collect();

    match tokio::fs::read_to_string(&path).await {
        Ok(content) => HttpResponse::Ok()
            .content_type("text/markdown; charset=utf-8")
            .body(content),
        Err(_) => HttpResponse::NotFound().json(serde_json::json!({
            "error": format!("Legal document '{}' not found", filename)
        })),
    }
}

pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/legal")
            .route("/impressum", web::get().to(get_impressum))
            .route("/datenschutz", web::get().to(get_datenschutz))
            .route("/agb", web::get().to(get_agb)),
    );
}
