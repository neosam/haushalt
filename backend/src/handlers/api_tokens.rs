//! Authenticated management of a user's own API tokens.
//!
//! Registered under `/api/users/me/api-tokens`. Being user-scoped, this surface is reachable
//! ONLY with a JWT — an API token cannot reach `/users/me` at all (the auth middleware lives
//! on `/households`), so a token can never mint or manage further tokens.
//!
//! The handlers stay thin; all logic lives in `services::api_tokens`.

use actix_web::{web, HttpResponse, Result};
use shared::{ApiError, ApiSuccess, ApiTokensResponse, CreateApiTokenRequest, UpdateApiTokenRequest};
use uuid::Uuid;

use crate::models::AppState;
use crate::services::api_tokens::{self as service, ApiTokenError};

/// Registered inside the `/users` scope BEFORE its `/{id}` routes, so "me" is never parsed
/// as a user id — exactly as the public report routes are.
pub fn configure_user_routes(cfg: &mut web::ServiceConfig) {
    cfg.route("/me/api-tokens", web::get().to(list_tokens))
        .route("/me/api-tokens", web::post().to(create_token))
        .route("/me/api-tokens/{token_id}", web::get().to(get_token))
        .route("/me/api-tokens/{token_id}", web::put().to(update_token))
        .route("/me/api-tokens/{token_id}", web::delete().to(delete_token));
}

fn unauthorized() -> HttpResponse {
    HttpResponse::Unauthorized().json(ApiError {
        error: "unauthorized".to_string(),
        message: "Invalid or missing token".to_string(),
    })
}

fn require_user(
    state: &web::Data<AppState>,
    req: &actix_web::HttpRequest,
) -> std::result::Result<Uuid, HttpResponse> {
    crate::middleware::auth::extract_user_id(req, &state.config.jwt_secret).map_err(|_| unauthorized())
}

/// Map a service error onto HTTP. `NotFound` covers both "does not exist" and "belongs to
/// somebody else", so ownership is never revealed.
fn map_error(error: ApiTokenError) -> HttpResponse {
    match error {
        ApiTokenError::NotFound => HttpResponse::NotFound().json(ApiError {
            error: "not_found".to_string(),
            message: "Token not found".to_string(),
        }),
        ApiTokenError::InvalidName => HttpResponse::BadRequest().json(ApiError {
            error: "invalid_name".to_string(),
            message: "Token name must not be empty and at most 100 characters".to_string(),
        }),
        ApiTokenError::NotAMemberOfHousehold(household_id) => HttpResponse::Forbidden().json(ApiError {
            error: "forbidden".to_string(),
            message: format!("Not a member of household {household_id}"),
        }),
        ApiTokenError::TooManyTokens => HttpResponse::BadRequest().json(ApiError {
            error: "too_many_tokens".to_string(),
            message: "Token limit reached".to_string(),
        }),
        other => {
            log::error!("API token error: {other:?}");
            HttpResponse::InternalServerError().json(ApiError {
                error: "internal_error".to_string(),
                message: "Failed to process token".to_string(),
            })
        }
    }
}

/// A malformed id is answered like a missing one, for the same reason the service hides
/// ownership behind `NotFound`.
fn parse_token_id(raw: &str) -> std::result::Result<Uuid, HttpResponse> {
    Uuid::parse_str(raw).map_err(|_| {
        HttpResponse::NotFound().json(ApiError {
            error: "not_found".to_string(),
            message: "Token not found".to_string(),
        })
    })
}

async fn list_tokens(state: web::Data<AppState>, req: actix_web::HttpRequest) -> Result<HttpResponse> {
    let user_id = match require_user(&state, &req) {
        Ok(id) => id,
        Err(response) => return Ok(response),
    };

    Ok(match service::list_tokens(&state.db, &user_id).await {
        Ok(tokens) => HttpResponse::Ok().json(ApiSuccess::new(ApiTokensResponse { tokens })),
        Err(e) => map_error(e),
    })
}

async fn get_token(
    state: web::Data<AppState>,
    req: actix_web::HttpRequest,
    path: web::Path<String>,
) -> Result<HttpResponse> {
    let user_id = match require_user(&state, &req) {
        Ok(id) => id,
        Err(response) => return Ok(response),
    };
    let token_id = match parse_token_id(&path.into_inner()) {
        Ok(id) => id,
        Err(response) => return Ok(response),
    };

    Ok(match service::get_token(&state.db, &token_id, &user_id).await {
        Ok(token) => HttpResponse::Ok().json(ApiSuccess::new(token)),
        Err(e) => map_error(e),
    })
}

/// The plaintext secret is in the 201 response body — the ONLY time it is ever returned.
async fn create_token(
    state: web::Data<AppState>,
    req: actix_web::HttpRequest,
    body: web::Json<CreateApiTokenRequest>,
) -> Result<HttpResponse> {
    let user_id = match require_user(&state, &req) {
        Ok(id) => id,
        Err(response) => return Ok(response),
    };

    Ok(
        match service::create_token(&state.db, &user_id, body.into_inner()).await {
            Ok(created) => HttpResponse::Created().json(ApiSuccess::new(created)),
            Err(e) => map_error(e),
        },
    )
}

async fn update_token(
    state: web::Data<AppState>,
    req: actix_web::HttpRequest,
    path: web::Path<String>,
    body: web::Json<UpdateApiTokenRequest>,
) -> Result<HttpResponse> {
    let user_id = match require_user(&state, &req) {
        Ok(id) => id,
        Err(response) => return Ok(response),
    };
    let token_id = match parse_token_id(&path.into_inner()) {
        Ok(id) => id,
        Err(response) => return Ok(response),
    };

    Ok(
        match service::update_token(&state.db, &token_id, &user_id, body.into_inner()).await {
            Ok(token) => HttpResponse::Ok().json(ApiSuccess::new(token)),
            Err(e) => map_error(e),
        },
    )
}

async fn delete_token(
    state: web::Data<AppState>,
    req: actix_web::HttpRequest,
    path: web::Path<String>,
) -> Result<HttpResponse> {
    let user_id = match require_user(&state, &req) {
        Ok(id) => id,
        Err(response) => return Ok(response),
    };
    let token_id = match parse_token_id(&path.into_inner()) {
        Ok(id) => id,
        Err(response) => return Ok(response),
    };

    Ok(match service::delete_token(&state.db, &token_id, &user_id).await {
        Ok(()) => HttpResponse::NoContent().finish(),
        Err(e) => map_error(e),
    })
}
