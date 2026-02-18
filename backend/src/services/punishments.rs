use chrono::Utc;
use rand::seq::SliceRandom;
use sqlx::SqlitePool;
use thiserror::Error;
use uuid::Uuid;

use crate::models::{PunishmentRow, UserPunishmentRow};
use shared::{CreatePunishmentRequest, PendingPunishmentCompletion, Punishment, PunishmentType, RandomPickResult, UpdatePunishmentRequest, User, UserPunishment, UserPunishmentWithUser};

#[derive(Debug, Error)]
pub enum PunishmentError {
    #[error("Punishment not found")]
    NotFound,
    #[error("User punishment not found")]
    UserPunishmentNotFound,
    #[error("No punishments to complete")]
    NothingToComplete,
    #[error("No pending completions")]
    NothingPending,
    #[error("Random choice punishment requires at least 2 options")]
    InsufficientOptions,
    #[error("Option punishment not found")]
    OptionNotFound,
    #[error("Punishment is not a random choice punishment")]
    NotRandomChoice,
    #[error("No options available for random selection")]
    NoOptions,
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
    let requires_confirmation = request.requires_confirmation.unwrap_or(false);
    let punishment_type = request.punishment_type.unwrap_or_default();

    // Validate option_ids if random_choice
    if punishment_type.is_random_choice() {
        if let Some(ref option_ids) = request.option_ids {
            if option_ids.len() < 2 {
                return Err(PunishmentError::InsufficientOptions);
            }
        } else {
            return Err(PunishmentError::InsufficientOptions);
        }
    }

    sqlx::query(
        r#"
        INSERT INTO punishments (id, household_id, name, description, requires_confirmation, punishment_type, created_at)
        VALUES (?, ?, ?, ?, ?, ?, ?)
        "#,
    )
    .bind(id.to_string())
    .bind(household_id.to_string())
    .bind(&request.name)
    .bind(request.description.as_deref().unwrap_or(""))
    .bind(requires_confirmation)
    .bind(punishment_type.as_str())
    .bind(now)
    .execute(pool)
    .await?;

    // Add options if provided
    if let Some(ref option_ids) = request.option_ids {
        for option_id in option_ids {
            // Verify option exists and is in same household
            let option = get_punishment(pool, option_id).await?;
            match option {
                Some(p) if p.household_id == *household_id => {
                    add_punishment_option(pool, &id, option_id).await?;
                }
                _ => return Err(PunishmentError::OptionNotFound),
            }
        }
    }

    Ok(Punishment {
        id,
        household_id: *household_id,
        name: request.name.clone(),
        description: request.description.clone().unwrap_or_default(),
        requires_confirmation,
        punishment_type,
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
    if let Some(requires_confirmation) = request.requires_confirmation {
        punishment.requires_confirmation = requires_confirmation;
    }
    if let Some(punishment_type) = request.punishment_type {
        punishment.punishment_type = punishment_type.as_str().to_string();
    }

    let punishment_type: PunishmentType = punishment.punishment_type.parse().unwrap_or_default();

    // Handle option_ids update
    if let Some(ref option_ids_opt) = request.option_ids {
        match option_ids_opt {
            None => {
                // Clear all options
                sqlx::query("DELETE FROM punishment_options WHERE parent_punishment_id = ?")
                    .bind(punishment_id.to_string())
                    .execute(pool)
                    .await?;
            }
            Some(option_ids) => {
                // Validate minimum options if random_choice
                if punishment_type.is_random_choice() && option_ids.len() < 2 {
                    return Err(PunishmentError::InsufficientOptions);
                }

                // Replace all options
                sqlx::query("DELETE FROM punishment_options WHERE parent_punishment_id = ?")
                    .bind(punishment_id.to_string())
                    .execute(pool)
                    .await?;

                let household_id = Uuid::parse_str(&punishment.household_id).unwrap();
                for option_id in option_ids {
                    // Verify option exists and is in same household
                    let option = get_punishment(pool, option_id).await?;
                    match option {
                        Some(p) if p.household_id == household_id => {
                            add_punishment_option(pool, punishment_id, option_id).await?;
                        }
                        _ => return Err(PunishmentError::OptionNotFound),
                    }
                }
            }
        }
    }

    sqlx::query("UPDATE punishments SET name = ?, description = ?, requires_confirmation = ?, punishment_type = ? WHERE id = ?")
        .bind(&punishment.name)
        .bind(&punishment.description)
        .bind(punishment.requires_confirmation)
        .bind(&punishment.punishment_type)
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

    // Delete punishment options (both as parent and as option)
    sqlx::query("DELETE FROM punishment_options WHERE parent_punishment_id = ? OR option_punishment_id = ?")
        .bind(punishment_id.to_string())
        .bind(punishment_id.to_string())
        .execute(pool)
        .await?;

    sqlx::query("DELETE FROM punishments WHERE id = ?")
        .bind(punishment_id.to_string())
        .execute(pool)
        .await?;

    Ok(())
}

/// Assign a punishment to a user (or increment amount if already assigned)
pub async fn assign_punishment(
    pool: &SqlitePool,
    punishment_id: &Uuid,
    user_id: &Uuid,
    household_id: &Uuid,
) -> Result<UserPunishment, PunishmentError> {
    let _punishment = get_punishment(pool, punishment_id)
        .await?
        .ok_or(PunishmentError::NotFound)?;
    let now = Utc::now();

    // Try to update existing record first
    let result = sqlx::query(
        r#"
        UPDATE user_punishments
        SET amount = amount + 1, updated_at = ?
        WHERE user_id = ? AND punishment_id = ? AND household_id = ?
        "#,
    )
    .bind(now)
    .bind(user_id.to_string())
    .bind(punishment_id.to_string())
    .bind(household_id.to_string())
    .execute(pool)
    .await?;

    if result.rows_affected() == 0 {
        // Insert new record
        let id = Uuid::new_v4();
        sqlx::query(
            r#"
            INSERT INTO user_punishments (id, user_id, punishment_id, household_id, amount, completed_amount, pending_completion, updated_at)
            VALUES (?, ?, ?, ?, 1, 0, 0, ?)
            "#,
        )
        .bind(id.to_string())
        .bind(user_id.to_string())
        .bind(punishment_id.to_string())
        .bind(household_id.to_string())
        .bind(now)
        .execute(pool)
        .await?;
    }

    // Fetch and return the updated record
    let user_punishment: UserPunishmentRow = sqlx::query_as(
        "SELECT * FROM user_punishments WHERE user_id = ? AND punishment_id = ? AND household_id = ?",
    )
    .bind(user_id.to_string())
    .bind(punishment_id.to_string())
    .bind(household_id.to_string())
    .fetch_one(pool)
    .await?;

    Ok(user_punishment.to_shared())
}

/// Remove one punishment assignment (decrement amount, delete if zero)
pub async fn unassign_punishment(
    pool: &SqlitePool,
    punishment_id: &Uuid,
    user_id: &Uuid,
    household_id: &Uuid,
) -> Result<(), PunishmentError> {
    let now = Utc::now();

    // Get current record
    let user_punishment: Option<UserPunishmentRow> = sqlx::query_as(
        "SELECT * FROM user_punishments WHERE user_id = ? AND punishment_id = ? AND household_id = ?",
    )
    .bind(user_id.to_string())
    .bind(punishment_id.to_string())
    .bind(household_id.to_string())
    .fetch_optional(pool)
    .await?;

    let user_punishment = user_punishment.ok_or(PunishmentError::UserPunishmentNotFound)?;

    if user_punishment.amount <= 1 {
        // Delete the record
        sqlx::query("DELETE FROM user_punishments WHERE user_id = ? AND punishment_id = ? AND household_id = ?")
            .bind(user_id.to_string())
            .bind(punishment_id.to_string())
            .bind(household_id.to_string())
            .execute(pool)
            .await?;
    } else {
        // Decrement amount
        sqlx::query(
            "UPDATE user_punishments SET amount = amount - 1, updated_at = ? WHERE user_id = ? AND punishment_id = ? AND household_id = ?",
        )
        .bind(now)
        .bind(user_id.to_string())
        .bind(punishment_id.to_string())
        .bind(household_id.to_string())
        .execute(pool)
        .await?;
    }

    Ok(())
}

pub async fn list_user_punishments(
    pool: &SqlitePool,
    user_id: &Uuid,
    household_id: &Uuid,
) -> Result<Vec<UserPunishment>, PunishmentError> {
    let punishments: Vec<UserPunishmentRow> = sqlx::query_as(
        "SELECT * FROM user_punishments WHERE user_id = ? AND household_id = ? ORDER BY updated_at DESC",
    )
    .bind(user_id.to_string())
    .bind(household_id.to_string())
    .fetch_all(pool)
    .await?;

    Ok(punishments.into_iter().map(|p| p.to_shared()).collect())
}

pub async fn list_all_user_punishments_in_household(
    pool: &SqlitePool,
    household_id: &Uuid,
) -> Result<Vec<UserPunishmentWithUser>, PunishmentError> {
    #[derive(sqlx::FromRow)]
    struct JoinedRow {
        // user_punishments fields
        id: String,
        user_id: String,
        punishment_id: String,
        household_id: String,
        amount: i32,
        completed_amount: i32,
        pending_completion: i32,
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
            up.id, up.user_id, up.punishment_id, up.household_id,
            up.amount, up.completed_amount, up.pending_completion, up.updated_at,
            u.id as u_id, u.username as u_username, u.email as u_email,
            u.created_at as u_created_at, u.updated_at as u_updated_at
        FROM user_punishments up
        JOIN users u ON up.user_id = u.id
        WHERE up.household_id = ?
        ORDER BY up.updated_at DESC
        "#,
    )
    .bind(household_id.to_string())
    .fetch_all(pool)
    .await?;

    Ok(rows
        .into_iter()
        .map(|row| UserPunishmentWithUser {
            user_punishment: UserPunishment {
                id: Uuid::parse_str(&row.id).unwrap(),
                user_id: Uuid::parse_str(&row.user_id).unwrap(),
                punishment_id: Uuid::parse_str(&row.punishment_id).unwrap(),
                household_id: Uuid::parse_str(&row.household_id).unwrap(),
                amount: row.amount,
                completed_amount: row.completed_amount,
                pending_completion: row.pending_completion,
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

pub async fn delete_user_punishment(
    pool: &SqlitePool,
    user_punishment_id: &Uuid,
) -> Result<(), PunishmentError> {
    let result = sqlx::query("DELETE FROM user_punishments WHERE id = ?")
        .bind(user_punishment_id.to_string())
        .execute(pool)
        .await?;

    if result.rows_affected() == 0 {
        return Err(PunishmentError::UserPunishmentNotFound);
    }

    Ok(())
}

/// Complete one punishment - if requires_confirmation, goes to pending; otherwise direct completion
/// Returns (UserPunishment, requires_confirmation) tuple
pub async fn complete_punishment(
    pool: &SqlitePool,
    user_punishment_id: &Uuid,
    _user_id: &Uuid,
) -> Result<(UserPunishment, bool), PunishmentError> {
    let user_punishment: UserPunishmentRow =
        sqlx::query_as("SELECT * FROM user_punishments WHERE id = ?")
            .bind(user_punishment_id.to_string())
            .fetch_optional(pool)
            .await?
            .ok_or(PunishmentError::UserPunishmentNotFound)?;

    // Get the punishment to check requires_confirmation
    let punishment_id = Uuid::parse_str(&user_punishment.punishment_id).unwrap();
    let punishment = get_punishment(pool, &punishment_id).await?.ok_or(PunishmentError::NotFound)?;

    // Check if there are uncompleted punishments (excluding pending)
    let available = user_punishment.amount - user_punishment.completed_amount - user_punishment.pending_completion;
    if available <= 0 {
        return Err(PunishmentError::NothingToComplete);
    }

    let now = Utc::now();

    if punishment.requires_confirmation {
        // Move to pending state
        sqlx::query("UPDATE user_punishments SET pending_completion = pending_completion + 1, updated_at = ? WHERE id = ?")
            .bind(now)
            .bind(user_punishment_id.to_string())
            .execute(pool)
            .await?;

        let mut result = user_punishment.to_shared();
        result.pending_completion += 1;
        result.updated_at = now;

        Ok((result, true))
    } else {
        // Direct completion
        sqlx::query("UPDATE user_punishments SET completed_amount = completed_amount + 1, updated_at = ? WHERE id = ?")
            .bind(now)
            .bind(user_punishment_id.to_string())
            .execute(pool)
            .await?;

        let mut result = user_punishment.to_shared();
        result.completed_amount += 1;
        result.updated_at = now;

        Ok((result, false))
    }
}

/// List all pending punishment completions for a household
pub async fn list_pending_completions(
    pool: &SqlitePool,
    household_id: &Uuid,
) -> Result<Vec<PendingPunishmentCompletion>, PunishmentError> {
    #[derive(sqlx::FromRow)]
    struct JoinedRow {
        // user_punishments fields
        up_id: String,
        up_user_id: String,
        up_punishment_id: String,
        up_household_id: String,
        up_amount: i32,
        up_completed_amount: i32,
        up_pending_completion: i32,
        up_updated_at: chrono::DateTime<chrono::Utc>,
        // punishment fields
        p_id: String,
        p_household_id: String,
        p_name: String,
        p_description: String,
        p_requires_confirmation: bool,
        p_punishment_type: String,
        p_created_at: chrono::DateTime<chrono::Utc>,
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
            up.id as up_id, up.user_id as up_user_id, up.punishment_id as up_punishment_id,
            up.household_id as up_household_id, up.amount as up_amount,
            up.completed_amount as up_completed_amount, up.pending_completion as up_pending_completion,
            up.updated_at as up_updated_at,
            p.id as p_id, p.household_id as p_household_id, p.name as p_name,
            p.description as p_description, p.requires_confirmation as p_requires_confirmation,
            p.punishment_type as p_punishment_type, p.created_at as p_created_at,
            u.id as u_id, u.username as u_username, u.email as u_email,
            u.created_at as u_created_at, u.updated_at as u_updated_at
        FROM user_punishments up
        JOIN punishments p ON up.punishment_id = p.id
        JOIN users u ON up.user_id = u.id
        WHERE up.household_id = ? AND up.pending_completion > 0
        ORDER BY up.updated_at DESC
        "#,
    )
    .bind(household_id.to_string())
    .fetch_all(pool)
    .await?;

    Ok(rows
        .into_iter()
        .map(|row| PendingPunishmentCompletion {
            user_punishment: UserPunishment {
                id: Uuid::parse_str(&row.up_id).unwrap(),
                user_id: Uuid::parse_str(&row.up_user_id).unwrap(),
                punishment_id: Uuid::parse_str(&row.up_punishment_id).unwrap(),
                household_id: Uuid::parse_str(&row.up_household_id).unwrap(),
                amount: row.up_amount,
                completed_amount: row.up_completed_amount,
                pending_completion: row.up_pending_completion,
                updated_at: row.up_updated_at,
            },
            punishment: Punishment {
                id: Uuid::parse_str(&row.p_id).unwrap(),
                household_id: Uuid::parse_str(&row.p_household_id).unwrap(),
                name: row.p_name,
                description: row.p_description,
                requires_confirmation: row.p_requires_confirmation,
                punishment_type: row.p_punishment_type.parse().unwrap_or_default(),
                created_at: row.p_created_at,
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

/// Approve a pending completion - decrement pending_completion, increment completed_amount
pub async fn approve_completion(
    pool: &SqlitePool,
    user_punishment_id: &Uuid,
) -> Result<UserPunishment, PunishmentError> {
    let user_punishment: UserPunishmentRow =
        sqlx::query_as("SELECT * FROM user_punishments WHERE id = ?")
            .bind(user_punishment_id.to_string())
            .fetch_optional(pool)
            .await?
            .ok_or(PunishmentError::UserPunishmentNotFound)?;

    if user_punishment.pending_completion <= 0 {
        return Err(PunishmentError::NothingPending);
    }

    let now = Utc::now();
    sqlx::query(
        "UPDATE user_punishments SET pending_completion = pending_completion - 1, completed_amount = completed_amount + 1, updated_at = ? WHERE id = ?",
    )
    .bind(now)
    .bind(user_punishment_id.to_string())
    .execute(pool)
    .await?;

    let mut result = user_punishment.to_shared();
    result.pending_completion -= 1;
    result.completed_amount += 1;
    result.updated_at = now;

    Ok(result)
}

/// Reject a pending completion - decrement pending_completion only (reset to available)
pub async fn reject_completion(
    pool: &SqlitePool,
    user_punishment_id: &Uuid,
) -> Result<UserPunishment, PunishmentError> {
    let user_punishment: UserPunishmentRow =
        sqlx::query_as("SELECT * FROM user_punishments WHERE id = ?")
            .bind(user_punishment_id.to_string())
            .fetch_optional(pool)
            .await?
            .ok_or(PunishmentError::UserPunishmentNotFound)?;

    if user_punishment.pending_completion <= 0 {
        return Err(PunishmentError::NothingPending);
    }

    let now = Utc::now();
    sqlx::query(
        "UPDATE user_punishments SET pending_completion = pending_completion - 1, updated_at = ? WHERE id = ?",
    )
    .bind(now)
    .bind(user_punishment_id.to_string())
    .execute(pool)
    .await?;

    let mut result = user_punishment.to_shared();
    result.pending_completion -= 1;
    result.updated_at = now;

    Ok(result)
}

// ============================================================================
// Random Choice Punishment Functions
// ============================================================================

/// Add a punishment option to a random choice punishment
pub async fn add_punishment_option(
    pool: &SqlitePool,
    parent_punishment_id: &Uuid,
    option_punishment_id: &Uuid,
) -> Result<(), PunishmentError> {
    // Self-reference is allowed - user can include themselves as an option
    let id = Uuid::new_v4();
    let now = Utc::now();

    sqlx::query(
        r#"
        INSERT INTO punishment_options (id, parent_punishment_id, option_punishment_id, created_at)
        VALUES (?, ?, ?, ?)
        "#,
    )
    .bind(id.to_string())
    .bind(parent_punishment_id.to_string())
    .bind(option_punishment_id.to_string())
    .bind(now)
    .execute(pool)
    .await?;

    Ok(())
}

/// Get all punishment options for a random choice punishment
pub async fn get_punishment_options(
    pool: &SqlitePool,
    punishment_id: &Uuid,
) -> Result<Vec<Punishment>, PunishmentError> {
    let rows: Vec<PunishmentRow> = sqlx::query_as(
        r#"
        SELECT p.*
        FROM punishments p
        JOIN punishment_options po ON p.id = po.option_punishment_id
        WHERE po.parent_punishment_id = ?
        ORDER BY p.name
        "#,
    )
    .bind(punishment_id.to_string())
    .fetch_all(pool)
    .await?;

    Ok(rows.into_iter().map(|r| r.to_shared()).collect())
}

/// Remove a punishment option from a random choice punishment
#[allow(dead_code)]
pub async fn remove_punishment_option(
    pool: &SqlitePool,
    parent_punishment_id: &Uuid,
    option_punishment_id: &Uuid,
) -> Result<(), PunishmentError> {
    // Check if parent is a random choice punishment
    let punishment = get_punishment(pool, parent_punishment_id).await?.ok_or(PunishmentError::NotFound)?;

    if punishment.punishment_type.is_random_choice() {
        // Count current options
        let count: (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM punishment_options WHERE parent_punishment_id = ?"
        )
        .bind(parent_punishment_id.to_string())
        .fetch_one(pool)
        .await?;

        // Ensure at least 2 options remain after deletion
        if count.0 <= 2 {
            return Err(PunishmentError::InsufficientOptions);
        }
    }

    sqlx::query(
        "DELETE FROM punishment_options WHERE parent_punishment_id = ? AND option_punishment_id = ?"
    )
    .bind(parent_punishment_id.to_string())
    .bind(option_punishment_id.to_string())
    .execute(pool)
    .await?;

    Ok(())
}

/// Pick a random punishment from a user's random choice punishment assignment
pub async fn pick_random_option(
    pool: &SqlitePool,
    user_punishment_id: &Uuid,
    user_id: &Uuid,
) -> Result<RandomPickResult, PunishmentError> {
    // Get the user punishment
    let user_punishment: UserPunishmentRow =
        sqlx::query_as("SELECT * FROM user_punishments WHERE id = ?")
            .bind(user_punishment_id.to_string())
            .fetch_optional(pool)
            .await?
            .ok_or(PunishmentError::UserPunishmentNotFound)?;

    // Verify user owns this punishment
    if user_punishment.user_id != user_id.to_string() {
        return Err(PunishmentError::UserPunishmentNotFound);
    }

    // Get the punishment
    let punishment_id = Uuid::parse_str(&user_punishment.punishment_id).unwrap();
    let punishment = get_punishment(pool, &punishment_id).await?.ok_or(PunishmentError::NotFound)?;

    // Verify it's a random choice punishment
    if !punishment.punishment_type.is_random_choice() {
        return Err(PunishmentError::NotRandomChoice);
    }

    // Get options
    let options = get_punishment_options(pool, &punishment_id).await?;
    if options.is_empty() {
        return Err(PunishmentError::NoOptions);
    }

    // Randomly select one
    let mut rng = rand::thread_rng();
    let picked = options.choose(&mut rng).unwrap();

    // Assign the picked punishment to the user
    let household_id = Uuid::parse_str(&user_punishment.household_id).unwrap();
    let new_user_punishment = assign_punishment(pool, &picked.id, user_id, &household_id).await?;

    // Mark the original random choice assignment as completed (decrement amount)
    let now = Utc::now();
    if user_punishment.amount <= 1 {
        // Delete the record
        sqlx::query("DELETE FROM user_punishments WHERE id = ?")
            .bind(user_punishment_id.to_string())
            .execute(pool)
            .await?;
    } else {
        // Decrement amount
        sqlx::query(
            "UPDATE user_punishments SET amount = amount - 1, updated_at = ? WHERE id = ?"
        )
        .bind(now)
        .bind(user_punishment_id.to_string())
        .execute(pool)
        .await?;
    }

    Ok(RandomPickResult {
        picked_punishment: picked.clone(),
        user_punishment: new_user_punishment,
    })
}

/// Get user punishment by ID
#[allow(dead_code)]
pub async fn get_user_punishment(
    pool: &SqlitePool,
    user_punishment_id: &Uuid,
) -> Result<Option<UserPunishment>, PunishmentError> {
    let row: Option<UserPunishmentRow> =
        sqlx::query_as("SELECT * FROM user_punishments WHERE id = ?")
            .bind(user_punishment_id.to_string())
            .fetch_optional(pool)
            .await?;

    Ok(row.map(|r| r.to_shared()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_punishment_error_display() {
        assert_eq!(PunishmentError::NotFound.to_string(), "Punishment not found");
        assert_eq!(
            PunishmentError::NothingToComplete.to_string(),
            "No punishments to complete"
        );
        assert_eq!(
            PunishmentError::InsufficientOptions.to_string(),
            "Random choice punishment requires at least 2 options"
        );
        assert_eq!(
            PunishmentError::NotRandomChoice.to_string(),
            "Punishment is not a random choice punishment"
        );
    }
}
