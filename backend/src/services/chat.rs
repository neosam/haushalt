use chrono::Utc;
use sqlx::SqlitePool;
use thiserror::Error;
use uuid::Uuid;

use crate::models::ChatMessageWithUserRow;
use shared::{ChatMessage, ChatMessageWithUser};

#[derive(Debug, Error)]
pub enum ChatError {
    #[error("Message not found")]
    NotFound,
    #[error("Not authorized to modify this message")]
    NotAuthorized,
    #[error("Message content cannot be empty")]
    EmptyContent,
    #[error("Database error: {0}")]
    DatabaseError(#[from] sqlx::Error),
}

/// Create a new chat message
pub async fn create_message(
    pool: &SqlitePool,
    household_id: &Uuid,
    user_id: &Uuid,
    content: &str,
) -> Result<ChatMessage, ChatError> {
    let content = content.trim();
    if content.is_empty() {
        return Err(ChatError::EmptyContent);
    }

    let id = Uuid::new_v4();
    let now = Utc::now();

    sqlx::query(
        r#"
        INSERT INTO chat_messages (id, household_id, user_id, content, created_at, updated_at)
        VALUES (?, ?, ?, ?, ?, ?)
        "#,
    )
    .bind(id.to_string())
    .bind(household_id.to_string())
    .bind(user_id.to_string())
    .bind(content)
    .bind(now)
    .bind(now)
    .execute(pool)
    .await?;

    Ok(ChatMessage {
        id,
        household_id: *household_id,
        user_id: *user_id,
        content: content.to_string(),
        created_at: now,
        updated_at: now,
        is_deleted: false,
    })
}

/// Get a single message by ID
pub async fn get_message(
    pool: &SqlitePool,
    message_id: &Uuid,
) -> Result<Option<ChatMessage>, ChatError> {
    #[derive(sqlx::FromRow)]
    struct Row {
        id: String,
        household_id: String,
        user_id: String,
        content: String,
        created_at: chrono::DateTime<chrono::Utc>,
        updated_at: chrono::DateTime<chrono::Utc>,
        deleted_at: Option<chrono::DateTime<chrono::Utc>>,
    }

    let row: Option<Row> = sqlx::query_as(
        "SELECT id, household_id, user_id, content, created_at, updated_at, deleted_at FROM chat_messages WHERE id = ?",
    )
    .bind(message_id.to_string())
    .fetch_optional(pool)
    .await?;

    Ok(row.map(|r| ChatMessage {
        id: Uuid::parse_str(&r.id).unwrap(),
        household_id: Uuid::parse_str(&r.household_id).unwrap(),
        user_id: Uuid::parse_str(&r.user_id).unwrap(),
        content: r.content,
        created_at: r.created_at,
        updated_at: r.updated_at,
        is_deleted: r.deleted_at.is_some(),
    }))
}

/// Get a message with its user information
pub async fn get_message_with_user(
    pool: &SqlitePool,
    message_id: &Uuid,
) -> Result<Option<ChatMessageWithUser>, ChatError> {
    let row: Option<ChatMessageWithUserRow> = sqlx::query_as(
        r#"
        SELECT
            m.id, m.household_id, m.user_id, m.content,
            m.created_at, m.updated_at, m.deleted_at,
            u.username, u.email,
            u.created_at as user_created_at, u.updated_at as user_updated_at
        FROM chat_messages m
        JOIN users u ON m.user_id = u.id
        WHERE m.id = ?
        "#,
    )
    .bind(message_id.to_string())
    .fetch_optional(pool)
    .await?;

    Ok(row.map(|r| r.to_shared()))
}

/// List messages for a household with pagination
/// Returns messages in descending order (newest first)
/// Use `before` to get messages older than a specific message ID
pub async fn list_messages(
    pool: &SqlitePool,
    household_id: &Uuid,
    limit: i64,
    before: Option<&Uuid>,
) -> Result<Vec<ChatMessageWithUser>, ChatError> {
    let rows: Vec<ChatMessageWithUserRow> = if let Some(before_id) = before {
        // Get the created_at of the before message
        let before_created_at: Option<chrono::DateTime<chrono::Utc>> = sqlx::query_scalar(
            "SELECT created_at FROM chat_messages WHERE id = ?",
        )
        .bind(before_id.to_string())
        .fetch_optional(pool)
        .await?;

        if let Some(created_at) = before_created_at {
            sqlx::query_as(
                r#"
                SELECT
                    m.id, m.household_id, m.user_id, m.content,
                    m.created_at, m.updated_at, m.deleted_at,
                    u.username, u.email,
                    u.created_at as user_created_at, u.updated_at as user_updated_at
                FROM chat_messages m
                JOIN users u ON m.user_id = u.id
                WHERE m.household_id = ? AND m.deleted_at IS NULL AND m.created_at < ?
                ORDER BY m.created_at DESC
                LIMIT ?
                "#,
            )
            .bind(household_id.to_string())
            .bind(created_at)
            .bind(limit)
            .fetch_all(pool)
            .await?
        } else {
            Vec::new()
        }
    } else {
        sqlx::query_as(
            r#"
            SELECT
                m.id, m.household_id, m.user_id, m.content,
                m.created_at, m.updated_at, m.deleted_at,
                u.username, u.email,
                u.created_at as user_created_at, u.updated_at as user_updated_at
            FROM chat_messages m
            JOIN users u ON m.user_id = u.id
            WHERE m.household_id = ? AND m.deleted_at IS NULL
            ORDER BY m.created_at DESC
            LIMIT ?
            "#,
        )
        .bind(household_id.to_string())
        .bind(limit)
        .fetch_all(pool)
        .await?
    };

    Ok(rows.into_iter().map(|r| r.to_shared()).collect())
}

/// Update message content - only the author can edit
pub async fn update_message(
    pool: &SqlitePool,
    message_id: &Uuid,
    user_id: &Uuid,
    content: &str,
) -> Result<ChatMessage, ChatError> {
    let content = content.trim();
    if content.is_empty() {
        return Err(ChatError::EmptyContent);
    }

    let message = get_message(pool, message_id).await?.ok_or(ChatError::NotFound)?;

    if message.user_id != *user_id {
        return Err(ChatError::NotAuthorized);
    }

    if message.is_deleted {
        return Err(ChatError::NotFound);
    }

    let now = Utc::now();

    sqlx::query("UPDATE chat_messages SET content = ?, updated_at = ? WHERE id = ?")
        .bind(content)
        .bind(now)
        .bind(message_id.to_string())
        .execute(pool)
        .await?;

    Ok(ChatMessage {
        id: message.id,
        household_id: message.household_id,
        user_id: message.user_id,
        content: content.to_string(),
        created_at: message.created_at,
        updated_at: now,
        is_deleted: false,
    })
}

/// Soft delete a message - only the author can delete
pub async fn delete_message(
    pool: &SqlitePool,
    message_id: &Uuid,
    user_id: &Uuid,
) -> Result<(), ChatError> {
    let message = get_message(pool, message_id).await?.ok_or(ChatError::NotFound)?;

    if message.user_id != *user_id {
        return Err(ChatError::NotAuthorized);
    }

    if message.is_deleted {
        return Ok(()); // Already deleted
    }

    let now = Utc::now();

    sqlx::query("UPDATE chat_messages SET deleted_at = ?, updated_at = ? WHERE id = ?")
        .bind(now)
        .bind(now)
        .bind(message_id.to_string())
        .execute(pool)
        .await?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_chat_error_display() {
        assert_eq!(ChatError::NotFound.to_string(), "Message not found");
        assert_eq!(ChatError::NotAuthorized.to_string(), "Not authorized to modify this message");
        assert_eq!(ChatError::EmptyContent.to_string(), "Message content cannot be empty");
    }
}
