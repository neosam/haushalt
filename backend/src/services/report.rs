//! Daily task report generation.
//!
//! D-01: the report text is generated here, in the backend, and is ALWAYS English —
//! it deliberately bypasses the frontend i18n system because a later phase will feed
//! this text to an LLM.
//! D-02: the layout is fixed and machine-parseable (stable headers, one task per line).
//! D-19: all logic lives in this service so it is unit-testable; the handler stays thin.

use chrono::{DateTime, NaiveDate, Utc};
use sqlx::SqlitePool;
use uuid::Uuid;

use shared::{HouseholdSettings, Task};

use crate::services::{household_settings, households, scheduler, tasks as tasks_service};

/// D-22: "Due today" first, "Missed yesterday" below.
const DUE_TODAY_HEADER: &str = "Due today:";
/// D-23: verbatim, user-chosen empty state.
const DUE_TODAY_EMPTY: &str = "No tasks scheduled for today";
const MISSED_YESTERDAY_HEADER: &str = "Missed yesterday:";
/// D-23: verbatim, user-chosen empty state.
const MISSED_YESTERDAY_EMPTY: &str = "All tasks completed yesterday";

#[derive(Debug, thiserror::Error)]
pub enum ReportError {
    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),
    #[error("Task error: {0}")]
    Task(#[from] crate::services::tasks::TaskError),
    #[error("Settings error: {0}")]
    Settings(#[from] crate::services::household_settings::SettingsError),
    #[error("Household error: {0}")]
    Household(#[from] crate::services::households::HouseholdError),
    #[error("Household not found")]
    HouseholdNotFound,
    #[error("Not a member of this household")]
    NotAMember,
}

/// One rendered task line. Shared by BOTH sections so D-21's `(by HH:MM)` suffix
/// comes from a single formatter (DRY).
#[derive(Debug, Clone, PartialEq)]
struct ReportLine {
    title: String,
    due_time: Option<String>,
    done: bool,
}

/// Generate the daily report for `user_id` in `household_id`.
///
/// `now_utc` is injected by the caller (the handler passes `Utc::now()`) precisely so
/// the date resolution stays testable with a pinned moment.
pub async fn generate_daily_report(
    pool: &SqlitePool,
    household_id: &Uuid,
    user_id: &Uuid,
    now_utc: DateTime<Utc>,
) -> Result<String, ReportError> {
    // T-02-A / T-02-C: the membership guard runs FIRST, before any household name,
    // settings or task data is read, so a non-member can neither read the report nor
    // distinguish "exists but not yours" from "does not exist".
    if !households::is_member(pool, household_id, user_id).await? {
        return Err(ReportError::NotAMember);
    }

    let settings = household_settings::get_or_create_settings(pool, household_id).await?;

    // D-04: "today" is resolved in the household timezone, never UTC.
    // Resolved inline rather than via `scheduler::today_in_timezone`, which hardcodes
    // `Utc::now()` internally and would make this function untestable.
    let tz = scheduler::parse_timezone(&settings.timezone);
    let today = now_utc.with_timezone(&tz).date_naive();
    // D-12: "yesterday" is likewise resolved in the household timezone.
    let yesterday = today - chrono::Duration::days(1);

    let household = households::get_household(pool, household_id)
        .await?
        .ok_or(ReportError::HouseholdNotFound)?;

    let due_today = build_due_today_section(pool, household_id, user_id, today).await?;
    let missed_yesterday =
        build_missed_yesterday_section(pool, household_id, user_id, &settings, yesterday).await?;

    Ok(format_report(
        &household.name,
        today,
        &due_today,
        &missed_yesterday,
    ))
}

/// The "Due today" section (D-03, D-06, D-07, D-13, D-14).
///
/// Deliberate asymmetry with "Missed yesterday": this section does NOT exclude one-time
/// tasks and does NOT apply the vacation check — D-14 scopes both exclusions to the
/// missed-yesterday section only.
async fn build_due_today_section(
    pool: &SqlitePool,
    household_id: &Uuid,
    user_id: &Uuid,
    today: NaiveDate,
) -> Result<Vec<ReportLine>, ReportError> {
    // `list_tasks` already applies D-14's archived filter and the suggestion filter
    // (`archived = 0 AND (suggestion IS NULL OR suggestion = 'approved')`).
    // D-05: the report never calls `tasks::get_due_tasks` — that one resolves "today" in
    // UTC instead of the household timezone.
    let tasks = tasks_service::list_tasks(pool, household_id).await?;

    let mut lines = Vec::new();
    for task in tasks {
        // D-14: paused tasks are excluded everywhere. `list_tasks` does NOT filter this.
        if task.paused {
            continue;
        }
        // D-13 (T-02-B): assigned to me, or unassigned — an unassigned task hits every
        // household member in the background job, so it hits this user too.
        if !task.assigned_user_id.map(|id| id == *user_id).unwrap_or(true) {
            continue;
        }
        // D-06: "due" makes no sense for a habit you are supposed to avoid.
        if task.habit_type.is_inverted() {
            continue;
        }
        // D-03: the due-date decision belongs to the scheduler, reused not reimplemented.
        if !scheduler::is_task_due_on_date(&task, today) {
            continue;
        }

        let done = is_completed_for_today(pool, &task, today).await?;
        lines.push(ReportLine {
            title: task.title,
            due_time: task.due_time,
            done,
        });
    }

    sort_report_lines(&mut lines);
    Ok(lines)
}

/// D-07: a task already completed today stays listed, marked `(done)`.
///
/// Mirrors `tasks::get_task_with_status` exactly. A raw `due_date = today` check would be
/// WRONG: `scheduler::get_next_due_date`'s Weekdays and Custom branches deliberately skip
/// today when today is itself a scheduled occurrence, so those completions are stored with
/// a FUTURE `due_date`.
async fn is_completed_for_today(
    pool: &SqlitePool,
    task: &Task,
    today: NaiveDate,
) -> Result<bool, ReportError> {
    let period_date = scheduler::get_next_due_date(task, today).unwrap_or(today);
    let (period_start, period_end) = scheduler::get_period_bounds(task, period_date);

    let count = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM task_completions WHERE task_id = ? AND due_date >= ? AND due_date <= ?",
    )
    .bind(task.id.to_string())
    .bind(period_start)
    .bind(period_end)
    .fetch_one(pool)
    .await?;

    Ok(count > 0)
}

/// The "Missed yesterday" section.
// Filled in by plan 02.1-03 (D-08/D-09/D-10/D-11/D-14). The signature is the contract.
async fn build_missed_yesterday_section(
    _pool: &SqlitePool,
    _household_id: &Uuid,
    _user_id: &Uuid,
    _settings: &HouseholdSettings,
    _yesterday: NaiveDate,
) -> Result<Vec<ReportLine>, ReportError> {
    Ok(Vec::new())
}

/// D-20/D-22/D-23: the exact, user-approved text shape. Both sections ALWAYS render —
/// there is no combined "everything empty" variant. No trailing newline.
fn format_report(
    household_name: &str,
    today: NaiveDate,
    due_today: &[ReportLine],
    missed_yesterday: &[ReportLine],
) -> String {
    format!(
        "Daily report — {} — {}\n\n{}\n{}\n\n{}\n{}",
        household_name,
        // D-01: `chrono` has no `unstable-locales` here, so `%a` is always English.
        today.format("%a, %Y-%m-%d"),
        DUE_TODAY_HEADER,
        format_section(due_today, DUE_TODAY_EMPTY),
        MISSED_YESTERDAY_HEADER,
        format_section(missed_yesterday, MISSED_YESTERDAY_EMPTY),
    )
}

fn format_section(lines: &[ReportLine], empty_state: &str) -> String {
    if lines.is_empty() {
        return empty_state.to_string();
    }
    lines
        .iter()
        .map(format_report_line)
        .collect::<Vec<_>>()
        .join("\n")
}

/// The ONE line formatter, shared by both sections (D-21 + DRY).
/// `- {title}`, then ` (by {due_time})` when set (D-21), then ` (done)` (D-07).
fn format_report_line(line: &ReportLine) -> String {
    let mut rendered = format!("- {}", line.title);
    if let Some(due_time) = &line.due_time {
        rendered.push_str(&format!(" (by {})", due_time));
    }
    if line.done {
        rendered.push_str(" (done)");
    }
    rendered
}

/// Sort by `due_time` ascending with `None` LAST, then by title case-insensitively.
/// `None` last is explicit: the derived `Option` ordering puts `None` first.
fn sort_report_lines(lines: &mut [ReportLine]) {
    lines.sort_by_key(|line| {
        (
            line.due_time.is_none(),
            line.due_time.clone().unwrap_or_default(),
            line.title.to_lowercase(),
        )
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::*;
    use chrono::TimeZone;
    use shared::{HabitType, RecurrenceType, RecurrenceValue, Role, SuggestionStatus};

    // 2027-01-04 is a Monday, safely in the future of every fixture's `created_at`
    // (`is_task_due_on_date` returns false for dates before a task was created).
    fn pinned_now() -> DateTime<Utc> {
        Utc.with_ymd_and_hms(2027, 1, 4, 12, 0, 0).unwrap()
    }

    fn pinned_today() -> NaiveDate {
        NaiveDate::from_ymd_opt(2027, 1, 4).unwrap()
    }

    fn line(title: &str, due_time: Option<&str>, done: bool) -> ReportLine {
        ReportLine {
            title: title.to_string(),
            due_time: due_time.map(|t| t.to_string()),
            done,
        }
    }

    /// Household + a member user who is the report's caller.
    async fn setup(pool: &SqlitePool) -> (Uuid, Uuid) {
        let household_id = create_test_household(pool).await;
        let user_id = create_test_user(pool, "member@test.com", Role::Member).await;
        create_test_membership(pool, &household_id, &user_id, Role::Member).await;
        (household_id, user_id)
    }

    async fn insert_completion(pool: &SqlitePool, task_id: &Uuid, user_id: &Uuid, due_date: NaiveDate) {
        sqlx::query(
            "INSERT INTO task_completions (id, task_id, user_id, due_date, status) VALUES (?, ?, ?, ?, ?)",
        )
        .bind(Uuid::new_v4().to_string())
        .bind(task_id.to_string())
        .bind(user_id.to_string())
        .bind(due_date)
        .bind("approved")
        .execute(pool)
        .await
        .unwrap();
    }

    // ------------------------------------------------------------------
    // Formatter / layout (D-20, D-21, D-22, D-23, D-01)
    // ------------------------------------------------------------------

    #[test]
    fn test_format_report_both_sections_empty() {
        let today = NaiveDate::from_ymd_opt(2026, 7, 25).unwrap();
        let expected = "Daily report — Test Household — Sat, 2026-07-25\n\
                        \n\
                        Due today:\n\
                        No tasks scheduled for today\n\
                        \n\
                        Missed yesterday:\n\
                        All tasks completed yesterday";

        assert_eq!(format_report("Test Household", today, &[], &[]), expected);
    }

    #[test]
    fn test_format_report_line_without_due_time() {
        assert_eq!(
            format_report_line(&line("Vacuuming", None, false)),
            "- Vacuuming"
        );
    }

    #[test]
    fn test_format_report_line_with_due_time() {
        assert_eq!(
            format_report_line(&line("Clean the litter box", Some("20:00"), false)),
            "- Clean the litter box (by 20:00)"
        );
    }

    #[test]
    fn test_format_report_line_done_marker() {
        assert_eq!(
            format_report_line(&line("Clean the litter box", Some("20:00"), true)),
            "- Clean the litter box (by 20:00) (done)"
        );
    }

    #[test]
    fn test_format_report_header_is_english() {
        let today = NaiveDate::from_ymd_opt(2026, 7, 25).unwrap();
        let report = format_report("Kitchen", today, &[], &[]);
        assert!(
            report.starts_with("Daily report — Kitchen — Sat, 2026-07-25"),
            "got: {report}"
        );
    }

    // ------------------------------------------------------------------
    // Authorization and date resolution (T-02-A, T-02-C, D-04)
    // ------------------------------------------------------------------

    #[tokio::test]
    async fn test_generate_daily_report_rejects_non_member() {
        let pool = create_test_pool().await;
        let household_id = create_test_household(&pool).await;
        let outsider = create_test_user(&pool, "outsider@test.com", Role::Member).await;

        let result = generate_daily_report(&pool, &household_id, &outsider, pinned_now()).await;
        assert!(matches!(result, Err(ReportError::NotAMember)));

        // T-02-C: a household that does not exist yields the SAME variant, so probing
        // cannot distinguish the two cases.
        let unknown_household = Uuid::new_v4();
        let result = generate_daily_report(&pool, &unknown_household, &outsider, pinned_now()).await;
        assert!(matches!(result, Err(ReportError::NotAMember)));
    }

    #[tokio::test]
    async fn test_generate_daily_report_resolves_today_in_household_timezone() {
        let pool = create_test_pool().await;
        let (household_id, user_id) = setup(&pool).await;
        set_household_timezone(&pool, &household_id, "Pacific/Auckland").await;

        // 2026-07-25T12:00:00Z is already 2026-07-26 in Auckland (UTC+12).
        let now = Utc.with_ymd_and_hms(2026, 7, 25, 12, 0, 0).unwrap();
        let report = generate_daily_report(&pool, &household_id, &user_id, now)
            .await
            .unwrap();

        assert!(
            report.starts_with("Daily report — Test Household — Sun, 2026-07-26"),
            "got: {report}"
        );
    }

    #[tokio::test]
    async fn test_report_line_preserves_special_characters() {
        let pool = create_test_pool().await;
        let (household_id, user_id) = setup(&pool).await;
        create_test_task(&pool, &household_id)
            .with_title("<b>Trash</b> - take out")
            .build()
            .await;

        let report = generate_daily_report(&pool, &household_id, &user_id, pinned_now())
            .await
            .unwrap();

        assert!(
            report.contains("- <b>Trash</b> - take out"),
            "got: {report}"
        );
    }

    // ------------------------------------------------------------------
    // "Due today" (D-03, D-06, D-07, D-13, D-14, D-23)
    // ------------------------------------------------------------------

    #[tokio::test]
    async fn test_due_today_lists_task_due_today() {
        let pool = create_test_pool().await;
        let (household_id, user_id) = setup(&pool).await;
        create_test_task(&pool, &household_id)
            .with_title("Empty the dishwasher")
            .with_assigned_user(user_id)
            .build()
            .await;

        let report = generate_daily_report(&pool, &household_id, &user_id, pinned_now())
            .await
            .unwrap();

        assert!(report.contains("- Empty the dishwasher"), "got: {report}");
    }

    #[tokio::test]
    async fn test_due_today_includes_unassigned_task() {
        let pool = create_test_pool().await;
        let (household_id, user_id) = setup(&pool).await;
        create_test_task(&pool, &household_id)
            .with_title("Take out the trash")
            .build()
            .await;

        let report = generate_daily_report(&pool, &household_id, &user_id, pinned_now())
            .await
            .unwrap();

        assert!(report.contains("- Take out the trash"), "got: {report}");
    }

    #[tokio::test]
    async fn test_due_today_excludes_other_users_tasks() {
        let pool = create_test_pool().await;
        let (household_id, user_id) = setup(&pool).await;
        let other_user = create_test_user(&pool, "other@test.com", Role::Member).await;
        create_test_membership(&pool, &household_id, &other_user, Role::Member).await;
        create_test_task(&pool, &household_id)
            .with_title("Not your business")
            .with_assigned_user(other_user)
            .build()
            .await;

        let report = generate_daily_report(&pool, &household_id, &user_id, pinned_now())
            .await
            .unwrap();

        assert!(!report.contains("Not your business"), "got: {report}");
    }

    #[tokio::test]
    async fn test_due_today_excludes_inverted_habit() {
        let pool = create_test_pool().await;
        let (household_id, user_id) = setup(&pool).await;
        create_test_task(&pool, &household_id)
            .with_title("Smoking")
            .with_habit_type(HabitType::Bad)
            .build()
            .await;

        let report = generate_daily_report(&pool, &household_id, &user_id, pinned_now())
            .await
            .unwrap();

        assert!(!report.contains("Smoking"), "got: {report}");
    }

    #[tokio::test]
    async fn test_due_today_excludes_paused_task() {
        let pool = create_test_pool().await;
        let (household_id, user_id) = setup(&pool).await;
        create_test_task(&pool, &household_id)
            .with_title("Paused chore")
            .with_paused(true)
            .build()
            .await;

        let report = generate_daily_report(&pool, &household_id, &user_id, pinned_now())
            .await
            .unwrap();

        assert!(!report.contains("Paused chore"), "got: {report}");
    }

    #[tokio::test]
    async fn test_due_today_excludes_archived_task() {
        let pool = create_test_pool().await;
        let (household_id, user_id) = setup(&pool).await;
        create_test_task(&pool, &household_id)
            .with_title("Archived chore")
            .with_archived(true)
            .build()
            .await;

        let report = generate_daily_report(&pool, &household_id, &user_id, pinned_now())
            .await
            .unwrap();

        assert!(!report.contains("Archived chore"), "got: {report}");
    }

    #[tokio::test]
    async fn test_due_today_excludes_pending_suggestion() {
        let pool = create_test_pool().await;
        let (household_id, user_id) = setup(&pool).await;
        create_test_task(&pool, &household_id)
            .with_title("Merely suggested")
            .with_suggestion(SuggestionStatus::Suggested)
            .build()
            .await;
        create_test_task(&pool, &household_id)
            .with_title("Already approved")
            .with_suggestion(SuggestionStatus::Approved)
            .build()
            .await;

        let report = generate_daily_report(&pool, &household_id, &user_id, pinned_now())
            .await
            .unwrap();

        assert!(!report.contains("Merely suggested"), "got: {report}");
        assert!(report.contains("- Already approved"), "got: {report}");
    }

    #[tokio::test]
    async fn test_due_today_excludes_task_not_due_today() {
        let pool = create_test_pool().await;
        let (household_id, user_id) = setup(&pool).await;
        // Weekly on Wednesday (3); the pinned today is a Monday.
        create_test_task(&pool, &household_id)
            .with_title("Wednesday only")
            .with_recurrence(RecurrenceType::Weekly)
            .with_recurrence_value(RecurrenceValue::WeekDay(3))
            .build()
            .await;

        let report = generate_daily_report(&pool, &household_id, &user_id, pinned_now())
            .await
            .unwrap();

        assert!(!report.contains("Wednesday only"), "got: {report}");
    }

    #[tokio::test]
    async fn test_due_today_marks_completed_task_done() {
        let pool = create_test_pool().await;
        let (household_id, user_id) = setup(&pool).await;
        let task = create_test_task(&pool, &household_id)
            .with_title("Empty the dishwasher")
            .build()
            .await;
        insert_completion(&pool, &task.id, &user_id, pinned_today()).await;

        let report = generate_daily_report(&pool, &household_id, &user_id, pinned_now())
            .await
            .unwrap();

        assert!(
            report.contains("- Empty the dishwasher (done)"),
            "got: {report}"
        );
    }

    #[tokio::test]
    async fn test_due_today_marks_weekdays_task_done_via_period_bounds() {
        let pool = create_test_pool().await;
        let (household_id, user_id) = setup(&pool).await;
        let task = create_test_task(&pool, &household_id)
            .with_title("Weekday workout")
            .with_recurrence(RecurrenceType::Weekdays)
            .build()
            .await;

        // `get_next_due_date` SKIPS today for Weekdays when today is scheduled, so a
        // completion made today is stored with next Monday's due_date. A naive
        // `due_date = today` check would miss it.
        let next_occurrence = NaiveDate::from_ymd_opt(2027, 1, 11).unwrap();
        insert_completion(&pool, &task.id, &user_id, next_occurrence).await;

        let report = generate_daily_report(&pool, &household_id, &user_id, pinned_now())
            .await
            .unwrap();

        assert!(
            report.contains("- Weekday workout (done)"),
            "got: {report}"
        );
    }

    #[tokio::test]
    async fn test_due_today_sorted_by_due_time_then_title() {
        let pool = create_test_pool().await;
        let (household_id, user_id) = setup(&pool).await;
        for (title, due_time) in [
            ("Zebra task", Some("08:00")),
            ("apple task", Some("08:00")),
            ("Beta task", Some("20:00")),
            ("Middle task", None),
        ] {
            let mut builder = create_test_task(&pool, &household_id).with_title(title);
            if let Some(time) = due_time {
                builder = builder.with_due_time(time);
            }
            builder.build().await;
        }

        let report = generate_daily_report(&pool, &household_id, &user_id, pinned_now())
            .await
            .unwrap();

        let due_section: Vec<&str> = report
            .lines()
            .skip_while(|l| *l != DUE_TODAY_HEADER)
            .skip(1)
            .take(4)
            .collect();

        assert_eq!(
            due_section,
            vec![
                "- apple task (by 08:00)",
                "- Zebra task (by 08:00)",
                "- Beta task (by 20:00)",
                "- Middle task",
            ],
            "got: {report}"
        );
    }

    #[tokio::test]
    async fn test_due_today_empty_renders_empty_state() {
        let pool = create_test_pool().await;
        let (household_id, user_id) = setup(&pool).await;
        create_test_task(&pool, &household_id)
            .with_title("Wednesday only")
            .with_recurrence(RecurrenceType::Weekly)
            .with_recurrence_value(RecurrenceValue::WeekDay(3))
            .build()
            .await;

        let report = generate_daily_report(&pool, &household_id, &user_id, pinned_now())
            .await
            .unwrap();

        assert!(
            report.contains("Due today:\nNo tasks scheduled for today"),
            "got: {report}"
        );
    }
}
