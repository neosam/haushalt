use leptos::*;
use shared::{PendingPunishmentCompletion, PendingRewardRedemption};

use crate::api::ApiClient;
use crate::i18n::use_i18n;

#[component]
pub fn PendingConfirmations(
    household_id: String,
    #[prop(into)] on_confirmation_complete: Callback<()>,
) -> impl IntoView {
    let i18n = use_i18n();
    let i18n_stored = store_value(i18n);

    let pending_rewards = create_rw_signal(Vec::<PendingRewardRedemption>::new());
    let pending_punishments = create_rw_signal(Vec::<PendingPunishmentCompletion>::new());
    let loading = create_rw_signal(true);
    let error = create_rw_signal(Option::<String>::None);
    let processing = create_rw_signal(Option::<String>::None);

    // Fetch pending confirmations
    {
        let household_id = household_id.clone();
        create_effect(move |_| {
            let household_id = household_id.clone();
            let household_id2 = household_id.clone();

            // Fetch pending reward redemptions
            wasm_bindgen_futures::spawn_local(async move {
                match ApiClient::get_pending_reward_redemptions(&household_id).await {
                    Ok(data) => {
                        pending_rewards.set(data);
                    }
                    Err(e) => {
                        error.set(Some(e));
                    }
                }
            });

            // Fetch pending punishment completions
            wasm_bindgen_futures::spawn_local(async move {
                match ApiClient::get_pending_punishment_completions(&household_id2).await {
                    Ok(data) => {
                        pending_punishments.set(data);
                        loading.set(false);
                    }
                    Err(e) => {
                        error.set(Some(e));
                        loading.set(false);
                    }
                }
            });
        });
    }

    let approve_reward = {
        let household_id = household_id.clone();
        move |user_reward_id: String| {
            let household_id = household_id.clone();
            processing.set(Some(user_reward_id.clone()));

            wasm_bindgen_futures::spawn_local(async move {
                match ApiClient::approve_reward_redemption(&household_id, &user_reward_id).await {
                    Ok(updated) => {
                        // Update local list - reduce pending count or remove if none left
                        pending_rewards.update(|items| {
                            if let Some(item) = items.iter_mut().find(|r| r.user_reward.id.to_string() == user_reward_id) {
                                item.user_reward.pending_redemption = updated.pending_redemption;
                                item.user_reward.redeemed_amount = updated.redeemed_amount;
                            }
                            items.retain(|r| r.user_reward.pending_redemption > 0);
                        });
                        processing.set(None);
                        on_confirmation_complete.call(());
                    }
                    Err(e) => {
                        error.set(Some(e));
                        processing.set(None);
                    }
                }
            });
        }
    };

    let reject_reward = {
        let household_id = household_id.clone();
        move |user_reward_id: String| {
            let household_id = household_id.clone();
            processing.set(Some(user_reward_id.clone()));

            wasm_bindgen_futures::spawn_local(async move {
                match ApiClient::reject_reward_redemption(&household_id, &user_reward_id).await {
                    Ok(updated) => {
                        // Update local list
                        pending_rewards.update(|items| {
                            if let Some(item) = items.iter_mut().find(|r| r.user_reward.id.to_string() == user_reward_id) {
                                item.user_reward.pending_redemption = updated.pending_redemption;
                            }
                            items.retain(|r| r.user_reward.pending_redemption > 0);
                        });
                        processing.set(None);
                        on_confirmation_complete.call(());
                    }
                    Err(e) => {
                        error.set(Some(e));
                        processing.set(None);
                    }
                }
            });
        }
    };

    let approve_punishment = {
        let household_id = household_id.clone();
        move |user_punishment_id: String| {
            let household_id = household_id.clone();
            processing.set(Some(user_punishment_id.clone()));

            wasm_bindgen_futures::spawn_local(async move {
                match ApiClient::approve_punishment_completion(&household_id, &user_punishment_id).await {
                    Ok(updated) => {
                        // Update local list
                        pending_punishments.update(|items| {
                            if let Some(item) = items.iter_mut().find(|p| p.user_punishment.id.to_string() == user_punishment_id) {
                                item.user_punishment.pending_completion = updated.pending_completion;
                                item.user_punishment.completed_amount = updated.completed_amount;
                            }
                            items.retain(|p| p.user_punishment.pending_completion > 0);
                        });
                        processing.set(None);
                        on_confirmation_complete.call(());
                    }
                    Err(e) => {
                        error.set(Some(e));
                        processing.set(None);
                    }
                }
            });
        }
    };

    let reject_punishment = {
        let household_id = household_id.clone();
        move |user_punishment_id: String| {
            let household_id = household_id.clone();
            processing.set(Some(user_punishment_id.clone()));

            wasm_bindgen_futures::spawn_local(async move {
                match ApiClient::reject_punishment_completion(&household_id, &user_punishment_id).await {
                    Ok(updated) => {
                        // Update local list
                        pending_punishments.update(|items| {
                            if let Some(item) = items.iter_mut().find(|p| p.user_punishment.id.to_string() == user_punishment_id) {
                                item.user_punishment.pending_completion = updated.pending_completion;
                            }
                            items.retain(|p| p.user_punishment.pending_completion > 0);
                        });
                        processing.set(None);
                        on_confirmation_complete.call(());
                    }
                    Err(e) => {
                        error.set(Some(e));
                        processing.set(None);
                    }
                }
            });
        }
    };

    let approve_reward = std::rc::Rc::new(approve_reward);
    let reject_reward = std::rc::Rc::new(reject_reward);
    let approve_punishment = std::rc::Rc::new(approve_punishment);
    let reject_punishment = std::rc::Rc::new(reject_punishment);

    view! {
        {
            let approve_reward = approve_reward.clone();
            let reject_reward = reject_reward.clone();
            let approve_punishment = approve_punishment.clone();
            let reject_punishment = reject_punishment.clone();
            move || {
            // Hide entire component when not loading and empty (no pending confirmations)
            // Also hide when features are disabled ("not enabled" errors)
            let is_feature_disabled = error.get().as_ref().is_some_and(|e| e.contains("not enabled"));
            if !loading.get() && pending_rewards.get().is_empty() && pending_punishments.get().is_empty()
                && (error.get().is_none() || is_feature_disabled) {
                return ().into_view();
            }

            let approve_reward = approve_reward.clone();
            let reject_reward = reject_reward.clone();
            let approve_punishment = approve_punishment.clone();
            let reject_punishment = reject_punishment.clone();
            view! {
                <div class="card">
                    <div class="card-header">
                        <h3 class="card-title">{i18n_stored.get_value().t("pending_confirmations.title")}</h3>
                    </div>

                    {move || error.get().filter(|e| !e.contains("not enabled")).map(|e| view! {
                        <div class="alert alert-error" style="margin: 1rem;">{e}</div>
                    })}

                    {move || {
                        if loading.get() {
                            view! { <div class="empty-state"><p>{i18n_stored.get_value().t("common.loading")}</p></div> }.into_view()
                        } else {
                            let rewards = pending_rewards.get();
                            let punishments = pending_punishments.get();
                        let reward_label = i18n_stored.get_value().t("pending_confirmations.reward");
                        let punishment_label = i18n_stored.get_value().t("pending_confirmations.punishment");
                        let redemption_by_label = i18n_stored.get_value().t("pending_confirmations.redemption_requested_by");
                        let completion_by_label = i18n_stored.get_value().t("pending_confirmations.completion_marked_by");
                        let approve_label = i18n_stored.get_value().t("pending_confirmations.approve");
                        let reject_label = i18n_stored.get_value().t("pending_confirmations.reject");

                        view! {
                            <div>
                                // Pending reward redemptions
                                {rewards.into_iter().map(|item| {
                                    let id = item.user_reward.id.to_string();
                                    let id_for_approve = id.clone();
                                    let id_for_reject = id.clone();
                                    let id_check_1 = id.clone();
                                    let id_check_2 = id.clone();
                                    let id_check_3 = id.clone();
                                    let id_check_4 = id.clone();
                                    let approve = approve_reward.clone();
                                    let reject = reject_reward.clone();
                                    let reward_label = reward_label.clone();
                                    let redemption_by_label = redemption_by_label.clone();
                                    let approve_label = approve_label.clone();
                                    let reject_label = reject_label.clone();

                                    view! {
                                        <div class="pending-review-item">
                                            <div class="pending-review-content">
                                                <div class="pending-review-task">
                                                    {reward_label.clone()} " " {item.reward.name.clone()}
                                                    {if item.user_reward.pending_redemption > 1 {
                                                        format!(" (x{})", item.user_reward.pending_redemption)
                                                    } else {
                                                        String::new()
                                                    }}
                                                </div>
                                                <div class="pending-review-meta">
                                                    {redemption_by_label.clone()} " "
                                                    <strong>{item.user.username.clone()}</strong>
                                                </div>
                                            </div>
                                            <div class="pending-review-actions">
                                                <button
                                                    class="btn btn-success"
                                                    style="padding: 0.25rem 0.75rem; font-size: 0.875rem;"
                                                    disabled=move || processing.get() == Some(id_check_1.clone())
                                                    on:click=move |_| approve(id_for_approve.clone())
                                                >
                                                    {
                                                        let approve_label = approve_label.clone();
                                                        move || if processing.get() == Some(id_check_2.clone()) { "...".to_string() } else { approve_label.clone() }
                                                    }
                                                </button>
                                                <button
                                                    class="btn btn-danger"
                                                    style="padding: 0.25rem 0.75rem; font-size: 0.875rem;"
                                                    disabled=move || processing.get() == Some(id_check_3.clone())
                                                    on:click=move |_| reject(id_for_reject.clone())
                                                >
                                                    {
                                                        let reject_label = reject_label.clone();
                                                        move || if processing.get() == Some(id_check_4.clone()) { "...".to_string() } else { reject_label.clone() }
                                                    }
                                                </button>
                                            </div>
                                        </div>
                                    }
                                }).collect_view()}

                                // Pending punishment completions
                                {punishments.into_iter().map(|item| {
                                    let id = item.user_punishment.id.to_string();
                                    let id_for_approve = id.clone();
                                    let id_for_reject = id.clone();
                                    let id_check_1 = id.clone();
                                    let id_check_2 = id.clone();
                                    let id_check_3 = id.clone();
                                    let id_check_4 = id.clone();
                                    let approve = approve_punishment.clone();
                                    let reject = reject_punishment.clone();
                                    let punishment_label = punishment_label.clone();
                                    let completion_by_label = completion_by_label.clone();
                                    let approve_label = approve_label.clone();
                                    let reject_label = reject_label.clone();

                                    view! {
                                        <div class="pending-review-item">
                                            <div class="pending-review-content">
                                                <div class="pending-review-task">
                                                    {punishment_label.clone()} " " {item.punishment.name.clone()}
                                                    {if item.user_punishment.pending_completion > 1 {
                                                        format!(" (x{})", item.user_punishment.pending_completion)
                                                    } else {
                                                        String::new()
                                                    }}
                                                </div>
                                                <div class="pending-review-meta">
                                                    {completion_by_label.clone()} " "
                                                    <strong>{item.user.username.clone()}</strong>
                                                </div>
                                            </div>
                                            <div class="pending-review-actions">
                                                <button
                                                    class="btn btn-success"
                                                    style="padding: 0.25rem 0.75rem; font-size: 0.875rem;"
                                                    disabled=move || processing.get() == Some(id_check_1.clone())
                                                    on:click=move |_| approve(id_for_approve.clone())
                                                >
                                                    {
                                                        let approve_label = approve_label.clone();
                                                        move || if processing.get() == Some(id_check_2.clone()) { "...".to_string() } else { approve_label.clone() }
                                                    }
                                                </button>
                                                <button
                                                    class="btn btn-danger"
                                                    style="padding: 0.25rem 0.75rem; font-size: 0.875rem;"
                                                    disabled=move || processing.get() == Some(id_check_3.clone())
                                                    on:click=move |_| reject(id_for_reject.clone())
                                                >
                                                    {
                                                        let reject_label = reject_label.clone();
                                                        move || if processing.get() == Some(id_check_4.clone()) { "...".to_string() } else { reject_label.clone() }
                                                    }
                                                </button>
                                            </div>
                                        </div>
                                    }
                                }).collect_view()}
                            </div>
                        }.into_view()
                        }
                    }}
                </div>
            }.into_view()
        }}
    }
}
