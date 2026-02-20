use chrono::{NaiveDate, Utc};
use sqlx::SqlitePool;
use thiserror::Error;
use uuid::Uuid;

use crate::models::HouseholdSettingsRow;
use shared::{HierarchyType, HouseholdSettings, UpdateHouseholdSettingsRequest};

#[derive(Debug, Error)]
pub enum SettingsError {
    #[error("Database error: {0}")]
    DatabaseError(#[from] sqlx::Error),
}

/// Get settings for a household, creating defaults if they don't exist
pub async fn get_or_create_settings(
    pool: &SqlitePool,
    household_id: &Uuid,
) -> Result<HouseholdSettings, SettingsError> {
    // Try to fetch existing settings
    let existing: Option<HouseholdSettingsRow> = sqlx::query_as(
        "SELECT * FROM household_settings WHERE household_id = ?"
    )
    .bind(household_id.to_string())
    .fetch_optional(pool)
    .await?;

    if let Some(settings) = existing {
        return Ok(settings.to_shared());
    }

    // Create default settings
    let now = Utc::now();
    let default_hierarchy = HierarchyType::default();
    let default_timezone = "UTC";
    sqlx::query(
        r#"
        INSERT INTO household_settings (household_id, dark_mode, role_label_owner, role_label_admin, role_label_member, hierarchy_type, timezone, rewards_enabled, punishments_enabled, chat_enabled, vacation_mode, vacation_start, vacation_end, auto_archive_days, allow_task_suggestions, updated_at)
        VALUES (?, FALSE, 'Owner', 'Admin', 'Member', ?, ?, FALSE, FALSE, FALSE, FALSE, NULL, NULL, 7, TRUE, ?)
        "#,
    )
    .bind(household_id.to_string())
    .bind(default_hierarchy.as_str())
    .bind(default_timezone)
    .bind(now)
    .execute(pool)
    .await?;

    Ok(HouseholdSettings {
        household_id: *household_id,
        dark_mode: false,
        role_label_owner: "Owner".to_string(),
        role_label_admin: "Admin".to_string(),
        role_label_member: "Member".to_string(),
        hierarchy_type: default_hierarchy,
        timezone: default_timezone.to_string(),
        rewards_enabled: false,
        punishments_enabled: false,
        chat_enabled: false,
        vacation_mode: false,
        vacation_start: None,
        vacation_end: None,
        auto_archive_days: Some(7),
        allow_task_suggestions: true,
        updated_at: now,
    })
}

/// Update household settings
pub async fn update_settings(
    pool: &SqlitePool,
    household_id: &Uuid,
    request: &UpdateHouseholdSettingsRequest,
) -> Result<HouseholdSettings, SettingsError> {
    // Ensure settings exist first
    let mut settings = get_or_create_settings(pool, household_id).await?;

    // Apply updates
    if let Some(dark_mode) = request.dark_mode {
        settings.dark_mode = dark_mode;
    }
    if let Some(ref label) = request.role_label_owner {
        settings.role_label_owner = label.clone();
    }
    if let Some(ref label) = request.role_label_admin {
        settings.role_label_admin = label.clone();
    }
    if let Some(ref label) = request.role_label_member {
        settings.role_label_member = label.clone();
    }
    if let Some(hierarchy_type) = request.hierarchy_type {
        settings.hierarchy_type = hierarchy_type;
    }
    if let Some(ref timezone) = request.timezone {
        settings.timezone = timezone.clone();
    }
    if let Some(rewards_enabled) = request.rewards_enabled {
        settings.rewards_enabled = rewards_enabled;
    }
    if let Some(punishments_enabled) = request.punishments_enabled {
        settings.punishments_enabled = punishments_enabled;
    }
    if let Some(chat_enabled) = request.chat_enabled {
        settings.chat_enabled = chat_enabled;
    }
    if let Some(vacation_mode) = request.vacation_mode {
        settings.vacation_mode = vacation_mode;
    }
    if let Some(ref vacation_start) = request.vacation_start {
        settings.vacation_start = *vacation_start;
    }
    if let Some(ref vacation_end) = request.vacation_end {
        settings.vacation_end = *vacation_end;
    }
    if let Some(ref auto_archive_days) = request.auto_archive_days {
        settings.auto_archive_days = *auto_archive_days;
    }
    if let Some(allow_task_suggestions) = request.allow_task_suggestions {
        settings.allow_task_suggestions = allow_task_suggestions;
    }

    let now = Utc::now();
    settings.updated_at = now;

    sqlx::query(
        r#"
        UPDATE household_settings
        SET dark_mode = ?, role_label_owner = ?, role_label_admin = ?, role_label_member = ?, hierarchy_type = ?, timezone = ?, rewards_enabled = ?, punishments_enabled = ?, chat_enabled = ?, vacation_mode = ?, vacation_start = ?, vacation_end = ?, auto_archive_days = ?, allow_task_suggestions = ?, updated_at = ?
        WHERE household_id = ?
        "#,
    )
    .bind(settings.dark_mode)
    .bind(&settings.role_label_owner)
    .bind(&settings.role_label_admin)
    .bind(&settings.role_label_member)
    .bind(settings.hierarchy_type.as_str())
    .bind(&settings.timezone)
    .bind(settings.rewards_enabled)
    .bind(settings.punishments_enabled)
    .bind(settings.chat_enabled)
    .bind(settings.vacation_mode)
    .bind(settings.vacation_start)
    .bind(settings.vacation_end)
    .bind(settings.auto_archive_days)
    .bind(settings.allow_task_suggestions)
    .bind(now)
    .bind(household_id.to_string())
    .execute(pool)
    .await?;

    Ok(settings)
}

/// Check if a household is currently on vacation
///
/// Returns true if vacation_mode is enabled AND the current date falls within
/// the vacation period (if dates are specified).
pub fn is_household_on_vacation(settings: &HouseholdSettings, today: NaiveDate) -> bool {
    if !settings.vacation_mode {
        return false;
    }

    // If dates are set, check if we're within the range
    match (settings.vacation_start, settings.vacation_end) {
        (Some(start), Some(end)) => today >= start && today <= end,
        (Some(start), None) => today >= start,
        (None, Some(end)) => today <= end,
        (None, None) => true, // vacation_mode on with no dates = indefinite
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_settings_error_display() {
        let error = SettingsError::DatabaseError(sqlx::Error::RowNotFound);
        assert!(error.to_string().contains("Database error"));
    }

    #[test]
    fn test_is_household_on_vacation_mode_off() {
        let settings = HouseholdSettings {
            vacation_mode: false,
            vacation_start: None,
            vacation_end: None,
            ..Default::default()
        };
        let today = NaiveDate::from_ymd_opt(2025, 1, 15).unwrap();
        assert!(!is_household_on_vacation(&settings, today));
    }

    #[test]
    fn test_is_household_on_vacation_mode_on_no_dates() {
        let settings = HouseholdSettings {
            vacation_mode: true,
            vacation_start: None,
            vacation_end: None,
            ..Default::default()
        };
        let today = NaiveDate::from_ymd_opt(2025, 1, 15).unwrap();
        assert!(is_household_on_vacation(&settings, today));
    }

    #[test]
    fn test_is_household_on_vacation_within_range() {
        let settings = HouseholdSettings {
            vacation_mode: true,
            vacation_start: Some(NaiveDate::from_ymd_opt(2025, 1, 10).unwrap()),
            vacation_end: Some(NaiveDate::from_ymd_opt(2025, 1, 20).unwrap()),
            ..Default::default()
        };

        // Before range
        let before = NaiveDate::from_ymd_opt(2025, 1, 5).unwrap();
        assert!(!is_household_on_vacation(&settings, before));

        // Within range
        let within = NaiveDate::from_ymd_opt(2025, 1, 15).unwrap();
        assert!(is_household_on_vacation(&settings, within));

        // On start date
        let start = NaiveDate::from_ymd_opt(2025, 1, 10).unwrap();
        assert!(is_household_on_vacation(&settings, start));

        // On end date
        let end = NaiveDate::from_ymd_opt(2025, 1, 20).unwrap();
        assert!(is_household_on_vacation(&settings, end));

        // After range
        let after = NaiveDate::from_ymd_opt(2025, 1, 25).unwrap();
        assert!(!is_household_on_vacation(&settings, after));
    }

    #[test]
    fn test_is_household_on_vacation_start_only() {
        let settings = HouseholdSettings {
            vacation_mode: true,
            vacation_start: Some(NaiveDate::from_ymd_opt(2025, 1, 10).unwrap()),
            vacation_end: None,
            ..Default::default()
        };

        // Before start
        let before = NaiveDate::from_ymd_opt(2025, 1, 5).unwrap();
        assert!(!is_household_on_vacation(&settings, before));

        // On start date
        let start = NaiveDate::from_ymd_opt(2025, 1, 10).unwrap();
        assert!(is_household_on_vacation(&settings, start));

        // After start (indefinite)
        let after = NaiveDate::from_ymd_opt(2025, 6, 15).unwrap();
        assert!(is_household_on_vacation(&settings, after));
    }

    #[test]
    fn test_is_household_on_vacation_end_only() {
        let settings = HouseholdSettings {
            vacation_mode: true,
            vacation_start: None,
            vacation_end: Some(NaiveDate::from_ymd_opt(2025, 1, 20).unwrap()),
            ..Default::default()
        };

        // Before end
        let before = NaiveDate::from_ymd_opt(2025, 1, 5).unwrap();
        assert!(is_household_on_vacation(&settings, before));

        // On end date
        let end = NaiveDate::from_ymd_opt(2025, 1, 20).unwrap();
        assert!(is_household_on_vacation(&settings, end));

        // After end
        let after = NaiveDate::from_ymd_opt(2025, 1, 25).unwrap();
        assert!(!is_household_on_vacation(&settings, after));
    }
}
