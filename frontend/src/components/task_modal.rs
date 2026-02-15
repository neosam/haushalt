use leptos::*;
use shared::{CreateTaskRequest, MemberWithUser, Punishment, RecurrenceType, RecurrenceValue, Reward, Task, TaskPunishmentLink, TaskRewardLink, UpdateTaskRequest};
use uuid::Uuid;

use crate::api::ApiClient;
use crate::components::calendar_picker::CalendarPicker;

#[component]
pub fn TaskModal(
    task: Option<Task>,
    household_id: String,
    members: Vec<MemberWithUser>,
    household_rewards: Vec<Reward>,
    household_punishments: Vec<Punishment>,
    linked_rewards: Vec<TaskRewardLink>,
    linked_punishments: Vec<TaskPunishmentLink>,
    #[prop(into)] on_close: Callback<()>,
    #[prop(into)] on_save: Callback<Task>,
) -> impl IntoView {
    let is_edit = task.is_some();
    let error = create_rw_signal(Option::<String>::None);
    let saving = create_rw_signal(false);

    // Form fields - initialize based on mode
    let title = create_rw_signal(task.as_ref().map(|t| t.title.clone()).unwrap_or_default());
    let description = create_rw_signal(task.as_ref().map(|t| t.description.clone()).unwrap_or_default());
    let recurrence_type = create_rw_signal(
        task.as_ref()
            .map(|t| t.recurrence_type.as_str().to_string())
            .unwrap_or_else(|| "daily".to_string())
    );
    // Auto-select if only one member can be assigned (create mode only)
    let initial_assigned_user_id = task.as_ref()
        .and_then(|t| t.assigned_user_id.map(|id| id.to_string()))
        .or_else(|| {
            // In create mode with exactly one assignable member, auto-select them
            if task.is_none() && members.len() == 1 {
                Some(members[0].user.id.to_string())
            } else {
                None
            }
        });
    let assigned_user = create_rw_signal(initial_assigned_user_id.clone().unwrap_or_default());
    let target_count = create_rw_signal(
        task.as_ref()
            .map(|t| t.target_count.to_string())
            .unwrap_or_else(|| "1".to_string())
    );
    let allow_exceed_target = create_rw_signal(
        task.as_ref()
            .map(|t| t.allow_exceed_target)
            .unwrap_or(true)  // Default to true for new tasks
    );

    // Recurrence value signals
    let selected_weekdays = create_rw_signal(
        task.as_ref()
            .and_then(|t| match &t.recurrence_value {
                Some(RecurrenceValue::Weekdays(days)) => Some(days.clone()),
                _ => None,
            })
            .unwrap_or_else(|| vec![1, 2, 3, 4, 5]) // Default Mon-Fri
    );

    // Single weekday for Weekly recurrence (0=Sun, 1=Mon, ..., 6=Sat)
    let selected_weekday = create_rw_signal(
        task.as_ref()
            .and_then(|t| match &t.recurrence_value {
                Some(RecurrenceValue::WeekDay(day)) => Some(*day),
                _ => None,
            })
            .unwrap_or(1) // Default to Monday
    );

    // Day of month for Monthly recurrence (1-31)
    let selected_month_day = create_rw_signal(
        task.as_ref()
            .and_then(|t| match &t.recurrence_value {
                Some(RecurrenceValue::MonthDay(day)) => Some(*day),
                _ => None,
            })
            .unwrap_or(1) // Default to 1st of month
    );

    let selected_custom_dates = create_rw_signal(
        task.as_ref()
            .and_then(|t| match &t.recurrence_value {
                Some(RecurrenceValue::CustomDates(dates)) => Some(dates.clone()),
                _ => None,
            })
            .unwrap_or_default()
    );

    // Track linked rewards/punishments with amounts: Vec<(id, amount)>
    let selected_rewards = create_rw_signal(
        linked_rewards.iter().map(|r| (r.reward.id.to_string(), r.amount)).collect::<Vec<_>>()
    );
    let selected_punishments = create_rw_signal(
        linked_punishments.iter().map(|p| (p.punishment.id.to_string(), p.amount)).collect::<Vec<_>>()
    );

    let original_rewards: Vec<(String, i32)> = linked_rewards.iter().map(|r| (r.reward.id.to_string(), r.amount)).collect();
    let original_punishments: Vec<(String, i32)> = linked_punishments.iter().map(|p| (p.punishment.id.to_string(), p.amount)).collect();

    // Signals for the "add new" dropdown selections
    let selected_new_reward = create_rw_signal(String::new());
    let new_reward_amount = create_rw_signal(1i32);
    let selected_new_punishment = create_rw_signal(String::new());
    let new_punishment_amount = create_rw_signal(1i32);

    let task_id = task.as_ref().map(|t| t.id.to_string());

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
                "onetime" => RecurrenceType::OneTime,
                "daily" => RecurrenceType::Daily,
                "weekly" => RecurrenceType::Weekly,
                "monthly" => RecurrenceType::Monthly,
                "weekdays" => RecurrenceType::Weekdays,
                "custom" => RecurrenceType::Custom,
                _ => RecurrenceType::Daily,
            };

            // Build recurrence value based on type
            let rec_value = match recurrence_type.get().as_str() {
                "weekly" => Some(RecurrenceValue::WeekDay(selected_weekday.get())),
                "monthly" => Some(RecurrenceValue::MonthDay(selected_month_day.get())),
                "weekdays" => Some(RecurrenceValue::Weekdays(selected_weekdays.get())),
                "custom" => Some(RecurrenceValue::CustomDates(selected_custom_dates.get())),
                _ => None,
            };

            let assigned = assigned_user.get();
            let assigned_user_id = if assigned.is_empty() {
                None
            } else {
                Uuid::parse_str(&assigned).ok()
            };

            let target = target_count.get().parse::<i32>().unwrap_or(1).max(1);
            let new_rewards = selected_rewards.get(); // Vec<(String, i32)>
            let new_punishments = selected_punishments.get(); // Vec<(String, i32)>

            wasm_bindgen_futures::spawn_local(async move {
                if let Some(task_id) = task_id {
                    // Edit mode - update existing task
                    let request = UpdateTaskRequest {
                        title: Some(title.get()),
                        description: Some(description.get()),
                        recurrence_type: Some(rec_type),
                        recurrence_value: rec_value,
                        assigned_user_id,
                        target_count: Some(target),
                        time_period: None,
                        allow_exceed_target: Some(allow_exceed_target.get()),
                    };

                    match ApiClient::update_task(&household_id, &task_id, request).await {
                        Ok(updated_task) => {
                            // Update reward links - compare by ID
                            let new_reward_ids: Vec<&String> = new_rewards.iter().map(|(id, _)| id).collect();
                            let original_reward_ids: Vec<&String> = original_rewards.iter().map(|(id, _)| id).collect();

                            // Add new rewards
                            for (reward_id, amount) in &new_rewards {
                                if !original_reward_ids.contains(&reward_id) {
                                    let _ = ApiClient::add_task_reward(&household_id, &task_id, reward_id, *amount).await;
                                }
                            }
                            // Remove rewards that were unlinked
                            for (reward_id, _) in &original_rewards {
                                if !new_reward_ids.contains(&reward_id) {
                                    let _ = ApiClient::remove_task_reward(&household_id, &task_id, reward_id).await;
                                }
                            }

                            // Update punishment links - compare by ID
                            let new_punishment_ids: Vec<&String> = new_punishments.iter().map(|(id, _)| id).collect();
                            let original_punishment_ids: Vec<&String> = original_punishments.iter().map(|(id, _)| id).collect();

                            // Add new punishments
                            for (punishment_id, amount) in &new_punishments {
                                if !original_punishment_ids.contains(&punishment_id) {
                                    let _ = ApiClient::add_task_punishment(&household_id, &task_id, punishment_id, *amount).await;
                                }
                            }
                            // Remove punishments that were unlinked
                            for (punishment_id, _) in &original_punishments {
                                if !new_punishment_ids.contains(&punishment_id) {
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
                } else {
                    // Create mode - create new task
                    let request = CreateTaskRequest {
                        title: title.get(),
                        description: Some(description.get()),
                        recurrence_type: rec_type,
                        recurrence_value: rec_value,
                        assigned_user_id,
                        target_count: Some(target),
                        time_period: None,
                        allow_exceed_target: Some(allow_exceed_target.get()),
                    };

                    match ApiClient::create_task(&household_id, request).await {
                        Ok(created_task) => {
                            let task_id = created_task.id.to_string();

                            // Add reward links with amounts
                            for (reward_id, amount) in &new_rewards {
                                let _ = ApiClient::add_task_reward(&household_id, &task_id, reward_id, *amount).await;
                            }

                            // Add punishment links with amounts
                            for (punishment_id, amount) in &new_punishments {
                                let _ = ApiClient::add_task_punishment(&household_id, &task_id, punishment_id, *amount).await;
                            }

                            saving.set(false);
                            on_save.call(created_task);
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

    let modal_title = if is_edit { "Edit Task" } else { "Create Task" };
    let submit_button_text = if is_edit { "Save Changes" } else { "Create" };
    let saving_text = if is_edit { "Saving..." } else { "Creating..." };

    // Weekday labels and values (0 = Sunday, 1 = Monday, etc.)
    let weekdays: [(u8, &str); 7] = [
        (1, "Mon"),
        (2, "Tue"),
        (3, "Wed"),
        (4, "Thu"),
        (5, "Fri"),
        (6, "Sat"),
        (0, "Sun"),
    ];

    view! {
        <div class="modal-backdrop" on:click=close>
            <div class="modal" style="max-width: 600px;" on:click=|e| e.stop_propagation()>
                <div class="modal-header">
                    <h3 class="modal-title">{modal_title}</h3>
                    <button class="modal-close" on:click=close>"×"</button>
                </div>

                {move || error.get().map(|e| view! {
                    <div class="alert alert-error" style="margin: 1rem;">{e}</div>
                })}

                <form on:submit=on_submit>
                    <div style="padding: 1rem; max-height: 60vh; overflow-y: auto;">
                        // Basic Info Section
                        <div class="form-group">
                            <label class="form-label" for="task-title">"Title"</label>
                            <input
                                type="text"
                                id="task-title"
                                class="form-input"
                                placeholder="e.g., Take out the trash"
                                prop:value=move || title.get()
                                on:input=move |ev| title.set(event_target_value(&ev))
                                required
                            />
                        </div>

                        <div class="form-group">
                            <label class="form-label" for="task-description">"Description"</label>
                            <input
                                type="text"
                                id="task-description"
                                class="form-input"
                                placeholder="Optional description"
                                prop:value=move || description.get()
                                on:input=move |ev| description.set(event_target_value(&ev))
                            />
                        </div>

                        <div class="form-group">
                            <label class="form-label" for="task-recurrence">"Recurrence"</label>
                            {
                                let initial_recurrence = task.as_ref()
                                    .map(|t| t.recurrence_type.as_str().to_string())
                                    .unwrap_or_else(|| "daily".to_string());
                                view! {
                                    <select
                                        id="task-recurrence"
                                        class="form-select"
                                        on:change=move |ev| recurrence_type.set(event_target_value(&ev))
                                    >
                                        <option value="onetime" selected=initial_recurrence == "onetime">"None (Free-form / One-time)"</option>
                                        <option value="daily" selected=initial_recurrence == "daily">"Daily"</option>
                                        <option value="weekly" selected=initial_recurrence == "weekly">"Weekly"</option>
                                        <option value="monthly" selected=initial_recurrence == "monthly">"Monthly"</option>
                                        <option value="weekdays" selected=initial_recurrence == "weekdays">"Specific Days"</option>
                                        <option value="custom" selected=initial_recurrence == "custom">"Custom Dates"</option>
                                    </select>
                                }
                            }
                        </div>

                        // Single weekday selection (shown when recurrence_type == "weekly")
                        <Show when=move || recurrence_type.get() == "weekly" fallback=|| ()>
                            <div class="form-group">
                                <label class="form-label" for="task-weekday">"Day of Week"</label>
                                <select
                                    id="task-weekday"
                                    class="form-select"
                                    on:change=move |ev| {
                                        if let Ok(day) = event_target_value(&ev).parse::<u8>() {
                                            selected_weekday.set(day);
                                        }
                                    }
                                >
                                    <option value="0" selected=move || selected_weekday.get() == 0>"Sunday"</option>
                                    <option value="1" selected=move || selected_weekday.get() == 1>"Monday"</option>
                                    <option value="2" selected=move || selected_weekday.get() == 2>"Tuesday"</option>
                                    <option value="3" selected=move || selected_weekday.get() == 3>"Wednesday"</option>
                                    <option value="4" selected=move || selected_weekday.get() == 4>"Thursday"</option>
                                    <option value="5" selected=move || selected_weekday.get() == 5>"Friday"</option>
                                    <option value="6" selected=move || selected_weekday.get() == 6>"Saturday"</option>
                                </select>
                                <small class="form-hint">"Task will be due on this day each week"</small>
                            </div>
                        </Show>

                        // Day of month selection (shown when recurrence_type == "monthly")
                        <Show when=move || recurrence_type.get() == "monthly" fallback=|| ()>
                            <div class="form-group">
                                <label class="form-label" for="task-monthday">"Day of Month"</label>
                                <input
                                    type="number"
                                    id="task-monthday"
                                    class="form-input"
                                    min="1"
                                    max="31"
                                    prop:value=move || selected_month_day.get().to_string()
                                    on:input=move |ev| {
                                        if let Ok(day) = event_target_value(&ev).parse::<u8>() {
                                            let clamped = day.clamp(1, 31);
                                            selected_month_day.set(clamped);
                                        }
                                    }
                                />
                                <small class="form-hint">"Task will be due on this day each month (adjusted for shorter months)"</small>
                            </div>
                        </Show>

                        // Multiple weekday selection (shown when recurrence_type == "weekdays")
                        <Show when=move || recurrence_type.get() == "weekdays" fallback=|| ()>
                            <div class="form-group">
                                <label class="form-label">"Select Days"</label>
                                <div style="display: flex; flex-wrap: wrap; gap: 0.5rem;">
                                    {weekdays.into_iter().map(|(day_num, day_name)| {
                                        view! {
                                            <label style="display: flex; align-items: center; gap: 0.25rem; padding: 0.5rem 0.75rem; border: 1px solid var(--card-border); border-radius: var(--border-radius); cursor: pointer; user-select: none;">
                                                <input
                                                    type="checkbox"
                                                    prop:checked=move || selected_weekdays.get().contains(&day_num)
                                                    on:change=move |ev| {
                                                        let checked = event_target_checked(&ev);
                                                        selected_weekdays.update(|days| {
                                                            if checked {
                                                                if !days.contains(&day_num) {
                                                                    days.push(day_num);
                                                                    days.sort();
                                                                }
                                                            } else {
                                                                days.retain(|d| *d != day_num);
                                                            }
                                                        });
                                                    }
                                                />
                                                <span>{day_name}</span>
                                            </label>
                                        }
                                    }).collect_view()}
                                </div>
                                <small class="form-hint">"Task will be due on selected days each week"</small>
                            </div>
                        </Show>

                        // Custom dates picker (shown when recurrence_type == "custom")
                        <Show when=move || recurrence_type.get() == "custom" fallback=|| ()>
                            <div class="form-group">
                                <label class="form-label">"Custom Dates"</label>
                                <CalendarPicker selected_dates=selected_custom_dates />
                                <small class="form-hint">"Task will be due on the specific dates you add"</small>
                            </div>
                        </Show>

                        <div class="form-group">
                            <label class="form-label" for="task-target-count">"Target Count"</label>
                            <input
                                type="number"
                                id="task-target-count"
                                class="form-input"
                                min="1"
                                prop:value=move || target_count.get()
                                on:input=move |ev| target_count.set(event_target_value(&ev))
                            />
                            <small class="form-hint">"How many times per period (1 for regular tasks, more for habits)"</small>
                        </div>

                        <div class="form-group">
                            <label style="display: flex; align-items: center; gap: 0.5rem; cursor: pointer;">
                                <input
                                    type="checkbox"
                                    prop:checked=move || allow_exceed_target.get()
                                    on:change=move |ev| allow_exceed_target.set(event_target_checked(&ev))
                                />
                                <span>"Allow exceeding target"</span>
                            </label>
                            <small class="form-hint">"When unchecked, the complete button is disabled once the target is reached"</small>
                        </div>

                        // Assignment Section
                        <div class="form-group">
                            <label class="form-label" for="task-assigned">"Assigned To"</label>
                            <select
                                id="task-assigned"
                                class="form-select"
                                prop:value=move || assigned_user.get()
                                on:change=move |ev| assigned_user.set(event_target_value(&ev))
                            >
                                <option value="" selected=initial_assigned_user_id.is_none()>"Not assigned (all members)"</option>
                                {members.clone().into_iter().map(|m| {
                                    let user_id = m.user.id.to_string();
                                    let is_selected = initial_assigned_user_id.as_ref() == Some(&user_id);
                                    let name = m.user.username.clone();
                                    view! {
                                        <option value=user_id selected=is_selected>{name}</option>
                                    }
                                }).collect_view()}
                            </select>
                            <small class="form-hint">"If assigned, only this user is penalized for missed tasks"</small>
                        </div>

                        // Rewards Section
                        <div class="form-group">
                            <label class="form-label">"Rewards on Completion"</label>
                            <div style="border: 1px solid var(--card-border); border-radius: var(--border-radius); padding: 0.75rem;">
                                // Add new reward row
                                {
                                    let household_rewards_for_dropdown = household_rewards.clone();
                                    view! {
                                        <div style="display: flex; gap: 0.5rem; align-items: center; margin-bottom: 0.75rem;">
                                            <select
                                                class="form-select"
                                                style="flex: 1;"
                                                prop:value=move || selected_new_reward.get()
                                                on:change=move |ev| selected_new_reward.set(event_target_value(&ev))
                                            >
                                                <option value="">"Select a reward..."</option>
                                                {move || {
                                                    let current_reward_ids: Vec<String> = selected_rewards.get().iter().map(|(id, _)| id.clone()).collect();
                                                    household_rewards_for_dropdown.iter()
                                                        .filter(|r| !current_reward_ids.contains(&r.id.to_string()))
                                                        .map(|reward| {
                                                            let reward_id = reward.id.to_string();
                                                            let name = reward.name.clone();
                                                            view! {
                                                                <option value=reward_id>{name}</option>
                                                            }
                                                        })
                                                        .collect_view()
                                                }}
                                            </select>
                                            <input
                                                type="number"
                                                class="form-input"
                                                style="width: 70px;"
                                                min="1"
                                                prop:value=move || new_reward_amount.get().to_string()
                                                on:input=move |ev| {
                                                    if let Ok(val) = event_target_value(&ev).parse::<i32>() {
                                                        new_reward_amount.set(val.max(1));
                                                    }
                                                }
                                            />
                                            <button
                                                type="button"
                                                class="btn btn-outline"
                                                style="padding: 0.5rem 1rem;"
                                                disabled=move || selected_new_reward.get().is_empty()
                                                on:click=move |_| {
                                                    let reward_id = selected_new_reward.get();
                                                    let amount = new_reward_amount.get();
                                                    if !reward_id.is_empty() {
                                                        selected_rewards.update(|r| {
                                                            if !r.iter().any(|(id, _)| id == &reward_id) {
                                                                r.push((reward_id.clone(), amount));
                                                            }
                                                        });
                                                        selected_new_reward.set(String::new());
                                                        new_reward_amount.set(1);
                                                    }
                                                }
                                            >
                                                "Add"
                                            </button>
                                        </div>
                                    }
                                }

                                // List of linked rewards
                                {
                                    let household_rewards_for_list = household_rewards.clone();
                                    view! {
                                        <div>
                                            {move || {
                                                let rewards = selected_rewards.get();
                                                if rewards.is_empty() {
                                                    view! { <p style="color: var(--text-muted); font-size: 0.875rem; margin: 0;">"No rewards linked"</p> }.into_view()
                                                } else {
                                                    rewards.iter().map(|(reward_id, amount)| {
                                                        let reward_name = household_rewards_for_list.iter()
                                                            .find(|r| r.id.to_string() == *reward_id)
                                                            .map(|r| r.name.clone())
                                                            .unwrap_or_else(|| "Unknown".to_string());
                                                        let reward_id_for_remove = reward_id.clone();
                                                        let amount_display = *amount;
                                                        view! {
                                                            <div style="display: flex; justify-content: space-between; align-items: center; padding: 0.5rem; background: var(--bg-secondary); border-radius: var(--border-radius); margin-bottom: 0.25rem;">
                                                                <span>
                                                                    {reward_name}
                                                                    {if amount_display > 1 {
                                                                        view! { <span style="color: var(--text-muted); margin-left: 0.5rem;">" ×"{amount_display}</span> }.into_view()
                                                                    } else {
                                                                        ().into_view()
                                                                    }}
                                                                </span>
                                                                <button
                                                                    type="button"
                                                                    class="btn btn-outline"
                                                                    style="padding: 0.25rem 0.5rem; font-size: 0.75rem;"
                                                                    on:click=move |_| {
                                                                        selected_rewards.update(|r| {
                                                                            r.retain(|(id, _)| id != &reward_id_for_remove);
                                                                        });
                                                                    }
                                                                >
                                                                    "Remove"
                                                                </button>
                                                            </div>
                                                        }
                                                    }).collect_view().into_view()
                                                }
                                            }}
                                        </div>
                                    }
                                }
                            </div>
                            <small class="form-hint">"Rewards will be automatically assigned when this task is completed"</small>
                        </div>

                        // Punishments Section
                        <div class="form-group">
                            <label class="form-label">"Punishments on Miss"</label>
                            <div style="border: 1px solid var(--card-border); border-radius: var(--border-radius); padding: 0.75rem;">
                                // Add new punishment row
                                {
                                    let household_punishments_for_dropdown = household_punishments.clone();
                                    view! {
                                        <div style="display: flex; gap: 0.5rem; align-items: center; margin-bottom: 0.75rem;">
                                            <select
                                                class="form-select"
                                                style="flex: 1;"
                                                prop:value=move || selected_new_punishment.get()
                                                on:change=move |ev| selected_new_punishment.set(event_target_value(&ev))
                                            >
                                                <option value="">"Select a punishment..."</option>
                                                {move || {
                                                    let current_punishment_ids: Vec<String> = selected_punishments.get().iter().map(|(id, _)| id.clone()).collect();
                                                    household_punishments_for_dropdown.iter()
                                                        .filter(|p| !current_punishment_ids.contains(&p.id.to_string()))
                                                        .map(|punishment| {
                                                            let punishment_id = punishment.id.to_string();
                                                            let name = punishment.name.clone();
                                                            view! {
                                                                <option value=punishment_id>{name}</option>
                                                            }
                                                        })
                                                        .collect_view()
                                                }}
                                            </select>
                                            <input
                                                type="number"
                                                class="form-input"
                                                style="width: 70px;"
                                                min="1"
                                                prop:value=move || new_punishment_amount.get().to_string()
                                                on:input=move |ev| {
                                                    if let Ok(val) = event_target_value(&ev).parse::<i32>() {
                                                        new_punishment_amount.set(val.max(1));
                                                    }
                                                }
                                            />
                                            <button
                                                type="button"
                                                class="btn btn-outline"
                                                style="padding: 0.5rem 1rem;"
                                                disabled=move || selected_new_punishment.get().is_empty()
                                                on:click=move |_| {
                                                    let punishment_id = selected_new_punishment.get();
                                                    let amount = new_punishment_amount.get();
                                                    if !punishment_id.is_empty() {
                                                        selected_punishments.update(|p| {
                                                            if !p.iter().any(|(id, _)| id == &punishment_id) {
                                                                p.push((punishment_id.clone(), amount));
                                                            }
                                                        });
                                                        selected_new_punishment.set(String::new());
                                                        new_punishment_amount.set(1);
                                                    }
                                                }
                                            >
                                                "Add"
                                            </button>
                                        </div>
                                    }
                                }

                                // List of linked punishments
                                {
                                    let household_punishments_for_list = household_punishments.clone();
                                    view! {
                                        <div>
                                            {move || {
                                                let punishments = selected_punishments.get();
                                                if punishments.is_empty() {
                                                    view! { <p style="color: var(--text-muted); font-size: 0.875rem; margin: 0;">"No punishments linked"</p> }.into_view()
                                                } else {
                                                    punishments.iter().map(|(punishment_id, amount)| {
                                                        let punishment_name = household_punishments_for_list.iter()
                                                            .find(|p| p.id.to_string() == *punishment_id)
                                                            .map(|p| p.name.clone())
                                                            .unwrap_or_else(|| "Unknown".to_string());
                                                        let punishment_id_for_remove = punishment_id.clone();
                                                        let amount_display = *amount;
                                                        view! {
                                                            <div style="display: flex; justify-content: space-between; align-items: center; padding: 0.5rem; background: var(--bg-secondary); border-radius: var(--border-radius); margin-bottom: 0.25rem;">
                                                                <span>
                                                                    {punishment_name}
                                                                    {if amount_display > 1 {
                                                                        view! { <span style="color: var(--text-muted); margin-left: 0.5rem;">" ×"{amount_display}</span> }.into_view()
                                                                    } else {
                                                                        ().into_view()
                                                                    }}
                                                                </span>
                                                                <button
                                                                    type="button"
                                                                    class="btn btn-outline"
                                                                    style="padding: 0.25rem 0.5rem; font-size: 0.75rem;"
                                                                    on:click=move |_| {
                                                                        selected_punishments.update(|p| {
                                                                            p.retain(|(id, _)| id != &punishment_id_for_remove);
                                                                        });
                                                                    }
                                                                >
                                                                    "Remove"
                                                                </button>
                                                            </div>
                                                        }
                                                    }).collect_view().into_view()
                                                }
                                            }}
                                        </div>
                                    }
                                }
                            </div>
                            <small class="form-hint">"Punishments will be automatically assigned when this task is missed"</small>
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

#[cfg(test)]
mod tests {
    use super::*;
    use wasm_bindgen_test::*;

    wasm_bindgen_test_configure!(run_in_browser);

    #[wasm_bindgen_test]
    fn test_recurrence_type_to_string_daily() {
        assert_eq!(RecurrenceType::Daily.as_str(), "daily");
    }

    #[wasm_bindgen_test]
    fn test_recurrence_type_to_string_weekly() {
        assert_eq!(RecurrenceType::Weekly.as_str(), "weekly");
    }

    #[wasm_bindgen_test]
    fn test_recurrence_type_to_string_monthly() {
        assert_eq!(RecurrenceType::Monthly.as_str(), "monthly");
    }

    #[wasm_bindgen_test]
    fn test_recurrence_type_to_string_onetime() {
        assert_eq!(RecurrenceType::OneTime.as_str(), "onetime");
    }

    #[wasm_bindgen_test]
    fn test_recurrence_type_from_string_daily() {
        assert_eq!(
            match "daily" {
                "daily" => RecurrenceType::Daily,
                _ => RecurrenceType::OneTime,
            },
            RecurrenceType::Daily
        );
    }

    #[wasm_bindgen_test]
    fn test_target_count_parse_valid() {
        let input = "5";
        let target = input.parse::<i32>().unwrap_or(1).max(1);
        assert_eq!(target, 5);
    }

    #[wasm_bindgen_test]
    fn test_target_count_parse_invalid() {
        let input = "invalid";
        let target = input.parse::<i32>().unwrap_or(1).max(1);
        assert_eq!(target, 1);
    }

    #[wasm_bindgen_test]
    fn test_target_count_parse_zero() {
        let input = "0";
        let target = input.parse::<i32>().unwrap_or(1).max(1);
        assert_eq!(target, 1);
    }

    #[wasm_bindgen_test]
    fn test_target_count_parse_negative() {
        let input = "-5";
        let target = input.parse::<i32>().unwrap_or(1).max(1);
        assert_eq!(target, 1);
    }

    #[wasm_bindgen_test]
    fn test_modal_title_create() {
        let is_edit = false;
        let modal_title = if is_edit { "Edit Task" } else { "Create Task" };
        assert_eq!(modal_title, "Create Task");
    }

    #[wasm_bindgen_test]
    fn test_modal_title_edit() {
        let is_edit = true;
        let modal_title = if is_edit { "Edit Task" } else { "Create Task" };
        assert_eq!(modal_title, "Edit Task");
    }

    #[wasm_bindgen_test]
    fn test_button_text_create() {
        let is_edit = false;
        let submit_button_text = if is_edit { "Save Changes" } else { "Create" };
        assert_eq!(submit_button_text, "Create");
    }

    #[wasm_bindgen_test]
    fn test_button_text_edit() {
        let is_edit = true;
        let submit_button_text = if is_edit { "Save Changes" } else { "Create" };
        assert_eq!(submit_button_text, "Save Changes");
    }

    #[wasm_bindgen_test]
    fn test_uuid_parse_valid() {
        let valid_uuid = "550e8400-e29b-41d4-a716-446655440000";
        let result = Uuid::parse_str(valid_uuid);
        assert!(result.is_ok());
    }

    #[wasm_bindgen_test]
    fn test_uuid_parse_invalid() {
        let invalid_uuid = "not-a-uuid";
        let result = Uuid::parse_str(invalid_uuid);
        assert!(result.is_err());
    }

    #[wasm_bindgen_test]
    fn test_empty_string_to_none() {
        let assigned = "";
        let assigned_user_id: Option<Uuid> = if assigned.is_empty() {
            None
        } else {
            Uuid::parse_str(assigned).ok()
        };
        assert!(assigned_user_id.is_none());
    }

    #[wasm_bindgen_test]
    fn test_weekday_values() {
        let weekdays: [(u8, &str); 7] = [
            (1, "Mon"),
            (2, "Tue"),
            (3, "Wed"),
            (4, "Thu"),
            (5, "Fri"),
            (6, "Sat"),
            (0, "Sun"),
        ];
        assert_eq!(weekdays[0], (1, "Mon"));
        assert_eq!(weekdays[6], (0, "Sun"));
    }

    #[wasm_bindgen_test]
    fn test_default_weekdays() {
        let default_weekdays: Vec<u8> = vec![1, 2, 3, 4, 5];
        assert_eq!(default_weekdays.len(), 5);
        assert!(default_weekdays.contains(&1)); // Monday
        assert!(default_weekdays.contains(&5)); // Friday
        assert!(!default_weekdays.contains(&0)); // Not Sunday
        assert!(!default_weekdays.contains(&6)); // Not Saturday
    }

    #[wasm_bindgen_test]
    fn test_rewards_list_change_add() {
        let mut selected: Vec<(String, i32)> = vec![("r1".to_string(), 1)];
        let reward_id = "r2".to_string();
        let amount = 2;
        if !selected.iter().any(|(id, _)| id == &reward_id) {
            selected.push((reward_id, amount));
        }
        assert_eq!(selected.len(), 2);
        assert!(selected.iter().any(|(id, _)| id == "r2"));
    }

    #[wasm_bindgen_test]
    fn test_rewards_list_change_remove() {
        let mut selected: Vec<(String, i32)> = vec![("r1".to_string(), 1), ("r2".to_string(), 2)];
        let reward_id = "r1".to_string();
        selected.retain(|(id, _)| id != &reward_id);
        assert_eq!(selected.len(), 1);
        assert!(!selected.iter().any(|(id, _)| id == "r1"));
    }

    #[wasm_bindgen_test]
    fn test_rewards_with_amounts() {
        let selected: Vec<(String, i32)> = vec![("r1".to_string(), 3), ("r2".to_string(), 1)];
        let r1_amount = selected.iter().find(|(id, _)| id == "r1").map(|(_, a)| *a);
        assert_eq!(r1_amount, Some(3));
    }
}
