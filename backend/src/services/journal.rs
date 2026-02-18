use chrono::{NaiveDate, Utc};
use sqlx::SqlitePool;
use thiserror::Error;
use uuid::Uuid;

use crate::models::JournalEntryRow;
use shared::{CreateJournalEntryRequest, JournalEntry, JournalEntryWithUser, UpdateJournalEntryRequest, User};

#[derive(Debug, Error)]
pub enum JournalError {
    #[error("Journal entry not found")]
    NotFound,
    #[error("Permission denied")]
    PermissionDenied,
    #[error("Database error: {0}")]
    DatabaseError(#[from] sqlx::Error),
}

pub async fn create_journal_entry(
    pool: &SqlitePool,
    household_id: &Uuid,
    user_id: &Uuid,
    request: &CreateJournalEntryRequest,
) -> Result<JournalEntry, JournalError> {
    let id = Uuid::new_v4();
    let now = Utc::now();
    let title = request.title.as_deref().unwrap_or("");
    let entry_date = request.entry_date.unwrap_or_else(|| now.date_naive());

    sqlx::query(
        r#"
        INSERT INTO journal_entries (id, household_id, user_id, title, content, entry_date, is_shared, created_at, updated_at)
        VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)
        "#,
    )
    .bind(id.to_string())
    .bind(household_id.to_string())
    .bind(user_id.to_string())
    .bind(title)
    .bind(&request.content)
    .bind(entry_date)
    .bind(request.is_shared)
    .bind(now)
    .bind(now)
    .execute(pool)
    .await?;

    Ok(JournalEntry {
        id,
        household_id: *household_id,
        user_id: *user_id,
        title: title.to_string(),
        content: request.content.clone(),
        entry_date,
        is_shared: request.is_shared,
        created_at: now,
        updated_at: now,
    })
}

pub async fn get_journal_entry(pool: &SqlitePool, entry_id: &Uuid) -> Result<Option<JournalEntry>, JournalError> {
    let entry: Option<JournalEntryRow> = sqlx::query_as("SELECT * FROM journal_entries WHERE id = ?")
        .bind(entry_id.to_string())
        .fetch_optional(pool)
        .await?;

    Ok(entry.map(|e| e.to_shared()))
}

/// List all journal entries visible to a user in a household:
/// - All shared entries (is_shared = true)
/// - User's private entries (is_shared = false AND user_id = current_user)
pub async fn list_journal_entries(
    pool: &SqlitePool,
    household_id: &Uuid,
    user_id: &Uuid,
) -> Result<Vec<JournalEntryWithUser>, JournalError> {
    #[derive(sqlx::FromRow)]
    struct JournalEntryWithUserRow {
        // Journal entry fields
        j_id: String,
        j_household_id: String,
        j_user_id: String,
        j_title: String,
        j_content: String,
        j_entry_date: NaiveDate,
        j_is_shared: bool,
        j_created_at: chrono::DateTime<chrono::Utc>,
        j_updated_at: chrono::DateTime<chrono::Utc>,
        // User fields
        u_id: String,
        u_username: String,
        u_email: String,
        u_created_at: chrono::DateTime<chrono::Utc>,
        u_updated_at: chrono::DateTime<chrono::Utc>,
    }

    let rows: Vec<JournalEntryWithUserRow> = sqlx::query_as(
        r#"
        SELECT
            j.id as j_id, j.household_id as j_household_id, j.user_id as j_user_id,
            j.title as j_title, j.content as j_content, j.entry_date as j_entry_date,
            j.is_shared as j_is_shared, j.created_at as j_created_at, j.updated_at as j_updated_at,
            u.id as u_id, u.username as u_username, u.email as u_email,
            u.created_at as u_created_at, u.updated_at as u_updated_at
        FROM journal_entries j
        JOIN users u ON j.user_id = u.id
        WHERE j.household_id = ?
          AND (j.is_shared = true OR j.user_id = ?)
        ORDER BY j.entry_date DESC, j.created_at DESC
        "#,
    )
    .bind(household_id.to_string())
    .bind(user_id.to_string())
    .fetch_all(pool)
    .await?;

    Ok(rows
        .into_iter()
        .map(|row| JournalEntryWithUser {
            entry: JournalEntry {
                id: Uuid::parse_str(&row.j_id).unwrap(),
                household_id: Uuid::parse_str(&row.j_household_id).unwrap(),
                user_id: Uuid::parse_str(&row.j_user_id).unwrap(),
                title: row.j_title,
                content: row.j_content,
                entry_date: row.j_entry_date,
                is_shared: row.j_is_shared,
                created_at: row.j_created_at,
                updated_at: row.j_updated_at,
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

/// Check if user can view a journal entry
pub fn can_view_entry(entry: &JournalEntry, user_id: &Uuid) -> bool {
    entry.is_shared || entry.user_id == *user_id
}

pub async fn update_journal_entry(
    pool: &SqlitePool,
    entry_id: &Uuid,
    user_id: &Uuid,
    request: &UpdateJournalEntryRequest,
) -> Result<JournalEntry, JournalError> {
    let mut entry: JournalEntryRow = sqlx::query_as("SELECT * FROM journal_entries WHERE id = ?")
        .bind(entry_id.to_string())
        .fetch_optional(pool)
        .await?
        .ok_or(JournalError::NotFound)?;

    // Check permission - only the author can edit
    if entry.user_id != user_id.to_string() {
        return Err(JournalError::PermissionDenied);
    }

    if let Some(ref title) = request.title {
        entry.title = title.clone();
    }
    if let Some(ref content) = request.content {
        entry.content = content.clone();
    }
    if let Some(entry_date) = request.entry_date {
        entry.entry_date = entry_date;
    }
    if let Some(is_shared) = request.is_shared {
        entry.is_shared = is_shared;
    }

    let now = Utc::now();
    entry.updated_at = now;

    sqlx::query(
        r#"
        UPDATE journal_entries SET title = ?, content = ?, entry_date = ?, is_shared = ?, updated_at = ?
        WHERE id = ?
        "#,
    )
    .bind(&entry.title)
    .bind(&entry.content)
    .bind(entry.entry_date)
    .bind(entry.is_shared)
    .bind(now)
    .bind(entry_id.to_string())
    .execute(pool)
    .await?;

    Ok(entry.to_shared())
}

pub async fn delete_journal_entry(
    pool: &SqlitePool,
    entry_id: &Uuid,
    user_id: &Uuid,
) -> Result<(), JournalError> {
    let entry: JournalEntryRow = sqlx::query_as("SELECT * FROM journal_entries WHERE id = ?")
        .bind(entry_id.to_string())
        .fetch_optional(pool)
        .await?
        .ok_or(JournalError::NotFound)?;

    // Check permission - only the author can delete
    if entry.user_id != user_id.to_string() {
        return Err(JournalError::PermissionDenied);
    }

    sqlx::query("DELETE FROM journal_entries WHERE id = ?")
        .bind(entry_id.to_string())
        .execute(pool)
        .await?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_journal_error_display() {
        assert_eq!(JournalError::NotFound.to_string(), "Journal entry not found");
        assert_eq!(JournalError::PermissionDenied.to_string(), "Permission denied");
    }

    #[test]
    fn test_can_view_entry_shared() {
        let entry = JournalEntry {
            id: Uuid::new_v4(),
            household_id: Uuid::new_v4(),
            user_id: Uuid::new_v4(),
            title: "Test".to_string(),
            content: "Content".to_string(),
            entry_date: NaiveDate::from_ymd_opt(2026, 2, 18).unwrap(),
            is_shared: true,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        let other_user = Uuid::new_v4();
        assert!(can_view_entry(&entry, &other_user));
        assert!(can_view_entry(&entry, &entry.user_id));
    }

    #[test]
    fn test_can_view_entry_private() {
        let entry = JournalEntry {
            id: Uuid::new_v4(),
            household_id: Uuid::new_v4(),
            user_id: Uuid::new_v4(),
            title: "Test".to_string(),
            content: "Content".to_string(),
            entry_date: NaiveDate::from_ymd_opt(2026, 2, 18).unwrap(),
            is_shared: false,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        let other_user = Uuid::new_v4();
        assert!(!can_view_entry(&entry, &other_user));
        assert!(can_view_entry(&entry, &entry.user_id));
    }
}
