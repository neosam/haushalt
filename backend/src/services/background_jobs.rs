use chrono::{Duration, NaiveDate, Utc};
use sqlx::SqlitePool;
use std::sync::Arc;
use thiserror::Error;
use tokio::time;
use uuid::Uuid;

use crate::models::{MembershipRow, TaskRow};
use crate::services::{points as points_service, scheduler, task_consequences};

#[derive(Debug, Error)]
pub enum BackgroundJobError {
    #[error("Database error: {0}")]
    DatabaseError(#[from] sqlx::Error),
    #[error("Points error: {0}")]
    PointsError(#[from] points_service::PointsError),
    #[error("Task consequence error: {0}")]
    TaskConsequenceError(#[from] task_consequences::TaskConsequenceError),
}

/// Report from processing missed tasks
#[derive(Debug, Clone)]
pub struct MissedTaskReport {
    pub processed_at: chrono::DateTime<Utc>,
    pub tasks_checked: i64,
    pub missed_tasks: i64,
    pub punishments_assigned: i64,
    pub points_deducted: i64,
}

/// Configuration for the background job scheduler
#[derive(Debug, Clone)]
pub struct JobConfig {
    /// Hour of day to run the missed task check (0-23)
    pub check_hour: u32,
    /// Minute of hour to run the check (0-59)
    pub check_minute: u32,
}

impl Default for JobConfig {
    fn default() -> Self {
        Self {
            check_hour: 2,   // Run at 2:00 AM
            check_minute: 0,
        }
    }
}

/// Start the background job scheduler
/// This runs in a loop and checks for missed tasks daily
pub async fn start_scheduler(pool: Arc<SqlitePool>, config: JobConfig) {
    log::info!(
        "Background job scheduler started. Missed task check scheduled for {:02}:{:02}",
        config.check_hour,
        config.check_minute
    );

    loop {
        // Calculate time until next scheduled run
        let now = Utc::now();
        let today_check = now
            .date_naive()
            .and_hms_opt(config.check_hour, config.check_minute, 0)
            .unwrap();

        let next_check = if now.naive_utc() < today_check {
            today_check
        } else {
            // Schedule for tomorrow
            today_check + Duration::days(1)
        };

        let sleep_duration = (next_check - now.naive_utc())
            .to_std()
            .unwrap_or(std::time::Duration::from_secs(3600));

        log::debug!(
            "Next missed task check scheduled in {} seconds",
            sleep_duration.as_secs()
        );

        time::sleep(sleep_duration).await;

        // Process missed tasks
        match process_missed_tasks(&pool).await {
            Ok(report) => {
                log::info!(
                    "Missed task processing complete: checked {} tasks, found {} missed, assigned {} punishments, deducted {} points",
                    report.tasks_checked,
                    report.missed_tasks,
                    report.punishments_assigned,
                    report.points_deducted
                );
            }
            Err(e) => {
                log::error!("Error processing missed tasks: {}", e);
            }
        }
    }
}

/// Process all missed tasks from yesterday
/// This function:
/// 1. Gets all tasks from all households
/// 2. Checks if each task was due yesterday
/// 3. For missed tasks, deducts points and assigns punishments
pub async fn process_missed_tasks(pool: &SqlitePool) -> Result<MissedTaskReport, BackgroundJobError> {
    let yesterday = Utc::now().date_naive() - Duration::days(1);
    let now = Utc::now();

    let mut tasks_checked: i64 = 0;
    let mut missed_tasks: i64 = 0;
    let mut punishments_assigned: i64 = 0;
    let mut points_deducted: i64 = 0;

    // Get all tasks
    let tasks: Vec<TaskRow> = sqlx::query_as("SELECT * FROM tasks")
        .fetch_all(pool)
        .await?;

    for task_row in tasks {
        let task = task_row.to_shared();

        // Skip free-form and one-time tasks (they can't be "missed")
        if task.recurrence_type == shared::RecurrenceType::OneTime {
            continue;
        }

        // Check if task was due yesterday
        if !scheduler::is_task_due_on_date(&task, yesterday) {
            continue;
        }

        tasks_checked += 1;

        // Check if task was completed yesterday
        let completion_count = sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*) FROM task_completions WHERE task_id = ? AND due_date = ?",
        )
        .bind(task.id.to_string())
        .bind(yesterday)
        .fetch_one(pool)
        .await?;

        if completion_count > 0 {
            // Task was completed, skip
            continue;
        }

        // Task was missed
        missed_tasks += 1;

        // Determine who to penalize
        let users_to_penalize: Vec<Uuid> = if let Some(assigned_user_id) = task.assigned_user_id {
            // Penalize only the assigned user
            vec![assigned_user_id]
        } else {
            // Penalize all household members
            get_household_member_ids(pool, &task.household_id).await?
        };

        // Check if the user had a streak that was broken
        // A streak is considered broken if they had at least one previous completion
        let had_previous_completion = sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*) FROM task_completions WHERE task_id = ?",
        )
        .bind(task.id.to_string())
        .fetch_one(pool)
        .await?
            > 0;

        for user_id in users_to_penalize {
            // Deduct points
            let points = points_service::deduct_missed_task_points(
                pool,
                &task.household_id,
                &user_id,
                &task.id,
                had_previous_completion,
            )
            .await?;
            points_deducted += points.abs();

            // Assign punishments linked to this task
            let assigned = task_consequences::assign_missed_task_punishments(
                pool,
                &task.id,
                &user_id,
                &task.household_id,
            )
            .await?;

            punishments_assigned += assigned.len() as i64;
        }
    }

    Ok(MissedTaskReport {
        processed_at: now,
        tasks_checked,
        missed_tasks,
        punishments_assigned,
        points_deducted,
    })
}

/// Process missed tasks for a specific date (useful for testing or manual runs)
pub async fn process_missed_tasks_for_date(
    pool: &SqlitePool,
    date: NaiveDate,
) -> Result<MissedTaskReport, BackgroundJobError> {
    let now = Utc::now();

    let mut tasks_checked: i64 = 0;
    let mut missed_tasks: i64 = 0;
    let mut punishments_assigned: i64 = 0;
    let mut points_deducted: i64 = 0;

    // Get all tasks
    let tasks: Vec<TaskRow> = sqlx::query_as("SELECT * FROM tasks")
        .fetch_all(pool)
        .await?;

    for task_row in tasks {
        let task = task_row.to_shared();

        // Skip free-form and one-time tasks (they can't be "missed")
        if task.recurrence_type == shared::RecurrenceType::OneTime {
            continue;
        }

        // Check if task was due on the given date
        if !scheduler::is_task_due_on_date(&task, date) {
            continue;
        }

        tasks_checked += 1;

        // Check if task was completed on that date
        let completion_count = sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*) FROM task_completions WHERE task_id = ? AND due_date = ?",
        )
        .bind(task.id.to_string())
        .bind(date)
        .fetch_one(pool)
        .await?;

        if completion_count > 0 {
            continue;
        }

        // Task was missed
        missed_tasks += 1;

        let users_to_penalize: Vec<Uuid> = if let Some(assigned_user_id) = task.assigned_user_id {
            vec![assigned_user_id]
        } else {
            get_household_member_ids(pool, &task.household_id).await?
        };

        let had_previous_completion = sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*) FROM task_completions WHERE task_id = ?",
        )
        .bind(task.id.to_string())
        .fetch_one(pool)
        .await?
            > 0;

        for user_id in users_to_penalize {
            let points = points_service::deduct_missed_task_points(
                pool,
                &task.household_id,
                &user_id,
                &task.id,
                had_previous_completion,
            )
            .await?;
            points_deducted += points.abs();

            let assigned = task_consequences::assign_missed_task_punishments(
                pool,
                &task.id,
                &user_id,
                &task.household_id,
            )
            .await?;

            punishments_assigned += assigned.len() as i64;
        }
    }

    Ok(MissedTaskReport {
        processed_at: now,
        tasks_checked,
        missed_tasks,
        punishments_assigned,
        points_deducted,
    })
}

/// Get all member IDs for a household
async fn get_household_member_ids(
    pool: &SqlitePool,
    household_id: &Uuid,
) -> Result<Vec<Uuid>, BackgroundJobError> {
    let memberships: Vec<MembershipRow> = sqlx::query_as(
        "SELECT * FROM household_memberships WHERE household_id = ?",
    )
    .bind(household_id.to_string())
    .fetch_all(pool)
    .await?;

    Ok(memberships
        .into_iter()
        .filter_map(|m| Uuid::parse_str(&m.user_id).ok())
        .collect())
}

/// Get the owner ID for a household
async fn get_household_owner_id(
    pool: &SqlitePool,
    household_id: &Uuid,
) -> Result<Uuid, BackgroundJobError> {
    let owner_id: String = sqlx::query_scalar("SELECT owner_id FROM households WHERE id = ?")
        .bind(household_id.to_string())
        .fetch_one(pool)
        .await?;

    Uuid::parse_str(&owner_id).map_err(|_| {
        BackgroundJobError::DatabaseError(sqlx::Error::RowNotFound)
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_job_config_default() {
        let config = JobConfig::default();
        assert_eq!(config.check_hour, 2);
        assert_eq!(config.check_minute, 0);
    }

    #[test]
    fn test_background_job_error_display() {
        let err = BackgroundJobError::DatabaseError(sqlx::Error::RowNotFound);
        assert!(err.to_string().contains("Database error"));
    }
}
