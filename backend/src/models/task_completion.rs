use chrono::{DateTime, Utc};
use sqlx::types::chrono::NaiveDate;
use serde::{Deserialize, Serialize};
use shared::CompletionStatus;
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
    pub status: String,
}

impl TaskCompletionRow {
    pub fn to_shared(&self) -> shared::TaskCompletion {
        shared::TaskCompletion {
            id: Uuid::parse_str(&self.id).unwrap(),
            task_id: Uuid::parse_str(&self.task_id).unwrap(),
            user_id: Uuid::parse_str(&self.user_id).unwrap(),
            completed_at: self.completed_at,
            due_date: self.due_date,
            status: self.status.parse().unwrap_or(CompletionStatus::Approved),
        }
    }
}
