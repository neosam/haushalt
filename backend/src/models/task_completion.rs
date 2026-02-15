use chrono::{DateTime, Utc};
use sqlx::types::chrono::NaiveDate;
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

/// Database model for task completions
#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct TaskCompletionRow {
    pub id: String,
    pub task_id: String,
    pub user_id: String,
    pub completed_at: DateTime<Utc>,
    pub due_date: NaiveDate,
}

impl TaskCompletionRow {
    pub fn to_shared(&self) -> shared::TaskCompletion {
        shared::TaskCompletion {
            id: Uuid::parse_str(&self.id).unwrap(),
            task_id: Uuid::parse_str(&self.task_id).unwrap(),
            user_id: Uuid::parse_str(&self.user_id).unwrap(),
            completed_at: self.completed_at,
            due_date: self.due_date,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_task_completion_row_to_shared() {
        let now = Utc::now();
        let id = Uuid::new_v4();
        let task_id = Uuid::new_v4();
        let user_id = Uuid::new_v4();
        let due_date = NaiveDate::from_ymd_opt(2024, 1, 15).unwrap();

        let row = TaskCompletionRow {
            id: id.to_string(),
            task_id: task_id.to_string(),
            user_id: user_id.to_string(),
            completed_at: now,
            due_date,
        };

        let shared = row.to_shared();

        assert_eq!(shared.id, id);
        assert_eq!(shared.task_id, task_id);
        assert_eq!(shared.user_id, user_id);
        assert_eq!(shared.due_date, due_date);
    }
}
