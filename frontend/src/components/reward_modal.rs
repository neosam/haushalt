use leptos::*;
use shared::{CreateRewardRequest, Reward, UpdateRewardRequest};

use crate::api::ApiClient;

#[component]
pub fn RewardModal(
    reward: Option<Reward>,
    household_id: String,
    #[prop(into)] on_close: Callback<()>,
    #[prop(into)] on_save: Callback<Reward>,
) -> impl IntoView {
    let is_edit = reward.is_some();
    let error = create_rw_signal(Option::<String>::None);
    let saving = create_rw_signal(false);

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

    let reward_id = reward.as_ref().map(|r| r.id.to_string());

    let on_submit = {
        let reward_id = reward_id.clone();
        let household_id = household_id.clone();

        move |ev: web_sys::SubmitEvent| {
            ev.prevent_default();
            saving.set(true);
            error.set(None);

            let reward_id = reward_id.clone();
            let household_id = household_id.clone();

            let cost: Option<i64> = point_cost.get().parse().ok();

            wasm_bindgen_futures::spawn_local(async move {
                if let Some(reward_id) = reward_id {
                    // Edit mode - update existing reward
                    let request = UpdateRewardRequest {
                        name: Some(name.get()),
                        description: Some(description.get()),
                        point_cost: cost,
                        is_purchasable: Some(is_purchasable.get()),
                        requires_confirmation: Some(requires_confirmation.get()),
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
                    let request = CreateRewardRequest {
                        name: name.get(),
                        description: Some(description.get()),
                        point_cost: cost,
                        is_purchasable: is_purchasable.get(),
                        requires_confirmation: Some(requires_confirmation.get()),
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
                            <input
                                type="text"
                                id="reward-description"
                                class="form-input"
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
