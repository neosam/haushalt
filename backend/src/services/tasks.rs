use chrono::{Datelike, NaiveDate, Utc};
use sqlx::SqlitePool;
use thiserror::Error;
use uuid::Uuid;

use crate::models::{TaskCompletionRow, TaskRow, TaskRowWithCategory, UserRow};
use crate::services::{households as household_service, period_results, points as points_service, scheduler, task_consequences};
use shared::{CompletionStatus, CreateTaskRequest, PendingReview, PeriodStatus, Task, TaskCompletion, TaskStatistics, TaskWithDetails, TaskWithStatus, UpdateTaskRequest};

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
    #[error("User is not assigned to this task")]
    NotAssigned,
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
    let requires_review = request.requires_review.unwrap_or(false);
    let habit_type = request.habit_type.unwrap_or_default();

    let recurrence_value = request
        .recurrence_value
        .as_ref()
        .map(|v| serde_json::to_string(v).unwrap_or_default());

    let time_period_str = request.time_period.as_ref().map(|p| p.as_str());

    sqlx::query(
        r#"
        INSERT INTO tasks (id, household_id, title, description, recurrence_type, recurrence_value, assigned_user_id, target_count, time_period, allow_exceed_target, requires_review, points_reward, points_penalty, due_time, habit_type, category_id, created_at, updated_at)
        VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
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
    .bind(requires_review)
    .bind(request.points_reward)
    .bind(request.points_penalty)
    .bind(&request.due_time)
    .bind(habit_type.as_str())
    .bind(request.category_id.map(|c| c.to_string()))
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
        requires_review,
        points_reward: request.points_reward,
        points_penalty: request.points_penalty,
        due_time: request.due_time.clone(),
        habit_type,
        category_id: request.category_id,
        category_name: None,
        archived: false,
        paused: false,
        created_at: now,
        updated_at: now,
    })
}

pub async fn get_task(pool: &SqlitePool, task_id: &Uuid) -> Result<Option<Task>, TaskError> {
    let task: Option<TaskRowWithCategory> = sqlx::query_as(
        r#"
        SELECT t.*, tc.name as category_name
        FROM tasks t
        LEFT JOIN task_categories tc ON t.category_id = tc.id
        WHERE t.id = ?
        "#
    )
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

    // Calculate next due date first
    let next_due_date = scheduler::get_next_due_date(&task, today);

    // Get completion count for the current period
    // Use next_due_date for period calculation to match how completions are stored
    // This ensures completions made "early" for the next occurrence are counted correctly
    let period_date = next_due_date.unwrap_or(today);
    let (period_start, period_end) = scheduler::get_period_bounds(&task, period_date);
    let completions_today = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM task_completions WHERE task_id = ? AND due_date >= ? AND due_date <= ?",
    )
    .bind(task_id.to_string())
    .bind(period_start)
    .bind(period_end)
    .fetch_one(pool)
    .await? as i32;

    // Get last completion (household-wide)
    let last_completion: Option<TaskCompletionRow> = sqlx::query_as(
        "SELECT * FROM task_completions WHERE task_id = ? ORDER BY completed_at DESC LIMIT 1",
    )
    .bind(task_id.to_string())
    .fetch_optional(pool)
    .await?;

    // Calculate streak
    let current_streak = calculate_streak(pool, &task, user_id).await?;

    // Check if user is assigned to this task
    let is_user_assigned = task.assigned_user_id
        .map(|assigned_id| assigned_id == *user_id)
        .unwrap_or(true); // If no assignment, anyone can complete

    // Get recent periods for habit tracker display (last 15)
    let recent_periods = period_results::get_recent_periods(pool, task_id, 15)
        .await
        .unwrap_or_default();

    Ok(Some(TaskWithStatus {
        task,
        completions_today,
        current_streak,
        last_completion: last_completion.map(|c| c.completed_at),
        next_due_date,
        is_user_assigned,
        recent_periods,
    }))
}

/// Get full task details including statistics for the detail view
pub async fn get_task_with_details(
    pool: &SqlitePool,
    task_id: &Uuid,
    user_id: &Uuid,
) -> Result<Option<TaskWithDetails>, TaskError> {
    let task = match get_task(pool, task_id).await? {
        Some(t) => t,
        None => return Ok(None),
    };

    let today = Utc::now().date_naive();

    // Get all completions for this task (ordered by due_date)
    let completions: Vec<TaskCompletionRow> = sqlx::query_as(
        "SELECT * FROM task_completions WHERE task_id = ? ORDER BY due_date ASC",
    )
    .bind(task_id.to_string())
    .fetch_all(pool)
    .await?;

    // Calculate statistics
    let statistics = calculate_task_statistics(pool, &task, &completions, today, user_id).await?;

    // Get assigned user if any
    let assigned_user = if let Some(assigned_id) = task.assigned_user_id {
        let user_row: Option<UserRow> = sqlx::query_as(
            "SELECT * FROM users WHERE id = ?"
        )
        .bind(assigned_id.to_string())
        .fetch_optional(pool)
        .await?;
        user_row.map(|u| u.to_shared())
    } else {
        None
    };

    // Get linked rewards and punishments
    let linked_rewards = task_consequences::get_task_rewards(pool, task_id)
        .await
        .unwrap_or_default();
    let linked_punishments = task_consequences::get_task_punishments(pool, task_id)
        .await
        .unwrap_or_default();

    // Get recent periods for habit tracker display (last 15)
    let recent_periods = period_results::get_recent_periods(pool, task_id, 15)
        .await
        .unwrap_or_default();

    Ok(Some(TaskWithDetails {
        task,
        statistics,
        assigned_user,
        linked_rewards,
        linked_punishments,
        recent_periods,
    }))
}

/// Calculate task statistics for the detail view
/// Statistics are based on explicitly recorded period results only.
/// If no period results exist, statistics will show 0/0.
async fn calculate_task_statistics(
    pool: &SqlitePool,
    task: &Task,
    completions: &[TaskCompletionRow],
    today: NaiveDate,
    user_id: &Uuid,
) -> Result<TaskStatistics, TaskError> {
    // Get last completion
    let last_completed = completions.last().map(|c| c.completed_at);

    // Calculate next due date
    let next_due = scheduler::get_next_due_date(task, today);

    // Get current streak
    let current_streak = calculate_streak(pool, task, user_id).await?;

    // Calculate best streak
    let best_streak = calculate_best_streak(task, completions);

    // Total completions
    let total_completions = completions.len() as i64;

    // Get period result counts from task_period_results table
    // Statistics are now based only on explicitly recorded period results
    let counts_week = period_results::count_period_results(
        pool,
        &task.id,
        get_week_start(today),
        today,
    )
    .await
    .unwrap_or(period_results::PeriodCounts {
        completed: 0,
        failed: 0,
        skipped: 0,
    });

    let counts_month = period_results::count_period_results(
        pool,
        &task.id,
        get_month_start(today),
        today,
    )
    .await
    .unwrap_or(period_results::PeriodCounts {
        completed: 0,
        failed: 0,
        skipped: 0,
    });

    let counts_all_time = period_results::count_period_results(
        pool,
        &task.id,
        task.created_at.date_naive(),
        today,
    )
    .await
    .unwrap_or(period_results::PeriodCounts {
        completed: 0,
        failed: 0,
        skipped: 0,
    });

    // Calculate totals (completed + failed, excluding skipped)
    let total_week = counts_week.completed + counts_week.failed;
    let total_month = counts_month.completed + counts_month.failed;
    let total_all_time = counts_all_time.completed + counts_all_time.failed;

    // Calculate completion rates: completed / (completed + failed) * 100
    // Returns None if no period results exist (0/0)
    let rate_week = if total_week > 0 {
        Some((counts_week.completed as f64 / total_week as f64) * 100.0)
    } else {
        None
    };

    let rate_month = if total_month > 0 {
        Some((counts_month.completed as f64 / total_month as f64) * 100.0)
    } else {
        None
    };

    let rate_all_time = if total_all_time > 0 {
        Some((counts_all_time.completed as f64 / total_all_time as f64) * 100.0)
    } else {
        None
    };

    Ok(TaskStatistics {
        completion_rate_week: rate_week,
        completion_rate_month: rate_month,
        completion_rate_all_time: rate_all_time,
        periods_completed_week: counts_week.completed,
        periods_total_week: total_week,
        periods_completed_month: counts_month.completed,
        periods_total_month: total_month,
        periods_completed_all_time: counts_all_time.completed,
        periods_total_all_time: total_all_time,
        periods_skipped_week: counts_week.skipped,
        periods_skipped_month: counts_month.skipped,
        periods_skipped_all_time: counts_all_time.skipped,
        current_streak,
        best_streak,
        total_completions,
        last_completed,
        next_due,
    })
}

/// Calculate the best (longest) streak ever achieved for a task
fn calculate_best_streak(task: &Task, completions: &[TaskCompletionRow]) -> i32 {
    if completions.is_empty() {
        return 0;
    }

    // For OneTime tasks, best streak is total completions
    if task.recurrence_type == shared::RecurrenceType::OneTime {
        return completions.len() as i32;
    }

    let mut best_streak = 0;
    let mut current_streak = 0;
    let mut last_due_date: Option<NaiveDate> = None;

    // Group completions by due_date and check consecutive periods
    let mut due_dates: Vec<NaiveDate> = completions.iter()
        .map(|c| c.due_date)
        .collect();
    due_dates.sort();
    due_dates.dedup();

    for due_date in due_dates {
        // Count completions for this date
        let count = completions.iter()
            .filter(|c| c.due_date == due_date)
            .count() as i32;

        // Check if target was met
        if count >= task.target_count {
            if let Some(last) = last_due_date {
                // Check if this is the expected next due date (get next due date after the last one)
                let expected_next = scheduler::get_next_due_date(task, last + chrono::Duration::days(1));
                if expected_next == Some(due_date) {
                    current_streak += 1;
                } else {
                    // Streak broken, start new one
                    current_streak = 1;
                }
            } else {
                current_streak = 1;
            }
            last_due_date = Some(due_date);
            best_streak = best_streak.max(current_streak);
        } else {
            // Target not met, streak broken
            current_streak = 0;
            last_due_date = None;
        }
    }

    best_streak
}

/// Get the start of the current week (Monday)
fn get_week_start(date: NaiveDate) -> NaiveDate {
    let days_from_monday = date.weekday().num_days_from_monday();
    date - chrono::Duration::days(days_from_monday as i64)
}

/// Get the start of the current month
fn get_month_start(date: NaiveDate) -> NaiveDate {
    NaiveDate::from_ymd_opt(date.year(), date.month(), 1).unwrap_or(date)
}

pub async fn list_tasks(pool: &SqlitePool, household_id: &Uuid) -> Result<Vec<Task>, TaskError> {
    let tasks: Vec<TaskRowWithCategory> = sqlx::query_as(
        r#"
        SELECT t.*, tc.name as category_name
        FROM tasks t
        LEFT JOIN task_categories tc ON t.category_id = tc.id
        WHERE t.household_id = ? AND t.archived = 0
        ORDER BY t.title COLLATE NOCASE ASC
        "#,
    )
    .bind(household_id.to_string())
    .fetch_all(pool)
    .await?;

    Ok(tasks.into_iter().map(|t| t.to_shared()).collect())
}

pub async fn list_archived_tasks(pool: &SqlitePool, household_id: &Uuid) -> Result<Vec<Task>, TaskError> {
    let tasks: Vec<TaskRowWithCategory> = sqlx::query_as(
        r#"
        SELECT t.*, tc.name as category_name
        FROM tasks t
        LEFT JOIN task_categories tc ON t.category_id = tc.id
        WHERE t.household_id = ? AND t.archived = 1
        ORDER BY t.title COLLATE NOCASE ASC
        "#,
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
    let tasks: Vec<TaskRowWithCategory> = sqlx::query_as(
        r#"
        SELECT t.*, tc.name as category_name
        FROM tasks t
        LEFT JOIN task_categories tc ON t.category_id = tc.id
        WHERE t.household_id = ? AND t.assigned_user_id = ? AND t.archived = 0
        ORDER BY t.title COLLATE NOCASE ASC
        "#,
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
    if let Some(requires_review) = request.requires_review {
        task.requires_review = requires_review;
    }
    if request.points_reward.is_some() {
        task.points_reward = request.points_reward;
    }
    if request.points_penalty.is_some() {
        task.points_penalty = request.points_penalty;
    }
    if request.due_time.is_some() {
        task.due_time = request.due_time.clone();
    }
    if let Some(habit_type) = request.habit_type {
        task.habit_type = habit_type.as_str().to_string();
    }
    if let Some(ref category_id_opt) = request.category_id {
        task.category_id = category_id_opt.map(|id| id.to_string());
    }
    if let Some(archived) = request.archived {
        task.archived = archived;
    }
    if let Some(paused) = request.paused {
        task.paused = paused;
    }

    let now = Utc::now();
    task.updated_at = now;

    sqlx::query(
        r#"
        UPDATE tasks SET title = ?, description = ?, recurrence_type = ?, recurrence_value = ?, assigned_user_id = ?, target_count = ?, time_period = ?, allow_exceed_target = ?, requires_review = ?, points_reward = ?, points_penalty = ?, due_time = ?, habit_type = ?, category_id = ?, archived = ?, paused = ?, updated_at = ?
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
    .bind(task.requires_review)
    .bind(task.points_reward)
    .bind(task.points_penalty)
    .bind(&task.due_time)
    .bind(&task.habit_type)
    .bind(&task.category_id)
    .bind(task.archived)
    .bind(task.paused)
    .bind(now)
    .bind(task_id.to_string())
    .execute(pool)
    .await?;

    Ok(task.to_shared())
}

pub async fn archive_task(pool: &SqlitePool, task_id: &Uuid) -> Result<Task, TaskError> {
    let now = Utc::now();
    let result = sqlx::query(
        "UPDATE tasks SET archived = 1, updated_at = ? WHERE id = ?",
    )
    .bind(now)
    .bind(task_id.to_string())
    .execute(pool)
    .await?;

    if result.rows_affected() == 0 {
        return Err(TaskError::NotFound);
    }

    get_task(pool, task_id).await?.ok_or(TaskError::NotFound)
}

pub async fn unarchive_task(pool: &SqlitePool, task_id: &Uuid) -> Result<Task, TaskError> {
    let now = Utc::now();
    let result = sqlx::query(
        "UPDATE tasks SET archived = 0, updated_at = ? WHERE id = ?",
    )
    .bind(now)
    .bind(task_id.to_string())
    .execute(pool)
    .await?;

    if result.rows_affected() == 0 {
        return Err(TaskError::NotFound);
    }

    get_task(pool, task_id).await?.ok_or(TaskError::NotFound)
}

pub async fn pause_task(pool: &SqlitePool, task_id: &Uuid) -> Result<Task, TaskError> {
    let now = Utc::now();
    let result = sqlx::query(
        "UPDATE tasks SET paused = 1, updated_at = ? WHERE id = ?",
    )
    .bind(now)
    .bind(task_id.to_string())
    .execute(pool)
    .await?;

    if result.rows_affected() == 0 {
        return Err(TaskError::NotFound);
    }

    get_task(pool, task_id).await?.ok_or(TaskError::NotFound)
}

pub async fn unpause_task(pool: &SqlitePool, task_id: &Uuid) -> Result<Task, TaskError> {
    let now = Utc::now();
    let result = sqlx::query(
        "UPDATE tasks SET paused = 0, updated_at = ? WHERE id = ?",
    )
    .bind(now)
    .bind(task_id.to_string())
    .execute(pool)
    .await?;

    if result.rows_affected() == 0 {
        return Err(TaskError::NotFound);
    }

    get_task(pool, task_id).await?.ok_or(TaskError::NotFound)
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

    // Check if user is allowed to complete this task based on assignment
    if let Some(assigned_id) = task.assigned_user_id {
        if assigned_id != *user_id {
            return Err(TaskError::NotAssigned);
        }
    }

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
        // Scheduled tasks: allow completion within the current period
        // Use next_due_date for period calculation to allow "early" completions
        let next_due = scheduler::get_next_due_date(&task, today);

        // If there's no next due date (e.g., Custom task with all dates passed), don't allow completion
        if next_due.is_none() && task.recurrence_type == shared::RecurrenceType::Custom {
            return Err(TaskError::NotDueToday);
        }

        // Check if target completions already reached for this period (only if exceed is disabled)
        if !task.allow_exceed_target {
            let period_date = next_due.unwrap_or(today);
            let (period_start, period_end) = scheduler::get_period_bounds(&task, period_date);
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

    // Determine the due_date for this completion
    // For scheduled tasks, use the next due date (not today) so completions are counted correctly
    // For OneTime tasks, use today (they have no schedule)
    let completion_due_date = if task.recurrence_type == shared::RecurrenceType::OneTime {
        today
    } else {
        scheduler::get_next_due_date(&task, today).unwrap_or(today)
    };

    // Determine status based on task's requires_review setting
    let status = if task.requires_review {
        CompletionStatus::Pending
    } else {
        CompletionStatus::Approved
    };

    sqlx::query(
        r#"
        INSERT INTO task_completions (id, task_id, user_id, completed_at, due_date, status)
        VALUES (?, ?, ?, ?, ?, ?)
        "#,
    )
    .bind(id.to_string())
    .bind(task_id.to_string())
    .bind(user_id.to_string())
    .bind(now)
    .bind(completion_due_date)
    .bind(status.as_str())
    .execute(pool)
    .await?;

    // Apply consequences based on habit type
    let streak = calculate_streak(pool, &task, user_id).await?;
    if task.habit_type.is_inverted() {
        // Bad habit completed = punishment + point deduction (the bad habit occurred)
        points_service::deduct_bad_habit_completion_points(pool, household_id, user_id, task_id, &task)
            .await
            .ok();

        // Assign task-specific punishments
        task_consequences::assign_task_completion_punishments(pool, task_id, user_id, household_id)
            .await
            .ok();
    } else {
        // Good habit completed = reward + point addition (existing behavior)
        points_service::award_task_completion_points(pool, household_id, user_id, task_id, streak)
            .await
            .ok();

        // Assign task-specific rewards immediately (will be reversed if rejected)
        task_consequences::assign_task_completion_rewards(pool, task_id, user_id, household_id)
            .await
            .ok();
    }

    // Check if period target is now met and finalize as completed
    // Only for scheduled tasks (not OneTime) with target_count > 0
    if task.recurrence_type != shared::RecurrenceType::OneTime && task.target_count > 0 {
        let (period_start, period_end) = scheduler::get_period_bounds(&task, completion_due_date);

        // Count completions for this period (all users if unassigned, specific user if assigned)
        let completions_for_period: i64 = if task.assigned_user_id.is_some() {
            sqlx::query_scalar(
                "SELECT COUNT(*) FROM task_completions WHERE task_id = ? AND user_id = ? AND due_date >= ? AND due_date <= ?",
            )
            .bind(task_id.to_string())
            .bind(user_id.to_string())
            .bind(period_start)
            .bind(period_end)
            .fetch_one(pool)
            .await?
        } else {
            sqlx::query_scalar(
                "SELECT COUNT(*) FROM task_completions WHERE task_id = ? AND due_date >= ? AND due_date <= ?",
            )
            .bind(task_id.to_string())
            .bind(period_start)
            .bind(period_end)
            .fetch_one(pool)
            .await?
        };

        // If target is met, finalize the period as completed
        // This will create a new record or update an existing one (e.g., failed -> completed)
        if completions_for_period >= task.target_count as i64 {
            let _ = period_results::finalize_period(
                pool,
                task_id,
                period_start,
                period_end,
                PeriodStatus::Completed,
                completions_for_period as i32,
                task.target_count,
                "system",
                None,
            )
            .await;
        }
    }

    Ok(TaskCompletion {
        id,
        task_id: *task_id,
        user_id: *user_id,
        completed_at: now,
        due_date: completion_due_date,
        status,
    })
}

pub async fn uncomplete_task(
    pool: &SqlitePool,
    task_id: &Uuid,
    user_id: &Uuid,
) -> Result<(), TaskError> {
    let task = get_task(pool, task_id).await?.ok_or(TaskError::NotFound)?;

    // Check if user is allowed to uncomplete this task based on assignment
    if let Some(assigned_id) = task.assigned_user_id {
        if assigned_id != *user_id {
            return Err(TaskError::NotAssigned);
        }
    }

    let today = Utc::now().date_naive();

    // Use next_due_date for period calculation to match how completions are stored
    let period_date = scheduler::get_next_due_date(&task, today).unwrap_or(today);
    let (period_start, period_end) = scheduler::get_period_bounds(&task, period_date);

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

    // After deleting completion, check if we're now below target
    // If so, delete the period result so it can be re-evaluated
    let completions_for_period: i64 = sqlx::query_scalar(
        r#"
        SELECT COUNT(*) FROM task_completions
        WHERE task_id = ? AND due_date >= ? AND due_date <= ?
        "#,
    )
    .bind(task_id.to_string())
    .bind(period_start)
    .bind(period_end)
    .fetch_one(pool)
    .await?;

    if completions_for_period < task.target_count as i64 {
        // Delete the period result - will be re-created when target is reached
        // or finalized as failed by background job when period ends
        let _ = period_results::delete_period_result(pool, task_id, period_start).await;
    }

    Ok(())
}

/// List all pending completions for a household (for owner review)
pub async fn list_pending_reviews(
    pool: &SqlitePool,
    household_id: &Uuid,
) -> Result<Vec<PendingReview>, TaskError> {
    #[derive(sqlx::FromRow)]
    struct PendingReviewRow {
        // TaskCompletion fields
        tc_id: String,
        tc_task_id: String,
        tc_user_id: String,
        tc_completed_at: chrono::DateTime<chrono::Utc>,
        tc_due_date: chrono::NaiveDate,
        tc_status: String,
        // Task fields
        t_id: String,
        t_household_id: String,
        t_title: String,
        t_description: String,
        t_recurrence_type: String,
        t_recurrence_value: Option<String>,
        t_assigned_user_id: Option<String>,
        t_target_count: i32,
        t_time_period: Option<String>,
        t_allow_exceed_target: bool,
        t_requires_review: bool,
        t_points_reward: Option<i64>,
        t_points_penalty: Option<i64>,
        t_due_time: Option<String>,
        t_habit_type: String,
        t_created_at: chrono::DateTime<chrono::Utc>,
        t_updated_at: chrono::DateTime<chrono::Utc>,
        // User fields
        u_id: String,
        u_username: String,
        u_email: String,
        u_created_at: chrono::DateTime<chrono::Utc>,
        u_updated_at: chrono::DateTime<chrono::Utc>,
    }

    let rows: Vec<PendingReviewRow> = sqlx::query_as(
        r#"
        SELECT
            tc.id as tc_id, tc.task_id as tc_task_id, tc.user_id as tc_user_id,
            tc.completed_at as tc_completed_at, tc.due_date as tc_due_date, tc.status as tc_status,
            t.id as t_id, t.household_id as t_household_id, t.title as t_title,
            t.description as t_description, t.recurrence_type as t_recurrence_type,
            t.recurrence_value as t_recurrence_value, t.assigned_user_id as t_assigned_user_id,
            t.target_count as t_target_count, t.time_period as t_time_period,
            t.allow_exceed_target as t_allow_exceed_target, t.requires_review as t_requires_review,
            t.points_reward as t_points_reward, t.points_penalty as t_points_penalty,
            t.due_time as t_due_time, t.habit_type as t_habit_type,
            t.created_at as t_created_at, t.updated_at as t_updated_at,
            u.id as u_id, u.username as u_username, u.email as u_email,
            u.created_at as u_created_at, u.updated_at as u_updated_at
        FROM task_completions tc
        JOIN tasks t ON tc.task_id = t.id
        JOIN users u ON tc.user_id = u.id
        WHERE t.household_id = ? AND tc.status = 'pending'
        ORDER BY tc.completed_at DESC
        "#,
    )
    .bind(household_id.to_string())
    .fetch_all(pool)
    .await?;

    Ok(rows
        .into_iter()
        .map(|row| {
            let recurrence_value = row.t_recurrence_value.as_ref().and_then(|v| {
                serde_json::from_str(v).ok()
            });
            let time_period = row.t_time_period.as_ref().and_then(|p| p.parse().ok());

            PendingReview {
                completion: shared::TaskCompletion {
                    id: Uuid::parse_str(&row.tc_id).unwrap(),
                    task_id: Uuid::parse_str(&row.tc_task_id).unwrap(),
                    user_id: Uuid::parse_str(&row.tc_user_id).unwrap(),
                    completed_at: row.tc_completed_at,
                    due_date: row.tc_due_date,
                    status: row.tc_status.parse().unwrap_or(CompletionStatus::Approved),
                },
                task: shared::Task {
                    id: Uuid::parse_str(&row.t_id).unwrap(),
                    household_id: Uuid::parse_str(&row.t_household_id).unwrap(),
                    title: row.t_title,
                    description: row.t_description,
                    recurrence_type: row.t_recurrence_type.parse().unwrap_or(shared::RecurrenceType::Daily),
                    recurrence_value,
                    assigned_user_id: row.t_assigned_user_id.as_ref().and_then(|id| Uuid::parse_str(id).ok()),
                    target_count: row.t_target_count,
                    time_period,
                    allow_exceed_target: row.t_allow_exceed_target,
                    requires_review: row.t_requires_review,
                    points_reward: row.t_points_reward,
                    points_penalty: row.t_points_penalty,
                    due_time: row.t_due_time,
                    habit_type: row.t_habit_type.parse().unwrap_or(shared::HabitType::Good),
                    category_id: None,
                    category_name: None,
                    archived: false, // Pending reviews are for active tasks
                    paused: false, // Pending reviews are for active tasks
                    created_at: row.t_created_at,
                    updated_at: row.t_updated_at,
                },
                user: shared::User {
                    id: Uuid::parse_str(&row.u_id).unwrap(),
                    username: row.u_username,
                    email: row.u_email,
                    created_at: row.u_created_at,
                    updated_at: row.u_updated_at,
                },
            }
        })
        .collect())
}

/// Get a specific task completion by ID
pub async fn get_completion(
    pool: &SqlitePool,
    completion_id: &Uuid,
) -> Result<Option<TaskCompletion>, TaskError> {
    let completion: Option<TaskCompletionRow> =
        sqlx::query_as("SELECT * FROM task_completions WHERE id = ?")
            .bind(completion_id.to_string())
            .fetch_optional(pool)
            .await?;

    Ok(completion.map(|c| c.to_shared()))
}

/// Approve a pending task completion
pub async fn approve_completion(
    pool: &SqlitePool,
    completion_id: &Uuid,
) -> Result<TaskCompletion, TaskError> {
    let result = sqlx::query(
        "UPDATE task_completions SET status = 'approved' WHERE id = ? AND status = 'pending'",
    )
    .bind(completion_id.to_string())
    .execute(pool)
    .await?;

    if result.rows_affected() == 0 {
        return Err(TaskError::NotFound);
    }

    get_completion(pool, completion_id)
        .await?
        .ok_or(TaskError::NotFound)
}

/// Reject a pending task completion (deletes it and reverses points/rewards)
pub async fn reject_completion(
    pool: &SqlitePool,
    completion_id: &Uuid,
    household_id: &Uuid,
) -> Result<TaskCompletion, TaskError> {
    // Get the completion before deleting
    let completion = get_completion(pool, completion_id)
        .await?
        .ok_or(TaskError::NotFound)?;

    if completion.status != CompletionStatus::Pending {
        return Err(TaskError::NotFound); // Can only reject pending completions
    }

    // Reverse points - deduct the points that were awarded
    points_service::reverse_task_completion_points(
        pool,
        household_id,
        &completion.user_id,
        &completion.task_id,
    )
    .await
    .ok();

    // Reverse rewards that were assigned from this completion
    // Note: This is a simplified approach - ideally we'd track which rewards came from which completion
    // For now, we'll just remove one instance of each linked reward
    task_consequences::reverse_task_completion_rewards(
        pool,
        &completion.task_id,
        &completion.user_id,
        household_id,
    )
    .await
    .ok();

    // Delete the completion
    sqlx::query("DELETE FROM task_completions WHERE id = ?")
        .bind(completion_id.to_string())
        .execute(pool)
        .await?;

    Ok(completion)
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
    // Secondary sort by title (alphabetical, case-insensitive)
    tasks_with_status.sort_by(|a, b| {
        match (&a.next_due_date, &b.next_due_date) {
            (Some(date_a), Some(date_b)) => date_a
                .cmp(date_b)
                .then_with(|| a.task.title.to_lowercase().cmp(&b.task.title.to_lowercase())),
            (Some(_), None) => std::cmp::Ordering::Less,
            (None, Some(_)) => std::cmp::Ordering::Greater,
            (None, None) => a.task.title.to_lowercase().cmp(&b.task.title.to_lowercase()),
        }
    });

    Ok(tasks_with_status)
}

async fn calculate_streak(pool: &SqlitePool, task: &Task, _user_id: &Uuid) -> Result<i32, TaskError> {
    // Edge case: Free-form and one-time tasks don't have traditional streaks
    if task.recurrence_type == shared::RecurrenceType::OneTime {
        if task.target_count == 0 {
            // Free-form: no schedule, no streak concept
            return Ok(0);
        } else {
            // One-time: return total completions (household-wide)
            let completions = sqlx::query_scalar::<_, i64>(
                "SELECT COUNT(*) FROM task_completions WHERE task_id = ?"
            )
            .bind(task.id.to_string())
            .fetch_one(pool)
            .await? as i32;
            return Ok(completions);
        }
    }

    // Get all completions ordered by due date descending (household-wide)
    let completions: Vec<TaskCompletionRow> = sqlx::query_as(
        "SELECT * FROM task_completions WHERE task_id = ? ORDER BY due_date DESC",
    )
    .bind(task.id.to_string())
    .fetch_all(pool)
    .await?;

    if completions.is_empty() {
        return Ok(0);
    }

    let today = Utc::now().date_naive();
    let mut streak = 0;
    // Start with next_due_date to match how completions are stored
    let mut expected_date = scheduler::get_next_due_date(task, today).unwrap_or(today);

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

// ============================================================================
// Dashboard Task Whitelist
// ============================================================================

/// Get all task IDs that the user has added to their dashboard (excluding archived tasks)
pub async fn get_dashboard_task_ids(
    pool: &SqlitePool,
    user_id: &str,
) -> Result<Vec<String>, TaskError> {
    let rows: Vec<(String,)> = sqlx::query_as(
        "SELECT udt.task_id FROM user_dashboard_tasks udt
         JOIN tasks t ON udt.task_id = t.id
         WHERE udt.user_id = ? AND t.archived = 0",
    )
    .bind(user_id)
    .fetch_all(pool)
    .await?;

    Ok(rows.into_iter().map(|(id,)| id).collect())
}

/// Check if a task is on the user's dashboard
pub async fn is_task_on_dashboard(
    pool: &SqlitePool,
    user_id: &str,
    task_id: &str,
) -> Result<bool, TaskError> {
    let result: Option<(i32,)> = sqlx::query_as(
        "SELECT 1 FROM user_dashboard_tasks WHERE user_id = ? AND task_id = ?",
    )
    .bind(user_id)
    .bind(task_id)
    .fetch_optional(pool)
    .await?;

    Ok(result.is_some())
}

/// Add a task to the user's dashboard
pub async fn add_task_to_dashboard(
    pool: &SqlitePool,
    user_id: &str,
    task_id: &str,
) -> Result<(), TaskError> {
    sqlx::query(
        "INSERT OR IGNORE INTO user_dashboard_tasks (user_id, task_id) VALUES (?, ?)",
    )
    .bind(user_id)
    .bind(task_id)
    .execute(pool)
    .await?;

    Ok(())
}

/// Remove a task from the user's dashboard
pub async fn remove_task_from_dashboard(
    pool: &SqlitePool,
    user_id: &str,
    task_id: &str,
) -> Result<(), TaskError> {
    sqlx::query(
        "DELETE FROM user_dashboard_tasks WHERE user_id = ? AND task_id = ?",
    )
    .bind(user_id)
    .bind(task_id)
    .execute(pool)
    .await?;

    Ok(())
}

/// Get all tasks with status that are on the user's dashboard
pub async fn get_dashboard_tasks_with_status(
    pool: &SqlitePool,
    user_id: &Uuid,
) -> Result<Vec<(TaskWithStatus, Uuid)>, TaskError> {
    let dashboard_task_ids = get_dashboard_task_ids(pool, &user_id.to_string()).await?;

    if dashboard_task_ids.is_empty() {
        return Ok(Vec::new());
    }

    let mut results = Vec::new();

    for task_id_str in dashboard_task_ids {
        if let Ok(task_id) = Uuid::parse_str(&task_id_str) {
            if let Some(task_with_status) = get_task_with_status(pool, &task_id, user_id).await? {
                let household_id = task_with_status.task.household_id;
                results.push((task_with_status, household_id));
            }
        }
    }

    // Sort by next_due_date (primary), then by title (secondary, case-insensitive)
    results.sort_by(|(a, _), (b, _)| {
        match (&a.next_due_date, &b.next_due_date) {
            (Some(date_a), Some(date_b)) => date_a
                .cmp(date_b)
                .then_with(|| a.task.title.to_lowercase().cmp(&b.task.title.to_lowercase())),
            (Some(_), None) => std::cmp::Ordering::Less,
            (None, Some(_)) => std::cmp::Ordering::Greater,
            (None, None) => a.task.title.to_lowercase().cmp(&b.task.title.to_lowercase()),
        }
    });

    Ok(results)
}

/// Get all tasks with status from all households the user is a member of
/// Used by the "Show all" toggle on the dashboard
pub async fn get_all_tasks_across_households(
    pool: &SqlitePool,
    user_id: &Uuid,
) -> Result<Vec<(TaskWithStatus, Uuid)>, TaskError> {
    // Get all households for user
    let households = household_service::list_user_households(pool, user_id)
        .await
        .map_err(|_| TaskError::DatabaseError(sqlx::Error::RowNotFound))?;

    let mut all_tasks = Vec::new();

    for household in households {
        if let Ok(tasks) = get_all_tasks_with_status(pool, &household.id, user_id).await {
            for task in tasks {
                all_tasks.push((task, household.id));
            }
        }
    }

    // Sort by next_due_date (primary), then by title (secondary, case-insensitive)
    all_tasks.sort_by(|(a, _), (b, _)| {
        match (&a.next_due_date, &b.next_due_date) {
            (Some(date_a), Some(date_b)) => date_a
                .cmp(date_b)
                .then_with(|| a.task.title.to_lowercase().cmp(&b.task.title.to_lowercase())),
            (Some(_), None) => std::cmp::Ordering::Less,
            (None, Some(_)) => std::cmp::Ordering::Greater,
            (None, None) => a.task.title.to_lowercase().cmp(&b.task.title.to_lowercase()),
        }
    });

    Ok(all_tasks)
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
                password_hash TEXT,
                oidc_subject TEXT,
                oidc_provider TEXT,
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
            CREATE TABLE IF NOT EXISTS task_categories (
                id TEXT PRIMARY KEY NOT NULL,
                household_id TEXT NOT NULL REFERENCES households(id),
                name TEXT NOT NULL,
                color TEXT,
                sort_order INTEGER NOT NULL DEFAULT 0,
                created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
                UNIQUE(household_id, name)
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
                requires_review BOOLEAN NOT NULL DEFAULT 0,
                points_reward INTEGER,
                points_penalty INTEGER,
                due_time TEXT,
                habit_type TEXT NOT NULL DEFAULT 'good',
                category_id TEXT REFERENCES task_categories(id),
                archived BOOLEAN NOT NULL DEFAULT 0,
                paused BOOLEAN NOT NULL DEFAULT 0,
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
                due_date DATE NOT NULL,
                status TEXT NOT NULL DEFAULT 'approved'
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

        // Also create household_memberships table for household service
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS household_memberships (
                id TEXT PRIMARY KEY NOT NULL,
                household_id TEXT NOT NULL REFERENCES households(id),
                user_id TEXT NOT NULL REFERENCES users(id),
                role TEXT NOT NULL DEFAULT 'member',
                points INTEGER NOT NULL DEFAULT 0,
                joined_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
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

        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS user_dashboard_tasks (
                user_id TEXT NOT NULL,
                task_id TEXT NOT NULL,
                created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
                PRIMARY KEY (user_id, task_id),
                FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE,
                FOREIGN KEY (task_id) REFERENCES tasks(id) ON DELETE CASCADE
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

        // Also add to household_memberships for household service
        sqlx::query(
            "INSERT INTO household_memberships (id, household_id, user_id, role) VALUES (?, ?, ?, ?)",
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
            requires_review: None,
            points_reward: None,
            points_penalty: None,
            due_time: None,
            habit_type: None,
            category_id: None,
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
            requires_review: None,
            points_reward: None,
            points_penalty: None,
            due_time: None,
            habit_type: None,
            category_id: None,
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
            requires_review: None,
            points_reward: None,
            points_penalty: None,
            due_time: None,
            habit_type: None,
            category_id: None,
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
            requires_review: None,
            points_reward: None,
            points_penalty: None,
            due_time: None,
            habit_type: None,
            category_id: None,
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

    #[tokio::test]
    async fn test_create_task_with_habit_type() {
        let pool = setup_test_db().await;
        let user_id = create_test_user(&pool).await;
        let household_id = create_test_household(&pool, &user_id).await;

        // Create good habit (default)
        let request_good = CreateTaskRequest {
            title: "Good Habit".to_string(),
            description: None,
            recurrence_type: RecurrenceType::Daily,
            recurrence_value: None,
            assigned_user_id: None,
            target_count: Some(1),
            time_period: None,
            allow_exceed_target: None,
            requires_review: None,
            points_reward: None,
            points_penalty: None,
            due_time: None,
            habit_type: None, // Default to Good
            category_id: None,
        };
        let task_good = create_task(&pool, &household_id, &request_good).await.unwrap();
        assert_eq!(task_good.habit_type, shared::HabitType::Good);
        assert!(!task_good.habit_type.is_inverted());

        // Create bad habit
        let request_bad = CreateTaskRequest {
            title: "Bad Habit".to_string(),
            description: None,
            recurrence_type: RecurrenceType::Daily,
            recurrence_value: None,
            assigned_user_id: None,
            target_count: Some(1),
            time_period: None,
            allow_exceed_target: None,
            requires_review: None,
            points_reward: None,
            points_penalty: None,
            due_time: None,
            habit_type: Some(shared::HabitType::Bad),
            category_id: None,
        };
        let task_bad = create_task(&pool, &household_id, &request_bad).await.unwrap();
        assert_eq!(task_bad.habit_type, shared::HabitType::Bad);
        assert!(task_bad.habit_type.is_inverted());
    }

    #[tokio::test]
    async fn test_get_dashboard_tasks_with_status_empty() {
        let pool = setup_test_db().await;
        let user_id = create_test_user(&pool).await;

        let dashboard_tasks = get_dashboard_tasks_with_status(&pool, &user_id).await.unwrap();
        assert!(dashboard_tasks.is_empty());
    }

    #[tokio::test]
    async fn test_get_dashboard_tasks_with_status() {
        let pool = setup_test_db().await;
        let user_id = create_test_user(&pool).await;
        let household_id = create_test_household(&pool, &user_id).await;

        // Create a task
        let request = CreateTaskRequest {
            title: "Dashboard Task".to_string(),
            description: None,
            recurrence_type: RecurrenceType::Daily,
            recurrence_value: None,
            assigned_user_id: None,
            target_count: Some(1),
            time_period: None,
            allow_exceed_target: None,
            requires_review: None,
            points_reward: None,
            points_penalty: None,
            due_time: None,
            habit_type: None,
            category_id: None,
        };
        let task = create_task(&pool, &household_id, &request).await.unwrap();

        // Add to dashboard
        add_task_to_dashboard(&pool, &user_id.to_string(), &task.id.to_string())
            .await
            .unwrap();

        // Get dashboard tasks
        let dashboard_tasks = get_dashboard_tasks_with_status(&pool, &user_id).await.unwrap();

        assert_eq!(dashboard_tasks.len(), 1);
        assert_eq!(dashboard_tasks[0].0.task.id, task.id);
        assert_eq!(dashboard_tasks[0].1, household_id);
    }

    #[tokio::test]
    async fn test_get_dashboard_tasks_includes_task_not_on_dashboard() {
        let pool = setup_test_db().await;
        let user_id = create_test_user(&pool).await;
        let household_id = create_test_household(&pool, &user_id).await;

        // Create a task but don't add it to dashboard
        let request = CreateTaskRequest {
            title: "Not on Dashboard".to_string(),
            description: None,
            recurrence_type: RecurrenceType::Daily,
            recurrence_value: None,
            assigned_user_id: None,
            target_count: Some(1),
            time_period: None,
            allow_exceed_target: None,
            requires_review: None,
            points_reward: None,
            points_penalty: None,
            due_time: None,
            habit_type: None,
            category_id: None,
        };
        let _task = create_task(&pool, &household_id, &request).await.unwrap();

        // Get dashboard tasks - should be empty since task is not on dashboard
        let dashboard_tasks = get_dashboard_tasks_with_status(&pool, &user_id).await.unwrap();
        assert!(dashboard_tasks.is_empty());
    }

    #[tokio::test]
    async fn test_get_dashboard_tasks_excludes_archived() {
        let pool = setup_test_db().await;
        let user_id = create_test_user(&pool).await;
        let household_id = create_test_household(&pool, &user_id).await;

        // Create two tasks
        let request1 = CreateTaskRequest {
            title: "Active Dashboard Task".to_string(),
            description: None,
            recurrence_type: RecurrenceType::Daily,
            recurrence_value: None,
            assigned_user_id: None,
            target_count: Some(1),
            time_period: None,
            allow_exceed_target: None,
            requires_review: None,
            points_reward: None,
            points_penalty: None,
            due_time: None,
            habit_type: None,
            category_id: None,
        };
        let task1 = create_task(&pool, &household_id, &request1).await.unwrap();

        let request2 = CreateTaskRequest {
            title: "Archived Dashboard Task".to_string(),
            description: None,
            recurrence_type: RecurrenceType::Daily,
            recurrence_value: None,
            assigned_user_id: None,
            target_count: Some(1),
            time_period: None,
            allow_exceed_target: None,
            requires_review: None,
            points_reward: None,
            points_penalty: None,
            due_time: None,
            habit_type: None,
            category_id: None,
        };
        let task2 = create_task(&pool, &household_id, &request2).await.unwrap();

        // Add both to dashboard
        add_task_to_dashboard(&pool, &user_id.to_string(), &task1.id.to_string())
            .await
            .unwrap();
        add_task_to_dashboard(&pool, &user_id.to_string(), &task2.id.to_string())
            .await
            .unwrap();

        // Verify both are on dashboard
        let dashboard_tasks = get_dashboard_tasks_with_status(&pool, &user_id).await.unwrap();
        assert_eq!(dashboard_tasks.len(), 2);

        // Archive task2
        archive_task(&pool, &task2.id).await.unwrap();

        // Verify only task1 is on dashboard now
        let dashboard_tasks = get_dashboard_tasks_with_status(&pool, &user_id).await.unwrap();
        assert_eq!(dashboard_tasks.len(), 1);
        assert_eq!(dashboard_tasks[0].0.task.id, task1.id);
    }

    #[tokio::test]
    async fn test_list_tasks_alphabetical_order() {
        let pool = setup_test_db().await;
        let user_id = create_test_user(&pool).await;
        let household_id = create_test_household(&pool, &user_id).await;

        // Create tasks with titles in non-alphabetical order
        for title in ["Zebra Task", "Apple Task", "Mango Task"] {
            let request = CreateTaskRequest {
                title: title.to_string(),
                description: None,
                recurrence_type: RecurrenceType::Daily,
                recurrence_value: None,
                assigned_user_id: None,
                target_count: Some(1),
                time_period: None,
                allow_exceed_target: None,
                requires_review: None,
                points_reward: None,
                points_penalty: None,
                due_time: None,
                habit_type: None,
                category_id: None,
            };
            create_task(&pool, &household_id, &request).await.unwrap();
        }

        let tasks = list_tasks(&pool, &household_id).await.unwrap();

        assert_eq!(tasks.len(), 3);
        assert_eq!(tasks[0].title, "Apple Task");
        assert_eq!(tasks[1].title, "Mango Task");
        assert_eq!(tasks[2].title, "Zebra Task");
    }

    #[tokio::test]
    async fn test_list_tasks_case_insensitive_order() {
        let pool = setup_test_db().await;
        let user_id = create_test_user(&pool).await;
        let household_id = create_test_household(&pool, &user_id).await;

        // Create tasks with mixed case titles
        for title in ["banana Task", "Apple Task", "CHERRY Task"] {
            let request = CreateTaskRequest {
                title: title.to_string(),
                description: None,
                recurrence_type: RecurrenceType::Daily,
                recurrence_value: None,
                assigned_user_id: None,
                target_count: Some(1),
                time_period: None,
                allow_exceed_target: None,
                requires_review: None,
                points_reward: None,
                points_penalty: None,
                due_time: None,
                habit_type: None,
                category_id: None,
            };
            create_task(&pool, &household_id, &request).await.unwrap();
        }

        let tasks = list_tasks(&pool, &household_id).await.unwrap();

        assert_eq!(tasks.len(), 3);
        assert_eq!(tasks[0].title, "Apple Task");
        assert_eq!(tasks[1].title, "banana Task");
        assert_eq!(tasks[2].title, "CHERRY Task");
    }

    #[tokio::test]
    async fn test_get_all_tasks_with_status_secondary_sort() {
        let pool = setup_test_db().await;
        let user_id = create_test_user(&pool).await;
        let household_id = create_test_household(&pool, &user_id).await;

        // Create multiple daily tasks (same due date) with different titles
        for title in ["Zebra Daily", "Apple Daily", "Mango Daily"] {
            let request = CreateTaskRequest {
                title: title.to_string(),
                description: None,
                recurrence_type: RecurrenceType::Daily,
                recurrence_value: None,
                assigned_user_id: None,
                target_count: Some(1),
                time_period: None,
                allow_exceed_target: None,
                requires_review: None,
                points_reward: None,
                points_penalty: None,
                due_time: None,
                habit_type: None,
                category_id: None,
            };
            create_task(&pool, &household_id, &request).await.unwrap();
        }

        let tasks = get_all_tasks_with_status(&pool, &household_id, &user_id).await.unwrap();

        // All tasks have the same due date (today for daily tasks)
        // Should be sorted alphabetically as secondary sort
        assert_eq!(tasks.len(), 3);
        assert_eq!(tasks[0].task.title, "Apple Daily");
        assert_eq!(tasks[1].task.title, "Mango Daily");
        assert_eq!(tasks[2].task.title, "Zebra Daily");
    }

    #[tokio::test]
    async fn test_archive_task() {
        let pool = setup_test_db().await;
        let user_id = create_test_user(&pool).await;
        let household_id = create_test_household(&pool, &user_id).await;

        let request = CreateTaskRequest {
            title: "Task to Archive".to_string(),
            description: None,
            recurrence_type: RecurrenceType::Daily,
            recurrence_value: None,
            assigned_user_id: None,
            target_count: Some(1),
            time_period: None,
            allow_exceed_target: None,
            requires_review: None,
            points_reward: None,
            points_penalty: None,
            due_time: None,
            habit_type: None,
            category_id: None,
        };
        let task = create_task(&pool, &household_id, &request).await.unwrap();
        assert!(!task.archived);

        let archived_task = archive_task(&pool, &task.id).await.unwrap();
        assert!(archived_task.archived);
        assert_eq!(archived_task.id, task.id);
    }

    #[tokio::test]
    async fn test_unarchive_task() {
        let pool = setup_test_db().await;
        let user_id = create_test_user(&pool).await;
        let household_id = create_test_household(&pool, &user_id).await;

        let request = CreateTaskRequest {
            title: "Task to Unarchive".to_string(),
            description: None,
            recurrence_type: RecurrenceType::Daily,
            recurrence_value: None,
            assigned_user_id: None,
            target_count: Some(1),
            time_period: None,
            allow_exceed_target: None,
            requires_review: None,
            points_reward: None,
            points_penalty: None,
            due_time: None,
            habit_type: None,
            category_id: None,
        };
        let task = create_task(&pool, &household_id, &request).await.unwrap();

        // Archive the task first
        let archived_task = archive_task(&pool, &task.id).await.unwrap();
        assert!(archived_task.archived);

        // Unarchive the task
        let unarchived_task = unarchive_task(&pool, &task.id).await.unwrap();
        assert!(!unarchived_task.archived);
        assert_eq!(unarchived_task.id, task.id);
    }

    #[tokio::test]
    async fn test_list_archived_tasks() {
        let pool = setup_test_db().await;
        let user_id = create_test_user(&pool).await;
        let household_id = create_test_household(&pool, &user_id).await;

        // Create two tasks
        for title in ["Active Task", "Archived Task"] {
            let request = CreateTaskRequest {
                title: title.to_string(),
                description: None,
                recurrence_type: RecurrenceType::Daily,
                recurrence_value: None,
                assigned_user_id: None,
                target_count: Some(1),
                time_period: None,
                allow_exceed_target: None,
                requires_review: None,
                points_reward: None,
                points_penalty: None,
                due_time: None,
                habit_type: None,
                category_id: None,
            };
            create_task(&pool, &household_id, &request).await.unwrap();
        }

        let tasks = list_tasks(&pool, &household_id).await.unwrap();
        assert_eq!(tasks.len(), 2);

        // Archive one task
        let task_to_archive = tasks.iter().find(|t| t.title == "Archived Task").unwrap();
        archive_task(&pool, &task_to_archive.id).await.unwrap();

        // Verify archived tasks list
        let archived = list_archived_tasks(&pool, &household_id).await.unwrap();
        assert_eq!(archived.len(), 1);
        assert_eq!(archived[0].title, "Archived Task");
    }

    #[tokio::test]
    async fn test_list_tasks_excludes_archived() {
        let pool = setup_test_db().await;
        let user_id = create_test_user(&pool).await;
        let household_id = create_test_household(&pool, &user_id).await;

        // Create two tasks
        for title in ["Active Task", "Archived Task"] {
            let request = CreateTaskRequest {
                title: title.to_string(),
                description: None,
                recurrence_type: RecurrenceType::Daily,
                recurrence_value: None,
                assigned_user_id: None,
                target_count: Some(1),
                time_period: None,
                allow_exceed_target: None,
                requires_review: None,
                points_reward: None,
                points_penalty: None,
                due_time: None,
                habit_type: None,
                category_id: None,
            };
            create_task(&pool, &household_id, &request).await.unwrap();
        }

        let tasks = list_tasks(&pool, &household_id).await.unwrap();
        assert_eq!(tasks.len(), 2);

        // Archive one task
        let task_to_archive = tasks.iter().find(|t| t.title == "Archived Task").unwrap();
        archive_task(&pool, &task_to_archive.id).await.unwrap();

        // Verify list_tasks excludes archived
        let active_tasks = list_tasks(&pool, &household_id).await.unwrap();
        assert_eq!(active_tasks.len(), 1);
        assert_eq!(active_tasks[0].title, "Active Task");
    }

    #[tokio::test]
    async fn test_get_all_tasks_across_households() {
        let pool = setup_test_db().await;
        let user_id = create_test_user(&pool).await;
        let household1_id = create_test_household(&pool, &user_id).await;

        // Create second household
        let household2_id = Uuid::new_v4();
        sqlx::query(
            "INSERT INTO households (id, name, owner_id) VALUES (?, ?, ?)",
        )
        .bind(household2_id.to_string())
        .bind("Second Household")
        .bind(user_id.to_string())
        .execute(&pool)
        .await
        .unwrap();

        // Add membership for second household
        sqlx::query(
            "INSERT INTO memberships (id, household_id, user_id, role) VALUES (?, ?, ?, ?)",
        )
        .bind(Uuid::new_v4().to_string())
        .bind(household2_id.to_string())
        .bind(user_id.to_string())
        .bind("owner")
        .execute(&pool)
        .await
        .unwrap();

        // Also add to household_memberships
        sqlx::query(
            "INSERT INTO household_memberships (id, household_id, user_id, role) VALUES (?, ?, ?, ?)",
        )
        .bind(Uuid::new_v4().to_string())
        .bind(household2_id.to_string())
        .bind(user_id.to_string())
        .bind("owner")
        .execute(&pool)
        .await
        .unwrap();

        // Create task in household 1
        let request1 = CreateTaskRequest {
            title: "Task in Household 1".to_string(),
            description: None,
            recurrence_type: RecurrenceType::Daily,
            recurrence_value: None,
            assigned_user_id: None,
            target_count: Some(1),
            time_period: None,
            allow_exceed_target: None,
            requires_review: None,
            points_reward: None,
            points_penalty: None,
            due_time: None,
            habit_type: None,
            category_id: None,
        };
        create_task(&pool, &household1_id, &request1).await.unwrap();

        // Create task in household 2
        let request2 = CreateTaskRequest {
            title: "Task in Household 2".to_string(),
            description: None,
            recurrence_type: RecurrenceType::Daily,
            recurrence_value: None,
            assigned_user_id: None,
            target_count: Some(1),
            time_period: None,
            allow_exceed_target: None,
            requires_review: None,
            points_reward: None,
            points_penalty: None,
            due_time: None,
            habit_type: None,
            category_id: None,
        };
        create_task(&pool, &household2_id, &request2).await.unwrap();

        // Get all tasks across households
        let all_tasks = get_all_tasks_across_households(&pool, &user_id).await.unwrap();

        // Should have 2 tasks (one from each household)
        assert_eq!(all_tasks.len(), 2);

        // Verify tasks are from both households
        let household_ids: Vec<_> = all_tasks.iter().map(|(_, h_id)| *h_id).collect();
        assert!(household_ids.contains(&household1_id));
        assert!(household_ids.contains(&household2_id));
    }

    #[tokio::test]
    async fn test_get_all_tasks_across_households_excludes_archived() {
        let pool = setup_test_db().await;
        let user_id = create_test_user(&pool).await;
        let household_id = create_test_household(&pool, &user_id).await;

        // Create two tasks
        let request1 = CreateTaskRequest {
            title: "Active Task".to_string(),
            description: None,
            recurrence_type: RecurrenceType::Daily,
            recurrence_value: None,
            assigned_user_id: None,
            target_count: Some(1),
            time_period: None,
            allow_exceed_target: None,
            requires_review: None,
            points_reward: None,
            points_penalty: None,
            due_time: None,
            habit_type: None,
            category_id: None,
        };
        let task1 = create_task(&pool, &household_id, &request1).await.unwrap();

        let request2 = CreateTaskRequest {
            title: "Archived Task".to_string(),
            description: None,
            recurrence_type: RecurrenceType::Daily,
            recurrence_value: None,
            assigned_user_id: None,
            target_count: Some(1),
            time_period: None,
            allow_exceed_target: None,
            requires_review: None,
            points_reward: None,
            points_penalty: None,
            due_time: None,
            habit_type: None,
            category_id: None,
        };
        let task2 = create_task(&pool, &household_id, &request2).await.unwrap();

        // Archive the second task
        archive_task(&pool, &task2.id).await.unwrap();

        // Get all tasks across households
        let all_tasks = get_all_tasks_across_households(&pool, &user_id).await.unwrap();

        // Should only have 1 task (the active one)
        assert_eq!(all_tasks.len(), 1);
        assert_eq!(all_tasks[0].0.task.id, task1.id);
        assert_eq!(all_tasks[0].0.task.title, "Active Task");
    }

    #[tokio::test]
    async fn test_get_task_with_details_basic() {
        let pool = setup_test_db().await;
        let user_id = create_test_user(&pool).await;
        let household_id = create_test_household(&pool, &user_id).await;

        // Create a task
        let request = CreateTaskRequest {
            title: "Detail Test Task".to_string(),
            description: Some("Test description".to_string()),
            recurrence_type: RecurrenceType::Daily,
            recurrence_value: None,
            assigned_user_id: None,
            target_count: Some(1),
            time_period: None,
            allow_exceed_target: None,
            requires_review: None,
            points_reward: Some(10),
            points_penalty: Some(5),
            due_time: None,
            habit_type: None,
            category_id: None,
        };
        let task = create_task(&pool, &household_id, &request).await.unwrap();

        // Get task details
        let details = get_task_with_details(&pool, &task.id, &user_id)
            .await
            .unwrap()
            .expect("Task should exist");

        // Verify basic task info
        assert_eq!(details.task.id, task.id);
        assert_eq!(details.task.title, "Detail Test Task");
        assert_eq!(details.task.description, "Test description");
        assert_eq!(details.task.points_reward, Some(10));
        assert_eq!(details.task.points_penalty, Some(5));

        // Verify statistics (no completions yet)
        assert_eq!(details.statistics.current_streak, 0);
        assert_eq!(details.statistics.best_streak, 0);
        assert_eq!(details.statistics.total_completions, 0);
        assert!(details.statistics.last_completed.is_none());

        // No assigned user
        assert!(details.assigned_user.is_none());

        // No linked rewards/punishments
        assert!(details.linked_rewards.is_empty());
        assert!(details.linked_punishments.is_empty());
    }

    #[tokio::test]
    async fn test_get_task_with_details_with_completions() {
        let pool = setup_test_db().await;
        let user_id = create_test_user(&pool).await;
        let household_id = create_test_household(&pool, &user_id).await;

        // Create a task
        let request = CreateTaskRequest {
            title: "Completion Test Task".to_string(),
            description: None,
            recurrence_type: RecurrenceType::Daily,
            recurrence_value: None,
            assigned_user_id: None,
            target_count: Some(1),
            time_period: None,
            allow_exceed_target: Some(true),
            requires_review: None,
            points_reward: None,
            points_penalty: None,
            due_time: None,
            habit_type: None,
            category_id: None,
        };
        let task = create_task(&pool, &household_id, &request).await.unwrap();

        // Complete the task
        complete_task(&pool, &task.id, &user_id, &household_id).await.unwrap();

        // Get task details
        let details = get_task_with_details(&pool, &task.id, &user_id)
            .await
            .unwrap()
            .expect("Task should exist");

        // Verify statistics
        assert_eq!(details.statistics.total_completions, 1);
        assert!(details.statistics.last_completed.is_some());
        assert!(details.statistics.current_streak >= 1);
    }

    #[tokio::test]
    async fn test_get_task_with_details_not_found() {
        let pool = setup_test_db().await;
        let user_id = create_test_user(&pool).await;

        let random_id = Uuid::new_v4();
        let details = get_task_with_details(&pool, &random_id, &user_id).await.unwrap();

        assert!(details.is_none());
    }

    #[tokio::test]
    async fn test_get_task_with_details_assigned_user() {
        let pool = setup_test_db().await;
        let user_id = create_test_user(&pool).await;
        let household_id = create_test_household(&pool, &user_id).await;

        // Create a task assigned to the user
        let request = CreateTaskRequest {
            title: "Assigned Task".to_string(),
            description: None,
            recurrence_type: RecurrenceType::Daily,
            recurrence_value: None,
            assigned_user_id: Some(user_id),
            target_count: Some(1),
            time_period: None,
            allow_exceed_target: None,
            requires_review: None,
            points_reward: None,
            points_penalty: None,
            due_time: None,
            habit_type: None,
            category_id: None,
        };
        let task = create_task(&pool, &household_id, &request).await.unwrap();

        // Get task details
        let details = get_task_with_details(&pool, &task.id, &user_id)
            .await
            .unwrap()
            .expect("Task should exist");

        // Verify assigned user is returned
        assert!(details.assigned_user.is_some());
        let assigned = details.assigned_user.unwrap();
        assert_eq!(assigned.id, user_id);
        assert_eq!(assigned.username, "testuser");
    }

    async fn create_second_test_user(pool: &SqlitePool) -> Uuid {
        let user_id = Uuid::new_v4();
        sqlx::query(
            "INSERT INTO users (id, username, email, password_hash) VALUES (?, ?, ?, ?)",
        )
        .bind(user_id.to_string())
        .bind("testuser2")
        .bind("test2@example.com")
        .bind("hash")
        .execute(pool)
        .await
        .unwrap();
        user_id
    }

    async fn add_user_to_household(pool: &SqlitePool, household_id: &Uuid, user_id: &Uuid) {
        // Add membership
        sqlx::query(
            "INSERT INTO memberships (id, household_id, user_id, role) VALUES (?, ?, ?, ?)",
        )
        .bind(Uuid::new_v4().to_string())
        .bind(household_id.to_string())
        .bind(user_id.to_string())
        .bind("member")
        .execute(pool)
        .await
        .unwrap();

        // Also add to household_memberships for household service
        sqlx::query(
            "INSERT INTO household_memberships (id, household_id, user_id, role) VALUES (?, ?, ?, ?)",
        )
        .bind(Uuid::new_v4().to_string())
        .bind(household_id.to_string())
        .bind(user_id.to_string())
        .bind("member")
        .execute(pool)
        .await
        .unwrap();
    }

    #[tokio::test]
    async fn test_complete_task_assigned_user_success() {
        let pool = setup_test_db().await;
        let user_id = create_test_user(&pool).await;
        let household_id = create_test_household(&pool, &user_id).await;

        // Create task assigned to the user
        let request = CreateTaskRequest {
            title: "Assigned Task".to_string(),
            description: None,
            recurrence_type: RecurrenceType::Daily,
            recurrence_value: None,
            assigned_user_id: Some(user_id),
            target_count: Some(1),
            time_period: None,
            allow_exceed_target: Some(true),
            requires_review: None,
            points_reward: None,
            points_penalty: None,
            due_time: None,
            habit_type: None,
            category_id: None,
        };
        let task = create_task(&pool, &household_id, &request).await.unwrap();

        // Assigned user should be able to complete the task
        let result = complete_task(&pool, &task.id, &user_id, &household_id).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_complete_task_unassigned_task_any_user() {
        let pool = setup_test_db().await;
        let user1 = create_test_user(&pool).await;
        let user2 = create_second_test_user(&pool).await;
        let household_id = create_test_household(&pool, &user1).await;
        add_user_to_household(&pool, &household_id, &user2).await;

        // Create task without assignment
        let request = CreateTaskRequest {
            title: "Unassigned Task".to_string(),
            description: None,
            recurrence_type: RecurrenceType::Daily,
            recurrence_value: None,
            assigned_user_id: None, // No assignment
            target_count: Some(1),
            time_period: None,
            allow_exceed_target: Some(true),
            requires_review: None,
            points_reward: None,
            points_penalty: None,
            due_time: None,
            habit_type: None,
            category_id: None,
        };
        let task = create_task(&pool, &household_id, &request).await.unwrap();

        // Both users should be able to complete the task
        let result1 = complete_task(&pool, &task.id, &user1, &household_id).await;
        assert!(result1.is_ok());

        let result2 = complete_task(&pool, &task.id, &user2, &household_id).await;
        assert!(result2.is_ok());
    }

    #[tokio::test]
    async fn test_complete_task_wrong_user_forbidden() {
        let pool = setup_test_db().await;
        let user1 = create_test_user(&pool).await;
        let user2 = create_second_test_user(&pool).await;
        let household_id = create_test_household(&pool, &user1).await;
        add_user_to_household(&pool, &household_id, &user2).await;

        // Create task assigned to user1
        let request = CreateTaskRequest {
            title: "User1's Task".to_string(),
            description: None,
            recurrence_type: RecurrenceType::Daily,
            recurrence_value: None,
            assigned_user_id: Some(user1),
            target_count: Some(1),
            time_period: None,
            allow_exceed_target: Some(true),
            requires_review: None,
            points_reward: None,
            points_penalty: None,
            due_time: None,
            habit_type: None,
            category_id: None,
        };
        let task = create_task(&pool, &household_id, &request).await.unwrap();

        // User2 should NOT be able to complete user1's task
        let result = complete_task(&pool, &task.id, &user2, &household_id).await;
        assert!(matches!(result, Err(TaskError::NotAssigned)));
    }

    #[tokio::test]
    async fn test_uncomplete_task_wrong_user_forbidden() {
        let pool = setup_test_db().await;
        let user1 = create_test_user(&pool).await;
        let user2 = create_second_test_user(&pool).await;
        let household_id = create_test_household(&pool, &user1).await;
        add_user_to_household(&pool, &household_id, &user2).await;

        // Create task assigned to user1
        let request = CreateTaskRequest {
            title: "User1's Task".to_string(),
            description: None,
            recurrence_type: RecurrenceType::Daily,
            recurrence_value: None,
            assigned_user_id: Some(user1),
            target_count: Some(1),
            time_period: None,
            allow_exceed_target: Some(true),
            requires_review: None,
            points_reward: None,
            points_penalty: None,
            due_time: None,
            habit_type: None,
            category_id: None,
        };
        let task = create_task(&pool, &household_id, &request).await.unwrap();

        // User1 completes the task first
        complete_task(&pool, &task.id, &user1, &household_id).await.unwrap();

        // User2 should NOT be able to uncomplete user1's task
        let result = uncomplete_task(&pool, &task.id, &user2).await;
        assert!(matches!(result, Err(TaskError::NotAssigned)));
    }

    #[tokio::test]
    async fn test_task_with_status_is_user_assigned_true() {
        let pool = setup_test_db().await;
        let user_id = create_test_user(&pool).await;
        let household_id = create_test_household(&pool, &user_id).await;

        // Create task assigned to the user
        let request = CreateTaskRequest {
            title: "My Assigned Task".to_string(),
            description: None,
            recurrence_type: RecurrenceType::Daily,
            recurrence_value: None,
            assigned_user_id: Some(user_id),
            target_count: Some(1),
            time_period: None,
            allow_exceed_target: None,
            requires_review: None,
            points_reward: None,
            points_penalty: None,
            due_time: None,
            habit_type: None,
            category_id: None,
        };
        let task = create_task(&pool, &household_id, &request).await.unwrap();

        // Get task with status for the assigned user
        let status = get_task_with_status(&pool, &task.id, &user_id)
            .await
            .unwrap()
            .expect("Task should exist");

        assert!(status.is_user_assigned);
    }

    #[tokio::test]
    async fn test_task_with_status_is_user_assigned_false() {
        let pool = setup_test_db().await;
        let user1 = create_test_user(&pool).await;
        let user2 = create_second_test_user(&pool).await;
        let household_id = create_test_household(&pool, &user1).await;
        add_user_to_household(&pool, &household_id, &user2).await;

        // Create task assigned to user1
        let request = CreateTaskRequest {
            title: "User1's Task".to_string(),
            description: None,
            recurrence_type: RecurrenceType::Daily,
            recurrence_value: None,
            assigned_user_id: Some(user1),
            target_count: Some(1),
            time_period: None,
            allow_exceed_target: None,
            requires_review: None,
            points_reward: None,
            points_penalty: None,
            due_time: None,
            habit_type: None,
            category_id: None,
        };
        let task = create_task(&pool, &household_id, &request).await.unwrap();

        // Get task with status for user2 (not assigned)
        let status = get_task_with_status(&pool, &task.id, &user2)
            .await
            .unwrap()
            .expect("Task should exist");

        assert!(!status.is_user_assigned);
    }

    #[tokio::test]
    async fn test_task_with_status_is_user_assigned_no_assignment() {
        let pool = setup_test_db().await;
        let user1 = create_test_user(&pool).await;
        let user2 = create_second_test_user(&pool).await;
        let household_id = create_test_household(&pool, &user1).await;
        add_user_to_household(&pool, &household_id, &user2).await;

        // Create task with no assignment
        let request = CreateTaskRequest {
            title: "Unassigned Task".to_string(),
            description: None,
            recurrence_type: RecurrenceType::Daily,
            recurrence_value: None,
            assigned_user_id: None, // No assignment
            target_count: Some(1),
            time_period: None,
            allow_exceed_target: None,
            requires_review: None,
            points_reward: None,
            points_penalty: None,
            due_time: None,
            habit_type: None,
            category_id: None,
        };
        let task = create_task(&pool, &household_id, &request).await.unwrap();

        // Both users should have is_user_assigned = true
        let status1 = get_task_with_status(&pool, &task.id, &user1)
            .await
            .unwrap()
            .expect("Task should exist");
        assert!(status1.is_user_assigned);

        let status2 = get_task_with_status(&pool, &task.id, &user2)
            .await
            .unwrap()
            .expect("Task should exist");
        assert!(status2.is_user_assigned);
    }
}
