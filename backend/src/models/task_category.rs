use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

/// Database model for task categories
#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct TaskCategoryRow {
    pub id: String,
    pub household_id: String,
    pub name: String,
    pub color: Option<String>,
    pub sort_order: i32,
    pub created_at: DateTime<Utc>,
}

impl TaskCategoryRow {
    pub fn to_shared(&self) -> shared::TaskCategory {
        shared::TaskCategory {
            id: Uuid::parse_str(&self.id).unwrap(),
            household_id: Uuid::parse_str(&self.household_id).unwrap(),
            name: self.name.clone(),
            color: self.color.clone(),
            sort_order: self.sort_order,
            created_at: self.created_at,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_task_category_row_to_shared() {
        let now = Utc::now();
        let id = Uuid::new_v4();
        let household_id = Uuid::new_v4();

        let row = TaskCategoryRow {
            id: id.to_string(),
            household_id: household_id.to_string(),
            name: "Household Chores".to_string(),
            color: Some("#FF5733".to_string()),
            sort_order: 1,
            created_at: now,
        };

        let shared = row.to_shared();

        assert_eq!(shared.id, id);
        assert_eq!(shared.household_id, household_id);
        assert_eq!(shared.name, "Household Chores");
        assert_eq!(shared.color, Some("#FF5733".to_string()));
        assert_eq!(shared.sort_order, 1);
    }

    #[test]
    fn test_task_category_row_without_color() {
        let now = Utc::now();
        let id = Uuid::new_v4();
        let household_id = Uuid::new_v4();

        let row = TaskCategoryRow {
            id: id.to_string(),
            household_id: household_id.to_string(),
            name: "Work".to_string(),
            color: None,
            sort_order: 0,
            created_at: now,
        };

        let shared = row.to_shared();

        assert_eq!(shared.name, "Work");
        assert!(shared.color.is_none());
    }
}
