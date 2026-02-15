use chrono::Utc;
use sqlx::SqlitePool;
use thiserror::Error;
use uuid::Uuid;

use crate::models::PointConditionRow;
use crate::services::households;
use shared::{ConditionType, CreatePointConditionRequest, PointCondition, UpdatePointConditionRequest};

#[derive(Debug, Error)]
pub enum PointsError {
    #[error("Point condition not found")]
    NotFound,
    #[error("Database error: {0}")]
    DatabaseError(#[from] sqlx::Error),
    #[error("Household error: {0}")]
    HouseholdError(#[from] super::households::HouseholdError),
}

pub async fn create_point_condition(
    pool: &SqlitePool,
    household_id: &Uuid,
    request: &CreatePointConditionRequest,
) -> Result<PointCondition, PointsError> {
    let id = Uuid::new_v4();
    let now = Utc::now();

    sqlx::query(
        r#"
        INSERT INTO point_conditions (id, household_id, name, condition_type, points_value, streak_threshold, multiplier, task_id, created_at)
        VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)
        "#,
    )
    .bind(id.to_string())
    .bind(household_id.to_string())
    .bind(&request.name)
    .bind(request.condition_type.as_str())
    .bind(request.points_value)
    .bind(request.streak_threshold)
    .bind(request.multiplier)
    .bind(request.task_id.map(|id| id.to_string()))
    .bind(now)
    .execute(pool)
    .await?;

    Ok(PointCondition {
        id,
        household_id: *household_id,
        name: request.name.clone(),
        condition_type: request.condition_type.clone(),
        points_value: request.points_value,
        streak_threshold: request.streak_threshold,
        multiplier: request.multiplier,
        task_id: request.task_id,
        created_at: now,
    })
}

pub async fn get_point_condition(
    pool: &SqlitePool,
    condition_id: &Uuid,
) -> Result<Option<PointCondition>, PointsError> {
    let condition: Option<PointConditionRow> =
        sqlx::query_as("SELECT * FROM point_conditions WHERE id = ?")
            .bind(condition_id.to_string())
            .fetch_optional(pool)
            .await?;

    Ok(condition.map(|c| c.to_shared()))
}

pub async fn list_point_conditions(
    pool: &SqlitePool,
    household_id: &Uuid,
) -> Result<Vec<PointCondition>, PointsError> {
    let conditions: Vec<PointConditionRow> = sqlx::query_as(
        "SELECT * FROM point_conditions WHERE household_id = ? ORDER BY created_at DESC",
    )
    .bind(household_id.to_string())
    .fetch_all(pool)
    .await?;

    Ok(conditions.into_iter().map(|c| c.to_shared()).collect())
}

pub async fn update_point_condition(
    pool: &SqlitePool,
    condition_id: &Uuid,
    request: &UpdatePointConditionRequest,
) -> Result<PointCondition, PointsError> {
    let mut condition: PointConditionRow =
        sqlx::query_as("SELECT * FROM point_conditions WHERE id = ?")
            .bind(condition_id.to_string())
            .fetch_optional(pool)
            .await?
            .ok_or(PointsError::NotFound)?;

    if let Some(ref name) = request.name {
        condition.name = name.clone();
    }
    if let Some(ref condition_type) = request.condition_type {
        condition.condition_type = condition_type.as_str().to_string();
    }
    if let Some(points_value) = request.points_value {
        condition.points_value = points_value;
    }
    if let Some(streak_threshold) = request.streak_threshold {
        condition.streak_threshold = Some(streak_threshold);
    }
    if let Some(multiplier) = request.multiplier {
        condition.multiplier = Some(multiplier);
    }
    if let Some(task_id) = request.task_id {
        condition.task_id = Some(task_id.to_string());
    }

    sqlx::query(
        r#"
        UPDATE point_conditions SET name = ?, condition_type = ?, points_value = ?, streak_threshold = ?, multiplier = ?, task_id = ?
        WHERE id = ?
        "#,
    )
    .bind(&condition.name)
    .bind(&condition.condition_type)
    .bind(condition.points_value)
    .bind(condition.streak_threshold)
    .bind(condition.multiplier)
    .bind(&condition.task_id)
    .bind(condition_id.to_string())
    .execute(pool)
    .await?;

    Ok(condition.to_shared())
}

pub async fn delete_point_condition(pool: &SqlitePool, condition_id: &Uuid) -> Result<(), PointsError> {
    sqlx::query("DELETE FROM point_conditions WHERE id = ?")
        .bind(condition_id.to_string())
        .execute(pool)
        .await?;

    Ok(())
}

/// Award points when a task is completed
pub async fn award_task_completion_points(
    pool: &SqlitePool,
    household_id: &Uuid,
    user_id: &Uuid,
    task_id: &Uuid,
    current_streak: i32,
) -> Result<i64, PointsError> {
    let conditions = list_point_conditions(pool, household_id).await?;

    let mut total_points: i64 = 0;

    // Check for direct points_reward on the task
    let task: Option<crate::models::TaskRow> =
        sqlx::query_as("SELECT * FROM tasks WHERE id = ?")
            .bind(task_id.to_string())
            .fetch_optional(pool)
            .await?;

    if let Some(task) = task {
        if let Some(reward) = task.points_reward {
            total_points += reward;
        }
    }

    for condition in conditions {
        match condition.condition_type {
            ConditionType::TaskComplete => {
                // Check if this condition applies to this task or all tasks
                let applies = condition.task_id.is_none() || condition.task_id == Some(*task_id);

                if applies {
                    let mut points = condition.points_value;

                    // Apply multiplier if set
                    if let Some(multiplier) = condition.multiplier {
                        points = (points as f64 * multiplier) as i64;
                    }

                    total_points += points;
                }
            }

            ConditionType::Streak => {
                // Check if streak threshold is met
                if let Some(threshold) = condition.streak_threshold {
                    let applies = condition.task_id.is_none() || condition.task_id == Some(*task_id);

                    if applies && current_streak >= threshold && current_streak % threshold == 0 {
                        let mut points = condition.points_value;

                        // Apply multiplier based on streak level
                        if let Some(multiplier) = condition.multiplier {
                            let streak_level = (current_streak / threshold) as f64;
                            points = (points as f64 * (1.0 + (streak_level - 1.0) * (multiplier - 1.0))) as i64;
                        }

                        total_points += points;
                    }
                }
            }

            _ => {}
        }
    }

    if total_points != 0 {
        households::update_member_points(pool, household_id, user_id, total_points).await?;
    }

    Ok(total_points)
}

/// Reverse points awarded for a task completion (used when rejecting a pending completion)
/// This calculates the base points that would have been awarded (without streak bonuses) and deducts them
pub async fn reverse_task_completion_points(
    pool: &SqlitePool,
    household_id: &Uuid,
    user_id: &Uuid,
    task_id: &Uuid,
) -> Result<i64, PointsError> {
    let conditions = list_point_conditions(pool, household_id).await?;

    let mut total_points: i64 = 0;

    // Check for direct points_reward on the task
    let task: Option<crate::models::TaskRow> =
        sqlx::query_as("SELECT * FROM tasks WHERE id = ?")
            .bind(task_id.to_string())
            .fetch_optional(pool)
            .await?;

    if let Some(task) = task {
        if let Some(reward) = task.points_reward {
            total_points += reward;
        }
    }

    for condition in conditions {
        if condition.condition_type == ConditionType::TaskComplete {
            // Check if this condition applies to this task or all tasks
            let applies = condition.task_id.is_none() || condition.task_id == Some(*task_id);

            if applies {
                let mut points = condition.points_value;

                // Apply multiplier if set
                if let Some(multiplier) = condition.multiplier {
                    points = (points as f64 * multiplier) as i64;
                }

                total_points += points;
            }
        }
        // Note: We don't reverse streak bonuses since the streak calculation is complex
        // and the completion being rejected may have affected the streak
    }

    // Deduct the points (negative adjustment)
    if total_points != 0 {
        households::update_member_points(pool, household_id, user_id, -total_points).await?;
    }

    Ok(total_points)
}

/// Deduct points when a task is missed
pub async fn deduct_missed_task_points(
    pool: &SqlitePool,
    household_id: &Uuid,
    user_id: &Uuid,
    task_id: &Uuid,
    streak_was_broken: bool,
) -> Result<i64, PointsError> {
    let conditions = list_point_conditions(pool, household_id).await?;

    let mut total_points: i64 = 0;

    // Check for direct points_penalty on the task
    let task: Option<crate::models::TaskRow> =
        sqlx::query_as("SELECT * FROM tasks WHERE id = ?")
            .bind(task_id.to_string())
            .fetch_optional(pool)
            .await?;

    if let Some(task) = task {
        if let Some(penalty) = task.points_penalty {
            // Penalty is stored as positive, deduct it
            total_points -= penalty;
        }
    }

    for condition in conditions {
        match condition.condition_type {
            ConditionType::TaskMissed => {
                let applies = condition.task_id.is_none() || condition.task_id == Some(*task_id);

                if applies {
                    total_points += condition.points_value; // Usually negative
                }
            }

            ConditionType::StreakBroken => {
                if streak_was_broken {
                    let applies = condition.task_id.is_none() || condition.task_id == Some(*task_id);

                    if applies {
                        total_points += condition.points_value; // Usually negative
                    }
                }
            }

            _ => {}
        }
    }

    if total_points != 0 {
        households::update_member_points(pool, household_id, user_id, total_points).await?;
    }

    Ok(total_points)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_points_error_display() {
        assert_eq!(PointsError::NotFound.to_string(), "Point condition not found");
    }

    #[test]
    fn test_condition_type_as_str() {
        assert_eq!(ConditionType::TaskComplete.as_str(), "task_complete");
        assert_eq!(ConditionType::TaskMissed.as_str(), "task_missed");
        assert_eq!(ConditionType::Streak.as_str(), "streak");
        assert_eq!(ConditionType::StreakBroken.as_str(), "streak_broken");
    }
}
