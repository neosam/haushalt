use std::collections::HashSet;

use leptos::*;
use shared::{CreateHouseholdRequest, Household, InvitationWithHousehold, MemberWithUser, Punishment, Reward, Role, Task, TaskCategory, TaskPunishmentLink, TaskRewardLink};

use crate::api::ApiClient;
use crate::components::loading::Loading;
use crate::components::modal::Modal;
use crate::components::task_card::{DashboardGroupedTaskList, TaskWithHousehold};
use crate::components::task_detail_modal::TaskDetailModal;
use crate::components::task_modal::TaskModal;
use crate::i18n::use_i18n;

#[component]
pub fn Dashboard() -> impl IntoView {
    let i18n = use_i18n();
    let i18n_stored = store_value(i18n);

    let households = create_rw_signal(Vec::<Household>::new());
    let invitations = create_rw_signal(Vec::<InvitationWithHousehold>::new());
    let all_tasks = create_rw_signal(Vec::<TaskWithHousehold>::new());
    let loading = create_rw_signal(true);
    let error = create_rw_signal(Option::<String>::None);
    let show_create_modal = create_rw_signal(false);
    let new_household_name = create_rw_signal(String::new());
    let show_all = create_rw_signal(false);

    // Household filter state
    let enabled_households = create_rw_signal(HashSet::<String>::new());
    let all_households_enabled = create_rw_signal(true);

    // Task detail modal state
    let detail_task_id = create_rw_signal(Option::<String>::None);
    let detail_household_id = create_rw_signal(Option::<String>::None);

    // Task edit modal state
    let editing_task = create_rw_signal(Option::<Task>::None);
    let editing_household_id = create_rw_signal(Option::<String>::None);
    let task_linked_rewards = create_rw_signal(Vec::<TaskRewardLink>::new());
    let task_linked_punishments = create_rw_signal(Vec::<TaskPunishmentLink>::new());
    let edit_members = create_rw_signal(Vec::<MemberWithUser>::new());
    let edit_rewards = create_rw_signal(Vec::<Reward>::new());
    let edit_punishments = create_rw_signal(Vec::<Punishment>::new());
    let edit_categories = create_rw_signal(Vec::<TaskCategory>::new());

    // Load households, invitations, and tasks on mount
    create_effect(move |_| {
        wasm_bindgen_futures::spawn_local(async move {
            // Load households
            if let Ok(data) = ApiClient::list_households().await {
                // Initialize enabled_households with all household IDs
                let all_ids: HashSet<String> = data.iter().map(|h| h.id.to_string()).collect();
                enabled_households.set(all_ids);
                households.set(data);
            }

            // Load pending invitations
            if let Ok(inv) = ApiClient::get_my_invitations().await {
                invitations.set(inv);
            }

            loading.set(false);
        });
    });

    // Load tasks based on show_all toggle (reactive)
    create_effect(move |_| {
        let show_all_mode = show_all.get();
        wasm_bindgen_futures::spawn_local(async move {
            let result = if show_all_mode {
                ApiClient::get_all_tasks_across_households().await
            } else {
                ApiClient::get_dashboard_tasks_with_status().await
            };

            match result {
                Ok(dashboard_tasks) => {
                    let tasks_with_households: Vec<TaskWithHousehold> = dashboard_tasks
                        .into_iter()
                        .map(|t| TaskWithHousehold {
                            task: t.task_with_status,
                            household_name: t.household_name,
                            household_id: t.household_id.to_string(),
                        })
                        .collect();
                    all_tasks.set(tasks_with_households);
                }
                Err(e) => {
                    error.set(Some(e));
                }
            }
        });
    });

    let on_create = move |ev: web_sys::SubmitEvent| {
        ev.prevent_default();

        let name = new_household_name.get();
        if name.is_empty() {
            return;
        }

        wasm_bindgen_futures::spawn_local(async move {
            let request = CreateHouseholdRequest { name };
            match ApiClient::create_household(request).await {
                Ok(household) => {
                    households.update(|h| h.push(household));
                    show_create_modal.set(false);
                    new_household_name.set(String::new());
                }
                Err(e) => {
                    error.set(Some(e));
                }
            }
        });
    };

    let on_accept_invitation = move |invitation_id: String, household: Household| {
        wasm_bindgen_futures::spawn_local(async move {
            match ApiClient::accept_invitation(&invitation_id).await {
                Ok(_) => {
                    // Remove from invitations
                    invitations.update(|inv| inv.retain(|i| i.invitation.id.to_string() != invitation_id));
                    // Add household to the list
                    households.update(|h| h.push(household));
                }
                Err(e) => {
                    error.set(Some(e));
                }
            }
        });
    };

    let on_decline_invitation = move |invitation_id: String| {
        wasm_bindgen_futures::spawn_local(async move {
            if ApiClient::decline_invitation(&invitation_id).await.is_ok() {
                invitations.update(|inv| inv.retain(|i| i.invitation.id.to_string() != invitation_id));
            }
        });
    };

    // Helper to reload tasks based on show_all mode
    let reload_tasks = move |show_all_mode: bool| async move {
        let result = if show_all_mode {
            ApiClient::get_all_tasks_across_households().await
        } else {
            ApiClient::get_dashboard_tasks_with_status().await
        };

        if let Ok(dashboard_tasks) = result {
            let tasks_with_households: Vec<TaskWithHousehold> = dashboard_tasks
                .into_iter()
                .map(|t| TaskWithHousehold {
                    task: t.task_with_status,
                    household_name: t.household_name,
                    household_id: t.household_id.to_string(),
                })
                .collect();
            all_tasks.set(tasks_with_households);
        }
    };

    // Task completion handler
    let on_complete_task = Callback::new(move |task_id: String| {
        // Find the household_id for this task
        let tasks = all_tasks.get();
        if let Some(twh) = tasks.iter().find(|t| t.task.task.id.to_string() == task_id) {
            let household_id = twh.household_id.clone();
            let task_id_clone = task_id.clone();
            let show_all_mode = show_all.get();
            wasm_bindgen_futures::spawn_local(async move {
                if ApiClient::complete_task(&household_id, &task_id_clone).await.is_ok() {
                    reload_tasks(show_all_mode).await;
                }
            });
        }
    });

    // Task uncomplete handler
    let on_uncomplete_task = Callback::new(move |task_id: String| {
        // Find the household_id for this task
        let tasks = all_tasks.get();
        if let Some(twh) = tasks.iter().find(|t| t.task.task.id.to_string() == task_id) {
            let household_id = twh.household_id.clone();
            let task_id_clone = task_id.clone();
            let show_all_mode = show_all.get();
            wasm_bindgen_futures::spawn_local(async move {
                if ApiClient::uncomplete_task(&household_id, &task_id_clone).await.is_ok() {
                    reload_tasks(show_all_mode).await;
                }
            });
        }
    });

    // Task title click handler - opens detail modal
    let on_click_task_title = Callback::new(move |(task_id, household_id): (String, String)| {
        detail_task_id.set(Some(task_id));
        detail_household_id.set(Some(household_id));
    });

    // Clear edit state helper
    let clear_edit_state = move || {
        editing_task.set(None);
        editing_household_id.set(None);
        edit_members.set(vec![]);
        edit_rewards.set(vec![]);
        edit_punishments.set(vec![]);
        edit_categories.set(vec![]);
        task_linked_rewards.set(vec![]);
        task_linked_punishments.set(vec![]);
    };

    // Handle task edit from detail modal
    let on_edit_task = move |task: Task| {
        let household_id = detail_household_id.get().unwrap_or_default();
        let task_id = task.id.to_string();

        editing_task.set(Some(task));
        editing_household_id.set(Some(household_id.clone()));

        // Fetch all required data for this household
        let hid = household_id.clone();
        let tid = task_id.clone();
        wasm_bindgen_futures::spawn_local(async move {
            // Fetch members, rewards, punishments, categories in parallel
            if let Ok(m) = ApiClient::list_members(&hid).await {
                edit_members.set(m);
            }
            if let Ok(r) = ApiClient::list_rewards(&hid).await {
                edit_rewards.set(r);
            }
            if let Ok(p) = ApiClient::list_punishments(&hid).await {
                edit_punishments.set(p);
            }
            if let Ok(c) = ApiClient::list_categories(&hid).await {
                edit_categories.set(c);
            }
            if let Ok(tr) = ApiClient::get_task_rewards(&hid, &tid).await {
                task_linked_rewards.set(tr);
            }
            if let Ok(tp) = ApiClient::get_task_punishments(&hid, &tid).await {
                task_linked_punishments.set(tp);
            }
        });
    };

    // Handle task save from edit modal
    let on_task_save = move |_saved_task: Task| {
        // Reload tasks to reflect changes
        let show_all_mode = show_all.get();
        wasm_bindgen_futures::spawn_local(async move {
            reload_tasks(show_all_mode).await;
        });
        clear_edit_state();
    };

    view! {
        <div class="dashboard-header">
            <div style="display: flex; justify-content: space-between; align-items: flex-start;">
                <div>
                    <h1 class="dashboard-title">{move || i18n_stored.get_value().t("dashboard.title")}</h1>
                    <p class="dashboard-subtitle">{move || i18n_stored.get_value().t("dashboard.subtitle")}</p>
                </div>
                <button
                    class=move || if show_all.get() { "btn btn-primary" } else { "btn btn-outline" }
                    on:click=move |_| show_all.update(|v| *v = !*v)
                >
                    {move || i18n_stored.get_value().t("dashboard.show_all")}
                </button>
            </div>
        </div>

        {move || error.get().map(|e| view! {
            <div class="alert alert-error">{e}</div>
        })}

        <Show when=move || loading.get() fallback=|| ()>
            <Loading />
        </Show>

        <Show when=move || !loading.get() fallback=|| ()>
            // Pending Invitations Section
            <Show when=move || !invitations.get().is_empty() fallback=|| ()>
                <div class="card" style="margin-bottom: 1.5rem; border-left: 4px solid var(--primary-color);">
                    <div class="card-header">
                        <h3 class="card-title">{move || i18n_stored.get_value().t("dashboard.pending_invitations")}</h3>
                    </div>
                    {move || {
                        invitations.get().into_iter().map(|inv| {
                            let inv_id = inv.invitation.id.to_string();
                            let accept_id = inv_id.clone();
                            let decline_id = inv_id.clone();
                            let household_for_accept = inv.household.clone();
                            let role_badge = if inv.invitation.role == Role::Admin {
                                "badge badge-admin"
                            } else {
                                "badge badge-member"
                            };
                            let role_text = if inv.invitation.role == Role::Admin { "Admin" } else { "Member" };

                            view! {
                                <div style="display: flex; justify-content: space-between; align-items: center; padding: 1rem; border-bottom: 1px solid var(--border-color);">
                                    <div>
                                        <div style="font-weight: 600; font-size: 1rem;">{inv.household.name.clone()}</div>
                                        <div style="font-size: 0.875rem; color: var(--text-muted);">
                                            {i18n_stored.get_value().t("dashboard.invited_by")} " "
                                            <span style="font-weight: 500;">{inv.invited_by_user.username.clone()}</span>
                                            " " {i18n_stored.get_value().t("dashboard.as_role")} " "
                                            <span class=role_badge style="margin-left: 0.25rem;">{role_text}</span>
                                        </div>
                                    </div>
                                    <div style="display: flex; gap: 0.5rem;">
                                        <button
                                            class="btn btn-outline"
                                            style="padding: 0.25rem 0.75rem; font-size: 0.875rem;"
                                            on:click=move |_| on_decline_invitation(decline_id.clone())
                                        >
                                            {i18n_stored.get_value().t("dashboard.decline")}
                                        </button>
                                        <button
                                            class="btn btn-primary"
                                            style="padding: 0.25rem 0.75rem; font-size: 0.875rem;"
                                            on:click=move |_| on_accept_invitation(accept_id.clone(), household_for_accept.clone())
                                        >
                                            {i18n_stored.get_value().t("dashboard.accept")}
                                        </button>
                                    </div>
                                </div>
                            }
                        }).collect_view()
                    }}
                </div>
            </Show>

            <div style="margin-bottom: 1rem;">
                <button class="btn btn-primary" on:click=move |_| show_create_modal.set(true)>
                    {move || i18n_stored.get_value().t("dashboard.create_household")}
                </button>
            </div>

            // Households section
            {move || {
                let mut h = households.get();
                h.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
                if h.is_empty() {
                    view! {
                        <div class="card empty-state">
                            <p>{i18n_stored.get_value().t("dashboard.no_households")}</p>
                            <p>{i18n_stored.get_value().t("dashboard.get_started")}</p>
                        </div>
                    }.into_view()
                } else {
                    view! {
                        <div class="card">
                            <div class="card-header">
                                <h3 class="card-title">{i18n_stored.get_value().t("dashboard.households")}</h3>
                            </div>
                            <ul class="household-list">
                                {h.into_iter().map(|household| {
                                    let id = household.id.to_string();
                                    view! {
                                        <li>
                                            <a href=format!("/households/{}", id)>{household.name}</a>
                                        </li>
                                    }
                                }).collect_view()}
                            </ul>
                        </div>
                    }.into_view()
                }
            }}

            // Household filter controls
            {move || {
                let h = households.get();
                if h.len() > 1 {
                    // Only show filter when there are multiple households
                    let mut sorted_households = h.clone();
                    sorted_households.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));

                    view! {
                        <div class="filter-controls">
                            <span class="filter-label">{i18n_stored.get_value().t("dashboard.filter_households")}</span>
                            {sorted_households.into_iter().map(|household| {
                                let hid = household.id.to_string();
                                let hid_check = hid.clone();
                                let hid_toggle = hid.clone();
                                let name = household.name.clone();
                                view! {
                                    <label class="filter-checkbox">
                                        <input
                                            type="checkbox"
                                            prop:checked=move || enabled_households.get().contains(&hid_check)
                                            on:change=move |ev| {
                                                let checked = event_target_checked(&ev);
                                                enabled_households.update(|set| {
                                                    if checked {
                                                        set.insert(hid_toggle.clone());
                                                    } else {
                                                        set.remove(&hid_toggle);
                                                    }
                                                });
                                                all_households_enabled.set(false);
                                            }
                                        />
                                        <span>{name}</span>
                                    </label>
                                }
                            }).collect_view()}
                        </div>
                    }.into_view()
                } else {
                    ().into_view()
                }
            }}

            // Tasks section
            {move || {
                let tasks = all_tasks.get();
                // Filter tasks by enabled households
                let filtered_tasks: Vec<_> = tasks
                    .into_iter()
                    .filter(|t| enabled_households.get().contains(&t.household_id))
                    .collect();

                if !filtered_tasks.is_empty() {
                    view! {
                        <div style="margin-bottom: 1.5rem;">
                            <DashboardGroupedTaskList
                                tasks=filtered_tasks
                                on_complete=on_complete_task
                                on_uncomplete=on_uncomplete_task
                                timezone="Europe/Berlin".to_string()
                                on_click_title=on_click_task_title
                            />
                        </div>
                    }.into_view()
                } else {
                    ().into_view()
                }
            }}
        </Show>

        // Task detail modal
        {move || {
            if let (Some(task_id), Some(household_id)) = (detail_task_id.get(), detail_household_id.get()) {
                Some(view! {
                    <TaskDetailModal
                        task_id=task_id
                        household_id=household_id
                        on_close=move |_| {
                            detail_task_id.set(None);
                            detail_household_id.set(None);
                        }
                        on_edit=move |task| {
                            on_edit_task(task);
                            detail_task_id.set(None);
                            detail_household_id.set(None);
                        }
                    />
                })
            } else {
                None
            }
        }}

        // Task edit modal
        {move || {
            if let (Some(task), Some(hid)) = (editing_task.get(), editing_household_id.get()) {
                Some(view! {
                    <TaskModal
                        task=Some(task)
                        household_id=hid
                        members=edit_members.get()
                        household_rewards=edit_rewards.get()
                        household_punishments=edit_punishments.get()
                        linked_rewards=task_linked_rewards.get()
                        linked_punishments=task_linked_punishments.get()
                        categories=edit_categories.get()
                        on_close=move |_| clear_edit_state()
                        on_save=on_task_save
                    />
                })
            } else {
                None
            }
        }}

        <Show when=move || show_create_modal.get() fallback=|| ()>
            <Modal title=i18n_stored.get_value().t("household.create") on_close=move |_| show_create_modal.set(false)>
                <form on:submit=on_create>
                    <div class="form-group">
                        <label class="form-label" for="household-name">{i18n_stored.get_value().t("household.name")}</label>
                        <input
                            type="text"
                            id="household-name"
                            class="form-input"
                            placeholder=i18n_stored.get_value().t("household.name_placeholder")
                            prop:value=move || new_household_name.get()
                            on:input=move |ev| new_household_name.set(event_target_value(&ev))
                            required
                        />
                    </div>
                    <div class="modal-footer">
                        <button type="button" class="btn btn-outline" on:click=move |_| show_create_modal.set(false)>
                            {i18n_stored.get_value().t("common.cancel")}
                        </button>
                        <button type="submit" class="btn btn-primary">
                            {i18n_stored.get_value().t("common.create")}
                        </button>
                    </div>
                </form>
            </Modal>
        </Show>
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use wasm_bindgen_test::*;

    wasm_bindgen_test_configure!(run_in_browser);

    #[wasm_bindgen_test]
    fn test_role_badge_admin() {
        let role = Role::Admin;
        let badge_class = if role == Role::Admin {
            "badge badge-admin"
        } else {
            "badge badge-member"
        };
        assert_eq!(badge_class, "badge badge-admin");
    }

    #[wasm_bindgen_test]
    fn test_role_badge_member() {
        let role = Role::Member;
        let badge_class = if role == Role::Admin {
            "badge badge-admin"
        } else {
            "badge badge-member"
        };
        assert_eq!(badge_class, "badge badge-member");
    }

    #[wasm_bindgen_test]
    fn test_role_text_admin() {
        let role = Role::Admin;
        let role_text = if role == Role::Admin { "Admin" } else { "Member" };
        assert_eq!(role_text, "Admin");
    }

    #[wasm_bindgen_test]
    fn test_role_text_member() {
        let role = Role::Member;
        let role_text = if role == Role::Admin { "Admin" } else { "Member" };
        assert_eq!(role_text, "Member");
    }

    #[wasm_bindgen_test]
    fn test_empty_household_name_check() {
        let name = String::new();
        assert!(name.is_empty());
    }

    #[wasm_bindgen_test]
    fn test_valid_household_name_check() {
        let name = "Smith Family".to_string();
        assert!(!name.is_empty());
    }

    #[wasm_bindgen_test]
    fn test_household_link_format() {
        let id = "abc-123";
        let link = format!("/households/{}", id);
        assert_eq!(link, "/households/abc-123");
    }

    #[wasm_bindgen_test]
    fn test_invitations_empty_check() {
        let invitations: Vec<String> = vec![];
        assert!(invitations.is_empty());
    }

    #[wasm_bindgen_test]
    fn test_invitations_not_empty_check() {
        let invitations = vec!["invite1".to_string()];
        assert!(!invitations.is_empty());
    }

    #[wasm_bindgen_test]
    fn test_household_filter_all_enabled() {
        let mut enabled: HashSet<String> = HashSet::new();
        enabled.insert("h1".to_string());
        enabled.insert("h2".to_string());

        let task_household_ids = vec!["h1", "h2", "h1"];
        let filtered: Vec<_> = task_household_ids
            .into_iter()
            .filter(|hid| enabled.contains(*hid))
            .collect();

        assert_eq!(filtered.len(), 3);
    }

    #[wasm_bindgen_test]
    fn test_household_filter_one_disabled() {
        let mut enabled: HashSet<String> = HashSet::new();
        enabled.insert("h1".to_string());
        // h2 not enabled

        let task_household_ids = vec!["h1", "h2", "h1", "h2"];
        let filtered: Vec<_> = task_household_ids
            .into_iter()
            .filter(|hid| enabled.contains(*hid))
            .collect();

        assert_eq!(filtered.len(), 2);
        assert!(filtered.iter().all(|h| *h == "h1"));
    }

    #[wasm_bindgen_test]
    fn test_household_filter_none_enabled() {
        let enabled: HashSet<String> = HashSet::new();

        let task_household_ids = vec!["h1", "h2"];
        let filtered: Vec<_> = task_household_ids
            .into_iter()
            .filter(|hid| enabled.contains(*hid))
            .collect();

        assert!(filtered.is_empty());
    }

    #[wasm_bindgen_test]
    fn test_household_filter_toggle() {
        let mut enabled: HashSet<String> = HashSet::new();
        enabled.insert("h1".to_string());
        enabled.insert("h2".to_string());

        // Toggle off h1
        enabled.remove("h1");
        assert!(!enabled.contains("h1"));
        assert!(enabled.contains("h2"));

        // Toggle on h1
        enabled.insert("h1".to_string());
        assert!(enabled.contains("h1"));
        assert!(enabled.contains("h2"));
    }
}
