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

/// Database model for punishment linked to a task with amount
#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct TaskPunishmentRow {
    pub id: String,
    pub household_id: String,
    pub name: String,
    pub description: String,
    pub created_at: DateTime<Utc>,
    pub amount: i32,
}

impl TaskPunishmentRow {
    pub fn to_task_punishment_link(&self) -> shared::TaskPunishmentLink {
        shared::TaskPunishmentLink {
            punishment: shared::Punishment {
                id: Uuid::parse_str(&self.id).unwrap(),
                household_id: Uuid::parse_str(&self.household_id).unwrap(),
                name: self.name.clone(),
                description: self.description.clone(),
                created_at: self.created_at,
            },
            amount: self.amount,
        }
    }
}

/// Database model for user punishments (amount-based)
#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct UserPunishmentRow {
    pub id: String,
    pub user_id: String,
    pub punishment_id: String,
    pub household_id: String,
    pub amount: i32,
    pub completed_amount: i32,
    pub updated_at: DateTime<Utc>,
}

impl UserPunishmentRow {
    pub fn to_shared(&self) -> shared::UserPunishment {
        shared::UserPunishment {
            id: Uuid::parse_str(&self.id).unwrap(),
            user_id: Uuid::parse_str(&self.user_id).unwrap(),
            punishment_id: Uuid::parse_str(&self.punishment_id).unwrap(),
            household_id: Uuid::parse_str(&self.household_id).unwrap(),
            amount: self.amount,
            completed_amount: self.completed_amount,
            updated_at: self.updated_at,
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

        let row = UserPunishmentRow {
            id: id.to_string(),
            user_id: user_id.to_string(),
            punishment_id: punishment_id.to_string(),
            household_id: household_id.to_string(),
            amount: 2,
            completed_amount: 0,
            updated_at: now,
        };

        let shared = row.to_shared();

        assert_eq!(shared.id, id);
        assert_eq!(shared.user_id, user_id);
        assert_eq!(shared.punishment_id, punishment_id);
        assert_eq!(shared.amount, 2);
        assert_eq!(shared.completed_amount, 0);
    }
}
