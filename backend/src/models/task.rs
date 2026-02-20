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
    pub time_period: Option<String>,
    pub allow_exceed_target: bool,
    pub requires_review: bool,
    pub points_reward: Option<i64>,
    pub points_penalty: Option<i64>,
    pub due_time: Option<String>,
    pub habit_type: String,
    pub category_id: Option<String>,
    pub archived: bool,
    pub paused: bool,
    pub suggestion: Option<String>,
    pub suggested_by: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Database model for tasks with category name (from JOIN)
#[allow(dead_code)]
#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct TaskRowWithCategory {
    pub id: String,
    pub household_id: String,
    pub title: String,
    pub description: String,
    pub recurrence_type: String,
    pub recurrence_value: Option<String>,
    pub assigned_user_id: Option<String>,
    pub target_count: i32,
    pub time_period: Option<String>,
    pub allow_exceed_target: bool,
    pub requires_review: bool,
    pub points_reward: Option<i64>,
    pub points_penalty: Option<i64>,
    pub due_time: Option<String>,
    pub habit_type: String,
    pub category_id: Option<String>,
    pub category_name: Option<String>,
    pub archived: bool,
    pub paused: bool,
    pub suggestion: Option<String>,
    pub suggested_by: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl TaskRow {
    pub fn to_shared(&self) -> shared::Task {
        let recurrence_value = self.recurrence_value.as_ref().and_then(|v| {
            serde_json::from_str(v).ok()
        });

        let time_period = self.time_period.as_ref().and_then(|p| p.parse().ok());

        shared::Task {
            id: Uuid::parse_str(&self.id).unwrap(),
            household_id: Uuid::parse_str(&self.household_id).unwrap(),
            title: self.title.clone(),
            description: self.description.clone(),
            recurrence_type: self.recurrence_type.parse().unwrap_or(shared::RecurrenceType::Daily),
            recurrence_value,
            assigned_user_id: self.assigned_user_id.as_ref().and_then(|id| Uuid::parse_str(id).ok()),
            target_count: self.target_count,
            time_period,
            allow_exceed_target: self.allow_exceed_target,
            requires_review: self.requires_review,
            points_reward: self.points_reward,
            points_penalty: self.points_penalty,
            due_time: self.due_time.clone(),
            habit_type: self.habit_type.parse().unwrap_or(shared::HabitType::Good),
            category_id: self.category_id.as_ref().and_then(|id| Uuid::parse_str(id).ok()),
            category_name: None,
            archived: self.archived,
            paused: self.paused,
            suggestion: self.suggestion.as_ref().and_then(|s| s.parse().ok()),
            suggested_by: self.suggested_by.as_ref().and_then(|id| Uuid::parse_str(id).ok()),
            created_at: self.created_at,
            updated_at: self.updated_at,
        }
    }
}

#[allow(dead_code)]
impl TaskRowWithCategory {
    pub fn to_shared(&self) -> shared::Task {
        let recurrence_value = self.recurrence_value.as_ref().and_then(|v| {
            serde_json::from_str(v).ok()
        });

        let time_period = self.time_period.as_ref().and_then(|p| p.parse().ok());

        shared::Task {
            id: Uuid::parse_str(&self.id).unwrap(),
            household_id: Uuid::parse_str(&self.household_id).unwrap(),
            title: self.title.clone(),
            description: self.description.clone(),
            recurrence_type: self.recurrence_type.parse().unwrap_or(shared::RecurrenceType::Daily),
            recurrence_value,
            assigned_user_id: self.assigned_user_id.as_ref().and_then(|id| Uuid::parse_str(id).ok()),
            target_count: self.target_count,
            time_period,
            allow_exceed_target: self.allow_exceed_target,
            requires_review: self.requires_review,
            points_reward: self.points_reward,
            points_penalty: self.points_penalty,
            due_time: self.due_time.clone(),
            habit_type: self.habit_type.parse().unwrap_or(shared::HabitType::Good),
            category_id: self.category_id.as_ref().and_then(|id| Uuid::parse_str(id).ok()),
            category_name: self.category_name.clone(),
            archived: self.archived,
            paused: self.paused,
            suggestion: self.suggestion.as_ref().and_then(|s| s.parse().ok()),
            suggested_by: self.suggested_by.as_ref().and_then(|id| Uuid::parse_str(id).ok()),
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
            time_period: None,
            allow_exceed_target: true,
            requires_review: false,
            points_reward: None,
            points_penalty: None,
            due_time: None,
            habit_type: "good".to_string(),
            category_id: None,
            archived: false,
            paused: false,
            suggestion: None,
            suggested_by: None,
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
        assert!(shared.allow_exceed_target);
        assert!(!shared.archived);
        assert!(!shared.paused);
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
            time_period: None,
            allow_exceed_target: false,
            requires_review: false,
            points_reward: None,
            points_penalty: None,
            due_time: None,
            habit_type: "good".to_string(),
            category_id: None,
            archived: false,
            paused: false,
            suggestion: None,
            suggested_by: None,
            created_at: now,
            updated_at: now,
        };

        let shared = row.to_shared();

        assert_eq!(shared.assigned_user_id, Some(user_id));
        assert_eq!(shared.recurrence_type, RecurrenceType::Weekly);
        assert_eq!(shared.target_count, 3);
        assert!(!shared.allow_exceed_target);
    }

    #[test]
    fn test_task_row_allow_exceed_target() {
        let now = Utc::now();

        // Test with allow_exceed_target = true
        let row_allow = TaskRow {
            id: Uuid::new_v4().to_string(),
            household_id: Uuid::new_v4().to_string(),
            title: "Allow Exceed".to_string(),
            description: "".to_string(),
            recurrence_type: "daily".to_string(),
            recurrence_value: None,
            assigned_user_id: None,
            target_count: 5,
            time_period: None,
            allow_exceed_target: true,
            requires_review: false,
            points_reward: None,
            points_penalty: None,
            due_time: None,
            habit_type: "good".to_string(),
            category_id: None,
            archived: false,
            paused: false,
            suggestion: None,
            suggested_by: None,
            created_at: now,
            updated_at: now,
        };
        assert!(row_allow.to_shared().allow_exceed_target);

        // Test with allow_exceed_target = false
        let row_restrict = TaskRow {
            id: Uuid::new_v4().to_string(),
            household_id: Uuid::new_v4().to_string(),
            title: "Restrict to Target".to_string(),
            description: "".to_string(),
            recurrence_type: "daily".to_string(),
            recurrence_value: None,
            assigned_user_id: None,
            target_count: 5,
            time_period: None,
            allow_exceed_target: false,
            requires_review: false,
            points_reward: None,
            points_penalty: None,
            due_time: None,
            habit_type: "good".to_string(),
            category_id: None,
            archived: false,
            paused: false,
            suggestion: None,
            suggested_by: None,
            created_at: now,
            updated_at: now,
        };
        assert!(!row_restrict.to_shared().allow_exceed_target);
    }

    #[test]
    fn test_task_row_habit_type() {
        let now = Utc::now();

        // Test with good habit
        let row_good = TaskRow {
            id: Uuid::new_v4().to_string(),
            household_id: Uuid::new_v4().to_string(),
            title: "Good Habit".to_string(),
            description: "".to_string(),
            recurrence_type: "daily".to_string(),
            recurrence_value: None,
            assigned_user_id: None,
            target_count: 1,
            time_period: None,
            allow_exceed_target: true,
            requires_review: false,
            points_reward: None,
            points_penalty: None,
            due_time: None,
            habit_type: "good".to_string(),
            category_id: None,
            archived: false,
            paused: false,
            suggestion: None,
            suggested_by: None,
            created_at: now,
            updated_at: now,
        };
        assert_eq!(row_good.to_shared().habit_type, shared::HabitType::Good);
        assert!(!row_good.to_shared().habit_type.is_inverted());

        // Test with bad habit
        let row_bad = TaskRow {
            id: Uuid::new_v4().to_string(),
            household_id: Uuid::new_v4().to_string(),
            title: "Bad Habit".to_string(),
            description: "".to_string(),
            recurrence_type: "daily".to_string(),
            recurrence_value: None,
            assigned_user_id: None,
            target_count: 1,
            time_period: None,
            allow_exceed_target: true,
            requires_review: false,
            points_reward: None,
            points_penalty: None,
            due_time: None,
            habit_type: "bad".to_string(),
            category_id: None,
            archived: false,
            paused: false,
            suggestion: None,
            suggested_by: None,
            created_at: now,
            updated_at: now,
        };
        assert_eq!(row_bad.to_shared().habit_type, shared::HabitType::Bad);
        assert!(row_bad.to_shared().habit_type.is_inverted());
    }
}
