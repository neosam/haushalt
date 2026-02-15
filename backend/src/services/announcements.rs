use chrono::Utc;
use sqlx::SqlitePool;
use thiserror::Error;
use uuid::Uuid;

use crate::models::AnnouncementRow;
use shared::{Announcement, CreateAnnouncementRequest, UpdateAnnouncementRequest};

#[derive(Debug, Error)]
pub enum AnnouncementError {
    #[error("Announcement not found")]
    NotFound,
    #[error("Database error: {0}")]
    DatabaseError(#[from] sqlx::Error),
}

pub async fn create_announcement(
    pool: &SqlitePool,
    household_id: &Uuid,
    user_id: &Uuid,
    request: &CreateAnnouncementRequest,
) -> Result<Announcement, AnnouncementError> {
    let id = Uuid::new_v4();
    let now = Utc::now();
    let content = request.content.as_deref().unwrap_or("");

    sqlx::query(
        r#"
        INSERT INTO announcements (id, household_id, created_by, title, content, starts_at, ends_at, created_at, updated_at)
        VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)
        "#,
    )
    .bind(id.to_string())
    .bind(household_id.to_string())
    .bind(user_id.to_string())
    .bind(&request.title)
    .bind(content)
    .bind(request.starts_at)
    .bind(request.ends_at)
    .bind(now)
    .bind(now)
    .execute(pool)
    .await?;

    Ok(Announcement {
        id,
        household_id: *household_id,
        created_by: *user_id,
        title: request.title.clone(),
        content: content.to_string(),
        starts_at: request.starts_at,
        ends_at: request.ends_at,
        created_at: now,
        updated_at: now,
    })
}

pub async fn get_announcement(
    pool: &SqlitePool,
    announcement_id: &Uuid,
) -> Result<Option<Announcement>, AnnouncementError> {
    let announcement: Option<AnnouncementRow> =
        sqlx::query_as("SELECT * FROM announcements WHERE id = ?")
            .bind(announcement_id.to_string())
            .fetch_optional(pool)
            .await?;

    Ok(announcement.map(|a| a.to_shared()))
}

/// List all announcements for a household (for management)
pub async fn list_announcements(
    pool: &SqlitePool,
    household_id: &Uuid,
) -> Result<Vec<Announcement>, AnnouncementError> {
    let rows: Vec<AnnouncementRow> = sqlx::query_as(
        r#"
        SELECT * FROM announcements
        WHERE household_id = ?
        ORDER BY created_at DESC
        "#,
    )
    .bind(household_id.to_string())
    .fetch_all(pool)
    .await?;

    Ok(rows.into_iter().map(|r| r.to_shared()).collect())
}

/// List currently active announcements for a household (for display)
/// Active means:
/// - starts_at IS NULL OR starts_at <= NOW
/// - ends_at IS NULL OR ends_at > NOW
pub async fn list_active_announcements(
    pool: &SqlitePool,
    household_id: &Uuid,
) -> Result<Vec<Announcement>, AnnouncementError> {
    let now = Utc::now();

    let rows: Vec<AnnouncementRow> = sqlx::query_as(
        r#"
        SELECT * FROM announcements
        WHERE household_id = ?
          AND (starts_at IS NULL OR starts_at <= ?)
          AND (ends_at IS NULL OR ends_at > ?)
        ORDER BY created_at DESC
        "#,
    )
    .bind(household_id.to_string())
    .bind(now)
    .bind(now)
    .fetch_all(pool)
    .await?;

    Ok(rows.into_iter().map(|r| r.to_shared()).collect())
}

pub async fn update_announcement(
    pool: &SqlitePool,
    announcement_id: &Uuid,
    request: &UpdateAnnouncementRequest,
) -> Result<Announcement, AnnouncementError> {
    let mut announcement: AnnouncementRow =
        sqlx::query_as("SELECT * FROM announcements WHERE id = ?")
            .bind(announcement_id.to_string())
            .fetch_optional(pool)
            .await?
            .ok_or(AnnouncementError::NotFound)?;

    if let Some(ref title) = request.title {
        announcement.title = title.clone();
    }
    if let Some(ref content) = request.content {
        announcement.content = content.clone();
    }
    if let Some(ref starts_at) = request.starts_at {
        announcement.starts_at = *starts_at;
    }
    if let Some(ref ends_at) = request.ends_at {
        announcement.ends_at = *ends_at;
    }

    let now = Utc::now();
    announcement.updated_at = now;

    sqlx::query(
        r#"
        UPDATE announcements
        SET title = ?, content = ?, starts_at = ?, ends_at = ?, updated_at = ?
        WHERE id = ?
        "#,
    )
    .bind(&announcement.title)
    .bind(&announcement.content)
    .bind(announcement.starts_at)
    .bind(announcement.ends_at)
    .bind(now)
    .bind(announcement_id.to_string())
    .execute(pool)
    .await?;

    Ok(announcement.to_shared())
}

pub async fn delete_announcement(
    pool: &SqlitePool,
    announcement_id: &Uuid,
) -> Result<(), AnnouncementError> {
    let result = sqlx::query("DELETE FROM announcements WHERE id = ?")
        .bind(announcement_id.to_string())
        .execute(pool)
        .await?;

    if result.rows_affected() == 0 {
        return Err(AnnouncementError::NotFound);
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_announcement_error_display() {
        assert_eq!(AnnouncementError::NotFound.to_string(), "Announcement not found");
    }
}
