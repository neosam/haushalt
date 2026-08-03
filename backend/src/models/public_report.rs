use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

/// A row of `public_reports` (Phase 6).
///
/// The household selection lives in the `public_report_households` junction table, so
/// converting to the shared type needs it passed in — see [`PublicReportRow::to_shared`].
#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct PublicReportRow {
    pub id: String,
    pub user_id: String,
    pub name: String,
    pub token: String,
    pub language: String,
    pub enabled: bool,
    pub include_missed: bool,
    pub separate_undated: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl PublicReportRow {
    /// Convert to the shared type, attaching the household selection read separately.
    ///
    /// Returns `None` when a stored id or token is not a valid UUID. Every sibling `*Row`
    /// unwraps here instead, but this type is reached through an unauthenticated endpoint,
    /// where a panic is a denial of service rather than a stack trace nobody sees.
    pub fn to_shared(&self, household_ids: Vec<Uuid>) -> Option<shared::PublicReport> {
        Some(shared::PublicReport {
            id: Uuid::parse_str(&self.id).ok()?,
            user_id: Uuid::parse_str(&self.user_id).ok()?,
            name: self.name.clone(),
            token: Uuid::parse_str(&self.token).ok()?,
            language: self.language.clone(),
            enabled: self.enabled,
            include_missed: self.include_missed,
            separate_undated: self.separate_undated,
            household_ids,
            created_at: self.created_at,
            updated_at: self.updated_at,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn row(id: &str, token: &str) -> PublicReportRow {
        PublicReportRow {
            id: id.to_string(),
            user_id: Uuid::new_v4().to_string(),
            name: "Alle Haushalte".to_string(),
            token: token.to_string(),
            language: "de".to_string(),
            enabled: true,
            include_missed: true,
            separate_undated: false,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }

    #[test]
    fn test_to_shared_carries_household_selection() {
        let id = Uuid::new_v4();
        let token = Uuid::new_v4();
        let households = vec![Uuid::new_v4(), Uuid::new_v4()];

        let shared = row(&id.to_string(), &token.to_string())
            .to_shared(households.clone())
            .expect("valid uuids convert");

        assert_eq!(shared.id, id);
        assert_eq!(shared.token, token);
        assert_eq!(shared.language, "de");
        assert!(shared.enabled);
        assert_eq!(shared.household_ids, households);
    }

    #[test]
    fn test_to_shared_returns_none_on_malformed_uuid() {
        assert!(row("not-a-uuid", &Uuid::new_v4().to_string())
            .to_shared(Vec::new())
            .is_none());
        assert!(row(&Uuid::new_v4().to_string(), "not-a-uuid")
            .to_shared(Vec::new())
            .is_none());
    }

    #[test]
    fn test_public_path_uses_the_token() {
        let token = Uuid::new_v4();
        let shared = row(&Uuid::new_v4().to_string(), &token.to_string())
            .to_shared(Vec::new())
            .unwrap();

        assert_eq!(shared.public_path(), format!("/api/public/reports/{token}"));
    }
}
