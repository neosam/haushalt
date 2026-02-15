use sqlx::SqlitePool;
use thiserror::Error;
use uuid::Uuid;

use crate::models::{RewardRow, PunishmentRow};
use crate::services::{rewards, punishments};
use shared::{Punishment, Reward, UserPunishment, UserReward};

#[derive(Debug, Error)]
pub enum TaskConsequenceError {
    #[error("Task not found")]
    TaskNotFound,
    #[error("Reward not found")]
    RewardNotFound,
    #[error("Punishment not found")]
    PunishmentNotFound,
    #[error("Association already exists")]
    AlreadyExists,
    #[error("Association not found")]
    AssociationNotFound,
    #[error("Database error: {0}")]
    DatabaseError(#[from] sqlx::Error),
    #[error("Reward error: {0}")]
    RewardError(#[from] rewards::RewardError),
    #[error("Punishment error: {0}")]
    PunishmentError(#[from] punishments::PunishmentError),
}

/// Get all rewards linked to a task
pub async fn get_task_rewards(
    pool: &SqlitePool,
    task_id: &Uuid,
) -> Result<Vec<Reward>, TaskConsequenceError> {
    let rewards: Vec<RewardRow> = sqlx::query_as(
        r#"
        SELECT r.* FROM rewards r
        INNER JOIN task_rewards tr ON r.id = tr.reward_id
        WHERE tr.task_id = ?
        ORDER BY r.name
        "#,
    )
    .bind(task_id.to_string())
    .fetch_all(pool)
    .await?;

    Ok(rewards.into_iter().map(|r| r.to_shared()).collect())
}

/// Get all punishments linked to a task
pub async fn get_task_punishments(
    pool: &SqlitePool,
    task_id: &Uuid,
) -> Result<Vec<Punishment>, TaskConsequenceError> {
    let punishments: Vec<PunishmentRow> = sqlx::query_as(
        r#"
        SELECT p.* FROM punishments p
        INNER JOIN task_punishments tp ON p.id = tp.punishment_id
        WHERE tp.task_id = ?
        ORDER BY p.name
        "#,
    )
    .bind(task_id.to_string())
    .fetch_all(pool)
    .await?;

    Ok(punishments.into_iter().map(|p| p.to_shared()).collect())
}

/// Link a reward to a task
pub async fn add_task_reward(
    pool: &SqlitePool,
    task_id: &Uuid,
    reward_id: &Uuid,
) -> Result<(), TaskConsequenceError> {
    // Check if the association already exists
    let existing = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM task_rewards WHERE task_id = ? AND reward_id = ?",
    )
    .bind(task_id.to_string())
    .bind(reward_id.to_string())
    .fetch_one(pool)
    .await?;

    if existing > 0 {
        return Err(TaskConsequenceError::AlreadyExists);
    }

    sqlx::query("INSERT INTO task_rewards (task_id, reward_id) VALUES (?, ?)")
        .bind(task_id.to_string())
        .bind(reward_id.to_string())
        .execute(pool)
        .await?;

    Ok(())
}

/// Remove a reward link from a task
pub async fn remove_task_reward(
    pool: &SqlitePool,
    task_id: &Uuid,
    reward_id: &Uuid,
) -> Result<(), TaskConsequenceError> {
    let result = sqlx::query("DELETE FROM task_rewards WHERE task_id = ? AND reward_id = ?")
        .bind(task_id.to_string())
        .bind(reward_id.to_string())
        .execute(pool)
        .await?;

    if result.rows_affected() == 0 {
        return Err(TaskConsequenceError::AssociationNotFound);
    }

    Ok(())
}

/// Link a punishment to a task
pub async fn add_task_punishment(
    pool: &SqlitePool,
    task_id: &Uuid,
    punishment_id: &Uuid,
) -> Result<(), TaskConsequenceError> {
    // Check if the association already exists
    let existing = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM task_punishments WHERE task_id = ? AND punishment_id = ?",
    )
    .bind(task_id.to_string())
    .bind(punishment_id.to_string())
    .fetch_one(pool)
    .await?;

    if existing > 0 {
        return Err(TaskConsequenceError::AlreadyExists);
    }

    sqlx::query("INSERT INTO task_punishments (task_id, punishment_id) VALUES (?, ?)")
        .bind(task_id.to_string())
        .bind(punishment_id.to_string())
        .execute(pool)
        .await?;

    Ok(())
}

/// Remove a punishment link from a task
pub async fn remove_task_punishment(
    pool: &SqlitePool,
    task_id: &Uuid,
    punishment_id: &Uuid,
) -> Result<(), TaskConsequenceError> {
    let result =
        sqlx::query("DELETE FROM task_punishments WHERE task_id = ? AND punishment_id = ?")
            .bind(task_id.to_string())
            .bind(punishment_id.to_string())
            .execute(pool)
            .await?;

    if result.rows_affected() == 0 {
        return Err(TaskConsequenceError::AssociationNotFound);
    }

    Ok(())
}

/// Assign all rewards linked to a task to a user (called on task completion)
pub async fn assign_task_completion_rewards(
    pool: &SqlitePool,
    task_id: &Uuid,
    user_id: &Uuid,
    household_id: &Uuid,
) -> Result<Vec<UserReward>, TaskConsequenceError> {
    let task_rewards = get_task_rewards(pool, task_id).await?;

    let mut assigned_rewards = Vec::new();

    for reward in task_rewards {
        // Use system user (same as user_id) as assigner for automatic rewards
        let user_reward = rewards::assign_reward(pool, &reward.id, user_id, household_id, user_id)
            .await?;
        assigned_rewards.push(user_reward);
    }

    Ok(assigned_rewards)
}

/// Assign all punishments linked to a task to a user (called on missed task)
pub async fn assign_missed_task_punishments(
    pool: &SqlitePool,
    task_id: &Uuid,
    user_id: &Uuid,
    household_id: &Uuid,
    assigned_by: &Uuid,
) -> Result<Vec<UserPunishment>, TaskConsequenceError> {
    let task_punishments = get_task_punishments(pool, task_id).await?;

    let mut assigned_punishments = Vec::new();

    for punishment in task_punishments {
        let user_punishment = punishments::assign_punishment(
            pool,
            &punishment.id,
            user_id,
            household_id,
            assigned_by,
            None, // No task_completion_id for missed tasks
        )
        .await?;
        assigned_punishments.push(user_punishment);
    }

    Ok(assigned_punishments)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_task_consequence_error_display() {
        assert_eq!(
            TaskConsequenceError::TaskNotFound.to_string(),
            "Task not found"
        );
        assert_eq!(
            TaskConsequenceError::AlreadyExists.to_string(),
            "Association already exists"
        );
        assert_eq!(
            TaskConsequenceError::AssociationNotFound.to_string(),
            "Association not found"
        );
    }
}
