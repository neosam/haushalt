//! Programmatic API access tokens.
//!
//! A household member mints a token so an external system can call the API on their behalf.
//! Every token is bound to exactly one household and one permission level (read, or
//! read+write). Authentication resolves a presented secret to its creator; the actix
//! middleware in [`crate::middleware::api_token`] then lets the request proceed AS that
//! creator, so every existing handler's membership and role checks apply unchanged.
//!
//! The secret is a high-entropy string that is shown to its owner exactly once, at creation.
//! Only its SHA-256 hash is stored, mirroring how refresh tokens are handled — a database
//! leak never yields a usable credential.
//!
//! All logic lives here so it is unit-testable; the handlers stay thin (D-19).

use chrono::Utc;
use sha2::{Digest, Sha256};
use sqlx::SqlitePool;
use uuid::Uuid;

use shared::{ApiToken, CreateApiTokenRequest, CreatedApiToken, UpdateApiTokenRequest};

use crate::models::ApiTokenRow;
use crate::services::households;

/// Every generated secret starts with this, so the auth middleware can tell an API token
/// apart from a JWT at a glance before doing any database work.
pub const API_TOKEN_PREFIX: &str = "hht_";

/// How many leading characters of the secret are kept for display (`"hht_" + 8 hex`). Long
/// enough to distinguish tokens in a list, short enough to reveal nothing usable.
const PREFIX_DISPLAY_LEN: usize = 12;

/// A token name has to fit a settings list; anything longer is a paste accident.
const MAX_NAME_LENGTH: usize = 100;

/// A single member cannot mint an unbounded number of standing credentials. Far above any
/// real use, low enough to stop a runaway loop.
const MAX_TOKENS_PER_USER: usize = 50;

#[derive(Debug, thiserror::Error)]
pub enum ApiTokenError {
    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),
    #[error("Household error: {0}")]
    Household(#[from] households::HouseholdError),
    #[error("Token not found")]
    NotFound,
    #[error("Token name must not be empty and at most {MAX_NAME_LENGTH} characters")]
    InvalidName,
    #[error("Not a member of household {0}")]
    NotAMemberOfHousehold(Uuid),
    #[error("A user may not have more than {MAX_TOKENS_PER_USER} tokens")]
    TooManyTokens,
    /// A stored id that is not a valid UUID. Unreachable through the API, which only ever
    /// writes generated UUIDs, but neither the auth path nor the listing may panic on it.
    #[error("Stored token is corrupt")]
    CorruptRow,
}

/// What authenticating a token yields: enough to let the request continue as its creator,
/// gated by the household it may touch and whether it may write.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ApiTokenAuth {
    pub user_id: Uuid,
    pub household_id: Uuid,
    pub can_write: bool,
}

// ============================================================================
// Secret generation and hashing
// ============================================================================

/// SHA-256 of a secret, as lowercase hex — the same construction refresh tokens use.
pub fn hash_token(secret: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(secret.as_bytes());
    format!("{:x}", hasher.finalize())
}

/// A fresh secret: the prefix plus 244 bits of randomness from two v4 UUIDs. Unguessable,
/// and it needs no new dependency — the same `uuid` crate every id already uses.
fn generate_secret() -> String {
    format!(
        "{API_TOKEN_PREFIX}{}{}",
        Uuid::new_v4().simple(),
        Uuid::new_v4().simple()
    )
}

/// The non-secret display prefix of a secret (`"hht_1a2b3c4d"`).
fn display_prefix(secret: &str) -> String {
    secret.chars().take(PREFIX_DISPLAY_LEN).collect()
}

fn validate_name(name: &str) -> Result<String, ApiTokenError> {
    let trimmed = name.trim();
    if trimmed.is_empty() || trimmed.chars().count() > MAX_NAME_LENGTH {
        return Err(ApiTokenError::InvalidName);
    }
    Ok(trimmed.to_string())
}

// ============================================================================
// Owner-scoped CRUD (the /users/me/api-tokens surface)
// ============================================================================

/// List the tokens the given user created, newest first. Secrets are never included.
pub async fn list_tokens(pool: &SqlitePool, user_id: &Uuid) -> Result<Vec<ApiToken>, ApiTokenError> {
    let rows: Vec<ApiTokenRow> = sqlx::query_as(
        "SELECT * FROM api_tokens WHERE user_id = ? ORDER BY created_at DESC",
    )
    .bind(user_id.to_string())
    .fetch_all(pool)
    .await?;

    // A corrupt row (non-UUID id) is dropped from the list rather than failing the whole
    // request — the same tolerance the reports listing takes.
    Ok(rows.iter().filter_map(ApiTokenRow::to_shared).collect())
}

/// Fetch one of the user's own tokens. `NotFound` covers both "does not exist" and "belongs
/// to somebody else", so ownership is never revealed.
pub async fn get_token(
    pool: &SqlitePool,
    token_id: &Uuid,
    user_id: &Uuid,
) -> Result<ApiToken, ApiTokenError> {
    load_owned_row(pool, token_id, user_id)
        .await?
        .to_shared()
        .ok_or(ApiTokenError::CorruptRow)
}

async fn load_owned_row(
    pool: &SqlitePool,
    token_id: &Uuid,
    user_id: &Uuid,
) -> Result<ApiTokenRow, ApiTokenError> {
    sqlx::query_as::<_, ApiTokenRow>("SELECT * FROM api_tokens WHERE id = ? AND user_id = ?")
        .bind(token_id.to_string())
        .bind(user_id.to_string())
        .fetch_optional(pool)
        .await?
        .ok_or(ApiTokenError::NotFound)
}

/// Mint a token for `user_id` in the requested household and return the plaintext ONCE.
pub async fn create_token(
    pool: &SqlitePool,
    user_id: &Uuid,
    request: CreateApiTokenRequest,
) -> Result<CreatedApiToken, ApiTokenError> {
    let name = validate_name(&request.name)?;
    let can_write = request.can_write.unwrap_or(false);

    // A token may only be bound to a household the creator actually belongs to — checked at
    // write time so an unauthorized id never reaches storage.
    if !households::is_member(pool, &request.household_id, user_id).await? {
        return Err(ApiTokenError::NotAMemberOfHousehold(request.household_id));
    }

    let count = sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM api_tokens WHERE user_id = ?")
        .bind(user_id.to_string())
        .fetch_one(pool)
        .await?;
    if count as usize >= MAX_TOKENS_PER_USER {
        return Err(ApiTokenError::TooManyTokens);
    }

    let secret = generate_secret();
    let token_hash = hash_token(&secret);
    let token_prefix = display_prefix(&secret);
    let id = Uuid::new_v4();
    let now = Utc::now();

    sqlx::query(
        r#"
        INSERT INTO api_tokens
            (id, household_id, user_id, name, token_hash, token_prefix, can_write, enabled, created_at, last_used_at)
        VALUES (?, ?, ?, ?, ?, ?, ?, 1, ?, NULL)
        "#,
    )
    .bind(id.to_string())
    .bind(request.household_id.to_string())
    .bind(user_id.to_string())
    .bind(&name)
    .bind(&token_hash)
    .bind(&token_prefix)
    .bind(can_write)
    .bind(now)
    .execute(pool)
    .await?;

    let token = ApiToken {
        id,
        household_id: request.household_id,
        user_id: *user_id,
        name,
        token_prefix,
        can_write,
        enabled: true,
        created_at: now,
        last_used_at: None,
    };
    Ok(CreatedApiToken { token, secret })
}

/// Update a token's name, enabled flag, and/or write permission. Absent fields are left
/// unchanged. The household binding and the secret are immutable — regenerate for a new
/// secret, delete-and-recreate for a different household.
pub async fn update_token(
    pool: &SqlitePool,
    token_id: &Uuid,
    user_id: &Uuid,
    request: UpdateApiTokenRequest,
) -> Result<ApiToken, ApiTokenError> {
    let mut row = load_owned_row(pool, token_id, user_id).await?;

    if let Some(name) = request.name {
        row.name = validate_name(&name)?;
    }
    if let Some(enabled) = request.enabled {
        row.enabled = enabled;
    }
    if let Some(can_write) = request.can_write {
        row.can_write = can_write;
    }

    sqlx::query("UPDATE api_tokens SET name = ?, enabled = ?, can_write = ? WHERE id = ? AND user_id = ?")
        .bind(&row.name)
        .bind(row.enabled)
        .bind(row.can_write)
        .bind(token_id.to_string())
        .bind(user_id.to_string())
        .execute(pool)
        .await?;

    row.to_shared().ok_or(ApiTokenError::CorruptRow)
}

/// Permanently delete one of the user's tokens. Its secret stops working immediately.
pub async fn delete_token(
    pool: &SqlitePool,
    token_id: &Uuid,
    user_id: &Uuid,
) -> Result<(), ApiTokenError> {
    let result = sqlx::query("DELETE FROM api_tokens WHERE id = ? AND user_id = ?")
        .bind(token_id.to_string())
        .bind(user_id.to_string())
        .execute(pool)
        .await?;

    if result.rows_affected() == 0 {
        return Err(ApiTokenError::NotFound);
    }
    Ok(())
}

// ============================================================================
// Authentication (the runtime path, called by the auth middleware)
// ============================================================================

/// Resolve a presented secret to its principal, or fail.
///
/// Only enabled tokens authenticate; a disabled token is indistinguishable from a
/// non-existent one (`NotFound`). On success the token's `last_used_at` is stamped
/// best-effort — a failure to record it must not fail the request.
pub async fn authenticate(pool: &SqlitePool, presented: &str) -> Result<ApiTokenAuth, ApiTokenError> {
    let token_hash = hash_token(presented);

    let row: ApiTokenRow =
        sqlx::query_as("SELECT * FROM api_tokens WHERE token_hash = ? AND enabled = 1")
            .bind(&token_hash)
            .fetch_optional(pool)
            .await?
            .ok_or(ApiTokenError::NotFound)?;

    let user_id = Uuid::parse_str(&row.user_id).map_err(|_| ApiTokenError::CorruptRow)?;
    let household_id = Uuid::parse_str(&row.household_id).map_err(|_| ApiTokenError::CorruptRow)?;

    // Best-effort: the request has already been authenticated, so a failed timestamp write
    // is logged and swallowed rather than surfaced.
    if let Err(e) = sqlx::query("UPDATE api_tokens SET last_used_at = ? WHERE id = ?")
        .bind(Utc::now())
        .bind(&row.id)
        .execute(pool)
        .await
    {
        log::warn!("Failed to stamp api_token.last_used_at for {}: {e:?}", row.id);
    }

    Ok(ApiTokenAuth {
        user_id,
        household_id,
        can_write: row.can_write,
    })
}

/// The most recent `last_used_at` for a token — a test/inspection helper.
#[cfg(test)]
pub async fn last_used_at(pool: &SqlitePool, token_id: &Uuid) -> Option<chrono::DateTime<Utc>> {
    sqlx::query_scalar::<_, Option<chrono::DateTime<Utc>>>(
        "SELECT last_used_at FROM api_tokens WHERE id = ?",
    )
    .bind(token_id.to_string())
    .fetch_one(pool)
    .await
    .ok()
    .flatten()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::*;
    use shared::Role;

    async fn member_of_household(pool: &SqlitePool) -> (Uuid, Uuid) {
        let household_id = create_test_household(pool).await;
        let user_id = create_test_user(pool, "member@test.com", Role::Member).await;
        create_test_membership(pool, &household_id, &user_id, Role::Member).await;
        (household_id, user_id)
    }

    fn create_request(household_id: Uuid, can_write: bool) -> CreateApiTokenRequest {
        CreateApiTokenRequest {
            household_id,
            name: "CI".to_string(),
            can_write: Some(can_write),
        }
    }

    #[test]
    fn test_generated_secret_is_prefixed_and_unguessable() {
        let a = generate_secret();
        let b = generate_secret();
        assert!(a.starts_with(API_TOKEN_PREFIX), "got: {a}");
        assert_ne!(a, b);
        // "hht_" + 64 hex from two simple UUIDs.
        assert_eq!(a.len(), API_TOKEN_PREFIX.len() + 64);
        assert_eq!(display_prefix(&a), a[..PREFIX_DISPLAY_LEN]);
    }

    #[test]
    fn test_hash_is_stable_and_distinct() {
        assert_eq!(hash_token("hht_abc"), hash_token("hht_abc"));
        assert_ne!(hash_token("hht_abc"), hash_token("hht_abd"));
        // Hex SHA-256 is 64 chars, and the plaintext never appears in it.
        let h = hash_token("hht_secret");
        assert_eq!(h.len(), 64);
        assert!(!h.contains("secret"));
    }

    #[tokio::test]
    async fn test_create_returns_plaintext_once_and_stores_only_the_hash() {
        let pool = create_test_pool().await;
        let (household_id, user_id) = member_of_household(&pool).await;

        let created = create_token(&pool, &user_id, create_request(household_id, true))
            .await
            .unwrap();

        assert!(created.secret.starts_with(API_TOKEN_PREFIX));
        assert_eq!(created.token.token_prefix, created.secret[..PREFIX_DISPLAY_LEN]);
        assert!(created.token.can_write);

        // The database holds the hash, not the secret.
        let stored_hash: String =
            sqlx::query_scalar("SELECT token_hash FROM api_tokens WHERE id = ?")
                .bind(created.token.id.to_string())
                .fetch_one(&pool)
                .await
                .unwrap();
        assert_eq!(stored_hash, hash_token(&created.secret));
        assert_ne!(stored_hash, created.secret);
    }

    #[tokio::test]
    async fn test_create_rejects_non_member_household() {
        let pool = create_test_pool().await;
        let (_household_id, user_id) = member_of_household(&pool).await;
        let foreign = create_test_household_with_name(&pool, "Not mine").await;

        let result = create_token(&pool, &user_id, create_request(foreign, false)).await;
        assert!(matches!(
            result,
            Err(ApiTokenError::NotAMemberOfHousehold(id)) if id == foreign
        ));
    }

    #[tokio::test]
    async fn test_create_rejects_blank_name() {
        let pool = create_test_pool().await;
        let (household_id, user_id) = member_of_household(&pool).await;
        let result = create_token(
            &pool,
            &user_id,
            CreateApiTokenRequest {
                household_id,
                name: "   ".to_string(),
                can_write: None,
            },
        )
        .await;
        assert!(matches!(result, Err(ApiTokenError::InvalidName)));
    }

    #[tokio::test]
    async fn test_create_defaults_to_read_only() {
        let pool = create_test_pool().await;
        let (household_id, user_id) = member_of_household(&pool).await;
        let created = create_token(
            &pool,
            &user_id,
            CreateApiTokenRequest {
                household_id,
                name: "Reader".to_string(),
                can_write: None,
            },
        )
        .await
        .unwrap();
        assert!(!created.token.can_write);
    }

    #[tokio::test]
    async fn test_authenticate_resolves_creator_household_and_permission() {
        let pool = create_test_pool().await;
        let (household_id, user_id) = member_of_household(&pool).await;
        let created = create_token(&pool, &user_id, create_request(household_id, true))
            .await
            .unwrap();

        let auth = authenticate(&pool, &created.secret).await.unwrap();
        assert_eq!(
            auth,
            ApiTokenAuth {
                user_id,
                household_id,
                can_write: true,
            }
        );
        // The successful authentication stamped last_used_at.
        assert!(last_used_at(&pool, &created.token.id).await.is_some());
    }

    #[tokio::test]
    async fn test_authenticate_rejects_unknown_and_disabled_tokens() {
        let pool = create_test_pool().await;
        let (household_id, user_id) = member_of_household(&pool).await;
        let created = create_token(&pool, &user_id, create_request(household_id, false))
            .await
            .unwrap();

        assert!(matches!(
            authenticate(&pool, "hht_does_not_exist").await,
            Err(ApiTokenError::NotFound)
        ));

        // A disabled token authenticates like a non-existent one.
        update_token(
            &pool,
            &created.token.id,
            &user_id,
            UpdateApiTokenRequest {
                enabled: Some(false),
                ..Default::default()
            },
        )
        .await
        .unwrap();
        assert!(matches!(
            authenticate(&pool, &created.secret).await,
            Err(ApiTokenError::NotFound)
        ));
    }

    #[tokio::test]
    async fn test_list_is_owner_scoped_and_hides_others() {
        let pool = create_test_pool().await;
        let (household_id, user_id) = member_of_household(&pool).await;
        let other = create_test_user(&pool, "other@test.com", Role::Member).await;
        create_test_membership(&pool, &household_id, &other, Role::Member).await;

        create_token(&pool, &user_id, create_request(household_id, false))
            .await
            .unwrap();
        create_token(&pool, &other, create_request(household_id, false))
            .await
            .unwrap();

        let mine = list_tokens(&pool, &user_id).await.unwrap();
        assert_eq!(mine.len(), 1);
        assert_eq!(mine[0].user_id, user_id);
    }

    #[tokio::test]
    async fn test_get_update_delete_are_owner_scoped() {
        let pool = create_test_pool().await;
        let (household_id, user_id) = member_of_household(&pool).await;
        let stranger = create_test_user(&pool, "stranger@test.com", Role::Member).await;
        let created = create_token(&pool, &user_id, create_request(household_id, false))
            .await
            .unwrap();

        // A stranger sees NotFound for all three, learning nothing about ownership.
        assert!(matches!(
            get_token(&pool, &created.token.id, &stranger).await,
            Err(ApiTokenError::NotFound)
        ));
        assert!(matches!(
            update_token(&pool, &created.token.id, &stranger, UpdateApiTokenRequest::default()).await,
            Err(ApiTokenError::NotFound)
        ));
        assert!(matches!(
            delete_token(&pool, &created.token.id, &stranger).await,
            Err(ApiTokenError::NotFound)
        ));

        // The owner can toggle write permission and then delete it.
        let updated = update_token(
            &pool,
            &created.token.id,
            &user_id,
            UpdateApiTokenRequest {
                can_write: Some(true),
                ..Default::default()
            },
        )
        .await
        .unwrap();
        assert!(updated.can_write);

        delete_token(&pool, &created.token.id, &user_id).await.unwrap();
        assert!(matches!(
            get_token(&pool, &created.token.id, &user_id).await,
            Err(ApiTokenError::NotFound)
        ));
    }

    #[tokio::test]
    async fn test_token_limit_is_enforced() {
        let pool = create_test_pool().await;
        let (household_id, user_id) = member_of_household(&pool).await;

        for _ in 0..MAX_TOKENS_PER_USER {
            create_token(&pool, &user_id, create_request(household_id, false))
                .await
                .unwrap();
        }
        assert!(matches!(
            create_token(&pool, &user_id, create_request(household_id, false)).await,
            Err(ApiTokenError::TooManyTokens)
        ));
    }
}
