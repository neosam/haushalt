use actix_web::{web, HttpResponse, Result};
use serde::Deserialize;
use shared::{ApiError, ApiSuccess, Role};
use uuid::Uuid;

use crate::models::AppState;
use crate::services::{activity_logs as activity_service, households as household_service};

#[derive(Debug, Deserialize)]
pub struct ListActivitiesQuery {
    pub limit: Option<i64>,
}

pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/activities")
            .route("", web::get().to(list_activities))
    );
}

/// List activity logs for the household
/// - Owners see all activities
/// - Members see only activities that affect them or were performed by them
async fn list_activities(
    state: web::Data<AppState>,
    req: actix_web::HttpRequest,
    path: web::Path<String>,
    query: web::Query<ListActivitiesQuery>,
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

    // Get user's role to determine visibility
    let role = household_service::get_member_role(&state.db, &household_id, &user_id).await;

    let activities = if role.map(|r| r == Role::Owner).unwrap_or(false) {
        // Owner sees all activities
        activity_service::list_household_activities(&state.db, &household_id, query.limit).await
    } else {
        // Non-owners see only their own activities
        activity_service::list_user_activities(&state.db, &household_id, &user_id, query.limit).await
    };

    match activities {
        Ok(logs) => Ok(HttpResponse::Ok().json(ApiSuccess::new(logs))),
        Err(e) => {
            log::error!("Error fetching activities: {:?}", e);
            Ok(HttpResponse::InternalServerError().json(ApiError {
                error: "internal_error".to_string(),
                message: "Failed to fetch activities".to_string(),
            }))
        }
    }
}
