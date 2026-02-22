use actix_web::{web, HttpResponse, Result};
use serde::Deserialize;
use shared::{
    ActivityType, ApiError, ApiSuccess, CreateTaskRequest, HierarchyType,
    RecurrenceType, RecurrenceValue, Task, UpdateTaskRequest,
};
use uuid::Uuid;

use crate::models::AppState;
use crate::services::{
    activity_logs,
    household_settings,
    households as household_service,
    solo_mode,
    task_consequences,
    tasks as task_service,
};

#[derive(Debug, Deserialize)]
struct AddLinkQuery {
    #[serde(default = "default_amount")]
    amount: i32,
}

fn default_amount() -> i32 {
    1
}

/// Check if this is a valid "Set Date" request in Solo Mode.
/// Only allows setting a date on an unscheduled task, with no other field changes.
fn is_solo_mode_set_date_request(request: &UpdateTaskRequest, task: &Task) -> bool {
    // 1. Task must currently be unscheduled (OneTime with no recurrence_value)
    let is_unscheduled = task.recurrence_type == RecurrenceType::OneTime
        && task.recurrence_value.is_none();

    if !is_unscheduled {
        return false; // Already has a schedule - cannot change in Solo Mode
    }

    // 2. Request must set recurrence_type to Custom
    let sets_custom = matches!(request.recurrence_type, Some(RecurrenceType::Custom));

    // 3. Request must provide recurrence_value with dates
    let sets_dates = matches!(
        request.recurrence_value,
        Some(RecurrenceValue::CustomDates(_))
    );

    // 4. All other fields must be None (no other changes allowed)
    let no_other_changes = request.title.is_none()
        && request.description.is_none()
        && request.assigned_user_id.is_none()
        && request.target_count.is_none()
        && request.time_period.is_none()
        && request.allow_exceed_target.is_none()
        && request.requires_review.is_none()
        && request.points_reward.is_none()
        && request.points_penalty.is_none()
        && request.due_time.is_none()
        && request.habit_type.is_none()
        && request.category_id.is_none()
        && request.archived.is_none()
        && request.paused.is_none();

    sets_custom && sets_dates && no_other_changes
}

pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/tasks")
            .route("", web::get().to(list_tasks))
            .route("", web::post().to(create_task))
            .route("/due", web::get().to(get_due_tasks))
            .route("/all", web::get().to(get_all_tasks_with_status))
            .route("/assigned-to-me", web::get().to(get_assigned_tasks))
            .route("/pending-reviews", web::get().to(get_pending_reviews))
            .route("/archived", web::get().to(list_archived_tasks))
            // Review endpoints (must come before /{task_id} routes)
            .route("/completions/{completion_id}/approve", web::post().to(approve_completion))
            .route("/completions/{completion_id}/reject", web::post().to(reject_completion))
            // Suggestion endpoints (must come before /{task_id} routes)
            .route("/suggestions", web::get().to(list_suggestions))
            // Task CRUD (/{task_id} routes must come last as they're catch-all patterns)
            .route("/{task_id}", web::get().to(get_task))
            .route("/{task_id}", web::put().to(update_task))
            .route("/{task_id}", web::delete().to(delete_task))
            .route("/{task_id}/details", web::get().to(get_task_details))
            .route("/{task_id}/complete", web::post().to(complete_task))
            .route("/{task_id}/uncomplete", web::post().to(uncomplete_task))
            .route("/{task_id}/archive", web::post().to(archive_task))
            .route("/{task_id}/unarchive", web::post().to(unarchive_task))
            .route("/{task_id}/pause", web::post().to(pause_task))
            .route("/{task_id}/unpause", web::post().to(unpause_task))
            .route("/{task_id}/approve", web::post().to(approve_suggestion))
            .route("/{task_id}/deny", web::post().to(deny_suggestion))
            // Task rewards endpoints
            .route("/{task_id}/rewards", web::get().to(get_task_rewards))
            .route("/{task_id}/rewards/{reward_id}", web::post().to(add_task_reward))
            .route("/{task_id}/rewards/{reward_id}", web::delete().to(remove_task_reward))
            // Task punishments endpoints
            .route("/{task_id}/punishments", web::get().to(get_task_punishments))
            .route("/{task_id}/punishments/{punishment_id}", web::post().to(add_task_punishment))
            .route("/{task_id}/punishments/{punishment_id}", web::delete().to(remove_task_punishment))
    );
}

async fn list_tasks(
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

    match task_service::list_tasks(&state.db, &household_id).await {
        Ok(tasks) => Ok(HttpResponse::Ok().json(ApiSuccess::new(tasks))),
        Err(e) => {
            log::error!("Error listing tasks: {:?}", e);
            Ok(HttpResponse::InternalServerError().json(ApiError {
                error: "internal_error".to_string(),
                message: "Failed to list tasks".to_string(),
            }))
        }
    }
}

async fn create_task(
    state: web::Data<AppState>,
    req: actix_web::HttpRequest,
    path: web::Path<String>,
    body: web::Json<CreateTaskRequest>,
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

    // Check if user can manage tasks based on hierarchy type
    let role = household_service::get_member_role(&state.db, &household_id, &user_id).await;
    let can_manage = role.as_ref().map(|r| solo_mode::can_manage_in_context(r, &settings)).unwrap_or(false);

    let mut request = body.into_inner();
    let is_suggestion = request.is_suggestion.unwrap_or(false);

    // Solo Mode special handling
    let solo_mode_active = settings.solo_mode;
    if solo_mode_active {
        // In Solo Mode, all tasks are treated as suggestions and auto-approved
        // Override points with household defaults
        request.points_reward = settings.default_points_reward;
        request.points_penalty = settings.default_points_penalty;
    }

    // If user can't manage and this is not a suggestion, deny access
    // (Solo Mode makes can_manage false for everyone, so all tasks become suggestions)
    if !can_manage && !is_suggestion && !solo_mode_active {
        return Ok(HttpResponse::Forbidden().json(ApiError {
            error: "forbidden".to_string(),
            message: "You do not have permission to create tasks".to_string(),
        }));
    }

    // If user is trying to suggest but suggestions are disabled, deny access
    // (Solo Mode bypasses this check since suggestions are auto-approved)
    if !can_manage && is_suggestion && !settings.allow_task_suggestions && !solo_mode_active {
        return Ok(HttpResponse::Forbidden().json(ApiError {
            error: "forbidden".to_string(),
            message: "Task suggestions are disabled for this household".to_string(),
        }));
    }

    // If user can manage but is_suggestion is set, ignore it (create normal task)
    // Suggestions are only for users without manage permission
    if request.title.is_empty() {
        return Ok(HttpResponse::BadRequest().json(ApiError {
            error: "validation_error".to_string(),
            message: "Task title is required".to_string(),
        }));
    }

    // Validate assignment in Hierarchy mode
    if let Some(ref assigned_id) = request.assigned_user_id {
        if settings.hierarchy_type == HierarchyType::Hierarchy {
            let assigned_role = household_service::get_member_role(&state.db, &household_id, assigned_id).await;
            if !assigned_role.as_ref().map(|r| settings.hierarchy_type.can_be_assigned(r)).unwrap_or(false) {
                return Ok(HttpResponse::BadRequest().json(ApiError {
                    error: "validation_error".to_string(),
                    message: "In Hierarchy mode, only Members can be assigned tasks".to_string(),
                }));
            }
        }
    }

    // Determine if this should be created as a suggestion
    // In Solo Mode, all tasks are created as suggestions by the user
    let suggested_by = if solo_mode_active || (is_suggestion && !can_manage) {
        Some(&user_id)
    } else {
        None
    };

    match task_service::create_task(&state.db, &household_id, &request, suggested_by).await {
        Ok(mut task) => {
            // Solo Mode: Auto-approve the suggestion and apply household defaults
            if solo_mode_active {
                // Auto-approve the task
                if let Ok(approved_task) = task_service::approve_suggestion(&state.db, &task.id).await {
                    task = approved_task;
                }

                // Apply household default rewards
                for default_reward in &settings.default_rewards {
                    let _ = task_consequences::add_task_reward(
                        &state.db,
                        &task.id,
                        &default_reward.reward.id,
                        default_reward.amount,
                    )
                    .await;
                }

                // Apply household default punishments
                for default_punishment in &settings.default_punishments {
                    let _ = task_consequences::add_task_punishment(
                        &state.db,
                        &task.id,
                        &default_punishment.punishment.id,
                        default_punishment.amount,
                    )
                    .await;
                }
            }

            // Log activity
            let details = serde_json::json!({ "title": task.title }).to_string();
            let _ = activity_logs::log_activity(
                &state.db,
                &household_id,
                &user_id,
                request.assigned_user_id.as_ref(),
                ActivityType::TaskCreated,
                Some("task"),
                Some(&task.id),
                Some(&details),
            ).await;

            // If task was assigned, also log assignment
            if let Some(ref assigned_id) = request.assigned_user_id {
                let _ = activity_logs::log_activity(
                    &state.db,
                    &household_id,
                    &user_id,
                    Some(assigned_id),
                    ActivityType::TaskAssigned,
                    Some("task"),
                    Some(&task.id),
                    Some(&details),
                ).await;
            }

            Ok(HttpResponse::Created().json(ApiSuccess::new(task)))
        }
        Err(e) => {
            log::error!("Error creating task: {:?}", e);
            Ok(HttpResponse::InternalServerError().json(ApiError {
                error: "internal_error".to_string(),
                message: "Failed to create task".to_string(),
            }))
        }
    }
}

async fn get_task(
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

    let (household_id_str, task_id_str) = path.into_inner();

    let household_id = match Uuid::parse_str(&household_id_str) {
        Ok(id) => id,
        Err(_) => {
            return Ok(HttpResponse::BadRequest().json(ApiError {
                error: "invalid_id".to_string(),
                message: "Invalid household ID format".to_string(),
            }));
        }
    };

    let task_id = match Uuid::parse_str(&task_id_str) {
        Ok(id) => id,
        Err(_) => {
            return Ok(HttpResponse::BadRequest().json(ApiError {
                error: "invalid_id".to_string(),
                message: "Invalid task ID format".to_string(),
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

    match task_service::get_task_with_status(&state.db, &task_id, &user_id).await {
        Ok(Some(task)) => Ok(HttpResponse::Ok().json(ApiSuccess::new(task))),
        Ok(None) => Ok(HttpResponse::NotFound().json(ApiError {
            error: "not_found".to_string(),
            message: "Task not found".to_string(),
        })),
        Err(e) => {
            log::error!("Error fetching task: {:?}", e);
            Ok(HttpResponse::InternalServerError().json(ApiError {
                error: "internal_error".to_string(),
                message: "Failed to fetch task".to_string(),
            }))
        }
    }
}

/// Get full task details including statistics for the detail view
async fn get_task_details(
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

    let (household_id_str, task_id_str) = path.into_inner();

    let household_id = match Uuid::parse_str(&household_id_str) {
        Ok(id) => id,
        Err(_) => {
            return Ok(HttpResponse::BadRequest().json(ApiError {
                error: "invalid_id".to_string(),
                message: "Invalid household ID format".to_string(),
            }));
        }
    };

    let task_id = match Uuid::parse_str(&task_id_str) {
        Ok(id) => id,
        Err(_) => {
            return Ok(HttpResponse::BadRequest().json(ApiError {
                error: "invalid_id".to_string(),
                message: "Invalid task ID format".to_string(),
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

    match task_service::get_task_with_details(&state.db, &task_id, &user_id).await {
        Ok(Some(details)) => {
            // Verify task belongs to this household
            if details.task.household_id != household_id {
                return Ok(HttpResponse::NotFound().json(ApiError {
                    error: "not_found".to_string(),
                    message: "Task not found in this household".to_string(),
                }));
            }
            Ok(HttpResponse::Ok().json(ApiSuccess::new(details)))
        }
        Ok(None) => Ok(HttpResponse::NotFound().json(ApiError {
            error: "not_found".to_string(),
            message: "Task not found".to_string(),
        })),
        Err(e) => {
            log::error!("Error fetching task details: {:?}", e);
            Ok(HttpResponse::InternalServerError().json(ApiError {
                error: "internal_error".to_string(),
                message: "Failed to fetch task details".to_string(),
            }))
        }
    }
}

async fn update_task(
    state: web::Data<AppState>,
    req: actix_web::HttpRequest,
    path: web::Path<(String, String)>,
    body: web::Json<UpdateTaskRequest>,
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

    let (household_id_str, task_id_str) = path.into_inner();

    let household_id = match Uuid::parse_str(&household_id_str) {
        Ok(id) => id,
        Err(_) => {
            return Ok(HttpResponse::BadRequest().json(ApiError {
                error: "invalid_id".to_string(),
                message: "Invalid household ID format".to_string(),
            }));
        }
    };

    let task_id = match Uuid::parse_str(&task_id_str) {
        Ok(id) => id,
        Err(_) => {
            return Ok(HttpResponse::BadRequest().json(ApiError {
                error: "invalid_id".to_string(),
                message: "Invalid task ID format".to_string(),
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

    // Parse request and get existing task BEFORE permission check
    // (needed to check if this is a valid "Set Date" request in Solo Mode)
    let request = body.into_inner();
    let old_task = task_service::get_task(&state.db, &task_id).await.ok().flatten();

    // Check if user can manage tasks based on hierarchy type
    let role = household_service::get_member_role(&state.db, &household_id, &user_id).await;
    let can_manage = role
        .as_ref()
        .map(|r| solo_mode::can_manage_in_context(r, &settings))
        .unwrap_or(false);

    // In Solo Mode, allow only "Set Date" operation for unscheduled tasks
    if settings.solo_mode && !can_manage {
        if let Some(ref task) = old_task {
            if !is_solo_mode_set_date_request(&request, task) {
                return Ok(HttpResponse::Forbidden().json(ApiError {
                    error: "forbidden".to_string(),
                    message: "In Solo Mode, you can only set a date for unscheduled tasks".to_string(),
                }));
            }
            // Valid "Set Date" request - allow it to proceed
        } else {
            return Ok(HttpResponse::NotFound().json(ApiError {
                error: "not_found".to_string(),
                message: "Task not found".to_string(),
            }));
        }
    } else if !can_manage {
        return Ok(HttpResponse::Forbidden().json(ApiError {
            error: "forbidden".to_string(),
            message: "You do not have permission to update tasks".to_string(),
        }));
    }

    // Validate assignment in Hierarchy mode - only if it's actually changing
    if let Some(ref assigned_id) = request.assigned_user_id {
        if settings.hierarchy_type == HierarchyType::Hierarchy {
            // Check if assignment is actually changing
            let old_assigned = old_task.as_ref().and_then(|t| t.assigned_user_id);
            let is_changing = old_assigned.map(|old| &old != assigned_id).unwrap_or(true);

            if is_changing {
                let assigned_role = household_service::get_member_role(&state.db, &household_id, assigned_id).await;
                if !assigned_role.as_ref().map(|r| settings.hierarchy_type.can_be_assigned(r)).unwrap_or(false) {
                    return Ok(HttpResponse::BadRequest().json(ApiError {
                        error: "validation_error".to_string(),
                        message: "In Hierarchy mode, only Members can be assigned tasks".to_string(),
                    }));
                }
            }
        }
    }
    let old_assigned = old_task.as_ref().and_then(|t| t.assigned_user_id);

    match task_service::update_task(&state.db, &task_id, &request).await {
        Ok(task) => {
            // Log activity
            let details = serde_json::json!({ "title": task.title }).to_string();
            let _ = activity_logs::log_activity(
                &state.db,
                &household_id,
                &user_id,
                task.assigned_user_id.as_ref(),
                ActivityType::TaskUpdated,
                Some("task"),
                Some(&task.id),
                Some(&details),
            ).await;

            // If assignment changed, log the assignment
            if request.assigned_user_id != old_assigned {
                if let Some(ref assigned_id) = request.assigned_user_id {
                    let _ = activity_logs::log_activity(
                        &state.db,
                        &household_id,
                        &user_id,
                        Some(assigned_id),
                        ActivityType::TaskAssigned,
                        Some("task"),
                        Some(&task.id),
                        Some(&details),
                    ).await;
                }
            }

            Ok(HttpResponse::Ok().json(ApiSuccess::new(task)))
        }
        Err(e) => {
            log::error!("Error updating task: {:?}", e);
            Ok(HttpResponse::InternalServerError().json(ApiError {
                error: "internal_error".to_string(),
                message: "Failed to update task".to_string(),
            }))
        }
    }
}

async fn delete_task(
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

    let (household_id_str, task_id_str) = path.into_inner();

    let household_id = match Uuid::parse_str(&household_id_str) {
        Ok(id) => id,
        Err(_) => {
            return Ok(HttpResponse::BadRequest().json(ApiError {
                error: "invalid_id".to_string(),
                message: "Invalid household ID format".to_string(),
            }));
        }
    };

    let task_id = match Uuid::parse_str(&task_id_str) {
        Ok(id) => id,
        Err(_) => {
            return Ok(HttpResponse::BadRequest().json(ApiError {
                error: "invalid_id".to_string(),
                message: "Invalid task ID format".to_string(),
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

    // Check if user can manage tasks based on hierarchy type
    let role = household_service::get_member_role(&state.db, &household_id, &user_id).await;
    if !role.as_ref().map(|r| solo_mode::can_manage_in_context(r, &settings)).unwrap_or(false) {
        return Ok(HttpResponse::Forbidden().json(ApiError {
            error: "forbidden".to_string(),
            message: "You do not have permission to delete tasks".to_string(),
        }));
    }

    // Get the task details before deletion for logging
    let task = task_service::get_task(&state.db, &task_id).await.ok().flatten();
    let details = task.as_ref()
        .map(|t| serde_json::json!({ "title": t.title }).to_string());

    match task_service::delete_task(&state.db, &task_id).await {
        Ok(_) => {
            // Log activity
            let _ = activity_logs::log_activity(
                &state.db,
                &household_id,
                &user_id,
                None,
                ActivityType::TaskDeleted,
                Some("task"),
                Some(&task_id),
                details.as_deref(),
            ).await;

            Ok(HttpResponse::NoContent().finish())
        }
        Err(e) => {
            log::error!("Error deleting task: {:?}", e);
            Ok(HttpResponse::InternalServerError().json(ApiError {
                error: "internal_error".to_string(),
                message: "Failed to delete task".to_string(),
            }))
        }
    }
}

async fn list_archived_tasks(
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

    match task_service::list_archived_tasks(&state.db, &household_id).await {
        Ok(tasks) => Ok(HttpResponse::Ok().json(ApiSuccess::new(tasks))),
        Err(e) => {
            log::error!("Error listing archived tasks: {:?}", e);
            Ok(HttpResponse::InternalServerError().json(ApiError {
                error: "internal_error".to_string(),
                message: "Failed to list archived tasks".to_string(),
            }))
        }
    }
}

async fn archive_task(
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

    let (household_id_str, task_id_str) = path.into_inner();

    let household_id = match Uuid::parse_str(&household_id_str) {
        Ok(id) => id,
        Err(_) => {
            return Ok(HttpResponse::BadRequest().json(ApiError {
                error: "invalid_id".to_string(),
                message: "Invalid household ID format".to_string(),
            }));
        }
    };

    let task_id = match Uuid::parse_str(&task_id_str) {
        Ok(id) => id,
        Err(_) => {
            return Ok(HttpResponse::BadRequest().json(ApiError {
                error: "invalid_id".to_string(),
                message: "Invalid task ID format".to_string(),
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

    // Check if user can manage tasks based on hierarchy type
    let role = household_service::get_member_role(&state.db, &household_id, &user_id).await;
    if !role.as_ref().map(|r| solo_mode::can_manage_in_context(r, &settings)).unwrap_or(false) {
        return Ok(HttpResponse::Forbidden().json(ApiError {
            error: "forbidden".to_string(),
            message: "You do not have permission to archive tasks".to_string(),
        }));
    }

    match task_service::archive_task(&state.db, &task_id).await {
        Ok(task) => {
            // Log activity
            let details = serde_json::json!({ "title": task.title }).to_string();
            let _ = activity_logs::log_activity(
                &state.db,
                &household_id,
                &user_id,
                None,
                ActivityType::TaskUpdated,
                Some("task"),
                Some(&task.id),
                Some(&details),
            ).await;

            Ok(HttpResponse::Ok().json(ApiSuccess::new(task)))
        }
        Err(task_service::TaskError::NotFound) => {
            Ok(HttpResponse::NotFound().json(ApiError {
                error: "not_found".to_string(),
                message: "Task not found".to_string(),
            }))
        }
        Err(e) => {
            log::error!("Error archiving task: {:?}", e);
            Ok(HttpResponse::InternalServerError().json(ApiError {
                error: "internal_error".to_string(),
                message: "Failed to archive task".to_string(),
            }))
        }
    }
}

async fn unarchive_task(
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

    let (household_id_str, task_id_str) = path.into_inner();

    let household_id = match Uuid::parse_str(&household_id_str) {
        Ok(id) => id,
        Err(_) => {
            return Ok(HttpResponse::BadRequest().json(ApiError {
                error: "invalid_id".to_string(),
                message: "Invalid household ID format".to_string(),
            }));
        }
    };

    let task_id = match Uuid::parse_str(&task_id_str) {
        Ok(id) => id,
        Err(_) => {
            return Ok(HttpResponse::BadRequest().json(ApiError {
                error: "invalid_id".to_string(),
                message: "Invalid task ID format".to_string(),
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

    // Check if user can manage tasks based on hierarchy type
    let role = household_service::get_member_role(&state.db, &household_id, &user_id).await;
    if !role.as_ref().map(|r| solo_mode::can_manage_in_context(r, &settings)).unwrap_or(false) {
        return Ok(HttpResponse::Forbidden().json(ApiError {
            error: "forbidden".to_string(),
            message: "You do not have permission to unarchive tasks".to_string(),
        }));
    }

    match task_service::unarchive_task(&state.db, &task_id).await {
        Ok(task) => {
            // Log activity
            let details = serde_json::json!({ "title": task.title }).to_string();
            let _ = activity_logs::log_activity(
                &state.db,
                &household_id,
                &user_id,
                None,
                ActivityType::TaskUpdated,
                Some("task"),
                Some(&task.id),
                Some(&details),
            ).await;

            Ok(HttpResponse::Ok().json(ApiSuccess::new(task)))
        }
        Err(task_service::TaskError::NotFound) => {
            Ok(HttpResponse::NotFound().json(ApiError {
                error: "not_found".to_string(),
                message: "Task not found".to_string(),
            }))
        }
        Err(e) => {
            log::error!("Error unarchiving task: {:?}", e);
            Ok(HttpResponse::InternalServerError().json(ApiError {
                error: "internal_error".to_string(),
                message: "Failed to unarchive task".to_string(),
            }))
        }
    }
}

async fn pause_task(
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

    let (household_id_str, task_id_str) = path.into_inner();

    let household_id = match Uuid::parse_str(&household_id_str) {
        Ok(id) => id,
        Err(_) => {
            return Ok(HttpResponse::BadRequest().json(ApiError {
                error: "invalid_id".to_string(),
                message: "Invalid household ID format".to_string(),
            }));
        }
    };

    let task_id = match Uuid::parse_str(&task_id_str) {
        Ok(id) => id,
        Err(_) => {
            return Ok(HttpResponse::BadRequest().json(ApiError {
                error: "invalid_id".to_string(),
                message: "Invalid task ID format".to_string(),
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

    // Check if user can manage tasks based on hierarchy type
    let role = household_service::get_member_role(&state.db, &household_id, &user_id).await;
    if !role.as_ref().map(|r| solo_mode::can_manage_in_context(r, &settings)).unwrap_or(false) {
        return Ok(HttpResponse::Forbidden().json(ApiError {
            error: "forbidden".to_string(),
            message: "You do not have permission to pause tasks".to_string(),
        }));
    }

    match task_service::pause_task(&state.db, &task_id).await {
        Ok(task) => {
            // Log activity
            let details = serde_json::json!({ "title": task.title, "paused": true }).to_string();
            let _ = activity_logs::log_activity(
                &state.db,
                &household_id,
                &user_id,
                None,
                ActivityType::TaskUpdated,
                Some("task"),
                Some(&task.id),
                Some(&details),
            ).await;

            Ok(HttpResponse::Ok().json(ApiSuccess::new(task)))
        }
        Err(task_service::TaskError::NotFound) => {
            Ok(HttpResponse::NotFound().json(ApiError {
                error: "not_found".to_string(),
                message: "Task not found".to_string(),
            }))
        }
        Err(e) => {
            log::error!("Error pausing task: {:?}", e);
            Ok(HttpResponse::InternalServerError().json(ApiError {
                error: "internal_error".to_string(),
                message: "Failed to pause task".to_string(),
            }))
        }
    }
}

async fn unpause_task(
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

    let (household_id_str, task_id_str) = path.into_inner();

    let household_id = match Uuid::parse_str(&household_id_str) {
        Ok(id) => id,
        Err(_) => {
            return Ok(HttpResponse::BadRequest().json(ApiError {
                error: "invalid_id".to_string(),
                message: "Invalid household ID format".to_string(),
            }));
        }
    };

    let task_id = match Uuid::parse_str(&task_id_str) {
        Ok(id) => id,
        Err(_) => {
            return Ok(HttpResponse::BadRequest().json(ApiError {
                error: "invalid_id".to_string(),
                message: "Invalid task ID format".to_string(),
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

    // Check if user can manage tasks based on hierarchy type
    let role = household_service::get_member_role(&state.db, &household_id, &user_id).await;
    if !role.as_ref().map(|r| solo_mode::can_manage_in_context(r, &settings)).unwrap_or(false) {
        return Ok(HttpResponse::Forbidden().json(ApiError {
            error: "forbidden".to_string(),
            message: "You do not have permission to unpause tasks".to_string(),
        }));
    }

    match task_service::unpause_task(&state.db, &task_id).await {
        Ok(task) => {
            // Log activity
            let details = serde_json::json!({ "title": task.title, "paused": false }).to_string();
            let _ = activity_logs::log_activity(
                &state.db,
                &household_id,
                &user_id,
                None,
                ActivityType::TaskUpdated,
                Some("task"),
                Some(&task.id),
                Some(&details),
            ).await;

            Ok(HttpResponse::Ok().json(ApiSuccess::new(task)))
        }
        Err(task_service::TaskError::NotFound) => {
            Ok(HttpResponse::NotFound().json(ApiError {
                error: "not_found".to_string(),
                message: "Task not found".to_string(),
            }))
        }
        Err(e) => {
            log::error!("Error unpausing task: {:?}", e);
            Ok(HttpResponse::InternalServerError().json(ApiError {
                error: "internal_error".to_string(),
                message: "Failed to unpause task".to_string(),
            }))
        }
    }
}

async fn complete_task(
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

    let (household_id_str, task_id_str) = path.into_inner();

    let household_id = match Uuid::parse_str(&household_id_str) {
        Ok(id) => id,
        Err(_) => {
            return Ok(HttpResponse::BadRequest().json(ApiError {
                error: "invalid_id".to_string(),
                message: "Invalid household ID format".to_string(),
            }));
        }
    };

    let task_id = match Uuid::parse_str(&task_id_str) {
        Ok(id) => id,
        Err(_) => {
            return Ok(HttpResponse::BadRequest().json(ApiError {
                error: "invalid_id".to_string(),
                message: "Invalid task ID format".to_string(),
            }));
        }
    };

    // Check membership (any member can complete tasks)
    if !household_service::is_member(&state.db, &household_id, &user_id).await.unwrap_or(false) {
        return Ok(HttpResponse::Forbidden().json(ApiError {
            error: "forbidden".to_string(),
            message: "You are not a member of this household".to_string(),
        }));
    }

    // Get settings to check for Solo Mode
    let settings = household_settings::get_or_create_settings(&state.db, &household_id)
        .await
        .unwrap_or_default();

    // Get the task details for logging
    let task = task_service::get_task(&state.db, &task_id).await.ok().flatten();
    let details = task.as_ref()
        .map(|t| serde_json::json!({ "title": t.title }).to_string());

    match task_service::complete_task(&state.db, &task_id, &user_id, &household_id).await {
        Ok(mut completion) => {
            // In Solo Mode, auto-approve completions that would otherwise be pending
            // (since nobody can approve them in Solo Mode)
            if settings.solo_mode && completion.status == shared::CompletionStatus::Pending {
                if let Ok(approved) = task_service::approve_completion(&state.db, &completion.id).await {
                    completion = approved;
                }
            }

            // Log activity
            let _ = activity_logs::log_activity(
                &state.db,
                &household_id,
                &user_id,
                Some(&user_id),
                ActivityType::TaskCompleted,
                Some("task"),
                Some(&task_id),
                details.as_deref(),
            ).await;

            Ok(HttpResponse::Created().json(ApiSuccess::new(completion)))
        }
        Err(e) => {
            log::error!("Error completing task: {:?}", e);
            Ok(HttpResponse::BadRequest().json(ApiError {
                error: "completion_error".to_string(),
                message: e.to_string(),
            }))
        }
    }
}

async fn uncomplete_task(
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

    let (household_id_str, task_id_str) = path.into_inner();

    let household_id = match Uuid::parse_str(&household_id_str) {
        Ok(id) => id,
        Err(_) => {
            return Ok(HttpResponse::BadRequest().json(ApiError {
                error: "invalid_id".to_string(),
                message: "Invalid household ID format".to_string(),
            }));
        }
    };

    let task_id = match Uuid::parse_str(&task_id_str) {
        Ok(id) => id,
        Err(_) => {
            return Ok(HttpResponse::BadRequest().json(ApiError {
                error: "invalid_id".to_string(),
                message: "Invalid task ID format".to_string(),
            }));
        }
    };

    // Check membership (any member can uncomplete their own tasks)
    if !household_service::is_member(&state.db, &household_id, &user_id).await.unwrap_or(false) {
        return Ok(HttpResponse::Forbidden().json(ApiError {
            error: "forbidden".to_string(),
            message: "You are not a member of this household".to_string(),
        }));
    }

    match task_service::uncomplete_task(&state.db, &task_id, &user_id).await {
        Ok(_) => Ok(HttpResponse::Ok().json(ApiSuccess::new(()))),
        Err(e) => {
            log::error!("Error uncompleting task: {:?}", e);
            Ok(HttpResponse::BadRequest().json(ApiError {
                error: "uncomplete_error".to_string(),
                message: e.to_string(),
            }))
        }
    }
}

async fn get_due_tasks(
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

    match task_service::get_due_tasks(&state.db, &household_id, &user_id).await {
        Ok(tasks) => Ok(HttpResponse::Ok().json(ApiSuccess::new(tasks))),
        Err(e) => {
            log::error!("Error fetching due tasks: {:?}", e);
            Ok(HttpResponse::InternalServerError().json(ApiError {
                error: "internal_error".to_string(),
                message: "Failed to fetch due tasks".to_string(),
            }))
        }
    }
}

async fn get_all_tasks_with_status(
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

    match task_service::get_all_tasks_with_status(&state.db, &household_id, &user_id).await {
        Ok(tasks) => Ok(HttpResponse::Ok().json(ApiSuccess::new(tasks))),
        Err(e) => {
            log::error!("Error fetching all tasks with status: {:?}", e);
            Ok(HttpResponse::InternalServerError().json(ApiError {
                error: "internal_error".to_string(),
                message: "Failed to fetch tasks".to_string(),
            }))
        }
    }
}

async fn get_assigned_tasks(
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

    match task_service::list_user_assigned_tasks(&state.db, &household_id, &user_id).await {
        Ok(tasks) => Ok(HttpResponse::Ok().json(ApiSuccess::new(tasks))),
        Err(e) => {
            log::error!("Error fetching assigned tasks: {:?}", e);
            Ok(HttpResponse::InternalServerError().json(ApiError {
                error: "internal_error".to_string(),
                message: "Failed to fetch assigned tasks".to_string(),
            }))
        }
    }
}

// ============================================================================
// Task Rewards/Punishments Endpoints
// ============================================================================

async fn get_task_rewards(
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

    let (household_id_str, task_id_str) = path.into_inner();

    let household_id = match Uuid::parse_str(&household_id_str) {
        Ok(id) => id,
        Err(_) => {
            return Ok(HttpResponse::BadRequest().json(ApiError {
                error: "invalid_id".to_string(),
                message: "Invalid household ID format".to_string(),
            }));
        }
    };

    let task_id = match Uuid::parse_str(&task_id_str) {
        Ok(id) => id,
        Err(_) => {
            return Ok(HttpResponse::BadRequest().json(ApiError {
                error: "invalid_id".to_string(),
                message: "Invalid task ID format".to_string(),
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

    match task_consequences::get_task_rewards(&state.db, &task_id).await {
        Ok(rewards) => Ok(HttpResponse::Ok().json(ApiSuccess::new(rewards))),
        Err(e) => {
            log::error!("Error fetching task rewards: {:?}", e);
            Ok(HttpResponse::InternalServerError().json(ApiError {
                error: "internal_error".to_string(),
                message: "Failed to fetch task rewards".to_string(),
            }))
        }
    }
}

async fn add_task_reward(
    state: web::Data<AppState>,
    req: actix_web::HttpRequest,
    path: web::Path<(String, String, String)>,
    query: web::Query<AddLinkQuery>,
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

    let (household_id_str, task_id_str, reward_id_str) = path.into_inner();
    let amount = query.amount;

    let household_id = match Uuid::parse_str(&household_id_str) {
        Ok(id) => id,
        Err(_) => {
            return Ok(HttpResponse::BadRequest().json(ApiError {
                error: "invalid_id".to_string(),
                message: "Invalid household ID format".to_string(),
            }));
        }
    };

    let task_id = match Uuid::parse_str(&task_id_str) {
        Ok(id) => id,
        Err(_) => {
            return Ok(HttpResponse::BadRequest().json(ApiError {
                error: "invalid_id".to_string(),
                message: "Invalid task ID format".to_string(),
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

    // Check if user can manage tasks based on hierarchy type
    let role = household_service::get_member_role(&state.db, &household_id, &user_id).await;
    if !role.as_ref().map(|r| solo_mode::can_manage_in_context(r, &settings)).unwrap_or(false) {
        return Ok(HttpResponse::Forbidden().json(ApiError {
            error: "forbidden".to_string(),
            message: "You do not have permission to link rewards to tasks".to_string(),
        }));
    }

    match task_consequences::add_task_reward(&state.db, &task_id, &reward_id, amount).await {
        Ok(_) => Ok(HttpResponse::Created().json(ApiSuccess::new(()))),
        Err(task_consequences::TaskConsequenceError::AlreadyExists) => {
            Ok(HttpResponse::Conflict().json(ApiError {
                error: "already_exists".to_string(),
                message: "Reward is already linked to this task".to_string(),
            }))
        }
        Err(e) => {
            log::error!("Error adding task reward: {:?}", e);
            Ok(HttpResponse::InternalServerError().json(ApiError {
                error: "internal_error".to_string(),
                message: "Failed to link reward to task".to_string(),
            }))
        }
    }
}

async fn remove_task_reward(
    state: web::Data<AppState>,
    req: actix_web::HttpRequest,
    path: web::Path<(String, String, String)>,
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

    let (household_id_str, task_id_str, reward_id_str) = path.into_inner();

    let household_id = match Uuid::parse_str(&household_id_str) {
        Ok(id) => id,
        Err(_) => {
            return Ok(HttpResponse::BadRequest().json(ApiError {
                error: "invalid_id".to_string(),
                message: "Invalid household ID format".to_string(),
            }));
        }
    };

    let task_id = match Uuid::parse_str(&task_id_str) {
        Ok(id) => id,
        Err(_) => {
            return Ok(HttpResponse::BadRequest().json(ApiError {
                error: "invalid_id".to_string(),
                message: "Invalid task ID format".to_string(),
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

    // Check if user can manage tasks based on hierarchy type
    let role = household_service::get_member_role(&state.db, &household_id, &user_id).await;
    if !role.as_ref().map(|r| solo_mode::can_manage_in_context(r, &settings)).unwrap_or(false) {
        return Ok(HttpResponse::Forbidden().json(ApiError {
            error: "forbidden".to_string(),
            message: "You do not have permission to unlink rewards from tasks".to_string(),
        }));
    }

    match task_consequences::remove_task_reward(&state.db, &task_id, &reward_id).await {
        Ok(_) => Ok(HttpResponse::NoContent().finish()),
        Err(task_consequences::TaskConsequenceError::AssociationNotFound) => {
            Ok(HttpResponse::NotFound().json(ApiError {
                error: "not_found".to_string(),
                message: "Reward is not linked to this task".to_string(),
            }))
        }
        Err(e) => {
            log::error!("Error removing task reward: {:?}", e);
            Ok(HttpResponse::InternalServerError().json(ApiError {
                error: "internal_error".to_string(),
                message: "Failed to unlink reward from task".to_string(),
            }))
        }
    }
}

async fn get_task_punishments(
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

    let (household_id_str, task_id_str) = path.into_inner();

    let household_id = match Uuid::parse_str(&household_id_str) {
        Ok(id) => id,
        Err(_) => {
            return Ok(HttpResponse::BadRequest().json(ApiError {
                error: "invalid_id".to_string(),
                message: "Invalid household ID format".to_string(),
            }));
        }
    };

    let task_id = match Uuid::parse_str(&task_id_str) {
        Ok(id) => id,
        Err(_) => {
            return Ok(HttpResponse::BadRequest().json(ApiError {
                error: "invalid_id".to_string(),
                message: "Invalid task ID format".to_string(),
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

    match task_consequences::get_task_punishments(&state.db, &task_id).await {
        Ok(punishments) => Ok(HttpResponse::Ok().json(ApiSuccess::new(punishments))),
        Err(e) => {
            log::error!("Error fetching task punishments: {:?}", e);
            Ok(HttpResponse::InternalServerError().json(ApiError {
                error: "internal_error".to_string(),
                message: "Failed to fetch task punishments".to_string(),
            }))
        }
    }
}

async fn add_task_punishment(
    state: web::Data<AppState>,
    req: actix_web::HttpRequest,
    path: web::Path<(String, String, String)>,
    query: web::Query<AddLinkQuery>,
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

    let (household_id_str, task_id_str, punishment_id_str) = path.into_inner();
    let amount = query.amount;

    let household_id = match Uuid::parse_str(&household_id_str) {
        Ok(id) => id,
        Err(_) => {
            return Ok(HttpResponse::BadRequest().json(ApiError {
                error: "invalid_id".to_string(),
                message: "Invalid household ID format".to_string(),
            }));
        }
    };

    let task_id = match Uuid::parse_str(&task_id_str) {
        Ok(id) => id,
        Err(_) => {
            return Ok(HttpResponse::BadRequest().json(ApiError {
                error: "invalid_id".to_string(),
                message: "Invalid task ID format".to_string(),
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

    // Check if user can manage tasks based on hierarchy type
    let role = household_service::get_member_role(&state.db, &household_id, &user_id).await;
    if !role.as_ref().map(|r| solo_mode::can_manage_in_context(r, &settings)).unwrap_or(false) {
        return Ok(HttpResponse::Forbidden().json(ApiError {
            error: "forbidden".to_string(),
            message: "You do not have permission to link punishments to tasks".to_string(),
        }));
    }

    match task_consequences::add_task_punishment(&state.db, &task_id, &punishment_id, amount).await {
        Ok(_) => Ok(HttpResponse::Created().json(ApiSuccess::new(()))),
        Err(task_consequences::TaskConsequenceError::AlreadyExists) => {
            Ok(HttpResponse::Conflict().json(ApiError {
                error: "already_exists".to_string(),
                message: "Punishment is already linked to this task".to_string(),
            }))
        }
        Err(e) => {
            log::error!("Error adding task punishment: {:?}", e);
            Ok(HttpResponse::InternalServerError().json(ApiError {
                error: "internal_error".to_string(),
                message: "Failed to link punishment to task".to_string(),
            }))
        }
    }
}

async fn remove_task_punishment(
    state: web::Data<AppState>,
    req: actix_web::HttpRequest,
    path: web::Path<(String, String, String)>,
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

    let (household_id_str, task_id_str, punishment_id_str) = path.into_inner();

    let household_id = match Uuid::parse_str(&household_id_str) {
        Ok(id) => id,
        Err(_) => {
            return Ok(HttpResponse::BadRequest().json(ApiError {
                error: "invalid_id".to_string(),
                message: "Invalid household ID format".to_string(),
            }));
        }
    };

    let task_id = match Uuid::parse_str(&task_id_str) {
        Ok(id) => id,
        Err(_) => {
            return Ok(HttpResponse::BadRequest().json(ApiError {
                error: "invalid_id".to_string(),
                message: "Invalid task ID format".to_string(),
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

    // Check if user can manage tasks based on hierarchy type
    let role = household_service::get_member_role(&state.db, &household_id, &user_id).await;
    if !role.as_ref().map(|r| solo_mode::can_manage_in_context(r, &settings)).unwrap_or(false) {
        return Ok(HttpResponse::Forbidden().json(ApiError {
            error: "forbidden".to_string(),
            message: "You do not have permission to unlink punishments from tasks".to_string(),
        }));
    }

    match task_consequences::remove_task_punishment(&state.db, &task_id, &punishment_id).await {
        Ok(_) => Ok(HttpResponse::NoContent().finish()),
        Err(task_consequences::TaskConsequenceError::AssociationNotFound) => {
            Ok(HttpResponse::NotFound().json(ApiError {
                error: "not_found".to_string(),
                message: "Punishment is not linked to this task".to_string(),
            }))
        }
        Err(e) => {
            log::error!("Error removing task punishment: {:?}", e);
            Ok(HttpResponse::InternalServerError().json(ApiError {
                error: "internal_error".to_string(),
                message: "Failed to unlink punishment from task".to_string(),
            }))
        }
    }
}

// ============================================================================
// Review Endpoints
// ============================================================================

async fn get_pending_reviews(
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

    // Get settings for hierarchy-aware permissions (only owners/managers can see pending reviews)
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

    // Check if user can manage tasks based on hierarchy type
    let role = household_service::get_member_role(&state.db, &household_id, &user_id).await;
    if !role.as_ref().map(|r| solo_mode::can_manage_in_context(r, &settings)).unwrap_or(false) {
        return Ok(HttpResponse::Forbidden().json(ApiError {
            error: "forbidden".to_string(),
            message: "You do not have permission to view pending reviews".to_string(),
        }));
    }

    match task_service::list_pending_reviews(&state.db, &household_id).await {
        Ok(reviews) => Ok(HttpResponse::Ok().json(ApiSuccess::new(reviews))),
        Err(e) => {
            log::error!("Error fetching pending reviews: {:?}", e);
            Ok(HttpResponse::InternalServerError().json(ApiError {
                error: "internal_error".to_string(),
                message: "Failed to fetch pending reviews".to_string(),
            }))
        }
    }
}

async fn approve_completion(
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

    let (household_id_str, completion_id_str) = path.into_inner();

    let household_id = match Uuid::parse_str(&household_id_str) {
        Ok(id) => id,
        Err(_) => {
            return Ok(HttpResponse::BadRequest().json(ApiError {
                error: "invalid_id".to_string(),
                message: "Invalid household ID format".to_string(),
            }));
        }
    };

    let completion_id = match Uuid::parse_str(&completion_id_str) {
        Ok(id) => id,
        Err(_) => {
            return Ok(HttpResponse::BadRequest().json(ApiError {
                error: "invalid_id".to_string(),
                message: "Invalid completion ID format".to_string(),
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

    // Check if user can manage tasks based on hierarchy type
    let role = household_service::get_member_role(&state.db, &household_id, &user_id).await;
    if !role.as_ref().map(|r| solo_mode::can_manage_in_context(r, &settings)).unwrap_or(false) {
        return Ok(HttpResponse::Forbidden().json(ApiError {
            error: "forbidden".to_string(),
            message: "You do not have permission to approve task completions".to_string(),
        }));
    }

    // Get completion details for logging
    let completion = task_service::get_completion(&state.db, &completion_id).await.ok().flatten();
    let task = if let Some(ref c) = completion {
        task_service::get_task(&state.db, &c.task_id).await.ok().flatten()
    } else {
        None
    };
    let details = task.as_ref()
        .map(|t| serde_json::json!({ "title": t.title }).to_string());

    match task_service::approve_completion(&state.db, &completion_id).await {
        Ok(approved) => {
            // Log activity
            let _ = activity_logs::log_activity(
                &state.db,
                &household_id,
                &user_id,
                completion.as_ref().map(|c| &c.user_id),
                ActivityType::TaskCompletionApproved,
                Some("task_completion"),
                Some(&completion_id),
                details.as_deref(),
            ).await;

            Ok(HttpResponse::Ok().json(ApiSuccess::new(approved)))
        }
        Err(task_service::TaskError::NotFound) => {
            Ok(HttpResponse::NotFound().json(ApiError {
                error: "not_found".to_string(),
                message: "Completion not found or not pending".to_string(),
            }))
        }
        Err(e) => {
            log::error!("Error approving completion: {:?}", e);
            Ok(HttpResponse::InternalServerError().json(ApiError {
                error: "internal_error".to_string(),
                message: "Failed to approve completion".to_string(),
            }))
        }
    }
}

async fn reject_completion(
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

    let (household_id_str, completion_id_str) = path.into_inner();

    let household_id = match Uuid::parse_str(&household_id_str) {
        Ok(id) => id,
        Err(_) => {
            return Ok(HttpResponse::BadRequest().json(ApiError {
                error: "invalid_id".to_string(),
                message: "Invalid household ID format".to_string(),
            }));
        }
    };

    let completion_id = match Uuid::parse_str(&completion_id_str) {
        Ok(id) => id,
        Err(_) => {
            return Ok(HttpResponse::BadRequest().json(ApiError {
                error: "invalid_id".to_string(),
                message: "Invalid completion ID format".to_string(),
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

    // Check if user can manage tasks based on hierarchy type
    let role = household_service::get_member_role(&state.db, &household_id, &user_id).await;
    if !role.as_ref().map(|r| solo_mode::can_manage_in_context(r, &settings)).unwrap_or(false) {
        return Ok(HttpResponse::Forbidden().json(ApiError {
            error: "forbidden".to_string(),
            message: "You do not have permission to reject task completions".to_string(),
        }));
    }

    // Get completion details for logging before deletion
    let completion = task_service::get_completion(&state.db, &completion_id).await.ok().flatten();
    let task = if let Some(ref c) = completion {
        task_service::get_task(&state.db, &c.task_id).await.ok().flatten()
    } else {
        None
    };
    let details = task.as_ref()
        .map(|t| serde_json::json!({ "title": t.title }).to_string());
    let affected_user_id = completion.as_ref().map(|c| c.user_id);

    match task_service::reject_completion(&state.db, &completion_id, &household_id).await {
        Ok(_) => {
            // Log activity
            let _ = activity_logs::log_activity(
                &state.db,
                &household_id,
                &user_id,
                affected_user_id.as_ref(),
                ActivityType::TaskCompletionRejected,
                Some("task_completion"),
                Some(&completion_id),
                details.as_deref(),
            ).await;

            Ok(HttpResponse::Ok().json(ApiSuccess::new(())))
        }
        Err(task_service::TaskError::NotFound) => {
            Ok(HttpResponse::NotFound().json(ApiError {
                error: "not_found".to_string(),
                message: "Completion not found or not pending".to_string(),
            }))
        }
        Err(e) => {
            log::error!("Error rejecting completion: {:?}", e);
            Ok(HttpResponse::InternalServerError().json(ApiError {
                error: "internal_error".to_string(),
                message: "Failed to reject completion".to_string(),
            }))
        }
    }
}

// ============================================================================
// Task Suggestion Endpoints
// ============================================================================

async fn list_suggestions(
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

    // Check if user can manage tasks based on hierarchy type
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
    if !role.as_ref().map(|r| solo_mode::can_manage_in_context(r, &settings)).unwrap_or(false) {
        return Ok(HttpResponse::Forbidden().json(ApiError {
            error: "forbidden".to_string(),
            message: "You do not have permission to view suggestions".to_string(),
        }));
    }

    match task_service::list_suggested_tasks(&state.db, &household_id).await {
        Ok(tasks) => Ok(HttpResponse::Ok().json(ApiSuccess::new(tasks))),
        Err(e) => {
            log::error!("Error listing suggestions: {:?}", e);
            Ok(HttpResponse::InternalServerError().json(ApiError {
                error: "internal_error".to_string(),
                message: "Failed to list suggestions".to_string(),
            }))
        }
    }
}

async fn approve_suggestion(
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

    let (household_id_str, task_id_str) = path.into_inner();

    let household_id = match Uuid::parse_str(&household_id_str) {
        Ok(id) => id,
        Err(_) => {
            return Ok(HttpResponse::BadRequest().json(ApiError {
                error: "invalid_id".to_string(),
                message: "Invalid household ID format".to_string(),
            }));
        }
    };

    let task_id = match Uuid::parse_str(&task_id_str) {
        Ok(id) => id,
        Err(_) => {
            return Ok(HttpResponse::BadRequest().json(ApiError {
                error: "invalid_id".to_string(),
                message: "Invalid task ID format".to_string(),
            }));
        }
    };

    // Check if user can manage tasks based on hierarchy type
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
    if !role.as_ref().map(|r| solo_mode::can_manage_in_context(r, &settings)).unwrap_or(false) {
        return Ok(HttpResponse::Forbidden().json(ApiError {
            error: "forbidden".to_string(),
            message: "You do not have permission to approve suggestions".to_string(),
        }));
    }

    match task_service::approve_suggestion(&state.db, &task_id).await {
        Ok(task) => {
            // Log activity
            let details = serde_json::json!({ "title": task.title }).to_string();
            let _ = activity_logs::log_activity(
                &state.db,
                &household_id,
                &user_id,
                task.suggested_by.as_ref(),
                ActivityType::TaskCreated, // Task is now active
                Some("task"),
                Some(&task.id),
                Some(&details),
            ).await;

            Ok(HttpResponse::Ok().json(ApiSuccess::new(task)))
        }
        Err(task_service::TaskError::NotFound) => {
            Ok(HttpResponse::NotFound().json(ApiError {
                error: "not_found".to_string(),
                message: "Suggestion not found".to_string(),
            }))
        }
        Err(e) => {
            log::error!("Error approving suggestion: {:?}", e);
            Ok(HttpResponse::InternalServerError().json(ApiError {
                error: "internal_error".to_string(),
                message: "Failed to approve suggestion".to_string(),
            }))
        }
    }
}

async fn deny_suggestion(
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

    let (household_id_str, task_id_str) = path.into_inner();

    let household_id = match Uuid::parse_str(&household_id_str) {
        Ok(id) => id,
        Err(_) => {
            return Ok(HttpResponse::BadRequest().json(ApiError {
                error: "invalid_id".to_string(),
                message: "Invalid household ID format".to_string(),
            }));
        }
    };

    let task_id = match Uuid::parse_str(&task_id_str) {
        Ok(id) => id,
        Err(_) => {
            return Ok(HttpResponse::BadRequest().json(ApiError {
                error: "invalid_id".to_string(),
                message: "Invalid task ID format".to_string(),
            }));
        }
    };

    // Check if user can manage tasks based on hierarchy type
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
    if !role.as_ref().map(|r| solo_mode::can_manage_in_context(r, &settings)).unwrap_or(false) {
        return Ok(HttpResponse::Forbidden().json(ApiError {
            error: "forbidden".to_string(),
            message: "You do not have permission to deny suggestions".to_string(),
        }));
    }

    match task_service::deny_suggestion(&state.db, &task_id).await {
        Ok(task) => Ok(HttpResponse::Ok().json(ApiSuccess::new(task))),
        Err(task_service::TaskError::NotFound) => {
            Ok(HttpResponse::NotFound().json(ApiError {
                error: "not_found".to_string(),
                message: "Suggestion not found".to_string(),
            }))
        }
        Err(e) => {
            log::error!("Error denying suggestion: {:?}", e);
            Ok(HttpResponse::InternalServerError().json(ApiError {
                error: "internal_error".to_string(),
                message: "Failed to deny suggestion".to_string(),
            }))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use shared::{HabitType, TimePeriod};

    fn create_test_task(recurrence_type: RecurrenceType, recurrence_value: Option<RecurrenceValue>) -> Task {
        Task {
            id: Uuid::new_v4(),
            household_id: Uuid::new_v4(),
            title: "Test Task".to_string(),
            description: "".to_string(),
            recurrence_type,
            recurrence_value,
            assigned_user_id: None,
            target_count: 1,
            time_period: Some(TimePeriod::Day),
            allow_exceed_target: false,
            requires_review: false,
            points_reward: None,
            points_penalty: None,
            due_time: None,
            habit_type: HabitType::Good,
            category_id: None,
            category_name: None,
            suggestion: None,
            suggested_by: None,
            archived: false,
            paused: false,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }

    fn create_set_date_request(date: chrono::NaiveDate) -> UpdateTaskRequest {
        UpdateTaskRequest {
            title: None,
            description: None,
            recurrence_type: Some(RecurrenceType::Custom),
            recurrence_value: Some(RecurrenceValue::CustomDates(vec![date])),
            assigned_user_id: None,
            target_count: None,
            time_period: None,
            allow_exceed_target: None,
            requires_review: None,
            points_reward: None,
            points_penalty: None,
            due_time: None,
            habit_type: None,
            category_id: None,
            archived: None,
            paused: None,
        }
    }

    #[test]
    fn test_solo_mode_set_date_valid_unscheduled_task() {
        // Unscheduled task (OneTime with no recurrence_value)
        let task = create_test_task(RecurrenceType::OneTime, None);
        let date = chrono::NaiveDate::from_ymd_opt(2026, 3, 15).unwrap();
        let request = create_set_date_request(date);

        assert!(is_solo_mode_set_date_request(&request, &task));
    }

    #[test]
    fn test_solo_mode_set_date_rejected_for_scheduled_task() {
        // Already scheduled task (Custom with dates)
        let date = chrono::NaiveDate::from_ymd_opt(2026, 3, 10).unwrap();
        let task = create_test_task(
            RecurrenceType::Custom,
            Some(RecurrenceValue::CustomDates(vec![date])),
        );
        let new_date = chrono::NaiveDate::from_ymd_opt(2026, 3, 15).unwrap();
        let request = create_set_date_request(new_date);

        // Should be rejected - task already has a schedule
        assert!(!is_solo_mode_set_date_request(&request, &task));
    }

    #[test]
    fn test_solo_mode_set_date_rejected_for_daily_task() {
        // Daily task
        let task = create_test_task(RecurrenceType::Daily, None);
        let date = chrono::NaiveDate::from_ymd_opt(2026, 3, 15).unwrap();
        let request = create_set_date_request(date);

        // Should be rejected - daily tasks have a schedule
        assert!(!is_solo_mode_set_date_request(&request, &task));
    }

    #[test]
    fn test_solo_mode_set_date_rejected_with_other_fields() {
        // Unscheduled task
        let task = create_test_task(RecurrenceType::OneTime, None);
        let date = chrono::NaiveDate::from_ymd_opt(2026, 3, 15).unwrap();

        // Request that also tries to change title
        let request = UpdateTaskRequest {
            title: Some("New Title".to_string()),
            description: None,
            recurrence_type: Some(RecurrenceType::Custom),
            recurrence_value: Some(RecurrenceValue::CustomDates(vec![date])),
            assigned_user_id: None,
            target_count: None,
            time_period: None,
            allow_exceed_target: None,
            requires_review: None,
            points_reward: None,
            points_penalty: None,
            due_time: None,
            habit_type: None,
            category_id: None,
            archived: None,
            paused: None,
        };

        // Should be rejected - trying to change other fields
        assert!(!is_solo_mode_set_date_request(&request, &task));
    }

    #[test]
    fn test_solo_mode_set_date_rejected_without_custom_type() {
        // Unscheduled task
        let task = create_test_task(RecurrenceType::OneTime, None);

        // Request that sets recurrence_value but not recurrence_type to Custom
        let request = UpdateTaskRequest {
            title: None,
            description: None,
            recurrence_type: Some(RecurrenceType::Daily),
            recurrence_value: None,
            assigned_user_id: None,
            target_count: None,
            time_period: None,
            allow_exceed_target: None,
            requires_review: None,
            points_reward: None,
            points_penalty: None,
            due_time: None,
            habit_type: None,
            category_id: None,
            archived: None,
            paused: None,
        };

        // Should be rejected - not setting to Custom type
        assert!(!is_solo_mode_set_date_request(&request, &task));
    }

    #[test]
    fn test_solo_mode_set_date_rejected_without_dates() {
        // Unscheduled task
        let task = create_test_task(RecurrenceType::OneTime, None);

        // Request that sets Custom type but no dates
        let request = UpdateTaskRequest {
            title: None,
            description: None,
            recurrence_type: Some(RecurrenceType::Custom),
            recurrence_value: None,
            assigned_user_id: None,
            target_count: None,
            time_period: None,
            allow_exceed_target: None,
            requires_review: None,
            points_reward: None,
            points_penalty: None,
            due_time: None,
            habit_type: None,
            category_id: None,
            archived: None,
            paused: None,
        };

        // Should be rejected - no dates provided
        assert!(!is_solo_mode_set_date_request(&request, &task));
    }
}
