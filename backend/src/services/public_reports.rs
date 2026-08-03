//! Public cross-household report links (Phase 6, PUBREP-01..07).
//!
//! A user configures named reports in their user settings. Each report spans an explicitly
//! chosen set of the households they belong to (D-01) and is retrievable without any
//! authentication through an unguessable UUID token (D-05).
//!
//! All logic lives here so it is unit-testable; the handlers stay thin, exactly as the
//! per-household report of Phase 2.1 does it.

use chrono::{DateTime, Utc};
use sqlx::SqlitePool;
use uuid::Uuid;

use shared::{CreatePublicReportRequest, PublicReport, UpdatePublicReportRequest};

use crate::models::PublicReportRow;
use crate::services::{
    households,
    report::{self, ReportError, ReportLanguage, ReportOptions},
};

/// D-06: the languages a report may render in.
const SUPPORTED_LANGUAGES: &[&str] = &["en", "de"];

/// A report name has to fit a settings list; anything longer is a paste accident.
const MAX_NAME_LENGTH: usize = 100;

/// Each report is a permanently reachable, unauthenticated URL, so the number a single
/// account can mint is bounded. Far above any real use, low enough to stop a runaway loop.
const MAX_REPORTS_PER_USER: usize = 20;

/// D-02: one `generate_daily_report` block per household, separated by a blank line. Each
/// block already opens with its household name, so no extra separator is needed.
const BLOCK_SEPARATOR: &str = "\n\n";

#[derive(Debug, thiserror::Error)]
pub enum PublicReportError {
    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),
    #[error("Household error: {0}")]
    Household(#[from] households::HouseholdError),
    #[error("Report error: {0}")]
    Report(#[from] ReportError),
    #[error("Report not found")]
    NotFound,
    #[error("Report name must not be empty and at most {MAX_NAME_LENGTH} characters")]
    InvalidName,
    #[error("Unsupported language")]
    InvalidLanguage,
    #[error("Not a member of household {0}")]
    NotAMemberOfHousehold(Uuid),
    #[error("A user may not have more than {MAX_REPORTS_PER_USER} reports")]
    TooManyReports,
    /// A stored id or token that is not a valid UUID. Unreachable through the API, which
    /// only ever writes generated UUIDs, but the public endpoint must not panic on it.
    #[error("Stored report is corrupt")]
    CorruptRow,
}

// ============================================================================
// Validation
// ============================================================================

fn validate_name(name: &str) -> Result<String, PublicReportError> {
    let trimmed = name.trim();
    if trimmed.is_empty() || trimmed.chars().count() > MAX_NAME_LENGTH {
        return Err(PublicReportError::InvalidName);
    }
    Ok(trimmed.to_string())
}

fn validate_language(language: &str) -> Result<String, PublicReportError> {
    if SUPPORTED_LANGUAGES.contains(&language) {
        Ok(language.to_string())
    } else {
        Err(PublicReportError::InvalidLanguage)
    }
}

/// D-01: a report may only span households the user actually belongs to — checked at write
/// time so an unauthorized id never even reaches storage. Duplicates in the request are
/// collapsed; the junction table's composite primary key would reject them anyway.
async fn validate_household_selection(
    pool: &SqlitePool,
    user_id: &Uuid,
    household_ids: &[Uuid],
) -> Result<Vec<Uuid>, PublicReportError> {
    let mut unique: Vec<Uuid> = Vec::with_capacity(household_ids.len());
    for household_id in household_ids {
        if unique.contains(household_id) {
            continue;
        }
        if !households::is_member(pool, household_id, user_id).await? {
            return Err(PublicReportError::NotAMemberOfHousehold(*household_id));
        }
        unique.push(*household_id);
    }
    Ok(unique)
}

// ============================================================================
// Reads
// ============================================================================

async fn load_household_ids(
    pool: &SqlitePool,
    report_id: &Uuid,
) -> Result<Vec<Uuid>, PublicReportError> {
    let raw = sqlx::query_scalar::<_, String>(
        "SELECT household_id FROM public_report_households WHERE report_id = ?",
    )
    .bind(report_id.to_string())
    .fetch_all(pool)
    .await?;

    // A malformed household id here is skipped rather than fatal: it can only come from a
    // hand-edited database, and it costs the report one block instead of the whole render.
    Ok(raw.iter().filter_map(|id| Uuid::parse_str(id).ok()).collect())
}

async fn row_to_report(
    pool: &SqlitePool,
    row: PublicReportRow,
) -> Result<PublicReport, PublicReportError> {
    let report_id = Uuid::parse_str(&row.id).map_err(|_| PublicReportError::CorruptRow)?;
    let household_ids = load_household_ids(pool, &report_id).await?;
    row.to_shared(household_ids)
        .ok_or(PublicReportError::CorruptRow)
}

/// All reports belonging to `user_id`, newest first.
pub async fn list_reports(
    pool: &SqlitePool,
    user_id: &Uuid,
) -> Result<Vec<PublicReport>, PublicReportError> {
    let rows: Vec<PublicReportRow> = sqlx::query_as(
        "SELECT * FROM public_reports WHERE user_id = ? ORDER BY created_at DESC, id",
    )
    .bind(user_id.to_string())
    .fetch_all(pool)
    .await?;

    let mut reports = Vec::with_capacity(rows.len());
    for row in rows {
        reports.push(row_to_report(pool, row).await?);
    }
    Ok(reports)
}

/// One report, scoped to its owner.
///
/// A report belonging to somebody else is reported as `NotFound`, not as a permission
/// error, so the endpoint cannot be used to probe which report ids exist.
pub async fn get_report(
    pool: &SqlitePool,
    report_id: &Uuid,
    user_id: &Uuid,
) -> Result<PublicReport, PublicReportError> {
    let row: Option<PublicReportRow> =
        sqlx::query_as("SELECT * FROM public_reports WHERE id = ? AND user_id = ?")
            .bind(report_id.to_string())
            .bind(user_id.to_string())
            .fetch_optional(pool)
            .await?;

    match row {
        Some(row) => row_to_report(pool, row).await,
        None => Err(PublicReportError::NotFound),
    }
}

// ============================================================================
// Writes
// ============================================================================

async fn replace_household_selection(
    pool: &SqlitePool,
    report_id: &Uuid,
    household_ids: &[Uuid],
) -> Result<(), PublicReportError> {
    sqlx::query("DELETE FROM public_report_households WHERE report_id = ?")
        .bind(report_id.to_string())
        .execute(pool)
        .await?;

    for household_id in household_ids {
        sqlx::query(
            "INSERT INTO public_report_households (report_id, household_id) VALUES (?, ?)",
        )
        .bind(report_id.to_string())
        .bind(household_id.to_string())
        .execute(pool)
        .await?;
    }
    Ok(())
}

pub async fn create_report(
    pool: &SqlitePool,
    user_id: &Uuid,
    request: CreatePublicReportRequest,
) -> Result<PublicReport, PublicReportError> {
    let existing: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM public_reports WHERE user_id = ?")
            .bind(user_id.to_string())
            .fetch_one(pool)
            .await?;
    if existing >= MAX_REPORTS_PER_USER as i64 {
        return Err(PublicReportError::TooManyReports);
    }

    let name = validate_name(&request.name)?;
    let language = validate_language(request.language.as_deref().unwrap_or("en"))?;
    let household_ids = validate_household_selection(
        pool,
        user_id,
        &request.household_ids.unwrap_or_default(),
    )
    .await?;

    let include_missed = request.include_missed.unwrap_or(true);
    let separate_undated = request.separate_undated.unwrap_or(false);

    let report_id = Uuid::new_v4();
    // D-05: the token is what authorizes the public read, so it is generated here and
    // never derived from the report id — knowing one must not reveal the other.
    let token = Uuid::new_v4();
    let now = Utc::now();

    sqlx::query(
        r#"
        INSERT INTO public_reports (id, user_id, name, token, language, enabled, include_missed, separate_undated, created_at, updated_at)
        VALUES (?, ?, ?, ?, ?, 1, ?, ?, ?, ?)
        "#,
    )
    .bind(report_id.to_string())
    .bind(user_id.to_string())
    .bind(&name)
    .bind(token.to_string())
    .bind(&language)
    .bind(include_missed)
    .bind(separate_undated)
    .bind(now)
    .bind(now)
    .execute(pool)
    .await?;

    replace_household_selection(pool, &report_id, &household_ids).await?;

    Ok(PublicReport {
        id: report_id,
        user_id: *user_id,
        name,
        token,
        language,
        enabled: true,
        include_missed,
        separate_undated,
        household_ids,
        created_at: now,
        updated_at: now,
    })
}

/// Update a report. Absent fields stay unchanged; a present `household_ids` REPLACES the
/// whole selection.
pub async fn update_report(
    pool: &SqlitePool,
    report_id: &Uuid,
    user_id: &Uuid,
    request: UpdatePublicReportRequest,
) -> Result<PublicReport, PublicReportError> {
    // Ownership is established before anything is written.
    let current = get_report(pool, report_id, user_id).await?;

    let name = match request.name {
        Some(name) => validate_name(&name)?,
        None => current.name,
    };
    let language = match request.language {
        Some(language) => validate_language(&language)?,
        None => current.language,
    };
    let enabled = request.enabled.unwrap_or(current.enabled);
    let include_missed = request.include_missed.unwrap_or(current.include_missed);
    let separate_undated = request.separate_undated.unwrap_or(current.separate_undated);

    // Validate the new selection BEFORE touching the row, so a rejected household id
    // leaves the report exactly as it was.
    let household_ids = match request.household_ids {
        Some(ids) => Some(validate_household_selection(pool, user_id, &ids).await?),
        None => None,
    };

    let now = Utc::now();
    sqlx::query(
        "UPDATE public_reports SET name = ?, language = ?, enabled = ?, include_missed = ?, separate_undated = ?, updated_at = ? WHERE id = ? AND user_id = ?",
    )
    .bind(&name)
    .bind(&language)
    .bind(enabled)
    .bind(include_missed)
    .bind(separate_undated)
    .bind(now)
    .bind(report_id.to_string())
    .bind(user_id.to_string())
    .execute(pool)
    .await?;

    if let Some(ref ids) = household_ids {
        replace_household_selection(pool, report_id, ids).await?;
    }

    Ok(PublicReport {
        id: current.id,
        user_id: current.user_id,
        name,
        token: current.token,
        language,
        enabled,
        include_missed,
        separate_undated,
        household_ids: household_ids.unwrap_or(current.household_ids),
        created_at: current.created_at,
        updated_at: now,
    })
}

/// D-05: mint a new token, which immediately invalidates every previously shared URL.
pub async fn regenerate_token(
    pool: &SqlitePool,
    report_id: &Uuid,
    user_id: &Uuid,
) -> Result<PublicReport, PublicReportError> {
    let current = get_report(pool, report_id, user_id).await?;

    let token = Uuid::new_v4();
    let now = Utc::now();
    sqlx::query("UPDATE public_reports SET token = ?, updated_at = ? WHERE id = ? AND user_id = ?")
        .bind(token.to_string())
        .bind(now)
        .bind(report_id.to_string())
        .bind(user_id.to_string())
        .execute(pool)
        .await?;

    Ok(PublicReport {
        token,
        updated_at: now,
        ..current
    })
}

pub async fn delete_report(
    pool: &SqlitePool,
    report_id: &Uuid,
    user_id: &Uuid,
) -> Result<(), PublicReportError> {
    // Establishes ownership and produces the NotFound the handler maps to 404.
    get_report(pool, report_id, user_id).await?;

    // The junction rows go first and explicitly: this project never enables
    // `PRAGMA foreign_keys`, so the schema's ON DELETE CASCADE does not fire.
    sqlx::query("DELETE FROM public_report_households WHERE report_id = ?")
        .bind(report_id.to_string())
        .execute(pool)
        .await?;

    sqlx::query("DELETE FROM public_reports WHERE id = ? AND user_id = ?")
        .bind(report_id.to_string())
        .bind(user_id.to_string())
        .execute(pool)
        .await?;

    Ok(())
}

// ============================================================================
// Public rendering (PUBREP-02, PUBREP-03)
// ============================================================================

/// D-08: what a report says when nothing is left to render — every selected household was
/// dropped, or none was ever selected.
fn empty_selection_text(language: ReportLanguage) -> &'static str {
    match language {
        ReportLanguage::En => "No households configured for this report",
        ReportLanguage::De => "Für diesen Bericht sind keine Haushalte konfiguriert",
    }
}

/// Render a report by its public token.
///
/// D-10: a disabled report and an unknown token both produce `NotFound`, so the response
/// tells an unauthenticated caller nothing about which tokens exist.
pub async fn render_by_token(
    pool: &SqlitePool,
    token: &Uuid,
    now_utc: DateTime<Utc>,
) -> Result<String, PublicReportError> {
    let row: Option<PublicReportRow> =
        sqlx::query_as("SELECT * FROM public_reports WHERE token = ? AND enabled = 1")
            .bind(token.to_string())
            .fetch_optional(pool)
            .await?;

    let Some(row) = row else {
        return Err(PublicReportError::NotFound);
    };

    let report = row_to_report(pool, row).await?;
    render_report(pool, &report, now_utc).await
}

/// D-02/D-04/D-07/D-08: one daily-report block per selected household, each resolved in its
/// own timezone, ordered alphabetically by household name.
///
/// The membership filter is not a separate check but a consequence of the source: the
/// selection is intersected with the households the owner currently belongs to. A household
/// they have left — or that no longer exists — is therefore silently absent (D-08), and a
/// stale link cannot leak data.
pub async fn render_report(
    pool: &SqlitePool,
    report: &PublicReport,
    now_utc: DateTime<Utc>,
) -> Result<String, PublicReportError> {
    let language = ReportLanguage::from_code(&report.language);
    let options = ReportOptions::new(language, report.include_missed)
        .with_separate_undated(report.separate_undated);

    let mut households = households::list_user_households(pool, &report.user_id)
        .await?
        .into_iter()
        .filter(|household| report.household_ids.contains(&household.id))
        .collect::<Vec<_>>();

    // D-07: deterministic order without storing a sort position. Case-insensitive so
    // "kitchen" and "Kitchen" do not end up on opposite ends of the list.
    households.sort_by_key(|household| household.name.to_lowercase());

    let mut blocks = Vec::with_capacity(households.len());
    for household in households {
        match report::generate_daily_report_with(
            pool,
            &household.id,
            &report.user_id,
            now_utc,
            options,
        )
        .await
        {
            Ok(block) => blocks.push(block),
            // Cannot normally happen — the list above is already membership-filtered — but
            // swallowing exactly this variant keeps a race (leaving a household mid-render)
            // from turning into a 500. Every other error still propagates.
            Err(ReportError::NotAMember) | Err(ReportError::HouseholdNotFound) => continue,
            Err(e) => return Err(e.into()),
        }
    }

    let body = if blocks.is_empty() {
        empty_selection_text(language).to_string()
    } else {
        blocks.join(BLOCK_SEPARATOR)
    };

    Ok(with_title(&report.name, &body))
}

/// Put the report's own name above the household blocks.
///
/// Without it the text says which households it covers but not which of the user's reports
/// it is — and several reports over overlapping households look alike at a glance. Underlined
/// with `=` so the title reads as a title in a plain-text file and stays machine-detectable.
fn with_title(name: &str, body: &str) -> String {
    let title = name.trim();
    if title.is_empty() {
        return body.to_string();
    }

    // Count characters, not bytes: an underline sized in bytes runs too long under "Küche".
    let underline = "=".repeat(title.chars().count());
    format!("{title}\n{underline}\n\n{body}")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::*;
    use chrono::TimeZone;
    use shared::{RecurrenceType, Role};

    /// 2027-01-04 is a Monday, safely after every fixture's `created_at`.
    fn pinned_now() -> DateTime<Utc> {
        Utc.with_ymd_and_hms(2027, 1, 4, 12, 0, 0).unwrap()
    }

    async fn user_with_household(pool: &SqlitePool, name: &str) -> (Uuid, Uuid) {
        let user_id = create_test_user(pool, &format!("{name}@test.com"), Role::Member).await;
        let household_id = create_test_household_with_name(pool, name).await;
        create_test_membership(pool, &household_id, &user_id, Role::Member).await;
        (user_id, household_id)
    }

    async fn join(pool: &SqlitePool, user_id: &Uuid, name: &str) -> Uuid {
        let household_id = create_test_household_with_name(pool, name).await;
        create_test_membership(pool, &household_id, user_id, Role::Member).await;
        household_id
    }

    fn create(name: &str, households: Vec<Uuid>) -> CreatePublicReportRequest {
        CreatePublicReportRequest {
            name: name.to_string(),
            language: None,
            include_missed: None,
            separate_undated: None,
            household_ids: Some(households),
        }
    }

    // ------------------------------------------------------------------
    // Creation and validation
    // ------------------------------------------------------------------

    #[tokio::test]
    async fn test_create_report_defaults_to_english_and_enabled() {
        let pool = create_test_pool().await;
        let (user_id, household_id) = user_with_household(&pool, "Kitchen").await;

        let report = create_report(&pool, &user_id, create("Everything", vec![household_id]))
            .await
            .unwrap();

        assert_eq!(report.name, "Everything");
        assert_eq!(report.language, "en");
        assert!(report.enabled);
        assert_eq!(report.household_ids, vec![household_id]);
        assert_ne!(report.token, Uuid::nil());
        // D-05: the token must not be derivable from the report id.
        assert_ne!(report.token, report.id);
    }

    #[tokio::test]
    async fn test_create_report_trims_name_and_rejects_empty() {
        let pool = create_test_pool().await;
        let (user_id, _) = user_with_household(&pool, "Kitchen").await;

        let report = create_report(&pool, &user_id, create("  Padded  ", vec![]))
            .await
            .unwrap();
        assert_eq!(report.name, "Padded");

        assert!(matches!(
            create_report(&pool, &user_id, create("   ", vec![])).await,
            Err(PublicReportError::InvalidName)
        ));

        let too_long = "x".repeat(MAX_NAME_LENGTH + 1);
        assert!(matches!(
            create_report(&pool, &user_id, create(&too_long, vec![])).await,
            Err(PublicReportError::InvalidName)
        ));
    }

    #[tokio::test]
    async fn test_create_report_rejects_unsupported_language() {
        let pool = create_test_pool().await;
        let (user_id, _) = user_with_household(&pool, "Kitchen").await;

        let request = CreatePublicReportRequest {
            name: "Report".to_string(),
            language: Some("fr".to_string()),
            include_missed: None,
            separate_undated: None,
            household_ids: None,
        };

        assert!(matches!(
            create_report(&pool, &user_id, request).await,
            Err(PublicReportError::InvalidLanguage)
        ));
    }

    /// PUBREP-01: the selection is restricted to the user's own households, enforced at
    /// write time so a foreign id never reaches storage.
    #[tokio::test]
    async fn test_create_report_rejects_foreign_household() {
        let pool = create_test_pool().await;
        let (user_id, _) = user_with_household(&pool, "Kitchen").await;
        let (_, foreign_household) = user_with_household(&pool, "Neighbour").await;

        let result = create_report(&pool, &user_id, create("Sneaky", vec![foreign_household])).await;

        assert!(matches!(
            result,
            Err(PublicReportError::NotAMemberOfHousehold(id)) if id == foreign_household
        ));
        assert!(list_reports(&pool, &user_id).await.unwrap().is_empty());
    }

    #[tokio::test]
    async fn test_create_report_collapses_duplicate_households() {
        let pool = create_test_pool().await;
        let (user_id, household_id) = user_with_household(&pool, "Kitchen").await;

        let report = create_report(
            &pool,
            &user_id,
            create("Dupes", vec![household_id, household_id]),
        )
        .await
        .unwrap();

        assert_eq!(report.household_ids, vec![household_id]);
        let reloaded = get_report(&pool, &report.id, &user_id).await.unwrap();
        assert_eq!(reloaded.household_ids, vec![household_id]);
    }

    #[tokio::test]
    async fn test_create_report_enforces_the_per_user_cap() {
        let pool = create_test_pool().await;
        let (user_id, _) = user_with_household(&pool, "Kitchen").await;

        for index in 0..MAX_REPORTS_PER_USER {
            create_report(&pool, &user_id, create(&format!("R{index}"), vec![]))
                .await
                .unwrap();
        }

        assert!(matches!(
            create_report(&pool, &user_id, create("One too many", vec![])).await,
            Err(PublicReportError::TooManyReports)
        ));
    }

    // ------------------------------------------------------------------
    // Ownership scoping
    // ------------------------------------------------------------------

    #[tokio::test]
    async fn test_list_reports_is_scoped_to_the_owner() {
        let pool = create_test_pool().await;
        let (mine, _) = user_with_household(&pool, "Kitchen").await;
        let (theirs, _) = user_with_household(&pool, "Neighbour").await;

        create_report(&pool, &mine, create("Mine", vec![])).await.unwrap();
        create_report(&pool, &theirs, create("Theirs", vec![])).await.unwrap();

        let listed = list_reports(&pool, &mine).await.unwrap();
        assert_eq!(listed.len(), 1);
        assert_eq!(listed[0].name, "Mine");
    }

    /// Someone else's report is NotFound rather than a permission error, so report ids
    /// cannot be probed through the authenticated API.
    #[tokio::test]
    async fn test_foreign_report_is_not_found_for_read_update_and_delete() {
        let pool = create_test_pool().await;
        let (owner, _) = user_with_household(&pool, "Kitchen").await;
        let (outsider, _) = user_with_household(&pool, "Neighbour").await;
        let report = create_report(&pool, &owner, create("Mine", vec![])).await.unwrap();

        assert!(matches!(
            get_report(&pool, &report.id, &outsider).await,
            Err(PublicReportError::NotFound)
        ));
        assert!(matches!(
            update_report(
                &pool,
                &report.id,
                &outsider,
                UpdatePublicReportRequest {
                    name: Some("Hijacked".to_string()),
                    ..Default::default()
                }
            )
            .await,
            Err(PublicReportError::NotFound)
        ));
        assert!(matches!(
            regenerate_token(&pool, &report.id, &outsider).await,
            Err(PublicReportError::NotFound)
        ));
        assert!(matches!(
            delete_report(&pool, &report.id, &outsider).await,
            Err(PublicReportError::NotFound)
        ));

        // The report is untouched.
        let reloaded = get_report(&pool, &report.id, &owner).await.unwrap();
        assert_eq!(reloaded.name, "Mine");
        assert_eq!(reloaded.token, report.token);
    }

    // ------------------------------------------------------------------
    // Updates
    // ------------------------------------------------------------------

    #[tokio::test]
    async fn test_update_report_leaves_absent_fields_unchanged() {
        let pool = create_test_pool().await;
        let (user_id, household_id) = user_with_household(&pool, "Kitchen").await;
        let report = create_report(&pool, &user_id, create("Before", vec![household_id]))
            .await
            .unwrap();

        let updated = update_report(
            &pool,
            &report.id,
            &user_id,
            UpdatePublicReportRequest {
                name: Some("After".to_string()),
                ..Default::default()
            },
        )
        .await
        .unwrap();

        assert_eq!(updated.name, "After");
        assert_eq!(updated.language, "en");
        assert!(updated.enabled);
        assert_eq!(updated.household_ids, vec![household_id]);
        assert_eq!(updated.token, report.token);
    }

    #[tokio::test]
    async fn test_update_report_replaces_the_household_selection() {
        let pool = create_test_pool().await;
        let (user_id, first) = user_with_household(&pool, "Kitchen").await;
        let second = join(&pool, &user_id, "Garage").await;
        let report = create_report(&pool, &user_id, create("Report", vec![first]))
            .await
            .unwrap();

        let updated = update_report(
            &pool,
            &report.id,
            &user_id,
            UpdatePublicReportRequest {
                household_ids: Some(vec![second]),
                ..Default::default()
            },
        )
        .await
        .unwrap();

        assert_eq!(updated.household_ids, vec![second]);
        assert_eq!(
            get_report(&pool, &report.id, &user_id).await.unwrap().household_ids,
            vec![second]
        );
    }

    #[tokio::test]
    async fn test_update_report_can_clear_the_household_selection() {
        let pool = create_test_pool().await;
        let (user_id, household_id) = user_with_household(&pool, "Kitchen").await;
        let report = create_report(&pool, &user_id, create("Report", vec![household_id]))
            .await
            .unwrap();

        let updated = update_report(
            &pool,
            &report.id,
            &user_id,
            UpdatePublicReportRequest {
                household_ids: Some(vec![]),
                ..Default::default()
            },
        )
        .await
        .unwrap();

        assert!(updated.household_ids.is_empty());
    }

    /// A rejected household id must leave the report exactly as it was — no half-applied
    /// rename, no cleared selection.
    #[tokio::test]
    async fn test_update_report_rejects_foreign_household_without_partial_write() {
        let pool = create_test_pool().await;
        let (user_id, household_id) = user_with_household(&pool, "Kitchen").await;
        let (_, foreign) = user_with_household(&pool, "Neighbour").await;
        let report = create_report(&pool, &user_id, create("Before", vec![household_id]))
            .await
            .unwrap();

        let result = update_report(
            &pool,
            &report.id,
            &user_id,
            UpdatePublicReportRequest {
                name: Some("After".to_string()),
                household_ids: Some(vec![foreign]),
                ..Default::default()
            },
        )
        .await;

        assert!(matches!(
            result,
            Err(PublicReportError::NotAMemberOfHousehold(id)) if id == foreign
        ));

        let reloaded = get_report(&pool, &report.id, &user_id).await.unwrap();
        assert_eq!(reloaded.name, "Before");
        assert_eq!(reloaded.household_ids, vec![household_id]);
    }

    #[tokio::test]
    async fn test_update_report_rejects_unsupported_language() {
        let pool = create_test_pool().await;
        let (user_id, _) = user_with_household(&pool, "Kitchen").await;
        let report = create_report(&pool, &user_id, create("Report", vec![])).await.unwrap();

        assert!(matches!(
            update_report(
                &pool,
                &report.id,
                &user_id,
                UpdatePublicReportRequest {
                    language: Some("fr".to_string()),
                    ..Default::default()
                }
            )
            .await,
            Err(PublicReportError::InvalidLanguage)
        ));
    }

    // ------------------------------------------------------------------
    // Token lifecycle (PUBREP-05)
    // ------------------------------------------------------------------

    #[tokio::test]
    async fn test_regenerate_token_invalidates_the_previous_url() {
        let pool = create_test_pool().await;
        let (user_id, household_id) = user_with_household(&pool, "Kitchen").await;
        let report = create_report(&pool, &user_id, create("Report", vec![household_id]))
            .await
            .unwrap();

        assert!(render_by_token(&pool, &report.token, pinned_now()).await.is_ok());

        let rotated = regenerate_token(&pool, &report.id, &user_id).await.unwrap();

        assert_ne!(rotated.token, report.token);
        assert_eq!(rotated.id, report.id);
        assert_eq!(rotated.name, report.name);
        assert_eq!(rotated.household_ids, report.household_ids);

        assert!(matches!(
            render_by_token(&pool, &report.token, pinned_now()).await,
            Err(PublicReportError::NotFound)
        ));
        assert!(render_by_token(&pool, &rotated.token, pinned_now()).await.is_ok());
    }

    #[tokio::test]
    async fn test_disabled_report_is_not_found_by_token() {
        let pool = create_test_pool().await;
        let (user_id, household_id) = user_with_household(&pool, "Kitchen").await;
        let report = create_report(&pool, &user_id, create("Report", vec![household_id]))
            .await
            .unwrap();

        update_report(
            &pool,
            &report.id,
            &user_id,
            UpdatePublicReportRequest {
                enabled: Some(false),
                ..Default::default()
            },
        )
        .await
        .unwrap();

        assert!(matches!(
            render_by_token(&pool, &report.token, pinned_now()).await,
            Err(PublicReportError::NotFound)
        ));

        // Re-enabling brings the same URL back — disabling does not rotate the token.
        update_report(
            &pool,
            &report.id,
            &user_id,
            UpdatePublicReportRequest {
                enabled: Some(true),
                ..Default::default()
            },
        )
        .await
        .unwrap();
        assert!(render_by_token(&pool, &report.token, pinned_now()).await.is_ok());
    }

    #[tokio::test]
    async fn test_unknown_token_is_not_found() {
        let pool = create_test_pool().await;
        assert!(matches!(
            render_by_token(&pool, &Uuid::new_v4(), pinned_now()).await,
            Err(PublicReportError::NotFound)
        ));
    }

    #[tokio::test]
    async fn test_delete_report_removes_it_and_its_selection() {
        let pool = create_test_pool().await;
        let (user_id, household_id) = user_with_household(&pool, "Kitchen").await;
        let report = create_report(&pool, &user_id, create("Report", vec![household_id]))
            .await
            .unwrap();

        delete_report(&pool, &report.id, &user_id).await.unwrap();

        assert!(matches!(
            get_report(&pool, &report.id, &user_id).await,
            Err(PublicReportError::NotFound)
        ));
        assert!(matches!(
            render_by_token(&pool, &report.token, pinned_now()).await,
            Err(PublicReportError::NotFound)
        ));
        assert!(load_household_ids(&pool, &report.id).await.unwrap().is_empty());
    }

    // ------------------------------------------------------------------
    // Rendering (PUBREP-03, PUBREP-04, PUBREP-06)
    // ------------------------------------------------------------------

    // ------------------------------------------------------------------
    // The title above the blocks
    // ------------------------------------------------------------------

    #[test]
    fn test_with_title_underlines_by_character_count() {
        // "Küche" is 5 characters but 6 bytes — a byte-sized underline would overhang.
        assert_eq!(with_title("Küche", "body"), "Küche\n=====\n\nbody");
    }

    #[test]
    fn test_with_title_trims_and_skips_an_empty_name() {
        assert_eq!(with_title("  Alles  ", "body"), "Alles\n=====\n\nbody");
        assert_eq!(with_title("   ", "body"), "body");
    }

    #[tokio::test]
    async fn test_render_starts_with_the_report_name() {
        let pool = create_test_pool().await;
        let (user_id, household_id) = user_with_household(&pool, "Kitchen").await;
        let report = create_report(&pool, &user_id, create("Weekday overview", vec![household_id]))
            .await
            .unwrap();

        let text = render_by_token(&pool, &report.token, pinned_now()).await.unwrap();

        assert!(
            text.starts_with("Weekday overview\n================\n\nDaily report — Kitchen"),
            "got: {text}"
        );
    }

    /// Renaming the report changes the title, so the text always identifies the report a
    /// reader is actually looking at.
    #[tokio::test]
    async fn test_render_title_follows_a_rename() {
        let pool = create_test_pool().await;
        let (user_id, household_id) = user_with_household(&pool, "Kitchen").await;
        let report = create_report(&pool, &user_id, create("Before", vec![household_id]))
            .await
            .unwrap();

        update_report(
            &pool,
            &report.id,
            &user_id,
            UpdatePublicReportRequest {
                name: Some("After".to_string()),
                ..Default::default()
            },
        )
        .await
        .unwrap();

        let text = render_by_token(&pool, &report.token, pinned_now()).await.unwrap();
        assert!(text.starts_with("After\n=====\n\n"), "got: {text}");
    }

    #[tokio::test]
    async fn test_render_titles_the_empty_selection_too() {
        let pool = create_test_pool().await;
        let (user_id, _) = user_with_household(&pool, "Kitchen").await;
        let report = create_report(&pool, &user_id, create("Nothing here", vec![]))
            .await
            .unwrap();

        let text = render_by_token(&pool, &report.token, pinned_now()).await.unwrap();
        assert_eq!(
            text,
            "Nothing here\n============\n\nNo households configured for this report"
        );
    }

    // ------------------------------------------------------------------
    // The "Missed yesterday" switch
    // ------------------------------------------------------------------

    #[tokio::test]
    async fn test_create_report_includes_the_missed_section_by_default() {
        let pool = create_test_pool().await;
        let (user_id, household_id) = user_with_household(&pool, "Kitchen").await;

        let report = create_report(&pool, &user_id, create("Report", vec![household_id]))
            .await
            .unwrap();

        assert!(report.include_missed);
        assert!(get_report(&pool, &report.id, &user_id).await.unwrap().include_missed);
    }

    #[tokio::test]
    async fn test_create_report_honours_an_explicit_missed_switch() {
        let pool = create_test_pool().await;
        let (user_id, household_id) = user_with_household(&pool, "Kitchen").await;

        let report = create_report(
            &pool,
            &user_id,
            CreatePublicReportRequest {
                name: "Today only".to_string(),
                language: None,
                include_missed: Some(false),
                separate_undated: None,
                household_ids: Some(vec![household_id]),
            },
        )
        .await
        .unwrap();

        assert!(!report.include_missed);
        let text = render_by_token(&pool, &report.token, pinned_now()).await.unwrap();
        assert!(text.contains("Due today:"), "got: {text}");
        assert!(!text.contains("Missed yesterday:"), "got: {text}");
    }

    #[tokio::test]
    async fn test_missed_switch_survives_a_round_trip_through_storage() {
        let pool = create_test_pool().await;
        let (user_id, household_id) = user_with_household(&pool, "Kitchen").await;
        let report = create_report(&pool, &user_id, create("Report", vec![household_id]))
            .await
            .unwrap();

        for expected in [false, true, false] {
            let updated = update_report(
                &pool,
                &report.id,
                &user_id,
                UpdatePublicReportRequest {
                    include_missed: Some(expected),
                    separate_undated: None,
                    ..Default::default()
                },
            )
            .await
            .unwrap();
            assert_eq!(updated.include_missed, expected);
            assert_eq!(
                get_report(&pool, &report.id, &user_id).await.unwrap().include_missed,
                expected
            );

            let text = render_by_token(&pool, &report.token, pinned_now()).await.unwrap();
            assert_eq!(text.contains("Missed yesterday:"), expected, "got: {text}");
        }
    }

    /// The switch must not be disturbed by an update that says nothing about it.
    #[tokio::test]
    async fn test_unrelated_update_leaves_the_missed_switch_alone() {
        let pool = create_test_pool().await;
        let (user_id, household_id) = user_with_household(&pool, "Kitchen").await;
        let report = create_report(
            &pool,
            &user_id,
            CreatePublicReportRequest {
                name: "Report".to_string(),
                language: None,
                include_missed: Some(false),
                separate_undated: None,
                household_ids: Some(vec![household_id]),
            },
        )
        .await
        .unwrap();

        let updated = update_report(
            &pool,
            &report.id,
            &user_id,
            UpdatePublicReportRequest {
                name: Some("Renamed".to_string()),
                ..Default::default()
            },
        )
        .await
        .unwrap();

        assert!(!updated.include_missed);
    }

    /// The switch applies to every household block, not just the first.
    #[tokio::test]
    async fn test_missed_switch_applies_to_all_blocks() {
        let pool = create_test_pool().await;
        let (user_id, kitchen) = user_with_household(&pool, "Kitchen").await;
        let garage = join(&pool, &user_id, "Garage").await;
        let report = create_report(
            &pool,
            &user_id,
            CreatePublicReportRequest {
                name: "Both".to_string(),
                language: None,
                include_missed: Some(false),
                separate_undated: None,
                household_ids: Some(vec![kitchen, garage]),
            },
        )
        .await
        .unwrap();

        let text = render_by_token(&pool, &report.token, pinned_now()).await.unwrap();

        assert_eq!(text.matches("Due today:").count(), 2, "got: {text}");
        assert_eq!(text.matches("Missed yesterday:").count(), 0, "got: {text}");
    }

    #[tokio::test]
    async fn test_missed_switch_respects_the_report_language() {
        let pool = create_test_pool().await;
        let (user_id, household_id) = user_with_household(&pool, "Küche").await;
        let report = create_report(
            &pool,
            &user_id,
            CreatePublicReportRequest {
                name: "Nur heute".to_string(),
                language: Some("de".to_string()),
                include_missed: Some(false),
                separate_undated: None,
                household_ids: Some(vec![household_id]),
            },
        )
        .await
        .unwrap();

        let text = render_by_token(&pool, &report.token, pinned_now()).await.unwrap();

        assert!(text.contains("Heute fällig:"), "got: {text}");
        assert!(!text.contains("Gestern verpasst:"), "got: {text}");
        assert!(!text.ends_with('\n'), "got: {text}");
    }

    #[tokio::test]
    async fn test_render_emits_one_block_per_household_alphabetically() {
        let pool = create_test_pool().await;
        // Created out of alphabetical order on purpose.
        let (user_id, kitchen) = user_with_household(&pool, "Kitchen").await;
        let attic = join(&pool, &user_id, "Attic").await;
        let garage = join(&pool, &user_id, "Garage").await;

        let report = create_report(
            &pool,
            &user_id,
            create("All", vec![kitchen, garage, attic]),
        )
        .await
        .unwrap();

        let text = render_by_token(&pool, &report.token, pinned_now()).await.unwrap();

        let headers: Vec<&str> = text
            .lines()
            .filter(|line| line.starts_with("Daily report — "))
            .collect();
        assert_eq!(
            headers,
            vec![
                "Daily report — Attic — Mon, 2027-01-04",
                "Daily report — Garage — Mon, 2027-01-04",
                "Daily report — Kitchen — Mon, 2027-01-04",
            ]
        );
    }

    #[tokio::test]
    async fn test_render_includes_task_lines_from_every_selected_household() {
        let pool = create_test_pool().await;
        let (user_id, kitchen) = user_with_household(&pool, "Kitchen").await;
        let garage = join(&pool, &user_id, "Garage").await;

        create_test_task(&pool, &kitchen)
            .with_title("Wash the dishes")
            .with_recurrence(RecurrenceType::Daily)
            .build()
            .await;
        create_test_task(&pool, &garage)
            .with_title("Sweep the floor")
            .with_recurrence(RecurrenceType::Daily)
            .build()
            .await;

        let report = create_report(&pool, &user_id, create("All", vec![kitchen, garage]))
            .await
            .unwrap();
        let text = render_by_token(&pool, &report.token, pinned_now()).await.unwrap();

        assert!(text.contains("- Wash the dishes"), "got: {text}");
        assert!(text.contains("- Sweep the floor"), "got: {text}");
    }

    /// PUBREP-04: the language is a property of the report, not of the endpoint.
    #[tokio::test]
    async fn test_render_uses_the_report_language() {
        let pool = create_test_pool().await;
        let (user_id, household_id) = user_with_household(&pool, "Küche").await;
        create_test_task(&pool, &household_id)
            .with_title("Staubsaugen")
            .with_recurrence(RecurrenceType::Daily)
            .build()
            .await;

        let report = create_report(
            &pool,
            &user_id,
            CreatePublicReportRequest {
                name: "Alles".to_string(),
                language: Some("de".to_string()),
                include_missed: None,
                separate_undated: None,
                household_ids: Some(vec![household_id]),
            },
        )
        .await
        .unwrap();

        let text = render_by_token(&pool, &report.token, pinned_now()).await.unwrap();

        assert!(
            text.starts_with("Alles\n=====\n\nTagesbericht — Küche — Mo, 2027-01-04"),
            "got: {text}"
        );
        assert!(text.contains("Heute fällig:"), "got: {text}");
        assert!(text.contains("- Staubsaugen"), "got: {text}");
    }

    /// PUBREP-06: a household the owner has left must vanish from the output, without
    /// taking the rest of the report with it.
    #[tokio::test]
    async fn test_render_drops_households_the_owner_left() {
        let pool = create_test_pool().await;
        let (user_id, kitchen) = user_with_household(&pool, "Kitchen").await;
        let garage = join(&pool, &user_id, "Garage").await;
        let report = create_report(&pool, &user_id, create("All", vec![kitchen, garage]))
            .await
            .unwrap();

        households::remove_member(&pool, &garage, &user_id).await.unwrap();

        let text = render_by_token(&pool, &report.token, pinned_now()).await.unwrap();

        assert!(text.contains("Daily report — Kitchen"), "got: {text}");
        assert!(!text.contains("Garage"), "got: {text}");
    }

    #[tokio::test]
    async fn test_render_falls_back_to_a_message_when_nothing_is_left() {
        let pool = create_test_pool().await;
        let (user_id, household_id) = user_with_household(&pool, "Kitchen").await;
        let report = create_report(&pool, &user_id, create("All", vec![household_id]))
            .await
            .unwrap();

        households::remove_member(&pool, &household_id, &user_id).await.unwrap();

        let text = render_by_token(&pool, &report.token, pinned_now()).await.unwrap();
        assert_eq!(text, "All\n===\n\nNo households configured for this report");
    }

    #[tokio::test]
    async fn test_render_empty_selection_uses_the_report_language() {
        let pool = create_test_pool().await;
        let (user_id, _) = user_with_household(&pool, "Küche").await;
        let report = create_report(
            &pool,
            &user_id,
            CreatePublicReportRequest {
                name: "Leer".to_string(),
                language: Some("de".to_string()),
                include_missed: None,
                separate_undated: None,
                household_ids: Some(vec![]),
            },
        )
        .await
        .unwrap();

        let text = render_by_token(&pool, &report.token, pinned_now()).await.unwrap();
        assert_eq!(
            text,
            "Leer\n====\n\nFür diesen Bericht sind keine Haushalte konfiguriert"
        );
    }

    /// D-04: each block resolves "today" in its OWN household's timezone, so two households
    /// either side of the dateline legitimately show different dates in one report.
    #[tokio::test]
    async fn test_render_resolves_each_household_in_its_own_timezone() {
        let pool = create_test_pool().await;
        let (user_id, auckland) = user_with_household(&pool, "Auckland").await;
        let honolulu = join(&pool, &user_id, "Honolulu").await;
        set_household_timezone(&pool, &auckland, "Pacific/Auckland").await;
        set_household_timezone(&pool, &honolulu, "Pacific/Honolulu").await;

        let report = create_report(&pool, &user_id, create("Both", vec![auckland, honolulu]))
            .await
            .unwrap();

        // 2027-01-04 12:00 UTC is already the 5th in Auckland and still the 4th in Honolulu.
        let text = render_by_token(&pool, &report.token, pinned_now()).await.unwrap();

        assert!(text.contains("Daily report — Auckland — Tue, 2027-01-05"), "got: {text}");
        assert!(text.contains("Daily report — Honolulu — Mon, 2027-01-04"), "got: {text}");
    }

    /// D-02: exactly one blank line between blocks, and no trailing newline — the shape the
    /// per-household report established.
    #[tokio::test]
    async fn test_render_separates_blocks_with_one_blank_line() {
        let pool = create_test_pool().await;
        let (user_id, kitchen) = user_with_household(&pool, "Kitchen").await;
        let garage = join(&pool, &user_id, "Garage").await;
        let report = create_report(&pool, &user_id, create("All", vec![kitchen, garage]))
            .await
            .unwrap();

        let text = render_by_token(&pool, &report.token, pinned_now()).await.unwrap();

        assert!(
            text.contains("All tasks completed yesterday\n\nDaily report — Kitchen"),
            "got: {text}"
        );
        assert!(!text.ends_with('\n'), "got: {text}");
    }

    /// A report only ever renders its OWNER's view — never the tasks of whoever opens the
    /// link, and never another member's assignments.
    #[tokio::test]
    async fn test_render_shows_only_the_owners_tasks() {
        let pool = create_test_pool().await;
        let (owner, household_id) = user_with_household(&pool, "Kitchen").await;
        let housemate = create_test_user(&pool, "housemate@test.com", Role::Member).await;
        create_test_membership(&pool, &household_id, &housemate, Role::Member).await;

        create_test_task(&pool, &household_id)
            .with_title("Owner's chore")
            .with_recurrence(RecurrenceType::Daily)
            .with_assigned_user(owner)
            .build()
            .await;
        create_test_task(&pool, &household_id)
            .with_title("Housemate's chore")
            .with_recurrence(RecurrenceType::Daily)
            .with_assigned_user(housemate)
            .build()
            .await;

        let report = create_report(&pool, &owner, create("Mine", vec![household_id]))
            .await
            .unwrap();
        let text = render_by_token(&pool, &report.token, pinned_now()).await.unwrap();

        assert!(text.contains("- Owner's chore"), "got: {text}");
        assert!(!text.contains("Housemate's chore"), "got: {text}");
    }
}
