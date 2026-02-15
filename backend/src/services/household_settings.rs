use chrono::Utc;
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
    sqlx::query(
        r#"
        INSERT INTO household_settings (household_id, dark_mode, role_label_owner, role_label_admin, role_label_member, hierarchy_type, updated_at)
        VALUES (?, FALSE, 'Owner', 'Admin', 'Member', ?, ?)
        "#,
    )
    .bind(household_id.to_string())
    .bind(default_hierarchy.as_str())
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

    let now = Utc::now();
    settings.updated_at = now;

    sqlx::query(
        r#"
        UPDATE household_settings
        SET dark_mode = ?, role_label_owner = ?, role_label_admin = ?, role_label_member = ?, hierarchy_type = ?, updated_at = ?
        WHERE household_id = ?
        "#,
    )
    .bind(settings.dark_mode)
    .bind(&settings.role_label_owner)
    .bind(&settings.role_label_admin)
    .bind(&settings.role_label_member)
    .bind(settings.hierarchy_type.as_str())
    .bind(now)
    .bind(household_id.to_string())
    .execute(pool)
    .await?;

    Ok(settings)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_settings_error_display() {
        let error = SettingsError::DatabaseError(sqlx::Error::RowNotFound);
        assert!(error.to_string().contains("Database error"));
    }
}
