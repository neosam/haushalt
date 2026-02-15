use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;

/// Database model for refresh tokens
#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct RefreshTokenRow {
    pub id: String,
    pub user_id: String,
    pub token_hash: String,
    pub expires_at: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    #[test]
    fn test_refresh_token_row_fields() {
        let now = Utc::now();
        let id = Uuid::new_v4();
        let user_id = Uuid::new_v4();

        let row = RefreshTokenRow {
            id: id.to_string(),
            user_id: user_id.to_string(),
            token_hash: "abc123hash".to_string(),
            expires_at: now,
            created_at: now,
        };

        assert_eq!(row.id, id.to_string());
        assert_eq!(row.user_id, user_id.to_string());
        assert_eq!(row.token_hash, "abc123hash");
    }
}
