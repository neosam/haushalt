//! Utility functions for loading task modal data.
//!
//! This module provides DRY helper functions to load all data required
//! for task creation and editing modals.

use shared::{MemberWithUser, Punishment, Reward, TaskCategory};

use crate::api::ApiClient;

/// Data required for task create/edit modals
#[derive(Clone, Default)]
pub struct TaskModalData {
    pub members: Vec<MemberWithUser>,
    pub rewards: Vec<Reward>,
    pub punishments: Vec<Punishment>,
    pub categories: Vec<TaskCategory>,
}

impl TaskModalData {
    /// Load all task modal data for a household.
    ///
    /// Fetches members, rewards, punishments, and categories in sequence.
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

        Self {
            members,
            rewards,
            punishments,
            categories,
        }
    }
}
