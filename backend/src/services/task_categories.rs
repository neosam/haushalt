use chrono::Utc;
use sqlx::SqlitePool;
use thiserror::Error;
use uuid::Uuid;

use crate::models::TaskCategoryRow;
use shared::{CreateTaskCategoryRequest, TaskCategory, UpdateTaskCategoryRequest};

#[derive(Debug, Error)]
pub enum TaskCategoryError {
    #[error("Category not found")]
    NotFound,
    #[error("Category name already exists in this household")]
    DuplicateName,
    #[error("Database error: {0}")]
    DatabaseError(#[from] sqlx::Error),
}

pub async fn create_category(
    pool: &SqlitePool,
    household_id: &Uuid,
    request: &CreateTaskCategoryRequest,
) -> Result<TaskCategory, TaskCategoryError> {
    let id = Uuid::new_v4();
    let now = Utc::now();
    let sort_order = request.sort_order.unwrap_or(0);

    let result = sqlx::query(
        r#"
        INSERT INTO task_categories (id, household_id, name, color, sort_order, created_at)
        VALUES (?, ?, ?, ?, ?, ?)
        "#,
    )
    .bind(id.to_string())
    .bind(household_id.to_string())
    .bind(&request.name)
    .bind(&request.color)
    .bind(sort_order)
    .bind(now)
    .execute(pool)
    .await;

    match result {
        Ok(_) => Ok(TaskCategory {
            id,
            household_id: *household_id,
            name: request.name.clone(),
            color: request.color.clone(),
            sort_order,
            created_at: now,
        }),
        Err(sqlx::Error::Database(e)) if e.message().contains("UNIQUE constraint failed") => {
            Err(TaskCategoryError::DuplicateName)
        }
        Err(e) => Err(TaskCategoryError::DatabaseError(e)),
    }
}

pub async fn get_category(
    pool: &SqlitePool,
    category_id: &Uuid,
) -> Result<Option<TaskCategory>, TaskCategoryError> {
    let category: Option<TaskCategoryRow> =
        sqlx::query_as("SELECT * FROM task_categories WHERE id = ?")
            .bind(category_id.to_string())
            .fetch_optional(pool)
            .await?;

    Ok(category.map(|c| c.to_shared()))
}

pub async fn list_categories(
    pool: &SqlitePool,
    household_id: &Uuid,
) -> Result<Vec<TaskCategory>, TaskCategoryError> {
    let categories: Vec<TaskCategoryRow> = sqlx::query_as(
        "SELECT * FROM task_categories WHERE household_id = ? ORDER BY sort_order ASC, name ASC",
    )
    .bind(household_id.to_string())
    .fetch_all(pool)
    .await?;

    Ok(categories.into_iter().map(|c| c.to_shared()).collect())
}

pub async fn update_category(
    pool: &SqlitePool,
    category_id: &Uuid,
    request: &UpdateTaskCategoryRequest,
) -> Result<TaskCategory, TaskCategoryError> {
    let mut category: TaskCategoryRow =
        sqlx::query_as("SELECT * FROM task_categories WHERE id = ?")
            .bind(category_id.to_string())
            .fetch_optional(pool)
            .await?
            .ok_or(TaskCategoryError::NotFound)?;

    if let Some(ref name) = request.name {
        category.name = name.clone();
    }
    if let Some(ref color) = request.color {
        category.color = Some(color.clone());
    }
    if let Some(sort_order) = request.sort_order {
        category.sort_order = sort_order;
    }

    let result = sqlx::query(
        r#"
        UPDATE task_categories
        SET name = ?, color = ?, sort_order = ?
        WHERE id = ?
        "#,
    )
    .bind(&category.name)
    .bind(&category.color)
    .bind(category.sort_order)
    .bind(category_id.to_string())
    .execute(pool)
    .await;

    match result {
        Ok(_) => Ok(category.to_shared()),
        Err(sqlx::Error::Database(e)) if e.message().contains("UNIQUE constraint failed") => {
            Err(TaskCategoryError::DuplicateName)
        }
        Err(e) => Err(TaskCategoryError::DatabaseError(e)),
    }
}

pub async fn delete_category(
    pool: &SqlitePool,
    category_id: &Uuid,
) -> Result<(), TaskCategoryError> {
    let result = sqlx::query("DELETE FROM task_categories WHERE id = ?")
        .bind(category_id.to_string())
        .execute(pool)
        .await?;

    if result.rows_affected() == 0 {
        return Err(TaskCategoryError::NotFound);
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_task_category_error_display() {
        assert_eq!(TaskCategoryError::NotFound.to_string(), "Category not found");
        assert_eq!(
            TaskCategoryError::DuplicateName.to_string(),
            "Category name already exists in this household"
        );
    }
}
