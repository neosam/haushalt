use chrono::Utc;
use sqlx::SqlitePool;
use thiserror::Error;
use uuid::Uuid;

use crate::models::{TaskCompletionRow, TaskRow};
use crate::services::{points as points_service, scheduler, task_consequences};
use shared::{CreateTaskRequest, Task, TaskCompletion, TaskWithStatus, UpdateTaskRequest};

#[derive(Debug, Error)]
pub enum TaskError {
    #[error("Task not found")]
    NotFound,
    #[error("Task already completed for today")]
    AlreadyCompleted,
    #[error("Task not due today")]
    NotDueToday,
    #[error("No completion to undo")]
    NotCompleted,
    #[error("Database error: {0}")]
    DatabaseError(#[from] sqlx::Error),
}

pub async fn create_task(
    pool: &SqlitePool,
    household_id: &Uuid,
    request: &CreateTaskRequest,
) -> Result<Task, TaskError> {
    let id = Uuid::new_v4();
    let now = Utc::now();
    let target_count = request.target_count.unwrap_or(1);

    let recurrence_value = request
        .recurrence_value
        .as_ref()
        .map(|v| serde_json::to_string(v).unwrap_or_default());

    let time_period_str = request.time_period.as_ref().map(|p| p.as_str());

    sqlx::query(
        r#"
        INSERT INTO tasks (id, household_id, title, description, recurrence_type, recurrence_value, assigned_user_id, target_count, time_period, created_at, updated_at)
        VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
        "#,
    )
    .bind(id.to_string())
    .bind(household_id.to_string())
    .bind(&request.title)
    .bind(request.description.as_deref().unwrap_or(""))
    .bind(request.recurrence_type.as_str())
    .bind(&recurrence_value)
    .bind(request.assigned_user_id.map(|u| u.to_string()))
    .bind(target_count)
    .bind(time_period_str)
    .bind(now)
    .bind(now)
    .execute(pool)
    .await?;

    Ok(Task {
        id,
        household_id: *household_id,
        title: request.title.clone(),
        description: request.description.clone().unwrap_or_default(),
        recurrence_type: request.recurrence_type.clone(),
        recurrence_value: request.recurrence_value.clone(),
        assigned_user_id: request.assigned_user_id,
        target_count,
        time_period: request.time_period,
        created_at: now,
        updated_at: now,
    })
}

pub async fn get_task(pool: &SqlitePool, task_id: &Uuid) -> Result<Option<Task>, TaskError> {
    let task: Option<TaskRow> = sqlx::query_as("SELECT * FROM tasks WHERE id = ?")
        .bind(task_id.to_string())
        .fetch_optional(pool)
        .await?;

    Ok(task.map(|t| t.to_shared()))
}

pub async fn get_task_with_status(
    pool: &SqlitePool,
    task_id: &Uuid,
    user_id: &Uuid,
) -> Result<Option<TaskWithStatus>, TaskError> {
    let task = match get_task(pool, task_id).await? {
        Some(t) => t,
        None => return Ok(None),
    };

    let today = Utc::now().date_naive();

    // Get completion count for today (or current period for weekly/monthly tasks)
    let (period_start, period_end) = scheduler::get_period_bounds(&task, today);
    let completions_today = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM task_completions WHERE task_id = ? AND user_id = ? AND due_date >= ? AND due_date <= ?",
    )
    .bind(task_id.to_string())
    .bind(user_id.to_string())
    .bind(period_start)
    .bind(period_end)
    .fetch_one(pool)
    .await? as i32;

    // Get last completion
    let last_completion: Option<TaskCompletionRow> = sqlx::query_as(
        "SELECT * FROM task_completions WHERE task_id = ? AND user_id = ? ORDER BY completed_at DESC LIMIT 1",
    )
    .bind(task_id.to_string())
    .bind(user_id.to_string())
    .fetch_optional(pool)
    .await?;

    // Calculate streak
    let current_streak = calculate_streak(pool, &task, user_id).await?;

    Ok(Some(TaskWithStatus {
        task,
        completions_today,
        current_streak,
        last_completion: last_completion.map(|c| c.completed_at),
    }))
}

pub async fn list_tasks(pool: &SqlitePool, household_id: &Uuid) -> Result<Vec<Task>, TaskError> {
    let tasks: Vec<TaskRow> = sqlx::query_as(
        "SELECT * FROM tasks WHERE household_id = ? ORDER BY created_at DESC",
    )
    .bind(household_id.to_string())
    .fetch_all(pool)
    .await?;

    Ok(tasks.into_iter().map(|t| t.to_shared()).collect())
}

pub async fn list_user_assigned_tasks(
    pool: &SqlitePool,
    household_id: &Uuid,
    user_id: &Uuid,
) -> Result<Vec<Task>, TaskError> {
    let tasks: Vec<TaskRow> = sqlx::query_as(
        "SELECT * FROM tasks WHERE household_id = ? AND assigned_user_id = ? ORDER BY created_at DESC",
    )
    .bind(household_id.to_string())
    .bind(user_id.to_string())
    .fetch_all(pool)
    .await?;

    Ok(tasks.into_iter().map(|t| t.to_shared()).collect())
}

pub async fn update_task(
    pool: &SqlitePool,
    task_id: &Uuid,
    request: &UpdateTaskRequest,
) -> Result<Task, TaskError> {
    let mut task: TaskRow = sqlx::query_as("SELECT * FROM tasks WHERE id = ?")
        .bind(task_id.to_string())
        .fetch_optional(pool)
        .await?
        .ok_or(TaskError::NotFound)?;

    if let Some(ref title) = request.title {
        task.title = title.clone();
    }
    if let Some(ref description) = request.description {
        task.description = description.clone();
    }
    if let Some(ref recurrence_type) = request.recurrence_type {
        task.recurrence_type = recurrence_type.as_str().to_string();
    }
    if let Some(ref recurrence_value) = request.recurrence_value {
        task.recurrence_value = Some(serde_json::to_string(recurrence_value).unwrap_or_default());
    }
    if let Some(assigned_user_id) = request.assigned_user_id {
        task.assigned_user_id = Some(assigned_user_id.to_string());
    }
    if let Some(target_count) = request.target_count {
        task.target_count = target_count;
    }
    if let Some(time_period) = request.time_period {
        task.time_period = Some(time_period.as_str().to_string());
    }

    let now = Utc::now();
    task.updated_at = now;

    sqlx::query(
        r#"
        UPDATE tasks SET title = ?, description = ?, recurrence_type = ?, recurrence_value = ?, assigned_user_id = ?, target_count = ?, time_period = ?, updated_at = ?
        WHERE id = ?
        "#,
    )
    .bind(&task.title)
    .bind(&task.description)
    .bind(&task.recurrence_type)
    .bind(&task.recurrence_value)
    .bind(&task.assigned_user_id)
    .bind(task.target_count)
    .bind(&task.time_period)
    .bind(now)
    .bind(task_id.to_string())
    .execute(pool)
    .await?;

    Ok(task.to_shared())
}

pub async fn delete_task(pool: &SqlitePool, task_id: &Uuid) -> Result<(), TaskError> {
    // Delete related data first
    sqlx::query("DELETE FROM task_completions WHERE task_id = ?")
        .bind(task_id.to_string())
        .execute(pool)
        .await?;

    sqlx::query("DELETE FROM task_rewards WHERE task_id = ?")
        .bind(task_id.to_string())
        .execute(pool)
        .await?;

    sqlx::query("DELETE FROM task_punishments WHERE task_id = ?")
        .bind(task_id.to_string())
        .execute(pool)
        .await?;

    // Update point conditions to remove task reference
    sqlx::query("UPDATE point_conditions SET task_id = NULL WHERE task_id = ?")
        .bind(task_id.to_string())
        .execute(pool)
        .await?;

    sqlx::query("DELETE FROM tasks WHERE id = ?")
        .bind(task_id.to_string())
        .execute(pool)
        .await?;

    Ok(())
}

pub async fn complete_task(
    pool: &SqlitePool,
    task_id: &Uuid,
    user_id: &Uuid,
    household_id: &Uuid,
) -> Result<TaskCompletion, TaskError> {
    let task = get_task(pool, task_id).await?.ok_or(TaskError::NotFound)?;

    let today = Utc::now().date_naive();

    // Special handling for RecurrenceType::OneTime (free-form and one-time tasks)
    if task.recurrence_type == shared::RecurrenceType::OneTime {
        if task.target_count > 0 {
            // One-time task: check total completions EVER (across all time)
            let total_completions = sqlx::query_scalar::<_, i64>(
                "SELECT COUNT(*) FROM task_completions WHERE task_id = ? AND user_id = ?"
            )
            .bind(task_id.to_string())
            .bind(user_id.to_string())
            .fetch_one(pool)
            .await?;

            if total_completions >= task.target_count as i64 {
                return Err(TaskError::AlreadyCompleted);
            }
        }
        // else: free-form (target=0), always allow completion
    } else {
        // Scheduled tasks: existing logic with period bounds
        // Check if task is due today
        if !scheduler::is_task_due_on_date(&task, today) {
            return Err(TaskError::NotDueToday);
        }

        // Check if target completions already reached for this period
        let (period_start, period_end) = scheduler::get_period_bounds(&task, today);
        let existing = sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*) FROM task_completions WHERE task_id = ? AND user_id = ? AND due_date >= ? AND due_date <= ?",
        )
        .bind(task_id.to_string())
        .bind(user_id.to_string())
        .bind(period_start)
        .bind(period_end)
        .fetch_one(pool)
        .await?;

        if existing >= task.target_count as i64 {
            return Err(TaskError::AlreadyCompleted);
        }
    }

    let id = Uuid::new_v4();
    let now = Utc::now();

    sqlx::query(
        r#"
        INSERT INTO task_completions (id, task_id, user_id, completed_at, due_date)
        VALUES (?, ?, ?, ?, ?)
        "#,
    )
    .bind(id.to_string())
    .bind(task_id.to_string())
    .bind(user_id.to_string())
    .bind(now)
    .bind(today)
    .execute(pool)
    .await?;

    // Award points
    let streak = calculate_streak(pool, &task, user_id).await?;
    points_service::award_task_completion_points(pool, household_id, user_id, task_id, streak)
        .await
        .ok();

    // Assign task-specific rewards
    task_consequences::assign_task_completion_rewards(pool, task_id, user_id, household_id)
        .await
        .ok();

    Ok(TaskCompletion {
        id,
        task_id: *task_id,
        user_id: *user_id,
        completed_at: now,
        due_date: today,
    })
}

pub async fn uncomplete_task(
    pool: &SqlitePool,
    task_id: &Uuid,
    user_id: &Uuid,
) -> Result<(), TaskError> {
    let task = get_task(pool, task_id).await?.ok_or(TaskError::NotFound)?;
    let today = Utc::now().date_naive();

    // Get the period bounds for this task
    let (period_start, period_end) = scheduler::get_period_bounds(&task, today);

    // Delete the most recent completion for this task/user in the current period
    let result = sqlx::query(
        r#"
        DELETE FROM task_completions
        WHERE id = (
            SELECT id FROM task_completions
            WHERE task_id = ? AND user_id = ? AND due_date >= ? AND due_date <= ?
            ORDER BY completed_at DESC
            LIMIT 1
        )
        "#,
    )
    .bind(task_id.to_string())
    .bind(user_id.to_string())
    .bind(period_start)
    .bind(period_end)
    .execute(pool)
    .await?;

    if result.rows_affected() == 0 {
        return Err(TaskError::NotCompleted);
    }

    Ok(())
}

pub async fn get_due_tasks(
    pool: &SqlitePool,
    household_id: &Uuid,
    user_id: &Uuid,
) -> Result<Vec<TaskWithStatus>, TaskError> {
    let tasks = list_tasks(pool, household_id).await?;
    let today = Utc::now().date_naive();

    let mut due_tasks = Vec::new();

    for task in tasks {
        if scheduler::is_task_due_on_date(&task, today) {
            let status = get_task_with_status(pool, &task.id, user_id).await?;
            if let Some(s) = status {
                due_tasks.push(s);
            }
        }
    }

    Ok(due_tasks)
}

async fn calculate_streak(pool: &SqlitePool, task: &Task, user_id: &Uuid) -> Result<i32, TaskError> {
    // Edge case: Free-form and one-time tasks don't have traditional streaks
    if task.recurrence_type == shared::RecurrenceType::OneTime {
        if task.target_count == 0 {
            // Free-form: no schedule, no streak concept
            return Ok(0);
        } else {
            // One-time: return total completions (more intuitive than "streak")
            let completions = sqlx::query_scalar::<_, i64>(
                "SELECT COUNT(*) FROM task_completions WHERE task_id = ? AND user_id = ?"
            )
            .bind(task.id.to_string())
            .bind(user_id.to_string())
            .fetch_one(pool)
            .await? as i32;
            return Ok(completions);
        }
    }

    // Get all completions ordered by due date descending
    let completions: Vec<TaskCompletionRow> = sqlx::query_as(
        "SELECT * FROM task_completions WHERE task_id = ? AND user_id = ? ORDER BY due_date DESC",
    )
    .bind(task.id.to_string())
    .bind(user_id.to_string())
    .fetch_all(pool)
    .await?;

    if completions.is_empty() {
        return Ok(0);
    }

    let today = Utc::now().date_naive();
    let mut streak = 0;
    let mut expected_date = today;

    for completion in completions {
        // For daily tasks, we expect consecutive days
        // For other recurrence types, we check if the completion matches expected due dates
        if completion.due_date == expected_date
            || (completion.due_date == expected_date - chrono::Duration::days(1) && streak == 0)
        {
            streak += 1;
            expected_date = scheduler::get_previous_due_date(task, completion.due_date);
        } else {
            break;
        }
    }

    Ok(streak)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_task_error_display() {
        assert_eq!(TaskError::NotFound.to_string(), "Task not found");
        assert_eq!(
            TaskError::AlreadyCompleted.to_string(),
            "Task already completed for today"
        );
    }
}
