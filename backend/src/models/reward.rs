use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

/// Database model for rewards
#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct RewardRow {
    pub id: String,
    pub household_id: String,
    pub name: String,
    pub description: String,
    pub point_cost: Option<i64>,
    pub is_purchasable: bool,
    pub created_at: DateTime<Utc>,
}

impl RewardRow {
    pub fn to_shared(&self) -> shared::Reward {
        shared::Reward {
            id: Uuid::parse_str(&self.id).unwrap(),
            household_id: Uuid::parse_str(&self.household_id).unwrap(),
            name: self.name.clone(),
            description: self.description.clone(),
            point_cost: self.point_cost,
            is_purchasable: self.is_purchasable,
            created_at: self.created_at,
        }
    }
}

/// Database model for user rewards (assigned or purchased)
#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct UserRewardRow {
    pub id: String,
    pub user_id: String,
    pub reward_id: String,
    pub household_id: String,
    pub assigned_by: Option<String>,
    pub is_purchased: bool,
    pub redeemed: bool,
    pub assigned_at: DateTime<Utc>,
}

impl UserRewardRow {
    pub fn to_shared(&self) -> shared::UserReward {
        shared::UserReward {
            id: Uuid::parse_str(&self.id).unwrap(),
            user_id: Uuid::parse_str(&self.user_id).unwrap(),
            reward_id: Uuid::parse_str(&self.reward_id).unwrap(),
            household_id: Uuid::parse_str(&self.household_id).unwrap(),
            assigned_by: self.assigned_by.as_ref().and_then(|id| Uuid::parse_str(id).ok()),
            is_purchased: self.is_purchased,
            redeemed: self.redeemed,
            assigned_at: self.assigned_at,
        }
    }
}

/// Database model for task-reward associations
#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct TaskRewardRow {
    pub task_id: String,
    pub reward_id: String,
}

impl TaskRewardRow {
    pub fn to_shared(&self) -> shared::TaskReward {
        shared::TaskReward {
            task_id: Uuid::parse_str(&self.task_id).unwrap(),
            reward_id: Uuid::parse_str(&self.reward_id).unwrap(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_reward_row_to_shared() {
        let now = Utc::now();
        let id = Uuid::new_v4();
        let household_id = Uuid::new_v4();

        let row = RewardRow {
            id: id.to_string(),
            household_id: household_id.to_string(),
            name: "Movie Night".to_string(),
            description: "Watch a movie of your choice".to_string(),
            point_cost: Some(100),
            is_purchasable: true,
            created_at: now,
        };

        let shared = row.to_shared();

        assert_eq!(shared.id, id);
        assert_eq!(shared.household_id, household_id);
        assert_eq!(shared.name, "Movie Night");
        assert_eq!(shared.point_cost, Some(100));
        assert!(shared.is_purchasable);
    }

    #[test]
    fn test_user_reward_row_to_shared() {
        let now = Utc::now();
        let id = Uuid::new_v4();
        let user_id = Uuid::new_v4();
        let reward_id = Uuid::new_v4();
        let household_id = Uuid::new_v4();
        let assigned_by = Uuid::new_v4();

        let row = UserRewardRow {
            id: id.to_string(),
            user_id: user_id.to_string(),
            reward_id: reward_id.to_string(),
            household_id: household_id.to_string(),
            assigned_by: Some(assigned_by.to_string()),
            is_purchased: false,
            redeemed: false,
            assigned_at: now,
        };

        let shared = row.to_shared();

        assert_eq!(shared.id, id);
        assert_eq!(shared.user_id, user_id);
        assert_eq!(shared.reward_id, reward_id);
        assert_eq!(shared.assigned_by, Some(assigned_by));
        assert!(!shared.is_purchased);
        assert!(!shared.redeemed);
    }

    #[test]
    fn test_user_reward_purchased() {
        let now = Utc::now();

        let row = UserRewardRow {
            id: Uuid::new_v4().to_string(),
            user_id: Uuid::new_v4().to_string(),
            reward_id: Uuid::new_v4().to_string(),
            household_id: Uuid::new_v4().to_string(),
            assigned_by: None,
            is_purchased: true,
            redeemed: false,
            assigned_at: now,
        };

        let shared = row.to_shared();

        assert!(shared.is_purchased);
        assert!(shared.assigned_by.is_none());
    }
}
