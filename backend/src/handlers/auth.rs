use actix_web::{web, HttpResponse, Result};
use shared::{ApiError, ApiSuccess, AuthResponse, CreateUserRequest, LoginRequest};

use crate::models::AppState;
use crate::services::auth as auth_service;

pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/auth")
            .route("/register", web::post().to(register))
            .route("/login", web::post().to(login))
            .route("/me", web::get().to(get_current_user))
    );
}

async fn register(
    state: web::Data<AppState>,
    body: web::Json<CreateUserRequest>,
) -> Result<HttpResponse> {
    let request = body.into_inner();

    // Validate input
    if request.username.is_empty() || request.email.is_empty() || request.password.is_empty() {
        return Ok(HttpResponse::BadRequest().json(ApiError {
            error: "validation_error".to_string(),
            message: "Username, email, and password are required".to_string(),
        }));
    }

    if request.password.len() < 8 {
        return Ok(HttpResponse::BadRequest().json(ApiError {
            error: "validation_error".to_string(),
            message: "Password must be at least 8 characters".to_string(),
        }));
    }

    match auth_service::register_user(&state.db, &request).await {
        Ok(user) => {
            match auth_service::create_jwt(&user.id, &state.config.jwt_secret, state.config.jwt_expiration_hours) {
                Ok(token) => Ok(HttpResponse::Created().json(ApiSuccess::new(AuthResponse { token, user }))),
                Err(e) => {
                    log::error!("JWT creation error: {:?}", e);
                    Ok(HttpResponse::InternalServerError().json(ApiError {
                        error: "jwt_error".to_string(),
                        message: "Failed to create token".to_string(),
                    }))
                }
            }
        }
        Err(e) => {
            log::error!("Registration error: {:?}", e);
            Ok(HttpResponse::BadRequest().json(ApiError {
                error: "registration_error".to_string(),
                message: e.to_string(),
            }))
        }
    }
}

async fn login(
    state: web::Data<AppState>,
    body: web::Json<LoginRequest>,
) -> Result<HttpResponse> {
    let request = body.into_inner();

    match auth_service::login_user(&state.db, &request).await {
        Ok(user) => {
            match auth_service::create_jwt(&user.id, &state.config.jwt_secret, state.config.jwt_expiration_hours) {
                Ok(token) => Ok(HttpResponse::Ok().json(ApiSuccess::new(AuthResponse { token, user }))),
                Err(e) => {
                    log::error!("JWT creation error: {:?}", e);
                    Ok(HttpResponse::InternalServerError().json(ApiError {
                        error: "jwt_error".to_string(),
                        message: "Failed to create token".to_string(),
                    }))
                }
            }
        }
        Err(e) => {
            log::error!("Login error: {:?}", e);
            Ok(HttpResponse::Unauthorized().json(ApiError {
                error: "authentication_error".to_string(),
                message: "Invalid username or password".to_string(),
            }))
        }
    }
}

async fn get_current_user(
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
