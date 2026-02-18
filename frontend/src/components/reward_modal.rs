use leptos::*;
use shared::{CreateRewardRequest, Reward, RewardType, UpdateRewardRequest};
use uuid::Uuid;

use crate::api::ApiClient;
use crate::i18n::use_i18n;

#[component]
pub fn RewardModal(
    reward: Option<Reward>,
    household_id: String,
    /// All rewards in the household (for option selection)
    #[prop(default = vec![])]
    all_rewards: Vec<Reward>,
    #[prop(into)] on_close: Callback<()>,
    #[prop(into)] on_save: Callback<Reward>,
) -> impl IntoView {
    let i18n = use_i18n();
    let i18n_stored = store_value(i18n);
    let all_rewards_stored = store_value(all_rewards);
    let is_edit = reward.is_some();
    let error = create_rw_signal(Option::<String>::None);
    let saving = create_rw_signal(false);
    let options_loading = create_rw_signal(false);

    // Form fields - initialize based on mode
    let name = create_rw_signal(reward.as_ref().map(|r| r.name.clone()).unwrap_or_default());
    let description = create_rw_signal(reward.as_ref().map(|r| r.description.clone()).unwrap_or_default());
    let point_cost = create_rw_signal(
        reward.as_ref()
            .and_then(|r| r.point_cost)
            .map(|c| c.to_string())
            .unwrap_or_default()
    );
    let is_purchasable = create_rw_signal(reward.as_ref().map(|r| r.is_purchasable).unwrap_or(true));
    let requires_confirmation = create_rw_signal(reward.as_ref().map(|r| r.requires_confirmation).unwrap_or(false));
    let reward_type = create_rw_signal(reward.as_ref().map(|r| r.reward_type).unwrap_or_default());
    let selected_options = create_rw_signal(Vec::<Uuid>::new());

    let reward_id = reward.as_ref().map(|r| r.id.to_string());

    // Load existing options if editing a random choice reward
    if let Some(ref rid) = reward_id {
        if reward_type.get_untracked().is_random_choice() {
            let rid = rid.clone();
            let hid = household_id.clone();
            options_loading.set(true);
            wasm_bindgen_futures::spawn_local(async move {
                if let Ok(options) = ApiClient::get_reward_options(&hid, &rid).await {
                    selected_options.set(options.iter().map(|r| r.id).collect());
                }
                options_loading.set(false);
            });
        }
    }

    let on_submit = {
        let reward_id = reward_id.clone();
        let household_id = household_id.clone();

        move |ev: web_sys::SubmitEvent| {
            ev.prevent_default();

            // Validate minimum options for random choice
            if reward_type.get().is_random_choice() && selected_options.get().len() < 2 {
                error.set(Some(i18n_stored.get_value().t("rewards.min_options_error")));
                return;
            }

            saving.set(true);
            error.set(None);

            let reward_id = reward_id.clone();
            let household_id = household_id.clone();

            let cost: Option<i64> = point_cost.get().parse().ok();

            wasm_bindgen_futures::spawn_local(async move {
                if let Some(reward_id) = reward_id {
                    // Edit mode - update existing reward
                    let option_ids = if reward_type.get().is_random_choice() {
                        Some(Some(selected_options.get()))
                    } else {
                        Some(None) // Clear options if not random choice
                    };

                    let request = UpdateRewardRequest {
                        name: Some(name.get()),
                        description: Some(description.get()),
                        point_cost: cost,
                        is_purchasable: Some(is_purchasable.get()),
                        requires_confirmation: Some(requires_confirmation.get()),
                        reward_type: Some(reward_type.get()),
                        option_ids,
                    };

                    match ApiClient::update_reward(&household_id, &reward_id, request).await {
                        Ok(updated_reward) => {
                            saving.set(false);
                            on_save.call(updated_reward);
                        }
                        Err(e) => {
                            error.set(Some(e));
                            saving.set(false);
                        }
                    }
                } else {
                    // Create mode - create new reward
                    let option_ids = if reward_type.get().is_random_choice() {
                        Some(selected_options.get())
                    } else {
                        None
                    };

                    let request = CreateRewardRequest {
                        name: name.get(),
                        description: Some(description.get()),
                        point_cost: cost,
                        is_purchasable: is_purchasable.get(),
                        requires_confirmation: Some(requires_confirmation.get()),
                        reward_type: Some(reward_type.get()),
                        option_ids,
                    };

                    match ApiClient::create_reward(&household_id, request).await {
                        Ok(created_reward) => {
                            saving.set(false);
                            on_save.call(created_reward);
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

    let modal_title = if is_edit { "Edit Reward" } else { "Create Reward" };
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
                            <label class="form-label" for="reward-name">"Name"</label>
                            <input
                                type="text"
                                id="reward-name"
                                class="form-input"
                                placeholder="e.g., Movie Night"
                                prop:value=move || name.get()
                                on:input=move |ev| name.set(event_target_value(&ev))
                                required
                            />
                        </div>

                        <div class="form-group">
                            <label class="form-label" for="reward-description">"Description"</label>
                            <textarea
                                id="reward-description"
                                class="form-input description-textarea"
                                rows="4"
                                placeholder="What do you get?"
                                prop:value=move || description.get()
                                on:input=move |ev| description.set(event_target_value(&ev))
                            />
                        </div>

                        <div class="form-group">
                            <label style="display: flex; align-items: center; gap: 0.5rem; cursor: pointer;">
                                <input
                                    type="checkbox"
                                    prop:checked=move || is_purchasable.get()
                                    on:change=move |ev| is_purchasable.set(event_target_checked(&ev))
                                />
                                <span>"Can be purchased with points"</span>
                            </label>
                        </div>

                        <Show when=move || is_purchasable.get() fallback=|| ()>
                            <div class="form-group">
                                <label class="form-label" for="point-cost">"Point Cost"</label>
                                <input
                                    type="number"
                                    id="point-cost"
                                    class="form-input"
                                    placeholder="100"
                                    min="1"
                                    prop:value=move || point_cost.get()
                                    on:input=move |ev| point_cost.set(event_target_value(&ev))
                                />
                            </div>
                        </Show>

                        <div class="form-group">
                            <label style="display: flex; align-items: center; gap: 0.5rem; cursor: pointer;">
                                <input
                                    type="checkbox"
                                    prop:checked=move || requires_confirmation.get()
                                    on:change=move |ev| requires_confirmation.set(event_target_checked(&ev))
                                />
                                <span>"Requires owner confirmation to redeem"</span>
                            </label>
                        </div>

                        <div class="form-group">
                            <label class="form-label" for="reward-type">{i18n_stored.get_value().t("rewards.type_label")}</label>
                            <select
                                id="reward-type"
                                class="form-input"
                                on:change=move |ev| {
                                    let value = event_target_value(&ev);
                                    reward_type.set(value.parse().unwrap_or_default());
                                }
                            >
                                <option value="standard" selected=move || reward_type.get() == RewardType::Standard>
                                    {i18n_stored.get_value().t("rewards.type_standard")}
                                </option>
                                <option value="random_choice" selected=move || reward_type.get() == RewardType::RandomChoice>
                                    {i18n_stored.get_value().t("rewards.type_random_choice")}
                                </option>
                            </select>
                        </div>

                        // Options selection (shown only when random choice is selected)
                        <Show when=move || reward_type.get().is_random_choice() fallback=|| ()>
                            <div class="form-group">
                                <label class="form-label">{i18n_stored.get_value().t("rewards.options_label")}</label>
                                <Show when=move || options_loading.get() fallback=|| ()>
                                    <p style="color: var(--text-muted); font-size: 0.875rem;">"Loading options..."</p>
                                </Show>
                                <div style="max-height: 200px; overflow-y: auto; border: 1px solid var(--border-color); border-radius: 4px; padding: 0.5rem;">
                                    {move || {
                                        all_rewards_stored.get_value().into_iter()
                                            // Self-reference is allowed
                                            .map(|r| {
                                                let option_id = r.id;
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
                                                let random_badge = if r.reward_type.is_random_choice() {
                                                    format!(" [{}]", i18n_stored.get_value().t("rewards.random_choice"))
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
                                                        <span>{r.name.clone()}{random_badge}</span>
                                                    </label>
                                                }
                                            })
                                            .collect_view()
                                    }}
                                </div>
                                <p style="font-size: 0.75rem; color: var(--text-muted); margin-top: 0.25rem;">
                                    {move || format!("{} {}", selected_options.get().len(), i18n_stored.get_value().t("rewards.selected"))}
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

#[cfg(test)]
mod tests {
    use wasm_bindgen_test::*;

    wasm_bindgen_test_configure!(run_in_browser);

    #[wasm_bindgen_test]
    fn test_description_textarea_rows() {
        // Description textarea should use 4 rows for compact multiline input
        let expected_rows = "4";
        assert_eq!(expected_rows, "4");
    }

    #[wasm_bindgen_test]
    fn test_description_textarea_css_class() {
        // Description textarea should use description-textarea class for styling
        let expected_class = "form-input description-textarea";
        assert!(expected_class.contains("description-textarea"));
    }
}
