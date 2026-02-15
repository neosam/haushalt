use leptos::*;
use leptos_router::*;
use shared::{CreatePunishmentRequest, Punishment, UserPunishment, UserPunishmentWithUser};

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
                            let is_completed = user_punishment.completed;
                            view! {
                                <div style="display: flex; justify-content: space-between; align-items: center; padding: 0.75rem; border-bottom: 1px solid var(--border-color);">
                                    <div>
                                        <div style="font-weight: 600;">{punishment_name}</div>
                                        <div style="font-size: 0.75rem; color: var(--text-muted);">
                                            {if is_completed { "Completed" } else { "Pending" }}
                                            {if !punishment_desc.is_empty() { format!(" • {}", punishment_desc) } else { String::new() }}
                                        </div>
                                    </div>
                                    {if is_completed {
                                        view! {
                                            <span class="badge" style="background: var(--success-color); color: white;">"Completed"</span>
                                        }.into_view()
                                    } else {
                                        view! {
                                            <span class="badge" style="background: var(--warning-color); color: white;">"Pending"</span>
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
                                let punishment_id = punishment.id.to_string();
                                let delete_id = punishment_id.clone();
                                view! {
                                    <div class="card">
                                        <h3 class="card-title">{punishment.name}</h3>
                                        <p style="color: var(--text-muted); font-size: 0.875rem; margin-bottom: 1rem;">
                                            {punishment.description}
                                        </p>
                                        <button
                                            class="btn btn-danger"
                                            style="width: 100%; padding: 0.25rem 0.5rem; font-size: 0.75rem;"
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

            // All Assigned Punishments Section
            <Show when=move || !all_user_punishments.get().is_empty() fallback=|| ()>
                <h3 style="margin-top: 2rem; margin-bottom: 1rem; color: var(--text-muted);">"Assigned Punishments (All Members)"</h3>
                <div class="card">
                    {move || {
                        let all_punishments = punishments.get();
                        all_user_punishments.get().into_iter().map(|user_punishment_with_user| {
                            let punishment_name = all_punishments.iter()
                                .find(|p| p.id == user_punishment_with_user.user_punishment.punishment_id)
                                .map(|p| p.name.clone())
                                .unwrap_or_else(|| "Unknown Punishment".to_string());
                            let is_completed = user_punishment_with_user.user_punishment.completed;
                            view! {
                                <div style="display: flex; justify-content: space-between; align-items: center; padding: 0.75rem; border-bottom: 1px solid var(--border-color);">
                                    <div>
                                        <div style="font-weight: 600;">{punishment_name}</div>
                                        <div style="font-size: 0.75rem; color: var(--text-muted);">
                                            "Assigned to: " {user_punishment_with_user.user.username}
                                        </div>
                                    </div>
                                    {if is_completed {
                                        view! {
                                            <span class="badge" style="background: var(--success-color); color: white;">"Completed"</span>
                                        }.into_view()
                                    } else {
                                        view! {
                                            <span class="badge" style="background: var(--warning-color); color: white;">"Pending"</span>
                                        }.into_view()
                                    }}
                                </div>
                            }
                        }).collect_view()
                    }}
                </div>
            </Show>
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
