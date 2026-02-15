use actix_web::{web, HttpResponse, Result};
use shared::{ApiError, ApiSuccess};
use uuid::Uuid;

use crate::models::AppState;
use crate::services::{auth as auth_service, invitations as invitation_service};

pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/invitations")
            .route("", web::get().to(list_user_invitations))
            .route("/{id}/accept", web::post().to(accept_invitation))
            .route("/{id}/decline", web::post().to(decline_invitation)),
    );
}

/// Get current user's pending invitations
async fn list_user_invitations(
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

    // Get user's email
    let user = match auth_service::get_user_by_id(&state.db, &user_id).await {
        Ok(Some(u)) => u,
        Ok(None) => {
            return Ok(HttpResponse::NotFound().json(ApiError {
                error: "not_found".to_string(),
                message: "User not found".to_string(),
            }));
        }
        Err(e) => {
            log::error!("Error fetching user: {:?}", e);
            return Ok(HttpResponse::InternalServerError().json(ApiError {
                error: "internal_error".to_string(),
                message: "Failed to fetch user".to_string(),
            }));
        }
    };

    match invitation_service::get_user_invitations(&state.db, &user.email).await {
        Ok(invitations) => Ok(HttpResponse::Ok().json(ApiSuccess::new(invitations))),
        Err(e) => {
            log::error!("Error fetching invitations: {:?}", e);
            Ok(HttpResponse::InternalServerError().json(ApiError {
                error: "internal_error".to_string(),
                message: "Failed to fetch invitations".to_string(),
            }))
        }
    }
}

/// Accept an invitation
async fn accept_invitation(
    state: web::Data<AppState>,
    req: actix_web::HttpRequest,
    path: web::Path<String>,
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

    let invitation_id = match Uuid::parse_str(&path.into_inner()) {
        Ok(id) => id,
        Err(_) => {
            return Ok(HttpResponse::BadRequest().json(ApiError {
                error: "invalid_id".to_string(),
                message: "Invalid invitation ID format".to_string(),
            }));
        }
    };

    // Get user
    let user = match auth_service::get_user_by_id(&state.db, &user_id).await {
        Ok(Some(u)) => u,
        Ok(None) => {
            return Ok(HttpResponse::NotFound().json(ApiError {
                error: "not_found".to_string(),
                message: "User not found".to_string(),
            }));
        }
        Err(e) => {
            log::error!("Error fetching user: {:?}", e);
            return Ok(HttpResponse::InternalServerError().json(ApiError {
                error: "internal_error".to_string(),
                message: "Failed to fetch user".to_string(),
            }));
        }
    };

    match invitation_service::accept_invitation(&state.db, &invitation_id, &user).await {
        Ok(membership) => Ok(HttpResponse::Ok().json(ApiSuccess::new(membership))),
        Err(invitation_service::InvitationError::NotFound) => {
            Ok(HttpResponse::NotFound().json(ApiError {
                error: "not_found".to_string(),
                message: "Invitation not found".to_string(),
            }))
        }
        Err(invitation_service::InvitationError::NotForUser) => {
            Ok(HttpResponse::Forbidden().json(ApiError {
                error: "forbidden".to_string(),
                message: "This invitation is not for you".to_string(),
            }))
        }
        Err(invitation_service::InvitationError::Expired) => {
            Ok(HttpResponse::BadRequest().json(ApiError {
                error: "expired".to_string(),
                message: "This invitation has expired".to_string(),
            }))
        }
        Err(invitation_service::InvitationError::AlreadyMember) => {
            Ok(HttpResponse::BadRequest().json(ApiError {
                error: "already_member".to_string(),
                message: "You are already a member of this household".to_string(),
            }))
        }
        Err(e) => {
            log::error!("Error accepting invitation: {:?}", e);
            Ok(HttpResponse::InternalServerError().json(ApiError {
                error: "internal_error".to_string(),
                message: "Failed to accept invitation".to_string(),
            }))
        }
    }
}

/// Decline an invitation
async fn decline_invitation(
    state: web::Data<AppState>,
    req: actix_web::HttpRequest,
    path: web::Path<String>,
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

    let invitation_id = match Uuid::parse_str(&path.into_inner()) {
        Ok(id) => id,
        Err(_) => {
            return Ok(HttpResponse::BadRequest().json(ApiError {
                error: "invalid_id".to_string(),
                message: "Invalid invitation ID format".to_string(),
            }));
        }
    };

    // Get user
    let user = match auth_service::get_user_by_id(&state.db, &user_id).await {
        Ok(Some(u)) => u,
        Ok(None) => {
            return Ok(HttpResponse::NotFound().json(ApiError {
                error: "not_found".to_string(),
                message: "User not found".to_string(),
            }));
        }
        Err(e) => {
            log::error!("Error fetching user: {:?}", e);
            return Ok(HttpResponse::InternalServerError().json(ApiError {
                error: "internal_error".to_string(),
                message: "Failed to fetch user".to_string(),
            }));
        }
    };

    match invitation_service::decline_invitation(&state.db, &invitation_id, &user).await {
        Ok(_) => Ok(HttpResponse::NoContent().finish()),
        Err(invitation_service::InvitationError::NotFound) => {
            Ok(HttpResponse::NotFound().json(ApiError {
                error: "not_found".to_string(),
                message: "Invitation not found".to_string(),
            }))
        }
        Err(invitation_service::InvitationError::NotForUser) => {
            Ok(HttpResponse::Forbidden().json(ApiError {
                error: "forbidden".to_string(),
                message: "This invitation is not for you".to_string(),
            }))
        }
        Err(e) => {
            log::error!("Error declining invitation: {:?}", e);
            Ok(HttpResponse::InternalServerError().json(ApiError {
                error: "internal_error".to_string(),
                message: "Failed to decline invitation".to_string(),
            }))
        }
    }
}
