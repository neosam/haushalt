//! Solo Mode Service
//!
//! Solo Mode is a self-discipline feature where all household members are treated
//! as Members with restricted permissions. Tasks can only be completed or suggested,
//! and suggestions are auto-accepted with household default rewards/punishments.

use chrono::Utc;
use sqlx::SqlitePool;
use thiserror::Error;
use uuid::Uuid;

use shared::{HierarchyType, HouseholdSettings, Role};

use super::household_settings::{get_or_create_settings, SettingsError};

/// Cooldown duration in hours before Solo Mode can be exited
#[allow(dead_code)]
pub const SOLO_MODE_COOLDOWN_HOURS: i64 = 48;

#[derive(Debug, Error)]
pub enum SoloModeError {
    #[error("Database error: {0}")]
    DatabaseError(#[from] sqlx::Error),
    #[error("Settings error: {0}")]
    SettingsError(#[from] SettingsError),
    #[error("Solo Mode is not active")]
    NotActive,
    #[error("Solo Mode is already active")]
    AlreadyActive,
    #[error("No exit request is pending")]
    NoExitPending,
    #[error("Exit request is already pending")]
    ExitAlreadyPending,
}

/// Check if a role can manage tasks/rewards/punishments considering Solo Mode
///
/// In Solo Mode, nobody can manage - all users are treated as Members.
/// Outside Solo Mode, uses the standard hierarchy type check.
pub fn can_manage_in_context(role: &Role, settings: &HouseholdSettings) -> bool {
    if settings.solo_mode {
        return false; // Nobody can manage in Solo Mode
    }
    settings.hierarchy_type.can_manage(role)
}

/// Activate Solo Mode for a household
///
/// Saves the current hierarchy type and enables Solo Mode.
pub async fn activate_solo_mode(
    pool: &SqlitePool,
    household_id: &Uuid,
) -> Result<HouseholdSettings, SoloModeError> {
    let household_id_str = household_id.to_string();
    let settings = get_or_create_settings(pool, household_id).await?;

    if settings.solo_mode {
        return Err(SoloModeError::AlreadyActive);
    }

    let now = Utc::now();
    let previous_hierarchy_type = settings.hierarchy_type.as_str();

    sqlx::query(
        r#"
        UPDATE household_settings
        SET solo_mode = TRUE,
            solo_mode_exit_requested_at = NULL,
            solo_mode_previous_hierarchy_type = ?,
            updated_at = ?
        WHERE household_id = ?
        "#,
    )
    .bind(previous_hierarchy_type)
    .bind(now)
    .bind(&household_id_str)
    .execute(pool)
    .await?;

    // Return updated settings
    get_or_create_settings(pool, household_id)
        .await
        .map_err(SoloModeError::from)
}

/// Request to exit Solo Mode (starts 48-hour cooldown)
pub async fn request_solo_mode_exit(
    pool: &SqlitePool,
    household_id: &Uuid,
) -> Result<HouseholdSettings, SoloModeError> {
    let household_id_str = household_id.to_string();
    let settings = get_or_create_settings(pool, household_id).await?;

    if !settings.solo_mode {
        return Err(SoloModeError::NotActive);
    }

    if settings.solo_mode_exit_requested_at.is_some() {
        return Err(SoloModeError::ExitAlreadyPending);
    }

    let now = Utc::now();

    sqlx::query(
        r#"
        UPDATE household_settings
        SET solo_mode_exit_requested_at = ?,
            updated_at = ?
        WHERE household_id = ?
        "#,
    )
    .bind(now)
    .bind(now)
    .bind(&household_id_str)
    .execute(pool)
    .await?;

    // Return updated settings
    get_or_create_settings(pool, household_id)
        .await
        .map_err(SoloModeError::from)
}

/// Cancel a pending Solo Mode exit request
pub async fn cancel_solo_mode_exit(
    pool: &SqlitePool,
    household_id: &Uuid,
) -> Result<HouseholdSettings, SoloModeError> {
    let household_id_str = household_id.to_string();
    let settings = get_or_create_settings(pool, household_id).await?;

    if !settings.solo_mode {
        return Err(SoloModeError::NotActive);
    }

    if settings.solo_mode_exit_requested_at.is_none() {
        return Err(SoloModeError::NoExitPending);
    }

    let now = Utc::now();

    sqlx::query(
        r#"
        UPDATE household_settings
        SET solo_mode_exit_requested_at = NULL,
            updated_at = ?
        WHERE household_id = ?
        "#,
    )
    .bind(now)
    .bind(&household_id_str)
    .execute(pool)
    .await?;

    // Return updated settings
    get_or_create_settings(pool, household_id)
        .await
        .map_err(SoloModeError::from)
}

/// Deactivate Solo Mode and restore previous hierarchy type
async fn deactivate_solo_mode(
    pool: &SqlitePool,
    household_id: &Uuid,
) -> Result<HouseholdSettings, SoloModeError> {
    let household_id_str = household_id.to_string();
    let settings = get_or_create_settings(pool, household_id).await?;

    if !settings.solo_mode {
        return Err(SoloModeError::NotActive);
    }

    let now = Utc::now();

    // Restore previous hierarchy type or default to Organized
    let restored_hierarchy = settings
        .solo_mode_previous_hierarchy_type
        .unwrap_or(HierarchyType::Organized);

    sqlx::query(
        r#"
        UPDATE household_settings
        SET solo_mode = FALSE,
            solo_mode_exit_requested_at = NULL,
            solo_mode_previous_hierarchy_type = NULL,
            hierarchy_type = ?,
            updated_at = ?
        WHERE household_id = ?
        "#,
    )
    .bind(restored_hierarchy.as_str())
    .bind(now)
    .bind(&household_id_str)
    .execute(pool)
    .await?;

    // Return updated settings
    get_or_create_settings(pool, household_id)
        .await
        .map_err(SoloModeError::from)
}

/// Check for expired Solo Mode exit requests and deactivate them
///
/// Returns a list of household IDs that were deactivated.
pub async fn check_and_deactivate_expired_solo_modes(
    pool: &SqlitePool,
) -> Result<Vec<Uuid>, SoloModeError> {
    let now = Utc::now();
    let cooldown_duration = chrono::Duration::hours(SOLO_MODE_COOLDOWN_HOURS);
    let cutoff_time = now - cooldown_duration;

    // Find all households with expired exit requests
    let expired_households: Vec<(String,)> = sqlx::query_as(
        r#"
        SELECT household_id
        FROM household_settings
        WHERE solo_mode = TRUE
          AND solo_mode_exit_requested_at IS NOT NULL
          AND solo_mode_exit_requested_at <= ?
        "#,
    )
    .bind(cutoff_time)
    .fetch_all(pool)
    .await?;

    let mut deactivated = Vec::new();

    for (household_id_str,) in expired_households {
        if let Ok(household_id) = Uuid::parse_str(&household_id_str) {
            match deactivate_solo_mode(pool, &household_id).await {
                Ok(_) => {
                    deactivated.push(household_id);
                }
                Err(e) => {
                    // Log error but continue with other households
                    eprintln!(
                        "Failed to deactivate Solo Mode for household {}: {}",
                        household_id, e
                    );
                }
            }
        }
    }

    Ok(deactivated)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_can_manage_in_context_solo_mode_active() {
        let settings = HouseholdSettings {
            solo_mode: true,
            hierarchy_type: HierarchyType::Equals,
            ..Default::default()
        };

        // In Solo Mode, nobody can manage
        assert!(!can_manage_in_context(&Role::Owner, &settings));
        assert!(!can_manage_in_context(&Role::Admin, &settings));
        assert!(!can_manage_in_context(&Role::Member, &settings));
    }

    #[test]
    fn test_can_manage_in_context_solo_mode_inactive() {
        let settings_equals = HouseholdSettings {
            solo_mode: false,
            hierarchy_type: HierarchyType::Equals,
            ..Default::default()
        };

        // In Equals mode, everyone can manage
        assert!(can_manage_in_context(&Role::Owner, &settings_equals));
        assert!(can_manage_in_context(&Role::Admin, &settings_equals));
        assert!(can_manage_in_context(&Role::Member, &settings_equals));

        let settings_organized = HouseholdSettings {
            solo_mode: false,
            hierarchy_type: HierarchyType::Organized,
            ..Default::default()
        };

        // In Organized mode, only Owner/Admin can manage
        assert!(can_manage_in_context(&Role::Owner, &settings_organized));
        assert!(can_manage_in_context(&Role::Admin, &settings_organized));
        assert!(!can_manage_in_context(&Role::Member, &settings_organized));
    }

    #[test]
    fn test_solo_mode_error_display() {
        let error = SoloModeError::NotActive;
        assert_eq!(error.to_string(), "Solo Mode is not active");

        let error = SoloModeError::AlreadyActive;
        assert_eq!(error.to_string(), "Solo Mode is already active");

        let error = SoloModeError::NoExitPending;
        assert_eq!(error.to_string(), "No exit request is pending");

        let error = SoloModeError::ExitAlreadyPending;
        assert_eq!(error.to_string(), "Exit request is already pending");
    }
}
