use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

/// Database model for journal entries
#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct JournalEntryRow {
    pub id: String,
    pub household_id: String,
    pub user_id: String,
    pub title: String,
    pub content: String,
    pub entry_date: NaiveDate,
    pub is_shared: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl JournalEntryRow {
    pub fn to_shared(&self) -> shared::JournalEntry {
        shared::JournalEntry {
            id: Uuid::parse_str(&self.id).unwrap(),
            household_id: Uuid::parse_str(&self.household_id).unwrap(),
            user_id: Uuid::parse_str(&self.user_id).unwrap(),
            title: self.title.clone(),
            content: self.content.clone(),
            entry_date: self.entry_date,
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
    fn test_journal_entry_row_to_shared() {
        let now = Utc::now();
        let id = Uuid::new_v4();
        let household_id = Uuid::new_v4();
        let user_id = Uuid::new_v4();
        let entry_date = NaiveDate::from_ymd_opt(2026, 2, 18).unwrap();

        let row = JournalEntryRow {
            id: id.to_string(),
            household_id: household_id.to_string(),
            user_id: user_id.to_string(),
            title: "My Day".to_string(),
            content: "Today was a great day!".to_string(),
            entry_date,
            is_shared: true,
            created_at: now,
            updated_at: now,
        };

        let shared = row.to_shared();

        assert_eq!(shared.id, id);
        assert_eq!(shared.household_id, household_id);
        assert_eq!(shared.user_id, user_id);
        assert_eq!(shared.title, "My Day");
        assert_eq!(shared.content, "Today was a great day!");
        assert_eq!(shared.entry_date, entry_date);
        assert!(shared.is_shared);
    }

    #[test]
    fn test_journal_entry_row_private() {
        let now = Utc::now();
        let entry_date = NaiveDate::from_ymd_opt(2026, 2, 18).unwrap();

        let row = JournalEntryRow {
            id: Uuid::new_v4().to_string(),
            household_id: Uuid::new_v4().to_string(),
            user_id: Uuid::new_v4().to_string(),
            title: "Private thoughts".to_string(),
            content: "Secret content".to_string(),
            entry_date,
            is_shared: false,
            created_at: now,
            updated_at: now,
        };

        let shared = row.to_shared();

        assert!(!shared.is_shared);
    }
}
