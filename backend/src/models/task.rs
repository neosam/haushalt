use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

/// Database model for tasks
#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct TaskRow {
    pub id: String,
    pub household_id: String,
    pub title: String,
    pub description: String,
    pub recurrence_type: String,
    pub recurrence_value: Option<String>,
    pub assigned_user_id: Option<String>,
    pub target_count: i32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl TaskRow {
    pub fn to_shared(&self) -> shared::Task {
        let recurrence_value = self.recurrence_value.as_ref().and_then(|v| {
            serde_json::from_str(v).ok()
        });

        shared::Task {
            id: Uuid::parse_str(&self.id).unwrap(),
            household_id: Uuid::parse_str(&self.household_id).unwrap(),
            title: self.title.clone(),
            description: self.description.clone(),
            recurrence_type: shared::RecurrenceType::from_str(&self.recurrence_type)
                .unwrap_or(shared::RecurrenceType::Daily),
            recurrence_value,
            assigned_user_id: self.assigned_user_id.as_ref().and_then(|id| Uuid::parse_str(id).ok()),
            target_count: self.target_count,
            created_at: self.created_at,
            updated_at: self.updated_at,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use shared::RecurrenceType;

    #[test]
    fn test_task_row_to_shared() {
        let now = Utc::now();
        let id = Uuid::new_v4();
        let household_id = Uuid::new_v4();

        let row = TaskRow {
            id: id.to_string(),
            household_id: household_id.to_string(),
            title: "Test Task".to_string(),
            description: "A test task".to_string(),
            recurrence_type: "daily".to_string(),
            recurrence_value: None,
            assigned_user_id: None,
            target_count: 1,
            created_at: now,
            updated_at: now,
        };

        let shared = row.to_shared();

        assert_eq!(shared.id, id);
        assert_eq!(shared.household_id, household_id);
        assert_eq!(shared.title, "Test Task");
        assert_eq!(shared.recurrence_type, RecurrenceType::Daily);
        assert!(shared.assigned_user_id.is_none());
        assert_eq!(shared.target_count, 1);
    }

    #[test]
    fn test_task_row_with_assigned_user() {
        let now = Utc::now();
        let user_id = Uuid::new_v4();

        let row = TaskRow {
            id: Uuid::new_v4().to_string(),
            household_id: Uuid::new_v4().to_string(),
            title: "Assigned Task".to_string(),
            description: "".to_string(),
            recurrence_type: "weekly".to_string(),
            recurrence_value: Some("1".to_string()),
            assigned_user_id: Some(user_id.to_string()),
            target_count: 3,
            created_at: now,
            updated_at: now,
        };

        let shared = row.to_shared();

        assert_eq!(shared.assigned_user_id, Some(user_id));
        assert_eq!(shared.recurrence_type, RecurrenceType::Weekly);
        assert_eq!(shared.target_count, 3);
    }
}
