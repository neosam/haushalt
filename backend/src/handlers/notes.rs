use actix_web::{web, HttpResponse, Result};
use shared::{ApiError, ApiSuccess, CreateNoteRequest, UpdateNoteRequest};
use uuid::Uuid;

use crate::models::AppState;
use crate::services::{households as household_service, notes as notes_service};

pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/notes")
            .route("", web::get().to(list_notes))
            .route("", web::post().to(create_note))
            .route("/{note_id}", web::get().to(get_note))
            .route("/{note_id}", web::put().to(update_note))
            .route("/{note_id}", web::delete().to(delete_note)),
    );
}

async fn list_notes(
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

    match notes_service::list_notes(&state.db, &household_id, &user_id).await {
        Ok(notes) => Ok(HttpResponse::Ok().json(ApiSuccess::new(notes))),
        Err(e) => {
            log::error!("Error listing notes: {:?}", e);
            Ok(HttpResponse::InternalServerError().json(ApiError {
                error: "internal_error".to_string(),
                message: "Failed to list notes".to_string(),
            }))
        }
    }
}

async fn create_note(
    state: web::Data<AppState>,
    req: actix_web::HttpRequest,
    path: web::Path<String>,
    body: web::Json<CreateNoteRequest>,
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

    // Check membership (any member can create notes)
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
    if request.title.trim().is_empty() {
        return Ok(HttpResponse::BadRequest().json(ApiError {
            error: "validation_error".to_string(),
            message: "Note title is required".to_string(),
        }));
    }

    match notes_service::create_note(&state.db, &household_id, &user_id, &request).await {
        Ok(note) => Ok(HttpResponse::Created().json(ApiSuccess::new(note))),
        Err(e) => {
            log::error!("Error creating note: {:?}", e);
            Ok(HttpResponse::InternalServerError().json(ApiError {
                error: "internal_error".to_string(),
                message: "Failed to create note".to_string(),
            }))
        }
    }
}

async fn get_note(
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

    let (household_id_str, note_id_str) = path.into_inner();

    let household_id = match Uuid::parse_str(&household_id_str) {
        Ok(id) => id,
        Err(_) => {
            return Ok(HttpResponse::BadRequest().json(ApiError {
                error: "invalid_id".to_string(),
                message: "Invalid household ID format".to_string(),
            }));
        }
    };

    let note_id = match Uuid::parse_str(&note_id_str) {
        Ok(id) => id,
        Err(_) => {
            return Ok(HttpResponse::BadRequest().json(ApiError {
                error: "invalid_id".to_string(),
                message: "Invalid note ID format".to_string(),
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

    match notes_service::get_note(&state.db, &note_id).await {
        Ok(Some(note)) => {
            // Check if user can view this note
            if !notes_service::can_view_note(&note, &user_id) {
                return Ok(HttpResponse::Forbidden().json(ApiError {
                    error: "forbidden".to_string(),
                    message: "You do not have permission to view this note".to_string(),
                }));
            }
            Ok(HttpResponse::Ok().json(ApiSuccess::new(note)))
        }
        Ok(None) => Ok(HttpResponse::NotFound().json(ApiError {
            error: "not_found".to_string(),
            message: "Note not found".to_string(),
        })),
        Err(e) => {
            log::error!("Error fetching note: {:?}", e);
            Ok(HttpResponse::InternalServerError().json(ApiError {
                error: "internal_error".to_string(),
                message: "Failed to fetch note".to_string(),
            }))
        }
    }
}

async fn update_note(
    state: web::Data<AppState>,
    req: actix_web::HttpRequest,
    path: web::Path<(String, String)>,
    body: web::Json<UpdateNoteRequest>,
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

    let (household_id_str, note_id_str) = path.into_inner();

    let household_id = match Uuid::parse_str(&household_id_str) {
        Ok(id) => id,
        Err(_) => {
            return Ok(HttpResponse::BadRequest().json(ApiError {
                error: "invalid_id".to_string(),
                message: "Invalid household ID format".to_string(),
            }));
        }
    };

    let note_id = match Uuid::parse_str(&note_id_str) {
        Ok(id) => id,
        Err(_) => {
            return Ok(HttpResponse::BadRequest().json(ApiError {
                error: "invalid_id".to_string(),
                message: "Invalid note ID format".to_string(),
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

    match notes_service::update_note(&state.db, &note_id, &user_id, &request).await {
        Ok(note) => Ok(HttpResponse::Ok().json(ApiSuccess::new(note))),
        Err(notes_service::NoteError::NotFound) => Ok(HttpResponse::NotFound().json(ApiError {
            error: "not_found".to_string(),
            message: "Note not found".to_string(),
        })),
        Err(notes_service::NoteError::PermissionDenied) => {
            Ok(HttpResponse::Forbidden().json(ApiError {
                error: "forbidden".to_string(),
                message: "You do not have permission to edit this note".to_string(),
            }))
        }
        Err(e) => {
            log::error!("Error updating note: {:?}", e);
            Ok(HttpResponse::InternalServerError().json(ApiError {
                error: "internal_error".to_string(),
                message: "Failed to update note".to_string(),
            }))
        }
    }
}

async fn delete_note(
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

    let (household_id_str, note_id_str) = path.into_inner();

    let household_id = match Uuid::parse_str(&household_id_str) {
        Ok(id) => id,
        Err(_) => {
            return Ok(HttpResponse::BadRequest().json(ApiError {
                error: "invalid_id".to_string(),
                message: "Invalid household ID format".to_string(),
            }));
        }
    };

    let note_id = match Uuid::parse_str(&note_id_str) {
        Ok(id) => id,
        Err(_) => {
            return Ok(HttpResponse::BadRequest().json(ApiError {
                error: "invalid_id".to_string(),
                message: "Invalid note ID format".to_string(),
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

    match notes_service::delete_note(&state.db, &note_id, &user_id).await {
        Ok(_) => Ok(HttpResponse::NoContent().finish()),
        Err(notes_service::NoteError::NotFound) => Ok(HttpResponse::NotFound().json(ApiError {
            error: "not_found".to_string(),
            message: "Note not found".to_string(),
        })),
        Err(notes_service::NoteError::PermissionDenied) => {
            Ok(HttpResponse::Forbidden().json(ApiError {
                error: "forbidden".to_string(),
                message: "You do not have permission to delete this note".to_string(),
            }))
        }
        Err(e) => {
            log::error!("Error deleting note: {:?}", e);
            Ok(HttpResponse::InternalServerError().json(ApiError {
                error: "internal_error".to_string(),
                message: "Failed to delete note".to_string(),
            }))
        }
    }
}
