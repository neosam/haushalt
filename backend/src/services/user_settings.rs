use chrono::Utc;
use sqlx::SqlitePool;
use thiserror::Error;
use uuid::Uuid;

use crate::models::UserSettingsRow;
use shared::{UpdateUserSettingsRequest, UserSettings};

#[derive(Debug, Error)]
pub enum UserSettingsError {
    #[error("Database error: {0}")]
    DatabaseError(#[from] sqlx::Error),
    #[error("Invalid language code")]
    InvalidLanguage,
}

/// Supported language codes
const SUPPORTED_LANGUAGES: &[&str] = &["en", "de"];

/// Validate that a language code is supported
fn validate_language(lang: &str) -> bool {
    SUPPORTED_LANGUAGES.contains(&lang)
}

/// Get settings for a user, creating defaults if they don't exist
pub async fn get_or_create_settings(
    pool: &SqlitePool,
    user_id: &Uuid,
) -> Result<UserSettings, UserSettingsError> {
    // Try to fetch existing settings
    let existing: Option<UserSettingsRow> = sqlx::query_as(
        "SELECT * FROM user_settings WHERE user_id = ?"
    )
    .bind(user_id.to_string())
    .fetch_optional(pool)
    .await?;

    if let Some(settings) = existing {
        return Ok(settings.to_shared());
    }

    // Create default settings
    let now = Utc::now();
    let default_language = "en";
    sqlx::query(
        r#"
        INSERT INTO user_settings (user_id, language, updated_at)
        VALUES (?, ?, ?)
        "#,
    )
    .bind(user_id.to_string())
    .bind(default_language)
    .bind(now)
    .execute(pool)
    .await?;

    Ok(UserSettings {
        user_id: *user_id,
        language: default_language.to_string(),
        updated_at: now,
    })
}

/// Update user settings
pub async fn update_settings(
    pool: &SqlitePool,
    user_id: &Uuid,
    request: &UpdateUserSettingsRequest,
) -> Result<UserSettings, UserSettingsError> {
    // Ensure settings exist first
    let mut settings = get_or_create_settings(pool, user_id).await?;

    // Apply updates
    if let Some(ref language) = request.language {
        if !validate_language(language) {
            return Err(UserSettingsError::InvalidLanguage);
        }
        settings.language = language.clone();
    }

    let now = Utc::now();
    settings.updated_at = now;

    sqlx::query(
        r#"
        UPDATE user_settings
        SET language = ?, updated_at = ?
        WHERE user_id = ?
        "#,
    )
    .bind(&settings.language)
    .bind(now)
    .bind(user_id.to_string())
    .execute(pool)
    .await?;

    Ok(settings)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_language() {
        assert!(validate_language("en"));
        assert!(validate_language("de"));
        assert!(!validate_language("fr"));
        assert!(!validate_language(""));
    }

    #[test]
    fn test_user_settings_error_display() {
        let error = UserSettingsError::InvalidLanguage;
        assert_eq!(error.to_string(), "Invalid language code");
    }
}
