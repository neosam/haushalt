//! Daily task report generation.
//!
//! D-01: the report text is generated here, in the backend, and is ALWAYS English —
//! it deliberately bypasses the frontend i18n system because a later phase will feed
//! this text to an LLM.
//! D-02: the layout is fixed and machine-parseable (stable headers, one task per line).
//! D-19: all logic lives in this service so it is unit-testable; the handler stays thin.

use chrono::{DateTime, Datelike, NaiveDate, Utc};
use sqlx::SqlitePool;
use uuid::Uuid;

use shared::{HouseholdSettings, RecurrenceType, SuggestionStatus, Task};

use crate::services::{household_settings, households, scheduler, tasks as tasks_service};

/// D-22: "Due today" first, "Missed yesterday" below.
const DUE_TODAY_HEADER: &str = "Due today:";
/// D-23: verbatim, user-chosen empty state.
const DUE_TODAY_EMPTY: &str = "No tasks scheduled for today";
const MISSED_YESTERDAY_HEADER: &str = "Missed yesterday:";
/// D-23: verbatim, user-chosen empty state.
const MISSED_YESTERDAY_EMPTY: &str = "All tasks completed yesterday";
/// Optional section (per-report switch) for OneTime / free-form tasks that have no real
/// due date. English defaults; the German variants live in `ReportStrings`.
const UNDATED_HEADER: &str = "No fixed date:";
const UNDATED_EMPTY: &str = "No undated tasks";

/// Phase 6 D-06: the output language of a rendered report.
///
/// This narrows Phase 2.1's D-01 ("the report is always English") rather than replacing it:
/// `GET /api/households/{id}/report` still emits English, so its existing tests and any later
/// LLM consumer are unaffected. Only the public cross-household reports of Phase 6 choose.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ReportLanguage {
    #[default]
    En,
    De,
}

impl ReportLanguage {
    /// Parse a language code from storage or an API request.
    ///
    /// An unknown code falls back to English instead of failing: `user_settings` treats
    /// English as its default too, and a report that renders in the wrong language is far
    /// less harmful than one that does not render at all.
    pub fn from_code(code: &str) -> Self {
        match code {
            "de" => Self::De,
            _ => Self::En,
        }
    }

    pub fn code(self) -> &'static str {
        match self {
            Self::En => "en",
            Self::De => "de",
        }
    }

    fn strings(self) -> ReportStrings {
        match self {
            Self::En => ReportStrings {
                title: "Daily report",
                due_today_header: DUE_TODAY_HEADER,
                due_today_empty: DUE_TODAY_EMPTY,
                undated_header: UNDATED_HEADER,
                undated_empty: UNDATED_EMPTY,
                missed_yesterday_header: MISSED_YESTERDAY_HEADER,
                missed_yesterday_empty: MISSED_YESTERDAY_EMPTY,
                by_prefix: "by",
                done_marker: "done",
                open_marker: "open",
                // Exactly what `chrono`'s `%a` produced before this struct existed, so the
                // English output is byte-for-byte unchanged.
                weekdays: ["Mon", "Tue", "Wed", "Thu", "Fri", "Sat", "Sun"],
            },
            Self::De => ReportStrings {
                title: "Tagesbericht",
                due_today_header: "Heute fällig:",
                due_today_empty: "Keine Aufgaben für heute geplant",
                undated_header: "Ohne festen Termin:",
                undated_empty: "Keine terminlosen Aufgaben",
                missed_yesterday_header: "Gestern verpasst:",
                missed_yesterday_empty: "Gestern wurden alle Aufgaben erledigt",
                by_prefix: "bis",
                done_marker: "erledigt",
                open_marker: "offen",
                weekdays: ["Mo", "Di", "Mi", "Do", "Fr", "Sa", "So"],
            },
        }
    }
}

/// What a caller may vary about a rendered report.
///
/// A struct rather than more parameters: the knobs are booleans-with-a-name from the call
/// site's point of view, and `Default` keeps the per-household endpoint's behaviour pinned
/// in one place instead of spelled out at every call.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ReportOptions {
    /// Phase 6 D-06.
    pub language: ReportLanguage,
    /// Whether the "Missed yesterday" section is rendered at all.
    ///
    /// When false the report stops after "Due today" — the header does not appear with an
    /// empty body, because a section nobody asked for should leave no trace.
    pub include_missed: bool,
    /// Whether OneTime / free-form ("no fixed date") tasks are pulled out of "Due today"
    /// into their own section. Off by default, so the per-household endpoint's output is
    /// unchanged — such tasks keep appearing under "Due today" exactly as before.
    pub separate_undated: bool,
}

impl Default for ReportOptions {
    fn default() -> Self {
        // D-01/D-20: English, both sections, undated tasks mixed into "Due today" — what the
        // per-household endpoint has always done.
        Self {
            language: ReportLanguage::En,
            include_missed: true,
            separate_undated: false,
        }
    }
}

impl ReportOptions {
    pub fn new(language: ReportLanguage, include_missed: bool) -> Self {
        Self {
            language,
            include_missed,
            separate_undated: false,
        }
    }

    /// Builder knob for the per-report "undated tasks in their own section" switch.
    pub fn with_separate_undated(mut self, separate_undated: bool) -> Self {
        self.separate_undated = separate_undated;
        self
    }
}

/// Every piece of language-dependent text in a report, resolved once per render.
///
/// `chrono` is built without `unstable-locales` here, so weekday names cannot come from
/// `%a` for anything but English — they are carried explicitly, Monday first.
struct ReportStrings {
    title: &'static str,
    due_today_header: &'static str,
    due_today_empty: &'static str,
    undated_header: &'static str,
    undated_empty: &'static str,
    missed_yesterday_header: &'static str,
    missed_yesterday_empty: &'static str,
    by_prefix: &'static str,
    done_marker: &'static str,
    open_marker: &'static str,
    weekdays: [&'static str; 7],
}

impl ReportStrings {
    fn weekday(&self, date: NaiveDate) -> &'static str {
        self.weekdays[date.weekday().num_days_from_monday() as usize]
    }
}

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

/// Whether a section spells out that an unfinished task is still outstanding.
///
/// "Missed yesterday" contains nothing BUT outstanding tasks, so an `(open)` on every one
/// of its lines would only repeat the header. The sections describing today's state —
/// "Due today" and "No fixed date" — mix finished and unfinished tasks, so there the
/// marker is what tells the two apart.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum OpenMarker {
    Show,
    Hide,
}

/// One rendered task line. Shared by BOTH sections so D-21's `(by HH:MM)` suffix
/// comes from a single formatter (DRY).
#[derive(Debug, Clone, PartialEq)]
struct ReportLine {
    title: String,
    due_time: Option<String>,
    /// Whether the task's target is met. `false` renders as `(open)` in the sections that
    /// carry [`OpenMarker::Show`], and as nothing at all in the missed section.
    done: bool,
    /// `Some((completed, target))` for a task that must be done more than once in its
    /// period, e.g. `(5, 8)` renders as `(5/8)`. `None` for a plain single-completion
    /// task, which falls back to the `(done)` marker. The completed count is clamped to
    /// the target, so an over-fulfilled task reads as `8/8`, never `10/8`.
    progress: Option<(i64, i64)>,
}

/// Generate the daily report for `user_id` in `household_id`, in English, both sections.
///
/// D-01: the per-household report is always English, and this signature keeps it that way —
/// callers cannot accidentally localize it. Phase 6's public reports call
/// [`generate_daily_report_with`] instead.
///
/// `now_utc` is injected by the caller (the handler passes `Utc::now()`) precisely so
/// the date resolution stays testable with a pinned moment.
pub async fn generate_daily_report(
    pool: &SqlitePool,
    household_id: &Uuid,
    user_id: &Uuid,
    now_utc: DateTime<Utc>,
) -> Result<String, ReportError> {
    generate_daily_report_with(
        pool,
        household_id,
        user_id,
        now_utc,
        ReportOptions::default(),
    )
    .await
}

/// Generate the daily report for `user_id` in `household_id` under `options` (Phase 6).
pub async fn generate_daily_report_with(
    pool: &SqlitePool,
    household_id: &Uuid,
    user_id: &Uuid,
    now_utc: DateTime<Utc>,
    options: ReportOptions,
) -> Result<String, ReportError> {
    // T-02-A / T-02-C: the membership guard runs FIRST, before any household name,
    // settings or task data is read, so a non-member can neither read the report nor
    // distinguish "exists but not yours" from "does not exist".
    if !households::is_member(pool, household_id, user_id).await? {
        return Err(ReportError::NotAMember);
    }

    let settings = household_settings::get_or_create_settings(pool, household_id).await?;

    // D-04: "today" is resolved in the household timezone, never UTC.
    // Resolved inline on purpose: the scheduler's convenience helper for the local date
    // hardcodes `Utc::now()` internally, which would make this function untestable with a
    // pinned moment.
    let tz = scheduler::parse_timezone(&settings.timezone);
    let today = now_utc.with_timezone(&tz).date_naive();
    // D-12: "yesterday" is likewise resolved in the household timezone.
    let yesterday = today - chrono::Duration::days(1);

    let household = households::get_household(pool, household_id)
        .await?
        .ok_or(ReportError::HouseholdNotFound)?;

    let (due_today, undated) =
        build_due_today_section(pool, household_id, user_id, today, options.separate_undated)
            .await?;
    // The undated section is rendered only when asked for; off leaves no trace, exactly like
    // the missed section. When off, `undated` is empty anyway (nothing was routed there).
    let undated_section = options.separate_undated.then_some(undated);

    // Skipped entirely when switched off — the section costs two queries per household,
    // and a report that will not print it has no reason to pay for them.
    let missed_yesterday = if options.include_missed {
        Some(build_missed_yesterday_section(pool, household_id, user_id, &settings, yesterday).await?)
    } else {
        None
    };

    Ok(format_report(
        &household.name,
        today,
        &due_today,
        undated_section.as_deref(),
        missed_yesterday.as_deref(),
        &options.language.strings(),
    ))
}

/// The "Due today" section (D-03, D-06, D-07, D-13, D-14), returned as `(due_today, undated)`.
///
/// Deliberate asymmetry with "Missed yesterday": this section does NOT apply the vacation
/// check — D-14 scopes that exclusion to the missed-yesterday section only.
///
/// One-time / free-form tasks (`RecurrenceType::OneTime`) have no real due date —
/// `is_task_due_on_date` treats them as always due. When `separate_undated` is set they are
/// routed into the second vec (the "No fixed date" section) instead of "Due today"; when it
/// is not, the second vec stays empty and everything lands in the first, exactly as before.
async fn build_due_today_section(
    pool: &SqlitePool,
    household_id: &Uuid,
    user_id: &Uuid,
    today: NaiveDate,
    separate_undated: bool,
) -> Result<(Vec<ReportLine>, Vec<ReportLine>), ReportError> {
    // `list_tasks` already applies D-14's archived filter and the suggestion filter
    // (`archived = 0 AND (suggestion IS NULL OR suggestion = 'approved')`).
    // D-05: the report deliberately does NOT reuse the existing due-tasks endpoint's
    // query, which resolves "today" in UTC instead of the household timezone. That
    // pre-existing inaccuracy stays untouched; the report computes its own due-today set.
    let tasks = tasks_service::list_tasks(pool, household_id).await?;

    let mut due_today = Vec::new();
    let mut undated = Vec::new();
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

        let completed = completions_this_period(pool, &task, today).await?;
        let target = i64::from(task.target_count);
        // D-07: "done" means the target is met; target_count 0 is free-form, never done.
        let done = target > 0 && completed >= target;
        // A task that must be done more than once shows its progress instead of a plain
        // "done" marker; the count is clamped so over-fulfilment reads as `8/8`, not `10/8`.
        let progress = (task.target_count > 1).then(|| (completed.min(target), target));
        let is_undated = task.recurrence_type == RecurrenceType::OneTime;
        let line = ReportLine {
            title: task.title,
            due_time: task.due_time,
            done,
            progress,
        };

        if separate_undated && is_undated {
            undated.push(line);
        } else {
            due_today.push(line);
        }
    }

    sort_report_lines(&mut due_today);
    sort_report_lines(&mut undated);
    Ok((due_today, undated))
}

/// How many times `task` has been completed within the period that covers `today`.
///
/// Returning the raw count (rather than a bool) lets the caller both decide "done" —
/// the target is met, not merely touched once: a task with `target_count = 3` and one
/// completion is still outstanding — and render the `(X/N)` progress of a
/// multi-completion task. The "done" decision mirrors `TaskWithStatus::is_target_met`
/// in the shared crate, including its treatment of `target_count = 0` as free-form
/// (never "met").
///
/// `get_next_due_date` returns `Some(today)` whenever `is_task_due_on_date(task, today)`
/// holds — and the caller above has already checked exactly that. The call stays anyway: it
/// also catches OneTime tasks (`None` falls back to `today`) and keeps the derivation of the
/// period in one place.
///
/// The period bounds themselves remain indispensable for a different, still valid reason:
/// Weekly and Monthly tasks count over a whole week or month, so a raw `due_date = today`
/// check would lose their completions.
async fn completions_this_period(
    pool: &SqlitePool,
    task: &Task,
    today: NaiveDate,
) -> Result<i64, ReportError> {
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

    Ok(count)
}

/// The "Missed yesterday" section — "everything that went wrong yesterday", from TWO
/// sources merged into ONE unlabelled section (D-11):
/// 1. good habits the system actually penalized (D-08, read from `missed_task_penalties`)
/// 2. bad habits that were indulged (D-10, read from `task_completions`)
async fn build_missed_yesterday_section(
    pool: &SqlitePool,
    household_id: &Uuid,
    user_id: &Uuid,
    settings: &HouseholdSettings,
    yesterday: NaiveDate,
) -> Result<Vec<ReportLine>, ReportError> {
    // D-14: the vacation exclusion covers the WHOLE section, not just the penalty half.
    // `process_missed_tasks` already skips writing penalty rows during vacation, so the
    // D-08 half is naturally vacation-clean — but `complete_task` never checks vacation
    // mode, so the D-10 half needs this guard.
    // This deliberately checks `yesterday`, while `background_jobs::process_missed_tasks`
    // checks `today_local`: that job runs at processing time, whereas this report is
    // describing yesterday.
    if household_settings::is_household_on_vacation(settings, yesterday) {
        return Ok(Vec::new());
    }

    let mut lines = Vec::new();

    // ---- D-08: penalized good habits, read from materialized state ----
    // Never recomputed live — the report shows what the system actually penalized.
    // `missed_task_penalties` carries neither `household_id` nor `user_id` (D-15), so this
    // query is intentionally broad and every filter below is applied on the resolved Task.
    let penalized_task_ids =
        sqlx::query_scalar::<_, String>("SELECT task_id FROM missed_task_penalties WHERE due_date = ?")
            .bind(yesterday)
            .fetch_all(pool)
            .await?;

    for raw_task_id in penalized_task_ids {
        let Ok(task_id) = Uuid::parse_str(&raw_task_id) else {
            continue;
        };
        let Some(task) = tasks_service::get_task(pool, &task_id).await? else {
            continue;
        };

        // T-03-B: the household scope the source table cannot provide.
        if task.household_id != *household_id {
            continue;
        }
        // D-14 (T-03-C): stale penalty rows survive archiving/pausing, so filter here.
        if task.archived || task.paused {
            continue;
        }
        // D-14: required because `process_missed_tasks` does NOT filter by suggestion.
        let suggestion_ok = task.suggestion.is_none()
            || task.suggestion == Some(SuggestionStatus::Approved);
        if !suggestion_ok {
            continue;
        }
        // D-14: one-time tasks are excluded from the missed section only.
        if task.recurrence_type == RecurrenceType::OneTime {
            continue;
        }
        // D-09: for inverted habits the background job awards a reward, not a penalty,
        // so any penalty row belonging to one does not mean "missed".
        if task.habit_type.is_inverted() {
            continue;
        }
        // D-13 (T-03-A): assigned to me, or unassigned.
        if !task.assigned_user_id.map(|id| id == *user_id).unwrap_or(true) {
            continue;
        }

        // `done` is always false here: a "done" marker on something you missed would be
        // nonsense, so the shared formatter emits nothing extra. `progress` is likewise
        // omitted — the missed section reports what went wrong, not how far you got.
        lines.push(ReportLine {
            title: task.title,
            due_time: task.due_time,
            done: false,
            progress: None,
        });
    }

    // ---- D-10: inverted habits that WERE performed yesterday ----
    // D-10 is worded as `task_completions.due_date = yesterday` and implemented exactly as
    // worded. Remaining edge: a Weekdays/Custom habit indulged on a day that is NOT a
    // scheduled occurrence is still stored against its next scheduled date, so it only
    // surfaces in the report after that date. That is the same deliberate shift early
    // completion relies on — intended, not broken.
    let indulged = sqlx::query_as::<_, (String, Option<String>)>(
        r#"
        SELECT t.title, t.due_time
        FROM task_completions tc
        JOIN tasks t ON tc.task_id = t.id
        WHERE tc.due_date = ?
          AND t.household_id = ?
          AND t.habit_type = 'bad'
          AND t.archived = 0 AND t.paused = 0
          AND (t.suggestion IS NULL OR t.suggestion = 'approved')
          AND t.recurrence_type != 'onetime'
          AND (t.assigned_user_id = ? OR t.assigned_user_id IS NULL)
        "#,
    )
    .bind(yesterday)
    .bind(household_id.to_string())
    .bind(user_id.to_string())
    .fetch_all(pool)
    .await?;

    for (title, due_time) in indulged {
        lines.push(ReportLine {
            title,
            due_time,
            done: false,
            progress: None,
        });
    }

    // A task could theoretically contribute from both sources — never print it twice.
    let mut seen = std::collections::HashSet::new();
    lines.retain(|line| seen.insert((line.title.clone(), line.due_time.clone())));

    sort_report_lines(&mut lines);
    Ok(lines)
}

/// D-20/D-22/D-23: the exact, user-approved text shape. "Due today" always renders; the
/// other two sections are optional. No trailing newline.
///
/// Both `undated` and `missed_yesterday` follow the same convention: `None` switches the
/// section off and renders nothing, while `Some(&[])` renders the header with its empty-state
/// line. The undated section sits between "Due today" and "Missed yesterday".
fn format_report(
    household_name: &str,
    today: NaiveDate,
    due_today: &[ReportLine],
    undated: Option<&[ReportLine]>,
    missed_yesterday: Option<&[ReportLine]>,
    strings: &ReportStrings,
) -> String {
    let head = format!(
        "{} — {} — {}, {}\n\n{}\n{}",
        strings.title,
        household_name,
        // The weekday comes from `ReportStrings`, not `%a`: `chrono` has no
        // `unstable-locales` here, so `%a` can only ever produce English.
        strings.weekday(today),
        today.format("%Y-%m-%d"),
        strings.due_today_header,
        format_section(due_today, strings.due_today_empty, strings, OpenMarker::Show),
    );

    let with_undated = match undated {
        Some(lines) => format!(
            "{head}\n\n{}\n{}",
            strings.undated_header,
            format_section(lines, strings.undated_empty, strings, OpenMarker::Show),
        ),
        None => head,
    };

    match missed_yesterday {
        Some(lines) => format!(
            "{with_undated}\n\n{}\n{}",
            strings.missed_yesterday_header,
            format_section(
                lines,
                strings.missed_yesterday_empty,
                strings,
                OpenMarker::Hide,
            ),
        ),
        None => with_undated,
    }
}

fn format_section(
    lines: &[ReportLine],
    empty_state: &str,
    strings: &ReportStrings,
    open_marker: OpenMarker,
) -> String {
    if lines.is_empty() {
        return empty_state.to_string();
    }
    lines
        .iter()
        .map(|line| format_report_line(line, strings, open_marker))
        .collect::<Vec<_>>()
        .join("\n")
}

/// The ONE line formatter, shared by both sections (D-21 + DRY).
/// `- {title}`, then ` (by {due_time})` when set (D-21), then either ` (X/N)` for a
/// multi-completion task or ` (done)` for a plain one (D-07).
/// An unfinished task ends in ` (open)` where `open_marker` asks for it — exactly one
/// status suffix is ever appended.
fn format_report_line(
    line: &ReportLine,
    strings: &ReportStrings,
    open_marker: OpenMarker,
) -> String {
    let mut rendered = format!("- {}", line.title);
    if let Some(due_time) = &line.due_time {
        rendered.push_str(&format!(" ({} {})", strings.by_prefix, due_time));
    }
    // A multi-completion task shows `(X/N)` in place of the `(done)`/`(open)` markers: the
    // count already says how far along it is, and reads the same in any language.
    if let Some((completed, target)) = line.progress {
        rendered.push_str(&format!(" ({completed}/{target})"));
    } else if line.done {
        rendered.push_str(&format!(" ({})", strings.done_marker));
    } else if open_marker == OpenMarker::Show {
        // A bare line used to mean "not done" only through the ABSENCE of a marker —
        // spelled out here so the text says so, for a reader and a later LLM alike.
        rendered.push_str(&format!(" ({})", strings.open_marker));
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
            progress: None,
        }
    }

    fn line_with_progress(
        title: &str,
        due_time: Option<&str>,
        completed: i64,
        target: i64,
    ) -> ReportLine {
        ReportLine {
            title: title.to_string(),
            due_time: due_time.map(|t| t.to_string()),
            done: completed >= target,
            progress: Some((completed, target)),
        }
    }

    fn en() -> ReportStrings {
        ReportLanguage::En.strings()
    }

    fn de() -> ReportStrings {
        ReportLanguage::De.strings()
    }

    /// Household + a member user who is the report's caller.
    async fn setup(pool: &SqlitePool) -> (Uuid, Uuid) {
        let household_id = create_test_household(pool).await;
        let user_id = create_test_user(pool, "member@test.com", Role::Member).await;
        create_test_membership(pool, &household_id, &user_id, Role::Member).await;
        (household_id, user_id)
    }

    fn pinned_yesterday() -> NaiveDate {
        NaiveDate::from_ymd_opt(2027, 1, 3).unwrap()
    }

    /// Everything printed under the `Missed yesterday:` header (it is the last section).
    fn missed_section(report: &str) -> Vec<String> {
        report
            .lines()
            .skip_while(|l| *l != MISSED_YESTERDAY_HEADER)
            .skip(1)
            .map(|l| l.to_string())
            .collect()
    }

    fn missed_contains(report: &str, needle: &str) -> bool {
        missed_section(report).iter().any(|l| l.contains(needle))
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

        assert_eq!(
            format_report("Test Household", today, &[], None, Some(&[]), &en()),
            expected
        );
    }

    #[test]
    fn test_format_report_line_without_due_time() {
        assert_eq!(
            format_report_line(&line("Vacuuming", None, false), &en(), OpenMarker::Show),
            "- Vacuuming (open)"
        );
    }

    #[test]
    fn test_format_report_line_with_due_time() {
        assert_eq!(
            format_report_line(
                &line("Clean the litter box", Some("20:00"), false),
                &en(),
                OpenMarker::Show,
            ),
            "- Clean the litter box (by 20:00) (open)"
        );
    }

    #[test]
    fn test_format_report_line_done_marker() {
        assert_eq!(
            format_report_line(
                &line("Clean the litter box", Some("20:00"), true),
                &en(),
                OpenMarker::Show,
            ),
            "- Clean the litter box (by 20:00) (done)"
        );
    }

    /// `OpenMarker::Hide` is what the missed section passes: an unfinished task there ends
    /// after the due time, with no status suffix at all.
    #[test]
    fn test_format_report_line_open_marker_can_be_suppressed() {
        assert_eq!(
            format_report_line(
                &line("Clean the litter box", Some("20:00"), false),
                &en(),
                OpenMarker::Hide,
            ),
            "- Clean the litter box (by 20:00)"
        );
        assert_eq!(
            format_report_line(&line("Katzenklo", None, false), &de(), OpenMarker::Hide),
            "- Katzenklo"
        );
    }

    #[test]
    fn test_format_report_line_shows_progress() {
        assert_eq!(
            format_report_line(
                &line_with_progress("Drink water", None, 5, 8),
                &en(),
                OpenMarker::Show,
            ),
            "- Drink water (5/8)"
        );
    }

    #[test]
    fn test_format_report_line_progress_after_due_time() {
        assert_eq!(
            format_report_line(
                &line_with_progress("Drink water", Some("20:00"), 5, 8),
                &en(),
                OpenMarker::Show,
            ),
            "- Drink water (by 20:00) (5/8)"
        );
    }

    /// An unmet counter already says the task is outstanding — it must NOT also collect
    /// an `(open)`/`(offen)`, in either language.
    #[test]
    fn test_format_report_line_progress_replaces_the_open_marker() {
        let unmet = line_with_progress("Drink water", None, 5, 8);
        assert_eq!(
            format_report_line(&unmet, &en(), OpenMarker::Show),
            "- Drink water (5/8)"
        );
        assert_eq!(
            format_report_line(&unmet, &de(), OpenMarker::Show),
            "- Drink water (5/8)"
        );
    }

    /// A met target renders as `N/N`, never with an extra `(done)`/`(erledigt)`, and the
    /// counter is byte-identical in both languages — `5/8` needs no translation.
    #[test]
    fn test_format_report_line_progress_replaces_done_marker_in_both_languages() {
        let met = line_with_progress("Drink water", None, 8, 8);
        let english = format_report_line(&met, &en(), OpenMarker::Show);
        let german = format_report_line(&met, &de(), OpenMarker::Show);
        assert_eq!(english, "- Drink water (8/8)");
        assert_eq!(german, "- Drink water (8/8)");
        assert!(!english.contains("done"));
        assert!(!german.contains("erledigt"));
    }

    #[test]
    fn test_format_report_header_is_english() {
        let today = NaiveDate::from_ymd_opt(2026, 7, 25).unwrap();
        let report = format_report("Kitchen", today, &[], None, Some(&[]), &en());
        assert!(
            report.starts_with("Daily report — Kitchen — Sat, 2026-07-25"),
            "got: {report}"
        );
    }

    // ------------------------------------------------------------------
    // Phase 6 D-06: per-report language
    // ------------------------------------------------------------------

    #[test]
    fn test_report_language_from_code() {
        assert_eq!(ReportLanguage::from_code("de"), ReportLanguage::De);
        assert_eq!(ReportLanguage::from_code("en"), ReportLanguage::En);
        // Unknown codes must not fail a render — they fall back to English.
        assert_eq!(ReportLanguage::from_code("fr"), ReportLanguage::En);
        assert_eq!(ReportLanguage::from_code(""), ReportLanguage::En);
        assert_eq!(ReportLanguage::default(), ReportLanguage::En);
    }

    #[test]
    fn test_report_language_code_round_trips() {
        for language in [ReportLanguage::En, ReportLanguage::De] {
            assert_eq!(ReportLanguage::from_code(language.code()), language);
        }
    }

    #[test]
    fn test_format_report_german_empty_sections() {
        let today = NaiveDate::from_ymd_opt(2026, 7, 25).unwrap();
        let expected = "Tagesbericht — Küche — Sa, 2026-07-25\n\
                        \n\
                        Heute fällig:\n\
                        Keine Aufgaben für heute geplant\n\
                        \n\
                        Gestern verpasst:\n\
                        Gestern wurden alle Aufgaben erledigt";

        assert_eq!(format_report("Küche", today, &[], None, Some(&[]), &de()), expected);
    }

    #[test]
    fn test_format_report_line_german() {
        assert_eq!(
            format_report_line(
                &line("Katzenklo", Some("20:00"), true),
                &de(),
                OpenMarker::Show,
            ),
            "- Katzenklo (bis 20:00) (erledigt)"
        );
    }

    #[test]
    fn test_format_report_line_open_marker_german() {
        assert_eq!(
            format_report_line(
                &line("Katzenklo", Some("20:00"), false),
                &de(),
                OpenMarker::Show,
            ),
            "- Katzenklo (bis 20:00) (offen)"
        );
    }

    /// Every weekday must map to the right abbreviation in both languages — an off-by-one
    /// in `num_days_from_monday` would otherwise only show up on one day of the week.
    #[test]
    fn test_weekday_abbreviations_cover_the_whole_week() {
        // 2026-07-20 is a Monday.
        let monday = NaiveDate::from_ymd_opt(2026, 7, 20).unwrap();
        let english = ["Mon", "Tue", "Wed", "Thu", "Fri", "Sat", "Sun"];
        let german = ["Mo", "Di", "Mi", "Do", "Fr", "Sa", "So"];

        for offset in 0..7 {
            let date = monday + chrono::Duration::days(offset);
            assert_eq!(en().weekday(date), english[offset as usize], "{date}");
            assert_eq!(de().weekday(date), german[offset as usize], "{date}");
        }
    }

    #[test]
    fn test_report_options_default_matches_the_household_endpoint() {
        let options = ReportOptions::default();
        assert_eq!(options.language, ReportLanguage::En);
        assert!(options.include_missed);
        // Off by default: undated tasks stay mixed into "Due today", as they always have.
        assert!(!options.separate_undated);
        assert!(!ReportOptions::new(ReportLanguage::De, false).separate_undated);
        assert!(
            ReportOptions::new(ReportLanguage::De, false)
                .with_separate_undated(true)
                .separate_undated
        );
    }

    #[test]
    fn test_format_report_renders_the_undated_section() {
        let today = NaiveDate::from_ymd_opt(2026, 7, 25).unwrap();
        let report = format_report(
            "Kitchen",
            today,
            &[line("Wash up", None, false)],
            Some(&[line("Fix the shelf", None, false)]),
            None,
            &en(),
        );
        assert!(
            report.contains("Due today:\n- Wash up"),
            "dated task stays under Due today, got: {report}"
        );
        assert!(
            report.contains("No fixed date:\n- Fix the shelf"),
            "undated task under its own header, got: {report}"
        );
    }

    /// The two sections describing today's state both mark what is still outstanding; the
    /// missed section does not, because every line in it is outstanding by definition.
    #[test]
    fn test_open_marker_covers_today_but_not_the_missed_section() {
        let today = NaiveDate::from_ymd_opt(2026, 7, 25).unwrap();
        let report = format_report(
            "Kitchen",
            today,
            &[line("Wash up", None, false), line("Sweep", None, true)],
            Some(&[line("Fix the shelf", None, false)]),
            Some(&[line("Water the plants", Some("20:00"), false)]),
            &en(),
        );

        assert!(report.contains("Due today:\n- Wash up (open)"), "got: {report}");
        assert!(report.contains("- Sweep (done)"), "got: {report}");
        assert!(
            report.contains("No fixed date:\n- Fix the shelf (open)"),
            "got: {report}"
        );
        // The missed section is last and carries no trailing newline, so `ends_with`
        // pins the line exactly — no status suffix may follow the due time.
        assert!(
            report.ends_with("Missed yesterday:\n- Water the plants (by 20:00)"),
            "missed line carries no status suffix, got: {report}"
        );
        assert_eq!(report.matches("(open)").count(), 2, "got: {report}");
    }

    #[test]
    fn test_format_report_undated_none_omits_it_but_empty_shows_the_header() {
        let today = NaiveDate::from_ymd_opt(2026, 7, 25).unwrap();
        let shown = format_report("Kitchen", today, &[], Some(&[]), None, &en());
        let omitted = format_report("Kitchen", today, &[], None, None, &en());

        assert!(shown.contains("No fixed date:\nNo undated tasks"), "got: {shown}");
        assert!(!omitted.contains("No fixed date:"), "got: {omitted}");
    }

    #[test]
    fn test_format_report_undated_german_header() {
        let today = NaiveDate::from_ymd_opt(2026, 7, 25).unwrap();
        let report = format_report("Küche", today, &[], Some(&[]), None, &de());
        assert!(
            report.contains("Ohne festen Termin:\nKeine terminlosen Aufgaben"),
            "got: {report}"
        );
    }

    /// Switching the section off must remove the header too, not leave it above an empty
    /// body — the whole point is that the report says nothing about yesterday.
    #[test]
    fn test_format_report_omits_the_missed_section_entirely() {
        let today = NaiveDate::from_ymd_opt(2026, 7, 25).unwrap();
        let expected = "Daily report — Test Household — Sat, 2026-07-25\n\
                        \n\
                        Due today:\n\
                        No tasks scheduled for today";

        assert_eq!(
            format_report("Test Household", today, &[], None, None, &en()),
            expected
        );
    }

    #[test]
    fn test_format_report_empty_section_differs_from_omitted_section() {
        let today = NaiveDate::from_ymd_opt(2026, 7, 25).unwrap();

        let empty = format_report("Kitchen", today, &[], None, Some(&[]), &en());
        let omitted = format_report("Kitchen", today, &[], None, None, &en());

        assert!(empty.contains(MISSED_YESTERDAY_HEADER));
        assert!(!omitted.contains(MISSED_YESTERDAY_HEADER));
        assert!(!omitted.ends_with('\n'), "got: {omitted}");
    }

    #[tokio::test]
    async fn test_generate_daily_report_with_can_omit_the_missed_section() {
        let pool = create_test_pool().await;
        let (household_id, user_id) = setup(&pool).await;
        let task = create_test_task(&pool, &household_id)
            .with_title("Yesterday's chore")
            .with_recurrence(RecurrenceType::Daily)
            .build()
            .await;
        // A genuinely missed task, so its absence proves the switch and not an empty database.
        insert_missed_task_penalty(&pool, &task.id, pinned_yesterday()).await;

        let with_missed = generate_daily_report_with(
            &pool,
            &household_id,
            &user_id,
            pinned_now(),
            ReportOptions::new(ReportLanguage::En, true),
        )
        .await
        .unwrap();
        assert!(with_missed.contains(MISSED_YESTERDAY_HEADER), "got: {with_missed}");
        assert!(with_missed.contains("- Yesterday's chore"), "got: {with_missed}");

        let without_missed = generate_daily_report_with(
            &pool,
            &household_id,
            &user_id,
            pinned_now(),
            ReportOptions::new(ReportLanguage::En, false),
        )
        .await
        .unwrap();
        assert!(!without_missed.contains(MISSED_YESTERDAY_HEADER), "got: {without_missed}");
        // "Due today" survives — only the second section is gone.
        assert!(without_missed.contains(DUE_TODAY_HEADER), "got: {without_missed}");
        assert!(without_missed.contains("- Yesterday's chore"), "got: {without_missed}");
    }

    #[tokio::test]
    async fn test_generate_daily_report_with_renders_german() {
        let pool = create_test_pool().await;
        let (household_id, user_id) = setup(&pool).await;
        create_test_task(&pool, &household_id)
            .with_title("Staubsaugen")
            .with_recurrence(RecurrenceType::Daily)
            .with_due_time("18:00")
            .build()
            .await;

        let report = generate_daily_report_with(
            &pool,
            &household_id,
            &user_id,
            pinned_now(),
            ReportOptions::new(ReportLanguage::De, true),
        )
        .await
        .unwrap();

        assert!(report.starts_with("Tagesbericht — "), "got: {report}");
        assert!(report.contains("Heute fällig:"), "got: {report}");
        assert!(
            report.contains("- Staubsaugen (bis 18:00) (offen)"),
            "got: {report}"
        );
        assert!(
            report.contains("Gestern wurden alle Aufgaben erledigt"),
            "got: {report}"
        );
    }

    /// D-06 explicitly preserves D-01 for the per-household endpoint: the un-suffixed
    /// entry point must stay English no matter what Phase 6 does.
    #[tokio::test]
    async fn test_generate_daily_report_stays_english() {
        let pool = create_test_pool().await;
        let (household_id, user_id) = setup(&pool).await;

        let report = generate_daily_report(&pool, &household_id, &user_id, pinned_now())
            .await
            .unwrap();

        assert!(report.starts_with("Daily report — "), "got: {report}");
        assert!(report.contains(DUE_TODAY_HEADER), "got: {report}");
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

    /// The counterpart to the test above, through the real query path: an untouched task
    /// says so on its own line instead of relying on a missing `(done)`.
    #[tokio::test]
    async fn test_due_today_marks_an_untouched_task_open() {
        let pool = create_test_pool().await;
        let (household_id, user_id) = setup(&pool).await;
        create_test_task(&pool, &household_id)
            .with_title("Empty the dishwasher")
            .build()
            .await;

        let report = generate_daily_report(&pool, &household_id, &user_id, pinned_now())
            .await
            .unwrap();

        assert!(
            report.contains("- Empty the dishwasher (open)"),
            "got: {report}"
        );
    }

    #[tokio::test]
    async fn test_due_today_not_done_until_target_count_reached() {
        let pool = create_test_pool().await;
        let (household_id, user_id) = setup(&pool).await;
        let task = create_test_task(&pool, &household_id)
            .with_title("Drink water")
            .with_target_count(3)
            .build()
            .await;
        // One of three — the target is not met yet.
        insert_completion(&pool, &task.id, &user_id, pinned_today()).await;

        let report = generate_daily_report(&pool, &household_id, &user_id, pinned_now())
            .await
            .unwrap();

        // Progress is shown, and 1/3 is plainly not done.
        assert!(report.contains("- Drink water (1/3)"), "got: {report}");
        assert!(!report.contains("(done)"), "got: {report}");
    }

    #[tokio::test]
    async fn test_due_today_done_once_target_count_reached() {
        let pool = create_test_pool().await;
        let (household_id, user_id) = setup(&pool).await;
        let task = create_test_task(&pool, &household_id)
            .with_title("Drink water")
            .with_target_count(3)
            .build()
            .await;
        for _ in 0..3 {
            insert_completion(&pool, &task.id, &user_id, pinned_today()).await;
        }

        let report = generate_daily_report(&pool, &household_id, &user_id, pinned_now())
            .await
            .unwrap();

        // A multi-completion task shows its progress: 3/3 is how "done" reads now.
        assert!(report.contains("- Drink water (3/3)"), "got: {report}");
        assert!(!report.contains("(done)"), "got: {report}");
    }

    #[tokio::test]
    async fn test_due_today_done_when_target_count_exceeded() {
        let pool = create_test_pool().await;
        let (household_id, user_id) = setup(&pool).await;
        let task = create_test_task(&pool, &household_id)
            .with_title("Drink water")
            .with_target_count(2)
            .build()
            .await;
        for _ in 0..4 {
            insert_completion(&pool, &task.id, &user_id, pinned_today()).await;
        }

        let report = generate_daily_report(&pool, &household_id, &user_id, pinned_now())
            .await
            .unwrap();

        // Over-fulfilment clamps to the target: 4 of 2 reads as 2/2, never 4/2.
        assert!(report.contains("- Drink water (2/2)"), "got: {report}");
        assert!(!report.contains("(done)"), "got: {report}");
    }

    /// Matches `TaskWithStatus::is_target_met`: target_count 0 is free-form and never "met".
    #[tokio::test]
    async fn test_due_today_free_form_task_is_never_done() {
        let pool = create_test_pool().await;
        let (household_id, user_id) = setup(&pool).await;
        let task = create_test_task(&pool, &household_id)
            .with_title("Tidy up")
            .with_target_count(0)
            .build()
            .await;
        insert_completion(&pool, &task.id, &user_id, pinned_today()).await;

        let report = generate_daily_report(&pool, &household_id, &user_id, pinned_now())
            .await
            .unwrap();

        assert!(report.contains("- Tidy up"), "got: {report}");
        assert!(!report.contains("- Tidy up (done)"), "got: {report}");
    }

    #[tokio::test]
    async fn test_due_today_shows_progress_for_multi_completion_task() {
        let pool = create_test_pool().await;
        let (household_id, user_id) = setup(&pool).await;
        let task = create_test_task(&pool, &household_id)
            .with_title("Drink water")
            .with_target_count(8)
            .build()
            .await;
        for _ in 0..5 {
            insert_completion(&pool, &task.id, &user_id, pinned_today()).await;
        }

        let report = generate_daily_report(&pool, &household_id, &user_id, pinned_now())
            .await
            .unwrap();

        assert!(report.contains("- Drink water (5/8)"), "got: {report}");
        assert!(!report.contains("(done)"), "got: {report}");
    }

    /// The default `target_count` is 1, so a normal task must NOT gain a `(1/1)` counter —
    /// it keeps the plain `(done)` marker. This guards against progress leaking onto every
    /// task in the report.
    #[tokio::test]
    async fn test_due_today_single_completion_task_keeps_done_marker() {
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
        assert!(!report.contains("/1"), "got: {report}");
    }

    #[tokio::test]
    async fn test_separate_undated_moves_onetime_into_its_own_section() {
        let pool = create_test_pool().await;
        let (household_id, user_id) = setup(&pool).await;
        create_test_task(&pool, &household_id)
            .with_title("Daily chore")
            .with_recurrence(RecurrenceType::Daily)
            .build()
            .await;
        create_test_task(&pool, &household_id)
            .with_title("Someday errand")
            .with_recurrence(RecurrenceType::OneTime)
            .build()
            .await;

        let report = generate_daily_report_with(
            &pool,
            &household_id,
            &user_id,
            pinned_now(),
            ReportOptions::new(ReportLanguage::En, true).with_separate_undated(true),
        )
        .await
        .unwrap();

        let due_idx = report.find("Due today:").unwrap();
        let undated_idx = report.find("No fixed date:").unwrap();
        let daily_idx = report.find("- Daily chore").unwrap();
        let errand_idx = report.find("- Someday errand").unwrap();

        // The recurring task stays under "Due today"; the one-time task moves below the
        // "No fixed date" header.
        assert!(due_idx < daily_idx && daily_idx < undated_idx, "got: {report}");
        assert!(undated_idx < errand_idx, "got: {report}");
    }

    /// With the switch off (the default) a one-time task keeps appearing under "Due today"
    /// and no "No fixed date" header exists — the pre-existing behaviour, unchanged.
    #[tokio::test]
    async fn test_undated_stays_in_due_today_when_switch_is_off() {
        let pool = create_test_pool().await;
        let (household_id, user_id) = setup(&pool).await;
        create_test_task(&pool, &household_id)
            .with_title("Someday errand")
            .with_recurrence(RecurrenceType::OneTime)
            .build()
            .await;

        let report = generate_daily_report(&pool, &household_id, &user_id, pinned_now())
            .await
            .unwrap();

        assert!(!report.contains("No fixed date:"), "got: {report}");
        let due_idx = report.find("Due today:").unwrap();
        let errand_idx = report.find("- Someday errand").unwrap();
        let missed_idx = report.find("Missed yesterday:").unwrap();
        assert!(due_idx < errand_idx && errand_idx < missed_idx, "got: {report}");
    }

    #[tokio::test]
    async fn test_separate_undated_uses_the_report_language() {
        let pool = create_test_pool().await;
        let (household_id, user_id) = setup(&pool).await;
        create_test_task(&pool, &household_id)
            .with_title("Irgendwann erledigen")
            .with_recurrence(RecurrenceType::OneTime)
            .build()
            .await;

        let report = generate_daily_report_with(
            &pool,
            &household_id,
            &user_id,
            pinned_now(),
            ReportOptions::new(ReportLanguage::De, true).with_separate_undated(true),
        )
        .await
        .unwrap();

        assert!(report.contains("Ohne festen Termin:"), "got: {report}");
        assert!(report.contains("- Irgendwann erledigen"), "got: {report}");
    }

    /// The counter must reach the public cross-household reports (Phase 6) unchanged: it flows
    /// through the same builder, and `5/8` is language-independent even in a German report.
    #[tokio::test]
    async fn test_due_today_progress_reaches_localized_report() {
        let pool = create_test_pool().await;
        let (household_id, user_id) = setup(&pool).await;
        let task = create_test_task(&pool, &household_id)
            .with_title("Wasser trinken")
            .with_target_count(8)
            .build()
            .await;
        for _ in 0..5 {
            insert_completion(&pool, &task.id, &user_id, pinned_today()).await;
        }

        let report = generate_daily_report_with(
            &pool,
            &household_id,
            &user_id,
            pinned_now(),
            ReportOptions::new(ReportLanguage::De, true),
        )
        .await
        .unwrap();

        assert!(report.contains("- Wasser trinken (5/8)"), "got: {report}");
    }

    #[tokio::test]
    async fn test_due_today_marks_weekdays_task_done_on_its_scheduled_day() {
        let pool = create_test_pool().await;
        let (household_id, user_id) = setup(&pool).await;
        let task = create_test_task(&pool, &household_id)
            .with_title("Weekday workout")
            .with_recurrence(RecurrenceType::Weekdays)
            .build()
            .await;

        // The pinned day is Monday 2027-01-04 and the default schedule is Mon-Fri, so today
        // IS a scheduled occurrence: `get_next_due_date` returns today and `complete_task`
        // stores the completion against today.
        insert_completion(&pool, &task.id, &user_id, pinned_today()).await;

        let report = generate_daily_report(&pool, &household_id, &user_id, pinned_now())
            .await
            .unwrap();

        assert!(
            report.contains("- Weekday workout (done)"),
            "got: {report}"
        );
    }

    /// Counterpart to the test above: before the "on or after" fix this was green, because a
    /// completion made today landed on the following occurrence. It must now be red-turned-
    /// green the other way round — a future due_date does NOT mark today as done.
    #[tokio::test]
    async fn test_due_today_weekdays_completion_on_later_occurrence_is_not_done() {
        let pool = create_test_pool().await;
        let (household_id, user_id) = setup(&pool).await;
        let task = create_test_task(&pool, &household_id)
            .with_title("Weekday workout")
            .with_recurrence(RecurrenceType::Weekdays)
            .build()
            .await;

        let later_occurrence = NaiveDate::from_ymd_opt(2027, 1, 11).unwrap();
        insert_completion(&pool, &task.id, &user_id, later_occurrence).await;

        let report = generate_daily_report(&pool, &household_id, &user_id, pinned_now())
            .await
            .unwrap();

        assert!(report.contains("- Weekday workout"), "got: {report}");
        assert!(
            !report.contains("- Weekday workout (done)"),
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
                "- apple task (by 08:00) (open)",
                "- Zebra task (by 08:00) (open)",
                "- Beta task (by 20:00) (open)",
                "- Middle task (open)",
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

    // ------------------------------------------------------------------
    // "Missed yesterday" — penalized good habits (D-08, D-09, D-13, D-14, D-15)
    // ------------------------------------------------------------------

    #[tokio::test]
    async fn test_missed_yesterday_lists_penalized_task() {
        let pool = create_test_pool().await;
        let (household_id, user_id) = setup(&pool).await;
        let task = create_test_task(&pool, &household_id)
            .with_title("Vacuuming")
            .with_assigned_user(user_id)
            .build()
            .await;
        insert_missed_task_penalty(&pool, &task.id, pinned_yesterday()).await;

        let report = generate_daily_report(&pool, &household_id, &user_id, pinned_now())
            .await
            .unwrap();

        assert!(missed_contains(&report, "- Vacuuming"), "got: {report}");
    }

    #[tokio::test]
    async fn test_missed_yesterday_includes_unassigned_task() {
        let pool = create_test_pool().await;
        let (household_id, user_id) = setup(&pool).await;
        let task = create_test_task(&pool, &household_id)
            .with_title("Vacuuming")
            .build()
            .await;
        insert_missed_task_penalty(&pool, &task.id, pinned_yesterday()).await;

        let report = generate_daily_report(&pool, &household_id, &user_id, pinned_now())
            .await
            .unwrap();

        assert!(missed_contains(&report, "- Vacuuming"), "got: {report}");
    }

    #[tokio::test]
    async fn test_missed_yesterday_excludes_other_users_penalty() {
        let pool = create_test_pool().await;
        let (household_id, user_id) = setup(&pool).await;
        let other_user = create_test_user(&pool, "other@test.com", Role::Member).await;
        create_test_membership(&pool, &household_id, &other_user, Role::Member).await;
        let task = create_test_task(&pool, &household_id)
            .with_title("Someone elses chore")
            .with_assigned_user(other_user)
            .build()
            .await;
        insert_missed_task_penalty(&pool, &task.id, pinned_yesterday()).await;

        let report = generate_daily_report(&pool, &household_id, &user_id, pinned_now())
            .await
            .unwrap();

        assert!(
            !missed_contains(&report, "Someone elses chore"),
            "got: {report}"
        );
    }

    #[tokio::test]
    async fn test_missed_yesterday_excludes_other_households() {
        let pool = create_test_pool().await;
        let (household_id, user_id) = setup(&pool).await;
        let other_household =
            create_test_household_with_name(&pool, "Other Household").await;
        let task = create_test_task(&pool, &other_household)
            .with_title("Foreign chore")
            .build()
            .await;
        insert_missed_task_penalty(&pool, &task.id, pinned_yesterday()).await;

        let report = generate_daily_report(&pool, &household_id, &user_id, pinned_now())
            .await
            .unwrap();

        assert!(!report.contains("Foreign chore"), "got: {report}");
    }

    #[tokio::test]
    async fn test_missed_yesterday_excludes_inverted_habit_penalty() {
        let pool = create_test_pool().await;
        let (household_id, user_id) = setup(&pool).await;
        let task = create_test_task(&pool, &household_id)
            .with_title("Smoking")
            .with_habit_type(HabitType::Bad)
            .build()
            .await;
        insert_missed_task_penalty(&pool, &task.id, pinned_yesterday()).await;

        let report = generate_daily_report(&pool, &household_id, &user_id, pinned_now())
            .await
            .unwrap();

        assert!(!missed_contains(&report, "Smoking"), "got: {report}");
    }

    #[tokio::test]
    async fn test_missed_yesterday_excludes_one_time_task() {
        let pool = create_test_pool().await;
        let (household_id, user_id) = setup(&pool).await;
        let task = create_test_task(&pool, &household_id)
            .with_title("One off errand")
            .with_recurrence(RecurrenceType::OneTime)
            .build()
            .await;
        insert_missed_task_penalty(&pool, &task.id, pinned_yesterday()).await;

        let report = generate_daily_report(&pool, &household_id, &user_id, pinned_now())
            .await
            .unwrap();

        assert!(!missed_contains(&report, "One off errand"), "got: {report}");
    }

    #[tokio::test]
    async fn test_missed_yesterday_excludes_archived_task() {
        let pool = create_test_pool().await;
        let (household_id, user_id) = setup(&pool).await;
        let task = create_test_task(&pool, &household_id)
            .with_title("Archived chore")
            .with_archived(true)
            .build()
            .await;
        insert_missed_task_penalty(&pool, &task.id, pinned_yesterday()).await;

        let report = generate_daily_report(&pool, &household_id, &user_id, pinned_now())
            .await
            .unwrap();

        assert!(!report.contains("Archived chore"), "got: {report}");
    }

    #[tokio::test]
    async fn test_missed_yesterday_excludes_paused_task() {
        let pool = create_test_pool().await;
        let (household_id, user_id) = setup(&pool).await;
        let task = create_test_task(&pool, &household_id)
            .with_title("Paused chore")
            .with_paused(true)
            .build()
            .await;
        insert_missed_task_penalty(&pool, &task.id, pinned_yesterday()).await;

        let report = generate_daily_report(&pool, &household_id, &user_id, pinned_now())
            .await
            .unwrap();

        assert!(!report.contains("Paused chore"), "got: {report}");
    }

    #[tokio::test]
    async fn test_missed_yesterday_excludes_pending_suggestion() {
        let pool = create_test_pool().await;
        let (household_id, user_id) = setup(&pool).await;
        let pending = create_test_task(&pool, &household_id)
            .with_title("Merely suggested chore")
            .with_suggestion(SuggestionStatus::Suggested)
            .build()
            .await;
        let approved = create_test_task(&pool, &household_id)
            .with_title("Approved chore")
            .with_suggestion(SuggestionStatus::Approved)
            .build()
            .await;
        insert_missed_task_penalty(&pool, &pending.id, pinned_yesterday()).await;
        insert_missed_task_penalty(&pool, &approved.id, pinned_yesterday()).await;

        let report = generate_daily_report(&pool, &household_id, &user_id, pinned_now())
            .await
            .unwrap();

        assert!(
            !missed_contains(&report, "Merely suggested chore"),
            "got: {report}"
        );
        assert!(
            missed_contains(&report, "- Approved chore"),
            "got: {report}"
        );
    }

    #[tokio::test]
    async fn test_missed_yesterday_ignores_other_dates() {
        let pool = create_test_pool().await;
        let (household_id, user_id) = setup(&pool).await;
        let task = create_test_task(&pool, &household_id)
            .with_title("Two days ago chore")
            .build()
            .await;
        let two_days_ago = NaiveDate::from_ymd_opt(2027, 1, 2).unwrap();
        insert_missed_task_penalty(&pool, &task.id, two_days_ago).await;

        let report = generate_daily_report(&pool, &household_id, &user_id, pinned_now())
            .await
            .unwrap();

        assert!(
            !missed_contains(&report, "Two days ago chore"),
            "got: {report}"
        );
    }

    #[tokio::test]
    async fn test_missed_yesterday_line_shows_due_time() {
        let pool = create_test_pool().await;
        let (household_id, user_id) = setup(&pool).await;
        let task = create_test_task(&pool, &household_id)
            .with_title("Vacuuming")
            .with_due_time("20:00")
            .build()
            .await;
        insert_missed_task_penalty(&pool, &task.id, pinned_yesterday()).await;

        let report = generate_daily_report(&pool, &household_id, &user_id, pinned_now())
            .await
            .unwrap();

        assert!(
            missed_contains(&report, "- Vacuuming (by 20:00)"),
            "got: {report}"
        );
    }

    /// The progress counter is a "Due today" affordance only — a missed multi-completion
    /// task must not sprout a `(0/3)` in the "Missed yesterday" section.
    #[tokio::test]
    async fn test_missed_yesterday_has_no_progress_counter() {
        let pool = create_test_pool().await;
        let (household_id, user_id) = setup(&pool).await;
        let task = create_test_task(&pool, &household_id)
            .with_title("Vacuuming")
            .with_target_count(3)
            .build()
            .await;
        insert_missed_task_penalty(&pool, &task.id, pinned_yesterday()).await;

        let report = generate_daily_report(&pool, &household_id, &user_id, pinned_now())
            .await
            .unwrap();

        assert!(missed_contains(&report, "- Vacuuming"), "got: {report}");
        assert!(!missed_contains(&report, "/3"), "got: {report}");
    }

    #[tokio::test]
    async fn test_missed_yesterday_empty_renders_empty_state() {
        let pool = create_test_pool().await;
        let (household_id, user_id) = setup(&pool).await;

        let report = generate_daily_report(&pool, &household_id, &user_id, pinned_now())
            .await
            .unwrap();

        assert!(
            report.contains("Missed yesterday:\nAll tasks completed yesterday"),
            "got: {report}"
        );
    }

    // ------------------------------------------------------------------
    // "Missed yesterday" — indulged bad habits and the vacation guard (D-10, D-11, D-14)
    // ------------------------------------------------------------------

    #[tokio::test]
    async fn test_missed_yesterday_lists_indulged_bad_habit() {
        let pool = create_test_pool().await;
        let (household_id, user_id) = setup(&pool).await;
        let task = create_test_task(&pool, &household_id)
            .with_title("Smoking")
            .with_habit_type(HabitType::Bad)
            .with_assigned_user(user_id)
            .build()
            .await;
        insert_completion(&pool, &task.id, &user_id, pinned_yesterday()).await;

        let report = generate_daily_report(&pool, &household_id, &user_id, pinned_now())
            .await
            .unwrap();

        assert!(missed_contains(&report, "- Smoking"), "got: {report}");
    }

    #[tokio::test]
    async fn test_missed_yesterday_bad_habit_and_penalty_in_one_section() {
        let pool = create_test_pool().await;
        let (household_id, user_id) = setup(&pool).await;
        let penalized = create_test_task(&pool, &household_id)
            .with_title("Vacuuming")
            .build()
            .await;
        insert_missed_task_penalty(&pool, &penalized.id, pinned_yesterday()).await;
        let indulged = create_test_task(&pool, &household_id)
            .with_title("Smoking")
            .with_habit_type(HabitType::Bad)
            .build()
            .await;
        insert_completion(&pool, &indulged.id, &user_id, pinned_yesterday()).await;

        let report = generate_daily_report(&pool, &household_id, &user_id, pinned_now())
            .await
            .unwrap();

        assert!(missed_contains(&report, "- Vacuuming"), "got: {report}");
        assert!(missed_contains(&report, "- Smoking"), "got: {report}");
        // D-11: one single unlabelled section, no sub-headers.
        assert_eq!(
            report.matches("Missed yesterday:").count(),
            1,
            "got: {report}"
        );
    }

    #[tokio::test]
    async fn test_missed_yesterday_excludes_other_users_indulged_bad_habit() {
        let pool = create_test_pool().await;
        let (household_id, user_id) = setup(&pool).await;
        let other_user = create_test_user(&pool, "other@test.com", Role::Member).await;
        create_test_membership(&pool, &household_id, &other_user, Role::Member).await;
        let task = create_test_task(&pool, &household_id)
            .with_title("Their smoking")
            .with_habit_type(HabitType::Bad)
            .with_assigned_user(other_user)
            .build()
            .await;
        insert_completion(&pool, &task.id, &other_user, pinned_yesterday()).await;

        let report = generate_daily_report(&pool, &household_id, &user_id, pinned_now())
            .await
            .unwrap();

        assert!(!missed_contains(&report, "Their smoking"), "got: {report}");
    }

    #[tokio::test]
    async fn test_missed_yesterday_excludes_good_habit_completion() {
        let pool = create_test_pool().await;
        let (household_id, user_id) = setup(&pool).await;
        let task = create_test_task(&pool, &household_id)
            .with_title("Went jogging")
            .build()
            .await;
        insert_completion(&pool, &task.id, &user_id, pinned_yesterday()).await;

        let report = generate_daily_report(&pool, &household_id, &user_id, pinned_now())
            .await
            .unwrap();

        assert!(!missed_contains(&report, "Went jogging"), "got: {report}");
    }

    #[tokio::test]
    async fn test_missed_yesterday_excludes_indulged_bad_habit_when_archived_paused_or_pending() {
        let pool = create_test_pool().await;
        let (household_id, user_id) = setup(&pool).await;

        let archived = create_test_task(&pool, &household_id)
            .with_title("Archived smoking")
            .with_habit_type(HabitType::Bad)
            .with_archived(true)
            .build()
            .await;
        let paused = create_test_task(&pool, &household_id)
            .with_title("Paused smoking")
            .with_habit_type(HabitType::Bad)
            .with_paused(true)
            .build()
            .await;
        let pending = create_test_task(&pool, &household_id)
            .with_title("Suggested smoking")
            .with_habit_type(HabitType::Bad)
            .with_suggestion(SuggestionStatus::Suggested)
            .build()
            .await;

        for task_id in [archived.id, paused.id, pending.id] {
            insert_completion(&pool, &task_id, &user_id, pinned_yesterday()).await;
        }

        let report = generate_daily_report(&pool, &household_id, &user_id, pinned_now())
            .await
            .unwrap();

        assert!(!report.contains("Archived smoking"), "got: {report}");
        assert!(!report.contains("Paused smoking"), "got: {report}");
        assert!(!missed_contains(&report, "Suggested smoking"), "got: {report}");
    }

    #[tokio::test]
    async fn test_missed_yesterday_excludes_indulged_bad_habit_one_time() {
        let pool = create_test_pool().await;
        let (household_id, user_id) = setup(&pool).await;
        let task = create_test_task(&pool, &household_id)
            .with_title("One off indulgence")
            .with_habit_type(HabitType::Bad)
            .with_recurrence(RecurrenceType::OneTime)
            .build()
            .await;
        insert_completion(&pool, &task.id, &user_id, pinned_yesterday()).await;

        let report = generate_daily_report(&pool, &household_id, &user_id, pinned_now())
            .await
            .unwrap();

        assert!(
            !missed_contains(&report, "One off indulgence"),
            "got: {report}"
        );
    }

    #[tokio::test]
    async fn test_missed_yesterday_excludes_other_household_bad_habit() {
        let pool = create_test_pool().await;
        let (household_id, user_id) = setup(&pool).await;
        let other_household =
            create_test_household_with_name(&pool, "Other Household").await;
        let task = create_test_task(&pool, &other_household)
            .with_title("Foreign smoking")
            .with_habit_type(HabitType::Bad)
            .build()
            .await;
        insert_completion(&pool, &task.id, &user_id, pinned_yesterday()).await;

        let report = generate_daily_report(&pool, &household_id, &user_id, pinned_now())
            .await
            .unwrap();

        assert!(!report.contains("Foreign smoking"), "got: {report}");
    }

    #[tokio::test]
    async fn test_missed_yesterday_empty_during_vacation_with_penalty_rows() {
        let pool = create_test_pool().await;
        let (household_id, user_id) = setup(&pool).await;
        let penalized = create_test_task(&pool, &household_id)
            .with_title("Vacuuming")
            .build()
            .await;
        insert_missed_task_penalty(&pool, &penalized.id, pinned_yesterday()).await;
        let indulged = create_test_task(&pool, &household_id)
            .with_title("Smoking")
            .with_habit_type(HabitType::Bad)
            .build()
            .await;
        insert_completion(&pool, &indulged.id, &user_id, pinned_yesterday()).await;

        set_vacation_mode(&pool, &household_id, true).await;
        set_vacation_dates(
            &pool,
            &household_id,
            Some(NaiveDate::from_ymd_opt(2027, 1, 1).unwrap()),
            Some(NaiveDate::from_ymd_opt(2027, 1, 10).unwrap()),
        )
        .await;

        let report = generate_daily_report(&pool, &household_id, &user_id, pinned_now())
            .await
            .unwrap();

        assert!(
            report.contains("Missed yesterday:\nAll tasks completed yesterday"),
            "got: {report}"
        );
    }

    #[tokio::test]
    async fn test_missed_yesterday_not_suppressed_when_vacation_range_excludes_yesterday() {
        let pool = create_test_pool().await;
        let (household_id, user_id) = setup(&pool).await;
        let penalized = create_test_task(&pool, &household_id)
            .with_title("Vacuuming")
            .build()
            .await;
        insert_missed_task_penalty(&pool, &penalized.id, pinned_yesterday()).await;

        // Vacation covers only today (2027-01-04), not yesterday (2027-01-03).
        set_vacation_mode(&pool, &household_id, true).await;
        set_vacation_dates(
            &pool,
            &household_id,
            Some(pinned_today()),
            Some(pinned_today()),
        )
        .await;

        let report = generate_daily_report(&pool, &household_id, &user_id, pinned_now())
            .await
            .unwrap();

        assert!(missed_contains(&report, "- Vacuuming"), "got: {report}");
    }

    #[tokio::test]
    async fn test_missed_yesterday_sorted_by_due_time_then_title() {
        let pool = create_test_pool().await;
        let (household_id, user_id) = setup(&pool).await;

        for (title, due_time) in [("Zebra chore", "08:00"), ("apple chore", "08:00")] {
            let task = create_test_task(&pool, &household_id)
                .with_title(title)
                .with_due_time(due_time)
                .build()
                .await;
            insert_missed_task_penalty(&pool, &task.id, pinned_yesterday()).await;
        }

        for (title, due_time) in [("Beta habit", Some("20:00")), ("Midnight habit", None)] {
            let mut builder = create_test_task(&pool, &household_id)
                .with_title(title)
                .with_habit_type(HabitType::Bad);
            if let Some(time) = due_time {
                builder = builder.with_due_time(time);
            }
            let task = builder.build().await;
            insert_completion(&pool, &task.id, &user_id, pinned_yesterday()).await;
        }

        let report = generate_daily_report(&pool, &household_id, &user_id, pinned_now())
            .await
            .unwrap();

        assert_eq!(
            missed_section(&report),
            vec![
                "- apple chore (by 08:00)",
                "- Zebra chore (by 08:00)",
                "- Beta habit (by 20:00)",
                "- Midnight habit",
            ],
            "got: {report}"
        );
    }
}
