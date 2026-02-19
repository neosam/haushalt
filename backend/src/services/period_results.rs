use chrono::{DateTime, NaiveDate, Utc};
use shared::{PeriodStatus, TaskPeriodResult};
use sqlx::SqlitePool;
use thiserror::Error;
use uuid::Uuid;

use crate::models::TaskPeriodResultRow;

#[derive(Debug, Error)]
pub enum PeriodResultError {
    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),
    #[allow(dead_code)]
    #[error("Period result not found")]
    NotFound,
}

/// Create or update a period result for a task
/// If a result already exists for the same task and period_start, it will be updated
#[allow(clippy::too_many_arguments)]
pub async fn finalize_period(
    pool: &SqlitePool,
    task_id: &Uuid,
    period_start: NaiveDate,
    period_end: NaiveDate,
    status: PeriodStatus,
    completions_count: i32,
    target_count: i32,
    finalized_by: &str,
    notes: Option<&str>,
) -> Result<TaskPeriodResult, PeriodResultError> {
    let now = Utc::now();

    // Check if a result already exists for this task and period
    let existing: Option<TaskPeriodResultRow> = sqlx::query_as(
        "SELECT * FROM task_period_results WHERE task_id = ? AND period_start = ?",
    )
    .bind(task_id.to_string())
    .bind(period_start)
    .fetch_optional(pool)
    .await?;

    if let Some(existing_row) = existing {
        // Update existing record (e.g., late completion changes failed -> completed)
        sqlx::query(
            r#"UPDATE task_period_results SET
                status = ?,
                completions_count = ?,
                target_count = ?,
                finalized_at = ?,
                finalized_by = ?,
                notes = ?
            WHERE id = ?"#,
        )
        .bind(status.as_str())
        .bind(completions_count)
        .bind(target_count)
        .bind(now)
        .bind(finalized_by)
        .bind(notes)
        .bind(&existing_row.id)
        .execute(pool)
        .await?;

        // Fetch updated record
        let updated: TaskPeriodResultRow = sqlx::query_as(
            "SELECT * FROM task_period_results WHERE id = ?",
        )
        .bind(&existing_row.id)
        .fetch_one(pool)
        .await?;

        Ok(updated.to_shared())
    } else {
        // Insert new record
        let id = Uuid::new_v4();

        sqlx::query(
            r#"INSERT INTO task_period_results
                (id, task_id, period_start, period_end, status, completions_count, target_count, finalized_at, finalized_by, notes)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)"#,
        )
        .bind(id.to_string())
        .bind(task_id.to_string())
        .bind(period_start)
        .bind(period_end)
        .bind(status.as_str())
        .bind(completions_count)
        .bind(target_count)
        .bind(now)
        .bind(finalized_by)
        .bind(notes)
        .execute(pool)
        .await?;

        // Fetch inserted record
        let inserted: TaskPeriodResultRow = sqlx::query_as(
            "SELECT * FROM task_period_results WHERE id = ?",
        )
        .bind(id.to_string())
        .fetch_one(pool)
        .await?;

        Ok(inserted.to_shared())
    }
}

/// Get a single period result by task ID and period start date
#[allow(dead_code)]
pub async fn get_period_result(
    pool: &SqlitePool,
    task_id: &Uuid,
    period_start: NaiveDate,
) -> Result<Option<TaskPeriodResult>, PeriodResultError> {
    let row: Option<TaskPeriodResultRow> = sqlx::query_as(
        "SELECT * FROM task_period_results WHERE task_id = ? AND period_start = ?",
    )
    .bind(task_id.to_string())
    .bind(period_start)
    .fetch_optional(pool)
    .await?;

    Ok(row.map(|r| r.to_shared()))
}

/// Get all period results for a task within a date range
#[allow(dead_code)]
pub async fn get_period_results_for_task(
    pool: &SqlitePool,
    task_id: &Uuid,
    start_date: NaiveDate,
    end_date: NaiveDate,
) -> Result<Vec<TaskPeriodResult>, PeriodResultError> {
    let rows: Vec<TaskPeriodResultRow> = sqlx::query_as(
        r#"SELECT * FROM task_period_results
        WHERE task_id = ?
        AND period_start >= ?
        AND period_start <= ?
        ORDER BY period_start DESC"#,
    )
    .bind(task_id.to_string())
    .bind(start_date)
    .bind(end_date)
    .fetch_all(pool)
    .await?;

    Ok(rows.into_iter().map(|r| r.to_shared()).collect())
}

/// Count period results by status for a task within a date range
#[derive(Debug, Clone)]
pub struct PeriodCounts {
    #[allow(dead_code)]
    pub completed: i32,
    #[allow(dead_code)]
    pub failed: i32,
    pub skipped: i32,
}

pub async fn count_period_results(
    pool: &SqlitePool,
    task_id: &Uuid,
    start_date: NaiveDate,
    end_date: NaiveDate,
) -> Result<PeriodCounts, PeriodResultError> {
    let completed: i64 = sqlx::query_scalar(
        r#"SELECT COUNT(*) FROM task_period_results
        WHERE task_id = ? AND period_start >= ? AND period_start <= ? AND status = 'completed'"#,
    )
    .bind(task_id.to_string())
    .bind(start_date)
    .bind(end_date)
    .fetch_one(pool)
    .await?;

    let failed: i64 = sqlx::query_scalar(
        r#"SELECT COUNT(*) FROM task_period_results
        WHERE task_id = ? AND period_start >= ? AND period_start <= ? AND status = 'failed'"#,
    )
    .bind(task_id.to_string())
    .bind(start_date)
    .bind(end_date)
    .fetch_one(pool)
    .await?;

    let skipped: i64 = sqlx::query_scalar(
        r#"SELECT COUNT(*) FROM task_period_results
        WHERE task_id = ? AND period_start >= ? AND period_start <= ? AND status = 'skipped'"#,
    )
    .bind(task_id.to_string())
    .bind(start_date)
    .bind(end_date)
    .fetch_one(pool)
    .await?;

    Ok(PeriodCounts {
        completed: completed as i32,
        failed: failed as i32,
        skipped: skipped as i32,
    })
}

/// Manually update a period result's status (for admin corrections)
#[allow(dead_code)]
pub async fn update_period_status(
    pool: &SqlitePool,
    task_id: &Uuid,
    period_start: NaiveDate,
    new_status: PeriodStatus,
    finalized_by: &str,
    notes: Option<&str>,
) -> Result<TaskPeriodResult, PeriodResultError> {
    let now = Utc::now();

    let result = sqlx::query(
        r#"UPDATE task_period_results SET
            status = ?,
            finalized_at = ?,
            finalized_by = ?,
            notes = ?
        WHERE task_id = ? AND period_start = ?"#,
    )
    .bind(new_status.as_str())
    .bind(now)
    .bind(finalized_by)
    .bind(notes)
    .bind(task_id.to_string())
    .bind(period_start)
    .execute(pool)
    .await?;

    if result.rows_affected() == 0 {
        return Err(PeriodResultError::NotFound);
    }

    let updated: TaskPeriodResultRow = sqlx::query_as(
        "SELECT * FROM task_period_results WHERE task_id = ? AND period_start = ?",
    )
    .bind(task_id.to_string())
    .bind(period_start)
    .fetch_one(pool)
    .await?;

    Ok(updated.to_shared())
}

/// Check if a period has already been finalized
pub async fn is_period_finalized(
    pool: &SqlitePool,
    task_id: &Uuid,
    period_start: NaiveDate,
) -> Result<bool, PeriodResultError> {
    let count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM task_period_results WHERE task_id = ? AND period_start = ?",
    )
    .bind(task_id.to_string())
    .bind(period_start)
    .fetch_one(pool)
    .await?;

    Ok(count > 0)
}

/// Get all unfinalized periods for a task that ended before a given date
/// This is useful for the background job to find periods that need to be finalized
#[allow(dead_code)]
pub async fn get_unfinalized_period_dates(
    pool: &SqlitePool,
    task_id: &Uuid,
    _task_created_at: DateTime<Utc>,
    _before_date: NaiveDate,
) -> Result<Vec<NaiveDate>, PeriodResultError> {
    // Get all existing period_start dates for this task
    let existing_periods: Vec<NaiveDate> = sqlx::query_scalar(
        "SELECT period_start FROM task_period_results WHERE task_id = ?",
    )
    .bind(task_id.to_string())
    .fetch_all(pool)
    .await?;

    // The caller will need to generate the expected period dates based on the task's
    // recurrence pattern and compare with existing_periods
    // For now, we just return the existing periods so the caller can determine what's missing

    Ok(existing_periods)
}

#[cfg(test)]
mod tests {
    use super::*;

    async fn setup_test_db() -> SqlitePool {
        let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();

        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS task_period_results (
                id TEXT PRIMARY KEY NOT NULL,
                task_id TEXT NOT NULL,
                period_start DATE NOT NULL,
                period_end DATE NOT NULL,
                status TEXT NOT NULL CHECK(status IN ('completed', 'failed', 'skipped')),
                completions_count INTEGER NOT NULL,
                target_count INTEGER NOT NULL,
                finalized_at DATETIME NOT NULL,
                finalized_by TEXT NOT NULL DEFAULT 'system',
                notes TEXT
            )
            "#,
        )
        .execute(&pool)
        .await
        .unwrap();

        sqlx::query(
            "CREATE UNIQUE INDEX IF NOT EXISTS idx_period_results_task_date ON task_period_results(task_id, period_start)",
        )
        .execute(&pool)
        .await
        .unwrap();

        pool
    }

    #[tokio::test]
    async fn test_finalize_period_creates_new_result() {
        let pool = setup_test_db().await;
        let task_id = Uuid::new_v4();
        let period_start = NaiveDate::from_ymd_opt(2024, 1, 15).unwrap();
        let period_end = NaiveDate::from_ymd_opt(2024, 1, 15).unwrap();

        let result = finalize_period(
            &pool,
            &task_id,
            period_start,
            period_end,
            PeriodStatus::Completed,
            3,
            3,
            "system",
            None,
        )
        .await
        .unwrap();

        assert_eq!(result.task_id, task_id);
        assert_eq!(result.period_start, period_start);
        assert_eq!(result.status, PeriodStatus::Completed);
        assert_eq!(result.completions_count, 3);
        assert_eq!(result.target_count, 3);
    }

    #[tokio::test]
    async fn test_finalize_period_updates_existing_result() {
        let pool = setup_test_db().await;
        let task_id = Uuid::new_v4();
        let period_start = NaiveDate::from_ymd_opt(2024, 1, 15).unwrap();
        let period_end = NaiveDate::from_ymd_opt(2024, 1, 15).unwrap();

        // Create initial result as failed
        finalize_period(
            &pool,
            &task_id,
            period_start,
            period_end,
            PeriodStatus::Failed,
            1,
            3,
            "system",
            None,
        )
        .await
        .unwrap();

        // Update to completed (late completion)
        let result = finalize_period(
            &pool,
            &task_id,
            period_start,
            period_end,
            PeriodStatus::Completed,
            3,
            3,
            "user",
            Some("Late completion"),
        )
        .await
        .unwrap();

        assert_eq!(result.status, PeriodStatus::Completed);
        assert_eq!(result.finalized_by, "user");
        assert_eq!(result.notes, Some("Late completion".to_string()));

        // Verify only one record exists
        let count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM task_period_results WHERE task_id = ?",
        )
        .bind(task_id.to_string())
        .fetch_one(&pool)
        .await
        .unwrap();

        assert_eq!(count, 1);
    }

    #[tokio::test]
    async fn test_get_period_result() {
        let pool = setup_test_db().await;
        let task_id = Uuid::new_v4();
        let period_start = NaiveDate::from_ymd_opt(2024, 1, 15).unwrap();

        // Should be None initially
        let result = get_period_result(&pool, &task_id, period_start)
            .await
            .unwrap();
        assert!(result.is_none());

        // Create a result
        finalize_period(
            &pool,
            &task_id,
            period_start,
            period_start,
            PeriodStatus::Completed,
            1,
            1,
            "system",
            None,
        )
        .await
        .unwrap();

        // Should now return the result
        let result = get_period_result(&pool, &task_id, period_start)
            .await
            .unwrap();
        assert!(result.is_some());
        assert_eq!(result.unwrap().status, PeriodStatus::Completed);
    }

    #[tokio::test]
    async fn test_get_period_results_for_task() {
        let pool = setup_test_db().await;
        let task_id = Uuid::new_v4();

        // Create multiple period results
        for day in 10..=15 {
            let date = NaiveDate::from_ymd_opt(2024, 1, day).unwrap();
            let status = if day % 2 == 0 {
                PeriodStatus::Completed
            } else {
                PeriodStatus::Failed
            };
            finalize_period(&pool, &task_id, date, date, status, 1, 1, "system", None)
                .await
                .unwrap();
        }

        // Get results for date range
        let start = NaiveDate::from_ymd_opt(2024, 1, 12).unwrap();
        let end = NaiveDate::from_ymd_opt(2024, 1, 14).unwrap();

        let results = get_period_results_for_task(&pool, &task_id, start, end)
            .await
            .unwrap();

        assert_eq!(results.len(), 3); // Days 12, 13, 14
    }

    #[tokio::test]
    async fn test_count_period_results() {
        let pool = setup_test_db().await;
        let task_id = Uuid::new_v4();

        // Create period results with different statuses
        for day in 1..=10 {
            let date = NaiveDate::from_ymd_opt(2024, 1, day).unwrap();
            let status = if day <= 5 {
                PeriodStatus::Completed
            } else if day <= 8 {
                PeriodStatus::Failed
            } else {
                PeriodStatus::Skipped
            };
            finalize_period(&pool, &task_id, date, date, status, 1, 1, "system", None)
                .await
                .unwrap();
        }

        let start = NaiveDate::from_ymd_opt(2024, 1, 1).unwrap();
        let end = NaiveDate::from_ymd_opt(2024, 1, 10).unwrap();

        let counts = count_period_results(&pool, &task_id, start, end)
            .await
            .unwrap();

        assert_eq!(counts.completed, 5);
        assert_eq!(counts.failed, 3);
        assert_eq!(counts.skipped, 2);
    }

    #[tokio::test]
    async fn test_update_period_status() {
        let pool = setup_test_db().await;
        let task_id = Uuid::new_v4();
        let period_start = NaiveDate::from_ymd_opt(2024, 1, 15).unwrap();

        // Create initial result
        finalize_period(
            &pool,
            &task_id,
            period_start,
            period_start,
            PeriodStatus::Failed,
            0,
            1,
            "system",
            None,
        )
        .await
        .unwrap();

        // Update status
        let result = update_period_status(
            &pool,
            &task_id,
            period_start,
            PeriodStatus::Skipped,
            "admin",
            Some("Marked skipped due to illness"),
        )
        .await
        .unwrap();

        assert_eq!(result.status, PeriodStatus::Skipped);
        assert_eq!(result.finalized_by, "admin");
    }

    #[tokio::test]
    async fn test_update_period_status_not_found() {
        let pool = setup_test_db().await;
        let task_id = Uuid::new_v4();
        let period_start = NaiveDate::from_ymd_opt(2024, 1, 15).unwrap();

        let result = update_period_status(
            &pool,
            &task_id,
            period_start,
            PeriodStatus::Skipped,
            "admin",
            None,
        )
        .await;

        assert!(matches!(result, Err(PeriodResultError::NotFound)));
    }

    #[tokio::test]
    async fn test_is_period_finalized() {
        let pool = setup_test_db().await;
        let task_id = Uuid::new_v4();
        let period_start = NaiveDate::from_ymd_opt(2024, 1, 15).unwrap();

        // Should be false initially
        assert!(!is_period_finalized(&pool, &task_id, period_start)
            .await
            .unwrap());

        // Create result
        finalize_period(
            &pool,
            &task_id,
            period_start,
            period_start,
            PeriodStatus::Completed,
            1,
            1,
            "system",
            None,
        )
        .await
        .unwrap();

        // Should be true now
        assert!(is_period_finalized(&pool, &task_id, period_start)
            .await
            .unwrap());
    }
}
