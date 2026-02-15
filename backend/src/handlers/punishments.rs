use actix_web::{web, HttpResponse, Result};
use shared::{ApiError, ApiSuccess, CreatePunishmentRequest, UpdatePunishmentRequest};
use uuid::Uuid;

use crate::models::AppState;
use crate::services::{household_settings, households as household_service, punishments as punishment_service};

pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/punishments")
            .route("", web::get().to(list_punishments))
            .route("", web::post().to(create_punishment))
            .route("/{punishment_id}", web::get().to(get_punishment))
            .route("/{punishment_id}", web::put().to(update_punishment))
            .route("/{punishment_id}", web::delete().to(delete_punishment))
            .route("/{punishment_id}/assign/{user_id}", web::post().to(assign_punishment))
            .route("/{punishment_id}/unassign/{user_id}", web::post().to(unassign_punishment))
            .route("/user-punishments", web::get().to(list_user_punishments))
            .route("/user-punishments/all", web::get().to(list_all_user_punishments))
            .route("/user-punishments/{id}", web::delete().to(delete_user_punishment))
            .route("/user-punishments/{id}/complete", web::post().to(complete_punishment))
    );
}

async fn list_punishments(
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

    if !household_service::is_member(&state.db, &household_id, &user_id).await.unwrap_or(false) {
        return Ok(HttpResponse::Forbidden().json(ApiError {
            error: "forbidden".to_string(),
            message: "You are not a member of this household".to_string(),
        }));
    }

    match punishment_service::list_punishments(&state.db, &household_id).await {
        Ok(punishments) => Ok(HttpResponse::Ok().json(ApiSuccess::new(punishments))),
        Err(e) => {
            log::error!("Error listing punishments: {:?}", e);
            Ok(HttpResponse::InternalServerError().json(ApiError {
                error: "internal_error".to_string(),
                message: "Failed to list punishments".to_string(),
            }))
        }
    }
}

async fn create_punishment(
    state: web::Data<AppState>,
    req: actix_web::HttpRequest,
    path: web::Path<String>,
    body: web::Json<CreatePunishmentRequest>,
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

    // Get settings for hierarchy-aware permissions
    let settings = match household_settings::get_or_create_settings(&state.db, &household_id).await {
        Ok(s) => s,
        Err(e) => {
            log::error!("Error fetching settings: {:?}", e);
            return Ok(HttpResponse::InternalServerError().json(ApiError {
                error: "internal_error".to_string(),
                message: "Failed to fetch household settings".to_string(),
            }));
        }
    };

    let role = household_service::get_member_role(&state.db, &household_id, &user_id).await;
    if !role.as_ref().map(|r| settings.hierarchy_type.can_manage(r)).unwrap_or(false) {
        return Ok(HttpResponse::Forbidden().json(ApiError {
            error: "forbidden".to_string(),
            message: "You do not have permission to create punishments".to_string(),
        }));
    }

    let request = body.into_inner();
    if request.name.is_empty() {
        return Ok(HttpResponse::BadRequest().json(ApiError {
            error: "validation_error".to_string(),
            message: "Punishment name is required".to_string(),
        }));
    }

    match punishment_service::create_punishment(&state.db, &household_id, &request).await {
        Ok(punishment) => Ok(HttpResponse::Created().json(ApiSuccess::new(punishment))),
        Err(e) => {
            log::error!("Error creating punishment: {:?}", e);
            Ok(HttpResponse::InternalServerError().json(ApiError {
                error: "internal_error".to_string(),
                message: "Failed to create punishment".to_string(),
            }))
        }
    }
}

async fn get_punishment(
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

    let (household_id_str, punishment_id_str) = path.into_inner();

    let household_id = match Uuid::parse_str(&household_id_str) {
        Ok(id) => id,
        Err(_) => {
            return Ok(HttpResponse::BadRequest().json(ApiError {
                error: "invalid_id".to_string(),
                message: "Invalid household ID format".to_string(),
            }));
        }
    };

    let punishment_id = match Uuid::parse_str(&punishment_id_str) {
        Ok(id) => id,
        Err(_) => {
            return Ok(HttpResponse::BadRequest().json(ApiError {
                error: "invalid_id".to_string(),
                message: "Invalid punishment ID format".to_string(),
            }));
        }
    };

    if !household_service::is_member(&state.db, &household_id, &user_id).await.unwrap_or(false) {
        return Ok(HttpResponse::Forbidden().json(ApiError {
            error: "forbidden".to_string(),
            message: "You are not a member of this household".to_string(),
        }));
    }

    match punishment_service::get_punishment(&state.db, &punishment_id).await {
        Ok(Some(punishment)) => Ok(HttpResponse::Ok().json(ApiSuccess::new(punishment))),
        Ok(None) => Ok(HttpResponse::NotFound().json(ApiError {
            error: "not_found".to_string(),
            message: "Punishment not found".to_string(),
        })),
        Err(e) => {
            log::error!("Error fetching punishment: {:?}", e);
            Ok(HttpResponse::InternalServerError().json(ApiError {
                error: "internal_error".to_string(),
                message: "Failed to fetch punishment".to_string(),
            }))
        }
    }
}

async fn update_punishment(
    state: web::Data<AppState>,
    req: actix_web::HttpRequest,
    path: web::Path<(String, String)>,
    body: web::Json<UpdatePunishmentRequest>,
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

    let (household_id_str, punishment_id_str) = path.into_inner();

    let household_id = match Uuid::parse_str(&household_id_str) {
        Ok(id) => id,
        Err(_) => {
            return Ok(HttpResponse::BadRequest().json(ApiError {
                error: "invalid_id".to_string(),
                message: "Invalid household ID format".to_string(),
            }));
        }
    };

    let punishment_id = match Uuid::parse_str(&punishment_id_str) {
        Ok(id) => id,
        Err(_) => {
            return Ok(HttpResponse::BadRequest().json(ApiError {
                error: "invalid_id".to_string(),
                message: "Invalid punishment ID format".to_string(),
            }));
        }
    };

    // Get settings for hierarchy-aware permissions
    let settings = match household_settings::get_or_create_settings(&state.db, &household_id).await {
        Ok(s) => s,
        Err(e) => {
            log::error!("Error fetching settings: {:?}", e);
            return Ok(HttpResponse::InternalServerError().json(ApiError {
                error: "internal_error".to_string(),
                message: "Failed to fetch household settings".to_string(),
            }));
        }
    };

    let role = household_service::get_member_role(&state.db, &household_id, &user_id).await;
    if !role.as_ref().map(|r| settings.hierarchy_type.can_manage(r)).unwrap_or(false) {
        return Ok(HttpResponse::Forbidden().json(ApiError {
            error: "forbidden".to_string(),
            message: "You do not have permission to update punishments".to_string(),
        }));
    }

    match punishment_service::update_punishment(&state.db, &punishment_id, &body.into_inner()).await {
        Ok(punishment) => Ok(HttpResponse::Ok().json(ApiSuccess::new(punishment))),
        Err(e) => {
            log::error!("Error updating punishment: {:?}", e);
            Ok(HttpResponse::InternalServerError().json(ApiError {
                error: "internal_error".to_string(),
                message: "Failed to update punishment".to_string(),
            }))
        }
    }
}

async fn delete_punishment(
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

    let (household_id_str, punishment_id_str) = path.into_inner();

    let household_id = match Uuid::parse_str(&household_id_str) {
        Ok(id) => id,
        Err(_) => {
            return Ok(HttpResponse::BadRequest().json(ApiError {
                error: "invalid_id".to_string(),
                message: "Invalid household ID format".to_string(),
            }));
        }
    };

    let punishment_id = match Uuid::parse_str(&punishment_id_str) {
        Ok(id) => id,
        Err(_) => {
            return Ok(HttpResponse::BadRequest().json(ApiError {
                error: "invalid_id".to_string(),
                message: "Invalid punishment ID format".to_string(),
            }));
        }
    };

    // Get settings for hierarchy-aware permissions
    let settings = match household_settings::get_or_create_settings(&state.db, &household_id).await {
        Ok(s) => s,
        Err(e) => {
            log::error!("Error fetching settings: {:?}", e);
            return Ok(HttpResponse::InternalServerError().json(ApiError {
                error: "internal_error".to_string(),
                message: "Failed to fetch household settings".to_string(),
            }));
        }
    };

    let role = household_service::get_member_role(&state.db, &household_id, &user_id).await;
    if !role.as_ref().map(|r| settings.hierarchy_type.can_manage(r)).unwrap_or(false) {
        return Ok(HttpResponse::Forbidden().json(ApiError {
            error: "forbidden".to_string(),
            message: "You do not have permission to delete punishments".to_string(),
        }));
    }

    match punishment_service::delete_punishment(&state.db, &punishment_id).await {
        Ok(_) => Ok(HttpResponse::NoContent().finish()),
        Err(e) => {
            log::error!("Error deleting punishment: {:?}", e);
            Ok(HttpResponse::InternalServerError().json(ApiError {
                error: "internal_error".to_string(),
                message: "Failed to delete punishment".to_string(),
            }))
        }
    }
}

async fn assign_punishment(
    state: web::Data<AppState>,
    req: actix_web::HttpRequest,
    path: web::Path<(String, String, String)>,
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

    let (household_id_str, punishment_id_str, target_user_id_str) = path.into_inner();

    let household_id = match Uuid::parse_str(&household_id_str) {
        Ok(id) => id,
        Err(_) => {
            return Ok(HttpResponse::BadRequest().json(ApiError {
                error: "invalid_id".to_string(),
                message: "Invalid household ID format".to_string(),
            }));
        }
    };

    let punishment_id = match Uuid::parse_str(&punishment_id_str) {
        Ok(id) => id,
        Err(_) => {
            return Ok(HttpResponse::BadRequest().json(ApiError {
                error: "invalid_id".to_string(),
                message: "Invalid punishment ID format".to_string(),
            }));
        }
    };

    let target_user_id = match Uuid::parse_str(&target_user_id_str) {
        Ok(id) => id,
        Err(_) => {
            return Ok(HttpResponse::BadRequest().json(ApiError {
                error: "invalid_id".to_string(),
                message: "Invalid user ID format".to_string(),
            }));
        }
    };

    // Get settings for hierarchy-aware permissions
    let settings = match household_settings::get_or_create_settings(&state.db, &household_id).await {
        Ok(s) => s,
        Err(e) => {
            log::error!("Error fetching settings: {:?}", e);
            return Ok(HttpResponse::InternalServerError().json(ApiError {
                error: "internal_error".to_string(),
                message: "Failed to fetch household settings".to_string(),
            }));
        }
    };

    let role = household_service::get_member_role(&state.db, &household_id, &current_user_id).await;
    if !role.as_ref().map(|r| settings.hierarchy_type.can_manage(r)).unwrap_or(false) {
        return Ok(HttpResponse::Forbidden().json(ApiError {
            error: "forbidden".to_string(),
            message: "You do not have permission to assign punishments".to_string(),
        }));
    }

    // Verify target user is a member
    if !household_service::is_member(&state.db, &household_id, &target_user_id).await.unwrap_or(false) {
        return Ok(HttpResponse::BadRequest().json(ApiError {
            error: "invalid_user".to_string(),
            message: "Target user is not a member of this household".to_string(),
        }));
    }

    match punishment_service::assign_punishment(&state.db, &punishment_id, &target_user_id, &household_id).await {
        Ok(user_punishment) => Ok(HttpResponse::Created().json(ApiSuccess::new(user_punishment))),
        Err(e) => {
            log::error!("Error assigning punishment: {:?}", e);
            Ok(HttpResponse::InternalServerError().json(ApiError {
                error: "internal_error".to_string(),
                message: "Failed to assign punishment".to_string(),
            }))
        }
    }
}

async fn unassign_punishment(
    state: web::Data<AppState>,
    req: actix_web::HttpRequest,
    path: web::Path<(String, String, String)>,
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

    let (household_id_str, punishment_id_str, target_user_id_str) = path.into_inner();

    let household_id = match Uuid::parse_str(&household_id_str) {
        Ok(id) => id,
        Err(_) => {
            return Ok(HttpResponse::BadRequest().json(ApiError {
                error: "invalid_id".to_string(),
                message: "Invalid household ID format".to_string(),
            }));
        }
    };

    let punishment_id = match Uuid::parse_str(&punishment_id_str) {
        Ok(id) => id,
        Err(_) => {
            return Ok(HttpResponse::BadRequest().json(ApiError {
                error: "invalid_id".to_string(),
                message: "Invalid punishment ID format".to_string(),
            }));
        }
    };

    let target_user_id = match Uuid::parse_str(&target_user_id_str) {
        Ok(id) => id,
        Err(_) => {
            return Ok(HttpResponse::BadRequest().json(ApiError {
                error: "invalid_id".to_string(),
                message: "Invalid user ID format".to_string(),
            }));
        }
    };

    // Get settings for hierarchy-aware permissions
    let settings = match household_settings::get_or_create_settings(&state.db, &household_id).await {
        Ok(s) => s,
        Err(e) => {
            log::error!("Error fetching settings: {:?}", e);
            return Ok(HttpResponse::InternalServerError().json(ApiError {
                error: "internal_error".to_string(),
                message: "Failed to fetch household settings".to_string(),
            }));
        }
    };

    let role = household_service::get_member_role(&state.db, &household_id, &current_user_id).await;
    if !role.as_ref().map(|r| settings.hierarchy_type.can_manage(r)).unwrap_or(false) {
        return Ok(HttpResponse::Forbidden().json(ApiError {
            error: "forbidden".to_string(),
            message: "You do not have permission to unassign punishments".to_string(),
        }));
    }

    match punishment_service::unassign_punishment(&state.db, &punishment_id, &target_user_id, &household_id).await {
        Ok(_) => Ok(HttpResponse::Ok().json(ApiSuccess::new(()))),
        Err(e) => {
            log::error!("Error unassigning punishment: {:?}", e);
            Ok(HttpResponse::BadRequest().json(ApiError {
                error: "unassign_error".to_string(),
                message: e.to_string(),
            }))
        }
    }
}

async fn list_user_punishments(
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

    if !household_service::is_member(&state.db, &household_id, &user_id).await.unwrap_or(false) {
        return Ok(HttpResponse::Forbidden().json(ApiError {
            error: "forbidden".to_string(),
            message: "You are not a member of this household".to_string(),
        }));
    }

    match punishment_service::list_user_punishments(&state.db, &user_id, &household_id).await {
        Ok(punishments) => Ok(HttpResponse::Ok().json(ApiSuccess::new(punishments))),
        Err(e) => {
            log::error!("Error listing user punishments: {:?}", e);
            Ok(HttpResponse::InternalServerError().json(ApiError {
                error: "internal_error".to_string(),
                message: "Failed to list user punishments".to_string(),
            }))
        }
    }
}

async fn list_all_user_punishments(
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

    if !household_service::is_member(&state.db, &household_id, &user_id).await.unwrap_or(false) {
        return Ok(HttpResponse::Forbidden().json(ApiError {
            error: "forbidden".to_string(),
            message: "You are not a member of this household".to_string(),
        }));
    }

    match punishment_service::list_all_user_punishments_in_household(&state.db, &household_id).await {
        Ok(punishments) => Ok(HttpResponse::Ok().json(ApiSuccess::new(punishments))),
        Err(e) => {
            log::error!("Error listing all user punishments: {:?}", e);
            Ok(HttpResponse::InternalServerError().json(ApiError {
                error: "internal_error".to_string(),
                message: "Failed to list all user punishments".to_string(),
            }))
        }
    }
}

async fn delete_user_punishment(
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

    let (household_id_str, user_punishment_id_str) = path.into_inner();

    let household_id = match Uuid::parse_str(&household_id_str) {
        Ok(id) => id,
        Err(_) => {
            return Ok(HttpResponse::BadRequest().json(ApiError {
                error: "invalid_id".to_string(),
                message: "Invalid household ID format".to_string(),
            }));
        }
    };

    let user_punishment_id = match Uuid::parse_str(&user_punishment_id_str) {
        Ok(id) => id,
        Err(_) => {
            return Ok(HttpResponse::BadRequest().json(ApiError {
                error: "invalid_id".to_string(),
                message: "Invalid user punishment ID format".to_string(),
            }));
        }
    };

    // Get settings for hierarchy-aware permissions
    let settings = match household_settings::get_or_create_settings(&state.db, &household_id).await {
        Ok(s) => s,
        Err(e) => {
            log::error!("Error fetching settings: {:?}", e);
            return Ok(HttpResponse::InternalServerError().json(ApiError {
                error: "internal_error".to_string(),
                message: "Failed to fetch household settings".to_string(),
            }));
        }
    };

    // Only users with manage permission can delete user punishments
    let role = household_service::get_member_role(&state.db, &household_id, &user_id).await;
    if !role.as_ref().map(|r| settings.hierarchy_type.can_manage(r)).unwrap_or(false) {
        return Ok(HttpResponse::Forbidden().json(ApiError {
            error: "forbidden".to_string(),
            message: "You do not have permission to remove punishment assignments".to_string(),
        }));
    }

    match punishment_service::delete_user_punishment(&state.db, &user_punishment_id).await {
        Ok(_) => Ok(HttpResponse::NoContent().finish()),
        Err(e) => {
            log::error!("Error deleting user punishment: {:?}", e);
            Ok(HttpResponse::BadRequest().json(ApiError {
                error: "delete_error".to_string(),
                message: e.to_string(),
            }))
        }
    }
}

async fn complete_punishment(
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

    let (household_id_str, user_punishment_id_str) = path.into_inner();

    let household_id = match Uuid::parse_str(&household_id_str) {
        Ok(id) => id,
        Err(_) => {
            return Ok(HttpResponse::BadRequest().json(ApiError {
                error: "invalid_id".to_string(),
                message: "Invalid household ID format".to_string(),
            }));
        }
    };

    let user_punishment_id = match Uuid::parse_str(&user_punishment_id_str) {
        Ok(id) => id,
        Err(_) => {
            return Ok(HttpResponse::BadRequest().json(ApiError {
                error: "invalid_id".to_string(),
                message: "Invalid user punishment ID format".to_string(),
            }));
        }
    };

    // Get settings for hierarchy-aware permissions
    let settings = match household_settings::get_or_create_settings(&state.db, &household_id).await {
        Ok(s) => s,
        Err(e) => {
            log::error!("Error fetching settings: {:?}", e);
            return Ok(HttpResponse::InternalServerError().json(ApiError {
                error: "internal_error".to_string(),
                message: "Failed to fetch household settings".to_string(),
            }));
        }
    };

    // Only users with manage permission can mark punishments as complete
    let role = household_service::get_member_role(&state.db, &household_id, &user_id).await;
    if !role.as_ref().map(|r| settings.hierarchy_type.can_manage(r)).unwrap_or(false) {
        return Ok(HttpResponse::Forbidden().json(ApiError {
            error: "forbidden".to_string(),
            message: "You do not have permission to mark punishments as complete".to_string(),
        }));
    }

    match punishment_service::complete_punishment(&state.db, &user_punishment_id, &user_id).await {
        Ok(user_punishment) => Ok(HttpResponse::Ok().json(ApiSuccess::new(user_punishment))),
        Err(e) => {
            log::error!("Error completing punishment: {:?}", e);
            Ok(HttpResponse::BadRequest().json(ApiError {
                error: "completion_error".to_string(),
                message: e.to_string(),
            }))
        }
    }
}
