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
    #[error("End date must not be before start date")]
    InvalidRange,
    #[error("Range covers {0} periods, at most {MAX_RECALCULATION_PERIODS} are allowed")]
    RangeTooLarge(usize),
}

/// Upper bound for a single range recalculation. Keeps an accidental "since 1970" request
/// from turning into a multi-minute database walk.
pub const MAX_RECALCULATION_PERIODS: usize = 260;

/// Get the week start date based on week_start_day setting and a reference date
/// week_start_day: 0 = Monday, 1 = Tuesday, ..., 6 = Sunday
pub fn get_week_start(date: NaiveDate, week_start_day: i32) -> NaiveDate {
    shared::week_start_for(date, week_start_day)
}

/// Get the week end date (6 days after start)
pub fn get_week_end(week_start: NaiveDate) -> NaiveDate {
    week_start + chrono::Duration::days(6)
}

/// Get the month start date (first day of month)
pub fn get_month_start(date: NaiveDate) -> NaiveDate {
    shared::month_start_for(date)
}

/// Get the month end date (last day of month)
pub fn get_month_end(date: NaiveDate) -> NaiveDate {
    next_month_start(date) - chrono::Duration::days(1)
}

/// First day of the month following the one containing `date`
fn next_month_start(date: NaiveDate) -> NaiveDate {
    if date.month() == 12 {
        NaiveDate::from_ymd_opt(date.year() + 1, 1, 1).unwrap()
    } else {
        NaiveDate::from_ymd_opt(date.year(), date.month() + 1, 1).unwrap()
    }
}

/// All week starts covering the range from..=to, aligned to the household's week start day.
///
/// The week containing `from` and the week containing `to` are both included, even when the
/// range boundaries fall in the middle of a week.
pub fn weekly_period_starts(
    from: NaiveDate,
    to: NaiveDate,
    week_start_day: i32,
) -> Result<Vec<NaiveDate>, StatisticsError> {
    if to < from {
        return Err(StatisticsError::InvalidRange);
    }

    let mut periods = Vec::new();
    let mut current = get_week_start(from, week_start_day);

    while current <= to {
        if periods.len() == MAX_RECALCULATION_PERIODS {
            return Err(StatisticsError::RangeTooLarge(
                count_weeks(current, to, periods.len()),
            ));
        }
        periods.push(current);
        current += chrono::Duration::days(7);
    }

    Ok(periods)
}

/// Total number of weeks the range would have produced — only used for the error message
fn count_weeks(next_start: NaiveDate, to: NaiveDate, already_counted: usize) -> usize {
    let remaining = (to - next_start).num_days() / 7 + 1;
    already_counted + remaining.max(0) as usize
}

/// All month starts covering the range from..=to.
///
/// The month containing `from` and the month containing `to` are both included.
pub fn monthly_period_starts(
    from: NaiveDate,
    to: NaiveDate,
) -> Result<Vec<NaiveDate>, StatisticsError> {
    if to < from {
        return Err(StatisticsError::InvalidRange);
    }

    let last = get_month_start(to);
    let mut periods = Vec::new();
    let mut current = get_month_start(from);

    while current <= last {
        if periods.len() == MAX_RECALCULATION_PERIODS {
            return Err(StatisticsError::RangeTooLarge(count_months(current, last)
                + periods.len()));
        }
        periods.push(current);
        current = next_month_start(current);
    }

    Ok(periods)
}

/// Number of months from `start` to `last` inclusive — only used for the error message
fn count_months(start: NaiveDate, last: NaiveDate) -> usize {
    let months = (last.year() - start.year()) * 12 + (last.month() as i32 - start.month() as i32);
    (months + 1).max(0) as usize
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

            // Count expected periods within the week (excluding skipped - paused/vacation)
            let expected: i64 = sqlx::query_scalar(
                r#"
                SELECT COUNT(*) FROM task_period_results
                WHERE task_id = ?
                AND period_start >= ? AND period_start <= ?
                AND status != 'skipped'
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

            // Count expected periods within the month (excluding skipped - paused/vacation)
            let expected: i64 = sqlx::query_scalar(
                r#"
                SELECT COUNT(*) FROM task_period_results
                WHERE task_id = ?
                AND period_start >= ? AND period_start <= ?
                AND status != 'skipped'
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

/// Recalculate weekly statistics for every week covering the given range
pub async fn recalculate_weekly_range(
    pool: &SqlitePool,
    household_id: &Uuid,
    from: NaiveDate,
    to: NaiveDate,
    week_start_day: i32,
) -> Result<Vec<NaiveDate>, StatisticsError> {
    let periods = weekly_period_starts(from, to, week_start_day)?;

    for week_start in &periods {
        calculate_weekly_statistics(pool, household_id, *week_start).await?;
    }

    Ok(periods)
}

/// Recalculate monthly statistics for every month covering the given range
pub async fn recalculate_monthly_range(
    pool: &SqlitePool,
    household_id: &Uuid,
    from: NaiveDate,
    to: NaiveDate,
) -> Result<Vec<NaiveDate>, StatisticsError> {
    let periods = monthly_period_starts(from, to)?;

    for month_start in &periods {
        calculate_monthly_statistics(pool, household_id, *month_start).await?;
    }

    Ok(periods)
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

    // ------------------------------------------------------------------
    // Period lists for range recalculation
    // ------------------------------------------------------------------

    fn date(year: i32, month: u32, day: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(year, month, day).unwrap()
    }

    #[test]
    fn test_weekly_period_starts_aligns_to_week_start_day() {
        // Wednesday 2024-01-10 through Tuesday 2024-01-23, weeks start on Monday
        let periods = weekly_period_starts(date(2024, 1, 10), date(2024, 1, 23), 0).unwrap();

        // The partial weeks at both ends are included
        assert_eq!(
            periods,
            vec![date(2024, 1, 8), date(2024, 1, 15), date(2024, 1, 22)]
        );
    }

    #[test]
    fn test_weekly_period_starts_honors_sunday_start() {
        // Same range, but weeks start on Sunday
        let periods = weekly_period_starts(date(2024, 1, 10), date(2024, 1, 23), 6).unwrap();

        assert_eq!(
            periods,
            vec![date(2024, 1, 7), date(2024, 1, 14), date(2024, 1, 21)]
        );
    }

    #[test]
    fn test_weekly_period_starts_single_day_range() {
        let periods = weekly_period_starts(date(2024, 1, 10), date(2024, 1, 10), 0).unwrap();
        assert_eq!(periods, vec![date(2024, 1, 8)]);
    }

    #[test]
    fn test_weekly_period_starts_rejects_inverted_range() {
        let result = weekly_period_starts(date(2024, 1, 23), date(2024, 1, 10), 0);
        assert!(matches!(result, Err(StatisticsError::InvalidRange)));
    }

    #[test]
    fn test_weekly_period_starts_rejects_oversized_range() {
        let from = date(2000, 1, 1);
        let to = from + chrono::Duration::days(7 * MAX_RECALCULATION_PERIODS as i64);

        let result = weekly_period_starts(from, to, 0);
        match result {
            Err(StatisticsError::RangeTooLarge(periods)) => {
                assert!(
                    periods > MAX_RECALCULATION_PERIODS,
                    "reported period count {} should exceed the limit",
                    periods
                );
            }
            other => panic!("expected RangeTooLarge, got {:?}", other),
        }
    }

    #[test]
    fn test_weekly_period_starts_allows_exactly_the_limit() {
        let from = date(2000, 1, 3); // a Monday
        let to = from + chrono::Duration::days(7 * (MAX_RECALCULATION_PERIODS as i64 - 1));

        let periods = weekly_period_starts(from, to, 0).unwrap();
        assert_eq!(periods.len(), MAX_RECALCULATION_PERIODS);
    }

    #[test]
    fn test_monthly_period_starts_covers_partial_months() {
        let periods = monthly_period_starts(date(2024, 1, 20), date(2024, 4, 5)).unwrap();

        assert_eq!(
            periods,
            vec![
                date(2024, 1, 1),
                date(2024, 2, 1),
                date(2024, 3, 1),
                date(2024, 4, 1)
            ]
        );
    }

    #[test]
    fn test_monthly_period_starts_crosses_year_boundary() {
        let periods = monthly_period_starts(date(2024, 11, 15), date(2025, 2, 3)).unwrap();

        assert_eq!(
            periods,
            vec![
                date(2024, 11, 1),
                date(2024, 12, 1),
                date(2025, 1, 1),
                date(2025, 2, 1)
            ]
        );
    }

    #[test]
    fn test_monthly_period_starts_same_month_yields_one_period() {
        let periods = monthly_period_starts(date(2024, 3, 2), date(2024, 3, 29)).unwrap();
        assert_eq!(periods, vec![date(2024, 3, 1)]);
    }

    #[test]
    fn test_monthly_period_starts_rejects_inverted_range() {
        let result = monthly_period_starts(date(2024, 4, 1), date(2024, 3, 1));
        assert!(matches!(result, Err(StatisticsError::InvalidRange)));
    }

    #[test]
    fn test_monthly_period_starts_allows_exactly_the_limit() {
        let from = date(2000, 1, 1);
        let months_after_start = MAX_RECALCULATION_PERIODS as i32 - 1;
        let to = date(
            2000 + months_after_start / 12,
            1 + months_after_start as u32 % 12,
            1,
        );

        let periods = monthly_period_starts(from, to).unwrap();
        assert_eq!(periods.len(), MAX_RECALCULATION_PERIODS);
    }

    #[test]
    fn test_monthly_period_starts_rejects_oversized_range() {
        let from = date(2000, 1, 1);
        // One month beyond the limit
        let months_after_start = MAX_RECALCULATION_PERIODS as i32;
        let to = date(
            2000 + months_after_start / 12,
            1 + months_after_start as u32 % 12,
            1,
        );

        match monthly_period_starts(from, to) {
            Err(StatisticsError::RangeTooLarge(periods)) => {
                assert_eq!(periods, MAX_RECALCULATION_PERIODS + 1);
            }
            other => panic!("expected RangeTooLarge, got {:?}", other),
        }
    }
}

#[cfg(test)]
mod range_recalculation_tests {
    use super::*;
    use crate::test_utils::{
        create_test_household, create_test_membership, create_test_pool, create_test_task,
        create_test_user, insert_period_result,
    };
    use shared::{PeriodStatus, Role};

    fn date(year: i32, month: u32, day: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(year, month, day).unwrap()
    }

    /// A household with one member — statistics are only written for members,
    /// so the membership row is what makes the fixture usable here.
    async fn household_with_member(pool: &SqlitePool) -> (Uuid, Uuid) {
        let household_id = create_test_household(pool).await;
        let user_id = create_test_user(pool, "stats@example.com", Role::Owner).await;
        create_test_membership(pool, &household_id, &user_id, Role::Owner).await;
        (household_id, user_id)
    }

    /// Weeks that were never calculated before must show up after a range recalculation,
    /// which is the whole point of the feature: past periods are reachable without having
    /// had a statistics row already.
    #[tokio::test]
    async fn test_recalculate_weekly_range_creates_rows_for_past_weeks() {
        let pool = create_test_pool().await;
        let (household_id, owner_id) = household_with_member(&pool).await;

        let task = create_test_task(&pool, &household_id)
            .with_assigned_user(owner_id)
            .build()
            .await;

        // Two days completed in the week of 2024-01-08, one failed in the week of 2024-01-15
        insert_period_result(
            &pool,
            &task.id,
            date(2024, 1, 8),
            date(2024, 1, 8),
            PeriodStatus::Completed,
        )
        .await;
        insert_period_result(
            &pool,
            &task.id,
            date(2024, 1, 9),
            date(2024, 1, 9),
            PeriodStatus::Completed,
        )
        .await;
        insert_period_result(
            &pool,
            &task.id,
            date(2024, 1, 16),
            date(2024, 1, 16),
            PeriodStatus::Failed,
        )
        .await;

        let periods = recalculate_weekly_range(
            &pool,
            &household_id,
            date(2024, 1, 10),
            date(2024, 1, 20),
            0,
        )
        .await
        .unwrap();

        assert_eq!(periods, vec![date(2024, 1, 8), date(2024, 1, 15)]);

        let first = get_weekly_statistics(&pool, &household_id, date(2024, 1, 8))
            .await
            .unwrap();
        assert_eq!(first.members.len(), 1);
        assert_eq!(first.members[0].total_expected, 2);
        assert_eq!(first.members[0].total_completed, 2);

        let second = get_weekly_statistics(&pool, &household_id, date(2024, 1, 15))
            .await
            .unwrap();
        assert_eq!(second.members.len(), 1);
        assert_eq!(second.members[0].total_expected, 1);
        assert_eq!(second.members[0].total_completed, 0);

        // Both weeks are now offered as available periods
        let available = list_available_weeks(&pool, &household_id).await.unwrap();
        assert_eq!(available, vec![date(2024, 1, 15), date(2024, 1, 8)]);
    }

    #[tokio::test]
    async fn test_recalculate_monthly_range_covers_every_month() {
        let pool = create_test_pool().await;
        let (household_id, owner_id) = household_with_member(&pool).await;

        let task = create_test_task(&pool, &household_id)
            .with_assigned_user(owner_id)
            .build()
            .await;

        insert_period_result(
            &pool,
            &task.id,
            date(2024, 1, 5),
            date(2024, 1, 5),
            PeriodStatus::Completed,
        )
        .await;
        insert_period_result(
            &pool,
            &task.id,
            date(2024, 3, 5),
            date(2024, 3, 5),
            PeriodStatus::Failed,
        )
        .await;

        let periods =
            recalculate_monthly_range(&pool, &household_id, date(2024, 1, 15), date(2024, 3, 2))
                .await
                .unwrap();

        assert_eq!(
            periods,
            vec![date(2024, 1, 1), date(2024, 2, 1), date(2024, 3, 1)]
        );

        let january = get_monthly_statistics(&pool, &household_id, date(2024, 1, 1))
            .await
            .unwrap();
        assert_eq!(january.members[0].total_completed, 1);

        let march = get_monthly_statistics(&pool, &household_id, date(2024, 3, 1))
            .await
            .unwrap();
        assert_eq!(march.members[0].total_expected, 1);
        assert_eq!(march.members[0].total_completed, 0);

        // February had no period results, so no member row is written for it
        let february = get_monthly_statistics(&pool, &household_id, date(2024, 2, 1))
            .await
            .unwrap();
        assert_eq!(february.members[0].total_expected, 0);
    }

    #[tokio::test]
    async fn test_recalculate_weekly_range_rejects_inverted_range() {
        let pool = create_test_pool().await;
        let (household_id, _) = household_with_member(&pool).await;

        let result = recalculate_weekly_range(
            &pool,
            &household_id,
            date(2024, 2, 1),
            date(2024, 1, 1),
            0,
        )
        .await;

        assert!(matches!(result, Err(StatisticsError::InvalidRange)));
    }
}
