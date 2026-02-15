use leptos::*;
use shared::{MemberWithUser, Punishment, RecurrenceType, Reward, Task, UpdateTaskRequest};
use uuid::Uuid;

use crate::api::ApiClient;

#[component]
pub fn TaskEditModal(
    task: Task,
    household_id: String,
    members: Vec<MemberWithUser>,
    household_rewards: Vec<Reward>,
    household_punishments: Vec<Punishment>,
    linked_rewards: Vec<Reward>,
    linked_punishments: Vec<Punishment>,
    #[prop(into)] on_close: Callback<()>,
    #[prop(into)] on_save: Callback<Task>,
) -> impl IntoView {
    let error = create_rw_signal(Option::<String>::None);
    let saving = create_rw_signal(false);

    // Form fields
    let title = create_rw_signal(task.title.clone());
    let description = create_rw_signal(task.description.clone());
    let recurrence_type = create_rw_signal(task.recurrence_type.as_str().to_string());
    let assigned_user = create_rw_signal(task.assigned_user_id.map(|id| id.to_string()).unwrap_or_default());

    // Track linked rewards/punishments
    let selected_rewards = create_rw_signal(
        linked_rewards.iter().map(|r| r.id.to_string()).collect::<Vec<_>>()
    );
    let selected_punishments = create_rw_signal(
        linked_punishments.iter().map(|p| p.id.to_string()).collect::<Vec<_>>()
    );

    let original_rewards = linked_rewards.iter().map(|r| r.id.to_string()).collect::<Vec<_>>();
    let original_punishments = linked_punishments.iter().map(|p| p.id.to_string()).collect::<Vec<_>>();

    let task_id = task.id.to_string();

    let on_submit = {
        let task_id = task_id.clone();
        let household_id = household_id.clone();
        let original_rewards = original_rewards.clone();
        let original_punishments = original_punishments.clone();

        move |ev: web_sys::SubmitEvent| {
            ev.prevent_default();
            saving.set(true);
            error.set(None);

            let task_id = task_id.clone();
            let household_id = household_id.clone();
            let original_rewards = original_rewards.clone();
            let original_punishments = original_punishments.clone();

            let rec_type = match recurrence_type.get().as_str() {
                "daily" => RecurrenceType::Daily,
                "weekly" => RecurrenceType::Weekly,
                "monthly" => RecurrenceType::Monthly,
                "weekdays" => RecurrenceType::Weekdays,
                _ => RecurrenceType::Daily,
            };

            let assigned = assigned_user.get();
            let assigned_user_id = if assigned.is_empty() {
                None
            } else {
                Uuid::parse_str(&assigned).ok()
            };

            let request = UpdateTaskRequest {
                title: Some(title.get()),
                description: Some(description.get()),
                recurrence_type: Some(rec_type),
                recurrence_value: None,
                assigned_user_id,
            };

            let new_rewards = selected_rewards.get();
            let new_punishments = selected_punishments.get();

            wasm_bindgen_futures::spawn_local(async move {
                // Update task
                match ApiClient::update_task(&household_id, &task_id, request).await {
                    Ok(updated_task) => {
                        // Update reward links
                        for reward_id in &new_rewards {
                            if !original_rewards.contains(reward_id) {
                                let _ = ApiClient::add_task_reward(&household_id, &task_id, reward_id).await;
                            }
                        }
                        for reward_id in &original_rewards {
                            if !new_rewards.contains(reward_id) {
                                let _ = ApiClient::remove_task_reward(&household_id, &task_id, reward_id).await;
                            }
                        }

                        // Update punishment links
                        for punishment_id in &new_punishments {
                            if !original_punishments.contains(punishment_id) {
                                let _ = ApiClient::add_task_punishment(&household_id, &task_id, punishment_id).await;
                            }
                        }
                        for punishment_id in &original_punishments {
                            if !new_punishments.contains(punishment_id) {
                                let _ = ApiClient::remove_task_punishment(&household_id, &task_id, punishment_id).await;
                            }
                        }

                        saving.set(false);
                        on_save.call(updated_task);
                    }
                    Err(e) => {
                        error.set(Some(e));
                        saving.set(false);
                    }
                }
            });
        }
    };

    let close = move |_| on_close.call(());

    view! {
        <div class="modal-backdrop" on:click=close.clone()>
            <div class="modal" style="max-width: 600px;" on:click=|e| e.stop_propagation()>
                <div class="modal-header">
                    <h3 class="modal-title">"Edit Task"</h3>
                    <button class="modal-close" on:click=close>"×"</button>
                </div>

                {move || error.get().map(|e| view! {
                    <div class="alert alert-error" style="margin: 1rem;">{e}</div>
                })}

                <form on:submit=on_submit>
                    <div style="padding: 1rem; max-height: 60vh; overflow-y: auto;">
                        // Basic Info Section
                        <div class="form-group">
                            <label class="form-label" for="edit-title">"Title"</label>
                            <input
                                type="text"
                                id="edit-title"
                                class="form-input"
                                prop:value=move || title.get()
                                on:input=move |ev| title.set(event_target_value(&ev))
                                required
                            />
                        </div>

                        <div class="form-group">
                            <label class="form-label" for="edit-description">"Description"</label>
                            <input
                                type="text"
                                id="edit-description"
                                class="form-input"
                                prop:value=move || description.get()
                                on:input=move |ev| description.set(event_target_value(&ev))
                            />
                        </div>

                        <div class="form-group">
                            <label class="form-label" for="edit-recurrence">"Recurrence"</label>
                            <select
                                id="edit-recurrence"
                                class="form-select"
                                prop:value=move || recurrence_type.get()
                                on:change=move |ev| recurrence_type.set(event_target_value(&ev))
                            >
                                <option value="daily">"Daily"</option>
                                <option value="weekly">"Weekly"</option>
                                <option value="monthly">"Monthly"</option>
                                <option value="weekdays">"Weekdays (Mon-Fri)"</option>
                            </select>
                        </div>

                        // Assignment Section
                        <div class="form-group">
                            <label class="form-label" for="edit-assigned">"Assigned To"</label>
                            <select
                                id="edit-assigned"
                                class="form-select"
                                prop:value=move || assigned_user.get()
                                on:change=move |ev| assigned_user.set(event_target_value(&ev))
                            >
                                <option value="">"Not assigned (all members)"</option>
                                {members.clone().into_iter().map(|m| {
                                    let user_id = m.user.id.to_string();
                                    let name = m.user.username.clone();
                                    view! {
                                        <option value=user_id>{name}</option>
                                    }
                                }).collect_view()}
                            </select>
                            <small class="form-hint">"If assigned, only this user is penalized for missed tasks"</small>
                        </div>

                        // Rewards Section
                        <div class="form-group">
                            <label class="form-label">"Rewards on Completion"</label>
                            <div style="border: 1px solid var(--card-border); border-radius: var(--border-radius); padding: 0.5rem;">
                                {if household_rewards.is_empty() {
                                    view! { <p style="color: var(--text-muted); font-size: 0.875rem;">"No rewards defined"</p> }.into_view()
                                } else {
                                    household_rewards.clone().into_iter().map(|reward| {
                                        let reward_id = reward.id.to_string();
                                        let reward_id_for_check = reward_id.clone();
                                        let reward_id_for_change = reward_id.clone();
                                        view! {
                                            <label style="display: flex; align-items: center; gap: 0.5rem; padding: 0.25rem 0; cursor: pointer;">
                                                <input
                                                    type="checkbox"
                                                    prop:checked=move || selected_rewards.get().contains(&reward_id_for_check)
                                                    on:change=move |ev| {
                                                        let checked = event_target_checked(&ev);
                                                        selected_rewards.update(|r| {
                                                            if checked {
                                                                if !r.contains(&reward_id_for_change) {
                                                                    r.push(reward_id_for_change.clone());
                                                                }
                                                            } else {
                                                                r.retain(|id| id != &reward_id_for_change);
                                                            }
                                                        });
                                                    }
                                                />
                                                <span>{reward.name}</span>
                                            </label>
                                        }
                                    }).collect_view().into_view()
                                }}
                            </div>
                            <small class="form-hint">"Selected rewards will be automatically assigned when this task is completed"</small>
                        </div>

                        // Punishments Section
                        <div class="form-group">
                            <label class="form-label">"Punishments on Miss"</label>
                            <div style="border: 1px solid var(--card-border); border-radius: var(--border-radius); padding: 0.5rem;">
                                {if household_punishments.is_empty() {
                                    view! { <p style="color: var(--text-muted); font-size: 0.875rem;">"No punishments defined"</p> }.into_view()
                                } else {
                                    household_punishments.clone().into_iter().map(|punishment| {
                                        let punishment_id = punishment.id.to_string();
                                        let punishment_id_for_check = punishment_id.clone();
                                        let punishment_id_for_change = punishment_id.clone();
                                        view! {
                                            <label style="display: flex; align-items: center; gap: 0.5rem; padding: 0.25rem 0; cursor: pointer;">
                                                <input
                                                    type="checkbox"
                                                    prop:checked=move || selected_punishments.get().contains(&punishment_id_for_check)
                                                    on:change=move |ev| {
                                                        let checked = event_target_checked(&ev);
                                                        selected_punishments.update(|p| {
                                                            if checked {
                                                                if !p.contains(&punishment_id_for_change) {
                                                                    p.push(punishment_id_for_change.clone());
                                                                }
                                                            } else {
                                                                p.retain(|id| id != &punishment_id_for_change);
                                                            }
                                                        });
                                                    }
                                                />
                                                <span>{punishment.name}</span>
                                            </label>
                                        }
                                    }).collect_view().into_view()
                                }}
                            </div>
                            <small class="form-hint">"Selected punishments will be automatically assigned when this task is missed"</small>
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
                            {move || if saving.get() { "Saving..." } else { "Save Changes" }}
                        </button>
                    </div>
                </form>
            </div>
        </div>
    }
}
