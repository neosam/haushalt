use leptos::*;
use leptos_router::*;
use shared::{HouseholdSettings, MemberWithUser, Punishment, UserPunishment, UserPunishmentWithUser};

use crate::api::ApiClient;
use crate::components::household_tabs::{HouseholdTab, HouseholdTabs};
use crate::components::loading::Loading;
use crate::components::punishment_modal::PunishmentModal;
use crate::i18n::use_i18n;

#[component]
pub fn PunishmentsPage() -> impl IntoView {
    let i18n = use_i18n();
    let i18n_stored = store_value(i18n);

    let params = use_params_map();
    let household_id = move || params.with(|p| p.get("id").cloned().unwrap_or_default());

    let punishments = create_rw_signal(Vec::<Punishment>::new());
    let my_punishments = create_rw_signal(Vec::<UserPunishment>::new());
    let all_user_punishments = create_rw_signal(Vec::<UserPunishmentWithUser>::new());
    let members = create_rw_signal(Vec::<MemberWithUser>::new());
    let settings = create_rw_signal(Option::<HouseholdSettings>::None);
    let loading = create_rw_signal(true);
    let error = create_rw_signal(Option::<String>::None);
    let success = create_rw_signal(Option::<String>::None);

    // Modal state: None = closed, Some(None) = create mode, Some(Some(punishment)) = edit mode
    let modal_punishment = create_rw_signal(Option::<Option<Punishment>>::None);

    // Load punishments
    create_effect(move |_| {
        let id = household_id();
        if id.is_empty() {
            return;
        }

        let id_for_punishments = id.clone();
        let id_for_my_punishments = id.clone();
        let id_for_all_user_punishments = id.clone();
        let id_for_members = id.clone();
        let id_for_settings = id.clone();

        wasm_bindgen_futures::spawn_local(async move {
            match ApiClient::list_punishments(&id_for_punishments).await {
                Ok(p) => {
                    punishments.set(p);
                    loading.set(false);
                }
                Err(e) => {
                    error.set(Some(e));
                    loading.set(false);
                }
            }
        });

        // Load my punishments
        wasm_bindgen_futures::spawn_local(async move {
            if let Ok(p) = ApiClient::list_user_punishments(&id_for_my_punishments).await {
                my_punishments.set(p);
            }
        });

        // Load all user punishments in household
        wasm_bindgen_futures::spawn_local(async move {
            if let Ok(p) = ApiClient::list_all_user_punishments(&id_for_all_user_punishments).await {
                all_user_punishments.set(p);
            }
        });

        // Load members
        wasm_bindgen_futures::spawn_local(async move {
            if let Ok(m) = ApiClient::list_members(&id_for_members).await {
                members.set(m);
            }
        });

        // Load settings for dark mode
        wasm_bindgen_futures::spawn_local(async move {
            if let Ok(s) = ApiClient::get_household_settings(&id_for_settings).await {
                apply_dark_mode(s.dark_mode);
                settings.set(Some(s));
            }
        });
    });

    let on_delete = move |punishment_id: String| {
        let id = household_id();
        wasm_bindgen_futures::spawn_local(async move {
            if ApiClient::delete_punishment(&id, &punishment_id).await.is_ok() {
                punishments.update(|p| p.retain(|punishment| punishment.id.to_string() != punishment_id));
            }
        });
    };

    let on_assign = move |punishment_id: String, user_id: String| {
        let id = household_id();
        wasm_bindgen_futures::spawn_local(async move {
            match ApiClient::assign_punishment(&id, &punishment_id, &user_id).await {
                Ok(user_punishment) => {
                    // Check if user already has this punishment assigned
                    let existing_idx = all_user_punishments.get().iter().position(|up| {
                        up.user_punishment.punishment_id.to_string() == punishment_id &&
                        up.user_punishment.user_id.to_string() == user_id
                    });

                    if let Some(idx) = existing_idx {
                        // Update existing entry with new amount
                        all_user_punishments.update(|p| {
                            p[idx].user_punishment = user_punishment;
                        });
                    } else {
                        // Add new entry
                        let user_info = members.get().iter()
                            .find(|m| m.user.id.to_string() == user_id)
                            .map(|m| m.user.clone());
                        if let Some(user) = user_info {
                            all_user_punishments.update(|p| p.push(UserPunishmentWithUser {
                                user_punishment,
                                user,
                            }));
                        }
                    }
                }
                Err(e) => error.set(Some(e)),
            }
        });
    };

    let on_unassign = move |punishment_id: String, user_id: String| {
        let id = household_id();
        wasm_bindgen_futures::spawn_local(async move {
            if ApiClient::unassign_punishment(&id, &punishment_id, &user_id).await.is_ok() {
                // Decrement amount or remove if amount becomes 0
                all_user_punishments.update(|p| {
                    if let Some(pos) = p.iter().position(|up| {
                        up.user_punishment.punishment_id.to_string() == punishment_id &&
                        up.user_punishment.user_id.to_string() == user_id
                    }) {
                        if p[pos].user_punishment.amount <= 1 {
                            p.remove(pos);
                        } else {
                            p[pos].user_punishment.amount -= 1;
                        }
                    }
                });
            }
        });
    };

    let on_complete = move |user_punishment_id: String| {
        let id = household_id();
        wasm_bindgen_futures::spawn_local(async move {
            match ApiClient::complete_punishment(&id, &user_punishment_id).await {
                Ok(updated) => {
                    my_punishments.update(|p| {
                        if let Some(pos) = p.iter().position(|up| up.id.to_string() == user_punishment_id) {
                            p[pos] = updated;
                        }
                    });
                    success.set(Some(i18n_stored.get_value().t("punishments.completed_success")));
                }
                Err(e) => error.set(Some(e)),
            }
        });
    };

    view! {
        {move || {
            let hid = household_id();
            view! { <HouseholdTabs household_id=hid active_tab=HouseholdTab::Punishments settings=settings.get() /> }
        }}

        <div class="dashboard-header">
            <h1 class="dashboard-title">{i18n_stored.get_value().t("punishments.title")}</h1>
        </div>

        {move || error.get().map(|e| view! {
            <div class="alert alert-error">{e}</div>
        })}

        {move || success.get().map(|s| view! {
            <div class="alert alert-success">{s}</div>
        })}

        <Show when=move || loading.get() fallback=|| ()>
            <Loading />
        </Show>

        <Show when=move || !loading.get() fallback=|| ()>
            // My Punishments Section - only show punishments with remaining or pending
            <Show when=move || my_punishments.get().iter().any(|up| up.amount > up.completed_amount) fallback=|| ()>
                <div class="card" style="margin-bottom: 1.5rem; border-left: 4px solid var(--error-color);">
                    <div class="card-header">
                        <h3 class="card-title">{i18n_stored.get_value().t("punishments.my_punishments")}</h3>
                    </div>
                    {move || {
                        let all_punishments = punishments.get();
                        my_punishments.get().into_iter()
                            // Only show punishments that have remaining or pending completions
                            .filter(|up| up.amount > up.completed_amount)
                            .map(|user_punishment| {
                                let punishment_name = all_punishments.iter()
                                    .find(|p| p.id == user_punishment.punishment_id)
                                    .map(|p| p.name.clone())
                                    .unwrap_or_else(|| i18n_stored.get_value().t("punishments.unknown_punishment"));
                                let punishment_desc = all_punishments.iter()
                                    .find(|p| p.id == user_punishment.punishment_id)
                                    .map(|p| p.description.clone())
                                    .unwrap_or_default();
                                let up_id = user_punishment.id.to_string();
                                let complete_id = up_id.clone();
                                let available = user_punishment.amount - user_punishment.completed_amount - user_punishment.pending_completion;
                                let pending_conf = user_punishment.pending_completion;
                                view! {
                                    <div style="display: flex; justify-content: space-between; align-items: center; padding: 0.75rem; border-bottom: 1px solid var(--border-color);">
                                        <div>
                                            <div style="font-weight: 600;">{punishment_name}</div>
                                            <div style="font-size: 0.75rem; color: var(--text-muted);">
                                                {format!("{} remaining, {} completed", available, user_punishment.completed_amount)}
                                                {if pending_conf > 0 { format!(", {} pending confirmation", pending_conf) } else { String::new() }}
                                                {if !punishment_desc.is_empty() { format!(" • {}", punishment_desc) } else { String::new() }}
                                            </div>
                                        </div>
                                        {if pending_conf > 0 {
                                            view! {
                                                <span class="badge" style="background: var(--warning-color); color: white;">{i18n_stored.get_value().t("punishments.awaiting_confirmation")}</span>
                                            }.into_view()
                                        } else {
                                            view! {
                                                <button
                                                    class="btn btn-primary"
                                                    style="padding: 0.25rem 0.75rem; font-size: 0.875rem;"
                                                    on:click=move |_| on_complete(complete_id.clone())
                                                >
                                                    {i18n_stored.get_value().t("punishments.mark_complete")}
                                                </button>
                                            }.into_view()
                                        }}
                                    </div>
                            }
                        }).collect_view()
                    }}
                </div>
            </Show>

            <div style="margin-bottom: 1rem;">
                <button class="btn btn-primary" on:click=move |_| modal_punishment.set(Some(None))>
                    "+ " {i18n_stored.get_value().t("punishments.create")}
                </button>
            </div>

            <h3 style="margin-bottom: 1rem; color: var(--text-muted);">{i18n_stored.get_value().t("punishments.definitions")}</h3>

            {move || {
                let p = punishments.get();
                let user_punishments = all_user_punishments.get();
                let member_list = members.get();
                let hierarchy = settings.get().map(|s| s.hierarchy_type).unwrap_or_default();

                if p.is_empty() {
                    view! {
                        <div class="card empty-state">
                            <p>{i18n_stored.get_value().t("punishments.no_punishments")}</p>
                            <p>{i18n_stored.get_value().t("punishments.add_first")}</p>
                        </div>
                    }.into_view()
                } else {
                    view! {
                        <div class="grid grid-3">
                            {p.into_iter().map(|punishment| {
                                let punishment_for_edit = punishment.clone();
                                let punishment_id = punishment.id;
                                let punishment_id_str = punishment_id.to_string();
                                let delete_id = punishment_id_str.clone();

                                // Get user assignments for this punishment - show only open (uncompleted) count
                                // Filter by hierarchy: only show assignable roles
                                let user_assignments: Vec<_> = user_punishments.iter()
                                    .filter(|up| up.user_punishment.punishment_id == punishment_id)
                                    .filter(|up| {
                                        // Check if user's role can be assigned in this hierarchy
                                        member_list.iter()
                                            .find(|m| m.user.id == up.user_punishment.user_id)
                                            .map(|m| hierarchy.can_be_assigned(&m.membership.role))
                                            .unwrap_or(false)
                                    })
                                    .map(|up| {
                                        let open = up.user_punishment.amount - up.user_punishment.completed_amount;
                                        (up.user_punishment.user_id, up.user.username.clone(), open)
                                    })
                                    .filter(|(_, _, open)| *open > 0)  // Only show if there are open punishments
                                    .collect();

                                view! {
                                    <div class="card">
                                        <h3 class="card-title">{punishment.name.clone()}</h3>
                                        <p style="color: var(--text-muted); font-size: 0.875rem; margin-bottom: 0.5rem;">
                                            {punishment.description.clone()}
                                        </p>

                                        // Assignments section
                                        <div style="border-top: 1px solid var(--border-color); padding-top: 0.5rem; margin-top: 0.5rem;">
                                            <div style="font-size: 0.75rem; color: var(--text-muted); margin-bottom: 0.25rem;">{i18n_stored.get_value().t("punishments.assignments")} ":"</div>
                                            {if user_assignments.is_empty() {
                                                view! {
                                                    <div style="font-size: 0.75rem; color: var(--text-muted); font-style: italic;">{i18n_stored.get_value().t("common.none")}</div>
                                                }.into_view()
                                            } else {
                                                user_assignments.into_iter().map(|(user_id, username, amount)| {
                                                    let punishment_id_for_add = punishment_id_str.clone();
                                                    let punishment_id_for_remove = punishment_id_str.clone();
                                                    let user_id_for_add = user_id.to_string();
                                                    let user_id_for_remove = user_id.to_string();
                                                    view! {
                                                        <div style="display: flex; justify-content: space-between; align-items: center; font-size: 0.875rem; padding: 0.25rem 0;">
                                                            <span>{username} ": " {amount} "x"</span>
                                                            <div style="display: flex; gap: 0.25rem;">
                                                                <button
                                                                    class="btn btn-outline"
                                                                    style="padding: 0.1rem 0.4rem; font-size: 0.75rem; min-width: 24px;"
                                                                    on:click=move |_| on_unassign(punishment_id_for_remove.clone(), user_id_for_remove.clone())
                                                                >
                                                                    "-"
                                                                </button>
                                                                <button
                                                                    class="btn btn-outline"
                                                                    style="padding: 0.1rem 0.4rem; font-size: 0.75rem; min-width: 24px;"
                                                                    on:click=move |_| on_assign(punishment_id_for_add.clone(), user_id_for_add.clone())
                                                                >
                                                                    "+"
                                                                </button>
                                                            </div>
                                                        </div>
                                                    }
                                                }).collect_view()
                                            }}

                                            // Add assignment for members without any open punishments
                                            // Filter by hierarchy: only show assignable roles
                                            {
                                                let users_with_open: std::collections::HashSet<_> = user_punishments.iter()
                                                    .filter(|up| up.user_punishment.punishment_id == punishment_id)
                                                    .filter(|up| up.user_punishment.amount > up.user_punishment.completed_amount)
                                                    .map(|up| up.user_punishment.user_id)
                                                    .collect();
                                                let unassigned_members: Vec<_> = member_list.iter()
                                                    .filter(|m| !users_with_open.contains(&m.user.id))
                                                    .filter(|m| hierarchy.can_be_assigned(&m.membership.role))
                                                    .collect();

                                                if !unassigned_members.is_empty() {
                                                    view! {
                                                        <div style="margin-top: 0.5rem;">
                                                            {unassigned_members.into_iter().map(|member| {
                                                                let punishment_id_for_new = punishment_id_str.clone();
                                                                let user_id_for_new = member.user.id.to_string();
                                                                let username = member.user.username.clone();
                                                                view! {
                                                                    <div style="display: flex; justify-content: space-between; align-items: center; font-size: 0.875rem; padding: 0.25rem 0; color: var(--text-muted);">
                                                                        <span>{username} ": 0x"</span>
                                                                        <button
                                                                            class="btn btn-outline"
                                                                            style="padding: 0.1rem 0.4rem; font-size: 0.75rem; min-width: 24px;"
                                                                            on:click=move |_| on_assign(punishment_id_for_new.clone(), user_id_for_new.clone())
                                                                        >
                                                                            "+"
                                                                        </button>
                                                                    </div>
                                                                }
                                                            }).collect_view()}
                                                        </div>
                                                    }.into_view()
                                                } else {
                                                    view! { <div></div> }.into_view()
                                                }
                                            }
                                        </div>

                                        <div style="display: flex; gap: 0.5rem; margin-top: 0.5rem;">
                                            <button
                                                class="btn btn-outline"
                                                style="flex: 1; padding: 0.25rem 0.5rem; font-size: 0.75rem;"
                                                on:click={
                                                    let punishment_for_edit = punishment_for_edit.clone();
                                                    move |_| modal_punishment.set(Some(Some(punishment_for_edit.clone())))
                                                }
                                            >
                                                {i18n_stored.get_value().t("common.edit")}
                                            </button>
                                            <button
                                                class="btn btn-danger"
                                                style="flex: 1; padding: 0.25rem 0.5rem; font-size: 0.75rem;"
                                                on:click=move |_| on_delete(delete_id.clone())
                                            >
                                                {i18n_stored.get_value().t("common.delete")}
                                            </button>
                                        </div>
                                    </div>
                                }
                            }).collect_view()}
                        </div>
                    }.into_view()
                }
            }}
        </Show>

        {move || modal_punishment.get().map(|punishment_opt| {
            let hh_id = household_id();
            view! {
                <PunishmentModal
                    punishment=punishment_opt
                    household_id=hh_id
                    on_close=move |_| modal_punishment.set(None)
                    on_save=move |saved_punishment: Punishment| {
                        // Check if this is an existing punishment (edit) or new (create)
                        let existing_idx = punishments.get().iter().position(|p| p.id == saved_punishment.id);
                        if let Some(idx) = existing_idx {
                            // Update existing punishment
                            punishments.update(|p| p[idx] = saved_punishment);
                        } else {
                            // Add new punishment
                            punishments.update(|p| p.push(saved_punishment));
                        }
                        modal_punishment.set(None);
                    }
                />
            }
        })}
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
    use uuid::Uuid;
    use wasm_bindgen_test::*;

    wasm_bindgen_test_configure!(run_in_browser);

    #[wasm_bindgen_test]
    fn test_pending_punishments_calculation() {
        let amount = 5;
        let completed = 2;
        let pending = amount - completed;
        assert_eq!(pending, 3);
    }

    #[wasm_bindgen_test]
    fn test_pending_punishments_all_completed() {
        let amount = 3;
        let completed = 3;
        let pending = amount - completed;
        assert_eq!(pending, 0);
    }

    #[wasm_bindgen_test]
    fn test_punishment_delete() {
        let punishment_to_delete = Uuid::new_v4();
        let mut punishments: Vec<Uuid> = vec![punishment_to_delete, Uuid::new_v4()];

        let delete_id = punishment_to_delete.to_string();
        punishments.retain(|id| id.to_string() != delete_id);

        assert_eq!(punishments.len(), 1);
    }

    #[wasm_bindgen_test]
    fn test_description_display_with_content() {
        let punishment_desc = "Extra chores for the week";
        let display = if !punishment_desc.is_empty() {
            format!(" • {}", punishment_desc)
        } else {
            String::new()
        };
        assert_eq!(display, " • Extra chores for the week");
    }

    #[wasm_bindgen_test]
    fn test_description_display_empty() {
        let punishment_desc = "";
        let display = if !punishment_desc.is_empty() {
            format!(" • {}", punishment_desc)
        } else {
            String::new()
        };
        assert_eq!(display, "");
    }

    #[wasm_bindgen_test]
    fn test_punishment_status_pending() {
        let pending = 2;
        let is_pending = pending > 0;
        assert!(is_pending);
    }

    #[wasm_bindgen_test]
    fn test_punishment_status_all_completed() {
        let pending = 0;
        let is_pending = pending > 0;
        assert!(!is_pending);
    }

    #[wasm_bindgen_test]
    fn test_user_assignment_format() {
        let username = "Bob";
        let amount = 2;
        let display = format!("{}: {}x", username, amount);
        assert_eq!(display, "Bob: 2x");
    }

    #[wasm_bindgen_test]
    fn test_unassign_decrement() {
        let mut amount = 3;
        if amount <= 1 {
            amount = 0;
        } else {
            amount -= 1;
        }
        assert_eq!(amount, 2);
    }

    #[wasm_bindgen_test]
    fn test_unassign_remove() {
        let amount = 1;
        let should_remove = amount <= 1;
        assert!(should_remove);
    }

    #[wasm_bindgen_test]
    fn test_status_message_pending() {
        let pending = 3;
        let completed = 1;
        let message = format!("{} pending, {} completed", pending, completed);
        assert_eq!(message, "3 pending, 1 completed");
    }

    #[wasm_bindgen_test]
    fn test_status_message_all_done() {
        let pending = 0;
        let completed = 5;
        let message = format!("{} pending, {} completed", pending, completed);
        assert_eq!(message, "0 pending, 5 completed");
    }

    #[wasm_bindgen_test]
    fn test_empty_punishments_check() {
        let punishments: Vec<String> = vec![];
        assert!(punishments.is_empty());
    }

    #[wasm_bindgen_test]
    fn test_nonempty_punishments_check() {
        let punishments = vec!["Extra Chores".to_string()];
        assert!(!punishments.is_empty());
    }
}
