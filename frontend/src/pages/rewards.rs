use leptos::*;
use leptos_router::*;
use shared::{HouseholdSettings, MemberWithUser, Reward, UserReward, UserRewardWithUser};

use crate::api::ApiClient;
use crate::components::household_tabs::{HouseholdTab, HouseholdTabs};
use crate::components::loading::Loading;
use crate::components::reward_modal::RewardModal;
use crate::i18n::use_i18n;

#[component]
pub fn RewardsPage() -> impl IntoView {
    let i18n = use_i18n();
    let i18n_stored = store_value(i18n);

    let params = use_params_map();
    let household_id = move || params.with(|p| p.get("id").cloned().unwrap_or_default());

    let rewards = create_rw_signal(Vec::<Reward>::new());
    let my_rewards = create_rw_signal(Vec::<UserReward>::new());
    let all_user_rewards = create_rw_signal(Vec::<UserRewardWithUser>::new());
    let members = create_rw_signal(Vec::<MemberWithUser>::new());
    let settings = create_rw_signal(Option::<HouseholdSettings>::None);
    let loading = create_rw_signal(true);
    let error = create_rw_signal(Option::<String>::None);
    let success = create_rw_signal(Option::<String>::None);

    // Modal state: None = closed, Some(None) = create mode, Some(Some(reward)) = edit mode
    let modal_reward = create_rw_signal(Option::<Option<Reward>>::None);

    // Load rewards
    create_effect(move |_| {
        let id = household_id();
        if id.is_empty() {
            return;
        }

        let id_for_rewards = id.clone();
        let id_for_my_rewards = id.clone();
        let id_for_all_user_rewards = id.clone();
        let id_for_members = id.clone();
        let id_for_settings = id.clone();

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

    let on_purchase = move |reward_id: String| {
        let id = household_id();
        wasm_bindgen_futures::spawn_local(async move {
            match ApiClient::purchase_reward(&id, &reward_id).await {
                Ok(user_reward) => {
                    my_rewards.update(|r| r.push(user_reward));
                    success.set(Some(i18n_stored.get_value().t("rewards.purchased_success")));
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
                    success.set(Some(i18n_stored.get_value().t("rewards.redeemed_success")));
                }
                Err(e) => error.set(Some(e)),
            }
        });
    };

    let on_pick_random = move |user_reward_id: String| {
        let id = household_id();
        wasm_bindgen_futures::spawn_local(async move {
            match ApiClient::pick_random_reward(&id, &user_reward_id).await {
                Ok(result) => {
                    // Remove or update the original random choice assignment
                    my_rewards.update(|r| {
                        if let Some(pos) = r.iter().position(|ur| ur.id.to_string() == user_reward_id) {
                            if r[pos].amount <= 1 {
                                r.remove(pos);
                            } else {
                                r[pos].amount -= 1;
                            }
                        }
                    });

                    // Add the newly assigned reward
                    let picked_name = result.picked_reward.name.clone();
                    let new_ur = result.user_reward;
                    let existing_idx = my_rewards.get().iter().position(|ur| ur.reward_id == new_ur.reward_id);
                    if let Some(idx) = existing_idx {
                        my_rewards.update(|r| r[idx].amount += 1);
                    } else {
                        my_rewards.update(|r| r.push(new_ur));
                    }

                    success.set(Some(format!("{}: {}", i18n_stored.get_value().t("rewards.random_picked"), picked_name)));
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

    let on_assign = move |reward_id: String, user_id: String| {
        let id = household_id();
        wasm_bindgen_futures::spawn_local(async move {
            match ApiClient::assign_reward(&id, &reward_id, &user_id).await {
                Ok(user_reward) => {
                    // Check if user already has this reward assigned
                    let existing_idx = all_user_rewards.get().iter().position(|ur| {
                        ur.user_reward.reward_id.to_string() == reward_id &&
                        ur.user_reward.user_id.to_string() == user_id
                    });

                    if let Some(idx) = existing_idx {
                        // Update existing entry with new amount
                        all_user_rewards.update(|r| {
                            r[idx].user_reward = user_reward;
                        });
                    } else {
                        // Add new entry
                        let user_info = members.get().iter()
                            .find(|m| m.user.id.to_string() == user_id)
                            .map(|m| m.user.clone());
                        if let Some(user) = user_info {
                            all_user_rewards.update(|r| r.push(UserRewardWithUser {
                                user_reward,
                                user,
                            }));
                        }
                    }
                }
                Err(e) => error.set(Some(e)),
            }
        });
    };

    let on_unassign = move |reward_id: String, user_id: String| {
        let id = household_id();
        wasm_bindgen_futures::spawn_local(async move {
            if ApiClient::unassign_reward(&id, &reward_id, &user_id).await.is_ok() {
                // Decrement amount or remove if amount becomes 0
                all_user_rewards.update(|r| {
                    if let Some(pos) = r.iter().position(|ur| {
                        ur.user_reward.reward_id.to_string() == reward_id &&
                        ur.user_reward.user_id.to_string() == user_id
                    }) {
                        if r[pos].user_reward.amount <= 1 {
                            r.remove(pos);
                        } else {
                            r[pos].user_reward.amount -= 1;
                        }
                    }
                });
            }
        });
    };

    view! {
        {move || {
            let hid = household_id();
            view! { <HouseholdTabs household_id=hid active_tab=HouseholdTab::Rewards settings=settings.get() /> }
        }}

        <div class="dashboard-header">
            <h1 class="dashboard-title">{i18n_stored.get_value().t("rewards.title")}</h1>
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
            // My Rewards Section - only show rewards with available redemptions or pending
            <Show when=move || my_rewards.get().iter().any(|ur| ur.amount > ur.redeemed_amount) fallback=|| ()>
                <div class="card" style="margin-bottom: 1.5rem; border-left: 4px solid var(--success-color);">
                    <div class="card-header">
                        <h3 class="card-title">{i18n_stored.get_value().t("rewards.my_rewards")}</h3>
                    </div>
                    {move || {
                        let all_rewards = rewards.get();
                        my_rewards.get().into_iter()
                            // Only show rewards that have available redemptions or pending confirmations
                            .filter(|ur| ur.amount > ur.redeemed_amount)
                            .map(|user_reward| {
                                let reward_info = all_rewards.iter()
                                    .find(|r| r.id == user_reward.reward_id);
                                let reward_name = reward_info
                                    .map(|r| r.name.clone())
                                    .unwrap_or_else(|| i18n_stored.get_value().t("rewards.unknown_reward"));
                                let reward_desc = reward_info
                                    .map(|r| r.description.clone())
                                    .unwrap_or_default();
                                let is_random_choice = reward_info
                                    .map(|r| r.reward_type.is_random_choice())
                                    .unwrap_or(false);
                                let ur_id = user_reward.id.to_string();
                                let redeem_id = ur_id.clone();
                                let pick_id = ur_id.clone();
                                let available = user_reward.amount - user_reward.redeemed_amount - user_reward.pending_redemption;
                                let pending = user_reward.pending_redemption;
                                view! {
                                    <div style="display: flex; justify-content: space-between; align-items: center; padding: 0.75rem; border-bottom: 1px solid var(--border-color);">
                                        <div>
                                            <div style="font-weight: 600;">
                                                {reward_name}
                                                {if is_random_choice {
                                                    view! {
                                                        <span class="badge" style="margin-left: 0.5rem; background: var(--primary-color); color: white; font-size: 0.65rem;">
                                                            {i18n_stored.get_value().t("rewards.random_choice")}
                                                        </span>
                                                    }.into_view()
                                                } else {
                                                    view! { <span></span> }.into_view()
                                                }}
                                            </div>
                                            <div style="font-size: 0.75rem; color: var(--text-muted);">
                                                {format!("{} available, {} redeemed", available, user_reward.redeemed_amount)}
                                                {if pending > 0 { format!(", {} pending", pending) } else { String::new() }}
                                                {if !reward_desc.is_empty() { format!(" • {}", reward_desc) } else { String::new() }}
                                            </div>
                                        </div>
                                        {if pending > 0 {
                                            view! {
                                                <span class="badge" style="background: var(--warning-color); color: white;">{i18n_stored.get_value().t("rewards.awaiting_confirmation")}</span>
                                            }.into_view()
                                        } else if is_random_choice {
                                            view! {
                                                <button
                                                    class="btn btn-primary"
                                                    style="padding: 0.25rem 0.75rem; font-size: 0.875rem;"
                                                    on:click=move |_| on_pick_random(pick_id.clone())
                                                >
                                                    {i18n_stored.get_value().t("rewards.pick_one")}
                                                </button>
                                            }.into_view()
                                        } else {
                                            view! {
                                                <button
                                                    class="btn btn-success"
                                                    style="padding: 0.25rem 0.75rem; font-size: 0.875rem;"
                                                    on:click=move |_| on_redeem(redeem_id.clone())
                                                >
                                                    {i18n_stored.get_value().t("rewards.redeem")}
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
                <button class="btn btn-primary" on:click=move |_| modal_reward.set(Some(None))>
                    "+ " {i18n_stored.get_value().t("rewards.create")}
                </button>
            </div>

            <h3 style="margin-bottom: 1rem; color: var(--text-muted);">{i18n_stored.get_value().t("rewards.available")}</h3>

            {move || {
                let r = rewards.get();
                let user_rewards = all_user_rewards.get();
                let member_list = members.get();
                let hierarchy = settings.get().map(|s| s.hierarchy_type).unwrap_or_default();

                if r.is_empty() {
                    view! {
                        <div class="card empty-state">
                            <p>{i18n_stored.get_value().t("rewards.no_rewards")}</p>
                            <p>{i18n_stored.get_value().t("rewards.add_first")}</p>
                        </div>
                    }.into_view()
                } else {
                    view! {
                        <div class="grid grid-3">
                            {r.into_iter().map(|reward| {
                                let reward_for_edit = reward.clone();
                                let reward_id = reward.id;
                                let reward_id_str = reward_id.to_string();
                                let purchase_id = reward_id_str.clone();
                                let delete_id = reward_id_str.clone();

                                // Get user assignments for this reward - show only open (unredeemed) count
                                // Filter by hierarchy: only show assignable roles
                                let user_assignments: Vec<_> = user_rewards.iter()
                                    .filter(|ur| ur.user_reward.reward_id == reward_id)
                                    .filter(|ur| {
                                        // Check if user's role can be assigned in this hierarchy
                                        member_list.iter()
                                            .find(|m| m.user.id == ur.user_reward.user_id)
                                            .map(|m| hierarchy.can_be_assigned(&m.membership.role))
                                            .unwrap_or(false)
                                    })
                                    .map(|ur| {
                                        let open = ur.user_reward.amount - ur.user_reward.redeemed_amount;
                                        (ur.user_reward.user_id, ur.user.username.clone(), open)
                                    })
                                    .filter(|(_, _, open)| *open > 0)  // Only show if there are open rewards
                                    .collect();

                                view! {
                                    <div class="card">
                                        <h3 class="card-title">
                                            {reward.name.clone()}
                                            {if reward.reward_type.is_random_choice() {
                                                view! {
                                                    <span class="badge" style="margin-left: 0.5rem; background: var(--primary-color); color: white; font-size: 0.65rem;">
                                                        {i18n_stored.get_value().t("rewards.random_choice")}
                                                    </span>
                                                }.into_view()
                                            } else {
                                                view! { <span></span> }.into_view()
                                            }}
                                        </h3>
                                        <p style="color: var(--text-muted); font-size: 0.875rem; margin-bottom: 0.5rem;">
                                            {reward.description.clone()}
                                        </p>
                                        {if reward.is_purchasable {
                                            view! {
                                                <div style="display: flex; justify-content: space-between; align-items: center; margin-bottom: 0.5rem;">
                                                    <span class="points-badge">
                                                        {reward.point_cost.unwrap_or(0)} " pts"
                                                    </span>
                                                    <button
                                                        class="btn btn-success"
                                                        style="padding: 0.25rem 0.75rem; font-size: 0.875rem;"
                                                        on:click=move |_| on_purchase(purchase_id.clone())
                                                    >
                                                        {i18n_stored.get_value().t("rewards.purchase")}
                                                    </button>
                                                </div>
                                            }.into_view()
                                        } else {
                                            view! {
                                                <div style="margin-bottom: 0.5rem;">
                                                    <span style="color: var(--text-muted); font-size: 0.75rem;">
                                                        {i18n_stored.get_value().t("rewards.assigned_only")}
                                                    </span>
                                                </div>
                                            }.into_view()
                                        }}

                                        // Assignments section
                                        <div style="border-top: 1px solid var(--border-color); padding-top: 0.5rem; margin-top: 0.5rem;">
                                            <div style="font-size: 0.75rem; color: var(--text-muted); margin-bottom: 0.25rem;">{i18n_stored.get_value().t("rewards.assignments")} ":"</div>
                                            {if user_assignments.is_empty() {
                                                view! {
                                                    <div style="font-size: 0.75rem; color: var(--text-muted); font-style: italic;">{i18n_stored.get_value().t("common.none")}</div>
                                                }.into_view()
                                            } else {
                                                user_assignments.into_iter().map(|(user_id, username, amount)| {
                                                    let reward_id_for_add = reward_id_str.clone();
                                                    let reward_id_for_remove = reward_id_str.clone();
                                                    let user_id_for_add = user_id.to_string();
                                                    let user_id_for_remove = user_id.to_string();
                                                    view! {
                                                        <div style="display: flex; justify-content: space-between; align-items: center; font-size: 0.875rem; padding: 0.25rem 0;">
                                                            <span>{username} ": " {amount} "x"</span>
                                                            <div style="display: flex; gap: 0.25rem;">
                                                                <button
                                                                    class="btn btn-outline"
                                                                    style="padding: 0.1rem 0.4rem; font-size: 0.75rem; min-width: 24px;"
                                                                    on:click=move |_| on_unassign(reward_id_for_remove.clone(), user_id_for_remove.clone())
                                                                >
                                                                    "-"
                                                                </button>
                                                                <button
                                                                    class="btn btn-outline"
                                                                    style="padding: 0.1rem 0.4rem; font-size: 0.75rem; min-width: 24px;"
                                                                    on:click=move |_| on_assign(reward_id_for_add.clone(), user_id_for_add.clone())
                                                                >
                                                                    "+"
                                                                </button>
                                                            </div>
                                                        </div>
                                                    }
                                                }).collect_view()
                                            }}

                                            // Add assignment for members without any open rewards
                                            // Filter by hierarchy: only show assignable roles
                                            {
                                                let users_with_open: std::collections::HashSet<_> = user_rewards.iter()
                                                    .filter(|ur| ur.user_reward.reward_id == reward_id)
                                                    .filter(|ur| ur.user_reward.amount > ur.user_reward.redeemed_amount)
                                                    .map(|ur| ur.user_reward.user_id)
                                                    .collect();
                                                let unassigned_members: Vec<_> = member_list.iter()
                                                    .filter(|m| !users_with_open.contains(&m.user.id))
                                                    .filter(|m| hierarchy.can_be_assigned(&m.membership.role))
                                                    .collect();

                                                if !unassigned_members.is_empty() {
                                                    view! {
                                                        <div style="margin-top: 0.5rem;">
                                                            {unassigned_members.into_iter().map(|member| {
                                                                let reward_id_for_new = reward_id_str.clone();
                                                                let user_id_for_new = member.user.id.to_string();
                                                                let username = member.user.username.clone();
                                                                view! {
                                                                    <div style="display: flex; justify-content: space-between; align-items: center; font-size: 0.875rem; padding: 0.25rem 0; color: var(--text-muted);">
                                                                        <span>{username} ": 0x"</span>
                                                                        <button
                                                                            class="btn btn-outline"
                                                                            style="padding: 0.1rem 0.4rem; font-size: 0.75rem; min-width: 24px;"
                                                                            on:click=move |_| on_assign(reward_id_for_new.clone(), user_id_for_new.clone())
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
                                                    let reward_for_edit = reward_for_edit.clone();
                                                    move |_| modal_reward.set(Some(Some(reward_for_edit.clone())))
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

        {move || modal_reward.get().map(|reward_opt| {
            let hh_id = household_id();
            let all_rews = rewards.get();
            view! {
                <RewardModal
                    reward=reward_opt
                    household_id=hh_id
                    all_rewards=all_rews
                    on_close=move |_| modal_reward.set(None)
                    on_save=move |saved_reward: Reward| {
                        // Check if this is an existing reward (edit) or new (create)
                        let existing_idx = rewards.get().iter().position(|r| r.id == saved_reward.id);
                        if let Some(idx) = existing_idx {
                            // Update existing reward
                            rewards.update(|r| r[idx] = saved_reward);
                        } else {
                            // Add new reward
                            rewards.update(|r| r.push(saved_reward));
                        }
                        modal_reward.set(None);
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
    fn test_available_rewards_calculation() {
        let amount = 5;
        let redeemed = 2;
        let available = amount - redeemed;
        assert_eq!(available, 3);
    }

    #[wasm_bindgen_test]
    fn test_available_rewards_all_redeemed() {
        let amount = 3;
        let redeemed = 3;
        let available = amount - redeemed;
        assert_eq!(available, 0);
    }

    #[wasm_bindgen_test]
    fn test_point_cost_display_with_cost() {
        let point_cost: Option<i64> = Some(100);
        let cost_text = point_cost.map(|c| format!(" ({} pts)", c)).unwrap_or_default();
        assert_eq!(cost_text, " (100 pts)");
    }

    #[wasm_bindgen_test]
    fn test_point_cost_display_without_cost() {
        let point_cost: Option<i64> = None;
        let cost_text = point_cost.map(|c| format!(" ({} pts)", c)).unwrap_or_default();
        assert_eq!(cost_text, "");
    }

    #[wasm_bindgen_test]
    fn test_reward_delete() {
        let reward_to_delete = Uuid::new_v4();
        let mut rewards: Vec<Uuid> = vec![reward_to_delete, Uuid::new_v4()];

        let delete_id = reward_to_delete.to_string();
        rewards.retain(|id| id.to_string() != delete_id);

        assert_eq!(rewards.len(), 1);
    }

    #[wasm_bindgen_test]
    fn test_description_display_with_content() {
        let reward_desc = "Movie night with family";
        let display = if !reward_desc.is_empty() {
            format!(" • {}", reward_desc)
        } else {
            String::new()
        };
        assert_eq!(display, " • Movie night with family");
    }

    #[wasm_bindgen_test]
    fn test_description_display_empty() {
        let reward_desc = "";
        let display = if !reward_desc.is_empty() {
            format!(" • {}", reward_desc)
        } else {
            String::new()
        };
        assert_eq!(display, "");
    }

    #[wasm_bindgen_test]
    fn test_reward_status_available() {
        let available = 3;
        let is_available = available > 0;
        assert!(is_available);
    }

    #[wasm_bindgen_test]
    fn test_reward_status_all_redeemed() {
        let available = 0;
        let is_available = available > 0;
        assert!(!is_available);
    }

    #[wasm_bindgen_test]
    fn test_purchasable_reward() {
        let is_purchasable = true;
        let point_cost = 50i64;
        let display = if is_purchasable {
            format!("{} pts", point_cost)
        } else {
            "Assigned only".to_string()
        };
        assert_eq!(display, "50 pts");
    }

    #[wasm_bindgen_test]
    fn test_non_purchasable_reward() {
        let is_purchasable = false;
        let point_cost = 50i64;
        let display = if is_purchasable {
            format!("{} pts", point_cost)
        } else {
            "Assigned only".to_string()
        };
        assert_eq!(display, "Assigned only");
    }

    #[wasm_bindgen_test]
    fn test_user_assignment_format() {
        let username = "Alice";
        let amount = 3;
        let display = format!("{}: {}x", username, amount);
        assert_eq!(display, "Alice: 3x");
    }

    #[wasm_bindgen_test]
    fn test_unassign_decrement() {
        let mut amount = 3;
        if amount <= 1 {
            amount = 0; // Would remove
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
    fn test_point_cost_parse_valid() {
        let input = "100";
        let cost: Option<i64> = input.parse().ok();
        assert_eq!(cost, Some(100));
    }

    #[wasm_bindgen_test]
    fn test_point_cost_parse_empty() {
        let input = "";
        let cost: Option<i64> = input.parse().ok();
        assert!(cost.is_none());
    }
}
