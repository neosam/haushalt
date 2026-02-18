use leptos::*;
use shared::HouseholdSettings;

use crate::i18n::use_i18n;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum HouseholdTab {
    Overview,
    Tasks,
    Notes,
    Journal,
    Rewards,
    Punishments,
    Chat,
    Activity,
    Settings,
}

impl HouseholdTab {
    fn translation_key(&self) -> &'static str {
        match self {
            HouseholdTab::Overview => "tabs.overview",
            HouseholdTab::Tasks => "tabs.tasks",
            HouseholdTab::Notes => "tabs.notes",
            HouseholdTab::Journal => "tabs.journal",
            HouseholdTab::Rewards => "tabs.rewards",
            HouseholdTab::Punishments => "tabs.punishments",
            HouseholdTab::Chat => "tabs.chat",
            HouseholdTab::Activity => "tabs.activity",
            HouseholdTab::Settings => "tabs.settings",
        }
    }

    fn path(&self, household_id: &str) -> String {
        match self {
            HouseholdTab::Overview => format!("/households/{}", household_id),
            HouseholdTab::Tasks => format!("/households/{}/tasks", household_id),
            HouseholdTab::Notes => format!("/households/{}/notes", household_id),
            HouseholdTab::Journal => format!("/households/{}/journal", household_id),
            HouseholdTab::Rewards => format!("/households/{}/rewards", household_id),
            HouseholdTab::Punishments => format!("/households/{}/punishments", household_id),
            HouseholdTab::Chat => format!("/households/{}/chat", household_id),
            HouseholdTab::Activity => format!("/households/{}/activity", household_id),
            HouseholdTab::Settings => format!("/households/{}/settings", household_id),
        }
    }
}

#[component]
pub fn HouseholdTabs(
    household_id: String,
    active_tab: HouseholdTab,
    settings: Option<HouseholdSettings>,
) -> impl IntoView {
    let i18n = use_i18n();
    let i18n_stored = store_value(i18n);

    // Build tabs based on settings
    let tabs = {
        let mut tabs = vec![
            HouseholdTab::Overview,
            HouseholdTab::Tasks,
            HouseholdTab::Notes,
            HouseholdTab::Journal,
        ];
        if let Some(ref s) = settings {
            if s.rewards_enabled {
                tabs.push(HouseholdTab::Rewards);
            }
            if s.punishments_enabled {
                tabs.push(HouseholdTab::Punishments);
            }
            if s.chat_enabled {
                tabs.push(HouseholdTab::Chat);
            }
        }
        tabs.push(HouseholdTab::Activity);
        tabs.push(HouseholdTab::Settings);
        tabs
    };

    view! {
        <nav class="household-tabs">
            {tabs.into_iter().map(|tab| {
                let href = tab.path(&household_id);
                let is_active = tab == active_tab;
                let class = if is_active { "tab-link active" } else { "tab-link" };
                let label = i18n_stored.get_value().t(tab.translation_key());
                view! {
                    <a href=href class=class>
                        {label}
                    </a>
                }
            }).collect_view()}
        </nav>
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use wasm_bindgen_test::*;

    wasm_bindgen_test_configure!(run_in_browser);

    #[wasm_bindgen_test]
    fn test_tab_path_overview() {
        let path = HouseholdTab::Overview.path("abc-123");
        assert_eq!(path, "/households/abc-123");
    }

    #[wasm_bindgen_test]
    fn test_tab_path_tasks() {
        let path = HouseholdTab::Tasks.path("abc-123");
        assert_eq!(path, "/households/abc-123/tasks");
    }

    #[wasm_bindgen_test]
    fn test_tab_path_rewards() {
        let path = HouseholdTab::Rewards.path("abc-123");
        assert_eq!(path, "/households/abc-123/rewards");
    }

    #[wasm_bindgen_test]
    fn test_tab_path_punishments() {
        let path = HouseholdTab::Punishments.path("abc-123");
        assert_eq!(path, "/households/abc-123/punishments");
    }

    #[wasm_bindgen_test]
    fn test_tab_path_activity() {
        let path = HouseholdTab::Activity.path("abc-123");
        assert_eq!(path, "/households/abc-123/activity");
    }

    #[wasm_bindgen_test]
    fn test_tab_equality() {
        assert_eq!(HouseholdTab::Overview, HouseholdTab::Overview);
        assert_ne!(HouseholdTab::Overview, HouseholdTab::Tasks);
    }

    #[wasm_bindgen_test]
    fn test_active_class_logic() {
        let active_tab = HouseholdTab::Tasks;
        let tab = HouseholdTab::Tasks;
        let is_active = tab == active_tab;
        let class = if is_active { "tab-link active" } else { "tab-link" };
        assert_eq!(class, "tab-link active");
    }

    #[wasm_bindgen_test]
    fn test_inactive_class_logic() {
        let active_tab = HouseholdTab::Tasks;
        let tab = HouseholdTab::Overview;
        let is_active = tab == active_tab;
        let class = if is_active { "tab-link active" } else { "tab-link" };
        assert_eq!(class, "tab-link");
    }
}
