//! Utility functions for loading task modal data.
//!
//! This module provides DRY helper functions to load all data required
//! for task creation and editing modals.

use shared::{HouseholdSettings, MemberWithUser, Punishment, Reward, TaskCategory};

use crate::api::ApiClient;

/// Data required for task create/edit modals
#[derive(Clone, Default)]
pub struct TaskModalData {
    pub members: Vec<MemberWithUser>,
    pub rewards: Vec<Reward>,
    pub punishments: Vec<Punishment>,
    pub categories: Vec<TaskCategory>,
    /// `None` when the settings could not be loaded — callers then fall back to showing
    /// everything rather than hiding sections the household may well be using.
    pub settings: Option<HouseholdSettings>,
}

/// Whether the task form should offer reward links.
///
/// Falls back to `true` when the settings are not loaded (yet): offering a section the
/// household does not use is a small annoyance, hiding links a task already carries would
/// silently drop them on the next save.
pub fn settings_rewards_enabled(settings: &Option<HouseholdSettings>) -> bool {
    settings.as_ref().map(|s| s.rewards_enabled).unwrap_or(true)
}

/// Whether the task form should offer punishment links. See [`settings_rewards_enabled`].
pub fn settings_punishments_enabled(settings: &Option<HouseholdSettings>) -> bool {
    settings
        .as_ref()
        .map(|s| s.punishments_enabled)
        .unwrap_or(true)
}

impl TaskModalData {
    /// Load all task modal data for a household.
    ///
    /// Fetches members, rewards, punishments, categories and settings in sequence.
    /// Returns default empty vectors for any failed requests.
    pub async fn load(household_id: &str) -> Self {
        let members = ApiClient::list_members(household_id)
            .await
            .unwrap_or_default();
        let rewards = ApiClient::list_rewards(household_id)
            .await
            .unwrap_or_default();
        let punishments = ApiClient::list_punishments(household_id)
            .await
            .unwrap_or_default();
        let categories = ApiClient::list_categories(household_id)
            .await
            .unwrap_or_default();
        let settings = ApiClient::get_household_settings(household_id).await.ok();

        Self {
            members,
            rewards,
            punishments,
            categories,
            settings,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn settings_with(rewards: bool, punishments: bool) -> HouseholdSettings {
        HouseholdSettings {
            rewards_enabled: rewards,
            punishments_enabled: punishments,
            ..HouseholdSettings::default()
        }
    }

    #[test]
    fn missing_settings_keep_both_sections_visible() {
        // Without settings the form must not silently hide sections that may be in use.
        assert!(settings_rewards_enabled(&None));
        assert!(settings_punishments_enabled(&None));
    }

    #[test]
    fn disabled_features_are_reported_as_disabled() {
        let settings = Some(settings_with(false, false));
        assert!(!settings_rewards_enabled(&settings));
        assert!(!settings_punishments_enabled(&settings));
    }

    #[test]
    fn features_are_reported_independently() {
        let settings = Some(settings_with(true, false));
        assert!(settings_rewards_enabled(&settings));
        assert!(!settings_punishments_enabled(&settings));
    }

    #[test]
    fn freshly_loaded_data_carries_no_settings() {
        // Default is the state before `load` ran - both sections stay visible.
        let data = TaskModalData::default();
        assert!(data.settings.is_none());
        assert!(settings_rewards_enabled(&data.settings));
    }
}
