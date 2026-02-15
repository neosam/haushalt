use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

/// Database model for punishments
#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct PunishmentRow {
    pub id: String,
    pub household_id: String,
    pub name: String,
    pub description: String,
    pub created_at: DateTime<Utc>,
}

impl PunishmentRow {
    pub fn to_shared(&self) -> shared::Punishment {
        shared::Punishment {
            id: Uuid::parse_str(&self.id).unwrap(),
            household_id: Uuid::parse_str(&self.household_id).unwrap(),
            name: self.name.clone(),
            description: self.description.clone(),
            created_at: self.created_at,
        }
    }
}

/// Database model for user punishments (assigned)
#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct UserPunishmentRow {
    pub id: String,
    pub user_id: String,
    pub punishment_id: String,
    pub household_id: String,
    pub assigned_by: String,
    pub task_completion_id: Option<String>,
    pub completed: bool,
    pub assigned_at: DateTime<Utc>,
}

impl UserPunishmentRow {
    pub fn to_shared(&self) -> shared::UserPunishment {
        shared::UserPunishment {
            id: Uuid::parse_str(&self.id).unwrap(),
            user_id: Uuid::parse_str(&self.user_id).unwrap(),
            punishment_id: Uuid::parse_str(&self.punishment_id).unwrap(),
            household_id: Uuid::parse_str(&self.household_id).unwrap(),
            assigned_by: Uuid::parse_str(&self.assigned_by).unwrap(),
            task_completion_id: self.task_completion_id.as_ref().and_then(|id| Uuid::parse_str(id).ok()),
            completed: self.completed,
            assigned_at: self.assigned_at,
        }
    }
}

/// Database model for task-punishment associations
#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct TaskPunishmentRow {
    pub task_id: String,
    pub punishment_id: String,
}

impl TaskPunishmentRow {
    pub fn to_shared(&self) -> shared::TaskPunishment {
        shared::TaskPunishment {
            task_id: Uuid::parse_str(&self.task_id).unwrap(),
            punishment_id: Uuid::parse_str(&self.punishment_id).unwrap(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_punishment_row_to_shared() {
        let now = Utc::now();
        let id = Uuid::new_v4();
        let household_id = Uuid::new_v4();

        let row = PunishmentRow {
            id: id.to_string(),
            household_id: household_id.to_string(),
            name: "Extra Chores".to_string(),
            description: "Do an extra chore as punishment".to_string(),
            created_at: now,
        };

        let shared = row.to_shared();

        assert_eq!(shared.id, id);
        assert_eq!(shared.household_id, household_id);
        assert_eq!(shared.name, "Extra Chores");
        assert_eq!(shared.description, "Do an extra chore as punishment");
    }

    #[test]
    fn test_user_punishment_row_to_shared() {
        let now = Utc::now();
        let id = Uuid::new_v4();
        let user_id = Uuid::new_v4();
        let punishment_id = Uuid::new_v4();
        let household_id = Uuid::new_v4();
        let assigned_by = Uuid::new_v4();

        let row = UserPunishmentRow {
            id: id.to_string(),
            user_id: user_id.to_string(),
            punishment_id: punishment_id.to_string(),
            household_id: household_id.to_string(),
            assigned_by: assigned_by.to_string(),
            task_completion_id: None,
            completed: false,
            assigned_at: now,
        };

        let shared = row.to_shared();

        assert_eq!(shared.id, id);
        assert_eq!(shared.user_id, user_id);
        assert_eq!(shared.punishment_id, punishment_id);
        assert_eq!(shared.assigned_by, assigned_by);
        assert!(!shared.completed);
        assert!(shared.task_completion_id.is_none());
    }

    #[test]
    fn test_user_punishment_from_task() {
        let now = Utc::now();
        let task_completion_id = Uuid::new_v4();

        let row = UserPunishmentRow {
            id: Uuid::new_v4().to_string(),
            user_id: Uuid::new_v4().to_string(),
            punishment_id: Uuid::new_v4().to_string(),
            household_id: Uuid::new_v4().to_string(),
            assigned_by: Uuid::new_v4().to_string(),
            task_completion_id: Some(task_completion_id.to_string()),
            completed: false,
            assigned_at: now,
        };

        let shared = row.to_shared();

        assert_eq!(shared.task_completion_id, Some(task_completion_id));
    }
}
