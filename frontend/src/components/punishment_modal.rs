use leptos::*;
use shared::{CreatePunishmentRequest, Punishment, UpdatePunishmentRequest};

use crate::api::ApiClient;

#[component]
pub fn PunishmentModal(
    punishment: Option<Punishment>,
    household_id: String,
    #[prop(into)] on_close: Callback<()>,
    #[prop(into)] on_save: Callback<Punishment>,
) -> impl IntoView {
    let is_edit = punishment.is_some();
    let error = create_rw_signal(Option::<String>::None);
    let saving = create_rw_signal(false);

    // Form fields - initialize based on mode
    let name = create_rw_signal(punishment.as_ref().map(|p| p.name.clone()).unwrap_or_default());
    let description = create_rw_signal(punishment.as_ref().map(|p| p.description.clone()).unwrap_or_default());
    let requires_confirmation = create_rw_signal(punishment.as_ref().map(|p| p.requires_confirmation).unwrap_or(false));

    let punishment_id = punishment.as_ref().map(|p| p.id.to_string());

    let on_submit = {
        let punishment_id = punishment_id.clone();
        let household_id = household_id.clone();

        move |ev: web_sys::SubmitEvent| {
            ev.prevent_default();
            saving.set(true);
            error.set(None);

            let punishment_id = punishment_id.clone();
            let household_id = household_id.clone();

            wasm_bindgen_futures::spawn_local(async move {
                if let Some(punishment_id) = punishment_id {
                    // Edit mode - update existing punishment
                    let request = UpdatePunishmentRequest {
                        name: Some(name.get()),
                        description: Some(description.get()),
                        requires_confirmation: Some(requires_confirmation.get()),
                    };

                    match ApiClient::update_punishment(&household_id, &punishment_id, request).await {
                        Ok(updated_punishment) => {
                            saving.set(false);
                            on_save.call(updated_punishment);
                        }
                        Err(e) => {
                            error.set(Some(e));
                            saving.set(false);
                        }
                    }
                } else {
                    // Create mode - create new punishment
                    let request = CreatePunishmentRequest {
                        name: name.get(),
                        description: Some(description.get()),
                        requires_confirmation: Some(requires_confirmation.get()),
                    };

                    match ApiClient::create_punishment(&household_id, request).await {
                        Ok(created_punishment) => {
                            saving.set(false);
                            on_save.call(created_punishment);
                        }
                        Err(e) => {
                            error.set(Some(e));
                            saving.set(false);
                        }
                    }
                }
            });
        }
    };

    let close = move |_| on_close.call(());

    let modal_title = if is_edit { "Edit Punishment" } else { "Create Punishment" };
    let submit_button_text = if is_edit { "Save Changes" } else { "Create" };
    let saving_text = if is_edit { "Saving..." } else { "Creating..." };

    view! {
        <div class="modal-backdrop" on:click=close>
            <div class="modal" on:click=|e| e.stop_propagation()>
                <div class="modal-header">
                    <h3 class="modal-title">{modal_title}</h3>
                    <button class="modal-close" on:click=close>"×"</button>
                </div>

                {move || error.get().map(|e| view! {
                    <div class="alert alert-error" style="margin: 1rem;">{e}</div>
                })}

                <form on:submit=on_submit>
                    <div style="padding: 1rem;">
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

                        <div class="form-group">
                            <label style="display: flex; align-items: center; gap: 0.5rem; cursor: pointer;">
                                <input
                                    type="checkbox"
                                    prop:checked=move || requires_confirmation.get()
                                    on:change=move |ev| requires_confirmation.set(event_target_checked(&ev))
                                />
                                <span>"Requires owner confirmation to complete"</span>
                            </label>
                        </div>
                    </div>

                    <div class="modal-footer">
                        <button
                            type="button"
                            class="btn btn-outline"
                            on:click=move |_| on_close.call(())
                            disabled=move || saving.get()
                        >
                            "Cancel"
                        </button>
                        <button
                            type="submit"
                            class="btn btn-primary"
                            disabled=move || saving.get()
                        >
                            {move || if saving.get() { saving_text } else { submit_button_text }}
                        </button>
                    </div>
                </form>
            </div>
        </div>
    }
}
