use chrono::Utc;
use sqlx::SqlitePool;
use thiserror::Error;
use uuid::Uuid;

use crate::models::{RewardRow, UserRewardRow};
use crate::services::households;
use shared::{CreateRewardRequest, PendingRewardRedemption, Reward, UpdateRewardRequest, User, UserReward, UserRewardWithUser};

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
    #[error("No rewards to redeem")]
    NothingToRedeem,
    #[error("No pending redemptions")]
    NothingPending,
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
    let requires_confirmation = request.requires_confirmation.unwrap_or(false);

    sqlx::query(
        r#"
        INSERT INTO rewards (id, household_id, name, description, point_cost, is_purchasable, requires_confirmation, created_at)
        VALUES (?, ?, ?, ?, ?, ?, ?, ?)
        "#,
    )
    .bind(id.to_string())
    .bind(household_id.to_string())
    .bind(&request.name)
    .bind(request.description.as_deref().unwrap_or(""))
    .bind(request.point_cost)
    .bind(request.is_purchasable)
    .bind(requires_confirmation)
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
        requires_confirmation,
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
    if let Some(requires_confirmation) = request.requires_confirmation {
        reward.requires_confirmation = requires_confirmation;
    }

    sqlx::query(
        "UPDATE rewards SET name = ?, description = ?, point_cost = ?, is_purchasable = ?, requires_confirmation = ? WHERE id = ?",
    )
    .bind(&reward.name)
    .bind(&reward.description)
    .bind(reward.point_cost)
    .bind(reward.is_purchasable)
    .bind(reward.requires_confirmation)
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

    // Use UPSERT to increment amount
    assign_reward(pool, reward_id, user_id, household_id).await
}

/// Assign a reward to a user (or increment amount if already assigned)
pub async fn assign_reward(
    pool: &SqlitePool,
    reward_id: &Uuid,
    user_id: &Uuid,
    household_id: &Uuid,
) -> Result<UserReward, RewardError> {
    let _reward = get_reward(pool, reward_id).await?.ok_or(RewardError::NotFound)?;
    let now = Utc::now();

    // Try to update existing record first
    let result = sqlx::query(
        r#"
        UPDATE user_rewards
        SET amount = amount + 1, updated_at = ?
        WHERE user_id = ? AND reward_id = ? AND household_id = ?
        "#,
    )
    .bind(now)
    .bind(user_id.to_string())
    .bind(reward_id.to_string())
    .bind(household_id.to_string())
    .execute(pool)
    .await?;

    if result.rows_affected() == 0 {
        // Insert new record
        let id = Uuid::new_v4();
        sqlx::query(
            r#"
            INSERT INTO user_rewards (id, user_id, reward_id, household_id, amount, redeemed_amount, pending_redemption, updated_at)
            VALUES (?, ?, ?, ?, 1, 0, 0, ?)
            "#,
        )
        .bind(id.to_string())
        .bind(user_id.to_string())
        .bind(reward_id.to_string())
        .bind(household_id.to_string())
        .bind(now)
        .execute(pool)
        .await?;
    }

    // Fetch and return the updated record
    let user_reward: UserRewardRow = sqlx::query_as(
        "SELECT * FROM user_rewards WHERE user_id = ? AND reward_id = ? AND household_id = ?",
    )
    .bind(user_id.to_string())
    .bind(reward_id.to_string())
    .bind(household_id.to_string())
    .fetch_one(pool)
    .await?;

    Ok(user_reward.to_shared())
}

/// Remove one reward assignment (decrement amount, delete if zero)
pub async fn unassign_reward(
    pool: &SqlitePool,
    reward_id: &Uuid,
    user_id: &Uuid,
    household_id: &Uuid,
) -> Result<(), RewardError> {
    let now = Utc::now();

    // Get current record
    let user_reward: Option<UserRewardRow> = sqlx::query_as(
        "SELECT * FROM user_rewards WHERE user_id = ? AND reward_id = ? AND household_id = ?",
    )
    .bind(user_id.to_string())
    .bind(reward_id.to_string())
    .bind(household_id.to_string())
    .fetch_optional(pool)
    .await?;

    let user_reward = user_reward.ok_or(RewardError::UserRewardNotFound)?;

    if user_reward.amount <= 1 {
        // Delete the record
        sqlx::query("DELETE FROM user_rewards WHERE user_id = ? AND reward_id = ? AND household_id = ?")
            .bind(user_id.to_string())
            .bind(reward_id.to_string())
            .bind(household_id.to_string())
            .execute(pool)
            .await?;
    } else {
        // Decrement amount
        sqlx::query(
            "UPDATE user_rewards SET amount = amount - 1, updated_at = ? WHERE user_id = ? AND reward_id = ? AND household_id = ?",
        )
        .bind(now)
        .bind(user_id.to_string())
        .bind(reward_id.to_string())
        .bind(household_id.to_string())
        .execute(pool)
        .await?;
    }

    Ok(())
}

pub async fn list_user_rewards(
    pool: &SqlitePool,
    user_id: &Uuid,
    household_id: &Uuid,
) -> Result<Vec<UserReward>, RewardError> {
    let rewards: Vec<UserRewardRow> = sqlx::query_as(
        "SELECT * FROM user_rewards WHERE user_id = ? AND household_id = ? ORDER BY updated_at DESC",
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
        amount: i32,
        redeemed_amount: i32,
        pending_redemption: i32,
        updated_at: chrono::DateTime<chrono::Utc>,
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
            ur.id, ur.user_id, ur.reward_id, ur.household_id,
            ur.amount, ur.redeemed_amount, ur.pending_redemption, ur.updated_at,
            u.id as u_id, u.username as u_username, u.email as u_email,
            u.created_at as u_created_at, u.updated_at as u_updated_at
        FROM user_rewards ur
        JOIN users u ON ur.user_id = u.id
        WHERE ur.household_id = ?
        ORDER BY ur.updated_at DESC
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
                amount: row.amount,
                redeemed_amount: row.redeemed_amount,
                pending_redemption: row.pending_redemption,
                updated_at: row.updated_at,
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

/// Redeem one reward - if requires_confirmation, goes to pending; otherwise direct redemption
/// Returns (UserReward, requires_confirmation) tuple
pub async fn redeem_reward(
    pool: &SqlitePool,
    user_reward_id: &Uuid,
    _user_id: &Uuid,
) -> Result<(UserReward, bool), RewardError> {
    let user_reward: UserRewardRow = sqlx::query_as("SELECT * FROM user_rewards WHERE id = ?")
        .bind(user_reward_id.to_string())
        .fetch_optional(pool)
        .await?
        .ok_or(RewardError::UserRewardNotFound)?;

    // Get the reward to check requires_confirmation
    let reward_id = Uuid::parse_str(&user_reward.reward_id).unwrap();
    let reward = get_reward(pool, &reward_id).await?.ok_or(RewardError::NotFound)?;

    // Check if there are unredeemed rewards (excluding pending)
    let available = user_reward.amount - user_reward.redeemed_amount - user_reward.pending_redemption;
    if available <= 0 {
        return Err(RewardError::NothingToRedeem);
    }

    let now = Utc::now();

    if reward.requires_confirmation {
        // Move to pending state
        sqlx::query("UPDATE user_rewards SET pending_redemption = pending_redemption + 1, updated_at = ? WHERE id = ?")
            .bind(now)
            .bind(user_reward_id.to_string())
            .execute(pool)
            .await?;

        let mut result = user_reward.to_shared();
        result.pending_redemption += 1;
        result.updated_at = now;

        Ok((result, true))
    } else {
        // Direct redemption
        sqlx::query("UPDATE user_rewards SET redeemed_amount = redeemed_amount + 1, updated_at = ? WHERE id = ?")
            .bind(now)
            .bind(user_reward_id.to_string())
            .execute(pool)
            .await?;

        let mut result = user_reward.to_shared();
        result.redeemed_amount += 1;
        result.updated_at = now;

        Ok((result, false))
    }
}

/// List all pending reward redemptions for a household
pub async fn list_pending_redemptions(
    pool: &SqlitePool,
    household_id: &Uuid,
) -> Result<Vec<PendingRewardRedemption>, RewardError> {
    #[derive(sqlx::FromRow)]
    struct JoinedRow {
        // user_rewards fields
        ur_id: String,
        ur_user_id: String,
        ur_reward_id: String,
        ur_household_id: String,
        ur_amount: i32,
        ur_redeemed_amount: i32,
        ur_pending_redemption: i32,
        ur_updated_at: chrono::DateTime<chrono::Utc>,
        // reward fields
        r_id: String,
        r_household_id: String,
        r_name: String,
        r_description: String,
        r_point_cost: Option<i64>,
        r_is_purchasable: bool,
        r_requires_confirmation: bool,
        r_created_at: chrono::DateTime<chrono::Utc>,
        // user fields
        u_id: String,
        u_username: String,
        u_email: String,
        u_created_at: chrono::DateTime<chrono::Utc>,
        u_updated_at: chrono::DateTime<chrono::Utc>,
    }

    let rows: Vec<JoinedRow> = sqlx::query_as(
        r#"
        SELECT
            ur.id as ur_id, ur.user_id as ur_user_id, ur.reward_id as ur_reward_id,
            ur.household_id as ur_household_id, ur.amount as ur_amount,
            ur.redeemed_amount as ur_redeemed_amount, ur.pending_redemption as ur_pending_redemption,
            ur.updated_at as ur_updated_at,
            r.id as r_id, r.household_id as r_household_id, r.name as r_name,
            r.description as r_description, r.point_cost as r_point_cost,
            r.is_purchasable as r_is_purchasable, r.requires_confirmation as r_requires_confirmation,
            r.created_at as r_created_at,
            u.id as u_id, u.username as u_username, u.email as u_email,
            u.created_at as u_created_at, u.updated_at as u_updated_at
        FROM user_rewards ur
        JOIN rewards r ON ur.reward_id = r.id
        JOIN users u ON ur.user_id = u.id
        WHERE ur.household_id = ? AND ur.pending_redemption > 0
        ORDER BY ur.updated_at DESC
        "#,
    )
    .bind(household_id.to_string())
    .fetch_all(pool)
    .await?;

    Ok(rows
        .into_iter()
        .map(|row| PendingRewardRedemption {
            user_reward: UserReward {
                id: Uuid::parse_str(&row.ur_id).unwrap(),
                user_id: Uuid::parse_str(&row.ur_user_id).unwrap(),
                reward_id: Uuid::parse_str(&row.ur_reward_id).unwrap(),
                household_id: Uuid::parse_str(&row.ur_household_id).unwrap(),
                amount: row.ur_amount,
                redeemed_amount: row.ur_redeemed_amount,
                pending_redemption: row.ur_pending_redemption,
                updated_at: row.ur_updated_at,
            },
            reward: Reward {
                id: Uuid::parse_str(&row.r_id).unwrap(),
                household_id: Uuid::parse_str(&row.r_household_id).unwrap(),
                name: row.r_name,
                description: row.r_description,
                point_cost: row.r_point_cost,
                is_purchasable: row.r_is_purchasable,
                requires_confirmation: row.r_requires_confirmation,
                created_at: row.r_created_at,
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

/// Approve a pending redemption - decrement pending_redemption, increment redeemed_amount
pub async fn approve_redemption(
    pool: &SqlitePool,
    user_reward_id: &Uuid,
) -> Result<UserReward, RewardError> {
    let user_reward: UserRewardRow = sqlx::query_as("SELECT * FROM user_rewards WHERE id = ?")
        .bind(user_reward_id.to_string())
        .fetch_optional(pool)
        .await?
        .ok_or(RewardError::UserRewardNotFound)?;

    if user_reward.pending_redemption <= 0 {
        return Err(RewardError::NothingPending);
    }

    let now = Utc::now();
    sqlx::query(
        "UPDATE user_rewards SET pending_redemption = pending_redemption - 1, redeemed_amount = redeemed_amount + 1, updated_at = ? WHERE id = ?",
    )
    .bind(now)
    .bind(user_reward_id.to_string())
    .execute(pool)
    .await?;

    let mut result = user_reward.to_shared();
    result.pending_redemption -= 1;
    result.redeemed_amount += 1;
    result.updated_at = now;

    Ok(result)
}

/// Reject a pending redemption - decrement pending_redemption only (reset to available)
pub async fn reject_redemption(
    pool: &SqlitePool,
    user_reward_id: &Uuid,
) -> Result<UserReward, RewardError> {
    let user_reward: UserRewardRow = sqlx::query_as("SELECT * FROM user_rewards WHERE id = ?")
        .bind(user_reward_id.to_string())
        .fetch_optional(pool)
        .await?
        .ok_or(RewardError::UserRewardNotFound)?;

    if user_reward.pending_redemption <= 0 {
        return Err(RewardError::NothingPending);
    }

    let now = Utc::now();
    sqlx::query(
        "UPDATE user_rewards SET pending_redemption = pending_redemption - 1, updated_at = ? WHERE id = ?",
    )
    .bind(now)
    .bind(user_reward_id.to_string())
    .execute(pool)
    .await?;

    let mut result = user_reward.to_shared();
    result.pending_redemption -= 1;
    result.updated_at = now;

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
