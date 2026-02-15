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
    pub requires_confirmation: bool,
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
            requires_confirmation: self.requires_confirmation,
            created_at: self.created_at,
        }
    }
}

/// Database model for reward linked to a task with amount
#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct TaskRewardRow {
    pub id: String,
    pub household_id: String,
    pub name: String,
    pub description: String,
    pub point_cost: Option<i64>,
    pub is_purchasable: bool,
    pub requires_confirmation: bool,
    pub created_at: DateTime<Utc>,
    pub amount: i32,
}

impl TaskRewardRow {
    pub fn to_task_reward_link(&self) -> shared::TaskRewardLink {
        shared::TaskRewardLink {
            reward: shared::Reward {
                id: Uuid::parse_str(&self.id).unwrap(),
                household_id: Uuid::parse_str(&self.household_id).unwrap(),
                name: self.name.clone(),
                description: self.description.clone(),
                point_cost: self.point_cost,
                is_purchasable: self.is_purchasable,
                requires_confirmation: self.requires_confirmation,
                created_at: self.created_at,
            },
            amount: self.amount,
        }
    }
}

/// Database model for user rewards (amount-based)
#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct UserRewardRow {
    pub id: String,
    pub user_id: String,
    pub reward_id: String,
    pub household_id: String,
    pub amount: i32,
    pub redeemed_amount: i32,
    pub pending_redemption: i32,
    pub updated_at: DateTime<Utc>,
}

impl UserRewardRow {
    pub fn to_shared(&self) -> shared::UserReward {
        shared::UserReward {
            id: Uuid::parse_str(&self.id).unwrap(),
            user_id: Uuid::parse_str(&self.user_id).unwrap(),
            reward_id: Uuid::parse_str(&self.reward_id).unwrap(),
            household_id: Uuid::parse_str(&self.household_id).unwrap(),
            amount: self.amount,
            redeemed_amount: self.redeemed_amount,
            pending_redemption: self.pending_redemption,
            updated_at: self.updated_at,
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
            requires_confirmation: false,
            created_at: now,
        };

        let shared = row.to_shared();

        assert_eq!(shared.id, id);
        assert_eq!(shared.household_id, household_id);
        assert_eq!(shared.name, "Movie Night");
        assert_eq!(shared.point_cost, Some(100));
        assert!(shared.is_purchasable);
        assert!(!shared.requires_confirmation);
    }

    #[test]
    fn test_user_reward_row_to_shared() {
        let now = Utc::now();
        let id = Uuid::new_v4();
        let user_id = Uuid::new_v4();
        let reward_id = Uuid::new_v4();
        let household_id = Uuid::new_v4();

        let row = UserRewardRow {
            id: id.to_string(),
            user_id: user_id.to_string(),
            reward_id: reward_id.to_string(),
            household_id: household_id.to_string(),
            amount: 3,
            redeemed_amount: 1,
            pending_redemption: 0,
            updated_at: now,
        };

        let shared = row.to_shared();

        assert_eq!(shared.id, id);
        assert_eq!(shared.user_id, user_id);
        assert_eq!(shared.reward_id, reward_id);
        assert_eq!(shared.amount, 3);
        assert_eq!(shared.redeemed_amount, 1);
        assert_eq!(shared.pending_redemption, 0);
    }
}
