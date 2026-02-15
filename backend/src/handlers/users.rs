use actix_web::{web, HttpResponse, Result};
use shared::{ApiError, ApiSuccess, UpdateUserRequest, UpdateUserSettingsRequest};
use uuid::Uuid;

use crate::models::AppState;
use crate::services::auth as auth_service;
use crate::services::user_settings as settings_service;

pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/users")
            // User settings routes must come before /{id} to avoid matching "me" as an id
            .route("/me/settings", web::get().to(get_user_settings))
            .route("/me/settings", web::put().to(update_user_settings))
            .route("/{id}", web::get().to(get_user))
            .route("/{id}", web::put().to(update_user))
    );
}

async fn get_user(
    state: web::Data<AppState>,
    path: web::Path<String>,
) -> Result<HttpResponse> {
    let user_id = match Uuid::parse_str(&path.into_inner()) {
        Ok(id) => id,
        Err(_) => {
            return Ok(HttpResponse::BadRequest().json(ApiError {
                error: "invalid_id".to_string(),
                message: "Invalid user ID format".to_string(),
            }));
        }
    };

    match auth_service::get_user_by_id(&state.db, &user_id).await {
        Ok(Some(user)) => Ok(HttpResponse::Ok().json(ApiSuccess::new(user))),
        Ok(None) => Ok(HttpResponse::NotFound().json(ApiError {
            error: "not_found".to_string(),
            message: "User not found".to_string(),
        })),
        Err(e) => {
            log::error!("Error fetching user: {:?}", e);
            Ok(HttpResponse::InternalServerError().json(ApiError {
                error: "internal_error".to_string(),
                message: "Failed to fetch user".to_string(),
            }))
        }
    }
}

async fn update_user(
    state: web::Data<AppState>,
    req: actix_web::HttpRequest,
    path: web::Path<String>,
    body: web::Json<UpdateUserRequest>,
) -> Result<HttpResponse> {
    let current_user_id = match crate::middleware::auth::extract_user_id(&req, &state.config.jwt_secret) {
        Ok(id) => id,
        Err(_) => {
            return Ok(HttpResponse::Unauthorized().json(ApiError {
                error: "unauthorized".to_string(),
                message: "Invalid or missing token".to_string(),
            }));
        }
    };

    let target_user_id = match Uuid::parse_str(&path.into_inner()) {
        Ok(id) => id,
        Err(_) => {
            return Ok(HttpResponse::BadRequest().json(ApiError {
                error: "invalid_id".to_string(),
                message: "Invalid user ID format".to_string(),
            }));
        }
    };

    // Users can only update their own profile
    if current_user_id != target_user_id {
        return Ok(HttpResponse::Forbidden().json(ApiError {
            error: "forbidden".to_string(),
            message: "You can only update your own profile".to_string(),
        }));
    }

    match auth_service::update_user(&state.db, &target_user_id, &body.into_inner()).await {
        Ok(user) => Ok(HttpResponse::Ok().json(ApiSuccess::new(user))),
        Err(e) => {
            log::error!("Error updating user: {:?}", e);
            Ok(HttpResponse::InternalServerError().json(ApiError {
                error: "internal_error".to_string(),
                message: e.to_string(),
            }))
        }
    }
}

async fn get_user_settings(
    state: web::Data<AppState>,
    req: actix_web::HttpRequest,
) -> Result<HttpResponse> {
    let user_id = match crate::middleware::auth::extract_user_id(&req, &state.config.jwt_secret) {
        Ok(id) => id,
        Err(_) => {
            return Ok(HttpResponse::Unauthorized().json(ApiError {
                error: "unauthorized".to_string(),
                message: "Invalid or missing token".to_string(),
            }));
        }
    };

    match settings_service::get_or_create_settings(&state.db, &user_id).await {
        Ok(settings) => Ok(HttpResponse::Ok().json(ApiSuccess::new(settings))),
        Err(e) => {
            log::error!("Error fetching user settings: {:?}", e);
            Ok(HttpResponse::InternalServerError().json(ApiError {
                error: "internal_error".to_string(),
                message: "Failed to fetch user settings".to_string(),
            }))
        }
    }
}

async fn update_user_settings(
    state: web::Data<AppState>,
    req: actix_web::HttpRequest,
    body: web::Json<UpdateUserSettingsRequest>,
) -> Result<HttpResponse> {
    let user_id = match crate::middleware::auth::extract_user_id(&req, &state.config.jwt_secret) {
        Ok(id) => id,
        Err(_) => {
            return Ok(HttpResponse::Unauthorized().json(ApiError {
                error: "unauthorized".to_string(),
                message: "Invalid or missing token".to_string(),
            }));
        }
    };

    match settings_service::update_settings(&state.db, &user_id, &body.into_inner()).await {
        Ok(settings) => Ok(HttpResponse::Ok().json(ApiSuccess::new(settings))),
        Err(settings_service::UserSettingsError::InvalidLanguage) => {
            Ok(HttpResponse::BadRequest().json(ApiError {
                error: "validation_error".to_string(),
                message: "Invalid language code. Supported: en, de".to_string(),
            }))
        }
        Err(e) => {
            log::error!("Error updating user settings: {:?}", e);
            Ok(HttpResponse::InternalServerError().json(ApiError {
                error: "internal_error".to_string(),
                message: "Failed to update user settings".to_string(),
            }))
        }
    }
}
