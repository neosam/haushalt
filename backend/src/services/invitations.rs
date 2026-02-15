use chrono::{Duration, Utc};
use sqlx::SqlitePool;
use thiserror::Error;
use uuid::Uuid;

use crate::models::{HouseholdRow, InvitationRow, UserRow};
use shared::{HouseholdMembership, Invitation, InvitationStatus, InvitationWithHousehold, Role, User};

const INVITATION_EXPIRY_DAYS: i64 = 7;

#[derive(Debug, Error)]
pub enum InvitationError {
    #[error("Invitation not found")]
    NotFound,
    #[error("User already has a pending invitation")]
    AlreadyExists,
    #[error("User is already a member of this household")]
    AlreadyMember,
    #[error("Invitation has expired")]
    Expired,
    #[error("Invitation is not for this user")]
    NotForUser,
    #[error("User not found")]
    UserNotFound,
    #[error("Database error: {0}")]
    DatabaseError(#[from] sqlx::Error),
}

/// Create a new invitation (7-day expiration)
pub async fn create_invitation(
    pool: &SqlitePool,
    household_id: &Uuid,
    email: &str,
    role: Role,
    invited_by: &Uuid,
) -> Result<Invitation, InvitationError> {
    // Check if user is already a member by email
    let existing_user: Option<UserRow> = sqlx::query_as("SELECT * FROM users WHERE email = ?")
        .bind(email)
        .fetch_optional(pool)
        .await?;

    if let Some(ref user) = existing_user {
        // Check if already a member
        let is_member = sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*) FROM household_memberships WHERE household_id = ? AND user_id = ?",
        )
        .bind(household_id.to_string())
        .bind(&user.id)
        .fetch_one(pool)
        .await?;

        if is_member > 0 {
            return Err(InvitationError::AlreadyMember);
        }
    }

    // Check for existing pending invitation
    let existing_pending: Option<InvitationRow> = sqlx::query_as(
        "SELECT * FROM household_invitations WHERE household_id = ? AND email = ? AND status = 'pending'",
    )
    .bind(household_id.to_string())
    .bind(email)
    .fetch_optional(pool)
    .await?;

    if existing_pending.is_some() {
        return Err(InvitationError::AlreadyExists);
    }

    let id = Uuid::new_v4();
    let now = Utc::now();
    let expires_at = now + Duration::days(INVITATION_EXPIRY_DAYS);

    sqlx::query(
        r#"
        INSERT INTO household_invitations (id, household_id, email, role, invited_by, status, created_at, expires_at)
        VALUES (?, ?, ?, ?, ?, 'pending', ?, ?)
        "#,
    )
    .bind(id.to_string())
    .bind(household_id.to_string())
    .bind(email)
    .bind(role.as_str())
    .bind(invited_by.to_string())
    .bind(now)
    .bind(expires_at)
    .execute(pool)
    .await?;

    Ok(Invitation {
        id,
        household_id: *household_id,
        email: email.to_string(),
        role,
        invited_by: *invited_by,
        status: InvitationStatus::Pending,
        created_at: now,
        expires_at,
        responded_at: None,
    })
}

/// Get pending invitations for a household
pub async fn get_household_invitations(
    pool: &SqlitePool,
    household_id: &Uuid,
) -> Result<Vec<Invitation>, InvitationError> {
    // First expire any old invitations
    expire_old_invitations(pool).await?;

    let invitations: Vec<InvitationRow> = sqlx::query_as(
        "SELECT * FROM household_invitations WHERE household_id = ? AND status = 'pending' ORDER BY created_at DESC",
    )
    .bind(household_id.to_string())
    .fetch_all(pool)
    .await?;

    Ok(invitations.into_iter().map(|i| i.to_shared()).collect())
}

/// Get pending invitations for a user (by email)
pub async fn get_user_invitations(
    pool: &SqlitePool,
    email: &str,
) -> Result<Vec<InvitationWithHousehold>, InvitationError> {
    // First expire any old invitations
    expire_old_invitations(pool).await?;

    let invitations: Vec<InvitationRow> = sqlx::query_as(
        "SELECT * FROM household_invitations WHERE email = ? AND status = 'pending' ORDER BY created_at DESC",
    )
    .bind(email)
    .fetch_all(pool)
    .await?;

    let mut result = Vec::new();
    for inv in invitations {
        let household: HouseholdRow = sqlx::query_as("SELECT * FROM households WHERE id = ?")
            .bind(&inv.household_id)
            .fetch_one(pool)
            .await?;

        let invited_by_user: UserRow = sqlx::query_as("SELECT * FROM users WHERE id = ?")
            .bind(&inv.invited_by)
            .fetch_one(pool)
            .await?;

        result.push(InvitationWithHousehold {
            invitation: inv.to_shared(),
            household: household.to_shared(),
            invited_by_user: invited_by_user.to_shared(),
        });
    }

    Ok(result)
}

/// Get a single invitation by ID
pub async fn get_invitation(
    pool: &SqlitePool,
    invitation_id: &Uuid,
) -> Result<Invitation, InvitationError> {
    let invitation: InvitationRow =
        sqlx::query_as("SELECT * FROM household_invitations WHERE id = ?")
            .bind(invitation_id.to_string())
            .fetch_optional(pool)
            .await?
            .ok_or(InvitationError::NotFound)?;

    Ok(invitation.to_shared())
}

/// Accept an invitation (creates membership, updates status)
pub async fn accept_invitation(
    pool: &SqlitePool,
    invitation_id: &Uuid,
    user: &User,
) -> Result<HouseholdMembership, InvitationError> {
    let invitation = get_invitation(pool, invitation_id).await?;

    // Check if the invitation is for this user
    if invitation.email.to_lowercase() != user.email.to_lowercase() {
        return Err(InvitationError::NotForUser);
    }

    // Check if invitation is still pending
    if invitation.status != InvitationStatus::Pending {
        return Err(InvitationError::NotFound);
    }

    // Check if expired
    if invitation.expires_at < Utc::now() {
        // Mark as expired
        sqlx::query("UPDATE household_invitations SET status = 'expired' WHERE id = ?")
            .bind(invitation_id.to_string())
            .execute(pool)
            .await?;
        return Err(InvitationError::Expired);
    }

    // Check if already a member (shouldn't happen but safety check)
    let is_member = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM household_memberships WHERE household_id = ? AND user_id = ?",
    )
    .bind(invitation.household_id.to_string())
    .bind(user.id.to_string())
    .fetch_one(pool)
    .await?;

    if is_member > 0 {
        return Err(InvitationError::AlreadyMember);
    }

    let now = Utc::now();

    // Update invitation status
    sqlx::query(
        "UPDATE household_invitations SET status = 'accepted', responded_at = ? WHERE id = ?",
    )
    .bind(now)
    .bind(invitation_id.to_string())
    .execute(pool)
    .await?;

    // Create membership
    let membership_id = Uuid::new_v4();
    sqlx::query(
        r#"
        INSERT INTO household_memberships (id, household_id, user_id, role, points, joined_at)
        VALUES (?, ?, ?, ?, ?, ?)
        "#,
    )
    .bind(membership_id.to_string())
    .bind(invitation.household_id.to_string())
    .bind(user.id.to_string())
    .bind(invitation.role.as_str())
    .bind(0i64)
    .bind(now)
    .execute(pool)
    .await?;

    Ok(HouseholdMembership {
        id: membership_id,
        household_id: invitation.household_id,
        user_id: user.id,
        role: invitation.role,
        points: 0,
        joined_at: now,
    })
}

/// Decline an invitation
pub async fn decline_invitation(
    pool: &SqlitePool,
    invitation_id: &Uuid,
    user: &User,
) -> Result<(), InvitationError> {
    let invitation = get_invitation(pool, invitation_id).await?;

    // Check if the invitation is for this user
    if invitation.email.to_lowercase() != user.email.to_lowercase() {
        return Err(InvitationError::NotForUser);
    }

    // Check if invitation is still pending
    if invitation.status != InvitationStatus::Pending {
        return Err(InvitationError::NotFound);
    }

    let now = Utc::now();
    sqlx::query(
        "UPDATE household_invitations SET status = 'declined', responded_at = ? WHERE id = ?",
    )
    .bind(now)
    .bind(invitation_id.to_string())
    .execute(pool)
    .await?;

    Ok(())
}

/// Cancel/revoke an invitation (by household admin)
pub async fn cancel_invitation(
    pool: &SqlitePool,
    invitation_id: &Uuid,
) -> Result<(), InvitationError> {
    let result = sqlx::query(
        "DELETE FROM household_invitations WHERE id = ? AND status = 'pending'",
    )
    .bind(invitation_id.to_string())
    .execute(pool)
    .await?;

    if result.rows_affected() == 0 {
        return Err(InvitationError::NotFound);
    }

    Ok(())
}

/// Expire old invitations (internal helper)
async fn expire_old_invitations(pool: &SqlitePool) -> Result<(), InvitationError> {
    let now = Utc::now();
    sqlx::query(
        "UPDATE household_invitations SET status = 'expired' WHERE status = 'pending' AND expires_at < ?",
    )
    .bind(now)
    .execute(pool)
    .await?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_invitation_expiry_days() {
        assert_eq!(INVITATION_EXPIRY_DAYS, 7);
    }
}
