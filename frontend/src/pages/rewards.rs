use leptos::*;
use leptos_router::*;
use shared::{CreateRewardRequest, Reward, UserReward, UserRewardWithUser};

use crate::api::ApiClient;
use crate::components::loading::Loading;
use crate::components::modal::Modal;

#[component]
pub fn RewardsPage() -> impl IntoView {
    let params = use_params_map();
    let household_id = move || params.with(|p| p.get("id").cloned().unwrap_or_default());

    let rewards = create_rw_signal(Vec::<Reward>::new());
    let my_rewards = create_rw_signal(Vec::<UserReward>::new());
    let all_user_rewards = create_rw_signal(Vec::<UserRewardWithUser>::new());
    let loading = create_rw_signal(true);
    let error = create_rw_signal(Option::<String>::None);
    let success = create_rw_signal(Option::<String>::None);
    let show_create_modal = create_rw_signal(false);

    // Form fields
    let name = create_rw_signal(String::new());
    let description = create_rw_signal(String::new());
    let point_cost = create_rw_signal(String::new());
    let is_purchasable = create_rw_signal(true);

    // Load rewards
    create_effect(move |_| {
        let id = household_id();
        if id.is_empty() {
            return;
        }

        let id_for_rewards = id.clone();
        let id_for_my_rewards = id.clone();
        let id_for_all_user_rewards = id.clone();

        wasm_bindgen_futures::spawn_local(async move {
            match ApiClient::list_rewards(&id_for_rewards).await {
                Ok(r) => {
                    rewards.set(r);
                    loading.set(false);
                }
                Err(e) => {
                    error.set(Some(e));
                    loading.set(false);
                }
            }
        });

        // Load my rewards
        wasm_bindgen_futures::spawn_local(async move {
            if let Ok(r) = ApiClient::list_user_rewards(&id_for_my_rewards).await {
                my_rewards.set(r);
            }
        });

        // Load all user rewards in household
        wasm_bindgen_futures::spawn_local(async move {
            if let Ok(r) = ApiClient::list_all_user_rewards(&id_for_all_user_rewards).await {
                all_user_rewards.set(r);
            }
        });
    });

    let on_create = move |ev: web_sys::SubmitEvent| {
        ev.prevent_default();

        let id = household_id();
        let cost: Option<i64> = point_cost.get().parse().ok();

        let request = CreateRewardRequest {
            name: name.get(),
            description: Some(description.get()),
            point_cost: cost,
            is_purchasable: is_purchasable.get(),
        };

        wasm_bindgen_futures::spawn_local(async move {
            match ApiClient::create_reward(&id, request).await {
                Ok(reward) => {
                    rewards.update(|r| r.push(reward));
                    show_create_modal.set(false);
                    name.set(String::new());
                    description.set(String::new());
                    point_cost.set(String::new());
                }
                Err(e) => error.set(Some(e)),
            }
        });
    };

    let on_purchase = move |reward_id: String| {
        let id = household_id();
        wasm_bindgen_futures::spawn_local(async move {
            match ApiClient::purchase_reward(&id, &reward_id).await {
                Ok(user_reward) => {
                    my_rewards.update(|r| r.push(user_reward));
                    success.set(Some("Reward purchased successfully!".to_string()));
                }
                Err(e) => error.set(Some(e)),
            }
        });
    };

    let on_redeem = move |user_reward_id: String| {
        let id = household_id();
        wasm_bindgen_futures::spawn_local(async move {
            match ApiClient::redeem_reward(&id, &user_reward_id).await {
                Ok(updated) => {
                    my_rewards.update(|r| {
                        if let Some(pos) = r.iter().position(|ur| ur.id.to_string() == user_reward_id) {
                            r[pos] = updated;
                        }
                    });
                    success.set(Some("Reward redeemed!".to_string()));
                }
                Err(e) => error.set(Some(e)),
            }
        });
    };

    let on_delete = move |reward_id: String| {
        let id = household_id();
        wasm_bindgen_futures::spawn_local(async move {
            if ApiClient::delete_reward(&id, &reward_id).await.is_ok() {
                rewards.update(|r| r.retain(|reward| reward.id.to_string() != reward_id));
            }
        });
    };

    view! {
        <div class="dashboard-header">
            <h1 class="dashboard-title">"Rewards"</h1>
            <a href=move || format!("/households/{}", household_id()) style="color: var(--text-muted);">
                "← Back to household"
            </a>
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
            // My Rewards Section
            <Show when=move || !my_rewards.get().is_empty() fallback=|| ()>
                <div class="card" style="margin-bottom: 1.5rem; border-left: 4px solid var(--success-color);">
                    <div class="card-header">
                        <h3 class="card-title">"My Rewards"</h3>
                    </div>
                    {move || {
                        let all_rewards = rewards.get();
                        my_rewards.get().into_iter().map(|user_reward| {
                            let reward_name = all_rewards.iter()
                                .find(|r| r.id == user_reward.reward_id)
                                .map(|r| r.name.clone())
                                .unwrap_or_else(|| "Unknown Reward".to_string());
                            let reward_desc = all_rewards.iter()
                                .find(|r| r.id == user_reward.reward_id)
                                .map(|r| r.description.clone())
                                .unwrap_or_default();
                            let ur_id = user_reward.id.to_string();
                            let redeem_id = ur_id.clone();
                            let is_redeemed = user_reward.redeemed;
                            let is_purchased = user_reward.is_purchased;
                            view! {
                                <div style="display: flex; justify-content: space-between; align-items: center; padding: 0.75rem; border-bottom: 1px solid var(--border-color);">
                                    <div>
                                        <div style="font-weight: 600;">{reward_name}</div>
                                        <div style="font-size: 0.75rem; color: var(--text-muted);">
                                            {if is_purchased { "Purchased" } else { "Assigned" }}
                                            {if is_redeemed { " • Redeemed" } else { "" }}
                                            {if !reward_desc.is_empty() { format!(" • {}", reward_desc) } else { String::new() }}
                                        </div>
                                    </div>
                                    {if !is_redeemed {
                                        view! {
                                            <button
                                                class="btn btn-success"
                                                style="padding: 0.25rem 0.75rem; font-size: 0.875rem;"
                                                on:click=move |_| on_redeem(redeem_id.clone())
                                            >
                                                "Redeem"
                                            </button>
                                        }.into_view()
                                    } else {
                                        view! {
                                            <span class="badge" style="background: var(--success-color); color: white;">"Redeemed"</span>
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
                    "+ Create Reward"
                </button>
            </div>

            <h3 style="margin-bottom: 1rem; color: var(--text-muted);">"Available Rewards"</h3>

            {move || {
                let r = rewards.get();
                if r.is_empty() {
                    view! {
                        <div class="card empty-state">
                            <p>"No rewards yet."</p>
                            <p>"Create rewards that members can earn!"</p>
                        </div>
                    }.into_view()
                } else {
                    view! {
                        <div class="grid grid-3">
                            {r.into_iter().map(|reward| {
                                let reward_id = reward.id.to_string();
                                let purchase_id = reward_id.clone();
                                let delete_id = reward_id.clone();
                                view! {
                                    <div class="card">
                                        <h3 class="card-title">{reward.name}</h3>
                                        <p style="color: var(--text-muted); font-size: 0.875rem; margin-bottom: 1rem;">
                                            {reward.description}
                                        </p>
                                        {if reward.is_purchasable {
                                            view! {
                                                <div style="display: flex; justify-content: space-between; align-items: center;">
                                                    <span class="points-badge">
                                                        {reward.point_cost.unwrap_or(0)} " pts"
                                                    </span>
                                                    <button
                                                        class="btn btn-success"
                                                        style="padding: 0.25rem 0.75rem; font-size: 0.875rem;"
                                                        on:click=move |_| on_purchase(purchase_id.clone())
                                                    >
                                                        "Purchase"
                                                    </button>
                                                </div>
                                            }.into_view()
                                        } else {
                                            view! {
                                                <span style="color: var(--text-muted); font-size: 0.75rem;">
                                                    "Assigned only"
                                                </span>
                                            }.into_view()
                                        }}
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

            // All Assigned Rewards Section
            <Show when=move || !all_user_rewards.get().is_empty() fallback=|| ()>
                <h3 style="margin-top: 2rem; margin-bottom: 1rem; color: var(--text-muted);">"Assigned Rewards (All Members)"</h3>
                <div class="card">
                    {move || {
                        let all_rewards = rewards.get();
                        all_user_rewards.get().into_iter().map(|user_reward_with_user| {
                            let reward_name = all_rewards.iter()
                                .find(|r| r.id == user_reward_with_user.user_reward.reward_id)
                                .map(|r| r.name.clone())
                                .unwrap_or_else(|| "Unknown Reward".to_string());
                            let is_redeemed = user_reward_with_user.user_reward.redeemed;
                            let is_purchased = user_reward_with_user.user_reward.is_purchased;
                            view! {
                                <div style="display: flex; justify-content: space-between; align-items: center; padding: 0.75rem; border-bottom: 1px solid var(--border-color);">
                                    <div>
                                        <div style="font-weight: 600;">{reward_name}</div>
                                        <div style="font-size: 0.75rem; color: var(--text-muted);">
                                            "Assigned to: " {user_reward_with_user.user.username}
                                            {if is_purchased { " (Purchased)" } else { " (Assigned)" }}
                                        </div>
                                    </div>
                                    {if is_redeemed {
                                        view! {
                                            <span class="badge" style="background: var(--success-color); color: white;">"Redeemed"</span>
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
            <Modal title="Create Reward" on_close=move |_| show_create_modal.set(false)>
                <form on:submit=on_create>
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
                        <label style="display: flex; align-items: center; gap: 0.5rem;">
                            <input
                                type="checkbox"
                                checked=move || is_purchasable.get()
                                on:change=move |ev| is_purchasable.set(event_target_checked(&ev))
                            />
                            "Can be purchased with points"
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
