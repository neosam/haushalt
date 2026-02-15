use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct HouseholdSettingsRow {
    pub household_id: String,
    pub dark_mode: bool,
    pub role_label_owner: String,
    pub role_label_admin: String,
    pub role_label_member: String,
    pub updated_at: DateTime<Utc>,
}

impl HouseholdSettingsRow {
    pub fn to_shared(&self) -> shared::HouseholdSettings {
        shared::HouseholdSettings {
            household_id: Uuid::parse_str(&self.household_id).unwrap(),
            dark_mode: self.dark_mode,
            role_label_owner: self.role_label_owner.clone(),
            role_label_admin: self.role_label_admin.clone(),
            role_label_member: self.role_label_member.clone(),
            updated_at: self.updated_at,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_household_settings_row_to_shared() {
        let now = Utc::now();
        let household_id = Uuid::new_v4();

        let row = HouseholdSettingsRow {
            household_id: household_id.to_string(),
            dark_mode: true,
            role_label_owner: "Parent".to_string(),
            role_label_admin: "Guardian".to_string(),
            role_label_member: "Child".to_string(),
            updated_at: now,
        };

        let shared = row.to_shared();

        assert_eq!(shared.household_id, household_id);
        assert!(shared.dark_mode);
        assert_eq!(shared.role_label_owner, "Parent");
        assert_eq!(shared.role_label_admin, "Guardian");
        assert_eq!(shared.role_label_member, "Child");
    }
}
