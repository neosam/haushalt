use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

/// Database model for announcements
#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct AnnouncementRow {
    pub id: String,
    pub household_id: String,
    pub created_by: String,
    pub title: String,
    pub content: String,
    pub starts_at: Option<DateTime<Utc>>,
    pub ends_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl AnnouncementRow {
    pub fn to_shared(&self) -> shared::Announcement {
        shared::Announcement {
            id: Uuid::parse_str(&self.id).unwrap(),
            household_id: Uuid::parse_str(&self.household_id).unwrap(),
            created_by: Uuid::parse_str(&self.created_by).unwrap(),
            title: self.title.clone(),
            content: self.content.clone(),
            starts_at: self.starts_at,
            ends_at: self.ends_at,
            created_at: self.created_at,
            updated_at: self.updated_at,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_announcement_row_to_shared() {
        let now = Utc::now();
        let id = Uuid::new_v4();
        let household_id = Uuid::new_v4();
        let created_by = Uuid::new_v4();

        let row = AnnouncementRow {
            id: id.to_string(),
            household_id: household_id.to_string(),
            created_by: created_by.to_string(),
            title: "Test Announcement".to_string(),
            content: "# Important\n\nThis is a test announcement.".to_string(),
            starts_at: Some(now),
            ends_at: None,
            created_at: now,
            updated_at: now,
        };

        let shared = row.to_shared();

        assert_eq!(shared.id, id);
        assert_eq!(shared.household_id, household_id);
        assert_eq!(shared.created_by, created_by);
        assert_eq!(shared.title, "Test Announcement");
        assert_eq!(shared.content, "# Important\n\nThis is a test announcement.");
        assert_eq!(shared.starts_at, Some(now));
        assert_eq!(shared.ends_at, None);
    }

    #[test]
    fn test_announcement_row_with_schedule() {
        let now = Utc::now();
        let start = now;
        let end = now + chrono::Duration::days(7);

        let row = AnnouncementRow {
            id: Uuid::new_v4().to_string(),
            household_id: Uuid::new_v4().to_string(),
            created_by: Uuid::new_v4().to_string(),
            title: "Scheduled Announcement".to_string(),
            content: "Content".to_string(),
            starts_at: Some(start),
            ends_at: Some(end),
            created_at: now,
            updated_at: now,
        };

        let shared = row.to_shared();

        assert_eq!(shared.starts_at, Some(start));
        assert_eq!(shared.ends_at, Some(end));
    }
}
