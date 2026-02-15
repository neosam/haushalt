use chrono::Utc;
use sqlx::SqlitePool;
use thiserror::Error;
use uuid::Uuid;

use crate::models::ActivityLogRow;
use shared::{ActivityLog, ActivityLogWithUsers, ActivityType, User};

#[derive(Debug, Error)]
pub enum ActivityLogError {
    #[error("Activity log not found")]
    #[allow(dead_code)]
    NotFound,
    #[error("Database error: {0}")]
    DatabaseError(#[from] sqlx::Error),
}

/// Log an activity event
#[allow(clippy::too_many_arguments)]
pub async fn log_activity(
    pool: &SqlitePool,
    household_id: &Uuid,
    actor_id: &Uuid,
    affected_user_id: Option<&Uuid>,
    activity_type: ActivityType,
    entity_type: Option<&str>,
    entity_id: Option<&Uuid>,
    details: Option<&str>,
) -> Result<ActivityLog, ActivityLogError> {
    let id = Uuid::new_v4();
    let now = Utc::now();

    sqlx::query(
        r#"
        INSERT INTO activity_logs (id, household_id, actor_id, affected_user_id, activity_type, entity_type, entity_id, details, created_at)
        VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)
        "#,
    )
    .bind(id.to_string())
    .bind(household_id.to_string())
    .bind(actor_id.to_string())
    .bind(affected_user_id.map(|u| u.to_string()))
    .bind(activity_type.as_str())
    .bind(entity_type)
    .bind(entity_id.map(|e| e.to_string()))
    .bind(details)
    .bind(now)
    .execute(pool)
    .await?;

    Ok(ActivityLog {
        id,
        household_id: *household_id,
        actor_id: *actor_id,
        affected_user_id: affected_user_id.copied(),
        activity_type,
        entity_type: entity_type.map(|s| s.to_string()),
        entity_id: entity_id.copied(),
        details: details.map(|s| s.to_string()),
        created_at: now,
    })
}

// Struct for joined queries
#[derive(sqlx::FromRow)]
struct JoinedActivityRow {
    id: String,
    household_id: String,
    actor_id: String,
    affected_user_id: Option<String>,
    activity_type: String,
    entity_type: Option<String>,
    entity_id: Option<String>,
    details: Option<String>,
    created_at: chrono::DateTime<chrono::Utc>,
    actor_username: String,
    actor_email: String,
    actor_created_at: chrono::DateTime<chrono::Utc>,
    actor_updated_at: chrono::DateTime<chrono::Utc>,
    affected_username: Option<String>,
    affected_email: Option<String>,
    affected_created_at: Option<chrono::DateTime<chrono::Utc>>,
    affected_updated_at: Option<chrono::DateTime<chrono::Utc>>,
}

impl JoinedActivityRow {
    fn into_activity_log_with_users(self) -> ActivityLogWithUsers {
        let log = ActivityLog {
            id: Uuid::parse_str(&self.id).unwrap(),
            household_id: Uuid::parse_str(&self.household_id).unwrap(),
            actor_id: Uuid::parse_str(&self.actor_id).unwrap(),
            affected_user_id: self.affected_user_id.as_ref().map(|s| Uuid::parse_str(s).unwrap()),
            activity_type: self.activity_type.parse().unwrap_or(ActivityType::TaskCreated),
            entity_type: self.entity_type,
            entity_id: self.entity_id.as_ref().map(|s| Uuid::parse_str(s).unwrap()),
            details: self.details,
            created_at: self.created_at,
        };

        let actor = User {
            id: Uuid::parse_str(&self.actor_id).unwrap(),
            username: self.actor_username,
            email: self.actor_email,
            created_at: self.actor_created_at,
            updated_at: self.actor_updated_at,
        };

        let affected_user = if let (Some(username), Some(email), Some(created_at), Some(updated_at)) = (
            self.affected_username,
            self.affected_email,
            self.affected_created_at,
            self.affected_updated_at,
        ) {
            self.affected_user_id.as_ref().map(|id| User {
                id: Uuid::parse_str(id).unwrap(),
                username,
                email,
                created_at,
                updated_at,
            })
        } else {
            None
        };

        ActivityLogWithUsers {
            log,
            actor,
            affected_user,
        }
    }
}

/// List all activities for a household (for owners)
pub async fn list_household_activities(
    pool: &SqlitePool,
    household_id: &Uuid,
    limit: Option<i64>,
) -> Result<Vec<ActivityLogWithUsers>, ActivityLogError> {
    let limit = limit.unwrap_or(100);

    let rows: Vec<JoinedActivityRow> = sqlx::query_as(
        r#"
        SELECT
            al.id, al.household_id, al.actor_id, al.affected_user_id,
            al.activity_type, al.entity_type, al.entity_id, al.details, al.created_at,
            actor.username as actor_username, actor.email as actor_email,
            actor.created_at as actor_created_at, actor.updated_at as actor_updated_at,
            affected.username as affected_username, affected.email as affected_email,
            affected.created_at as affected_created_at, affected.updated_at as affected_updated_at
        FROM activity_logs al
        JOIN users actor ON al.actor_id = actor.id
        LEFT JOIN users affected ON al.affected_user_id = affected.id
        WHERE al.household_id = ?
        ORDER BY al.created_at DESC
        LIMIT ?
        "#,
    )
    .bind(household_id.to_string())
    .bind(limit)
    .fetch_all(pool)
    .await?;

    Ok(rows.into_iter().map(|row| row.into_activity_log_with_users()).collect())
}

/// List activities affecting a specific user (for non-owners)
pub async fn list_user_activities(
    pool: &SqlitePool,
    household_id: &Uuid,
    user_id: &Uuid,
    limit: Option<i64>,
) -> Result<Vec<ActivityLogWithUsers>, ActivityLogError> {
    let limit = limit.unwrap_or(100);

    let rows: Vec<JoinedActivityRow> = sqlx::query_as(
        r#"
        SELECT
            al.id, al.household_id, al.actor_id, al.affected_user_id,
            al.activity_type, al.entity_type, al.entity_id, al.details, al.created_at,
            actor.username as actor_username, actor.email as actor_email,
            actor.created_at as actor_created_at, actor.updated_at as actor_updated_at,
            affected.username as affected_username, affected.email as affected_email,
            affected.created_at as affected_created_at, affected.updated_at as affected_updated_at
        FROM activity_logs al
        JOIN users actor ON al.actor_id = actor.id
        LEFT JOIN users affected ON al.affected_user_id = affected.id
        WHERE al.household_id = ? AND (al.affected_user_id = ? OR al.actor_id = ?)
        ORDER BY al.created_at DESC
        LIMIT ?
        "#,
    )
    .bind(household_id.to_string())
    .bind(user_id.to_string())
    .bind(user_id.to_string())
    .bind(limit)
    .fetch_all(pool)
    .await?;

    Ok(rows.into_iter().map(|row| row.into_activity_log_with_users()).collect())
}

/// Get a single activity log by ID
#[allow(dead_code)]
pub async fn get_activity_log(
    pool: &SqlitePool,
    activity_id: &Uuid,
) -> Result<Option<ActivityLog>, ActivityLogError> {
    let log: Option<ActivityLogRow> = sqlx::query_as("SELECT * FROM activity_logs WHERE id = ?")
        .bind(activity_id.to_string())
        .fetch_optional(pool)
        .await?;

    Ok(log.map(|l| l.to_shared()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_activity_log_error_display() {
        assert_eq!(ActivityLogError::NotFound.to_string(), "Activity log not found");
    }
}
