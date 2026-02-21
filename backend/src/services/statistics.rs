use chrono::{Datelike, NaiveDate, Utc};
use sqlx::SqlitePool;
use thiserror::Error;
use uuid::Uuid;

use crate::models::{
    MonthlyStatisticsRow, MonthlyStatisticsTaskRow, WeeklyStatisticsRow, WeeklyStatisticsTaskRow,
};

#[derive(Debug, Error)]
pub enum StatisticsError {
    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),
    #[allow(dead_code)]
    #[error("Statistics not found")]
    NotFound,
}

/// Get the week start date based on week_start_day setting and a reference date
/// week_start_day: 0 = Monday, 1 = Tuesday, ..., 6 = Sunday
pub fn get_week_start(date: NaiveDate, week_start_day: i32) -> NaiveDate {
    let current_weekday = date.weekday().num_days_from_monday() as i32; // 0 = Monday, 6 = Sunday
    let target_weekday = week_start_day;

    let days_since_start = (current_weekday - target_weekday + 7) % 7;
    date - chrono::Duration::days(days_since_start as i64)
}

/// Get the week end date (6 days after start)
pub fn get_week_end(week_start: NaiveDate) -> NaiveDate {
    week_start + chrono::Duration::days(6)
}

/// Get the month start date (first day of month)
pub fn get_month_start(date: NaiveDate) -> NaiveDate {
    NaiveDate::from_ymd_opt(date.year(), date.month(), 1).unwrap()
}

/// Get the month end date (last day of month)
pub fn get_month_end(date: NaiveDate) -> NaiveDate {
    let next_month = if date.month() == 12 {
        NaiveDate::from_ymd_opt(date.year() + 1, 1, 1).unwrap()
    } else {
        NaiveDate::from_ymd_opt(date.year(), date.month() + 1, 1).unwrap()
    };
    next_month - chrono::Duration::days(1)
}

/// Calculate and store weekly statistics for a household
pub async fn calculate_weekly_statistics(
    pool: &SqlitePool,
    household_id: &Uuid,
    week_start: NaiveDate,
) -> Result<(), StatisticsError> {
    let week_end = get_week_end(week_start);
    let now = Utc::now();

    // Get all members with their usernames
    let members: Vec<(String, String)> = sqlx::query_as(
        r#"
        SELECT m.user_id, u.username
        FROM household_memberships m
        JOIN users u ON m.user_id = u.id
        WHERE m.household_id = ?
        "#,
    )
    .bind(household_id.to_string())
    .fetch_all(pool)
    .await?;

    // Get all tasks for this household with assigned users and habit type
    let tasks: Vec<(String, String, Option<String>, String)> = sqlx::query_as(
        r#"
        SELECT id, title, assigned_user_id, habit_type
        FROM tasks
        WHERE household_id = ? AND archived = FALSE
        "#,
    )
    .bind(household_id.to_string())
    .fetch_all(pool)
    .await?;

    // For each member, calculate their statistics
    for (user_id, _username) in &members {
        // Find tasks assigned to this user
        let user_tasks: Vec<&(String, String, Option<String>, String)> = tasks
            .iter()
            .filter(|(_, _, assigned, _)| assigned.as_ref() == Some(user_id))
            .collect();

        if user_tasks.is_empty() {
            continue;
        }

        let mut total_expected = 0i32;
        let mut total_completed = 0i32;
        let mut task_stats: Vec<(String, String, i32, i32)> = Vec::new();

        for (task_id, task_title, _, habit_type) in user_tasks {
            let is_bad_habit = habit_type == "bad";

            // Count expected periods within the week (based on period_start)
            let expected: i64 = sqlx::query_scalar(
                r#"
                SELECT COUNT(*) FROM task_period_results
                WHERE task_id = ?
                AND period_start >= ? AND period_start <= ?
                "#,
            )
            .bind(task_id)
            .bind(week_start)
            .bind(week_end)
            .fetch_one(pool)
            .await?;

            // Count completed periods
            let completed: i64 = sqlx::query_scalar(
                r#"
                SELECT COUNT(*) FROM task_period_results
                WHERE task_id = ?
                AND period_start >= ? AND period_start <= ?
                AND status = 'completed'
                "#,
            )
            .bind(task_id)
            .bind(week_start)
            .bind(week_end)
            .fetch_one(pool)
            .await?;

            // For bad habits, invert the logic: success = NOT completing the bad habit
            let successful = if is_bad_habit {
                expected - completed
            } else {
                completed
            };

            total_expected += expected as i32;
            total_completed += successful as i32;

            if expected > 0 {
                task_stats.push((
                    task_id.clone(),
                    task_title.clone(),
                    expected as i32,
                    successful as i32,
                ));
            }
        }

        let completion_rate = if total_expected > 0 {
            (total_completed as f64 / total_expected as f64) * 100.0
        } else {
            0.0
        };

        // Upsert weekly statistics
        let stats_id = Uuid::new_v4();
        sqlx::query(
            r#"
            INSERT INTO weekly_statistics (id, household_id, user_id, week_start, week_end, total_expected, total_completed, completion_rate, calculated_at)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)
            ON CONFLICT(household_id, user_id, week_start) DO UPDATE SET
                week_end = excluded.week_end,
                total_expected = excluded.total_expected,
                total_completed = excluded.total_completed,
                completion_rate = excluded.completion_rate,
                calculated_at = excluded.calculated_at
            "#,
        )
        .bind(stats_id.to_string())
        .bind(household_id.to_string())
        .bind(user_id)
        .bind(week_start)
        .bind(week_end)
        .bind(total_expected)
        .bind(total_completed)
        .bind(completion_rate)
        .bind(now)
        .execute(pool)
        .await?;

        // Get the actual stats ID (might be existing row)
        let actual_stats_id: String = sqlx::query_scalar(
            "SELECT id FROM weekly_statistics WHERE household_id = ? AND user_id = ? AND week_start = ?",
        )
        .bind(household_id.to_string())
        .bind(user_id)
        .bind(week_start)
        .fetch_one(pool)
        .await?;

        // Delete existing task breakdowns and insert new ones
        sqlx::query("DELETE FROM weekly_statistics_tasks WHERE weekly_statistics_id = ?")
            .bind(&actual_stats_id)
            .execute(pool)
            .await?;

        for (task_id, task_title, expected, completed) in task_stats {
            let task_completion_rate = if expected > 0 {
                (completed as f64 / expected as f64) * 100.0
            } else {
                0.0
            };

            sqlx::query(
                r#"
                INSERT INTO weekly_statistics_tasks (id, weekly_statistics_id, task_id, task_title, expected, completed, completion_rate)
                VALUES (?, ?, ?, ?, ?, ?, ?)
                "#,
            )
            .bind(Uuid::new_v4().to_string())
            .bind(&actual_stats_id)
            .bind(&task_id)
            .bind(&task_title)
            .bind(expected)
            .bind(completed)
            .bind(task_completion_rate)
            .execute(pool)
            .await?;
        }
    }

    Ok(())
}

/// Calculate and store monthly statistics for a household
pub async fn calculate_monthly_statistics(
    pool: &SqlitePool,
    household_id: &Uuid,
    month: NaiveDate,
) -> Result<(), StatisticsError> {
    let month_start = get_month_start(month);
    let month_end = get_month_end(month);
    let now = Utc::now();

    // Get all members with their usernames
    let members: Vec<(String, String)> = sqlx::query_as(
        r#"
        SELECT m.user_id, u.username
        FROM household_memberships m
        JOIN users u ON m.user_id = u.id
        WHERE m.household_id = ?
        "#,
    )
    .bind(household_id.to_string())
    .fetch_all(pool)
    .await?;

    // Get all tasks for this household with assigned users and habit type
    let tasks: Vec<(String, String, Option<String>, String)> = sqlx::query_as(
        r#"
        SELECT id, title, assigned_user_id, habit_type
        FROM tasks
        WHERE household_id = ? AND archived = FALSE
        "#,
    )
    .bind(household_id.to_string())
    .fetch_all(pool)
    .await?;

    // For each member, calculate their statistics
    for (user_id, _username) in &members {
        // Find tasks assigned to this user
        let user_tasks: Vec<&(String, String, Option<String>, String)> = tasks
            .iter()
            .filter(|(_, _, assigned, _)| assigned.as_ref() == Some(user_id))
            .collect();

        if user_tasks.is_empty() {
            continue;
        }

        let mut total_expected = 0i32;
        let mut total_completed = 0i32;
        let mut task_stats: Vec<(String, String, i32, i32)> = Vec::new();

        for (task_id, task_title, _, habit_type) in user_tasks {
            let is_bad_habit = habit_type == "bad";

            // Count expected periods within the month
            let expected: i64 = sqlx::query_scalar(
                r#"
                SELECT COUNT(*) FROM task_period_results
                WHERE task_id = ?
                AND period_start >= ? AND period_start <= ?
                "#,
            )
            .bind(task_id)
            .bind(month_start)
            .bind(month_end)
            .fetch_one(pool)
            .await?;

            // Count completed periods
            let completed: i64 = sqlx::query_scalar(
                r#"
                SELECT COUNT(*) FROM task_period_results
                WHERE task_id = ?
                AND period_start >= ? AND period_start <= ?
                AND status = 'completed'
                "#,
            )
            .bind(task_id)
            .bind(month_start)
            .bind(month_end)
            .fetch_one(pool)
            .await?;

            // For bad habits, invert the logic: success = NOT completing the bad habit
            let successful = if is_bad_habit {
                expected - completed
            } else {
                completed
            };

            total_expected += expected as i32;
            total_completed += successful as i32;

            if expected > 0 {
                task_stats.push((
                    task_id.clone(),
                    task_title.clone(),
                    expected as i32,
                    successful as i32,
                ));
            }
        }

        let completion_rate = if total_expected > 0 {
            (total_completed as f64 / total_expected as f64) * 100.0
        } else {
            0.0
        };

        // Upsert monthly statistics (use month_start as the month identifier)
        let stats_id = Uuid::new_v4();
        sqlx::query(
            r#"
            INSERT INTO monthly_statistics (id, household_id, user_id, month, total_expected, total_completed, completion_rate, calculated_at)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?)
            ON CONFLICT(household_id, user_id, month) DO UPDATE SET
                total_expected = excluded.total_expected,
                total_completed = excluded.total_completed,
                completion_rate = excluded.completion_rate,
                calculated_at = excluded.calculated_at
            "#,
        )
        .bind(stats_id.to_string())
        .bind(household_id.to_string())
        .bind(user_id)
        .bind(month_start)
        .bind(total_expected)
        .bind(total_completed)
        .bind(completion_rate)
        .bind(now)
        .execute(pool)
        .await?;

        // Get the actual stats ID
        let actual_stats_id: String = sqlx::query_scalar(
            "SELECT id FROM monthly_statistics WHERE household_id = ? AND user_id = ? AND month = ?",
        )
        .bind(household_id.to_string())
        .bind(user_id)
        .bind(month_start)
        .fetch_one(pool)
        .await?;

        // Delete existing task breakdowns and insert new ones
        sqlx::query("DELETE FROM monthly_statistics_tasks WHERE monthly_statistics_id = ?")
            .bind(&actual_stats_id)
            .execute(pool)
            .await?;

        for (task_id, task_title, expected, completed) in task_stats {
            let task_completion_rate = if expected > 0 {
                (completed as f64 / expected as f64) * 100.0
            } else {
                0.0
            };

            sqlx::query(
                r#"
                INSERT INTO monthly_statistics_tasks (id, monthly_statistics_id, task_id, task_title, expected, completed, completion_rate)
                VALUES (?, ?, ?, ?, ?, ?, ?)
                "#,
            )
            .bind(Uuid::new_v4().to_string())
            .bind(&actual_stats_id)
            .bind(&task_id)
            .bind(&task_title)
            .bind(expected)
            .bind(completed)
            .bind(task_completion_rate)
            .execute(pool)
            .await?;
        }
    }

    Ok(())
}

/// Get weekly statistics for a household
pub async fn get_weekly_statistics(
    pool: &SqlitePool,
    household_id: &Uuid,
    week_start: NaiveDate,
) -> Result<shared::WeeklyStatisticsResponse, StatisticsError> {
    // Get all member statistics for this week
    let stats_rows: Vec<WeeklyStatisticsRow> = sqlx::query_as(
        r#"
        SELECT * FROM weekly_statistics
        WHERE household_id = ? AND week_start = ?
        "#,
    )
    .bind(household_id.to_string())
    .bind(week_start)
    .fetch_all(pool)
    .await?;

    let week_end = get_week_end(week_start);

    let mut members = Vec::new();
    for stats in stats_rows {
        // Get username
        let username: String = sqlx::query_scalar("SELECT username FROM users WHERE id = ?")
            .bind(&stats.user_id)
            .fetch_one(pool)
            .await?;

        // Get task breakdowns
        let task_rows: Vec<WeeklyStatisticsTaskRow> = sqlx::query_as(
            "SELECT * FROM weekly_statistics_tasks WHERE weekly_statistics_id = ?",
        )
        .bind(&stats.id)
        .fetch_all(pool)
        .await?;

        let task_stats: Vec<shared::TaskStatistic> =
            task_rows.iter().map(|r| r.to_shared()).collect();

        members.push(stats.to_member_statistic(username, task_stats));
    }

    Ok(shared::WeeklyStatisticsResponse {
        week_start,
        week_end,
        members,
    })
}

/// Get monthly statistics for a household
pub async fn get_monthly_statistics(
    pool: &SqlitePool,
    household_id: &Uuid,
    month: NaiveDate,
) -> Result<shared::MonthlyStatisticsResponse, StatisticsError> {
    let month_start = get_month_start(month);

    // Get all member statistics for this month
    let stats_rows: Vec<MonthlyStatisticsRow> = sqlx::query_as(
        r#"
        SELECT * FROM monthly_statistics
        WHERE household_id = ? AND month = ?
        "#,
    )
    .bind(household_id.to_string())
    .bind(month_start)
    .fetch_all(pool)
    .await?;

    let mut members = Vec::new();
    for stats in stats_rows {
        // Get username
        let username: String = sqlx::query_scalar("SELECT username FROM users WHERE id = ?")
            .bind(&stats.user_id)
            .fetch_one(pool)
            .await?;

        // Get task breakdowns
        let task_rows: Vec<MonthlyStatisticsTaskRow> = sqlx::query_as(
            "SELECT * FROM monthly_statistics_tasks WHERE monthly_statistics_id = ?",
        )
        .bind(&stats.id)
        .fetch_all(pool)
        .await?;

        let task_stats: Vec<shared::TaskStatistic> =
            task_rows.iter().map(|r| r.to_shared()).collect();

        members.push(stats.to_member_statistic(username, task_stats));
    }

    Ok(shared::MonthlyStatisticsResponse {
        month: month_start,
        members,
    })
}

/// List available weeks with statistics for a household
pub async fn list_available_weeks(
    pool: &SqlitePool,
    household_id: &Uuid,
) -> Result<Vec<NaiveDate>, StatisticsError> {
    let weeks: Vec<NaiveDate> = sqlx::query_scalar(
        r#"
        SELECT DISTINCT week_start FROM weekly_statistics
        WHERE household_id = ?
        ORDER BY week_start DESC
        "#,
    )
    .bind(household_id.to_string())
    .fetch_all(pool)
    .await?;

    Ok(weeks)
}

/// List available months with statistics for a household
pub async fn list_available_months(
    pool: &SqlitePool,
    household_id: &Uuid,
) -> Result<Vec<NaiveDate>, StatisticsError> {
    let months: Vec<NaiveDate> = sqlx::query_scalar(
        r#"
        SELECT DISTINCT month FROM monthly_statistics
        WHERE household_id = ?
        ORDER BY month DESC
        "#,
    )
    .bind(household_id.to_string())
    .fetch_all(pool)
    .await?;

    Ok(months)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_week_start_monday() {
        // week_start_day = 0 means Monday
        let friday = NaiveDate::from_ymd_opt(2024, 1, 12).unwrap(); // Friday
        let week_start = get_week_start(friday, 0);
        assert_eq!(week_start, NaiveDate::from_ymd_opt(2024, 1, 8).unwrap()); // Monday
    }

    #[test]
    fn test_get_week_start_sunday() {
        // week_start_day = 6 means Sunday
        let friday = NaiveDate::from_ymd_opt(2024, 1, 12).unwrap(); // Friday
        let week_start = get_week_start(friday, 6);
        assert_eq!(week_start, NaiveDate::from_ymd_opt(2024, 1, 7).unwrap()); // Sunday
    }

    #[test]
    fn test_get_week_start_on_start_day() {
        // If the date is already the start day, it should return itself
        let monday = NaiveDate::from_ymd_opt(2024, 1, 8).unwrap();
        let week_start = get_week_start(monday, 0);
        assert_eq!(week_start, monday);
    }

    #[test]
    fn test_get_week_end() {
        let week_start = NaiveDate::from_ymd_opt(2024, 1, 8).unwrap();
        let week_end = get_week_end(week_start);
        assert_eq!(week_end, NaiveDate::from_ymd_opt(2024, 1, 14).unwrap());
    }

    #[test]
    fn test_get_month_start() {
        let date = NaiveDate::from_ymd_opt(2024, 1, 15).unwrap();
        let month_start = get_month_start(date);
        assert_eq!(month_start, NaiveDate::from_ymd_opt(2024, 1, 1).unwrap());
    }

    #[test]
    fn test_get_month_end() {
        // January has 31 days
        let date = NaiveDate::from_ymd_opt(2024, 1, 15).unwrap();
        let month_end = get_month_end(date);
        assert_eq!(month_end, NaiveDate::from_ymd_opt(2024, 1, 31).unwrap());

        // February 2024 has 29 days (leap year)
        let date = NaiveDate::from_ymd_opt(2024, 2, 15).unwrap();
        let month_end = get_month_end(date);
        assert_eq!(month_end, NaiveDate::from_ymd_opt(2024, 2, 29).unwrap());

        // December
        let date = NaiveDate::from_ymd_opt(2024, 12, 15).unwrap();
        let month_end = get_month_end(date);
        assert_eq!(month_end, NaiveDate::from_ymd_opt(2024, 12, 31).unwrap());
    }
}
