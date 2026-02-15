use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

/// Database model for household invitations
#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct InvitationRow {
    pub id: String,
    pub household_id: String,
    pub email: String,
    pub role: String,
    pub invited_by: String,
    pub status: String,
    pub created_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
    pub responded_at: Option<DateTime<Utc>>,
}

impl InvitationRow {
    pub fn to_shared(&self) -> shared::Invitation {
        shared::Invitation {
            id: Uuid::parse_str(&self.id).unwrap(),
            household_id: Uuid::parse_str(&self.household_id).unwrap(),
            email: self.email.clone(),
            role: shared::Role::from_str(&self.role).unwrap_or(shared::Role::Member),
            invited_by: Uuid::parse_str(&self.invited_by).unwrap(),
            status: shared::InvitationStatus::from_str(&self.status)
                .unwrap_or(shared::InvitationStatus::Pending),
            created_at: self.created_at,
            expires_at: self.expires_at,
            responded_at: self.responded_at,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use shared::{InvitationStatus, Role};

    #[test]
    fn test_invitation_row_to_shared() {
        let now = Utc::now();
        let expires = now + chrono::Duration::days(7);
        let id = Uuid::new_v4();
        let household_id = Uuid::new_v4();
        let invited_by = Uuid::new_v4();

        let row = InvitationRow {
            id: id.to_string(),
            household_id: household_id.to_string(),
            email: "test@example.com".to_string(),
            role: "admin".to_string(),
            invited_by: invited_by.to_string(),
            status: "pending".to_string(),
            created_at: now,
            expires_at: expires,
            responded_at: None,
        };

        let shared = row.to_shared();

        assert_eq!(shared.id, id);
        assert_eq!(shared.household_id, household_id);
        assert_eq!(shared.email, "test@example.com");
        assert_eq!(shared.role, Role::Admin);
        assert_eq!(shared.invited_by, invited_by);
        assert_eq!(shared.status, InvitationStatus::Pending);
        assert!(shared.responded_at.is_none());
    }

    #[test]
    fn test_invitation_row_invalid_status_defaults_to_pending() {
        let now = Utc::now();
        let expires = now + chrono::Duration::days(7);

        let row = InvitationRow {
            id: Uuid::new_v4().to_string(),
            household_id: Uuid::new_v4().to_string(),
            email: "test@example.com".to_string(),
            role: "member".to_string(),
            invited_by: Uuid::new_v4().to_string(),
            status: "invalid_status".to_string(),
            created_at: now,
            expires_at: expires,
            responded_at: None,
        };

        let shared = row.to_shared();
        assert_eq!(shared.status, InvitationStatus::Pending);
    }

    #[test]
    fn test_invitation_row_with_responded_at() {
        let now = Utc::now();
        let expires = now + chrono::Duration::days(7);
        let responded = now + chrono::Duration::hours(1);

        let row = InvitationRow {
            id: Uuid::new_v4().to_string(),
            household_id: Uuid::new_v4().to_string(),
            email: "test@example.com".to_string(),
            role: "member".to_string(),
            invited_by: Uuid::new_v4().to_string(),
            status: "accepted".to_string(),
            created_at: now,
            expires_at: expires,
            responded_at: Some(responded),
        };

        let shared = row.to_shared();
        assert_eq!(shared.status, InvitationStatus::Accepted);
        assert!(shared.responded_at.is_some());
    }
}
