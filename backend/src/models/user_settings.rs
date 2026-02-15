use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct UserSettingsRow {
    pub user_id: String,
    pub language: String,
    pub updated_at: DateTime<Utc>,
}

impl UserSettingsRow {
    pub fn to_shared(&self) -> shared::UserSettings {
        shared::UserSettings {
            user_id: Uuid::parse_str(&self.user_id).unwrap(),
            language: self.language.clone(),
            updated_at: self.updated_at,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_user_settings_row_to_shared() {
        let now = Utc::now();
        let user_id = Uuid::new_v4();

        let row = UserSettingsRow {
            user_id: user_id.to_string(),
            language: "de".to_string(),
            updated_at: now,
        };

        let shared = row.to_shared();

        assert_eq!(shared.user_id, user_id);
        assert_eq!(shared.language, "de");
    }
}
