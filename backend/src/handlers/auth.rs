use actix_web::{web, HttpResponse, Result};
use shared::{ApiError, ApiSuccess, AuthResponse, CreateUserRequest, LoginRequest, RefreshTokenRequest};

use crate::models::AppState;
use crate::services::auth as auth_service;

pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/auth")
            .route("/register", web::post().to(register))
            .route("/login", web::post().to(login))
            .route("/refresh", web::post().to(refresh))
            .route("/logout", web::post().to(logout))
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
            // Create access token
            let token = match auth_service::create_access_token(
                &user.id,
                &state.config.jwt_secret,
                state.config.access_token_expiration_minutes,
            ) {
                Ok(t) => t,
                Err(e) => {
                    log::error!("JWT creation error: {:?}", e);
                    return Ok(HttpResponse::InternalServerError().json(ApiError {
                        error: "jwt_error".to_string(),
                        message: "Failed to create token".to_string(),
                    }));
                }
            };

            // Create refresh token
            let refresh_token = match auth_service::create_refresh_token(
                &state.db,
                &user.id,
                state.config.refresh_token_expiration_days,
            )
            .await
            {
                Ok(t) => t,
                Err(e) => {
                    log::error!("Refresh token creation error: {:?}", e);
                    return Ok(HttpResponse::InternalServerError().json(ApiError {
                        error: "token_error".to_string(),
                        message: "Failed to create refresh token".to_string(),
                    }));
                }
            };

            Ok(HttpResponse::Created().json(ApiSuccess::new(AuthResponse {
                token,
                refresh_token,
                user,
            })))
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
    req: actix_web::HttpRequest,
) -> Result<HttpResponse> {
    let request = body.into_inner();

    // Get client IP for rate limiting
    let client_ip = req
        .connection_info()
        .realip_remote_addr()
        .unwrap_or("unknown")
        .to_string();

    // Check rate limit
    if !state.login_rate_limiter.check(&client_ip) {
        return Ok(HttpResponse::TooManyRequests().json(ApiError {
            error: "rate_limited".to_string(),
            message: "Too many login attempts. Please try again later.".to_string(),
        }));
    }

    match auth_service::login_user(&state.db, &request).await {
        Ok(user) => {
            // Create access token
            let token = match auth_service::create_access_token(
                &user.id,
                &state.config.jwt_secret,
                state.config.access_token_expiration_minutes,
            ) {
                Ok(t) => t,
                Err(e) => {
                    log::error!("JWT creation error: {:?}", e);
                    return Ok(HttpResponse::InternalServerError().json(ApiError {
                        error: "jwt_error".to_string(),
                        message: "Failed to create token".to_string(),
                    }));
                }
            };

            // Create refresh token
            let refresh_token = match auth_service::create_refresh_token(
                &state.db,
                &user.id,
                state.config.refresh_token_expiration_days,
            )
            .await
            {
                Ok(t) => t,
                Err(e) => {
                    log::error!("Refresh token creation error: {:?}", e);
                    return Ok(HttpResponse::InternalServerError().json(ApiError {
                        error: "token_error".to_string(),
                        message: "Failed to create refresh token".to_string(),
                    }));
                }
            };

            Ok(HttpResponse::Ok().json(ApiSuccess::new(AuthResponse {
                token,
                refresh_token,
                user,
            })))
        }
        Err(e) => {
            // Record failed attempt for rate limiting
            state.login_rate_limiter.record(&client_ip);

            log::error!("Login error: {:?}", e);
            Ok(HttpResponse::Unauthorized().json(ApiError {
                error: "authentication_error".to_string(),
                message: "Invalid username or password".to_string(),
            }))
        }
    }
}

async fn refresh(
    state: web::Data<AppState>,
    body: web::Json<RefreshTokenRequest>,
) -> Result<HttpResponse> {
    let request = body.into_inner();

    match auth_service::refresh_tokens(
        &state.db,
        &request.refresh_token,
        &state.config.jwt_secret,
        state.config.access_token_expiration_minutes,
        state.config.refresh_token_expiration_days,
    )
    .await
    {
        Ok((token, refresh_token, user)) => {
            Ok(HttpResponse::Ok().json(ApiSuccess::new(AuthResponse {
                token,
                refresh_token,
                user,
            })))
        }
        Err(auth_service::AuthError::InvalidRefreshToken) => {
            Ok(HttpResponse::Unauthorized().json(ApiError {
                error: "invalid_token".to_string(),
                message: "Invalid refresh token".to_string(),
            }))
        }
        Err(auth_service::AuthError::RefreshTokenExpired) => {
            Ok(HttpResponse::Unauthorized().json(ApiError {
                error: "token_expired".to_string(),
                message: "Refresh token has expired".to_string(),
            }))
        }
        Err(e) => {
            log::error!("Refresh token error: {:?}", e);
            Ok(HttpResponse::InternalServerError().json(ApiError {
                error: "refresh_error".to_string(),
                message: "Failed to refresh token".to_string(),
            }))
        }
    }
}

async fn logout(
    state: web::Data<AppState>,
    body: web::Json<RefreshTokenRequest>,
) -> Result<HttpResponse> {
    let request = body.into_inner();

    // Delete the refresh token from database
    if let Err(e) = auth_service::delete_refresh_token(&state.db, &request.refresh_token).await {
        log::error!("Logout error: {:?}", e);
        // Don't return error to client - logout should always succeed from client perspective
    }

    Ok(HttpResponse::Ok().json(ApiSuccess::new("Logged out successfully")))
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
