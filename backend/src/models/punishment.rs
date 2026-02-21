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
    pub requires_confirmation: bool,
    pub punishment_type: String,
    pub created_at: DateTime<Utc>,
}

impl PunishmentRow {
    pub fn to_shared(&self) -> shared::Punishment {
        shared::Punishment {
            id: Uuid::parse_str(&self.id).unwrap(),
            household_id: Uuid::parse_str(&self.household_id).unwrap(),
            name: self.name.clone(),
            description: self.description.clone(),
            requires_confirmation: self.requires_confirmation,
            punishment_type: self.punishment_type.parse().unwrap_or_default(),
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
    pub requires_confirmation: bool,
    pub punishment_type: String,
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
                requires_confirmation: self.requires_confirmation,
                punishment_type: self.punishment_type.parse().unwrap_or_default(),
                created_at: self.created_at,
            },
            amount: self.amount,
        }
    }
}

/// Database model for household default punishment with amount
#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct HouseholdDefaultPunishmentRow {
    pub id: String,
    pub household_id: String,
    pub name: String,
    pub description: String,
    pub requires_confirmation: bool,
    pub punishment_type: String,
    pub created_at: DateTime<Utc>,
    pub amount: i32,
}

impl HouseholdDefaultPunishmentRow {
    pub fn to_link(&self) -> shared::HouseholdDefaultPunishmentLink {
        shared::HouseholdDefaultPunishmentLink {
            punishment: shared::Punishment {
                id: Uuid::parse_str(&self.id).unwrap(),
                household_id: Uuid::parse_str(&self.household_id).unwrap(),
                name: self.name.clone(),
                description: self.description.clone(),
                requires_confirmation: self.requires_confirmation,
                punishment_type: self.punishment_type.parse().unwrap_or_default(),
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
    pub pending_completion: i32,
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
            pending_completion: self.pending_completion,
            updated_at: self.updated_at,
        }
    }
}

/// Database model for punishment options (links random choice punishment to its options)
#[allow(dead_code)]
#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct PunishmentOptionRow {
    pub id: String,
    pub parent_punishment_id: String,
    pub option_punishment_id: String,
    pub created_at: DateTime<Utc>,
}

impl PunishmentOptionRow {
    #[allow(dead_code)]
    pub fn to_shared(&self) -> shared::PunishmentOption {
        shared::PunishmentOption {
            id: Uuid::parse_str(&self.id).unwrap(),
            parent_punishment_id: Uuid::parse_str(&self.parent_punishment_id).unwrap(),
            option_punishment_id: Uuid::parse_str(&self.option_punishment_id).unwrap(),
            created_at: self.created_at,
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
            requires_confirmation: true,
            punishment_type: "standard".to_string(),
            created_at: now,
        };

        let shared = row.to_shared();

        assert_eq!(shared.id, id);
        assert_eq!(shared.household_id, household_id);
        assert_eq!(shared.name, "Extra Chores");
        assert_eq!(shared.description, "Do an extra chore as punishment");
        assert!(shared.requires_confirmation);
        assert_eq!(shared.punishment_type, shared::PunishmentType::Standard);
    }

    #[test]
    fn test_punishment_row_random_choice_to_shared() {
        let now = Utc::now();
        let id = Uuid::new_v4();
        let household_id = Uuid::new_v4();

        let row = PunishmentRow {
            id: id.to_string(),
            household_id: household_id.to_string(),
            name: "Random Punishment".to_string(),
            description: "Pick one randomly".to_string(),
            requires_confirmation: false,
            punishment_type: "random_choice".to_string(),
            created_at: now,
        };

        let shared = row.to_shared();

        assert_eq!(shared.punishment_type, shared::PunishmentType::RandomChoice);
        assert!(shared.punishment_type.is_random_choice());
    }

    #[test]
    fn test_punishment_option_row_to_shared() {
        let now = Utc::now();
        let id = Uuid::new_v4();
        let parent_id = Uuid::new_v4();
        let option_id = Uuid::new_v4();

        let row = PunishmentOptionRow {
            id: id.to_string(),
            parent_punishment_id: parent_id.to_string(),
            option_punishment_id: option_id.to_string(),
            created_at: now,
        };

        let shared = row.to_shared();

        assert_eq!(shared.id, id);
        assert_eq!(shared.parent_punishment_id, parent_id);
        assert_eq!(shared.option_punishment_id, option_id);
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
            pending_completion: 1,
            updated_at: now,
        };

        let shared = row.to_shared();

        assert_eq!(shared.id, id);
        assert_eq!(shared.user_id, user_id);
        assert_eq!(shared.punishment_id, punishment_id);
        assert_eq!(shared.amount, 2);
        assert_eq!(shared.completed_amount, 0);
        assert_eq!(shared.pending_completion, 1);
    }
}
