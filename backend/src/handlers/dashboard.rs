use actix_web::{web, HttpResponse, Result};
use shared::{ApiError, ApiSuccess, DashboardTasksResponse, IsTaskOnDashboardResponse};
use uuid::Uuid;

use crate::models::AppState;
use crate::services::tasks as task_service;

pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/dashboard")
            .route("/tasks", web::get().to(get_dashboard_task_ids))
            .route("/tasks/{task_id}", web::get().to(is_task_on_dashboard))
            .route("/tasks/{task_id}", web::post().to(add_task_to_dashboard))
            .route("/tasks/{task_id}", web::delete().to(remove_task_from_dashboard)),
    );
}

/// Get all task IDs that the user has added to their dashboard
async fn get_dashboard_task_ids(
    state: web::Data<AppState>,
    req: actix_web::HttpRequest,
) -> Result<HttpResponse> {
    let user_id = match crate::middleware::auth::extract_user_id(&req, &state.config.jwt_secret) {
        Ok(id) => id,
        Err(_) => {
            return Ok(HttpResponse::Unauthorized().json(ApiError {
                error: "unauthorized".to_string(),
                message: "Invalid or missing authentication".to_string(),
            }));
        }
    };

    let user_id_str = user_id.to_string();
    match task_service::get_dashboard_task_ids(&state.db, &user_id_str).await {
        Ok(ids) => {
            let task_ids: Vec<Uuid> = ids
                .iter()
                .filter_map(|id| Uuid::parse_str(id).ok())
                .collect();
            Ok(HttpResponse::Ok().json(ApiSuccess::new(DashboardTasksResponse { task_ids })))
        }
        Err(e) => Ok(HttpResponse::InternalServerError().json(ApiError {
            error: "internal_error".to_string(),
            message: e.to_string(),
        })),
    }
}

/// Check if a specific task is on the user's dashboard
async fn is_task_on_dashboard(
    state: web::Data<AppState>,
    req: actix_web::HttpRequest,
    path: web::Path<String>,
) -> Result<HttpResponse> {
    let user_id = match crate::middleware::auth::extract_user_id(&req, &state.config.jwt_secret) {
        Ok(id) => id,
        Err(_) => {
            return Ok(HttpResponse::Unauthorized().json(ApiError {
                error: "unauthorized".to_string(),
                message: "Invalid or missing authentication".to_string(),
            }));
        }
    };

    let task_id = path.into_inner();
    let user_id_str = user_id.to_string();

    match task_service::is_task_on_dashboard(&state.db, &user_id_str, &task_id).await {
        Ok(on_dashboard) => {
            Ok(HttpResponse::Ok().json(ApiSuccess::new(IsTaskOnDashboardResponse { on_dashboard })))
        }
        Err(e) => Ok(HttpResponse::InternalServerError().json(ApiError {
            error: "internal_error".to_string(),
            message: e.to_string(),
        })),
    }
}

/// Add a task to the user's dashboard
async fn add_task_to_dashboard(
    state: web::Data<AppState>,
    req: actix_web::HttpRequest,
    path: web::Path<String>,
) -> Result<HttpResponse> {
    let user_id = match crate::middleware::auth::extract_user_id(&req, &state.config.jwt_secret) {
        Ok(id) => id,
        Err(_) => {
            return Ok(HttpResponse::Unauthorized().json(ApiError {
                error: "unauthorized".to_string(),
                message: "Invalid or missing authentication".to_string(),
            }));
        }
    };

    let task_id = path.into_inner();
    let user_id_str = user_id.to_string();

    match task_service::add_task_to_dashboard(&state.db, &user_id_str, &task_id).await {
        Ok(()) => Ok(HttpResponse::Ok().json(ApiSuccess::new(()))),
        Err(e) => Ok(HttpResponse::InternalServerError().json(ApiError {
            error: "internal_error".to_string(),
            message: e.to_string(),
        })),
    }
}

/// Remove a task from the user's dashboard
async fn remove_task_from_dashboard(
    state: web::Data<AppState>,
    req: actix_web::HttpRequest,
    path: web::Path<String>,
) -> Result<HttpResponse> {
    let user_id = match crate::middleware::auth::extract_user_id(&req, &state.config.jwt_secret) {
        Ok(id) => id,
        Err(_) => {
            return Ok(HttpResponse::Unauthorized().json(ApiError {
                error: "unauthorized".to_string(),
                message: "Invalid or missing authentication".to_string(),
            }));
        }
    };

    let task_id = path.into_inner();
    let user_id_str = user_id.to_string();

    match task_service::remove_task_from_dashboard(&state.db, &user_id_str, &task_id).await {
        Ok(()) => Ok(HttpResponse::Ok().json(ApiSuccess::new(()))),
        Err(e) => Ok(HttpResponse::InternalServerError().json(ApiError {
            error: "internal_error".to_string(),
            message: e.to_string(),
        })),
    }
}
