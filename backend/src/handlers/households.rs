use actix_web::{web, HttpResponse, Result};
use shared::{ActivityType, AdjustPointsRequest, AdjustPointsResponse, ApiError, ApiSuccess, CreateHouseholdRequest, CreateInvitationRequest, UpdateHouseholdRequest, UpdateHouseholdSettingsRequest, UpdateRoleRequest};
use uuid::Uuid;

use crate::models::AppState;
use crate::services::{activity_logs as activity_log_service, households as household_service, household_settings as settings_service, invitations as invitation_service};
use crate::handlers::{tasks, rewards, punishments, point_conditions, activity_logs, chat, notes, announcements};

pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/households")
            .route("", web::get().to(list_households))
            .route("", web::post().to(create_household))
            .route("/{id}", web::get().to(get_household))
            .route("/{id}", web::put().to(update_household))
            .route("/{id}", web::delete().to(delete_household))
            .route("/{id}/members", web::get().to(list_members))
            .route("/{id}/invite", web::post().to(invite_member))
            .route("/{id}/invitations", web::get().to(list_household_invitations))
            .route("/{id}/invitations/{inv_id}", web::delete().to(cancel_invitation))
            .route("/{id}/members/{user_id}", web::delete().to(remove_member))
            .route("/{id}/members/{user_id}/role", web::put().to(update_member_role))
            .route("/{id}/members/{user_id}/points", web::post().to(adjust_member_points))
            .route("/{id}/leaderboard", web::get().to(get_leaderboard))
            .route("/{id}/settings", web::get().to(get_household_settings))
            .route("/{id}/settings", web::put().to(update_household_settings))
            .service(
                web::scope("/{household_id}")
                    .configure(tasks::configure)
                    .configure(rewards::configure)
                    .configure(punishments::configure)
                    .configure(point_conditions::configure)
                    .configure(activity_logs::configure)
                    .configure(chat::configure)
                    .configure(notes::configure)
                    .configure(announcements::configure)
            )
    );
}

async fn list_households(
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

    match household_service::list_user_households(&state.db, &user_id).await {
        Ok(households) => Ok(HttpResponse::Ok().json(ApiSuccess::new(households))),
        Err(e) => {
            log::error!("Error listing households: {:?}", e);
            Ok(HttpResponse::InternalServerError().json(ApiError {
                error: "internal_error".to_string(),
                message: "Failed to list households".to_string(),
            }))
        }
    }
}

async fn create_household(
    state: web::Data<AppState>,
    req: actix_web::HttpRequest,
    body: web::Json<CreateHouseholdRequest>,
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

    let request = body.into_inner();
    if request.name.is_empty() {
        return Ok(HttpResponse::BadRequest().json(ApiError {
            error: "validation_error".to_string(),
            message: "Household name is required".to_string(),
        }));
    }

    match household_service::create_household(&state.db, &user_id, &request).await {
        Ok(household) => Ok(HttpResponse::Created().json(ApiSuccess::new(household))),
        Err(e) => {
            log::error!("Error creating household: {:?}", e);
            Ok(HttpResponse::InternalServerError().json(ApiError {
                error: "internal_error".to_string(),
                message: "Failed to create household".to_string(),
            }))
        }
    }
}

async fn get_household(
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
    if !household_service::is_member(&state.db, &household_id, &user_id).await.unwrap_or(false) {
        return Ok(HttpResponse::Forbidden().json(ApiError {
            error: "forbidden".to_string(),
            message: "You are not a member of this household".to_string(),
        }));
    }

    match household_service::get_household(&state.db, &household_id).await {
        Ok(Some(household)) => Ok(HttpResponse::Ok().json(ApiSuccess::new(household))),
        Ok(None) => Ok(HttpResponse::NotFound().json(ApiError {
            error: "not_found".to_string(),
            message: "Household not found".to_string(),
        })),
        Err(e) => {
            log::error!("Error fetching household: {:?}", e);
            Ok(HttpResponse::InternalServerError().json(ApiError {
                error: "internal_error".to_string(),
                message: "Failed to fetch household".to_string(),
            }))
        }
    }
}

async fn update_household(
    state: web::Data<AppState>,
    req: actix_web::HttpRequest,
    path: web::Path<String>,
    body: web::Json<UpdateHouseholdRequest>,
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

    // Check if user is owner or admin
    let role = household_service::get_member_role(&state.db, &household_id, &user_id).await;
    if !role.map(|r| r.can_manage_tasks()).unwrap_or(false) {
        return Ok(HttpResponse::Forbidden().json(ApiError {
            error: "forbidden".to_string(),
            message: "Only owners and admins can update household settings".to_string(),
        }));
    }

    match household_service::update_household(&state.db, &household_id, &body.into_inner()).await {
        Ok(household) => Ok(HttpResponse::Ok().json(ApiSuccess::new(household))),
        Err(e) => {
            log::error!("Error updating household: {:?}", e);
            Ok(HttpResponse::InternalServerError().json(ApiError {
                error: "internal_error".to_string(),
                message: "Failed to update household".to_string(),
            }))
        }
    }
}

async fn delete_household(
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

    // Check if user is owner
    let role = household_service::get_member_role(&state.db, &household_id, &user_id).await;
    if !role.map(|r| r.can_delete_household()).unwrap_or(false) {
        return Ok(HttpResponse::Forbidden().json(ApiError {
            error: "forbidden".to_string(),
            message: "Only owners can delete households".to_string(),
        }));
    }

    match household_service::delete_household(&state.db, &household_id).await {
        Ok(_) => Ok(HttpResponse::NoContent().finish()),
        Err(e) => {
            log::error!("Error deleting household: {:?}", e);
            Ok(HttpResponse::InternalServerError().json(ApiError {
                error: "internal_error".to_string(),
                message: "Failed to delete household".to_string(),
            }))
        }
    }
}

async fn list_members(
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
    if !household_service::is_member(&state.db, &household_id, &user_id).await.unwrap_or(false) {
        return Ok(HttpResponse::Forbidden().json(ApiError {
            error: "forbidden".to_string(),
            message: "You are not a member of this household".to_string(),
        }));
    }

    match household_service::list_members(&state.db, &household_id).await {
        Ok(members) => Ok(HttpResponse::Ok().json(ApiSuccess::new(members))),
        Err(e) => {
            log::error!("Error listing members: {:?}", e);
            Ok(HttpResponse::InternalServerError().json(ApiError {
                error: "internal_error".to_string(),
                message: "Failed to list members".to_string(),
            }))
        }
    }
}

async fn invite_member(
    state: web::Data<AppState>,
    req: actix_web::HttpRequest,
    path: web::Path<String>,
    body: web::Json<CreateInvitationRequest>,
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

    // Check if user can manage members
    let role = household_service::get_member_role(&state.db, &household_id, &user_id).await;
    if !role.map(|r| r.can_manage_members()).unwrap_or(false) {
        return Ok(HttpResponse::Forbidden().json(ApiError {
            error: "forbidden".to_string(),
            message: "Only owners and admins can invite members".to_string(),
        }));
    }

    let request = body.into_inner();
    let member_role = request.role.unwrap_or(shared::Role::Member);

    // Only owner can invite as admins
    if member_role == shared::Role::Admin && !role.map(|r| r == shared::Role::Owner).unwrap_or(false) {
        return Ok(HttpResponse::Forbidden().json(ApiError {
            error: "forbidden".to_string(),
            message: "Only owners can invite as admin".to_string(),
        }));
    }

    // Cannot invite as owner
    if member_role == shared::Role::Owner {
        return Ok(HttpResponse::BadRequest().json(ApiError {
            error: "invalid_role".to_string(),
            message: "Cannot invite as owner".to_string(),
        }));
    }

    match invitation_service::create_invitation(&state.db, &household_id, &request.email, member_role, &user_id).await {
        Ok(invitation) => {
            // Log activity
            let details = serde_json::json!({ "email": request.email }).to_string();
            let _ = activity_log_service::log_activity(
                &state.db,
                &household_id,
                &user_id,
                None,
                ActivityType::InvitationSent,
                Some("invitation"),
                Some(&invitation.id),
                Some(&details),
            ).await;

            Ok(HttpResponse::Created().json(ApiSuccess::new(invitation)))
        }
        Err(invitation_service::InvitationError::AlreadyExists) => {
            Ok(HttpResponse::BadRequest().json(ApiError {
                error: "already_invited".to_string(),
                message: "User already has a pending invitation".to_string(),
            }))
        }
        Err(invitation_service::InvitationError::AlreadyMember) => {
            Ok(HttpResponse::BadRequest().json(ApiError {
                error: "already_member".to_string(),
                message: "User is already a member of this household".to_string(),
            }))
        }
        Err(e) => {
            log::error!("Error creating invitation: {:?}", e);
            Ok(HttpResponse::InternalServerError().json(ApiError {
                error: "internal_error".to_string(),
                message: "Failed to create invitation".to_string(),
            }))
        }
    }
}

/// List pending invitations for a household
async fn list_household_invitations(
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

    // Check if user can manage members
    let role = household_service::get_member_role(&state.db, &household_id, &user_id).await;
    if !role.map(|r| r.can_manage_members()).unwrap_or(false) {
        return Ok(HttpResponse::Forbidden().json(ApiError {
            error: "forbidden".to_string(),
            message: "Only owners and admins can view invitations".to_string(),
        }));
    }

    match invitation_service::get_household_invitations(&state.db, &household_id).await {
        Ok(invitations) => Ok(HttpResponse::Ok().json(ApiSuccess::new(invitations))),
        Err(e) => {
            log::error!("Error listing invitations: {:?}", e);
            Ok(HttpResponse::InternalServerError().json(ApiError {
                error: "internal_error".to_string(),
                message: "Failed to list invitations".to_string(),
            }))
        }
    }
}

/// Cancel a pending invitation
async fn cancel_invitation(
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

    let (household_id_str, invitation_id_str) = path.into_inner();

    let household_id = match Uuid::parse_str(&household_id_str) {
        Ok(id) => id,
        Err(_) => {
            return Ok(HttpResponse::BadRequest().json(ApiError {
                error: "invalid_id".to_string(),
                message: "Invalid household ID format".to_string(),
            }));
        }
    };

    let invitation_id = match Uuid::parse_str(&invitation_id_str) {
        Ok(id) => id,
        Err(_) => {
            return Ok(HttpResponse::BadRequest().json(ApiError {
                error: "invalid_id".to_string(),
                message: "Invalid invitation ID format".to_string(),
            }));
        }
    };

    // Check if user can manage members
    let role = household_service::get_member_role(&state.db, &household_id, &user_id).await;
    if !role.map(|r| r.can_manage_members()).unwrap_or(false) {
        return Ok(HttpResponse::Forbidden().json(ApiError {
            error: "forbidden".to_string(),
            message: "Only owners and admins can cancel invitations".to_string(),
        }));
    }

    // Verify invitation belongs to this household
    let invitation = match invitation_service::get_invitation(&state.db, &invitation_id).await {
        Ok(inv) => inv,
        Err(invitation_service::InvitationError::NotFound) => {
            return Ok(HttpResponse::NotFound().json(ApiError {
                error: "not_found".to_string(),
                message: "Invitation not found".to_string(),
            }));
        }
        Err(e) => {
            log::error!("Error fetching invitation: {:?}", e);
            return Ok(HttpResponse::InternalServerError().json(ApiError {
                error: "internal_error".to_string(),
                message: "Failed to fetch invitation".to_string(),
            }));
        }
    };

    if invitation.household_id != household_id {
        return Ok(HttpResponse::NotFound().json(ApiError {
            error: "not_found".to_string(),
            message: "Invitation not found".to_string(),
        }));
    }

    match invitation_service::cancel_invitation(&state.db, &invitation_id).await {
        Ok(_) => Ok(HttpResponse::NoContent().finish()),
        Err(invitation_service::InvitationError::NotFound) => {
            Ok(HttpResponse::NotFound().json(ApiError {
                error: "not_found".to_string(),
                message: "Invitation not found or already responded".to_string(),
            }))
        }
        Err(e) => {
            log::error!("Error canceling invitation: {:?}", e);
            Ok(HttpResponse::InternalServerError().json(ApiError {
                error: "internal_error".to_string(),
                message: "Failed to cancel invitation".to_string(),
            }))
        }
    }
}

async fn remove_member(
    state: web::Data<AppState>,
    req: actix_web::HttpRequest,
    path: web::Path<(String, String)>,
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

    let (household_id_str, target_user_id_str) = path.into_inner();

    let household_id = match Uuid::parse_str(&household_id_str) {
        Ok(id) => id,
        Err(_) => {
            return Ok(HttpResponse::BadRequest().json(ApiError {
                error: "invalid_id".to_string(),
                message: "Invalid household ID format".to_string(),
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

    // Check permissions
    let role = household_service::get_member_role(&state.db, &household_id, &current_user_id).await;
    let target_role = household_service::get_member_role(&state.db, &household_id, &target_user_id).await;

    // Users can leave by removing themselves
    let is_self_removal = current_user_id == target_user_id;

    // Cannot remove the owner
    if target_role == Some(shared::Role::Owner) {
        return Ok(HttpResponse::BadRequest().json(ApiError {
            error: "cannot_remove_owner".to_string(),
            message: "Cannot remove the owner from the household".to_string(),
        }));
    }

    // Must be able to manage members or be removing self
    if !is_self_removal && !role.map(|r| r.can_manage_members()).unwrap_or(false) {
        return Ok(HttpResponse::Forbidden().json(ApiError {
            error: "forbidden".to_string(),
            message: "Only owners and admins can remove members".to_string(),
        }));
    }

    match household_service::remove_member(&state.db, &household_id, &target_user_id).await {
        Ok(_) => {
            // Log activity
            let _ = activity_log_service::log_activity(
                &state.db,
                &household_id,
                &current_user_id,
                if is_self_removal { None } else { Some(&target_user_id) },
                ActivityType::MemberLeft,
                Some("member"),
                None,
                None,
            ).await;

            Ok(HttpResponse::NoContent().finish())
        }
        Err(e) => {
            log::error!("Error removing member: {:?}", e);
            Ok(HttpResponse::InternalServerError().json(ApiError {
                error: "internal_error".to_string(),
                message: "Failed to remove member".to_string(),
            }))
        }
    }
}

async fn update_member_role(
    state: web::Data<AppState>,
    req: actix_web::HttpRequest,
    path: web::Path<(String, String)>,
    body: web::Json<UpdateRoleRequest>,
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

    let (household_id_str, target_user_id_str) = path.into_inner();

    let household_id = match Uuid::parse_str(&household_id_str) {
        Ok(id) => id,
        Err(_) => {
            return Ok(HttpResponse::BadRequest().json(ApiError {
                error: "invalid_id".to_string(),
                message: "Invalid household ID format".to_string(),
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

    // Only owner can change roles
    let role = household_service::get_member_role(&state.db, &household_id, &current_user_id).await;
    if !role.map(|r| r.can_manage_roles()).unwrap_or(false) {
        return Ok(HttpResponse::Forbidden().json(ApiError {
            error: "forbidden".to_string(),
            message: "Only owners can change member roles".to_string(),
        }));
    }

    let new_role = body.into_inner().role;

    // Cannot change to or from owner
    if new_role == shared::Role::Owner {
        return Ok(HttpResponse::BadRequest().json(ApiError {
            error: "invalid_role".to_string(),
            message: "Cannot change role to owner".to_string(),
        }));
    }

    let target_role = household_service::get_member_role(&state.db, &household_id, &target_user_id).await;
    if target_role == Some(shared::Role::Owner) {
        return Ok(HttpResponse::BadRequest().json(ApiError {
            error: "invalid_role".to_string(),
            message: "Cannot change owner's role".to_string(),
        }));
    }

    match household_service::update_member_role(&state.db, &household_id, &target_user_id, new_role.clone()).await {
        Ok(membership) => {
            // Log activity
            let details = serde_json::json!({ "new_role": format!("{:?}", new_role) }).to_string();
            let _ = activity_log_service::log_activity(
                &state.db,
                &household_id,
                &current_user_id,
                Some(&target_user_id),
                ActivityType::MemberRoleChanged,
                Some("member"),
                None,
                Some(&details),
            ).await;

            Ok(HttpResponse::Ok().json(ApiSuccess::new(membership)))
        }
        Err(e) => {
            log::error!("Error updating role: {:?}", e);
            Ok(HttpResponse::InternalServerError().json(ApiError {
                error: "internal_error".to_string(),
                message: "Failed to update role".to_string(),
            }))
        }
    }
}

async fn get_leaderboard(
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
    if !household_service::is_member(&state.db, &household_id, &user_id).await.unwrap_or(false) {
        return Ok(HttpResponse::Forbidden().json(ApiError {
            error: "forbidden".to_string(),
            message: "You are not a member of this household".to_string(),
        }));
    }

    match household_service::get_leaderboard(&state.db, &household_id).await {
        Ok(leaderboard) => Ok(HttpResponse::Ok().json(ApiSuccess::new(leaderboard))),
        Err(e) => {
            log::error!("Error fetching leaderboard: {:?}", e);
            Ok(HttpResponse::InternalServerError().json(ApiError {
                error: "internal_error".to_string(),
                message: "Failed to fetch leaderboard".to_string(),
            }))
        }
    }
}

/// Manually adjust a member's points (add or remove)
async fn adjust_member_points(
    state: web::Data<AppState>,
    req: actix_web::HttpRequest,
    path: web::Path<(String, String)>,
    body: web::Json<AdjustPointsRequest>,
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

    let (household_id_str, target_user_id_str) = path.into_inner();

    let household_id = match Uuid::parse_str(&household_id_str) {
        Ok(id) => id,
        Err(_) => {
            return Ok(HttpResponse::BadRequest().json(ApiError {
                error: "invalid_id".to_string(),
                message: "Invalid household ID format".to_string(),
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

    // Only owner and admin can adjust points
    let role = household_service::get_member_role(&state.db, &household_id, &current_user_id).await;
    if !role.map(|r| r.can_manage_members()).unwrap_or(false) {
        return Ok(HttpResponse::Forbidden().json(ApiError {
            error: "forbidden".to_string(),
            message: "Only owners and admins can adjust member points".to_string(),
        }));
    }

    // Verify target user is a member
    if !household_service::is_member(&state.db, &household_id, &target_user_id).await.unwrap_or(false) {
        return Ok(HttpResponse::BadRequest().json(ApiError {
            error: "invalid_user".to_string(),
            message: "Target user is not a member of this household".to_string(),
        }));
    }

    let request = body.into_inner();

    match household_service::update_member_points(&state.db, &household_id, &target_user_id, request.points).await {
        Ok(new_points) => {
            // Log activity
            let details = serde_json::json!({ "points": request.points }).to_string();
            let _ = activity_log_service::log_activity(
                &state.db,
                &household_id,
                &current_user_id,
                Some(&target_user_id),
                ActivityType::PointsAdjusted,
                Some("member"),
                None,
                Some(&details),
            ).await;

            Ok(HttpResponse::Ok().json(ApiSuccess::new(AdjustPointsResponse { new_points })))
        }
        Err(e) => {
            log::error!("Error adjusting points: {:?}", e);
            Ok(HttpResponse::InternalServerError().json(ApiError {
                error: "internal_error".to_string(),
                message: "Failed to adjust points".to_string(),
            }))
        }
    }
}

/// Get household settings
async fn get_household_settings(
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

    // Check membership - any member can view settings
    if !household_service::is_member(&state.db, &household_id, &user_id).await.unwrap_or(false) {
        return Ok(HttpResponse::Forbidden().json(ApiError {
            error: "forbidden".to_string(),
            message: "You are not a member of this household".to_string(),
        }));
    }

    match settings_service::get_or_create_settings(&state.db, &household_id).await {
        Ok(settings) => Ok(HttpResponse::Ok().json(ApiSuccess::new(settings))),
        Err(e) => {
            log::error!("Error fetching settings: {:?}", e);
            Ok(HttpResponse::InternalServerError().json(ApiError {
                error: "internal_error".to_string(),
                message: "Failed to fetch settings".to_string(),
            }))
        }
    }
}

/// Update household settings (owner only)
async fn update_household_settings(
    state: web::Data<AppState>,
    req: actix_web::HttpRequest,
    path: web::Path<String>,
    body: web::Json<UpdateHouseholdSettingsRequest>,
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

    // Only owner can modify settings
    let role = household_service::get_member_role(&state.db, &household_id, &user_id).await;
    if !role.map(|r| r == shared::Role::Owner).unwrap_or(false) {
        return Ok(HttpResponse::Forbidden().json(ApiError {
            error: "forbidden".to_string(),
            message: "Only owners can modify household settings".to_string(),
        }));
    }

    match settings_service::update_settings(&state.db, &household_id, &body.into_inner()).await {
        Ok(settings) => {
            // Log activity
            let _ = activity_log_service::log_activity(
                &state.db,
                &household_id,
                &user_id,
                None,
                ActivityType::SettingsChanged,
                Some("settings"),
                None,
                None,
            ).await;

            Ok(HttpResponse::Ok().json(ApiSuccess::new(settings)))
        }
        Err(e) => {
            log::error!("Error updating settings: {:?}", e);
            Ok(HttpResponse::InternalServerError().json(ApiError {
                error: "internal_error".to_string(),
                message: "Failed to update settings".to_string(),
            }))
        }
    }
}
