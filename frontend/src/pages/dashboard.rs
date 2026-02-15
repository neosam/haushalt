use leptos::*;
use shared::{CreateHouseholdRequest, Household};

use crate::api::ApiClient;
use crate::components::loading::Loading;
use crate::components::modal::Modal;

#[component]
pub fn Dashboard() -> impl IntoView {
    let households = create_rw_signal(Vec::<Household>::new());
    let loading = create_rw_signal(true);
    let error = create_rw_signal(Option::<String>::None);
    let show_create_modal = create_rw_signal(false);
    let new_household_name = create_rw_signal(String::new());

    // Load households on mount
    create_effect(move |_| {
        wasm_bindgen_futures::spawn_local(async move {
            match ApiClient::list_households().await {
                Ok(data) => {
                    households.set(data);
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

    view! {
        <div class="dashboard-header">
            <h1 class="dashboard-title">"Your Households"</h1>
            <p class="dashboard-subtitle">"Manage your households and tasks"</p>
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
                    "+ Create Household"
                </button>
            </div>

            {move || {
                let h = households.get();
                if h.is_empty() {
                    view! {
                        <div class="card empty-state">
                            <p>"You don't have any households yet."</p>
                            <p>"Create one to get started!"</p>
                        </div>
                    }.into_view()
                } else {
                    view! {
                        <div class="grid grid-3">
                            {h.into_iter().map(|household| {
                                let id = household.id.to_string();
                                view! {
                                    <a href=format!("/households/{}", id) style="text-decoration: none;">
                                        <div class="card" style="cursor: pointer; transition: transform 0.2s;">
                                            <h3 class="card-title">{household.name}</h3>
                                            <p style="color: var(--text-muted); font-size: 0.875rem;">
                                                "Click to manage"
                                            </p>
                                        </div>
                                    </a>
                                }
                            }).collect_view()}
                        </div>
                    }.into_view()
                }
            }}
        </Show>

        <Show when=move || show_create_modal.get() fallback=|| ()>
            <Modal title="Create Household" on_close=move |_| show_create_modal.set(false)>
                <form on:submit=on_create>
                    <div class="form-group">
                        <label class="form-label" for="household-name">"Household Name"</label>
                        <input
                            type="text"
                            id="household-name"
                            class="form-input"
                            placeholder="e.g., Smith Family"
                            prop:value=move || new_household_name.get()
                            on:input=move |ev| new_household_name.set(event_target_value(&ev))
                            required
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
