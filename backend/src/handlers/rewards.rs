use actix_web::{web, HttpResponse, Result};
use shared::{ActivityType, ApiError, ApiSuccess, CreateRewardRequest, UpdateRewardRequest};
use uuid::Uuid;

use crate::models::AppState;
use crate::services::{activity_logs, household_settings, households as household_service, rewards as reward_service};

pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/rewards")
            .route("", web::get().to(list_rewards))
            .route("", web::post().to(create_reward))
            .route("/{reward_id}", web::get().to(get_reward))
            .route("/{reward_id}", web::put().to(update_reward))
            .route("/{reward_id}", web::delete().to(delete_reward))
            .route("/{reward_id}/purchase", web::post().to(purchase_reward))
            .route("/{reward_id}/assign/{user_id}", web::post().to(assign_reward))
            .route("/{reward_id}/unassign/{user_id}", web::post().to(unassign_reward))
            .route("/user-rewards", web::get().to(list_user_rewards))
            .route("/user-rewards/all", web::get().to(list_all_user_rewards))
            .route("/user-rewards/{id}", web::delete().to(delete_user_reward))
            .route("/user-rewards/{id}/redeem", web::post().to(redeem_reward))
    );
}

async fn list_rewards(
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

    match reward_service::list_rewards(&state.db, &household_id).await {
        Ok(rewards) => Ok(HttpResponse::Ok().json(ApiSuccess::new(rewards))),
        Err(e) => {
            log::error!("Error listing rewards: {:?}", e);
            Ok(HttpResponse::InternalServerError().json(ApiError {
                error: "internal_error".to_string(),
                message: "Failed to list rewards".to_string(),
            }))
        }
    }
}

async fn create_reward(
    state: web::Data<AppState>,
    req: actix_web::HttpRequest,
    path: web::Path<String>,
    body: web::Json<CreateRewardRequest>,
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
            message: "You do not have permission to create rewards".to_string(),
        }));
    }

    let request = body.into_inner();
    if request.name.is_empty() {
        return Ok(HttpResponse::BadRequest().json(ApiError {
            error: "validation_error".to_string(),
            message: "Reward name is required".to_string(),
        }));
    }

    match reward_service::create_reward(&state.db, &household_id, &request).await {
        Ok(reward) => {
            // Log activity
            let details = serde_json::json!({ "name": reward.name }).to_string();
            let _ = activity_logs::log_activity(
                &state.db,
                &household_id,
                &user_id,
                None,
                ActivityType::RewardCreated,
                Some("reward"),
                Some(&reward.id),
                Some(&details),
            ).await;

            Ok(HttpResponse::Created().json(ApiSuccess::new(reward)))
        }
        Err(e) => {
            log::error!("Error creating reward: {:?}", e);
            Ok(HttpResponse::InternalServerError().json(ApiError {
                error: "internal_error".to_string(),
                message: "Failed to create reward".to_string(),
            }))
        }
    }
}

async fn get_reward(
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

    let (household_id_str, reward_id_str) = path.into_inner();

    let household_id = match Uuid::parse_str(&household_id_str) {
        Ok(id) => id,
        Err(_) => {
            return Ok(HttpResponse::BadRequest().json(ApiError {
                error: "invalid_id".to_string(),
                message: "Invalid household ID format".to_string(),
            }));
        }
    };

    let reward_id = match Uuid::parse_str(&reward_id_str) {
        Ok(id) => id,
        Err(_) => {
            return Ok(HttpResponse::BadRequest().json(ApiError {
                error: "invalid_id".to_string(),
                message: "Invalid reward ID format".to_string(),
            }));
        }
    };

    if !household_service::is_member(&state.db, &household_id, &user_id).await.unwrap_or(false) {
        return Ok(HttpResponse::Forbidden().json(ApiError {
            error: "forbidden".to_string(),
            message: "You are not a member of this household".to_string(),
        }));
    }

    match reward_service::get_reward(&state.db, &reward_id).await {
        Ok(Some(reward)) => Ok(HttpResponse::Ok().json(ApiSuccess::new(reward))),
        Ok(None) => Ok(HttpResponse::NotFound().json(ApiError {
            error: "not_found".to_string(),
            message: "Reward not found".to_string(),
        })),
        Err(e) => {
            log::error!("Error fetching reward: {:?}", e);
            Ok(HttpResponse::InternalServerError().json(ApiError {
                error: "internal_error".to_string(),
                message: "Failed to fetch reward".to_string(),
            }))
        }
    }
}

async fn update_reward(
    state: web::Data<AppState>,
    req: actix_web::HttpRequest,
    path: web::Path<(String, String)>,
    body: web::Json<UpdateRewardRequest>,
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

    let (household_id_str, reward_id_str) = path.into_inner();

    let household_id = match Uuid::parse_str(&household_id_str) {
        Ok(id) => id,
        Err(_) => {
            return Ok(HttpResponse::BadRequest().json(ApiError {
                error: "invalid_id".to_string(),
                message: "Invalid household ID format".to_string(),
            }));
        }
    };

    let reward_id = match Uuid::parse_str(&reward_id_str) {
        Ok(id) => id,
        Err(_) => {
            return Ok(HttpResponse::BadRequest().json(ApiError {
                error: "invalid_id".to_string(),
                message: "Invalid reward ID format".to_string(),
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
            message: "You do not have permission to update rewards".to_string(),
        }));
    }

    match reward_service::update_reward(&state.db, &reward_id, &body.into_inner()).await {
        Ok(reward) => Ok(HttpResponse::Ok().json(ApiSuccess::new(reward))),
        Err(e) => {
            log::error!("Error updating reward: {:?}", e);
            Ok(HttpResponse::InternalServerError().json(ApiError {
                error: "internal_error".to_string(),
                message: "Failed to update reward".to_string(),
            }))
        }
    }
}

async fn delete_reward(
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

    let (household_id_str, reward_id_str) = path.into_inner();

    let household_id = match Uuid::parse_str(&household_id_str) {
        Ok(id) => id,
        Err(_) => {
            return Ok(HttpResponse::BadRequest().json(ApiError {
                error: "invalid_id".to_string(),
                message: "Invalid household ID format".to_string(),
            }));
        }
    };

    let reward_id = match Uuid::parse_str(&reward_id_str) {
        Ok(id) => id,
        Err(_) => {
            return Ok(HttpResponse::BadRequest().json(ApiError {
                error: "invalid_id".to_string(),
                message: "Invalid reward ID format".to_string(),
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
            message: "You do not have permission to delete rewards".to_string(),
        }));
    }

    // Get reward details before deletion for logging
    let reward = reward_service::get_reward(&state.db, &reward_id).await.ok().flatten();
    let details = reward.as_ref()
        .map(|r| serde_json::json!({ "name": r.name }).to_string());

    match reward_service::delete_reward(&state.db, &reward_id).await {
        Ok(_) => {
            // Log activity
            let _ = activity_logs::log_activity(
                &state.db,
                &household_id,
                &user_id,
                None,
                ActivityType::RewardDeleted,
                Some("reward"),
                Some(&reward_id),
                details.as_deref(),
            ).await;

            Ok(HttpResponse::NoContent().finish())
        }
        Err(e) => {
            log::error!("Error deleting reward: {:?}", e);
            Ok(HttpResponse::InternalServerError().json(ApiError {
                error: "internal_error".to_string(),
                message: "Failed to delete reward".to_string(),
            }))
        }
    }
}

async fn purchase_reward(
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

    let (household_id_str, reward_id_str) = path.into_inner();

    let household_id = match Uuid::parse_str(&household_id_str) {
        Ok(id) => id,
        Err(_) => {
            return Ok(HttpResponse::BadRequest().json(ApiError {
                error: "invalid_id".to_string(),
                message: "Invalid household ID format".to_string(),
            }));
        }
    };

    let reward_id = match Uuid::parse_str(&reward_id_str) {
        Ok(id) => id,
        Err(_) => {
            return Ok(HttpResponse::BadRequest().json(ApiError {
                error: "invalid_id".to_string(),
                message: "Invalid reward ID format".to_string(),
            }));
        }
    };

    if !household_service::is_member(&state.db, &household_id, &user_id).await.unwrap_or(false) {
        return Ok(HttpResponse::Forbidden().json(ApiError {
            error: "forbidden".to_string(),
            message: "You are not a member of this household".to_string(),
        }));
    }

    // Get reward details for logging
    let reward = reward_service::get_reward(&state.db, &reward_id).await.ok().flatten();
    let details = reward.as_ref()
        .map(|r| serde_json::json!({ "name": r.name }).to_string());

    match reward_service::purchase_reward(&state.db, &reward_id, &user_id, &household_id).await {
        Ok(user_reward) => {
            // Log activity
            let _ = activity_logs::log_activity(
                &state.db,
                &household_id,
                &user_id,
                Some(&user_id),
                ActivityType::RewardPurchased,
                Some("reward"),
                Some(&reward_id),
                details.as_deref(),
            ).await;

            Ok(HttpResponse::Created().json(ApiSuccess::new(user_reward)))
        }
        Err(e) => {
            log::error!("Error purchasing reward: {:?}", e);
            Ok(HttpResponse::BadRequest().json(ApiError {
                error: "purchase_error".to_string(),
                message: e.to_string(),
            }))
        }
    }
}

async fn assign_reward(
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

    let (household_id_str, reward_id_str, target_user_id_str) = path.into_inner();

    let household_id = match Uuid::parse_str(&household_id_str) {
        Ok(id) => id,
        Err(_) => {
            return Ok(HttpResponse::BadRequest().json(ApiError {
                error: "invalid_id".to_string(),
                message: "Invalid household ID format".to_string(),
            }));
        }
    };

    let reward_id = match Uuid::parse_str(&reward_id_str) {
        Ok(id) => id,
        Err(_) => {
            return Ok(HttpResponse::BadRequest().json(ApiError {
                error: "invalid_id".to_string(),
                message: "Invalid reward ID format".to_string(),
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
            message: "You do not have permission to assign rewards".to_string(),
        }));
    }

    // Verify target user is a member
    if !household_service::is_member(&state.db, &household_id, &target_user_id).await.unwrap_or(false) {
        return Ok(HttpResponse::BadRequest().json(ApiError {
            error: "invalid_user".to_string(),
            message: "Target user is not a member of this household".to_string(),
        }));
    }

    // Get reward details for logging
    let reward = reward_service::get_reward(&state.db, &reward_id).await.ok().flatten();
    let details = reward.as_ref()
        .map(|r| serde_json::json!({ "name": r.name }).to_string());

    match reward_service::assign_reward(&state.db, &reward_id, &target_user_id, &household_id).await {
        Ok(user_reward) => {
            // Log activity
            let _ = activity_logs::log_activity(
                &state.db,
                &household_id,
                &current_user_id,
                Some(&target_user_id),
                ActivityType::RewardAssigned,
                Some("reward"),
                Some(&reward_id),
                details.as_deref(),
            ).await;

            Ok(HttpResponse::Created().json(ApiSuccess::new(user_reward)))
        }
        Err(e) => {
            log::error!("Error assigning reward: {:?}", e);
            Ok(HttpResponse::InternalServerError().json(ApiError {
                error: "internal_error".to_string(),
                message: "Failed to assign reward".to_string(),
            }))
        }
    }
}

async fn unassign_reward(
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

    let (household_id_str, reward_id_str, target_user_id_str) = path.into_inner();

    let household_id = match Uuid::parse_str(&household_id_str) {
        Ok(id) => id,
        Err(_) => {
            return Ok(HttpResponse::BadRequest().json(ApiError {
                error: "invalid_id".to_string(),
                message: "Invalid household ID format".to_string(),
            }));
        }
    };

    let reward_id = match Uuid::parse_str(&reward_id_str) {
        Ok(id) => id,
        Err(_) => {
            return Ok(HttpResponse::BadRequest().json(ApiError {
                error: "invalid_id".to_string(),
                message: "Invalid reward ID format".to_string(),
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
            message: "You do not have permission to unassign rewards".to_string(),
        }));
    }

    match reward_service::unassign_reward(&state.db, &reward_id, &target_user_id, &household_id).await {
        Ok(_) => Ok(HttpResponse::Ok().json(ApiSuccess::new(()))),
        Err(e) => {
            log::error!("Error unassigning reward: {:?}", e);
            Ok(HttpResponse::BadRequest().json(ApiError {
                error: "unassign_error".to_string(),
                message: e.to_string(),
            }))
        }
    }
}

async fn list_user_rewards(
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

    match reward_service::list_user_rewards(&state.db, &user_id, &household_id).await {
        Ok(rewards) => Ok(HttpResponse::Ok().json(ApiSuccess::new(rewards))),
        Err(e) => {
            log::error!("Error listing user rewards: {:?}", e);
            Ok(HttpResponse::InternalServerError().json(ApiError {
                error: "internal_error".to_string(),
                message: "Failed to list user rewards".to_string(),
            }))
        }
    }
}

async fn list_all_user_rewards(
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

    match reward_service::list_all_user_rewards_in_household(&state.db, &household_id).await {
        Ok(rewards) => Ok(HttpResponse::Ok().json(ApiSuccess::new(rewards))),
        Err(e) => {
            log::error!("Error listing all user rewards: {:?}", e);
            Ok(HttpResponse::InternalServerError().json(ApiError {
                error: "internal_error".to_string(),
                message: "Failed to list all user rewards".to_string(),
            }))
        }
    }
}

async fn delete_user_reward(
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

    let (household_id_str, user_reward_id_str) = path.into_inner();

    let household_id = match Uuid::parse_str(&household_id_str) {
        Ok(id) => id,
        Err(_) => {
            return Ok(HttpResponse::BadRequest().json(ApiError {
                error: "invalid_id".to_string(),
                message: "Invalid household ID format".to_string(),
            }));
        }
    };

    let user_reward_id = match Uuid::parse_str(&user_reward_id_str) {
        Ok(id) => id,
        Err(_) => {
            return Ok(HttpResponse::BadRequest().json(ApiError {
                error: "invalid_id".to_string(),
                message: "Invalid user reward ID format".to_string(),
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

    // Only users with manage permission can delete user rewards
    let role = household_service::get_member_role(&state.db, &household_id, &user_id).await;
    if !role.as_ref().map(|r| settings.hierarchy_type.can_manage(r)).unwrap_or(false) {
        return Ok(HttpResponse::Forbidden().json(ApiError {
            error: "forbidden".to_string(),
            message: "You do not have permission to remove reward assignments".to_string(),
        }));
    }

    match reward_service::delete_user_reward(&state.db, &user_reward_id).await {
        Ok(_) => Ok(HttpResponse::NoContent().finish()),
        Err(e) => {
            log::error!("Error deleting user reward: {:?}", e);
            Ok(HttpResponse::BadRequest().json(ApiError {
                error: "delete_error".to_string(),
                message: e.to_string(),
            }))
        }
    }
}

async fn redeem_reward(
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

    let (household_id_str, user_reward_id_str) = path.into_inner();

    let household_id = match Uuid::parse_str(&household_id_str) {
        Ok(id) => id,
        Err(_) => {
            return Ok(HttpResponse::BadRequest().json(ApiError {
                error: "invalid_id".to_string(),
                message: "Invalid household ID format".to_string(),
            }));
        }
    };

    let user_reward_id = match Uuid::parse_str(&user_reward_id_str) {
        Ok(id) => id,
        Err(_) => {
            return Ok(HttpResponse::BadRequest().json(ApiError {
                error: "invalid_id".to_string(),
                message: "Invalid user reward ID format".to_string(),
            }));
        }
    };

    if !household_service::is_member(&state.db, &household_id, &user_id).await.unwrap_or(false) {
        return Ok(HttpResponse::Forbidden().json(ApiError {
            error: "forbidden".to_string(),
            message: "You are not a member of this household".to_string(),
        }));
    }

    match reward_service::redeem_reward(&state.db, &user_reward_id, &user_id).await {
        Ok(user_reward) => {
            // Get reward details for logging
            let reward = reward_service::get_reward(&state.db, &user_reward.reward_id).await.ok().flatten();
            let details = reward.as_ref()
                .map(|r| serde_json::json!({ "name": r.name }).to_string());

            // Log activity
            let _ = activity_logs::log_activity(
                &state.db,
                &household_id,
                &user_id,
                Some(&user_id),
                ActivityType::RewardRedeemed,
                Some("reward"),
                Some(&user_reward.reward_id),
                details.as_deref(),
            ).await;

            Ok(HttpResponse::Ok().json(ApiSuccess::new(user_reward)))
        }
        Err(e) => {
            log::error!("Error redeeming reward: {:?}", e);
            Ok(HttpResponse::BadRequest().json(ApiError {
                error: "redeem_error".to_string(),
                message: e.to_string(),
            }))
        }
    }
}
