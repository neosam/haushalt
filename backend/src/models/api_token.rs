use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

/// A row of `api_tokens`.
///
/// Ids are stored as strings, as everywhere in this schema. `token_hash` is the SHA-256 of
/// the secret; the plaintext is never stored. Conversion to the shared [`shared::ApiToken`]
/// deliberately drops `token_hash` — the shared type is what reaches the client, and it must
/// never carry anything secret.
#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct ApiTokenRow {
    pub id: String,
    pub household_id: String,
    pub user_id: String,
    pub name: String,
    pub token_hash: String,
    pub token_prefix: String,
    pub can_write: bool,
    pub enabled: bool,
    pub created_at: DateTime<Utc>,
    pub last_used_at: Option<DateTime<Utc>>,
}

impl ApiTokenRow {
    /// Convert to the shared type. Returns `None` when a stored id is not a valid UUID —
    /// unreachable through the API, which only ever writes generated UUIDs, but a corrupt
    /// row must not panic the token-management endpoints.
    pub fn to_shared(&self) -> Option<shared::ApiToken> {
        Some(shared::ApiToken {
            id: Uuid::parse_str(&self.id).ok()?,
            household_id: Uuid::parse_str(&self.household_id).ok()?,
            user_id: Uuid::parse_str(&self.user_id).ok()?,
            name: self.name.clone(),
            token_prefix: self.token_prefix.clone(),
            can_write: self.can_write,
            enabled: self.enabled,
            created_at: self.created_at,
            last_used_at: self.last_used_at,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn row(id: &str) -> ApiTokenRow {
        ApiTokenRow {
            id: id.to_string(),
            household_id: Uuid::new_v4().to_string(),
            user_id: Uuid::new_v4().to_string(),
            name: "CI pipeline".to_string(),
            token_hash: "deadbeef".to_string(),
            token_prefix: "hht_1a2b3c4d".to_string(),
            can_write: true,
            enabled: true,
            created_at: Utc::now(),
            last_used_at: None,
        }
    }

    #[test]
    fn test_to_shared_never_exposes_the_hash() {
        let shared = row(&Uuid::new_v4().to_string()).to_shared().unwrap();
        // The shared type has no hash field at all; this asserts the prefix is what survives.
        assert_eq!(shared.token_prefix, "hht_1a2b3c4d");
        assert!(shared.can_write);
        assert!(shared.enabled);
        assert_eq!(shared.last_used_at, None);
    }

    #[test]
    fn test_to_shared_returns_none_on_malformed_uuid() {
        assert!(row("not-a-uuid").to_shared().is_none());
    }
}
