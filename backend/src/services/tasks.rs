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
    let allow_exceed_target = request.allow_exceed_target.unwrap_or(true);

    let recurrence_value = request
        .recurrence_value
        .as_ref()
        .map(|v| serde_json::to_string(v).unwrap_or_default());

    let time_period_str = request.time_period.as_ref().map(|p| p.as_str());

    sqlx::query(
        r#"
        INSERT INTO tasks (id, household_id, title, description, recurrence_type, recurrence_value, assigned_user_id, target_count, time_period, allow_exceed_target, created_at, updated_at)
        VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
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
    .bind(allow_exceed_target)
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
        allow_exceed_target,
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

    // Calculate next due date
    let next_due_date = scheduler::get_next_due_date(&task, today);

    Ok(Some(TaskWithStatus {
        task,
        completions_today,
        current_streak,
        last_completion: last_completion.map(|c| c.completed_at),
        next_due_date,
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
    if let Some(allow_exceed_target) = request.allow_exceed_target {
        task.allow_exceed_target = allow_exceed_target;
    }

    let now = Utc::now();
    task.updated_at = now;

    sqlx::query(
        r#"
        UPDATE tasks SET title = ?, description = ?, recurrence_type = ?, recurrence_value = ?, assigned_user_id = ?, target_count = ?, time_period = ?, allow_exceed_target = ?, updated_at = ?
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
    .bind(task.allow_exceed_target)
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
        if task.target_count > 0 && !task.allow_exceed_target {
            // One-time task with exceed disabled: check total completions EVER (across all time)
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
        // else: free-form (target=0) or allow_exceed_target=true, always allow completion
    } else {
        // Scheduled tasks: existing logic with period bounds
        // Check if task is due today
        if !scheduler::is_task_due_on_date(&task, today) {
            return Err(TaskError::NotDueToday);
        }

        // Check if target completions already reached for this period (only if exceed is disabled)
        if !task.allow_exceed_target {
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

/// Get all tasks for a household with their status (not just due today)
/// Tasks are returned sorted by next_due_date (tasks due sooner first, None last)
pub async fn get_all_tasks_with_status(
    pool: &SqlitePool,
    household_id: &Uuid,
    user_id: &Uuid,
) -> Result<Vec<TaskWithStatus>, TaskError> {
    let tasks = list_tasks(pool, household_id).await?;

    let mut tasks_with_status = Vec::new();

    for task in tasks {
        let status = get_task_with_status(pool, &task.id, user_id).await?;
        if let Some(s) = status {
            tasks_with_status.push(s);
        }
    }

    // Sort by next_due_date: tasks with dates first (ascending), then tasks without dates
    tasks_with_status.sort_by(|a, b| {
        match (&a.next_due_date, &b.next_due_date) {
            (Some(date_a), Some(date_b)) => date_a.cmp(date_b),
            (Some(_), None) => std::cmp::Ordering::Less,
            (None, Some(_)) => std::cmp::Ordering::Greater,
            (None, None) => std::cmp::Ordering::Equal,
        }
    });

    Ok(tasks_with_status)
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
    use shared::{CreateTaskRequest, RecurrenceType};

    #[test]
    fn test_task_error_display() {
        assert_eq!(TaskError::NotFound.to_string(), "Task not found");
        assert_eq!(
            TaskError::AlreadyCompleted.to_string(),
            "Task already completed for today"
        );
    }

    async fn setup_test_db() -> SqlitePool {
        let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();

        // Run migrations
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS users (
                id TEXT PRIMARY KEY NOT NULL,
                username TEXT NOT NULL UNIQUE,
                email TEXT NOT NULL UNIQUE,
                password_hash TEXT NOT NULL,
                created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
                updated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
            )
            "#,
        )
        .execute(&pool)
        .await
        .unwrap();

        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS households (
                id TEXT PRIMARY KEY NOT NULL,
                name TEXT NOT NULL,
                owner_id TEXT NOT NULL REFERENCES users(id),
                created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
                updated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
            )
            "#,
        )
        .execute(&pool)
        .await
        .unwrap();

        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS tasks (
                id TEXT PRIMARY KEY NOT NULL,
                household_id TEXT NOT NULL REFERENCES households(id),
                title TEXT NOT NULL,
                description TEXT NOT NULL DEFAULT '',
                recurrence_type TEXT NOT NULL DEFAULT 'daily',
                recurrence_value TEXT,
                assigned_user_id TEXT REFERENCES users(id),
                target_count INTEGER NOT NULL DEFAULT 1,
                time_period TEXT,
                allow_exceed_target BOOLEAN NOT NULL DEFAULT 1,
                created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
                updated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
            )
            "#,
        )
        .execute(&pool)
        .await
        .unwrap();

        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS task_completions (
                id TEXT PRIMARY KEY NOT NULL,
                task_id TEXT NOT NULL REFERENCES tasks(id),
                user_id TEXT NOT NULL REFERENCES users(id),
                completed_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
                due_date DATE NOT NULL
            )
            "#,
        )
        .execute(&pool)
        .await
        .unwrap();

        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS memberships (
                id TEXT PRIMARY KEY NOT NULL,
                household_id TEXT NOT NULL REFERENCES households(id),
                user_id TEXT NOT NULL REFERENCES users(id),
                role TEXT NOT NULL DEFAULT 'member',
                points INTEGER NOT NULL DEFAULT 0,
                created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
                UNIQUE(household_id, user_id)
            )
            "#,
        )
        .execute(&pool)
        .await
        .unwrap();

        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS point_conditions (
                id TEXT PRIMARY KEY NOT NULL,
                household_id TEXT NOT NULL REFERENCES households(id),
                name TEXT NOT NULL,
                condition_type TEXT NOT NULL,
                points_value INTEGER NOT NULL,
                streak_threshold INTEGER,
                multiplier REAL,
                task_id TEXT REFERENCES tasks(id),
                created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
            )
            "#,
        )
        .execute(&pool)
        .await
        .unwrap();

        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS task_rewards (
                id TEXT PRIMARY KEY NOT NULL,
                task_id TEXT NOT NULL REFERENCES tasks(id),
                reward_id TEXT NOT NULL,
                created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
            )
            "#,
        )
        .execute(&pool)
        .await
        .unwrap();

        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS task_punishments (
                id TEXT PRIMARY KEY NOT NULL,
                task_id TEXT NOT NULL REFERENCES tasks(id),
                punishment_id TEXT NOT NULL,
                created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
            )
            "#,
        )
        .execute(&pool)
        .await
        .unwrap();

        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS user_rewards (
                id TEXT PRIMARY KEY NOT NULL,
                user_id TEXT NOT NULL REFERENCES users(id),
                reward_id TEXT NOT NULL,
                household_id TEXT NOT NULL REFERENCES households(id),
                amount INTEGER NOT NULL DEFAULT 1,
                redeemed_amount INTEGER NOT NULL DEFAULT 0,
                created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
            )
            "#,
        )
        .execute(&pool)
        .await
        .unwrap();

        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS user_punishments (
                id TEXT PRIMARY KEY NOT NULL,
                user_id TEXT NOT NULL REFERENCES users(id),
                punishment_id TEXT NOT NULL,
                household_id TEXT NOT NULL REFERENCES households(id),
                amount INTEGER NOT NULL DEFAULT 1,
                completed_amount INTEGER NOT NULL DEFAULT 0,
                created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
            )
            "#,
        )
        .execute(&pool)
        .await
        .unwrap();

        pool
    }

    async fn create_test_user(pool: &SqlitePool) -> Uuid {
        let user_id = Uuid::new_v4();
        sqlx::query(
            "INSERT INTO users (id, username, email, password_hash) VALUES (?, ?, ?, ?)",
        )
        .bind(user_id.to_string())
        .bind("testuser")
        .bind("test@example.com")
        .bind("hash")
        .execute(pool)
        .await
        .unwrap();
        user_id
    }

    async fn create_test_household(pool: &SqlitePool, owner_id: &Uuid) -> Uuid {
        let household_id = Uuid::new_v4();
        sqlx::query(
            "INSERT INTO households (id, name, owner_id) VALUES (?, ?, ?)",
        )
        .bind(household_id.to_string())
        .bind("Test Household")
        .bind(owner_id.to_string())
        .execute(pool)
        .await
        .unwrap();

        // Add membership
        sqlx::query(
            "INSERT INTO memberships (id, household_id, user_id, role) VALUES (?, ?, ?, ?)",
        )
        .bind(Uuid::new_v4().to_string())
        .bind(household_id.to_string())
        .bind(owner_id.to_string())
        .bind("owner")
        .execute(pool)
        .await
        .unwrap();

        household_id
    }

    #[tokio::test]
    async fn test_complete_task_allow_exceed_true() {
        let pool = setup_test_db().await;
        let user_id = create_test_user(&pool).await;
        let household_id = create_test_household(&pool, &user_id).await;

        // Create task with allow_exceed_target = true and target_count = 1
        let request = CreateTaskRequest {
            title: "Test Task".to_string(),
            description: None,
            recurrence_type: RecurrenceType::Daily,
            recurrence_value: None,
            assigned_user_id: None,
            target_count: Some(1),
            time_period: None,
            allow_exceed_target: Some(true),
        };
        let task = create_task(&pool, &household_id, &request).await.unwrap();

        // First completion should succeed
        let result1 = complete_task(&pool, &task.id, &user_id, &household_id).await;
        assert!(result1.is_ok());

        // Second completion should also succeed (exceeding target)
        let result2 = complete_task(&pool, &task.id, &user_id, &household_id).await;
        assert!(result2.is_ok());

        // Third completion should also succeed
        let result3 = complete_task(&pool, &task.id, &user_id, &household_id).await;
        assert!(result3.is_ok());
    }

    #[tokio::test]
    async fn test_complete_task_allow_exceed_false() {
        let pool = setup_test_db().await;
        let user_id = create_test_user(&pool).await;
        let household_id = create_test_household(&pool, &user_id).await;

        // Create task with allow_exceed_target = false and target_count = 1
        let request = CreateTaskRequest {
            title: "Test Task".to_string(),
            description: None,
            recurrence_type: RecurrenceType::Daily,
            recurrence_value: None,
            assigned_user_id: None,
            target_count: Some(1),
            time_period: None,
            allow_exceed_target: Some(false),
        };
        let task = create_task(&pool, &household_id, &request).await.unwrap();

        // First completion should succeed
        let result1 = complete_task(&pool, &task.id, &user_id, &household_id).await;
        assert!(result1.is_ok());

        // Second completion should fail (target reached, no exceed allowed)
        let result2 = complete_task(&pool, &task.id, &user_id, &household_id).await;
        assert!(matches!(result2, Err(TaskError::AlreadyCompleted)));
    }

    #[tokio::test]
    async fn test_complete_task_allow_exceed_false_target_2() {
        let pool = setup_test_db().await;
        let user_id = create_test_user(&pool).await;
        let household_id = create_test_household(&pool, &user_id).await;

        // Create task with allow_exceed_target = false and target_count = 2
        let request = CreateTaskRequest {
            title: "Test Task".to_string(),
            description: None,
            recurrence_type: RecurrenceType::Daily,
            recurrence_value: None,
            assigned_user_id: None,
            target_count: Some(2),
            time_period: None,
            allow_exceed_target: Some(false),
        };
        let task = create_task(&pool, &household_id, &request).await.unwrap();

        // First completion should succeed
        let result1 = complete_task(&pool, &task.id, &user_id, &household_id).await;
        assert!(result1.is_ok());

        // Second completion should succeed (reaching target)
        let result2 = complete_task(&pool, &task.id, &user_id, &household_id).await;
        assert!(result2.is_ok());

        // Third completion should fail (target reached, no exceed allowed)
        let result3 = complete_task(&pool, &task.id, &user_id, &household_id).await;
        assert!(matches!(result3, Err(TaskError::AlreadyCompleted)));
    }

    #[tokio::test]
    async fn test_complete_task_default_allow_exceed() {
        let pool = setup_test_db().await;
        let user_id = create_test_user(&pool).await;
        let household_id = create_test_household(&pool, &user_id).await;

        // Create task without specifying allow_exceed_target (should default to true)
        let request = CreateTaskRequest {
            title: "Test Task".to_string(),
            description: None,
            recurrence_type: RecurrenceType::Daily,
            recurrence_value: None,
            assigned_user_id: None,
            target_count: Some(1),
            time_period: None,
            allow_exceed_target: None, // Default
        };
        let task = create_task(&pool, &household_id, &request).await.unwrap();

        // Verify default is true
        assert!(task.allow_exceed_target);

        // First completion should succeed
        let result1 = complete_task(&pool, &task.id, &user_id, &household_id).await;
        assert!(result1.is_ok());

        // Second completion should also succeed (default allows exceeding)
        let result2 = complete_task(&pool, &task.id, &user_id, &household_id).await;
        assert!(result2.is_ok());
    }
}
