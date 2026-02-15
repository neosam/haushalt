use chrono::Utc;
use sqlx::SqlitePool;
use thiserror::Error;
use uuid::Uuid;

use crate::models::{RewardRow, UserRewardRow};
use crate::services::households;
use shared::{CreateRewardRequest, Reward, UpdateRewardRequest, User, UserReward, UserRewardWithUser};

#[derive(Debug, Error)]
pub enum RewardError {
    #[error("Reward not found")]
    NotFound,
    #[error("Reward is not purchasable")]
    NotPurchasable,
    #[error("Insufficient points")]
    InsufficientPoints,
    #[error("User reward not found")]
    UserRewardNotFound,
    #[error("Reward already redeemed")]
    AlreadyRedeemed,
    #[error("Cannot redeem another user's reward")]
    NotOwner,
    #[error("Database error: {0}")]
    DatabaseError(#[from] sqlx::Error),
    #[error("Household error: {0}")]
    HouseholdError(#[from] super::households::HouseholdError),
}

pub async fn create_reward(
    pool: &SqlitePool,
    household_id: &Uuid,
    request: &CreateRewardRequest,
) -> Result<Reward, RewardError> {
    let id = Uuid::new_v4();
    let now = Utc::now();

    sqlx::query(
        r#"
        INSERT INTO rewards (id, household_id, name, description, point_cost, is_purchasable, created_at)
        VALUES (?, ?, ?, ?, ?, ?, ?)
        "#,
    )
    .bind(id.to_string())
    .bind(household_id.to_string())
    .bind(&request.name)
    .bind(request.description.as_deref().unwrap_or(""))
    .bind(request.point_cost)
    .bind(request.is_purchasable)
    .bind(now)
    .execute(pool)
    .await?;

    Ok(Reward {
        id,
        household_id: *household_id,
        name: request.name.clone(),
        description: request.description.clone().unwrap_or_default(),
        point_cost: request.point_cost,
        is_purchasable: request.is_purchasable,
        created_at: now,
    })
}

pub async fn get_reward(pool: &SqlitePool, reward_id: &Uuid) -> Result<Option<Reward>, RewardError> {
    let reward: Option<RewardRow> = sqlx::query_as("SELECT * FROM rewards WHERE id = ?")
        .bind(reward_id.to_string())
        .fetch_optional(pool)
        .await?;

    Ok(reward.map(|r| r.to_shared()))
}

pub async fn list_rewards(pool: &SqlitePool, household_id: &Uuid) -> Result<Vec<Reward>, RewardError> {
    let rewards: Vec<RewardRow> = sqlx::query_as(
        "SELECT * FROM rewards WHERE household_id = ? ORDER BY created_at DESC",
    )
    .bind(household_id.to_string())
    .fetch_all(pool)
    .await?;

    Ok(rewards.into_iter().map(|r| r.to_shared()).collect())
}

pub async fn update_reward(
    pool: &SqlitePool,
    reward_id: &Uuid,
    request: &UpdateRewardRequest,
) -> Result<Reward, RewardError> {
    let mut reward: RewardRow = sqlx::query_as("SELECT * FROM rewards WHERE id = ?")
        .bind(reward_id.to_string())
        .fetch_optional(pool)
        .await?
        .ok_or(RewardError::NotFound)?;

    if let Some(ref name) = request.name {
        reward.name = name.clone();
    }
    if let Some(ref description) = request.description {
        reward.description = description.clone();
    }
    if let Some(point_cost) = request.point_cost {
        reward.point_cost = Some(point_cost);
    }
    if let Some(is_purchasable) = request.is_purchasable {
        reward.is_purchasable = is_purchasable;
    }

    sqlx::query(
        "UPDATE rewards SET name = ?, description = ?, point_cost = ?, is_purchasable = ? WHERE id = ?",
    )
    .bind(&reward.name)
    .bind(&reward.description)
    .bind(reward.point_cost)
    .bind(reward.is_purchasable)
    .bind(reward_id.to_string())
    .execute(pool)
    .await?;

    Ok(reward.to_shared())
}

pub async fn delete_reward(pool: &SqlitePool, reward_id: &Uuid) -> Result<(), RewardError> {
    // Delete related user rewards first
    sqlx::query("DELETE FROM user_rewards WHERE reward_id = ?")
        .bind(reward_id.to_string())
        .execute(pool)
        .await?;

    // Delete task associations
    sqlx::query("DELETE FROM task_rewards WHERE reward_id = ?")
        .bind(reward_id.to_string())
        .execute(pool)
        .await?;

    sqlx::query("DELETE FROM rewards WHERE id = ?")
        .bind(reward_id.to_string())
        .execute(pool)
        .await?;

    Ok(())
}

pub async fn purchase_reward(
    pool: &SqlitePool,
    reward_id: &Uuid,
    user_id: &Uuid,
    household_id: &Uuid,
) -> Result<UserReward, RewardError> {
    let reward = get_reward(pool, reward_id).await?.ok_or(RewardError::NotFound)?;

    if !reward.is_purchasable {
        return Err(RewardError::NotPurchasable);
    }

    let point_cost = reward.point_cost.ok_or(RewardError::NotPurchasable)?;

    // Get user's current points
    let current_points = sqlx::query_scalar::<_, i64>(
        "SELECT points FROM household_memberships WHERE household_id = ? AND user_id = ?",
    )
    .bind(household_id.to_string())
    .bind(user_id.to_string())
    .fetch_one(pool)
    .await?;

    if current_points < point_cost {
        return Err(RewardError::InsufficientPoints);
    }

    // Deduct points
    households::update_member_points(pool, household_id, user_id, -point_cost).await?;

    // Create user reward
    let id = Uuid::new_v4();
    let now = Utc::now();

    sqlx::query(
        r#"
        INSERT INTO user_rewards (id, user_id, reward_id, household_id, assigned_by, is_purchased, redeemed, assigned_at)
        VALUES (?, ?, ?, ?, NULL, TRUE, FALSE, ?)
        "#,
    )
    .bind(id.to_string())
    .bind(user_id.to_string())
    .bind(reward_id.to_string())
    .bind(household_id.to_string())
    .bind(now)
    .execute(pool)
    .await?;

    Ok(UserReward {
        id,
        user_id: *user_id,
        reward_id: *reward_id,
        household_id: *household_id,
        assigned_by: None,
        is_purchased: true,
        redeemed: false,
        assigned_at: now,
    })
}

pub async fn assign_reward(
    pool: &SqlitePool,
    reward_id: &Uuid,
    user_id: &Uuid,
    household_id: &Uuid,
    assigned_by: &Uuid,
) -> Result<UserReward, RewardError> {
    let _reward = get_reward(pool, reward_id).await?.ok_or(RewardError::NotFound)?;

    let id = Uuid::new_v4();
    let now = Utc::now();

    sqlx::query(
        r#"
        INSERT INTO user_rewards (id, user_id, reward_id, household_id, assigned_by, is_purchased, redeemed, assigned_at)
        VALUES (?, ?, ?, ?, ?, FALSE, FALSE, ?)
        "#,
    )
    .bind(id.to_string())
    .bind(user_id.to_string())
    .bind(reward_id.to_string())
    .bind(household_id.to_string())
    .bind(assigned_by.to_string())
    .bind(now)
    .execute(pool)
    .await?;

    Ok(UserReward {
        id,
        user_id: *user_id,
        reward_id: *reward_id,
        household_id: *household_id,
        assigned_by: Some(*assigned_by),
        is_purchased: false,
        redeemed: false,
        assigned_at: now,
    })
}

pub async fn list_user_rewards(
    pool: &SqlitePool,
    user_id: &Uuid,
    household_id: &Uuid,
) -> Result<Vec<UserReward>, RewardError> {
    let rewards: Vec<UserRewardRow> = sqlx::query_as(
        "SELECT * FROM user_rewards WHERE user_id = ? AND household_id = ? ORDER BY assigned_at DESC",
    )
    .bind(user_id.to_string())
    .bind(household_id.to_string())
    .fetch_all(pool)
    .await?;

    Ok(rewards.into_iter().map(|r| r.to_shared()).collect())
}

pub async fn list_all_user_rewards_in_household(
    pool: &SqlitePool,
    household_id: &Uuid,
) -> Result<Vec<UserRewardWithUser>, RewardError> {
    #[derive(sqlx::FromRow)]
    struct JoinedRow {
        // user_rewards fields
        id: String,
        user_id: String,
        reward_id: String,
        household_id: String,
        assigned_by: Option<String>,
        is_purchased: bool,
        redeemed: bool,
        assigned_at: chrono::DateTime<chrono::Utc>,
        // users fields (aliased)
        u_id: String,
        u_username: String,
        u_email: String,
        u_created_at: chrono::DateTime<chrono::Utc>,
        u_updated_at: chrono::DateTime<chrono::Utc>,
    }

    let rows: Vec<JoinedRow> = sqlx::query_as(
        r#"
        SELECT
            ur.id, ur.user_id, ur.reward_id, ur.household_id, ur.assigned_by,
            ur.is_purchased, ur.redeemed, ur.assigned_at,
            u.id as u_id, u.username as u_username, u.email as u_email,
            u.created_at as u_created_at, u.updated_at as u_updated_at
        FROM user_rewards ur
        JOIN users u ON ur.user_id = u.id
        WHERE ur.household_id = ?
        ORDER BY ur.assigned_at DESC
        "#,
    )
    .bind(household_id.to_string())
    .fetch_all(pool)
    .await?;

    Ok(rows
        .into_iter()
        .map(|row| UserRewardWithUser {
            user_reward: UserReward {
                id: Uuid::parse_str(&row.id).unwrap(),
                user_id: Uuid::parse_str(&row.user_id).unwrap(),
                reward_id: Uuid::parse_str(&row.reward_id).unwrap(),
                household_id: Uuid::parse_str(&row.household_id).unwrap(),
                assigned_by: row.assigned_by.map(|s| Uuid::parse_str(&s).unwrap()),
                is_purchased: row.is_purchased,
                redeemed: row.redeemed,
                assigned_at: row.assigned_at,
            },
            user: User {
                id: Uuid::parse_str(&row.u_id).unwrap(),
                username: row.u_username,
                email: row.u_email,
                created_at: row.u_created_at,
                updated_at: row.u_updated_at,
            },
        })
        .collect())
}

pub async fn delete_user_reward(
    pool: &SqlitePool,
    user_reward_id: &Uuid,
) -> Result<(), RewardError> {
    let result = sqlx::query("DELETE FROM user_rewards WHERE id = ?")
        .bind(user_reward_id.to_string())
        .execute(pool)
        .await?;

    if result.rows_affected() == 0 {
        return Err(RewardError::UserRewardNotFound);
    }

    Ok(())
}

pub async fn redeem_reward(
    pool: &SqlitePool,
    user_reward_id: &Uuid,
    user_id: &Uuid,
) -> Result<UserReward, RewardError> {
    let user_reward: UserRewardRow = sqlx::query_as("SELECT * FROM user_rewards WHERE id = ?")
        .bind(user_reward_id.to_string())
        .fetch_optional(pool)
        .await?
        .ok_or(RewardError::UserRewardNotFound)?;

    if Uuid::parse_str(&user_reward.user_id).unwrap() != *user_id {
        return Err(RewardError::NotOwner);
    }

    if user_reward.redeemed {
        return Err(RewardError::AlreadyRedeemed);
    }

    sqlx::query("UPDATE user_rewards SET redeemed = TRUE WHERE id = ?")
        .bind(user_reward_id.to_string())
        .execute(pool)
        .await?;

    let mut result = user_reward.to_shared();
    result.redeemed = true;

    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_reward_error_display() {
        assert_eq!(RewardError::NotFound.to_string(), "Reward not found");
        assert_eq!(RewardError::NotPurchasable.to_string(), "Reward is not purchasable");
        assert_eq!(RewardError::InsufficientPoints.to_string(), "Insufficient points");
    }
}
