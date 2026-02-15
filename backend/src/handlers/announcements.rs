use actix_web::{web, HttpResponse, Result};
use shared::{ApiError, ApiSuccess, CreateAnnouncementRequest, Role, UpdateAnnouncementRequest};
use uuid::Uuid;

use crate::models::AppState;
use crate::services::{announcements as announcements_service, households as household_service};

pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/announcements")
            .route("", web::get().to(list_announcements))
            .route("/active", web::get().to(list_active_announcements))
            .route("", web::post().to(create_announcement))
            .route("/{announcement_id}", web::get().to(get_announcement))
            .route("/{announcement_id}", web::put().to(update_announcement))
            .route("/{announcement_id}", web::delete().to(delete_announcement)),
    );
}

/// List all announcements for a household (for management, any member can view)
async fn list_announcements(
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

    let household_id = match Uuid::parse_str(&path.into_inner()) {
        Ok(id) => id,
        Err(_) => {
            return Ok(HttpResponse::BadRequest().json(ApiError {
                error: "invalid_id".to_string(),
                message: "Invalid household ID format".to_string(),
            }));
        }
    };

    // Check membership
    if !household_service::is_member(&state.db, &household_id, &user_id)
        .await
        .unwrap_or(false)
    {
        return Ok(HttpResponse::Forbidden().json(ApiError {
            error: "forbidden".to_string(),
            message: "You are not a member of this household".to_string(),
        }));
    }

    match announcements_service::list_announcements(&state.db, &household_id).await {
        Ok(announcements) => Ok(HttpResponse::Ok().json(ApiSuccess::new(announcements))),
        Err(e) => {
            log::error!("Error listing announcements: {:?}", e);
            Ok(HttpResponse::InternalServerError().json(ApiError {
                error: "internal_error".to_string(),
                message: "Failed to list announcements".to_string(),
            }))
        }
    }
}

/// List currently active announcements for display (any member can view)
async fn list_active_announcements(
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

    let household_id = match Uuid::parse_str(&path.into_inner()) {
        Ok(id) => id,
        Err(_) => {
            return Ok(HttpResponse::BadRequest().json(ApiError {
                error: "invalid_id".to_string(),
                message: "Invalid household ID format".to_string(),
            }));
        }
    };

    // Check membership
    if !household_service::is_member(&state.db, &household_id, &user_id)
        .await
        .unwrap_or(false)
    {
        return Ok(HttpResponse::Forbidden().json(ApiError {
            error: "forbidden".to_string(),
            message: "You are not a member of this household".to_string(),
        }));
    }

    match announcements_service::list_active_announcements(&state.db, &household_id).await {
        Ok(announcements) => Ok(HttpResponse::Ok().json(ApiSuccess::new(announcements))),
        Err(e) => {
            log::error!("Error listing active announcements: {:?}", e);
            Ok(HttpResponse::InternalServerError().json(ApiError {
                error: "internal_error".to_string(),
                message: "Failed to list active announcements".to_string(),
            }))
        }
    }
}

/// Create a new announcement (owner only)
async fn create_announcement(
    state: web::Data<AppState>,
    req: actix_web::HttpRequest,
    path: web::Path<String>,
    body: web::Json<CreateAnnouncementRequest>,
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

    let household_id = match Uuid::parse_str(&path.into_inner()) {
        Ok(id) => id,
        Err(_) => {
            return Ok(HttpResponse::BadRequest().json(ApiError {
                error: "invalid_id".to_string(),
                message: "Invalid household ID format".to_string(),
            }));
        }
    };

    // Check owner permission
    let role = household_service::get_member_role(&state.db, &household_id, &user_id).await;
    if !role.map(|r| r == Role::Owner).unwrap_or(false) {
        return Ok(HttpResponse::Forbidden().json(ApiError {
            error: "forbidden".to_string(),
            message: "Only owners can create announcements".to_string(),
        }));
    }

    let request = body.into_inner();
    if request.title.trim().is_empty() {
        return Ok(HttpResponse::BadRequest().json(ApiError {
            error: "validation_error".to_string(),
            message: "Announcement title is required".to_string(),
        }));
    }

    match announcements_service::create_announcement(&state.db, &household_id, &user_id, &request)
        .await
    {
        Ok(announcement) => Ok(HttpResponse::Created().json(ApiSuccess::new(announcement))),
        Err(e) => {
            log::error!("Error creating announcement: {:?}", e);
            Ok(HttpResponse::InternalServerError().json(ApiError {
                error: "internal_error".to_string(),
                message: "Failed to create announcement".to_string(),
            }))
        }
    }
}

/// Get a single announcement
async fn get_announcement(
    state: web::Data<AppState>,
    req: actix_web::HttpRequest,
    path: web::Path<(String, String)>,
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

    let (household_id_str, announcement_id_str) = path.into_inner();

    let household_id = match Uuid::parse_str(&household_id_str) {
        Ok(id) => id,
        Err(_) => {
            return Ok(HttpResponse::BadRequest().json(ApiError {
                error: "invalid_id".to_string(),
                message: "Invalid household ID format".to_string(),
            }));
        }
    };

    let announcement_id = match Uuid::parse_str(&announcement_id_str) {
        Ok(id) => id,
        Err(_) => {
            return Ok(HttpResponse::BadRequest().json(ApiError {
                error: "invalid_id".to_string(),
                message: "Invalid announcement ID format".to_string(),
            }));
        }
    };

    // Check membership
    if !household_service::is_member(&state.db, &household_id, &user_id)
        .await
        .unwrap_or(false)
    {
        return Ok(HttpResponse::Forbidden().json(ApiError {
            error: "forbidden".to_string(),
            message: "You are not a member of this household".to_string(),
        }));
    }

    match announcements_service::get_announcement(&state.db, &announcement_id).await {
        Ok(Some(announcement)) => Ok(HttpResponse::Ok().json(ApiSuccess::new(announcement))),
        Ok(None) => Ok(HttpResponse::NotFound().json(ApiError {
            error: "not_found".to_string(),
            message: "Announcement not found".to_string(),
        })),
        Err(e) => {
            log::error!("Error fetching announcement: {:?}", e);
            Ok(HttpResponse::InternalServerError().json(ApiError {
                error: "internal_error".to_string(),
                message: "Failed to fetch announcement".to_string(),
            }))
        }
    }
}

/// Update an announcement (owner only)
async fn update_announcement(
    state: web::Data<AppState>,
    req: actix_web::HttpRequest,
    path: web::Path<(String, String)>,
    body: web::Json<UpdateAnnouncementRequest>,
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

    let (household_id_str, announcement_id_str) = path.into_inner();

    let household_id = match Uuid::parse_str(&household_id_str) {
        Ok(id) => id,
        Err(_) => {
            return Ok(HttpResponse::BadRequest().json(ApiError {
                error: "invalid_id".to_string(),
                message: "Invalid household ID format".to_string(),
            }));
        }
    };

    let announcement_id = match Uuid::parse_str(&announcement_id_str) {
        Ok(id) => id,
        Err(_) => {
            return Ok(HttpResponse::BadRequest().json(ApiError {
                error: "invalid_id".to_string(),
                message: "Invalid announcement ID format".to_string(),
            }));
        }
    };

    // Check owner permission
    let role = household_service::get_member_role(&state.db, &household_id, &user_id).await;
    if !role.map(|r| r == Role::Owner).unwrap_or(false) {
        return Ok(HttpResponse::Forbidden().json(ApiError {
            error: "forbidden".to_string(),
            message: "Only owners can update announcements".to_string(),
        }));
    }

    let request = body.into_inner();

    match announcements_service::update_announcement(&state.db, &announcement_id, &request).await {
        Ok(announcement) => Ok(HttpResponse::Ok().json(ApiSuccess::new(announcement))),
        Err(announcements_service::AnnouncementError::NotFound) => {
            Ok(HttpResponse::NotFound().json(ApiError {
                error: "not_found".to_string(),
                message: "Announcement not found".to_string(),
            }))
        }
        Err(e) => {
            log::error!("Error updating announcement: {:?}", e);
            Ok(HttpResponse::InternalServerError().json(ApiError {
                error: "internal_error".to_string(),
                message: "Failed to update announcement".to_string(),
            }))
        }
    }
}

/// Delete an announcement (owner only)
async fn delete_announcement(
    state: web::Data<AppState>,
    req: actix_web::HttpRequest,
    path: web::Path<(String, String)>,
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

    let (household_id_str, announcement_id_str) = path.into_inner();

    let household_id = match Uuid::parse_str(&household_id_str) {
        Ok(id) => id,
        Err(_) => {
            return Ok(HttpResponse::BadRequest().json(ApiError {
                error: "invalid_id".to_string(),
                message: "Invalid household ID format".to_string(),
            }));
        }
    };

    let announcement_id = match Uuid::parse_str(&announcement_id_str) {
        Ok(id) => id,
        Err(_) => {
            return Ok(HttpResponse::BadRequest().json(ApiError {
                error: "invalid_id".to_string(),
                message: "Invalid announcement ID format".to_string(),
            }));
        }
    };

    // Check owner permission
    let role = household_service::get_member_role(&state.db, &household_id, &user_id).await;
    if !role.map(|r| r == Role::Owner).unwrap_or(false) {
        return Ok(HttpResponse::Forbidden().json(ApiError {
            error: "forbidden".to_string(),
            message: "Only owners can delete announcements".to_string(),
        }));
    }

    match announcements_service::delete_announcement(&state.db, &announcement_id).await {
        Ok(_) => Ok(HttpResponse::NoContent().finish()),
        Err(announcements_service::AnnouncementError::NotFound) => {
            Ok(HttpResponse::NotFound().json(ApiError {
                error: "not_found".to_string(),
                message: "Announcement not found".to_string(),
            }))
        }
        Err(e) => {
            log::error!("Error deleting announcement: {:?}", e);
            Ok(HttpResponse::InternalServerError().json(ApiError {
                error: "internal_error".to_string(),
                message: "Failed to delete announcement".to_string(),
            }))
        }
    }
}
