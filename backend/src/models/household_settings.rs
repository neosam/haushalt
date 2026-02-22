use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use shared::HierarchyType;
use sqlx::FromRow;
use std::str::FromStr;
use uuid::Uuid;

#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct HouseholdSettingsRow {
    pub household_id: String,
    pub dark_mode: bool,
    pub role_label_owner: String,
    pub role_label_admin: String,
    pub role_label_member: String,
    pub hierarchy_type: String,
    pub timezone: String,
    pub rewards_enabled: bool,
    pub punishments_enabled: bool,
    pub chat_enabled: bool,
    pub vacation_mode: bool,
    pub vacation_start: Option<NaiveDate>,
    pub vacation_end: Option<NaiveDate>,
    pub auto_archive_days: Option<i32>,
    pub allow_task_suggestions: bool,
    pub week_start_day: i32,
    pub default_points_reward: Option<i64>,
    pub default_points_penalty: Option<i64>,
    pub solo_mode: bool,
    pub solo_mode_exit_requested_at: Option<DateTime<Utc>>,
    pub solo_mode_previous_hierarchy_type: Option<String>,
    pub updated_at: DateTime<Utc>,
}

impl HouseholdSettingsRow {
    /// Convert to shared type. Note: default_rewards and default_punishments
    /// are empty - they should be loaded separately from junction tables.
    pub fn to_shared(&self) -> shared::HouseholdSettings {
        shared::HouseholdSettings {
            household_id: Uuid::parse_str(&self.household_id).unwrap(),
            dark_mode: self.dark_mode,
            role_label_owner: self.role_label_owner.clone(),
            role_label_admin: self.role_label_admin.clone(),
            role_label_member: self.role_label_member.clone(),
            hierarchy_type: HierarchyType::from_str(&self.hierarchy_type)
                .unwrap_or_default(),
            timezone: self.timezone.clone(),
            rewards_enabled: self.rewards_enabled,
            punishments_enabled: self.punishments_enabled,
            chat_enabled: self.chat_enabled,
            vacation_mode: self.vacation_mode,
            vacation_start: self.vacation_start,
            vacation_end: self.vacation_end,
            auto_archive_days: self.auto_archive_days,
            allow_task_suggestions: self.allow_task_suggestions,
            week_start_day: self.week_start_day,
            default_points_reward: self.default_points_reward,
            default_points_penalty: self.default_points_penalty,
            default_rewards: Vec::new(),  // Loaded separately from junction table
            default_punishments: Vec::new(),  // Loaded separately from junction table
            solo_mode: self.solo_mode,
            solo_mode_exit_requested_at: self.solo_mode_exit_requested_at,
            solo_mode_previous_hierarchy_type: self
                .solo_mode_previous_hierarchy_type
                .as_ref()
                .and_then(|s| HierarchyType::from_str(s).ok()),
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
            hierarchy_type: "hierarchy".to_string(),
            timezone: "America/New_York".to_string(),
            rewards_enabled: true,
            punishments_enabled: false,
            chat_enabled: true,
            vacation_mode: false,
            vacation_start: None,
            vacation_end: None,
            auto_archive_days: Some(7),
            allow_task_suggestions: true,
            week_start_day: 0,
            default_points_reward: Some(10),
            default_points_penalty: Some(5),
            solo_mode: false,
            solo_mode_exit_requested_at: None,
            solo_mode_previous_hierarchy_type: None,
            updated_at: now,
        };

        let shared = row.to_shared();

        assert_eq!(shared.household_id, household_id);
        assert!(shared.dark_mode);
        assert_eq!(shared.default_points_reward, Some(10));
        assert_eq!(shared.default_points_penalty, Some(5));
        assert_eq!(shared.role_label_owner, "Parent");
        assert_eq!(shared.role_label_admin, "Guardian");
        assert_eq!(shared.role_label_member, "Child");
        assert_eq!(shared.hierarchy_type, HierarchyType::Hierarchy);
        assert_eq!(shared.timezone, "America/New_York");
        assert!(shared.rewards_enabled);
        assert!(!shared.punishments_enabled);
        assert!(shared.chat_enabled);
        assert!(!shared.vacation_mode);
        assert!(shared.vacation_start.is_none());
        assert!(shared.vacation_end.is_none());
        assert_eq!(shared.auto_archive_days, Some(7));
        assert_eq!(shared.week_start_day, 0);
        assert!(!shared.solo_mode);
        assert!(shared.solo_mode_exit_requested_at.is_none());
        assert!(shared.solo_mode_previous_hierarchy_type.is_none());
    }

    #[test]
    fn test_household_settings_row_invalid_hierarchy_type_defaults() {
        let now = Utc::now();
        let household_id = Uuid::new_v4();

        let row = HouseholdSettingsRow {
            household_id: household_id.to_string(),
            dark_mode: false,
            role_label_owner: "Owner".to_string(),
            role_label_admin: "Admin".to_string(),
            role_label_member: "Member".to_string(),
            hierarchy_type: "invalid".to_string(),
            timezone: "UTC".to_string(),
            rewards_enabled: false,
            punishments_enabled: false,
            chat_enabled: false,
            vacation_mode: false,
            vacation_start: None,
            vacation_end: None,
            auto_archive_days: None,
            allow_task_suggestions: true,
            week_start_day: 6, // Sunday
            default_points_reward: None,
            default_points_penalty: None,
            solo_mode: false,
            solo_mode_exit_requested_at: None,
            solo_mode_previous_hierarchy_type: None,
            updated_at: now,
        };

        let shared = row.to_shared();
        // Should default to Organized when invalid
        assert_eq!(shared.hierarchy_type, HierarchyType::Organized);
        assert_eq!(shared.week_start_day, 6);
    }

    #[test]
    fn test_household_settings_row_solo_mode() {
        let now = Utc::now();
        let exit_requested_at = now - chrono::Duration::hours(24);
        let household_id = Uuid::new_v4();

        let row = HouseholdSettingsRow {
            household_id: household_id.to_string(),
            dark_mode: false,
            role_label_owner: "Owner".to_string(),
            role_label_admin: "Admin".to_string(),
            role_label_member: "Member".to_string(),
            hierarchy_type: "organized".to_string(),
            timezone: "UTC".to_string(),
            rewards_enabled: false,
            punishments_enabled: false,
            chat_enabled: false,
            vacation_mode: false,
            vacation_start: None,
            vacation_end: None,
            auto_archive_days: None,
            allow_task_suggestions: true,
            week_start_day: 0,
            default_points_reward: None,
            default_points_penalty: None,
            solo_mode: true,
            solo_mode_exit_requested_at: Some(exit_requested_at),
            solo_mode_previous_hierarchy_type: Some("hierarchy".to_string()),
            updated_at: now,
        };

        let shared = row.to_shared();
        assert!(shared.solo_mode);
        assert_eq!(shared.solo_mode_exit_requested_at, Some(exit_requested_at));
        assert_eq!(
            shared.solo_mode_previous_hierarchy_type,
            Some(HierarchyType::Hierarchy)
        );
    }
}
