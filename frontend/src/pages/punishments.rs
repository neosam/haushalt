use leptos::*;
use leptos_router::*;
use shared::{CreatePunishmentRequest, Punishment};

use crate::api::ApiClient;
use crate::components::loading::Loading;
use crate::components::modal::Modal;

#[component]
pub fn PunishmentsPage() -> impl IntoView {
    let params = use_params_map();
    let household_id = move || params.with(|p| p.get("id").cloned().unwrap_or_default());

    let punishments = create_rw_signal(Vec::<Punishment>::new());
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

        wasm_bindgen_futures::spawn_local(async move {
            match ApiClient::list_punishments(&id).await {
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
            <div style="margin-bottom: 1rem;">
                <button class="btn btn-primary" on:click=move |_| show_create_modal.set(true)>
                    "+ Create Punishment"
                </button>
            </div>

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
