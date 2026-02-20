use chrono::{Duration, Utc};
use sqlx::SqlitePool;
use std::sync::Arc;
use thiserror::Error;
use tokio::time;
use uuid::Uuid;

use crate::models::{MembershipRow, TaskRow};
use crate::services::{
    activity_logs, household_settings, period_results, points as points_service, scheduler,
    task_consequences, tasks as tasks_service,
};
use shared::{ActivityType, HouseholdSettings, PeriodStatus, RecurrenceType, RecurrenceValue};

#[derive(Debug, Error)]
pub enum BackgroundJobError {
    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),
    #[error("Points error: {0}")]
    Points(#[from] points_service::PointsError),
    #[error("Task consequence error: {0}")]
    TaskConsequence(#[from] task_consequences::TaskConsequenceError),
    #[error("Activity log error: {0}")]
    ActivityLog(#[from] activity_logs::ActivityLogError),
    #[error("Task error: {0}")]
    TaskError(#[from] tasks_service::TaskError),
    #[error("Period result error: {0}")]
    PeriodResult(#[from] period_results::PeriodResultError),
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

/// Report from auto-archiving tasks
#[derive(Debug, Clone)]
pub struct AutoArchiveReport {
    pub tasks_checked: u32,
    pub tasks_archived: u32,
}

/// Report from period finalization
#[derive(Debug, Clone)]
pub struct PeriodFinalizationReport {
    pub tasks_checked: u32,
    pub periods_completed: u32,
    pub periods_failed: u32,
    pub periods_skipped: u32,
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

        // Process auto-archive
        match process_auto_archive(&pool).await {
            Ok(report) => {
                if report.tasks_archived > 0 {
                    log::info!(
                        "Auto-archive complete: checked {} tasks, archived {}",
                        report.tasks_checked,
                        report.tasks_archived
                    );
                } else {
                    log::debug!(
                        "Auto-archive check complete: checked {} tasks, none eligible",
                        report.tasks_checked
                    );
                }
            }
            Err(e) => {
                log::error!("Error processing auto-archive: {}", e);
            }
        }

        // Process period finalization (create failed/skipped records for ended periods)
        match process_period_finalization(&pool).await {
            Ok(report) => {
                let total = report.periods_completed + report.periods_failed + report.periods_skipped;
                if total > 0 {
                    log::info!(
                        "Period finalization complete: checked {} tasks, finalized {} periods (completed: {}, failed: {}, skipped: {})",
                        report.tasks_checked,
                        total,
                        report.periods_completed,
                        report.periods_failed,
                        report.periods_skipped
                    );
                } else {
                    log::debug!(
                        "Period finalization check complete: checked {} tasks, no periods to finalize",
                        report.tasks_checked
                    );
                }
            }
            Err(e) => {
                log::error!("Error processing period finalization: {}", e);
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

/// Process auto-archiving of completed one-time and custom tasks
/// This function:
/// 1. Gets all non-archived tasks from all households
/// 2. For one-time tasks: archives if completed and grace period elapsed
/// 3. For custom tasks: archives if completed, last date passed, and grace period elapsed
/// 4. Logs activity for each auto-archived task
pub async fn process_auto_archive(pool: &SqlitePool) -> Result<AutoArchiveReport, BackgroundJobError> {
    let mut tasks_checked: u32 = 0;
    let mut tasks_archived: u32 = 0;

    // Get all non-archived tasks that are candidates for auto-archive (OneTime or Custom)
    // Exclude suggestions (only process regular or approved tasks)
    let tasks: Vec<TaskRow> = sqlx::query_as(
        "SELECT * FROM tasks WHERE archived = 0 AND (recurrence_type = 'onetime' OR recurrence_type = 'custom') AND (suggestion IS NULL OR suggestion = 'approved')",
    )
    .fetch_all(pool)
    .await?;

    // Cache household settings to avoid repeated lookups
    let mut settings_cache: std::collections::HashMap<Uuid, HouseholdSettings> =
        std::collections::HashMap::new();

    for task_row in tasks {
        let task = task_row.to_shared();

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

        // Check if auto-archive is enabled for this household
        let auto_archive_days = match settings.auto_archive_days {
            Some(days) if days > 0 => days,
            _ => continue, // Auto-archive disabled
        };

        // Get "today" in the household's timezone
        let tz = scheduler::parse_timezone(&settings.timezone);
        let today_local = scheduler::today_in_timezone(tz);

        tasks_checked += 1;

        // Check if task is eligible for auto-archive
        if is_eligible_for_auto_archive(pool, &task, auto_archive_days, today_local).await? {
            // Archive the task
            tasks_service::archive_task(pool, &task.id).await?;

            // Log activity (use Uuid::nil() for system actor)
            let _ = activity_logs::log_activity(
                pool,
                &task.household_id,
                &Uuid::nil(), // System actor
                None,
                ActivityType::TaskAutoArchived,
                Some("task"),
                Some(&task.id),
                Some(&task.title),
            )
            .await;

            tasks_archived += 1;
        }
    }

    Ok(AutoArchiveReport {
        tasks_checked,
        tasks_archived,
    })
}

/// Check if a task is eligible for auto-archive
async fn is_eligible_for_auto_archive(
    pool: &SqlitePool,
    task: &shared::Task,
    grace_days: i32,
    today: chrono::NaiveDate,
) -> Result<bool, BackgroundJobError> {
    // Must have at least one completion
    let completion_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM task_completions WHERE task_id = ?",
    )
    .bind(task.id.to_string())
    .fetch_one(pool)
    .await?;

    if completion_count == 0 {
        return Ok(false); // Never archive uncompleted tasks
    }

    // Get the last completion date
    let last_completion: Option<chrono::NaiveDate> = sqlx::query_scalar(
        "SELECT MAX(due_date) FROM task_completions WHERE task_id = ?",
    )
    .bind(task.id.to_string())
    .fetch_one(pool)
    .await?;

    let last_completion = match last_completion {
        Some(date) => date,
        None => return Ok(false),
    };

    let grace_period = Duration::days(i64::from(grace_days));

    match task.recurrence_type {
        RecurrenceType::OneTime => {
            // Archive if completed + grace period elapsed
            Ok(last_completion + grace_period <= today)
        }
        RecurrenceType::Custom => {
            // Archive if last custom date passed AND completed AND grace period elapsed
            if let Some(RecurrenceValue::CustomDates(dates)) = &task.recurrence_value {
                let last_custom_date = dates.iter().max();
                if let Some(last_date) = last_custom_date {
                    // Use the later of: last completion date or last custom date
                    let archive_after = std::cmp::max(*last_date, last_completion);
                    Ok(archive_after + grace_period <= today)
                } else {
                    Ok(false)
                }
            } else {
                Ok(false)
            }
        }
        _ => Ok(false), // Other recurrence types never auto-archive
    }
}

/// Process period finalization for all tasks
/// This function:
/// 1. Gets all scheduled tasks from all households (not OneTime)
/// 2. For each household, uses the household's timezone to determine "yesterday"
/// 3. For each task due yesterday without a period result, creates one
/// 4. Status is: completed (if target met), failed (if not met), skipped (if paused/vacation)
pub async fn process_period_finalization(pool: &SqlitePool) -> Result<PeriodFinalizationReport, BackgroundJobError> {
    let mut tasks_checked: u32 = 0;
    let mut periods_completed: u32 = 0;
    let mut periods_failed: u32 = 0;
    let mut periods_skipped: u32 = 0;

    // Get all scheduled tasks (not OneTime, not archived, not pending suggestions)
    let tasks: Vec<TaskRow> = sqlx::query_as(
        "SELECT * FROM tasks WHERE recurrence_type != 'onetime' AND archived = 0 AND (suggestion IS NULL OR suggestion = 'approved')",
    )
    .fetch_all(pool)
    .await?;

    // Cache household settings to avoid repeated lookups
    let mut settings_cache: std::collections::HashMap<Uuid, HouseholdSettings> =
        std::collections::HashMap::new();

    for task_row in tasks {
        let task = task_row.to_shared();

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

        // Check if task was due yesterday
        if !scheduler::is_task_due_on_date(&task, yesterday_local) {
            continue;
        }

        // Get the period bounds for yesterday
        let (period_start, period_end) = scheduler::get_period_bounds(&task, yesterday_local);

        // Check if period is already finalized
        if period_results::is_period_finalized(pool, &task.id, period_start)
            .await
            .unwrap_or(false)
        {
            continue;
        }

        tasks_checked += 1;

        // Count completions for this period
        let completions_count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM task_completions WHERE task_id = ? AND due_date >= ? AND due_date <= ?",
        )
        .bind(task.id.to_string())
        .bind(period_start)
        .bind(period_end)
        .fetch_one(pool)
        .await?;

        // Determine status
        let status = if task.paused || household_settings::is_household_on_vacation(&settings, yesterday_local) {
            // Task was paused or household on vacation - skip
            periods_skipped += 1;
            PeriodStatus::Skipped
        } else if completions_count >= task.target_count as i64 {
            // Target was met
            periods_completed += 1;
            PeriodStatus::Completed
        } else {
            // Target was not met
            periods_failed += 1;
            PeriodStatus::Failed
        };

        // Create the period result
        let _ = period_results::finalize_period(
            pool,
            &task.id,
            period_start,
            period_end,
            status,
            completions_count as i32,
            task.target_count,
            "system",
            None,
        )
        .await;
    }

    Ok(PeriodFinalizationReport {
        tasks_checked,
        periods_completed,
        periods_failed,
        periods_skipped,
    })
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

    #[test]
    fn test_auto_archive_report_creation() {
        let report = AutoArchiveReport {
            tasks_checked: 10,
            tasks_archived: 3,
        };
        assert_eq!(report.tasks_checked, 10);
        assert_eq!(report.tasks_archived, 3);
    }

    async fn setup_test_db() -> SqlitePool {
        let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();

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
            CREATE TABLE IF NOT EXISTS household_settings (
                household_id TEXT PRIMARY KEY NOT NULL REFERENCES households(id),
                dark_mode BOOLEAN NOT NULL DEFAULT 0,
                role_label_owner TEXT NOT NULL DEFAULT 'Owner',
                role_label_admin TEXT NOT NULL DEFAULT 'Admin',
                role_label_member TEXT NOT NULL DEFAULT 'Member',
                hierarchy_type TEXT NOT NULL DEFAULT 'organized',
                timezone TEXT NOT NULL DEFAULT 'UTC',
                rewards_enabled BOOLEAN NOT NULL DEFAULT 0,
                punishments_enabled BOOLEAN NOT NULL DEFAULT 0,
                chat_enabled BOOLEAN NOT NULL DEFAULT 0,
                vacation_mode BOOLEAN NOT NULL DEFAULT 0,
                vacation_start DATE,
                vacation_end DATE,
                auto_archive_days INTEGER DEFAULT 7,
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
                description TEXT,
                recurrence_type TEXT NOT NULL DEFAULT 'daily',
                recurrence_value TEXT,
                assigned_user_id TEXT REFERENCES users(id),
                target_count INTEGER NOT NULL DEFAULT 1,
                time_period TEXT NOT NULL DEFAULT 'day',
                allow_exceed_target BOOLEAN NOT NULL DEFAULT 0,
                requires_review BOOLEAN NOT NULL DEFAULT 0,
                points_reward INTEGER NOT NULL DEFAULT 0,
                points_penalty INTEGER NOT NULL DEFAULT 0,
                due_time TEXT,
                habit_type TEXT NOT NULL DEFAULT 'good',
                category_id TEXT REFERENCES task_categories(id),
                archived BOOLEAN NOT NULL DEFAULT 0,
                paused BOOLEAN NOT NULL DEFAULT 0,
                suggestion TEXT CHECK(suggestion IN ('suggested', 'approved', 'denied')),
                suggested_by TEXT REFERENCES users(id),
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
                due_date DATE NOT NULL,
                status TEXT NOT NULL DEFAULT 'approved',
                completed_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
                reviewed_by TEXT REFERENCES users(id),
                reviewed_at DATETIME
            )
            "#,
        )
        .execute(&pool)
        .await
        .unwrap();

        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS activity_logs (
                id TEXT PRIMARY KEY NOT NULL,
                household_id TEXT NOT NULL REFERENCES households(id),
                actor_id TEXT NOT NULL,
                affected_user_id TEXT,
                activity_type TEXT NOT NULL,
                entity_type TEXT,
                entity_id TEXT,
                details TEXT,
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
            "INSERT INTO users (id, username, email, password_hash) VALUES (?, 'testuser', 'test@example.com', 'hash')",
        )
        .bind(user_id.to_string())
        .execute(pool)
        .await
        .unwrap();
        user_id
    }

    async fn create_test_household(pool: &SqlitePool, owner_id: &Uuid) -> Uuid {
        let household_id = Uuid::new_v4();
        sqlx::query("INSERT INTO households (id, name, owner_id) VALUES (?, 'Test Household', ?)")
            .bind(household_id.to_string())
            .bind(owner_id.to_string())
            .execute(pool)
            .await
            .unwrap();

        // Create settings with auto_archive_days = 7
        sqlx::query(
            "INSERT INTO household_settings (household_id, auto_archive_days) VALUES (?, 7)",
        )
        .bind(household_id.to_string())
        .execute(pool)
        .await
        .unwrap();

        household_id
    }

    #[tokio::test]
    async fn test_process_auto_archive_no_tasks() {
        let pool = setup_test_db().await;
        let user_id = create_test_user(&pool).await;
        let _ = create_test_household(&pool, &user_id).await;

        let report = process_auto_archive(&pool).await.unwrap();
        assert_eq!(report.tasks_checked, 0);
        assert_eq!(report.tasks_archived, 0);
    }

    #[tokio::test]
    async fn test_process_auto_archive_uncompleted_task_not_archived() {
        let pool = setup_test_db().await;
        let user_id = create_test_user(&pool).await;
        let household_id = create_test_household(&pool, &user_id).await;

        // Create a one-time task without completion
        let task_id = Uuid::new_v4();
        sqlx::query(
            "INSERT INTO tasks (id, household_id, title, recurrence_type) VALUES (?, ?, 'Test Task', 'onetime')",
        )
        .bind(task_id.to_string())
        .bind(household_id.to_string())
        .execute(&pool)
        .await
        .unwrap();

        let report = process_auto_archive(&pool).await.unwrap();
        assert_eq!(report.tasks_checked, 1);
        assert_eq!(report.tasks_archived, 0); // Not archived because not completed
    }

    #[tokio::test]
    async fn test_process_auto_archive_completed_onetime_task() {
        let pool = setup_test_db().await;
        let user_id = create_test_user(&pool).await;
        let household_id = create_test_household(&pool, &user_id).await;

        // Create a one-time task
        let task_id = Uuid::new_v4();
        sqlx::query(
            "INSERT INTO tasks (id, household_id, title, recurrence_type) VALUES (?, ?, 'Test Task', 'onetime')",
        )
        .bind(task_id.to_string())
        .bind(household_id.to_string())
        .execute(&pool)
        .await
        .unwrap();

        // Complete the task 10 days ago (past grace period of 7 days)
        let completion_id = Uuid::new_v4();
        let completion_date = chrono::Utc::now().date_naive() - Duration::days(10);
        sqlx::query(
            "INSERT INTO task_completions (id, task_id, user_id, due_date, status) VALUES (?, ?, ?, ?, 'approved')",
        )
        .bind(completion_id.to_string())
        .bind(task_id.to_string())
        .bind(user_id.to_string())
        .bind(completion_date)
        .execute(&pool)
        .await
        .unwrap();

        let report = process_auto_archive(&pool).await.unwrap();
        assert_eq!(report.tasks_checked, 1);
        assert_eq!(report.tasks_archived, 1); // Should be archived

        // Verify task is archived
        let archived: bool = sqlx::query_scalar("SELECT archived FROM tasks WHERE id = ?")
            .bind(task_id.to_string())
            .fetch_one(&pool)
            .await
            .unwrap();
        assert!(archived);
    }

    #[tokio::test]
    async fn test_process_auto_archive_completed_within_grace_period() {
        let pool = setup_test_db().await;
        let user_id = create_test_user(&pool).await;
        let household_id = create_test_household(&pool, &user_id).await;

        // Create a one-time task
        let task_id = Uuid::new_v4();
        sqlx::query(
            "INSERT INTO tasks (id, household_id, title, recurrence_type) VALUES (?, ?, 'Test Task', 'onetime')",
        )
        .bind(task_id.to_string())
        .bind(household_id.to_string())
        .execute(&pool)
        .await
        .unwrap();

        // Complete the task 3 days ago (within grace period of 7 days)
        let completion_id = Uuid::new_v4();
        let completion_date = chrono::Utc::now().date_naive() - Duration::days(3);
        sqlx::query(
            "INSERT INTO task_completions (id, task_id, user_id, due_date, status) VALUES (?, ?, ?, ?, 'approved')",
        )
        .bind(completion_id.to_string())
        .bind(task_id.to_string())
        .bind(user_id.to_string())
        .bind(completion_date)
        .execute(&pool)
        .await
        .unwrap();

        let report = process_auto_archive(&pool).await.unwrap();
        assert_eq!(report.tasks_checked, 1);
        assert_eq!(report.tasks_archived, 0); // Should NOT be archived yet
    }

    #[tokio::test]
    async fn test_process_auto_archive_disabled() {
        let pool = setup_test_db().await;
        let user_id = create_test_user(&pool).await;
        let household_id = Uuid::new_v4();

        // Create household with auto_archive disabled
        sqlx::query("INSERT INTO households (id, name, owner_id) VALUES (?, 'Test Household', ?)")
            .bind(household_id.to_string())
            .bind(user_id.to_string())
            .execute(&pool)
            .await
            .unwrap();

        sqlx::query(
            "INSERT INTO household_settings (household_id, auto_archive_days) VALUES (?, NULL)",
        )
        .bind(household_id.to_string())
        .execute(&pool)
        .await
        .unwrap();

        // Create a one-time task
        let task_id = Uuid::new_v4();
        sqlx::query(
            "INSERT INTO tasks (id, household_id, title, recurrence_type) VALUES (?, ?, 'Test Task', 'onetime')",
        )
        .bind(task_id.to_string())
        .bind(household_id.to_string())
        .execute(&pool)
        .await
        .unwrap();

        // Complete the task 10 days ago
        let completion_id = Uuid::new_v4();
        let completion_date = chrono::Utc::now().date_naive() - Duration::days(10);
        sqlx::query(
            "INSERT INTO task_completions (id, task_id, user_id, due_date, status) VALUES (?, ?, ?, ?, 'approved')",
        )
        .bind(completion_id.to_string())
        .bind(task_id.to_string())
        .bind(user_id.to_string())
        .bind(completion_date)
        .execute(&pool)
        .await
        .unwrap();

        let report = process_auto_archive(&pool).await.unwrap();
        // Task should be checked but not archived because auto_archive_days is NULL
        assert_eq!(report.tasks_archived, 0);
    }

    #[tokio::test]
    async fn test_process_auto_archive_daily_task_not_archived() {
        let pool = setup_test_db().await;
        let user_id = create_test_user(&pool).await;
        let household_id = create_test_household(&pool, &user_id).await;

        // Create a daily task (should never be auto-archived)
        let task_id = Uuid::new_v4();
        sqlx::query(
            "INSERT INTO tasks (id, household_id, title, recurrence_type) VALUES (?, ?, 'Daily Task', 'daily')",
        )
        .bind(task_id.to_string())
        .bind(household_id.to_string())
        .execute(&pool)
        .await
        .unwrap();

        // Complete the task 10 days ago
        let completion_id = Uuid::new_v4();
        let completion_date = chrono::Utc::now().date_naive() - Duration::days(10);
        sqlx::query(
            "INSERT INTO task_completions (id, task_id, user_id, due_date, status) VALUES (?, ?, ?, ?, 'approved')",
        )
        .bind(completion_id.to_string())
        .bind(task_id.to_string())
        .bind(user_id.to_string())
        .bind(completion_date)
        .execute(&pool)
        .await
        .unwrap();

        let report = process_auto_archive(&pool).await.unwrap();
        // Daily tasks should not be checked for auto-archive (query filters them)
        assert_eq!(report.tasks_checked, 0);
        assert_eq!(report.tasks_archived, 0);
    }
}
