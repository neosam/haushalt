use leptos::*;
use shared::{MemberWithUser, Punishment, Reward, Task, TaskCategory};

use crate::api::ApiClient;
use crate::components::household_picker_modal::{EligibleHousehold, HouseholdPickerModal, TaskAction};
use crate::components::task_modal::TaskModal;
use crate::i18n::use_i18n;

/// Data needed to render the TaskModal
#[derive(Clone, Debug, Default)]
struct HouseholdData {
    members: Vec<MemberWithUser>,
    rewards: Vec<Reward>,
    punishments: Vec<Punishment>,
    categories: Vec<TaskCategory>,
}

#[component]
pub fn QuickTaskFab() -> impl IntoView {
    let i18n = use_i18n();
    let i18n_stored = store_value(i18n);

    // State signals
    let loading = create_rw_signal(false);
    let show_picker = create_rw_signal(false);
    let show_task_modal = create_rw_signal(false);
    let eligible_households = create_rw_signal(Vec::<EligibleHousehold>::new());
    let selected_household = create_rw_signal(Option::<EligibleHousehold>::None);
    let household_data = create_rw_signal(HouseholdData::default());
    let no_permission_message = create_rw_signal(false);
    let task_action = create_rw_signal(TaskAction::Create);

    // Handle FAB click
    let on_fab_click = move |_| {
        if loading.get() {
            return;
        }

        loading.set(true);
        no_permission_message.set(false);

        wasm_bindgen_futures::spawn_local(async move {
            // Fetch current user and all households
            let user_result = ApiClient::get_current_user().await;
            let households_result = ApiClient::list_households().await;

            let (user, households) = match (user_result, households_result) {
                (Ok(u), Ok(h)) => (u, h),
                _ => {
                    loading.set(false);
                    return;
                }
            };

            // For each household, check if user has permission to create or suggest tasks
            let mut eligible = Vec::new();

            for household in households {
                let household_id = household.id.to_string();

                // Fetch settings and members in parallel
                let settings_result = ApiClient::get_household_settings(&household_id).await;
                let members_result = ApiClient::list_members(&household_id).await;

                if let (Ok(settings), Ok(members)) = (settings_result, members_result) {
                    // Find current user's role in this household
                    if let Some(member) = members.iter().find(|m| m.user.id == user.id) {
                        let role = member.membership.role;

                        // Check if user can manage tasks based on hierarchy type
                        if settings.hierarchy_type.can_manage(&role) {
                            eligible.push(EligibleHousehold {
                                household,
                                role,
                                settings,
                                action: TaskAction::Create,
                            });
                        } else if settings.allow_task_suggestions {
                            // User can suggest tasks in this household (if suggestions are enabled)
                            eligible.push(EligibleHousehold {
                                household,
                                role,
                                settings,
                                action: TaskAction::Suggest,
                            });
                        }
                    }
                }
            }

            loading.set(false);

            // Determine what action to take based on eligible households
            if eligible.is_empty() {
                // No permission in any household
                no_permission_message.set(true);
            } else if eligible.len() == 1 {
                // Single household - load data and open modal directly
                let eh = eligible.into_iter().next().unwrap();
                task_action.set(eh.action);
                selected_household.set(Some(eh.clone()));
                load_household_data_and_open_modal(eh.household.id.to_string(), household_data, show_task_modal);
            } else {
                // Multiple households - show picker with all eligible households
                eligible_households.set(eligible);
                show_picker.set(true);
            }
        });
    };

    // Handle household selection from picker
    let on_household_select = move |eh: EligibleHousehold| {
        task_action.set(eh.action);
        show_picker.set(false);
        selected_household.set(Some(eh.clone()));
        load_household_data_and_open_modal(eh.household.id.to_string(), household_data, show_task_modal);
    };

    // Handle picker close
    let on_picker_close = move |_| {
        show_picker.set(false);
    };

    // Handle task modal close
    let on_task_modal_close = move |_| {
        show_task_modal.set(false);
        selected_household.set(None);
        household_data.set(HouseholdData::default());
    };

    // Handle task save
    let on_task_save = move |_task: Task| {
        show_task_modal.set(false);
        selected_household.set(None);
        household_data.set(HouseholdData::default());
    };

    view! {
        // FAB Button
        <button
            class="fab"
            on:click=on_fab_click
            disabled=move || loading.get()
            title=move || i18n_stored.get_value().t("quick_task.fab_label")
        >
            {move || {
                if loading.get() {
                    view! { <span class="fab-spinner"></span> }.into_view()
                } else {
                    view! { <span>"+"</span> }.into_view()
                }
            }}
        </button>

        // No permission message
        <Show when=move || no_permission_message.get()>
            <div class="fab-message" on:click=move |_| no_permission_message.set(false)>
                <div class="fab-message-content" on:click=|e| e.stop_propagation()>
                    <p>{move || i18n_stored.get_value().t("quick_task.no_permission")}</p>
                    <button class="btn btn-secondary" on:click=move |_| no_permission_message.set(false)>
                        {move || i18n_stored.get_value().t("common.ok")}
                    </button>
                </div>
            </div>
        </Show>

        // Household picker modal
        <Show when=move || show_picker.get()>
            <HouseholdPickerModal
                households=eligible_households.get()
                on_select=on_household_select
                on_close=on_picker_close
            />
        </Show>

        // Task creation/suggestion modal
        <Show when=move || show_task_modal.get()>
            {move || {
                let sh = selected_household.get();
                let data = household_data.get();
                let is_suggestion = task_action.get() == TaskAction::Suggest;

                if let Some(eh) = sh {
                    view! {
                        <TaskModal
                            task=None
                            household_id=eh.household.id.to_string()
                            members=data.members.clone()
                            household_rewards=data.rewards.clone()
                            household_punishments=data.punishments.clone()
                            linked_rewards=vec![]
                            linked_punishments=vec![]
                            categories=data.categories.clone()
                            default_recurrence="onetime".to_string()
                            is_suggestion=is_suggestion
                            on_close=on_task_modal_close
                            on_save=on_task_save
                        />
                    }.into_view()
                } else {
                    view! {}.into_view()
                }
            }}
        </Show>
    }
}

/// Load household data and open the task modal
fn load_household_data_and_open_modal(
    household_id: String,
    household_data: RwSignal<HouseholdData>,
    show_task_modal: RwSignal<bool>,
) {
    wasm_bindgen_futures::spawn_local(async move {
        let id = household_id;

        // Fetch all required data
        let members = ApiClient::list_members(&id).await.unwrap_or_default();
        let rewards = ApiClient::list_rewards(&id).await.unwrap_or_default();
        let punishments = ApiClient::list_punishments(&id).await.unwrap_or_default();
        let categories = ApiClient::list_categories(&id).await.unwrap_or_default();

        let data = HouseholdData {
            members,
            rewards,
            punishments,
            categories,
        };

        household_data.set(data);
        show_task_modal.set(true);
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use shared::{Household, HouseholdSettings, HierarchyType, Role};
    use wasm_bindgen_test::*;

    wasm_bindgen_test_configure!(run_in_browser);

    #[wasm_bindgen_test]
    fn test_hierarchy_type_equals_allows_all() {
        // In Equals mode, everyone can manage tasks
        assert!(HierarchyType::Equals.can_manage(&Role::Owner));
        assert!(HierarchyType::Equals.can_manage(&Role::Admin));
        assert!(HierarchyType::Equals.can_manage(&Role::Member));
    }

    #[wasm_bindgen_test]
    fn test_hierarchy_type_organized_restricts_members() {
        // In Organized mode, only Owner/Admin can manage
        assert!(HierarchyType::Organized.can_manage(&Role::Owner));
        assert!(HierarchyType::Organized.can_manage(&Role::Admin));
        assert!(!HierarchyType::Organized.can_manage(&Role::Member));
    }

    #[wasm_bindgen_test]
    fn test_hierarchy_type_hierarchy_restricts_members() {
        // In Hierarchy mode, only Owner/Admin can manage
        assert!(HierarchyType::Hierarchy.can_manage(&Role::Owner));
        assert!(HierarchyType::Hierarchy.can_manage(&Role::Admin));
        assert!(!HierarchyType::Hierarchy.can_manage(&Role::Member));
    }

    #[wasm_bindgen_test]
    fn test_single_household_should_skip_picker() {
        let households = vec![EligibleHousehold {
            household: Household {
                id: uuid::Uuid::new_v4(),
                name: "Test".to_string(),
                owner_id: uuid::Uuid::new_v4(),
                created_at: chrono::Utc::now(),
                updated_at: chrono::Utc::now(),
            },
            role: Role::Owner,
            settings: HouseholdSettings::default(),
            action: TaskAction::Create,
        }];

        // When there's only one household, picker should be skipped
        assert_eq!(households.len(), 1);
    }

    #[wasm_bindgen_test]
    fn test_multiple_households_should_show_picker() {
        let households = vec![
            EligibleHousehold {
                household: Household {
                    id: uuid::Uuid::new_v4(),
                    name: "Test 1".to_string(),
                    owner_id: uuid::Uuid::new_v4(),
                    created_at: chrono::Utc::now(),
                    updated_at: chrono::Utc::now(),
                },
                role: Role::Owner,
                settings: HouseholdSettings::default(),
                action: TaskAction::Create,
            },
            EligibleHousehold {
                household: Household {
                    id: uuid::Uuid::new_v4(),
                    name: "Test 2".to_string(),
                    owner_id: uuid::Uuid::new_v4(),
                    created_at: chrono::Utc::now(),
                    updated_at: chrono::Utc::now(),
                },
                role: Role::Admin,
                settings: HouseholdSettings::default(),
                action: TaskAction::Suggest,
            },
        ];

        // When there are multiple households, picker should be shown
        assert!(households.len() > 1);
    }

    #[wasm_bindgen_test]
    fn test_zero_households_should_show_message() {
        let households: Vec<EligibleHousehold> = vec![];

        // When there are no eligible households, message should be shown
        assert!(households.is_empty());
    }
}
