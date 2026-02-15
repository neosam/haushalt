use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

/// Database model for notes
#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct NoteRow {
    pub id: String,
    pub household_id: String,
    pub user_id: String,
    pub title: String,
    pub content: String,
    pub is_shared: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl NoteRow {
    pub fn to_shared(&self) -> shared::Note {
        shared::Note {
            id: Uuid::parse_str(&self.id).unwrap(),
            household_id: Uuid::parse_str(&self.household_id).unwrap(),
            user_id: Uuid::parse_str(&self.user_id).unwrap(),
            title: self.title.clone(),
            content: self.content.clone(),
            is_shared: self.is_shared,
            created_at: self.created_at,
            updated_at: self.updated_at,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_note_row_to_shared() {
        let now = Utc::now();
        let id = Uuid::new_v4();
        let household_id = Uuid::new_v4();
        let user_id = Uuid::new_v4();

        let row = NoteRow {
            id: id.to_string(),
            household_id: household_id.to_string(),
            user_id: user_id.to_string(),
            title: "Test Note".to_string(),
            content: "# Hello\n\nThis is a test note.".to_string(),
            is_shared: true,
            created_at: now,
            updated_at: now,
        };

        let shared = row.to_shared();

        assert_eq!(shared.id, id);
        assert_eq!(shared.household_id, household_id);
        assert_eq!(shared.user_id, user_id);
        assert_eq!(shared.title, "Test Note");
        assert_eq!(shared.content, "# Hello\n\nThis is a test note.");
        assert!(shared.is_shared);
    }

    #[test]
    fn test_note_row_private() {
        let now = Utc::now();

        let row = NoteRow {
            id: Uuid::new_v4().to_string(),
            household_id: Uuid::new_v4().to_string(),
            user_id: Uuid::new_v4().to_string(),
            title: "Private Note".to_string(),
            content: "Secret content".to_string(),
            is_shared: false,
            created_at: now,
            updated_at: now,
        };

        let shared = row.to_shared();

        assert!(!shared.is_shared);
    }
}
