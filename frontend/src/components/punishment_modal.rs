use leptos::*;
use shared::{CreatePunishmentRequest, Punishment, PunishmentType, UpdatePunishmentRequest};
use uuid::Uuid;

use crate::api::ApiClient;
use crate::i18n::use_i18n;

#[component]
pub fn PunishmentModal(
    punishment: Option<Punishment>,
    household_id: String,
    /// All punishments in the household (for option selection)
    #[prop(default = vec![])]
    all_punishments: Vec<Punishment>,
    #[prop(into)] on_close: Callback<()>,
    #[prop(into)] on_save: Callback<Punishment>,
) -> impl IntoView {
    let i18n = use_i18n();
    let i18n_stored = store_value(i18n);
    let all_punishments_stored = store_value(all_punishments);
    let is_edit = punishment.is_some();
    let error = create_rw_signal(Option::<String>::None);
    let saving = create_rw_signal(false);
    let options_loading = create_rw_signal(false);

    // Form fields - initialize based on mode
    let name = create_rw_signal(punishment.as_ref().map(|p| p.name.clone()).unwrap_or_default());
    let description = create_rw_signal(punishment.as_ref().map(|p| p.description.clone()).unwrap_or_default());
    let requires_confirmation = create_rw_signal(punishment.as_ref().map(|p| p.requires_confirmation).unwrap_or(false));
    let punishment_type = create_rw_signal(punishment.as_ref().map(|p| p.punishment_type).unwrap_or_default());
    let selected_options = create_rw_signal(Vec::<Uuid>::new());

    let punishment_id = punishment.as_ref().map(|p| p.id.to_string());

    // Load existing options if editing a random choice punishment
    if let Some(ref pid) = punishment_id {
        if punishment_type.get_untracked().is_random_choice() {
            let pid = pid.clone();
            let hid = household_id.clone();
            options_loading.set(true);
            wasm_bindgen_futures::spawn_local(async move {
                if let Ok(options) = ApiClient::get_punishment_options(&hid, &pid).await {
                    selected_options.set(options.iter().map(|p| p.id).collect());
                }
                options_loading.set(false);
            });
        }
    }

    let on_submit = {
        let punishment_id = punishment_id.clone();
        let household_id = household_id.clone();

        move |ev: web_sys::SubmitEvent| {
            ev.prevent_default();

            // Validate minimum options for random choice
            if punishment_type.get().is_random_choice() && selected_options.get().len() < 2 {
                error.set(Some(i18n_stored.get_value().t("punishments.min_options_error")));
                return;
            }

            saving.set(true);
            error.set(None);

            let punishment_id = punishment_id.clone();
            let household_id = household_id.clone();

            wasm_bindgen_futures::spawn_local(async move {
                if let Some(punishment_id) = punishment_id {
                    // Edit mode - update existing punishment
                    let option_ids = if punishment_type.get().is_random_choice() {
                        Some(Some(selected_options.get()))
                    } else {
                        Some(None) // Clear options if not random choice
                    };

                    let request = UpdatePunishmentRequest {
                        name: Some(name.get()),
                        description: Some(description.get()),
                        requires_confirmation: Some(requires_confirmation.get()),
                        punishment_type: Some(punishment_type.get()),
                        option_ids,
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
                    let option_ids = if punishment_type.get().is_random_choice() {
                        Some(selected_options.get())
                    } else {
                        None
                    };

                    let request = CreatePunishmentRequest {
                        name: name.get(),
                        description: Some(description.get()),
                        requires_confirmation: Some(requires_confirmation.get()),
                        punishment_type: Some(punishment_type.get()),
                        option_ids,
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
                                <span>{i18n_stored.get_value().t("punishments.requires_confirmation")}</span>
                            </label>
                        </div>

                        <div class="form-group">
                            <label class="form-label" for="punishment-type">{i18n_stored.get_value().t("punishments.type_label")}</label>
                            <select
                                id="punishment-type"
                                class="form-input"
                                on:change=move |ev| {
                                    let value = event_target_value(&ev);
                                    punishment_type.set(value.parse().unwrap_or_default());
                                }
                            >
                                <option value="standard" selected=move || punishment_type.get() == PunishmentType::Standard>
                                    {i18n_stored.get_value().t("punishments.type_standard")}
                                </option>
                                <option value="random_choice" selected=move || punishment_type.get() == PunishmentType::RandomChoice>
                                    {i18n_stored.get_value().t("punishments.type_random_choice")}
                                </option>
                            </select>
                        </div>

                        // Options selection (shown only when random choice is selected)
                        <Show when=move || punishment_type.get().is_random_choice() fallback=|| ()>
                            <div class="form-group">
                                <label class="form-label">{i18n_stored.get_value().t("punishments.options_label")}</label>
                                <Show when=move || options_loading.get() fallback=|| ()>
                                    <p style="color: var(--text-muted); font-size: 0.875rem;">"Loading options..."</p>
                                </Show>
                                <div style="max-height: 200px; overflow-y: auto; border: 1px solid var(--border-color); border-radius: 4px; padding: 0.5rem;">
                                    {move || {
                                        all_punishments_stored.get_value().into_iter()
                                            // Self-reference is allowed
                                            .map(|p| {
                                                let option_id = p.id;
                                                let is_selected = move || selected_options.get().contains(&option_id);
                                                let toggle = move |_| {
                                                    selected_options.update(|opts| {
                                                        if opts.contains(&option_id) {
                                                            opts.retain(|id| *id != option_id);
                                                        } else {
                                                            opts.push(option_id);
                                                        }
                                                    });
                                                };
                                                let random_badge = if p.punishment_type.is_random_choice() {
                                                    format!(" [{}]", i18n_stored.get_value().t("punishments.random_choice"))
                                                } else {
                                                    String::new()
                                                };
                                                view! {
                                                    <label style="display: flex; align-items: center; gap: 0.5rem; padding: 0.25rem 0; cursor: pointer;">
                                                        <input
                                                            type="checkbox"
                                                            prop:checked=is_selected
                                                            on:change=toggle
                                                        />
                                                        <span>{p.name.clone()}{random_badge}</span>
                                                    </label>
                                                }
                                            })
                                            .collect_view()
                                    }}
                                </div>
                                <p style="font-size: 0.75rem; color: var(--text-muted); margin-top: 0.25rem;">
                                    {move || format!("{} {}", selected_options.get().len(), i18n_stored.get_value().t("punishments.selected"))}
                                </p>
                            </div>
                        </Show>
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
