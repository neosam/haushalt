use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

/// Database model for chat messages
#[allow(dead_code)]
#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct ChatMessageRow {
    pub id: String,
    pub household_id: String,
    pub user_id: String,
    pub content: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub deleted_at: Option<DateTime<Utc>>,
}

impl ChatMessageRow {
    #[allow(dead_code)]
    pub fn to_shared(&self) -> shared::ChatMessage {
        shared::ChatMessage {
            id: Uuid::parse_str(&self.id).unwrap(),
            household_id: Uuid::parse_str(&self.household_id).unwrap(),
            user_id: Uuid::parse_str(&self.user_id).unwrap(),
            content: self.content.clone(),
            created_at: self.created_at,
            updated_at: self.updated_at,
            is_deleted: self.deleted_at.is_some(),
        }
    }
}

/// Database model for chat messages with user info (for JOIN queries)
#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct ChatMessageWithUserRow {
    // Chat message fields
    pub id: String,
    pub household_id: String,
    pub user_id: String,
    pub content: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub deleted_at: Option<DateTime<Utc>>,
    // User fields
    pub username: String,
    pub email: String,
    pub user_created_at: DateTime<Utc>,
    pub user_updated_at: DateTime<Utc>,
}

impl ChatMessageWithUserRow {
    pub fn to_shared(&self) -> shared::ChatMessageWithUser {
        shared::ChatMessageWithUser {
            message: shared::ChatMessage {
                id: Uuid::parse_str(&self.id).unwrap(),
                household_id: Uuid::parse_str(&self.household_id).unwrap(),
                user_id: Uuid::parse_str(&self.user_id).unwrap(),
                content: self.content.clone(),
                created_at: self.created_at,
                updated_at: self.updated_at,
                is_deleted: self.deleted_at.is_some(),
            },
            user: shared::User {
                id: Uuid::parse_str(&self.user_id).unwrap(),
                username: self.username.clone(),
                email: self.email.clone(),
                created_at: self.user_created_at,
                updated_at: self.user_updated_at,
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_chat_message_row_to_shared() {
        let now = Utc::now();
        let id = Uuid::new_v4();
        let household_id = Uuid::new_v4();
        let user_id = Uuid::new_v4();

        let row = ChatMessageRow {
            id: id.to_string(),
            household_id: household_id.to_string(),
            user_id: user_id.to_string(),
            content: "Hello, world!".to_string(),
            created_at: now,
            updated_at: now,
            deleted_at: None,
        };

        let shared = row.to_shared();

        assert_eq!(shared.id, id);
        assert_eq!(shared.household_id, household_id);
        assert_eq!(shared.user_id, user_id);
        assert_eq!(shared.content, "Hello, world!");
        assert!(!shared.is_deleted);
    }

    #[test]
    fn test_chat_message_row_deleted() {
        let now = Utc::now();
        let id = Uuid::new_v4();
        let household_id = Uuid::new_v4();
        let user_id = Uuid::new_v4();

        let row = ChatMessageRow {
            id: id.to_string(),
            household_id: household_id.to_string(),
            user_id: user_id.to_string(),
            content: "[deleted]".to_string(),
            created_at: now,
            updated_at: now,
            deleted_at: Some(now),
        };

        let shared = row.to_shared();
        assert!(shared.is_deleted);
    }

    #[test]
    fn test_chat_message_with_user_row_to_shared() {
        let now = Utc::now();
        let id = Uuid::new_v4();
        let household_id = Uuid::new_v4();
        let user_id = Uuid::new_v4();

        let row = ChatMessageWithUserRow {
            id: id.to_string(),
            household_id: household_id.to_string(),
            user_id: user_id.to_string(),
            content: "Hello!".to_string(),
            created_at: now,
            updated_at: now,
            deleted_at: None,
            username: "testuser".to_string(),
            email: "test@example.com".to_string(),
            user_created_at: now,
            user_updated_at: now,
        };

        let shared = row.to_shared();

        assert_eq!(shared.message.id, id);
        assert_eq!(shared.message.content, "Hello!");
        assert_eq!(shared.user.id, user_id);
        assert_eq!(shared.user.username, "testuser");
    }
}
