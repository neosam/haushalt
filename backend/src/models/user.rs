use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

/// Database model for users
#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct UserRow {
    pub id: String,
    pub username: String,
    pub email: String,
    pub password_hash: Option<String>,
    pub oidc_subject: Option<String>,
    pub oidc_provider: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl UserRow {
    pub fn to_shared(&self) -> shared::User {
        shared::User {
            id: Uuid::parse_str(&self.id).unwrap(),
            username: self.username.clone(),
            email: self.email.clone(),
            created_at: self.created_at,
            updated_at: self.updated_at,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_user_row_to_shared() {
        let now = Utc::now();
        let id = Uuid::new_v4();

        let row = UserRow {
            id: id.to_string(),
            username: "testuser".to_string(),
            email: "test@example.com".to_string(),
            password_hash: Some("hashed".to_string()),
            oidc_subject: None,
            oidc_provider: None,
            created_at: now,
            updated_at: now,
        };

        let shared = row.to_shared();

        assert_eq!(shared.id, id);
        assert_eq!(shared.username, "testuser");
        assert_eq!(shared.email, "test@example.com");
    }
}
