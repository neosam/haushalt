use chrono::Utc;
use sqlx::SqlitePool;
use thiserror::Error;
use uuid::Uuid;

use crate::models::NoteRow;
use shared::{CreateNoteRequest, Note, NoteWithUser, UpdateNoteRequest, User};

#[derive(Debug, Error)]
pub enum NoteError {
    #[error("Note not found")]
    NotFound,
    #[error("Permission denied")]
    PermissionDenied,
    #[error("Database error: {0}")]
    DatabaseError(#[from] sqlx::Error),
}

pub async fn create_note(
    pool: &SqlitePool,
    household_id: &Uuid,
    user_id: &Uuid,
    request: &CreateNoteRequest,
) -> Result<Note, NoteError> {
    let id = Uuid::new_v4();
    let now = Utc::now();
    let content = request.content.as_deref().unwrap_or("");

    sqlx::query(
        r#"
        INSERT INTO notes (id, household_id, user_id, title, content, is_shared, created_at, updated_at)
        VALUES (?, ?, ?, ?, ?, ?, ?, ?)
        "#,
    )
    .bind(id.to_string())
    .bind(household_id.to_string())
    .bind(user_id.to_string())
    .bind(&request.title)
    .bind(content)
    .bind(request.is_shared)
    .bind(now)
    .bind(now)
    .execute(pool)
    .await?;

    Ok(Note {
        id,
        household_id: *household_id,
        user_id: *user_id,
        title: request.title.clone(),
        content: content.to_string(),
        is_shared: request.is_shared,
        created_at: now,
        updated_at: now,
    })
}

pub async fn get_note(pool: &SqlitePool, note_id: &Uuid) -> Result<Option<Note>, NoteError> {
    let note: Option<NoteRow> = sqlx::query_as("SELECT * FROM notes WHERE id = ?")
        .bind(note_id.to_string())
        .fetch_optional(pool)
        .await?;

    Ok(note.map(|n| n.to_shared()))
}

/// List all notes visible to a user in a household:
/// - All shared notes (is_shared = true)
/// - User's private notes (is_shared = false AND user_id = current_user)
pub async fn list_notes(
    pool: &SqlitePool,
    household_id: &Uuid,
    user_id: &Uuid,
) -> Result<Vec<NoteWithUser>, NoteError> {
    #[derive(sqlx::FromRow)]
    struct NoteWithUserRow {
        // Note fields
        n_id: String,
        n_household_id: String,
        n_user_id: String,
        n_title: String,
        n_content: String,
        n_is_shared: bool,
        n_created_at: chrono::DateTime<chrono::Utc>,
        n_updated_at: chrono::DateTime<chrono::Utc>,
        // User fields
        u_id: String,
        u_username: String,
        u_email: String,
        u_created_at: chrono::DateTime<chrono::Utc>,
        u_updated_at: chrono::DateTime<chrono::Utc>,
    }

    let rows: Vec<NoteWithUserRow> = sqlx::query_as(
        r#"
        SELECT
            n.id as n_id, n.household_id as n_household_id, n.user_id as n_user_id,
            n.title as n_title, n.content as n_content, n.is_shared as n_is_shared,
            n.created_at as n_created_at, n.updated_at as n_updated_at,
            u.id as u_id, u.username as u_username, u.email as u_email,
            u.created_at as u_created_at, u.updated_at as u_updated_at
        FROM notes n
        JOIN users u ON n.user_id = u.id
        WHERE n.household_id = ?
          AND (n.is_shared = true OR n.user_id = ?)
        ORDER BY n.updated_at DESC
        "#,
    )
    .bind(household_id.to_string())
    .bind(user_id.to_string())
    .fetch_all(pool)
    .await?;

    Ok(rows
        .into_iter()
        .map(|row| NoteWithUser {
            note: Note {
                id: Uuid::parse_str(&row.n_id).unwrap(),
                household_id: Uuid::parse_str(&row.n_household_id).unwrap(),
                user_id: Uuid::parse_str(&row.n_user_id).unwrap(),
                title: row.n_title,
                content: row.n_content,
                is_shared: row.n_is_shared,
                created_at: row.n_created_at,
                updated_at: row.n_updated_at,
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

/// Check if user can view a note
pub fn can_view_note(note: &Note, user_id: &Uuid) -> bool {
    note.is_shared || note.user_id == *user_id
}

pub async fn update_note(
    pool: &SqlitePool,
    note_id: &Uuid,
    user_id: &Uuid,
    request: &UpdateNoteRequest,
) -> Result<Note, NoteError> {
    let mut note: NoteRow = sqlx::query_as("SELECT * FROM notes WHERE id = ?")
        .bind(note_id.to_string())
        .fetch_optional(pool)
        .await?
        .ok_or(NoteError::NotFound)?;

    // Check permission
    if note.user_id != user_id.to_string() {
        return Err(NoteError::PermissionDenied);
    }

    if let Some(ref title) = request.title {
        note.title = title.clone();
    }
    if let Some(ref content) = request.content {
        note.content = content.clone();
    }
    if let Some(is_shared) = request.is_shared {
        note.is_shared = is_shared;
    }

    let now = Utc::now();
    note.updated_at = now;

    sqlx::query(
        r#"
        UPDATE notes SET title = ?, content = ?, is_shared = ?, updated_at = ?
        WHERE id = ?
        "#,
    )
    .bind(&note.title)
    .bind(&note.content)
    .bind(note.is_shared)
    .bind(now)
    .bind(note_id.to_string())
    .execute(pool)
    .await?;

    Ok(note.to_shared())
}

pub async fn delete_note(
    pool: &SqlitePool,
    note_id: &Uuid,
    user_id: &Uuid,
) -> Result<(), NoteError> {
    let note: NoteRow = sqlx::query_as("SELECT * FROM notes WHERE id = ?")
        .bind(note_id.to_string())
        .fetch_optional(pool)
        .await?
        .ok_or(NoteError::NotFound)?;

    // Check permission
    if note.user_id != user_id.to_string() {
        return Err(NoteError::PermissionDenied);
    }

    sqlx::query("DELETE FROM notes WHERE id = ?")
        .bind(note_id.to_string())
        .execute(pool)
        .await?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_note_error_display() {
        assert_eq!(NoteError::NotFound.to_string(), "Note not found");
        assert_eq!(NoteError::PermissionDenied.to_string(), "Permission denied");
    }

    #[test]
    fn test_can_view_note_shared() {
        let note = Note {
            id: Uuid::new_v4(),
            household_id: Uuid::new_v4(),
            user_id: Uuid::new_v4(),
            title: "Test".to_string(),
            content: "Content".to_string(),
            is_shared: true,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        let other_user = Uuid::new_v4();
        assert!(can_view_note(&note, &other_user));
        assert!(can_view_note(&note, &note.user_id));
    }

    #[test]
    fn test_can_view_note_private() {
        let note = Note {
            id: Uuid::new_v4(),
            household_id: Uuid::new_v4(),
            user_id: Uuid::new_v4(),
            title: "Test".to_string(),
            content: "Content".to_string(),
            is_shared: false,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        let other_user = Uuid::new_v4();
        assert!(!can_view_note(&note, &other_user));
        assert!(can_view_note(&note, &note.user_id));
    }

}
