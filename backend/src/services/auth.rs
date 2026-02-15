use argon2::{
    password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
    Argon2,
};
use rand_core::OsRng;
use chrono::{Duration, Utc};
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;
use thiserror::Error;
use uuid::Uuid;

use crate::models::UserRow;
use shared::{CreateUserRequest, UpdateUserRequest, User};

#[derive(Debug, Error)]
pub enum AuthError {
    #[error("User already exists")]
    UserAlreadyExists,
    #[error("Invalid credentials")]
    InvalidCredentials,
    #[error("User not found")]
    UserNotFound,
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
    let user: UserRow = sqlx::query_as(
        "SELECT * FROM users WHERE username = ?"
    )
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

pub async fn get_user_by_email(pool: &SqlitePool, email: &str) -> Result<Option<User>, AuthError> {
    let user: Option<UserRow> = sqlx::query_as(
        "SELECT * FROM users WHERE email = ?"
    )
    .bind(email)
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

pub fn create_jwt(user_id: &Uuid, secret: &str, expiration_hours: i64) -> Result<String, AuthError> {
    let now = Utc::now();
    let exp = now + Duration::hours(expiration_hours);

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

pub fn verify_jwt(token: &str, secret: &str) -> Result<Uuid, AuthError> {
    let token_data = decode::<Claims>(
        token,
        &DecodingKey::from_secret(secret.as_bytes()),
        &Validation::default(),
    )?;

    Uuid::parse_str(&token_data.claims.sub)
        .map_err(|_| AuthError::InvalidCredentials)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_and_verify_jwt() {
        let user_id = Uuid::new_v4();
        let secret = "test-secret";

        let token = create_jwt(&user_id, secret, 24).unwrap();
        let verified_id = verify_jwt(&token, secret).unwrap();

        assert_eq!(user_id, verified_id);
    }

    #[test]
    fn test_verify_jwt_invalid_secret() {
        let user_id = Uuid::new_v4();
        let token = create_jwt(&user_id, "secret1", 24).unwrap();

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
}
