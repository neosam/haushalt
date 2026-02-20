use argon2::{
    password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
    Argon2,
};
use rand_core::OsRng;
use chrono::{Duration, Utc};
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};
use sha2::{Sha256, Digest};
use sqlx::SqlitePool;
use thiserror::Error;
use uuid::Uuid;

use crate::models::{UserRow, RefreshTokenRow};
use shared::{CreateUserRequest, UpdateUserRequest, User};

#[derive(Debug, Error)]
pub enum AuthError {
    #[error("User already exists")]
    UserAlreadyExists,
    #[error("Invalid credentials")]
    InvalidCredentials,
    #[error("User not found")]
    UserNotFound,
    #[error("Invalid refresh token")]
    InvalidRefreshToken,
    #[error("Refresh token expired")]
    RefreshTokenExpired,
    #[error("Database error: {0}")]
    DatabaseError(#[from] sqlx::Error),
    #[error("Password hashing error")]
    HashingError,
    #[error("JWT error: {0}")]
    JwtError(#[from] jsonwebtoken::errors::Error),
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    pub sub: String,
    pub exp: i64,
    pub iat: i64,
}

pub async fn register_user(pool: &SqlitePool, request: &CreateUserRequest) -> Result<User, AuthError> {
    // Check if user exists
    let existing = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM users WHERE username = ? OR email = ?"
    )
    .bind(&request.username)
    .bind(&request.email)
    .fetch_one(pool)
    .await?;

    if existing > 0 {
        return Err(AuthError::UserAlreadyExists);
    }

    // Hash password
    let salt = SaltString::generate(&mut OsRng);
    let argon2 = Argon2::default();
    let password_hash = argon2
        .hash_password(request.password.as_bytes(), &salt)
        .map_err(|_| AuthError::HashingError)?
        .to_string();

    let id = Uuid::new_v4();
    let now = Utc::now();

    sqlx::query(
        r#"
        INSERT INTO users (id, username, email, password_hash, created_at, updated_at)
        VALUES (?, ?, ?, ?, ?, ?)
        "#
    )
    .bind(id.to_string())
    .bind(&request.username)
    .bind(&request.email)
    .bind(&password_hash)
    .bind(now)
    .bind(now)
    .execute(pool)
    .await?;

    Ok(User {
        id,
        username: request.username.clone(),
        email: request.email.clone(),
        created_at: now,
        updated_at: now,
    })
}

pub async fn login_user(pool: &SqlitePool, request: &shared::LoginRequest) -> Result<User, AuthError> {
    // Accept username or email, case insensitive
    let user: UserRow = sqlx::query_as(
        "SELECT * FROM users WHERE LOWER(username) = LOWER(?) OR LOWER(email) = LOWER(?)"
    )
    .bind(&request.username)
    .bind(&request.username)
    .fetch_optional(pool)
    .await?
    .ok_or(AuthError::InvalidCredentials)?;

    let password_hash = user.password_hash.as_ref().ok_or(AuthError::InvalidCredentials)?;

    let parsed_hash = PasswordHash::new(password_hash)
        .map_err(|_| AuthError::InvalidCredentials)?;

    Argon2::default()
        .verify_password(request.password.as_bytes(), &parsed_hash)
        .map_err(|_| AuthError::InvalidCredentials)?;

    Ok(user.to_shared())
}

pub async fn get_user_by_id(pool: &SqlitePool, user_id: &Uuid) -> Result<Option<User>, AuthError> {
    let user: Option<UserRow> = sqlx::query_as(
        "SELECT * FROM users WHERE id = ?"
    )
    .bind(user_id.to_string())
    .fetch_optional(pool)
    .await?;

    Ok(user.map(|u| u.to_shared()))
}

pub async fn update_user(pool: &SqlitePool, user_id: &Uuid, request: &UpdateUserRequest) -> Result<User, AuthError> {
    let mut user: UserRow = sqlx::query_as(
        "SELECT * FROM users WHERE id = ?"
    )
    .bind(user_id.to_string())
    .fetch_optional(pool)
    .await?
    .ok_or(AuthError::UserNotFound)?;

    if let Some(ref username) = request.username {
        // Check if username is taken
        let existing = sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*) FROM users WHERE username = ? AND id != ?"
        )
        .bind(username)
        .bind(user_id.to_string())
        .fetch_one(pool)
        .await?;

        if existing > 0 {
            return Err(AuthError::UserAlreadyExists);
        }
        user.username = username.clone();
    }

    if let Some(ref email) = request.email {
        // Check if email is taken
        let existing = sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*) FROM users WHERE email = ? AND id != ?"
        )
        .bind(email)
        .bind(user_id.to_string())
        .fetch_one(pool)
        .await?;

        if existing > 0 {
            return Err(AuthError::UserAlreadyExists);
        }
        user.email = email.clone();
    }

    let now = Utc::now();
    user.updated_at = now;

    sqlx::query(
        "UPDATE users SET username = ?, email = ?, updated_at = ? WHERE id = ?"
    )
    .bind(&user.username)
    .bind(&user.email)
    .bind(now)
    .bind(user_id.to_string())
    .execute(pool)
    .await?;

    Ok(user.to_shared())
}


pub fn verify_jwt(token: &str, secret: &str) -> Result<Uuid, AuthError> {
    let token_data = decode::<Claims>(
        token,
        &DecodingKey::from_secret(secret.as_bytes()),
        &Validation::default(),
    )?;

    Uuid::parse_str(&token_data.claims.sub)
        .map_err(|_| AuthError::InvalidCredentials)
}

/// Hash a refresh token using SHA256
pub fn hash_refresh_token(token: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(token.as_bytes());
    format!("{:x}", hasher.finalize())
}

/// Generate a new refresh token (returns raw token and its hash)
pub fn generate_refresh_token() -> (String, String) {
    let token = Uuid::new_v4().to_string();
    let hash = hash_refresh_token(&token);
    (token, hash)
}

/// Create and store a refresh token in the database
pub async fn create_refresh_token(
    pool: &SqlitePool,
    user_id: &Uuid,
    expiration_days: i64,
) -> Result<String, AuthError> {
    let (token, hash) = generate_refresh_token();
    let id = Uuid::new_v4();
    let now = Utc::now();
    let expires_at = now + Duration::days(expiration_days);

    sqlx::query(
        r#"
        INSERT INTO refresh_tokens (id, user_id, token_hash, expires_at, created_at)
        VALUES (?, ?, ?, ?, ?)
        "#,
    )
    .bind(id.to_string())
    .bind(user_id.to_string())
    .bind(&hash)
    .bind(expires_at)
    .bind(now)
    .execute(pool)
    .await?;

    Ok(token)
}

/// Validate a refresh token and return the user_id if valid
/// Also rotates the token (deletes old, creates new)
pub async fn refresh_tokens(
    pool: &SqlitePool,
    refresh_token: &str,
    jwt_secret: &str,
    access_token_expiration_minutes: i64,
    refresh_token_expiration_days: i64,
) -> Result<(String, String, User), AuthError> {
    let token_hash = hash_refresh_token(refresh_token);
    let now = Utc::now();

    // Find the refresh token
    let token_row: Option<RefreshTokenRow> = sqlx::query_as(
        "SELECT * FROM refresh_tokens WHERE token_hash = ?",
    )
    .bind(&token_hash)
    .fetch_optional(pool)
    .await?;

    let token_row = token_row.ok_or(AuthError::InvalidRefreshToken)?;

    // Check if expired
    if token_row.expires_at < now {
        // Delete expired token
        sqlx::query("DELETE FROM refresh_tokens WHERE id = ?")
            .bind(&token_row.id)
            .execute(pool)
            .await?;
        return Err(AuthError::RefreshTokenExpired);
    }

    // Get the user
    let user_id = Uuid::parse_str(&token_row.user_id)
        .map_err(|_| AuthError::InvalidRefreshToken)?;

    let user = get_user_by_id(pool, &user_id)
        .await?
        .ok_or(AuthError::UserNotFound)?;

    // Delete the old refresh token (rotation)
    sqlx::query("DELETE FROM refresh_tokens WHERE id = ?")
        .bind(&token_row.id)
        .execute(pool)
        .await?;

    // Create new tokens
    let new_access_token = create_access_token(&user_id, jwt_secret, access_token_expiration_minutes)?;
    let new_refresh_token = create_refresh_token(pool, &user_id, refresh_token_expiration_days).await?;

    Ok((new_access_token, new_refresh_token, user))
}

/// Create an access token (short-lived JWT)
pub fn create_access_token(user_id: &Uuid, secret: &str, expiration_minutes: i64) -> Result<String, AuthError> {
    let now = Utc::now();
    let exp = now + Duration::minutes(expiration_minutes);

    let claims = Claims {
        sub: user_id.to_string(),
        exp: exp.timestamp(),
        iat: now.timestamp(),
    };

    let token = encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(secret.as_bytes()),
    )?;

    Ok(token)
}

/// Delete all refresh tokens for a user (used on logout)
#[allow(dead_code)]
pub async fn delete_user_refresh_tokens(pool: &SqlitePool, user_id: &Uuid) -> Result<(), AuthError> {
    sqlx::query("DELETE FROM refresh_tokens WHERE user_id = ?")
        .bind(user_id.to_string())
        .execute(pool)
        .await?;
    Ok(())
}

/// Delete a specific refresh token (used on logout with specific token)
pub async fn delete_refresh_token(pool: &SqlitePool, refresh_token: &str) -> Result<(), AuthError> {
    let token_hash = hash_refresh_token(refresh_token);
    sqlx::query("DELETE FROM refresh_tokens WHERE token_hash = ?")
        .bind(&token_hash)
        .execute(pool)
        .await?;
    Ok(())
}

/// Clean up expired refresh tokens (can be called periodically)
#[allow(dead_code)]
pub async fn cleanup_expired_refresh_tokens(pool: &SqlitePool) -> Result<u64, AuthError> {
    let now = Utc::now();
    let result = sqlx::query("DELETE FROM refresh_tokens WHERE expires_at < ?")
        .bind(now)
        .execute(pool)
        .await?;
    Ok(result.rows_affected())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_and_verify_jwt() {
        let user_id = Uuid::new_v4();
        let secret = "test-secret";

        let token = create_access_token(&user_id, secret, 15).unwrap();
        let verified_id = verify_jwt(&token, secret).unwrap();

        assert_eq!(user_id, verified_id);
    }

    #[test]
    fn test_verify_jwt_invalid_secret() {
        let user_id = Uuid::new_v4();
        let token = create_access_token(&user_id, "secret1", 15).unwrap();

        let result = verify_jwt(&token, "secret2");
        assert!(result.is_err());
    }

    #[test]
    fn test_password_hashing() {
        let password = "test_password123";
        let salt = SaltString::generate(&mut OsRng);
        let argon2 = Argon2::default();

        let hash = argon2.hash_password(password.as_bytes(), &salt).unwrap();
        let hash_string = hash.to_string();
        let parsed_hash = PasswordHash::new(&hash_string).unwrap();

        assert!(argon2.verify_password(password.as_bytes(), &parsed_hash).is_ok());
        assert!(argon2.verify_password(b"wrong_password", &parsed_hash).is_err());
    }

    #[test]
    fn test_hash_refresh_token() {
        let token = "test-token-123";
        let hash1 = hash_refresh_token(token);
        let hash2 = hash_refresh_token(token);

        // Same token should produce same hash
        assert_eq!(hash1, hash2);

        // Different tokens should produce different hashes
        let hash3 = hash_refresh_token("different-token");
        assert_ne!(hash1, hash3);
    }

    #[test]
    fn test_generate_refresh_token() {
        let (token1, hash1) = generate_refresh_token();
        let (token2, hash2) = generate_refresh_token();

        // Each call should generate unique tokens
        assert_ne!(token1, token2);
        assert_ne!(hash1, hash2);

        // Hash should match the token
        assert_eq!(hash_refresh_token(&token1), hash1);
        assert_eq!(hash_refresh_token(&token2), hash2);
    }

    // Helper function to set up a test database
    async fn setup_test_db() -> SqlitePool {
        let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();

        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS users (
                id TEXT PRIMARY KEY NOT NULL,
                username TEXT NOT NULL UNIQUE,
                email TEXT NOT NULL UNIQUE,
                password_hash TEXT,
                oidc_subject TEXT,
                oidc_provider TEXT,
                created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
                updated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
            )
            "#,
        )
        .execute(&pool)
        .await
        .unwrap();

        pool
    }

    // Helper function to create a test user with a known password
    async fn create_test_user_with_password(
        pool: &SqlitePool,
        username: &str,
        email: &str,
        password: &str,
    ) -> Uuid {
        let user_id = Uuid::new_v4();
        let salt = SaltString::generate(&mut OsRng);
        let argon2 = Argon2::default();
        let password_hash = argon2
            .hash_password(password.as_bytes(), &salt)
            .unwrap()
            .to_string();

        sqlx::query(
            "INSERT INTO users (id, username, email, password_hash) VALUES (?, ?, ?, ?)",
        )
        .bind(user_id.to_string())
        .bind(username)
        .bind(email)
        .bind(&password_hash)
        .execute(pool)
        .await
        .unwrap();

        user_id
    }

    #[tokio::test]
    async fn test_login_with_username() {
        let pool = setup_test_db().await;
        create_test_user_with_password(&pool, "testuser", "test@example.com", "password123").await;

        let request = shared::LoginRequest {
            username: "testuser".to_string(),
            password: "password123".to_string(),
        };

        let result = login_user(&pool, &request).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap().username, "testuser");
    }

    #[tokio::test]
    async fn test_login_with_email() {
        let pool = setup_test_db().await;
        create_test_user_with_password(&pool, "testuser", "test@example.com", "password123").await;

        let request = shared::LoginRequest {
            username: "test@example.com".to_string(),
            password: "password123".to_string(),
        };

        let result = login_user(&pool, &request).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap().username, "testuser");
    }

    #[tokio::test]
    async fn test_login_case_insensitive_username() {
        let pool = setup_test_db().await;
        create_test_user_with_password(&pool, "TestUser", "test@example.com", "password123").await;

        let request = shared::LoginRequest {
            username: "testuser".to_string(), // lowercase
            password: "password123".to_string(),
        };

        let result = login_user(&pool, &request).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap().username, "TestUser");
    }

    #[tokio::test]
    async fn test_login_case_insensitive_email() {
        let pool = setup_test_db().await;
        create_test_user_with_password(&pool, "testuser", "Test@Example.com", "password123").await;

        let request = shared::LoginRequest {
            username: "test@example.com".to_string(), // lowercase
            password: "password123".to_string(),
        };

        let result = login_user(&pool, &request).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap().username, "testuser");
    }
}
