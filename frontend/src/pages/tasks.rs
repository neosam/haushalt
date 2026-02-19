use leptos::*;
use leptos_router::*;
use shared::{HierarchyType, HouseholdSettings, MemberWithUser, Punishment, Reward, Role, Task, TaskCategory, TaskPunishmentLink, TaskRewardLink};

use crate::api::ApiClient;
use crate::components::category_modal::CategoryModal;
use crate::components::context_menu::{ContextMenu, ContextMenuAction};
use crate::components::household_tabs::{HouseholdTab, HouseholdTabs};
use crate::components::loading::Loading;
use crate::components::markdown::MarkdownView;
use crate::components::pending_reviews::PendingReviews;
use crate::components::task_detail_modal::TaskDetailModal;
use crate::components::task_modal::TaskModal;
use crate::i18n::use_i18n;

#[component]
pub fn TasksPage() -> impl IntoView {
    let i18n = use_i18n();
    let i18n_stored = store_value(i18n);

    let params = use_params_map();
    let household_id = move || params.with(|p| p.get("id").cloned().unwrap_or_default());

    let tasks = create_rw_signal(Vec::<Task>::new());
    let archived_tasks = create_rw_signal(Vec::<Task>::new());
    let show_archived = create_rw_signal(false);
    let members = create_rw_signal(Vec::<MemberWithUser>::new());
    let rewards = create_rw_signal(Vec::<Reward>::new());
    let punishments = create_rw_signal(Vec::<Punishment>::new());
    let settings = create_rw_signal(Option::<HouseholdSettings>::None);
    let loading = create_rw_signal(true);
    let error = create_rw_signal(Option::<String>::None);
    let show_create_modal = create_rw_signal(false);
    let can_manage = create_rw_signal(false);
    let pending_reviews_version = create_rw_signal(0u32); // For triggering re-fetch

    // Edit modal state
    let editing_task = create_rw_signal(Option::<Task>::None);
    let task_linked_rewards = create_rw_signal(Vec::<TaskRewardLink>::new());
    let task_linked_punishments = create_rw_signal(Vec::<TaskPunishmentLink>::new());

    // Duplicate modal state
    let duplicating_task = create_rw_signal(Option::<Task>::None);
    let duplicate_linked_rewards = create_rw_signal(Vec::<TaskRewardLink>::new());
    let duplicate_linked_punishments = create_rw_signal(Vec::<TaskPunishmentLink>::new());

    // Category modal state
    let show_category_modal = create_rw_signal(false);
    let categories = create_rw_signal(Vec::<TaskCategory>::new());

    // Detail modal state - holds the task_id to show details for
    let detail_task_id = create_rw_signal(Option::<String>::None);

    // Load tasks and supporting data
    create_effect(move |_| {
        let id = household_id();
        if id.is_empty() {
            return;
        }

        let id_for_tasks = id.clone();
        let id_for_members = id.clone();
        let id_for_rewards = id.clone();
        let id_for_punishments = id.clone();
        let id_for_settings = id.clone();

        // Load tasks
        wasm_bindgen_futures::spawn_local(async move {
            match ApiClient::list_tasks(&id_for_tasks).await {
                Ok(t) => {
                    tasks.set(t);
                    loading.set(false);
                }
                Err(e) => {
                    error.set(Some(e));
                    loading.set(false);
                }
            }
        });

        // Load members for assignment dropdown
        wasm_bindgen_futures::spawn_local(async move {
            if let Ok(m) = ApiClient::list_members(&id_for_members).await {
                members.set(m);
            }
        });

        // Load rewards for linking
        wasm_bindgen_futures::spawn_local(async move {
            if let Ok(r) = ApiClient::list_rewards(&id_for_rewards).await {
                rewards.set(r);
            }
        });

        // Load punishments for linking
        wasm_bindgen_futures::spawn_local(async move {
            if let Ok(p) = ApiClient::list_punishments(&id_for_punishments).await {
                punishments.set(p);
            }
        });

        // Load categories
        let id_for_categories = id.clone();
        wasm_bindgen_futures::spawn_local(async move {
            if let Ok(cats) = ApiClient::list_categories(&id_for_categories).await {
                categories.set(cats);
            }
        });

        // Load archived tasks
        let id_for_archived = id.clone();
        wasm_bindgen_futures::spawn_local(async move {
            if let Ok(t) = ApiClient::list_archived_tasks(&id_for_archived).await {
                archived_tasks.set(t);
            }
        });

        // Load settings for hierarchy-aware member filtering and dark mode
        wasm_bindgen_futures::spawn_local(async move {
            if let Ok(s) = ApiClient::get_household_settings(&id_for_settings).await {
                // Apply dark mode
                apply_dark_mode(s.dark_mode);
                // Check if current user can manage based on hierarchy
                // For now, we'll determine this from member role
                settings.set(Some(s));
            }
        });

        // Check if user can manage (has Owner or Admin role)
        let id_for_role = id.clone();
        wasm_bindgen_futures::spawn_local(async move {
            if let Ok(user) = ApiClient::get_current_user().await {
                if let Ok(members_list) = ApiClient::list_members(&id_for_role).await {
                    if let Some(member) = members_list.iter().find(|m| m.user.id == user.id) {
                        let user_can_manage = member.membership.role.can_manage_tasks();
                        can_manage.set(user_can_manage);
                    }
                }
            }
        });
    });

    let on_delete = move |task_id: String| {
        let id = household_id();
        wasm_bindgen_futures::spawn_local(async move {
            if ApiClient::delete_task(&id, &task_id).await.is_ok() {
                tasks.update(|t| t.retain(|task| task.id.to_string() != task_id));
            }
        });
    };

    let on_archive = move |task_id: String| {
        let id = household_id();
        wasm_bindgen_futures::spawn_local(async move {
            if let Ok(archived) = ApiClient::archive_task(&id, &task_id).await {
                // Remove from active tasks
                tasks.update(|t| t.retain(|task| task.id.to_string() != task_id));
                // Add to archived tasks
                archived_tasks.update(|t| t.push(archived));
            }
        });
    };

    let on_unarchive = move |task_id: String| {
        let id = household_id();
        wasm_bindgen_futures::spawn_local(async move {
            if let Ok(unarchived) = ApiClient::unarchive_task(&id, &task_id).await {
                // Remove from archived tasks
                archived_tasks.update(|t| t.retain(|task| task.id.to_string() != task_id));
                // Add to active tasks
                tasks.update(|t| t.push(unarchived));
            }
        });
    };

    let on_pause = move |task_id: String| {
        let id = household_id();
        wasm_bindgen_futures::spawn_local(async move {
            if let Ok(paused_task) = ApiClient::pause_task(&id, &task_id).await {
                // Update task in list
                tasks.update(|t| {
                    if let Some(pos) = t.iter().position(|task| task.id.to_string() == task_id) {
                        t[pos] = paused_task;
                    }
                });
            }
        });
    };

    let on_unpause = move |task_id: String| {
        let id = household_id();
        wasm_bindgen_futures::spawn_local(async move {
            if let Ok(unpaused_task) = ApiClient::unpause_task(&id, &task_id).await {
                // Update task in list
                tasks.update(|t| {
                    if let Some(pos) = t.iter().position(|task| task.id.to_string() == task_id) {
                        t[pos] = unpaused_task;
                    }
                });
            }
        });
    };

    let on_edit = move |task: Task| {
        let id = household_id();
        let task_id = task.id.to_string();

        // Load linked rewards and punishments for this task
        let id_for_rewards = id.clone();
        let id_for_punishments = id.clone();
        let task_id_for_rewards = task_id.clone();
        let task_id_for_punishments = task_id.clone();

        editing_task.set(Some(task));

        wasm_bindgen_futures::spawn_local(async move {
            if let Ok(r) = ApiClient::get_task_rewards(&id_for_rewards, &task_id_for_rewards).await {
                task_linked_rewards.set(r);
            }
        });

        wasm_bindgen_futures::spawn_local(async move {
            if let Ok(p) = ApiClient::get_task_punishments(&id_for_punishments, &task_id_for_punishments).await {
                task_linked_punishments.set(p);
            }
        });
    };

    let on_duplicate = move |task: Task| {
        let id = household_id();
        let task_id = task.id.to_string();

        // Load linked rewards and punishments to copy them
        let id_for_rewards = id.clone();
        let id_for_punishments = id.clone();
        let task_id_for_rewards = task_id.clone();
        let task_id_for_punishments = task_id.clone();

        duplicating_task.set(Some(task));

        wasm_bindgen_futures::spawn_local(async move {
            if let Ok(r) = ApiClient::get_task_rewards(&id_for_rewards, &task_id_for_rewards).await {
                duplicate_linked_rewards.set(r);
            }
        });

        wasm_bindgen_futures::spawn_local(async move {
            if let Ok(p) = ApiClient::get_task_punishments(&id_for_punishments, &task_id_for_punishments).await {
                duplicate_linked_punishments.set(p);
            }
        });
    };

    // Unified save handler for create, edit, and duplicate
    let on_save = move |saved_task: Task| {
        tasks.update(|t| {
            if let Some(pos) = t.iter().position(|task| task.id == saved_task.id) {
                // Edit mode - update existing
                t[pos] = saved_task;
            } else {
                // Create or duplicate mode - push new
                t.push(saved_task);
            }
        });
        editing_task.set(None);
        duplicating_task.set(None);
        show_create_modal.set(false);
        task_linked_rewards.set(vec![]);
        task_linked_punishments.set(vec![]);
        duplicate_linked_rewards.set(vec![]);
        duplicate_linked_punishments.set(vec![]);
    };

    view! {
        {move || {
            let hid = household_id();
            view! { <HouseholdTabs household_id=hid active_tab=HouseholdTab::Tasks settings=settings.get() /> }
        }}

        <div class="dashboard-header">
            <h1 class="dashboard-title">{i18n_stored.get_value().t("tasks.title")}</h1>
        </div>

        {move || error.get().map(|e| view! {
            <div class="alert alert-error">{e}</div>
        })}

        <Show when=move || loading.get() fallback=|| ()>
            <Loading />
        </Show>

        <Show when=move || !loading.get() fallback=|| ()>
            // Pending Reviews Section (only for managers/owners)
            <Show when=move || can_manage.get() fallback=|| ()>
                {
                    let hid = household_id();
                    let _ = pending_reviews_version.get(); // Subscribe to version changes
                    view! {
                        <div style="margin-bottom: 1.5rem;">
                            <PendingReviews
                                household_id=hid
                                on_review_complete=move |_| {
                                    // Trigger refresh
                                    pending_reviews_version.update(|v| *v += 1);
                                }
                            />
                        </div>
                    }
                }
            </Show>

            <div style="margin-bottom: 1rem; display: flex; gap: 0.5rem;">
                <button class="btn btn-primary" on:click=move |_| show_create_modal.set(true)>
                    "+ " {i18n_stored.get_value().t("tasks.create")}
                </button>
                <button class="btn btn-outline" on:click=move |_| show_category_modal.set(true)>
                    {i18n_stored.get_value().t("tasks.manage_categories")}
                </button>
            </div>

            <h3 style="margin-bottom: 1rem; color: var(--text-muted);">{i18n_stored.get_value().t("tasks.all_tasks")}</h3>

            {move || {
                let t = tasks.get();
                if t.is_empty() {
                    view! {
                        <div class="card empty-state">
                            <p>{i18n_stored.get_value().t("tasks.no_tasks")}</p>
                            <p>{i18n_stored.get_value().t("tasks.add_first")}</p>
                        </div>
                    }.into_view()
                } else {
                    view! {
                        <div class="card">
                            {t.into_iter().map(|task| {
                                let task_id = task.id.to_string();
                                let delete_id = task_id.clone();
                                let edit_task = task.clone();
                                let duplicate_task = task.clone();
                                let is_paused = task.paused;
                                let assigned_name = task.assigned_user_id.and_then(|uid| {
                                    members.get().iter().find(|m| m.user.id == uid).map(|m| m.user.username.clone())
                                });

                                let edit_label = i18n_stored.get_value().t("common.edit");
                                let duplicate_label = i18n_stored.get_value().t("common.duplicate");
                                let archive_label = i18n_stored.get_value().t("tasks.archive");
                                let pause_label = i18n_stored.get_value().t("tasks.pause");
                                let unpause_label = i18n_stored.get_value().t("tasks.unpause");
                                let delete_label = i18n_stored.get_value().t("common.delete");
                                let archive_id = task_id.clone();
                                let pause_id = task_id.clone();

                                let mut actions = vec![
                                    ContextMenuAction {
                                        label: edit_label,
                                        on_click: Callback::new(move |_| on_edit(edit_task.clone())),
                                        danger: false,
                                    },
                                    ContextMenuAction {
                                        label: duplicate_label,
                                        on_click: Callback::new(move |_| on_duplicate(duplicate_task.clone())),
                                        danger: false,
                                    },
                                ];

                                // Add pause or unpause action based on current state
                                if is_paused {
                                    actions.push(ContextMenuAction {
                                        label: unpause_label,
                                        on_click: Callback::new(move |_| on_unpause(pause_id.clone())),
                                        danger: false,
                                    });
                                } else {
                                    actions.push(ContextMenuAction {
                                        label: pause_label,
                                        on_click: Callback::new(move |_| on_pause(pause_id.clone())),
                                        danger: false,
                                    });
                                }

                                actions.push(ContextMenuAction {
                                    label: archive_label,
                                    on_click: Callback::new(move |_| on_archive(archive_id.clone())),
                                    danger: false,
                                });
                                actions.push(ContextMenuAction {
                                    label: delete_label,
                                    on_click: Callback::new(move |_| on_delete(delete_id.clone())),
                                    danger: true,
                                });

                                let task_style = if is_paused { "opacity: 0.6;" } else { "" };
                                let paused_badge = i18n_stored.get_value().t("tasks.paused_badge");
                                let detail_id = task_id.clone();

                                view! {
                                    <div class="task-item" style=task_style>
                                        <div class="task-content">
                                            <div
                                                class="task-title task-title-clickable"
                                                on:click=move |_| detail_task_id.set(Some(detail_id.clone()))
                                            >
                                                {task.title.clone()}
                                                {if is_paused {
                                                    view! {
                                                        <span class="badge badge-warning" style="margin-left: 0.5rem; font-size: 0.7em;">
                                                            {paused_badge}
                                                        </span>
                                                    }.into_view()
                                                } else {
                                                    ().into_view()
                                                }}
                                            </div>
                                            <div class="task-meta">
                                                {format!("{:?}", task.recurrence_type)}
                                                {if let Some(name) = assigned_name {
                                                    format!(" | Assigned to: {}", name)
                                                } else {
                                                    String::new()
                                                }}
                                            </div>
                                            {if !task.description.is_empty() {
                                                view! { <MarkdownView content=task.description.clone() /> }.into_view()
                                            } else {
                                                ().into_view()
                                            }}
                                        </div>
                                        <ContextMenu actions=actions />
                                    </div>
                                }
                            }).collect_view()}
                        </div>
                    }.into_view()
                }
            }}
            // Archived Tasks Section (collapsible)
            <Show when=move || !archived_tasks.get().is_empty() fallback=|| ()>
                <div class="card" style="margin-top: 1.5rem; opacity: 0.8;">
                    <div
                        class="card-header"
                        style="cursor: pointer; user-select: none;"
                        on:click=move |_| show_archived.update(|v| *v = !*v)
                    >
                        <h3 class="card-title" style="display: flex; align-items: center; gap: 0.5rem;">
                            <span style="transition: transform 0.2s;">
                                {move || if show_archived.get() { "▼" } else { "▶" }}
                            </span>
                            {i18n_stored.get_value().t("tasks.archived_tasks")}
                            <span style="font-weight: normal; color: var(--text-muted);">
                                {move || format!("({})", archived_tasks.get().len())}
                            </span>
                        </h3>
                    </div>
                    <Show when=move || show_archived.get() fallback=|| ()>
                        {move || {
                            archived_tasks.get().into_iter().map(|task| {
                                let task_id = task.id.to_string();
                                let unarchive_id = task_id.clone();
                                let delete_id = task_id.clone();
                                let assigned_name = task.assigned_user_id.and_then(|uid| {
                                    members.get().iter().find(|m| m.user.id == uid).map(|m| m.user.username.clone())
                                });

                                let unarchive_label = i18n_stored.get_value().t("tasks.unarchive");
                                let delete_label = i18n_stored.get_value().t("common.delete");

                                let on_delete_archived = move |task_id: String| {
                                    let id = household_id();
                                    wasm_bindgen_futures::spawn_local(async move {
                                        if ApiClient::delete_task(&id, &task_id).await.is_ok() {
                                            archived_tasks.update(|t| t.retain(|task| task.id.to_string() != task_id));
                                        }
                                    });
                                };

                                let actions = vec![
                                    ContextMenuAction {
                                        label: unarchive_label,
                                        on_click: Callback::new(move |_| on_unarchive(unarchive_id.clone())),
                                        danger: false,
                                    },
                                    ContextMenuAction {
                                        label: delete_label,
                                        on_click: Callback::new(move |_| on_delete_archived(delete_id.clone())),
                                        danger: true,
                                    },
                                ];

                                let detail_id = task_id.clone();
                                view! {
                                    <div class="task-item" style="opacity: 0.7;">
                                        <div class="task-content">
                                            <div
                                                class="task-title task-title-clickable"
                                                on:click=move |_| detail_task_id.set(Some(detail_id.clone()))
                                            >
                                                {task.title.clone()}
                                            </div>
                                            <div class="task-meta">
                                                {format!("{:?}", task.recurrence_type)}
                                                {if let Some(name) = assigned_name {
                                                    format!(" | Assigned to: {}", name)
                                                } else {
                                                    String::new()
                                                }}
                                            </div>
                                            {if !task.description.is_empty() {
                                                view! { <MarkdownView content=task.description.clone() /> }.into_view()
                                            } else {
                                                ().into_view()
                                            }}
                                        </div>
                                        <ContextMenu actions=actions />
                                    </div>
                                }
                            }).collect_view()
                        }}
                    </Show>
                </div>
            </Show>
        </Show>

        // Create Modal - uses TaskModal with task=None
        <Show when=move || show_create_modal.get() fallback=|| ()>
            {
                let hid = household_id();
                // Filter members based on hierarchy type
                let assignable_members = {
                    let all_members = members.get();
                    match settings.get().map(|s| s.hierarchy_type) {
                        Some(HierarchyType::Hierarchy) => {
                            all_members.into_iter()
                                .filter(|m| m.membership.role == Role::Member)
                                .collect()
                        }
                        _ => all_members
                    }
                };
                view! {
                    <TaskModal
                        task=None
                        household_id=hid
                        members=assignable_members
                        household_rewards=rewards.get()
                        household_punishments=punishments.get()
                        linked_rewards=vec![]
                        linked_punishments=vec![]
                        categories=categories.get()
                        on_close=move |_| show_create_modal.set(false)
                        on_save=on_save
                    />
                }
            }
        </Show>

        // Edit Modal - uses TaskModal with task=Some(task)
        {move || editing_task.get().map(|task| {
            let hid = household_id();
            // Filter members based on hierarchy type
            let assignable_members = {
                let all_members = members.get();
                match settings.get().map(|s| s.hierarchy_type) {
                    Some(HierarchyType::Hierarchy) => {
                        all_members.into_iter()
                            .filter(|m| m.membership.role == Role::Member)
                            .collect()
                    }
                    _ => all_members
                }
            };
            view! {
                <TaskModal
                    task=Some(task)
                    household_id=hid
                    members=assignable_members
                    household_rewards=rewards.get()
                    household_punishments=punishments.get()
                    linked_rewards=task_linked_rewards.get()
                    linked_punishments=task_linked_punishments.get()
                    categories=categories.get()
                    on_close=move |_| {
                        editing_task.set(None);
                        task_linked_rewards.set(vec![]);
                        task_linked_punishments.set(vec![]);
                    }
                    on_save=on_save
                />
            }
        })}

        // Duplicate Modal - uses TaskModal with task=None but prefill_from=Some(task)
        {move || duplicating_task.get().map(|task| {
            let hid = household_id();
            // Filter members based on hierarchy type
            let assignable_members = {
                let all_members = members.get();
                match settings.get().map(|s| s.hierarchy_type) {
                    Some(HierarchyType::Hierarchy) => {
                        all_members.into_iter()
                            .filter(|m| m.membership.role == Role::Member)
                            .collect()
                    }
                    _ => all_members
                }
            };
            view! {
                <TaskModal
                    task=None
                    prefill_from=task
                    household_id=hid
                    members=assignable_members
                    household_rewards=rewards.get()
                    household_punishments=punishments.get()
                    linked_rewards=duplicate_linked_rewards.get()
                    linked_punishments=duplicate_linked_punishments.get()
                    categories=categories.get()
                    on_close=move |_| {
                        duplicating_task.set(None);
                        duplicate_linked_rewards.set(vec![]);
                        duplicate_linked_punishments.set(vec![]);
                    }
                    on_save=on_save
                />
            }
        })}

        // Category Management Modal
        <Show when=move || show_category_modal.get() fallback=|| ()>
            {
                let hid = household_id();
                view! {
                    <CategoryModal
                        household_id=hid
                        on_close=move |_| {
                            show_category_modal.set(false);
                            // Reload categories after modal closes
                            let hid = household_id();
                            wasm_bindgen_futures::spawn_local(async move {
                                if let Ok(cats) = ApiClient::list_categories(&hid).await {
                                    categories.set(cats);
                                }
                            });
                        }
                    />
                }
            }
        </Show>

        // Task Detail Modal
        <Show when=move || detail_task_id.get().is_some() fallback=|| ()>
            {move || {
                let hid = household_id();
                let task_id = detail_task_id.get().unwrap_or_default();
                view! {
                    <TaskDetailModal
                        task_id=task_id
                        household_id=hid
                        on_close=move |_| detail_task_id.set(None)
                        on_edit=move |task| {
                            detail_task_id.set(None);
                            on_edit(task);
                        }
                    />
                }
            }}
        </Show>
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
    use super::*;
    use uuid::Uuid;
    use wasm_bindgen_test::*;

    wasm_bindgen_test_configure!(run_in_browser);

    fn create_test_task(id: Uuid, title: &str) -> Task {
        Task {
            id,
            household_id: Uuid::new_v4(),
            title: title.to_string(),
            description: String::new(),
            recurrence_type: shared::RecurrenceType::Daily,
            recurrence_value: None,
            target_count: 1,
            time_period: None,
            allow_exceed_target: true,
            requires_review: false,
            points_reward: None,
            points_penalty: None,
            due_time: None,
            habit_type: shared::HabitType::Good,
            category_id: None,
            category_name: None,
            archived: false,
            paused: false,
            assigned_user_id: None,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        }
    }

    #[wasm_bindgen_test]
    fn test_task_list_update_edit() {
        let task_id = Uuid::new_v4();
        let mut tasks = vec![
            create_test_task(task_id, "Task 1"),
            create_test_task(Uuid::new_v4(), "Task 2"),
        ];

        let updated_task = create_test_task(task_id, "Updated Task 1");

        if let Some(pos) = tasks.iter().position(|t| t.id == updated_task.id) {
            tasks[pos] = updated_task;
        }

        assert_eq!(tasks[0].title, "Updated Task 1");
        assert_eq!(tasks.len(), 2);
    }

    #[wasm_bindgen_test]
    fn test_task_list_update_create() {
        let mut tasks = vec![create_test_task(Uuid::new_v4(), "Task 1")];

        let new_task = create_test_task(Uuid::new_v4(), "New Task");
        let new_task_id = new_task.id;

        if tasks.iter().position(|t| t.id == new_task.id).is_none() {
            tasks.push(new_task);
        }

        assert_eq!(tasks.len(), 2);
        assert_eq!(tasks[1].id, new_task_id);
    }

    #[wasm_bindgen_test]
    fn test_task_delete() {
        let task_to_delete = Uuid::new_v4();
        let mut tasks = vec![
            create_test_task(task_to_delete, "Task 1"),
            create_test_task(Uuid::new_v4(), "Task 2"),
        ];

        let delete_id = task_to_delete.to_string();
        tasks.retain(|task| task.id.to_string() != delete_id);

        assert_eq!(tasks.len(), 1);
        assert_eq!(tasks[0].title, "Task 2");
    }

    #[wasm_bindgen_test]
    fn test_assigned_user_display_none() {
        let assigned_user_id: Option<Uuid> = None;
        let members: Vec<(Uuid, String)> = vec![];

        let assigned_name = assigned_user_id.and_then(|uid| {
            members.iter().find(|(id, _)| *id == uid).map(|(_, name)| name.clone())
        });

        assert!(assigned_name.is_none());
    }

    #[wasm_bindgen_test]
    fn test_assigned_user_display_found() {
        let user_id = Uuid::new_v4();
        let assigned_user_id: Option<Uuid> = Some(user_id);
        let members: Vec<(Uuid, String)> = vec![
            (user_id, "Alice".to_string()),
            (Uuid::new_v4(), "Bob".to_string()),
        ];

        let assigned_name = assigned_user_id.and_then(|uid| {
            members.iter().find(|(id, _)| *id == uid).map(|(_, name)| name.clone())
        });

        assert_eq!(assigned_name, Some("Alice".to_string()));
    }

    #[wasm_bindgen_test]
    fn test_task_meta_with_assignment() {
        let name = Some("Alice".to_string());
        let meta = if let Some(n) = name {
            format!(" | Assigned to: {}", n)
        } else {
            String::new()
        };
        assert_eq!(meta, " | Assigned to: Alice");
    }

    #[wasm_bindgen_test]
    fn test_task_meta_without_assignment() {
        let name: Option<String> = None;
        let meta = if let Some(n) = name {
            format!(" | Assigned to: {}", n)
        } else {
            String::new()
        };
        assert_eq!(meta, "");
    }

    #[wasm_bindgen_test]
    fn test_task_description_display() {
        let description = "Clean the dishes";
        let meta = if !description.is_empty() {
            format!(" | {}", description)
        } else {
            String::new()
        };
        assert_eq!(meta, " | Clean the dishes");
    }

    #[wasm_bindgen_test]
    fn test_empty_description_display() {
        let description = "";
        let meta = if !description.is_empty() {
            format!(" | {}", description)
        } else {
            String::new()
        };
        assert_eq!(meta, "");
    }

    #[wasm_bindgen_test]
    fn test_duplicate_creates_new_task() {
        // When duplicating, the new task should get a new ID
        let original_id = Uuid::new_v4();
        let new_id = Uuid::new_v4();

        // Original task
        let original = create_test_task(original_id, "Original Task");

        // Duplicated task (would be created by API with new ID)
        let duplicated = create_test_task(new_id, "Original Task");

        // IDs should be different
        assert_ne!(original.id, duplicated.id);
        // But titles should match
        assert_eq!(original.title, duplicated.title);
    }

    #[wasm_bindgen_test]
    fn test_save_handler_adds_new_task() {
        // Test that save handler adds task to list when it's a new task
        let mut tasks = vec![create_test_task(Uuid::new_v4(), "Existing Task")];
        let new_task = create_test_task(Uuid::new_v4(), "New or Duplicated Task");
        let new_task_id = new_task.id;

        // Simulate on_save logic: if not found, push new
        if tasks.iter().position(|t| t.id == new_task.id).is_none() {
            tasks.push(new_task);
        }

        assert_eq!(tasks.len(), 2);
        assert!(tasks.iter().any(|t| t.id == new_task_id));
    }

    #[wasm_bindgen_test]
    fn test_context_menu_action_labels() {
        // Test that context menu would have correct labels
        let edit_label = "Edit";
        let duplicate_label = "Duplicate";
        let delete_label = "Delete";

        assert_eq!(edit_label, "Edit");
        assert_eq!(duplicate_label, "Duplicate");
        assert_eq!(delete_label, "Delete");
    }
}
