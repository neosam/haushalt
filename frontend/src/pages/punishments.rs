use leptos::*;
use leptos_router::*;
use shared::{CreatePunishmentRequest, MemberWithUser, Punishment, UserPunishment, UserPunishmentWithUser};

use crate::api::ApiClient;
use crate::components::loading::Loading;
use crate::components::modal::Modal;

#[component]
pub fn PunishmentsPage() -> impl IntoView {
    let params = use_params_map();
    let household_id = move || params.with(|p| p.get("id").cloned().unwrap_or_default());

    let punishments = create_rw_signal(Vec::<Punishment>::new());
    let my_punishments = create_rw_signal(Vec::<UserPunishment>::new());
    let all_user_punishments = create_rw_signal(Vec::<UserPunishmentWithUser>::new());
    let members = create_rw_signal(Vec::<MemberWithUser>::new());
    let loading = create_rw_signal(true);
    let error = create_rw_signal(Option::<String>::None);
    let show_create_modal = create_rw_signal(false);

    // Form fields
    let name = create_rw_signal(String::new());
    let description = create_rw_signal(String::new());

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
    });

    let on_create = move |ev: web_sys::SubmitEvent| {
        ev.prevent_default();

        let id = household_id();

        let request = CreatePunishmentRequest {
            name: name.get(),
            description: Some(description.get()),
        };

        wasm_bindgen_futures::spawn_local(async move {
            match ApiClient::create_punishment(&id, request).await {
                Ok(punishment) => {
                    punishments.update(|p| p.push(punishment));
                    show_create_modal.set(false);
                    name.set(String::new());
                    description.set(String::new());
                }
                Err(e) => error.set(Some(e)),
            }
        });
    };

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

    view! {
        <div class="dashboard-header">
            <h1 class="dashboard-title">"Punishments"</h1>
            <a href=move || format!("/households/{}", household_id()) style="color: var(--text-muted);">
                "← Back to household"
            </a>
        </div>

        {move || error.get().map(|e| view! {
            <div class="alert alert-error">{e}</div>
        })}

        <Show when=move || loading.get() fallback=|| ()>
            <Loading />
        </Show>

        <Show when=move || !loading.get() fallback=|| ()>
            // My Punishments Section
            <Show when=move || !my_punishments.get().is_empty() fallback=|| ()>
                <div class="card" style="margin-bottom: 1.5rem; border-left: 4px solid var(--error-color);">
                    <div class="card-header">
                        <h3 class="card-title">"My Punishments"</h3>
                    </div>
                    {move || {
                        let all_punishments = punishments.get();
                        my_punishments.get().into_iter().map(|user_punishment| {
                            let punishment_name = all_punishments.iter()
                                .find(|p| p.id == user_punishment.punishment_id)
                                .map(|p| p.name.clone())
                                .unwrap_or_else(|| "Unknown Punishment".to_string());
                            let punishment_desc = all_punishments.iter()
                                .find(|p| p.id == user_punishment.punishment_id)
                                .map(|p| p.description.clone())
                                .unwrap_or_default();
                            let pending = user_punishment.amount - user_punishment.completed_amount;
                            view! {
                                <div style="display: flex; justify-content: space-between; align-items: center; padding: 0.75rem; border-bottom: 1px solid var(--border-color);">
                                    <div>
                                        <div style="font-weight: 600;">{punishment_name}</div>
                                        <div style="font-size: 0.75rem; color: var(--text-muted);">
                                            {format!("{} pending, {} completed", pending, user_punishment.completed_amount)}
                                            {if !punishment_desc.is_empty() { format!(" • {}", punishment_desc) } else { String::new() }}
                                        </div>
                                    </div>
                                    {if pending == 0 {
                                        view! {
                                            <span class="badge" style="background: var(--success-color); color: white;">"All Completed"</span>
                                        }.into_view()
                                    } else {
                                        view! {
                                            <span class="badge" style="background: var(--warning-color); color: white;">{pending} " Pending"</span>
                                        }.into_view()
                                    }}
                                </div>
                            }
                        }).collect_view()
                    }}
                </div>
            </Show>

            <div style="margin-bottom: 1rem;">
                <button class="btn btn-primary" on:click=move |_| show_create_modal.set(true)>
                    "+ Create Punishment"
                </button>
            </div>

            <h3 style="margin-bottom: 1rem; color: var(--text-muted);">"Punishment Definitions"</h3>

            {move || {
                let p = punishments.get();
                let user_punishments = all_user_punishments.get();
                let member_list = members.get();

                if p.is_empty() {
                    view! {
                        <div class="card empty-state">
                            <p>"No punishments yet."</p>
                            <p>"Create punishments for missed tasks!"</p>
                        </div>
                    }.into_view()
                } else {
                    view! {
                        <div class="grid grid-3">
                            {p.into_iter().map(|punishment| {
                                let punishment_id = punishment.id;
                                let punishment_id_str = punishment_id.to_string();
                                let delete_id = punishment_id_str.clone();

                                // Get user assignments for this punishment (now one row per user with amount)
                                let user_assignments: Vec<_> = user_punishments.iter()
                                    .filter(|up| up.user_punishment.punishment_id == punishment_id)
                                    .map(|up| (up.user_punishment.user_id, up.user.username.clone(), up.user_punishment.amount))
                                    .collect();

                                view! {
                                    <div class="card">
                                        <h3 class="card-title">{punishment.name.clone()}</h3>
                                        <p style="color: var(--text-muted); font-size: 0.875rem; margin-bottom: 0.5rem;">
                                            {punishment.description.clone()}
                                        </p>

                                        // Assignments section
                                        <div style="border-top: 1px solid var(--border-color); padding-top: 0.5rem; margin-top: 0.5rem;">
                                            <div style="font-size: 0.75rem; color: var(--text-muted); margin-bottom: 0.25rem;">"Assignments:"</div>
                                            {if user_assignments.is_empty() {
                                                view! {
                                                    <div style="font-size: 0.75rem; color: var(--text-muted); font-style: italic;">"None"</div>
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

                                            // Add assignment for members without any
                                            {
                                                let assigned_user_ids: std::collections::HashSet<_> = user_punishments.iter()
                                                    .filter(|up| up.user_punishment.punishment_id == punishment_id)
                                                    .map(|up| up.user_punishment.user_id)
                                                    .collect();
                                                let unassigned_members: Vec<_> = member_list.iter()
                                                    .filter(|m| !assigned_user_ids.contains(&m.user.id))
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

                                        <button
                                            class="btn btn-danger"
                                            style="width: 100%; margin-top: 0.5rem; padding: 0.25rem 0.5rem; font-size: 0.75rem;"
                                            on:click=move |_| on_delete(delete_id.clone())
                                        >
                                            "Delete"
                                        </button>
                                    </div>
                                }
                            }).collect_view()}
                        </div>
                    }.into_view()
                }
            }}
        </Show>

        <Show when=move || show_create_modal.get() fallback=|| ()>
            <Modal title="Create Punishment" on_close=move |_| show_create_modal.set(false)>
                <form on:submit=on_create>
                    <div class="form-group">
                        <label class="form-label" for="punishment-name">"Name"</label>
                        <input
                            type="text"
                            id="punishment-name"
                            class="form-input"
                            placeholder="e.g., Extra Chores"
                            prop:value=move || name.get()
                            on:input=move |ev| name.set(event_target_value(&ev))
                            required
                        />
                    </div>

                    <div class="form-group">
                        <label class="form-label" for="punishment-description">"Description"</label>
                        <input
                            type="text"
                            id="punishment-description"
                            class="form-input"
                            placeholder="What happens?"
                            prop:value=move || description.get()
                            on:input=move |ev| description.set(event_target_value(&ev))
                        />
                    </div>

                    <div class="modal-footer">
                        <button type="button" class="btn btn-outline" on:click=move |_| show_create_modal.set(false)>
                            "Cancel"
                        </button>
                        <button type="submit" class="btn btn-primary">
                            "Create"
                        </button>
                    </div>
                </form>
            </Modal>
        </Show>
    }
}
