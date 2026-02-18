use chrono::{Duration, Utc};
use sqlx::SqlitePool;
use std::sync::Arc;
use thiserror::Error;
use tokio::time;
use uuid::Uuid;

use crate::models::{MembershipRow, TaskRow};
use crate::services::{household_settings, points as points_service, scheduler, task_consequences};
use shared::HouseholdSettings;

#[derive(Debug, Error)]
pub enum BackgroundJobError {
    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),
    #[error("Points error: {0}")]
    Points(#[from] points_service::PointsError),
    #[error("Task consequence error: {0}")]
    TaskConsequence(#[from] task_consequences::TaskConsequenceError),
}

/// Report from processing missed tasks
#[derive(Debug, Clone)]
pub struct MissedTaskReport {
    pub tasks_checked: i64,
    pub missed_tasks: i64,
    pub punishments_assigned: i64,
    pub points_deducted: i64,
    /// For bad habits that were avoided
    pub rewards_assigned: i64,
    pub points_added: i64,
}

/// Configuration for the background job scheduler
#[derive(Debug, Clone)]
pub struct JobConfig {
    /// Interval in minutes between missed task checks
    /// Since we support different timezones and due times, we check more frequently
    pub check_interval_minutes: u32,
}

impl Default for JobConfig {
    fn default() -> Self {
        Self {
            check_interval_minutes: 1, // Run every minute
        }
    }
}

/// Start the background job scheduler
/// This runs in a loop and checks for missed tasks at the configured interval
pub async fn start_scheduler(pool: Arc<SqlitePool>, config: JobConfig) {
    log::info!(
        "Background job scheduler started. Missed task check every {} minutes",
        config.check_interval_minutes
    );

    let interval = std::time::Duration::from_secs((config.check_interval_minutes * 60) as u64);

    loop {
        time::sleep(interval).await;

        // Process missed tasks
        match process_missed_tasks(&pool).await {
            Ok(report) => {
                if report.missed_tasks > 0 {
                    log::info!(
                        "Missed task processing complete: checked {} tasks, found {} missed, assigned {} punishments/{} rewards, deducted {}/added {} points",
                        report.tasks_checked,
                        report.missed_tasks,
                        report.punishments_assigned,
                        report.rewards_assigned,
                        report.points_deducted,
                        report.points_added
                    );
                } else {
                    log::debug!(
                        "Missed task check complete: checked {} tasks, no missed tasks found",
                        report.tasks_checked
                    );
                }
            }
            Err(e) => {
                log::error!("Error processing missed tasks: {}", e);
            }
        }
    }
}

/// Process all missed tasks
/// This function:
/// 1. Gets all tasks from all households
/// 2. For each household, uses the household's timezone to determine "yesterday"
/// 3. Checks if each task was due yesterday (in the household's timezone) and is now overdue
/// 4. For missed tasks, deducts points and assigns punishments
pub async fn process_missed_tasks(pool: &SqlitePool) -> Result<MissedTaskReport, BackgroundJobError> {
    let now_utc = Utc::now();

    let mut tasks_checked: i64 = 0;
    let mut missed_tasks: i64 = 0;
    let mut punishments_assigned: i64 = 0;
    let mut points_deducted: i64 = 0;
    let mut rewards_assigned: i64 = 0;
    let mut points_added: i64 = 0;

    // Get all tasks
    let tasks: Vec<TaskRow> = sqlx::query_as("SELECT * FROM tasks")
        .fetch_all(pool)
        .await?;

    // Cache household settings to avoid repeated lookups
    let mut settings_cache: std::collections::HashMap<Uuid, HouseholdSettings> =
        std::collections::HashMap::new();

    for task_row in tasks {
        let task = task_row.to_shared();

        // Skip free-form and one-time tasks (they can't be "missed")
        if task.recurrence_type == shared::RecurrenceType::OneTime {
            continue;
        }

        // Skip paused tasks - no punishments while paused
        if task.paused {
            continue;
        }

        // Skip archived tasks - they are no longer active
        if task.archived {
            continue;
        }

        // Get household settings (cached)
        let settings = if let Some(s) = settings_cache.get(&task.household_id) {
            s.clone()
        } else {
            let s = household_settings::get_or_create_settings(pool, &task.household_id)
                .await
                .unwrap_or_default();
            settings_cache.insert(task.household_id, s.clone());
            s
        };

        let timezone = settings.timezone.clone();

        // Get "yesterday" in the household's timezone
        let tz = scheduler::parse_timezone(&timezone);
        let today_local = scheduler::today_in_timezone(tz);
        let yesterday_local = today_local - Duration::days(1);

        // Skip if household is on vacation - no punishments during vacation
        if household_settings::is_household_on_vacation(&settings, today_local) {
            continue;
        }

        // Check if task was due yesterday
        if !scheduler::is_task_due_on_date(&task, yesterday_local) {
            continue;
        }

        // Check if the task is now overdue (deadline has passed)
        if !scheduler::is_task_overdue(&task, yesterday_local, &timezone, now_utc) {
            continue;
        }

        tasks_checked += 1;

        // Check if task was completed yesterday (in local timezone)
        let completion_count = sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*) FROM task_completions WHERE task_id = ? AND due_date = ?",
        )
        .bind(task.id.to_string())
        .bind(yesterday_local)
        .fetch_one(pool)
        .await?;

        if completion_count > 0 {
            // Task was completed, skip
            continue;
        }

        // Check if we already processed this task for this due date
        let already_processed = sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*) FROM missed_task_penalties WHERE task_id = ? AND due_date = ?",
        )
        .bind(task.id.to_string())
        .bind(yesterday_local)
        .fetch_one(pool)
        .await?;

        if already_processed > 0 {
            // Already processed for this date, skip
            continue;
        }

        // Task was not completed in time
        missed_tasks += 1;

        // Determine who to apply consequences to
        let affected_users: Vec<Uuid> = if let Some(assigned_user_id) = task.assigned_user_id {
            vec![assigned_user_id]
        } else {
            get_household_member_ids(pool, &task.household_id).await?
        };

        // Check if the user had a streak that was broken (for good habits)
        let had_previous_completion = sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*) FROM task_completions WHERE task_id = ?",
        )
        .bind(task.id.to_string())
        .fetch_one(pool)
        .await?
            > 0;

        for user_id in affected_users {
            if task.habit_type.is_inverted() {
                // Bad habit not completed = REWARD (successfully avoided!)
                let points = points_service::award_bad_habit_avoided_points(
                    pool,
                    &task.household_id,
                    &user_id,
                    &task,
                )
                .await?;
                points_added += points;

                // Assign rewards linked to this task
                let assigned = task_consequences::assign_bad_habit_avoided_rewards(
                    pool,
                    &task.id,
                    &user_id,
                    &task.household_id,
                )
                .await?;

                rewards_assigned += assigned.len() as i64;
            } else {
                // Good habit not completed = PUNISHMENT (missed)
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

        // Record that we processed this task for this due date
        sqlx::query(
            "INSERT INTO missed_task_penalties (task_id, due_date) VALUES (?, ?)",
        )
        .bind(task.id.to_string())
        .bind(yesterday_local)
        .execute(pool)
        .await?;
    }

    Ok(MissedTaskReport {
        tasks_checked,
        missed_tasks,
        punishments_assigned,
        points_deducted,
        rewards_assigned,
        points_added,
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_job_config_default() {
        let config = JobConfig::default();
        assert_eq!(config.check_interval_minutes, 1);
    }

    #[test]
    fn test_background_job_error_display() {
        let err = BackgroundJobError::Database(sqlx::Error::RowNotFound);
        assert!(err.to_string().contains("Database error"));
    }
}
