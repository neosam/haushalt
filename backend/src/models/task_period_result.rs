use chrono::{DateTime, Utc};
use shared::PeriodStatus;
use sqlx::types::chrono::NaiveDate;
use sqlx::FromRow;
use uuid::Uuid;

/// Database model for task period results
#[derive(Debug, Clone, FromRow)]
pub struct TaskPeriodResultRow {
    pub id: String,
    pub task_id: String,
    pub period_start: NaiveDate,
    pub period_end: NaiveDate,
    pub status: String,
    pub completions_count: i32,
    pub target_count: i32,
    pub finalized_at: DateTime<Utc>,
    pub finalized_by: String,
    pub notes: Option<String>,
}

impl TaskPeriodResultRow {
    pub fn to_shared(&self) -> shared::TaskPeriodResult {
        shared::TaskPeriodResult {
            id: Uuid::parse_str(&self.id).unwrap(),
            task_id: Uuid::parse_str(&self.task_id).unwrap(),
            period_start: self.period_start,
            period_end: self.period_end,
            status: self.status.parse().unwrap_or(PeriodStatus::Failed),
            completions_count: self.completions_count,
            target_count: self.target_count,
            finalized_at: self.finalized_at,
            finalized_by: self.finalized_by.clone(),
            notes: self.notes.clone(),
        }
    }
}
