use chrono::Utc;
use sqlx::SqlitePool;
use thiserror::Error;
use uuid::Uuid;

use crate::models::{HouseholdRow, MembershipRow, UserRow};
use shared::{
    CreateHouseholdRequest, Household, HouseholdMembership, LeaderboardEntry, MemberWithUser,
    Role, UpdateHouseholdRequest,
};

#[derive(Debug, Error)]
pub enum HouseholdError {
    #[error("Household not found")]
    NotFound,
    #[error("Database error: {0}")]
    DatabaseError(#[from] sqlx::Error),
}

pub async fn create_household(
    pool: &SqlitePool,
    owner_id: &Uuid,
    request: &CreateHouseholdRequest,
) -> Result<Household, HouseholdError> {
    let id = Uuid::new_v4();
    let now = Utc::now();

    // Create household
    sqlx::query(
        r#"
        INSERT INTO households (id, name, owner_id, created_at, updated_at)
        VALUES (?, ?, ?, ?, ?)
        "#,
    )
    .bind(id.to_string())
    .bind(&request.name)
    .bind(owner_id.to_string())
    .bind(now)
    .bind(now)
    .execute(pool)
    .await?;

    // Add owner as member with owner role
    let membership_id = Uuid::new_v4();
    sqlx::query(
        r#"
        INSERT INTO household_memberships (id, household_id, user_id, role, points, joined_at)
        VALUES (?, ?, ?, ?, ?, ?)
        "#,
    )
    .bind(membership_id.to_string())
    .bind(id.to_string())
    .bind(owner_id.to_string())
    .bind(Role::Owner.as_str())
    .bind(0i64)
    .bind(now)
    .execute(pool)
    .await?;

    Ok(Household {
        id,
        name: request.name.clone(),
        owner_id: *owner_id,
        created_at: now,
        updated_at: now,
    })
}

pub async fn get_household(pool: &SqlitePool, household_id: &Uuid) -> Result<Option<Household>, HouseholdError> {
    let household: Option<HouseholdRow> = sqlx::query_as("SELECT * FROM households WHERE id = ?")
        .bind(household_id.to_string())
        .fetch_optional(pool)
        .await?;

    Ok(household.map(|h| h.to_shared()))
}

pub async fn list_user_households(pool: &SqlitePool, user_id: &Uuid) -> Result<Vec<Household>, HouseholdError> {
    let households: Vec<HouseholdRow> = sqlx::query_as(
        r#"
        SELECT h.* FROM households h
        JOIN household_memberships m ON h.id = m.household_id
        WHERE m.user_id = ?
        ORDER BY h.created_at DESC
        "#,
    )
    .bind(user_id.to_string())
    .fetch_all(pool)
    .await?;

    Ok(households.into_iter().map(|h| h.to_shared()).collect())
}

pub async fn update_household(
    pool: &SqlitePool,
    household_id: &Uuid,
    request: &UpdateHouseholdRequest,
) -> Result<Household, HouseholdError> {
    let mut household: HouseholdRow = sqlx::query_as("SELECT * FROM households WHERE id = ?")
        .bind(household_id.to_string())
        .fetch_optional(pool)
        .await?
        .ok_or(HouseholdError::NotFound)?;

    if let Some(ref name) = request.name {
        household.name = name.clone();
    }

    let now = Utc::now();
    household.updated_at = now;

    sqlx::query("UPDATE households SET name = ?, updated_at = ? WHERE id = ?")
        .bind(&household.name)
        .bind(now)
        .bind(household_id.to_string())
        .execute(pool)
        .await?;

    Ok(household.to_shared())
}

pub async fn delete_household(pool: &SqlitePool, household_id: &Uuid) -> Result<(), HouseholdError> {
    // Delete all related data (cascade in order)
    sqlx::query("DELETE FROM user_punishments WHERE household_id = ?")
        .bind(household_id.to_string())
        .execute(pool)
        .await?;

    sqlx::query("DELETE FROM user_rewards WHERE household_id = ?")
        .bind(household_id.to_string())
        .execute(pool)
        .await?;

    sqlx::query("DELETE FROM task_punishments WHERE task_id IN (SELECT id FROM tasks WHERE household_id = ?)")
        .bind(household_id.to_string())
        .execute(pool)
        .await?;

    sqlx::query("DELETE FROM task_rewards WHERE task_id IN (SELECT id FROM tasks WHERE household_id = ?)")
        .bind(household_id.to_string())
        .execute(pool)
        .await?;

    sqlx::query("DELETE FROM task_completions WHERE task_id IN (SELECT id FROM tasks WHERE household_id = ?)")
        .bind(household_id.to_string())
        .execute(pool)
        .await?;

    sqlx::query("DELETE FROM punishments WHERE household_id = ?")
        .bind(household_id.to_string())
        .execute(pool)
        .await?;

    sqlx::query("DELETE FROM rewards WHERE household_id = ?")
        .bind(household_id.to_string())
        .execute(pool)
        .await?;

    sqlx::query("DELETE FROM point_conditions WHERE household_id = ?")
        .bind(household_id.to_string())
        .execute(pool)
        .await?;

    sqlx::query("DELETE FROM tasks WHERE household_id = ?")
        .bind(household_id.to_string())
        .execute(pool)
        .await?;

    sqlx::query("DELETE FROM household_memberships WHERE household_id = ?")
        .bind(household_id.to_string())
        .execute(pool)
        .await?;

    sqlx::query("DELETE FROM households WHERE id = ?")
        .bind(household_id.to_string())
        .execute(pool)
        .await?;

    Ok(())
}

pub async fn is_member(pool: &SqlitePool, household_id: &Uuid, user_id: &Uuid) -> Result<bool, HouseholdError> {
    let count = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM household_memberships WHERE household_id = ? AND user_id = ?",
    )
    .bind(household_id.to_string())
    .bind(user_id.to_string())
    .fetch_one(pool)
    .await?;

    Ok(count > 0)
}

pub async fn get_member_role(pool: &SqlitePool, household_id: &Uuid, user_id: &Uuid) -> Option<Role> {
    let membership: Option<MembershipRow> = sqlx::query_as(
        "SELECT * FROM household_memberships WHERE household_id = ? AND user_id = ?",
    )
    .bind(household_id.to_string())
    .bind(user_id.to_string())
    .fetch_optional(pool)
    .await
    .ok()
    .flatten();

    membership.map(|m| m.role.parse().unwrap_or(Role::Member))
}

pub async fn list_members(pool: &SqlitePool, household_id: &Uuid) -> Result<Vec<MemberWithUser>, HouseholdError> {
    let memberships: Vec<MembershipRow> = sqlx::query_as(
        "SELECT * FROM household_memberships WHERE household_id = ? ORDER BY points DESC, joined_at ASC",
    )
    .bind(household_id.to_string())
    .fetch_all(pool)
    .await?;

    let mut result = Vec::new();
    for m in memberships {
        let user: UserRow = sqlx::query_as("SELECT * FROM users WHERE id = ?")
            .bind(&m.user_id)
            .fetch_one(pool)
            .await?;

        result.push(MemberWithUser {
            membership: m.to_shared(),
            user: user.to_shared(),
        });
    }

    Ok(result)
}

pub async fn remove_member(pool: &SqlitePool, household_id: &Uuid, user_id: &Uuid) -> Result<(), HouseholdError> {
    sqlx::query("DELETE FROM household_memberships WHERE household_id = ? AND user_id = ?")
        .bind(household_id.to_string())
        .bind(user_id.to_string())
        .execute(pool)
        .await?;

    Ok(())
}

pub async fn update_member_role(
    pool: &SqlitePool,
    household_id: &Uuid,
    user_id: &Uuid,
    role: Role,
) -> Result<HouseholdMembership, HouseholdError> {
    sqlx::query("UPDATE household_memberships SET role = ? WHERE household_id = ? AND user_id = ?")
        .bind(role.as_str())
        .bind(household_id.to_string())
        .bind(user_id.to_string())
        .execute(pool)
        .await?;

    let membership: MembershipRow = sqlx::query_as(
        "SELECT * FROM household_memberships WHERE household_id = ? AND user_id = ?",
    )
    .bind(household_id.to_string())
    .bind(user_id.to_string())
    .fetch_one(pool)
    .await?;

    Ok(membership.to_shared())
}

/// Transfer ownership from current owner to new owner.
/// The current owner becomes an admin.
pub async fn transfer_ownership(
    pool: &SqlitePool,
    household_id: &Uuid,
    current_owner_id: &Uuid,
    new_owner_id: &Uuid,
) -> Result<HouseholdMembership, HouseholdError> {
    // Use a transaction to ensure atomicity
    let mut tx = pool.begin().await?;

    // Demote current owner to admin
    sqlx::query("UPDATE household_memberships SET role = ? WHERE household_id = ? AND user_id = ?")
        .bind(Role::Admin.as_str())
        .bind(household_id.to_string())
        .bind(current_owner_id.to_string())
        .execute(&mut *tx)
        .await?;

    // Promote new owner
    sqlx::query("UPDATE household_memberships SET role = ? WHERE household_id = ? AND user_id = ?")
        .bind(Role::Owner.as_str())
        .bind(household_id.to_string())
        .bind(new_owner_id.to_string())
        .execute(&mut *tx)
        .await?;

    // Commit the transaction
    tx.commit().await?;

    // Return the new owner's membership
    let membership: MembershipRow = sqlx::query_as(
        "SELECT * FROM household_memberships WHERE household_id = ? AND user_id = ?",
    )
    .bind(household_id.to_string())
    .bind(new_owner_id.to_string())
    .fetch_one(pool)
    .await?;

    Ok(membership.to_shared())
}

pub async fn update_member_points(
    pool: &SqlitePool,
    household_id: &Uuid,
    user_id: &Uuid,
    points_delta: i64,
) -> Result<i64, HouseholdError> {
    sqlx::query("UPDATE household_memberships SET points = points + ? WHERE household_id = ? AND user_id = ?")
        .bind(points_delta)
        .bind(household_id.to_string())
        .bind(user_id.to_string())
        .execute(pool)
        .await?;

    let new_points = sqlx::query_scalar::<_, i64>(
        "SELECT points FROM household_memberships WHERE household_id = ? AND user_id = ?",
    )
    .bind(household_id.to_string())
    .bind(user_id.to_string())
    .fetch_one(pool)
    .await?;

    Ok(new_points)
}

pub async fn get_leaderboard(pool: &SqlitePool, household_id: &Uuid) -> Result<Vec<LeaderboardEntry>, HouseholdError> {
    let members = list_members(pool, household_id).await?;

    let mut entries: Vec<LeaderboardEntry> = Vec::new();

    for (rank, member) in members.iter().enumerate() {
        // Count task completions
        let tasks_completed = sqlx::query_scalar::<_, i64>(
            r#"
            SELECT COUNT(*) FROM task_completions tc
            JOIN tasks t ON tc.task_id = t.id
            WHERE t.household_id = ? AND tc.user_id = ?
            "#,
        )
        .bind(household_id.to_string())
        .bind(member.user.id.to_string())
        .fetch_one(pool)
        .await?;

        entries.push(LeaderboardEntry {
            user: member.user.clone(),
            points: member.membership.points,
            rank: (rank + 1) as i32,
            tasks_completed,
            current_streak: 0, // TODO: Calculate actual streak
        });
    }

    Ok(entries)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_role_from_str() {
        assert_eq!("owner".parse(), Ok(Role::Owner));
        assert_eq!("admin".parse(), Ok(Role::Admin));
        assert_eq!("member".parse(), Ok(Role::Member));
        assert!("invalid".parse::<Role>().is_err());
    }
}
