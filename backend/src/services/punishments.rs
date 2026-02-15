use chrono::Utc;
use sqlx::SqlitePool;
use thiserror::Error;
use uuid::Uuid;

use crate::models::{PunishmentRow, UserPunishmentRow};
use shared::{CreatePunishmentRequest, Punishment, UpdatePunishmentRequest, UserPunishment};

#[derive(Debug, Error)]
pub enum PunishmentError {
    #[error("Punishment not found")]
    NotFound,
    #[error("User punishment not found")]
    UserPunishmentNotFound,
    #[error("Punishment already completed")]
    AlreadyCompleted,
    #[error("Database error: {0}")]
    DatabaseError(#[from] sqlx::Error),
}

pub async fn create_punishment(
    pool: &SqlitePool,
    household_id: &Uuid,
    request: &CreatePunishmentRequest,
) -> Result<Punishment, PunishmentError> {
    let id = Uuid::new_v4();
    let now = Utc::now();

    sqlx::query(
        r#"
        INSERT INTO punishments (id, household_id, name, description, created_at)
        VALUES (?, ?, ?, ?, ?)
        "#,
    )
    .bind(id.to_string())
    .bind(household_id.to_string())
    .bind(&request.name)
    .bind(request.description.as_deref().unwrap_or(""))
    .bind(now)
    .execute(pool)
    .await?;

    Ok(Punishment {
        id,
        household_id: *household_id,
        name: request.name.clone(),
        description: request.description.clone().unwrap_or_default(),
        created_at: now,
    })
}

pub async fn get_punishment(
    pool: &SqlitePool,
    punishment_id: &Uuid,
) -> Result<Option<Punishment>, PunishmentError> {
    let punishment: Option<PunishmentRow> =
        sqlx::query_as("SELECT * FROM punishments WHERE id = ?")
            .bind(punishment_id.to_string())
            .fetch_optional(pool)
            .await?;

    Ok(punishment.map(|p| p.to_shared()))
}

pub async fn list_punishments(
    pool: &SqlitePool,
    household_id: &Uuid,
) -> Result<Vec<Punishment>, PunishmentError> {
    let punishments: Vec<PunishmentRow> = sqlx::query_as(
        "SELECT * FROM punishments WHERE household_id = ? ORDER BY created_at DESC",
    )
    .bind(household_id.to_string())
    .fetch_all(pool)
    .await?;

    Ok(punishments.into_iter().map(|p| p.to_shared()).collect())
}

pub async fn update_punishment(
    pool: &SqlitePool,
    punishment_id: &Uuid,
    request: &UpdatePunishmentRequest,
) -> Result<Punishment, PunishmentError> {
    let mut punishment: PunishmentRow =
        sqlx::query_as("SELECT * FROM punishments WHERE id = ?")
            .bind(punishment_id.to_string())
            .fetch_optional(pool)
            .await?
            .ok_or(PunishmentError::NotFound)?;

    if let Some(ref name) = request.name {
        punishment.name = name.clone();
    }
    if let Some(ref description) = request.description {
        punishment.description = description.clone();
    }

    sqlx::query("UPDATE punishments SET name = ?, description = ? WHERE id = ?")
        .bind(&punishment.name)
        .bind(&punishment.description)
        .bind(punishment_id.to_string())
        .execute(pool)
        .await?;

    Ok(punishment.to_shared())
}

pub async fn delete_punishment(pool: &SqlitePool, punishment_id: &Uuid) -> Result<(), PunishmentError> {
    // Delete related user punishments first
    sqlx::query("DELETE FROM user_punishments WHERE punishment_id = ?")
        .bind(punishment_id.to_string())
        .execute(pool)
        .await?;

    // Delete task associations
    sqlx::query("DELETE FROM task_punishments WHERE punishment_id = ?")
        .bind(punishment_id.to_string())
        .execute(pool)
        .await?;

    sqlx::query("DELETE FROM punishments WHERE id = ?")
        .bind(punishment_id.to_string())
        .execute(pool)
        .await?;

    Ok(())
}

pub async fn assign_punishment(
    pool: &SqlitePool,
    punishment_id: &Uuid,
    user_id: &Uuid,
    household_id: &Uuid,
    assigned_by: &Uuid,
    task_completion_id: Option<Uuid>,
) -> Result<UserPunishment, PunishmentError> {
    let _punishment = get_punishment(pool, punishment_id)
        .await?
        .ok_or(PunishmentError::NotFound)?;

    let id = Uuid::new_v4();
    let now = Utc::now();

    sqlx::query(
        r#"
        INSERT INTO user_punishments (id, user_id, punishment_id, household_id, assigned_by, task_completion_id, completed, assigned_at)
        VALUES (?, ?, ?, ?, ?, ?, FALSE, ?)
        "#,
    )
    .bind(id.to_string())
    .bind(user_id.to_string())
    .bind(punishment_id.to_string())
    .bind(household_id.to_string())
    .bind(assigned_by.to_string())
    .bind(task_completion_id.map(|id| id.to_string()))
    .bind(now)
    .execute(pool)
    .await?;

    Ok(UserPunishment {
        id,
        user_id: *user_id,
        punishment_id: *punishment_id,
        household_id: *household_id,
        assigned_by: *assigned_by,
        task_completion_id,
        completed: false,
        assigned_at: now,
    })
}

pub async fn list_user_punishments(
    pool: &SqlitePool,
    user_id: &Uuid,
    household_id: &Uuid,
) -> Result<Vec<UserPunishment>, PunishmentError> {
    let punishments: Vec<UserPunishmentRow> = sqlx::query_as(
        "SELECT * FROM user_punishments WHERE user_id = ? AND household_id = ? ORDER BY assigned_at DESC",
    )
    .bind(user_id.to_string())
    .bind(household_id.to_string())
    .fetch_all(pool)
    .await?;

    Ok(punishments.into_iter().map(|p| p.to_shared()).collect())
}

pub async fn complete_punishment(
    pool: &SqlitePool,
    user_punishment_id: &Uuid,
) -> Result<UserPunishment, PunishmentError> {
    let user_punishment: UserPunishmentRow =
        sqlx::query_as("SELECT * FROM user_punishments WHERE id = ?")
            .bind(user_punishment_id.to_string())
            .fetch_optional(pool)
            .await?
            .ok_or(PunishmentError::UserPunishmentNotFound)?;

    if user_punishment.completed {
        return Err(PunishmentError::AlreadyCompleted);
    }

    sqlx::query("UPDATE user_punishments SET completed = TRUE WHERE id = ?")
        .bind(user_punishment_id.to_string())
        .execute(pool)
        .await?;

    let mut result = user_punishment.to_shared();
    result.completed = true;

    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_punishment_error_display() {
        assert_eq!(PunishmentError::NotFound.to_string(), "Punishment not found");
        assert_eq!(
            PunishmentError::AlreadyCompleted.to_string(),
            "Punishment already completed"
        );
    }
}
