use actix_web::{web, HttpResponse, Result};
use shared::{ApiError, ApiSuccess, CreateJournalEntryRequest, UpdateJournalEntryRequest};
use uuid::Uuid;

use crate::models::AppState;
use crate::services::{households as household_service, journal as journal_service};

pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/journal")
            .route("", web::get().to(list_journal_entries))
            .route("", web::post().to(create_journal_entry))
            .route("/{entry_id}", web::get().to(get_journal_entry))
            .route("/{entry_id}", web::put().to(update_journal_entry))
            .route("/{entry_id}", web::delete().to(delete_journal_entry)),
    );
}

async fn list_journal_entries(
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

    match journal_service::list_journal_entries(&state.db, &household_id, &user_id).await {
        Ok(entries) => Ok(HttpResponse::Ok().json(ApiSuccess::new(entries))),
        Err(e) => {
            log::error!("Error listing journal entries: {:?}", e);
            Ok(HttpResponse::InternalServerError().json(ApiError {
                error: "internal_error".to_string(),
                message: "Failed to list journal entries".to_string(),
            }))
        }
    }
}

async fn create_journal_entry(
    state: web::Data<AppState>,
    req: actix_web::HttpRequest,
    path: web::Path<String>,
    body: web::Json<CreateJournalEntryRequest>,
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

    // Check membership (any member can create journal entries)
    if !household_service::is_member(&state.db, &household_id, &user_id)
        .await
        .unwrap_or(false)
    {
        return Ok(HttpResponse::Forbidden().json(ApiError {
            error: "forbidden".to_string(),
            message: "You are not a member of this household".to_string(),
        }));
    }

    let request = body.into_inner();
    if request.content.trim().is_empty() {
        return Ok(HttpResponse::BadRequest().json(ApiError {
            error: "validation_error".to_string(),
            message: "Journal entry content is required".to_string(),
        }));
    }

    match journal_service::create_journal_entry(&state.db, &household_id, &user_id, &request).await {
        Ok(entry) => Ok(HttpResponse::Created().json(ApiSuccess::new(entry))),
        Err(e) => {
            log::error!("Error creating journal entry: {:?}", e);
            Ok(HttpResponse::InternalServerError().json(ApiError {
                error: "internal_error".to_string(),
                message: "Failed to create journal entry".to_string(),
            }))
        }
    }
}

async fn get_journal_entry(
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

    let (household_id_str, entry_id_str) = path.into_inner();

    let household_id = match Uuid::parse_str(&household_id_str) {
        Ok(id) => id,
        Err(_) => {
            return Ok(HttpResponse::BadRequest().json(ApiError {
                error: "invalid_id".to_string(),
                message: "Invalid household ID format".to_string(),
            }));
        }
    };

    let entry_id = match Uuid::parse_str(&entry_id_str) {
        Ok(id) => id,
        Err(_) => {
            return Ok(HttpResponse::BadRequest().json(ApiError {
                error: "invalid_id".to_string(),
                message: "Invalid entry ID format".to_string(),
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

    match journal_service::get_journal_entry(&state.db, &entry_id).await {
        Ok(Some(entry)) => {
            // Check if user can view this entry
            if !journal_service::can_view_entry(&entry, &user_id) {
                return Ok(HttpResponse::Forbidden().json(ApiError {
                    error: "forbidden".to_string(),
                    message: "You do not have permission to view this journal entry".to_string(),
                }));
            }
            Ok(HttpResponse::Ok().json(ApiSuccess::new(entry)))
        }
        Ok(None) => Ok(HttpResponse::NotFound().json(ApiError {
            error: "not_found".to_string(),
            message: "Journal entry not found".to_string(),
        })),
        Err(e) => {
            log::error!("Error fetching journal entry: {:?}", e);
            Ok(HttpResponse::InternalServerError().json(ApiError {
                error: "internal_error".to_string(),
                message: "Failed to fetch journal entry".to_string(),
            }))
        }
    }
}

async fn update_journal_entry(
    state: web::Data<AppState>,
    req: actix_web::HttpRequest,
    path: web::Path<(String, String)>,
    body: web::Json<UpdateJournalEntryRequest>,
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

    let (household_id_str, entry_id_str) = path.into_inner();

    let household_id = match Uuid::parse_str(&household_id_str) {
        Ok(id) => id,
        Err(_) => {
            return Ok(HttpResponse::BadRequest().json(ApiError {
                error: "invalid_id".to_string(),
                message: "Invalid household ID format".to_string(),
            }));
        }
    };

    let entry_id = match Uuid::parse_str(&entry_id_str) {
        Ok(id) => id,
        Err(_) => {
            return Ok(HttpResponse::BadRequest().json(ApiError {
                error: "invalid_id".to_string(),
                message: "Invalid entry ID format".to_string(),
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

    let request = body.into_inner();

    match journal_service::update_journal_entry(&state.db, &entry_id, &user_id, &request).await {
        Ok(entry) => Ok(HttpResponse::Ok().json(ApiSuccess::new(entry))),
        Err(journal_service::JournalError::NotFound) => Ok(HttpResponse::NotFound().json(ApiError {
            error: "not_found".to_string(),
            message: "Journal entry not found".to_string(),
        })),
        Err(journal_service::JournalError::PermissionDenied) => {
            Ok(HttpResponse::Forbidden().json(ApiError {
                error: "forbidden".to_string(),
                message: "You do not have permission to edit this journal entry".to_string(),
            }))
        }
        Err(e) => {
            log::error!("Error updating journal entry: {:?}", e);
            Ok(HttpResponse::InternalServerError().json(ApiError {
                error: "internal_error".to_string(),
                message: "Failed to update journal entry".to_string(),
            }))
        }
    }
}

async fn delete_journal_entry(
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

    let (household_id_str, entry_id_str) = path.into_inner();

    let household_id = match Uuid::parse_str(&household_id_str) {
        Ok(id) => id,
        Err(_) => {
            return Ok(HttpResponse::BadRequest().json(ApiError {
                error: "invalid_id".to_string(),
                message: "Invalid household ID format".to_string(),
            }));
        }
    };

    let entry_id = match Uuid::parse_str(&entry_id_str) {
        Ok(id) => id,
        Err(_) => {
            return Ok(HttpResponse::BadRequest().json(ApiError {
                error: "invalid_id".to_string(),
                message: "Invalid entry ID format".to_string(),
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

    match journal_service::delete_journal_entry(&state.db, &entry_id, &user_id).await {
        Ok(_) => Ok(HttpResponse::NoContent().finish()),
        Err(journal_service::JournalError::NotFound) => Ok(HttpResponse::NotFound().json(ApiError {
            error: "not_found".to_string(),
            message: "Journal entry not found".to_string(),
        })),
        Err(journal_service::JournalError::PermissionDenied) => {
            Ok(HttpResponse::Forbidden().json(ApiError {
                error: "forbidden".to_string(),
                message: "You do not have permission to delete this journal entry".to_string(),
            }))
        }
        Err(e) => {
            log::error!("Error deleting journal entry: {:?}", e);
            Ok(HttpResponse::InternalServerError().json(ApiError {
                error: "internal_error".to_string(),
                message: "Failed to delete journal entry".to_string(),
            }))
        }
    }
}
