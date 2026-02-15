use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

/// Database model for households
#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct HouseholdRow {
    pub id: String,
    pub name: String,
    pub owner_id: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl HouseholdRow {
    pub fn to_shared(&self) -> shared::Household {
        shared::Household {
            id: Uuid::parse_str(&self.id).unwrap(),
            name: self.name.clone(),
            owner_id: Uuid::parse_str(&self.owner_id).unwrap(),
            created_at: self.created_at,
            updated_at: self.updated_at,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_household_row_to_shared() {
        let now = Utc::now();
        let id = Uuid::new_v4();
        let owner_id = Uuid::new_v4();

        let row = HouseholdRow {
            id: id.to_string(),
            name: "Test Household".to_string(),
            owner_id: owner_id.to_string(),
            created_at: now,
            updated_at: now,
        };

        let shared = row.to_shared();

        assert_eq!(shared.id, id);
        assert_eq!(shared.name, "Test Household");
        assert_eq!(shared.owner_id, owner_id);
    }
}
