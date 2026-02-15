use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

/// Database model for household memberships
#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct MembershipRow {
    pub id: String,
    pub household_id: String,
    pub user_id: String,
    pub role: String,
    pub points: i64,
    pub joined_at: DateTime<Utc>,
}

impl MembershipRow {
    pub fn to_shared(&self) -> shared::HouseholdMembership {
        shared::HouseholdMembership {
            id: Uuid::parse_str(&self.id).unwrap(),
            household_id: Uuid::parse_str(&self.household_id).unwrap(),
            user_id: Uuid::parse_str(&self.user_id).unwrap(),
            role: shared::Role::from_str(&self.role).unwrap_or(shared::Role::Member),
            points: self.points,
            joined_at: self.joined_at,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use shared::Role;

    #[test]
    fn test_membership_row_to_shared() {
        let now = Utc::now();
        let id = Uuid::new_v4();
        let household_id = Uuid::new_v4();
        let user_id = Uuid::new_v4();

        let row = MembershipRow {
            id: id.to_string(),
            household_id: household_id.to_string(),
            user_id: user_id.to_string(),
            role: "admin".to_string(),
            points: 100,
            joined_at: now,
        };

        let shared = row.to_shared();

        assert_eq!(shared.id, id);
        assert_eq!(shared.household_id, household_id);
        assert_eq!(shared.user_id, user_id);
        assert_eq!(shared.role, Role::Admin);
        assert_eq!(shared.points, 100);
    }

    #[test]
    fn test_membership_row_invalid_role_defaults_to_member() {
        let now = Utc::now();

        let row = MembershipRow {
            id: Uuid::new_v4().to_string(),
            household_id: Uuid::new_v4().to_string(),
            user_id: Uuid::new_v4().to_string(),
            role: "invalid_role".to_string(),
            points: 0,
            joined_at: now,
        };

        let shared = row.to_shared();
        assert_eq!(shared.role, Role::Member);
    }
}
