use actix_web::{web, HttpResponse, Result};
use shared::{ApiError, ApiSuccess, CreateChatMessageRequest, ListChatMessagesRequest, UpdateChatMessageRequest};
use uuid::Uuid;

use crate::models::AppState;
use crate::services::{chat as chat_service, households as household_service};

pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/chat")
            .route("", web::get().to(list_messages))
            .route("", web::post().to(create_message))
            .route("/{message_id}", web::put().to(update_message))
            .route("/{message_id}", web::delete().to(delete_message)),
    );
}

/// List chat messages for a household with pagination
async fn list_messages(
    state: web::Data<AppState>,
    req: actix_web::HttpRequest,
    path: web::Path<String>,
    query: web::Query<ListChatMessagesRequest>,
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

    let limit = query.limit.unwrap_or(50).min(100);
    let before = query.before.as_ref();

    match chat_service::list_messages(&state.db, &household_id, limit, before).await {
        Ok(messages) => Ok(HttpResponse::Ok().json(ApiSuccess::new(messages))),
        Err(e) => {
            log::error!("Error listing chat messages: {:?}", e);
            Ok(HttpResponse::InternalServerError().json(ApiError {
                error: "internal_error".to_string(),
                message: "Failed to list messages".to_string(),
            }))
        }
    }
}

/// Create a new chat message (REST fallback - prefer WebSocket)
async fn create_message(
    state: web::Data<AppState>,
    req: actix_web::HttpRequest,
    path: web::Path<String>,
    body: web::Json<CreateChatMessageRequest>,
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

    let content = body.into_inner().content;

    match chat_service::create_message(&state.db, &household_id, &user_id, &content).await {
        Ok(message) => {
            // Get message with user for response
            match chat_service::get_message_with_user(&state.db, &message.id).await {
                Ok(Some(msg_with_user)) => {
                    // Broadcast to WebSocket if available
                    if let Some(ws_manager) = req.app_data::<web::Data<std::sync::Arc<crate::services::websocket::WsManager>>>() {
                        ws_manager.broadcast_new_message(&household_id, msg_with_user.clone()).await;
                    }
                    Ok(HttpResponse::Created().json(ApiSuccess::new(msg_with_user)))
                }
                _ => Ok(HttpResponse::Created().json(ApiSuccess::new(message))),
            }
        }
        Err(chat_service::ChatError::EmptyContent) => {
            Ok(HttpResponse::BadRequest().json(ApiError {
                error: "validation_error".to_string(),
                message: "Message content cannot be empty".to_string(),
            }))
        }
        Err(e) => {
            log::error!("Error creating chat message: {:?}", e);
            Ok(HttpResponse::InternalServerError().json(ApiError {
                error: "internal_error".to_string(),
                message: "Failed to create message".to_string(),
            }))
        }
    }
}

/// Update a chat message (only the author can edit)
async fn update_message(
    state: web::Data<AppState>,
    req: actix_web::HttpRequest,
    path: web::Path<(String, String)>,
    body: web::Json<UpdateChatMessageRequest>,
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

    let (household_id_str, message_id_str) = path.into_inner();
    let household_id = match Uuid::parse_str(&household_id_str) {
        Ok(id) => id,
        Err(_) => {
            return Ok(HttpResponse::BadRequest().json(ApiError {
                error: "invalid_id".to_string(),
                message: "Invalid household ID format".to_string(),
            }));
        }
    };

    let message_id = match Uuid::parse_str(&message_id_str) {
        Ok(id) => id,
        Err(_) => {
            return Ok(HttpResponse::BadRequest().json(ApiError {
                error: "invalid_id".to_string(),
                message: "Invalid message ID format".to_string(),
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

    let content = body.into_inner().content;

    match chat_service::update_message(&state.db, &message_id, &user_id, &content).await {
        Ok(_message) => {
            // Get updated message with user
            match chat_service::get_message_with_user(&state.db, &message_id).await {
                Ok(Some(msg_with_user)) => {
                    // Broadcast to WebSocket if available
                    if let Some(ws_manager) = req.app_data::<web::Data<std::sync::Arc<crate::services::websocket::WsManager>>>() {
                        ws_manager.broadcast_message_edited(&household_id, msg_with_user.clone()).await;
                    }
                    Ok(HttpResponse::Ok().json(ApiSuccess::new(msg_with_user)))
                }
                _ => Ok(HttpResponse::Ok().json(ApiSuccess::new(_message))),
            }
        }
        Err(chat_service::ChatError::NotFound) => {
            Ok(HttpResponse::NotFound().json(ApiError {
                error: "not_found".to_string(),
                message: "Message not found".to_string(),
            }))
        }
        Err(chat_service::ChatError::NotAuthorized) => {
            Ok(HttpResponse::Forbidden().json(ApiError {
                error: "forbidden".to_string(),
                message: "You can only edit your own messages".to_string(),
            }))
        }
        Err(chat_service::ChatError::EmptyContent) => {
            Ok(HttpResponse::BadRequest().json(ApiError {
                error: "validation_error".to_string(),
                message: "Message content cannot be empty".to_string(),
            }))
        }
        Err(e) => {
            log::error!("Error updating chat message: {:?}", e);
            Ok(HttpResponse::InternalServerError().json(ApiError {
                error: "internal_error".to_string(),
                message: "Failed to update message".to_string(),
            }))
        }
    }
}

/// Delete a chat message (only the author can delete)
async fn delete_message(
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

    let (household_id_str, message_id_str) = path.into_inner();
    let household_id = match Uuid::parse_str(&household_id_str) {
        Ok(id) => id,
        Err(_) => {
            return Ok(HttpResponse::BadRequest().json(ApiError {
                error: "invalid_id".to_string(),
                message: "Invalid household ID format".to_string(),
            }));
        }
    };

    let message_id = match Uuid::parse_str(&message_id_str) {
        Ok(id) => id,
        Err(_) => {
            return Ok(HttpResponse::BadRequest().json(ApiError {
                error: "invalid_id".to_string(),
                message: "Invalid message ID format".to_string(),
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

    match chat_service::delete_message(&state.db, &message_id, &user_id).await {
        Ok(()) => {
            // Broadcast to WebSocket if available
            if let Some(ws_manager) = req.app_data::<web::Data<std::sync::Arc<crate::services::websocket::WsManager>>>() {
                ws_manager.broadcast_message_deleted(&household_id, message_id).await;
            }
            Ok(HttpResponse::NoContent().finish())
        }
        Err(chat_service::ChatError::NotFound) => {
            Ok(HttpResponse::NotFound().json(ApiError {
                error: "not_found".to_string(),
                message: "Message not found".to_string(),
            }))
        }
        Err(chat_service::ChatError::NotAuthorized) => {
            Ok(HttpResponse::Forbidden().json(ApiError {
                error: "forbidden".to_string(),
                message: "You can only delete your own messages".to_string(),
            }))
        }
        Err(e) => {
            log::error!("Error deleting chat message: {:?}", e);
            Ok(HttpResponse::InternalServerError().json(ApiError {
                error: "internal_error".to_string(),
                message: "Failed to delete message".to_string(),
            }))
        }
    }
}
