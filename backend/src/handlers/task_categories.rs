use actix_web::{web, HttpResponse, Result};
use shared::{ApiError, ApiSuccess, CreateTaskCategoryRequest, TaskCategoriesResponse, UpdateTaskCategoryRequest};
use uuid::Uuid;

use crate::models::AppState;
use crate::services::{households as household_service, task_categories as category_service};

pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/categories")
            .route("", web::get().to(list_categories))
            .route("", web::post().to(create_category))
            .route("/{category_id}", web::get().to(get_category))
            .route("/{category_id}", web::put().to(update_category))
            .route("/{category_id}", web::delete().to(delete_category)),
    );
}

async fn list_categories(
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

    if !household_service::is_member(&state.db, &household_id, &user_id)
        .await
        .unwrap_or(false)
    {
        return Ok(HttpResponse::Forbidden().json(ApiError {
            error: "forbidden".to_string(),
            message: "You are not a member of this household".to_string(),
        }));
    }

    match category_service::list_categories(&state.db, &household_id).await {
        Ok(categories) => {
            Ok(HttpResponse::Ok().json(ApiSuccess::new(TaskCategoriesResponse { categories })))
        }
        Err(e) => {
            log::error!("Error listing categories: {:?}", e);
            Ok(HttpResponse::InternalServerError().json(ApiError {
                error: "internal_error".to_string(),
                message: "Failed to list categories".to_string(),
            }))
        }
    }
}

async fn create_category(
    state: web::Data<AppState>,
    req: actix_web::HttpRequest,
    path: web::Path<String>,
    body: web::Json<CreateTaskCategoryRequest>,
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

    // Check membership and admin role
    let role = match household_service::get_member_role(&state.db, &household_id, &user_id).await {
        Some(r) => r,
        None => {
            return Ok(HttpResponse::Forbidden().json(ApiError {
                error: "forbidden".to_string(),
                message: "You are not a member of this household".to_string(),
            }));
        }
    };

    if !role.can_manage_tasks() {
        return Ok(HttpResponse::Forbidden().json(ApiError {
            error: "forbidden".to_string(),
            message: "You don't have permission to manage categories".to_string(),
        }));
    }

    match category_service::create_category(&state.db, &household_id, &body).await {
        Ok(category) => Ok(HttpResponse::Created().json(ApiSuccess::new(category))),
        Err(category_service::TaskCategoryError::DuplicateName) => {
            Ok(HttpResponse::Conflict().json(ApiError {
                error: "duplicate_name".to_string(),
                message: "A category with this name already exists".to_string(),
            }))
        }
        Err(e) => {
            log::error!("Error creating category: {:?}", e);
            Ok(HttpResponse::InternalServerError().json(ApiError {
                error: "internal_error".to_string(),
                message: "Failed to create category".to_string(),
            }))
        }
    }
}

async fn get_category(
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

    let (household_id_str, category_id_str) = path.into_inner();
    let household_id = match Uuid::parse_str(&household_id_str) {
        Ok(id) => id,
        Err(_) => {
            return Ok(HttpResponse::BadRequest().json(ApiError {
                error: "invalid_id".to_string(),
                message: "Invalid household ID format".to_string(),
            }));
        }
    };
    let category_id = match Uuid::parse_str(&category_id_str) {
        Ok(id) => id,
        Err(_) => {
            return Ok(HttpResponse::BadRequest().json(ApiError {
                error: "invalid_id".to_string(),
                message: "Invalid category ID format".to_string(),
            }));
        }
    };

    if !household_service::is_member(&state.db, &household_id, &user_id)
        .await
        .unwrap_or(false)
    {
        return Ok(HttpResponse::Forbidden().json(ApiError {
            error: "forbidden".to_string(),
            message: "You are not a member of this household".to_string(),
        }));
    }

    match category_service::get_category(&state.db, &category_id).await {
        Ok(Some(category)) => Ok(HttpResponse::Ok().json(ApiSuccess::new(category))),
        Ok(None) => Ok(HttpResponse::NotFound().json(ApiError {
            error: "not_found".to_string(),
            message: "Category not found".to_string(),
        })),
        Err(e) => {
            log::error!("Error getting category: {:?}", e);
            Ok(HttpResponse::InternalServerError().json(ApiError {
                error: "internal_error".to_string(),
                message: "Failed to get category".to_string(),
            }))
        }
    }
}

async fn update_category(
    state: web::Data<AppState>,
    req: actix_web::HttpRequest,
    path: web::Path<(String, String)>,
    body: web::Json<UpdateTaskCategoryRequest>,
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

    let (household_id_str, category_id_str) = path.into_inner();
    let household_id = match Uuid::parse_str(&household_id_str) {
        Ok(id) => id,
        Err(_) => {
            return Ok(HttpResponse::BadRequest().json(ApiError {
                error: "invalid_id".to_string(),
                message: "Invalid household ID format".to_string(),
            }));
        }
    };
    let category_id = match Uuid::parse_str(&category_id_str) {
        Ok(id) => id,
        Err(_) => {
            return Ok(HttpResponse::BadRequest().json(ApiError {
                error: "invalid_id".to_string(),
                message: "Invalid category ID format".to_string(),
            }));
        }
    };

    let role = match household_service::get_member_role(&state.db, &household_id, &user_id).await {
        Some(r) => r,
        None => {
            return Ok(HttpResponse::Forbidden().json(ApiError {
                error: "forbidden".to_string(),
                message: "You are not a member of this household".to_string(),
            }));
        }
    };

    if !role.can_manage_tasks() {
        return Ok(HttpResponse::Forbidden().json(ApiError {
            error: "forbidden".to_string(),
            message: "You don't have permission to manage categories".to_string(),
        }));
    }

    match category_service::update_category(&state.db, &category_id, &body).await {
        Ok(category) => Ok(HttpResponse::Ok().json(ApiSuccess::new(category))),
        Err(category_service::TaskCategoryError::NotFound) => {
            Ok(HttpResponse::NotFound().json(ApiError {
                error: "not_found".to_string(),
                message: "Category not found".to_string(),
            }))
        }
        Err(category_service::TaskCategoryError::DuplicateName) => {
            Ok(HttpResponse::Conflict().json(ApiError {
                error: "duplicate_name".to_string(),
                message: "A category with this name already exists".to_string(),
            }))
        }
        Err(e) => {
            log::error!("Error updating category: {:?}", e);
            Ok(HttpResponse::InternalServerError().json(ApiError {
                error: "internal_error".to_string(),
                message: "Failed to update category".to_string(),
            }))
        }
    }
}

async fn delete_category(
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

    let (household_id_str, category_id_str) = path.into_inner();
    let household_id = match Uuid::parse_str(&household_id_str) {
        Ok(id) => id,
        Err(_) => {
            return Ok(HttpResponse::BadRequest().json(ApiError {
                error: "invalid_id".to_string(),
                message: "Invalid household ID format".to_string(),
            }));
        }
    };
    let category_id = match Uuid::parse_str(&category_id_str) {
        Ok(id) => id,
        Err(_) => {
            return Ok(HttpResponse::BadRequest().json(ApiError {
                error: "invalid_id".to_string(),
                message: "Invalid category ID format".to_string(),
            }));
        }
    };

    let role = match household_service::get_member_role(&state.db, &household_id, &user_id).await {
        Some(r) => r,
        None => {
            return Ok(HttpResponse::Forbidden().json(ApiError {
                error: "forbidden".to_string(),
                message: "You are not a member of this household".to_string(),
            }));
        }
    };

    if !role.can_manage_tasks() {
        return Ok(HttpResponse::Forbidden().json(ApiError {
            error: "forbidden".to_string(),
            message: "You don't have permission to manage categories".to_string(),
        }));
    }

    match category_service::delete_category(&state.db, &category_id).await {
        Ok(()) => Ok(HttpResponse::NoContent().finish()),
        Err(category_service::TaskCategoryError::NotFound) => {
            Ok(HttpResponse::NotFound().json(ApiError {
                error: "not_found".to_string(),
                message: "Category not found".to_string(),
            }))
        }
        Err(e) => {
            log::error!("Error deleting category: {:?}", e);
            Ok(HttpResponse::InternalServerError().json(ApiError {
                error: "internal_error".to_string(),
                message: "Failed to delete category".to_string(),
            }))
        }
    }
}
