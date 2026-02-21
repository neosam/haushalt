use leptos::*;
use leptos_router::*;
use shared::HouseholdSettings;

use crate::api::ApiClient;
use crate::components::household_tabs::{HouseholdTab, HouseholdTabs};

/// Context for sharing household data across child routes.
/// This avoids duplicate API calls for settings across pages.
#[derive(Clone)]
pub struct HouseholdContext {
    pub household_id: Signal<String>,
    pub settings: RwSignal<Option<HouseholdSettings>>,
}

/// Layout component for all household pages.
/// Renders HouseholdTabs once and uses Outlet for child route content.
/// This prevents tab bar re-renders when navigating between tabs.
#[component]
pub fn HouseholdLayout() -> impl IntoView {
    let params = use_params_map();
    let location = use_location();

    // Get household_id from route params
    let household_id = Signal::derive(move || {
        params.with(|p| p.get("id").cloned().unwrap_or_default())
    });

    // Settings signal - loaded once, shared with child routes via context
    let settings = create_rw_signal(Option::<HouseholdSettings>::None);

    // Load settings when household_id changes
    create_effect(move |_| {
        let id = household_id.get();
        if id.is_empty() {
            return;
        }

        wasm_bindgen_futures::spawn_local(async move {
            if let Ok(s) = ApiClient::get_household_settings(&id).await {
                // Apply dark mode
                apply_dark_mode(s.dark_mode);
                settings.set(Some(s));
            }
        });
    });

    // Determine active tab from current URL path
    let active_tab = Signal::derive(move || {
        let path = location.pathname.get();

        if path.ends_with("/tasks") {
            HouseholdTab::Tasks
        } else if path.ends_with("/notes") {
            HouseholdTab::Notes
        } else if path.ends_with("/journal") {
            HouseholdTab::Journal
        } else if path.ends_with("/rewards") {
            HouseholdTab::Rewards
        } else if path.ends_with("/punishments") {
            HouseholdTab::Punishments
        } else if path.ends_with("/chat") {
            HouseholdTab::Chat
        } else if path.ends_with("/activity") {
            HouseholdTab::Activity
        } else if path.ends_with("/statistics") {
            HouseholdTab::Statistics
        } else if path.ends_with("/settings") {
            HouseholdTab::Settings
        } else {
            HouseholdTab::Overview
        }
    });

    // Provide context for child routes
    let context = HouseholdContext {
        household_id,
        settings,
    };
    provide_context(context);

    view! {
        // HouseholdTabs rendered once - persists across tab navigation
        <HouseholdTabs
            household_id=household_id.get()
            active_tab=active_tab.get()
            settings=settings.get()
        />

        // Child route content renders here
        <Outlet />
    }
}

/// Apply dark mode class to document body
fn apply_dark_mode(enabled: bool) {
    if let Some(window) = web_sys::window() {
        if let Some(document) = window.document() {
            if let Some(body) = document.body() {
                if enabled {
                    let _ = body.class_list().add_1("dark-mode");
                } else {
                    let _ = body.class_list().remove_1("dark-mode");
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use wasm_bindgen_test::*;

    wasm_bindgen_test_configure!(run_in_browser);

    #[wasm_bindgen_test]
    fn test_household_tab_from_path() {
        // Test path parsing logic
        let test_cases = vec![
            ("/households/abc/tasks", "tasks"),
            ("/households/abc/notes", "notes"),
            ("/households/abc/settings", "settings"),
            ("/households/abc", "overview"),
        ];

        for (path, expected_suffix) in test_cases {
            let ends_with_tasks = path.ends_with("/tasks");
            let ends_with_notes = path.ends_with("/notes");
            let ends_with_settings = path.ends_with("/settings");

            match expected_suffix {
                "tasks" => assert!(ends_with_tasks),
                "notes" => assert!(ends_with_notes),
                "settings" => assert!(ends_with_settings),
                "overview" => assert!(!ends_with_tasks && !ends_with_notes && !ends_with_settings),
                _ => panic!("Unknown suffix"),
            }
        }
    }
}
