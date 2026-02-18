use actix_web::{web, HttpResponse, Result};
use shared::{
    ApiError, ApiSuccess, DashboardTaskWithHousehold, DashboardTasksResponse,
    DashboardTasksWithStatusResponse, IsTaskOnDashboardResponse,
};
use uuid::Uuid;

use crate::models::AppState;
use crate::services::households as household_service;
use crate::services::tasks as task_service;

pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/dashboard")
            .route("/tasks", web::get().to(get_dashboard_task_ids))
            .route("/tasks/details", web::get().to(get_dashboard_tasks_with_status))
            .route("/tasks/all", web::get().to(get_all_tasks_across_households))
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

/// Get all dashboard tasks with their full status information
async fn get_dashboard_tasks_with_status(
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

    match task_service::get_dashboard_tasks_with_status(&state.db, &user_id).await {
        Ok(tasks_with_households) => {
            let mut response_tasks = Vec::new();
            for (task_status, household_id) in tasks_with_households {
                let household_name =
                    match household_service::get_household(&state.db, &household_id).await {
                        Ok(Some(h)) => h.name,
                        _ => "Unknown".to_string(),
                    };
                response_tasks.push(DashboardTaskWithHousehold {
                    task_with_status: task_status,
                    household_id,
                    household_name,
                });
            }
            Ok(HttpResponse::Ok()
                .json(ApiSuccess::new(DashboardTasksWithStatusResponse { tasks: response_tasks })))
        }
        Err(e) => Ok(HttpResponse::InternalServerError().json(ApiError {
            error: "internal_error".to_string(),
            message: e.to_string(),
        })),
    }
}

/// Get all tasks from all households the user is a member of
/// Used by the "Show all" toggle on the dashboard
async fn get_all_tasks_across_households(
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

    match task_service::get_all_tasks_across_households(&state.db, &user_id).await {
        Ok(tasks_with_households) => {
            let mut response_tasks = Vec::new();
            for (task_status, household_id) in tasks_with_households {
                let household_name =
                    match household_service::get_household(&state.db, &household_id).await {
                        Ok(Some(h)) => h.name,
                        _ => "Unknown".to_string(),
                    };
                response_tasks.push(DashboardTaskWithHousehold {
                    task_with_status: task_status,
                    household_id,
                    household_name,
                });
            }
            Ok(HttpResponse::Ok()
                .json(ApiSuccess::new(DashboardTasksWithStatusResponse { tasks: response_tasks })))
        }
        Err(e) => Ok(HttpResponse::InternalServerError().json(ApiError {
            error: "internal_error".to_string(),
            message: e.to_string(),
        })),
    }
}
