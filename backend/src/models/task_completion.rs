use chrono::{DateTime, Utc};
use sqlx::types::chrono::NaiveDate;
use serde::{Deserialize, Serialize};
use sqlx::FromRow;

/// Database model for task completions
#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct TaskCompletionRow {
    pub id: String,
    pub task_id: String,
    pub user_id: String,
    pub completed_at: DateTime<Utc>,
    pub due_date: NaiveDate,
}
