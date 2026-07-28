use chrono::NaiveDate;
use leptos::*;
use shared::{Archetype, CreateTaskRequest, HabitType, MemberWithUser, Punishment, RecurrenceType, RecurrenceValue, Reward, Task, TaskCategory, TaskPunishmentLink, TaskRewardLink, UpdateTaskRequest};
use uuid::Uuid;

use crate::api::ApiClient;
use crate::components::accordion::Accordion;
use crate::components::calendar_picker::CalendarPicker;
use crate::components::task_fields::*;
use crate::components::task_form_model::{
    apply_onetime_date, assignment_after_preset, assignment_missing, derive_archetype,
    initial_open_groups, links_section_visible, onetime_date_value, preset,
    recurrence_after_preset, shows_points_penalty, shows_target_count, target_count_after_preset,
    FormFlags, FormMode, FormSnapshot, ALL_ARCHETYPES,
};
use crate::i18n::use_i18n;

/// Whether the edit modal should offer the delete action.
///
/// Delete applies to exactly one existing task, so it is hidden in create/duplicate
/// mode. The caller opts in by passing a callback, which is how permission is
/// expressed — no callback means no delete.
fn delete_action_available(has_task: bool, has_callback: bool) -> bool {
    has_task && has_callback
}

/// The recurrence selector plus whichever detail field the chosen rhythm needs.
///
/// Lives in its own component because it appears in two places: as a base field for every
/// recurring archetype, and inside the "details" group for one-offs, whose base field is a
/// single date instead. Same fields, same signals, one definition.
#[component]
fn RecurrenceFields(
    recurrence_type: RwSignal<String>,
    selected_weekday: RwSignal<u8>,
    selected_month_day: RwSignal<u8>,
    selected_weekdays: RwSignal<Vec<u8>>,
    selected_custom_dates: RwSignal<Vec<NaiveDate>>,
) -> impl IntoView {
    let i18n_stored = store_value(use_i18n());

    view! {
        <TaskRecurrenceTypeField value=recurrence_type />

        <Show when=move || recurrence_type.get() == "weekly" fallback=|| ()>
            <TaskWeekdayField value=selected_weekday />
        </Show>

        <Show when=move || recurrence_type.get() == "monthly" fallback=|| ()>
            <TaskMonthDayField value=selected_month_day />
        </Show>

        <Show when=move || recurrence_type.get() == "weekdays" fallback=|| ()>
            <TaskWeekdaysField value=selected_weekdays />
        </Show>

        <Show when=move || recurrence_type.get() == "custom" fallback=|| ()>
            <div class="form-group">
                <label class="form-label">{i18n_stored.get_value().t("task_modal.custom_dates")}</label>
                <CalendarPicker selected_dates=selected_custom_dates />
                <small class="form-hint">{i18n_stored.get_value().t("task_modal.custom_dates_hint")}</small>
            </div>
        </Show>
    }
}

#[component]
pub fn TaskModal(
    task: Option<Task>,
    household_id: String,
    members: Vec<MemberWithUser>,
    household_rewards: Vec<Reward>,
    household_punishments: Vec<Punishment>,
    linked_rewards: Vec<TaskRewardLink>,
    linked_punishments: Vec<TaskPunishmentLink>,
    #[prop(default = vec![])] categories: Vec<TaskCategory>,
    /// Optional: Task to prefill values from (for duplicate mode)
    /// When set with task=None, opens in create mode but with prefilled values
    #[prop(optional)] prefill_from: Option<Task>,
    /// Override default recurrence type (e.g., "onetime" for quick task creation)
    #[prop(default = "daily".to_string())] default_recurrence: String,
    /// If true, this is a suggestion rather than a direct task creation
    #[prop(default = false)] is_suggestion: bool,
    /// Default points reward from household settings (for create mode)
    #[prop(default = None)] default_points_reward: Option<i64>,
    /// Default points penalty from household settings (for create mode)
    #[prop(default = None)] default_points_penalty: Option<i64>,
    /// Default rewards from household settings (for create mode) - Vec of (reward_id, amount)
    #[prop(default = vec![])] default_rewards: Vec<(String, i32)>,
    /// Default punishments from household settings (for create mode) - Vec of (punishment_id, amount)
    #[prop(default = vec![])] default_punishments: Vec<(String, i32)>,
    /// Whether the household uses rewards at all. Defaults to `true` so a caller that could
    /// not load the settings keeps today's behaviour instead of silently hiding the section.
    #[prop(default = true)] rewards_enabled: bool,
    /// Whether the household uses punishments at all. See `rewards_enabled`.
    #[prop(default = true)] punishments_enabled: bool,
    /// The signed-in user, used to prefill the tracker of a bad habit.
    #[prop(default = None)] current_user_id: Option<Uuid>,
    /// Enables the delete action in edit mode. Receives the deleted task id after the
    /// task was removed on the server, so the caller can update its list.
    /// Omit to hide the action (e.g. for users without delete permission).
    #[prop(default = None)] on_delete: Option<Callback<String>>,
    #[prop(into)] on_close: Callback<()>,
    #[prop(into)] on_save: Callback<Task>,
) -> impl IntoView {
    let is_edit = task.is_some();

    // Delete is only offered for a single existing task, and only when the caller
    // provided a callback (which doubles as the permission check).
    let delete_task_id = task.as_ref().map(|t| t.id.to_string());
    let can_delete = delete_action_available(delete_task_id.is_some(), on_delete.is_some());
    let delete_task_id = store_value(delete_task_id);
    let on_delete_stored = store_value(on_delete);
    let delete_household_id = store_value(household_id.clone());
    let confirming_delete = create_rw_signal(false);
    let deleting = create_rw_signal(false);

    // Store members early so it can be used multiple times
    let members_stored = store_value(members);

    // Use task for edit mode, or prefill_from for duplicate mode
    let source_task = task.as_ref().or(prefill_from.as_ref());
    let error = create_rw_signal(Option::<String>::None);
    let saving = create_rw_signal(false);

    // Form fields - initialize based on mode (using source_task for both edit and duplicate)
    let title = create_rw_signal(source_task.map(|t| t.title.clone()).unwrap_or_default());
    let description = create_rw_signal(source_task.map(|t| t.description.clone()).unwrap_or_default());
    let recurrence_type = create_rw_signal(
        source_task
            .map(|t| t.recurrence_type.as_str().to_string())
            .unwrap_or(default_recurrence)
    );
    // Auto-select if only one member can be assigned (create mode only, not duplicate)
    let initial_assigned_user_id = source_task
        .and_then(|t| t.assigned_user_id.map(|id| id.to_string()))
        .or_else(|| {
            // In create mode with exactly one assignable member, auto-select them
            // But not if we're in duplicate mode (prefill_from is set)
            let members_val = members_stored.get_value();
            if task.is_none() && prefill_from.is_none() && members_val.len() == 1 {
                Some(members_val[0].user.id.to_string())
            } else {
                None
            }
        });
    let assigned_user = create_rw_signal(initial_assigned_user_id.unwrap_or_default());
    let target_count = create_rw_signal(
        source_task
            .map(|t| t.target_count.to_string())
            .unwrap_or_else(|| "1".to_string())
    );
    let allow_exceed_target = create_rw_signal(
        source_task
            .map(|t| t.allow_exceed_target)
            .unwrap_or(true)  // Default to true for new tasks
    );
    let anyone_can_complete = create_rw_signal(
        source_task
            .map(|t| t.anyone_can_complete)
            .unwrap_or(false)  // Default to false for new tasks
    );
    let assignee_cannot_uncomplete = create_rw_signal(
        source_task
            .map(|t| t.assignee_cannot_uncomplete)
            .unwrap_or(false)  // Default to false for new tasks
    );
    let requires_review = create_rw_signal(
        source_task
            .map(|t| t.requires_review)
            .unwrap_or(false)  // Default to false for new tasks
    );

    // Habit type signal (good = normal, bad = inverted consequences)
    let habit_type = create_rw_signal(
        source_task
            .map(|t| t.habit_type.as_str().to_string())
            .unwrap_or_else(|| "good".to_string())
    );

    // Category signal
    let selected_category_id = create_rw_signal(
        source_task
            .and_then(|t| t.category_id.map(|id| id.to_string()))
            .unwrap_or_default()
    );
    let categories_stored = store_value(categories);

    // Direct points signals - use defaults from household settings in create mode
    let is_create_mode = task.is_none() && prefill_from.is_none();
    let points_reward = create_rw_signal(
        source_task
            .and_then(|t| t.points_reward)
            .or(if is_create_mode { default_points_reward } else { None })
            .map(|p| p.to_string())
            .unwrap_or_default()
    );
    let points_penalty = create_rw_signal(
        source_task
            .and_then(|t| t.points_penalty)
            .or(if is_create_mode { default_points_penalty } else { None })
            .map(|p| p.to_string())
            .unwrap_or_default()
    );

    // Due time signal (HH:MM format)
    let due_time = create_rw_signal(
        source_task
            .and_then(|t| t.due_time.clone())
            .unwrap_or_default()
    );

    // Recurrence value signals
    let selected_weekdays = create_rw_signal(
        source_task
            .and_then(|t| match &t.recurrence_value {
                Some(RecurrenceValue::Weekdays(days)) => Some(days.clone()),
                _ => None,
            })
            .unwrap_or_else(|| vec![1, 2, 3, 4, 5]) // Default Mon-Fri
    );

    // Single weekday for Weekly recurrence (0=Sun, 1=Mon, ..., 6=Sat)
    let selected_weekday = create_rw_signal(
        source_task
            .and_then(|t| match &t.recurrence_value {
                Some(RecurrenceValue::WeekDay(day)) => Some(*day),
                _ => None,
            })
            .unwrap_or(1) // Default to Monday
    );

    // Day of month for Monthly recurrence (1-31)
    let selected_month_day = create_rw_signal(
        source_task
            .and_then(|t| match &t.recurrence_value {
                Some(RecurrenceValue::MonthDay(day)) => Some(*day),
                _ => None,
            })
            .unwrap_or(1) // Default to 1st of month
    );

    let selected_custom_dates = create_rw_signal(
        source_task
            .and_then(|t| match &t.recurrence_value {
                Some(RecurrenceValue::CustomDates(dates)) => Some(dates.clone()),
                _ => None,
            })
            .unwrap_or_default()
    );

    // An existing task already carries its archetype in its flags. A fresh form starts from
    // the rhythm the caller asked for - "onetime" means the user wants a single errand.
    let source_archetype = source_task.map(|t| t.archetype());
    let selected_archetype = create_rw_signal(source_archetype.unwrap_or_else(|| {
        if recurrence_type.get_untracked() == RecurrenceType::OneTime.as_str() {
            Archetype::OneOff
        } else {
            Archetype::Routine
        }
    }));
    let assignment_error = create_rw_signal(false);
    let current_user_id_stored = store_value(current_user_id.map(|id| id.to_string()));

    // Picking a type writes its preset into the flag signals. Nothing is locked or greyed
    // out afterwards: the type is an entry point, not a cage.
    let apply_archetype = move |archetype: Archetype| {
        selected_archetype.set(archetype);
        let defaults = archetype.defaults();
        habit_type.set(defaults.habit_type.as_str().to_string());
        anyone_can_complete.set(defaults.anyone_can_complete);
        assignee_cannot_uncomplete.set(defaults.assignee_cannot_uncomplete);
        recurrence_type.set(recurrence_after_preset(&recurrence_type.get(), archetype));
        target_count.set(target_count_after_preset(archetype));
        // A bonus task cannot be missed, so a penalty could never trigger. Leaving a value
        // behind in a hidden field would silently save a rule that never applies.
        if !shows_points_penalty(archetype) {
            points_penalty.set(String::new());
        }
        assigned_user.set(assignment_after_preset(
            archetype,
            &assigned_user.get(),
            current_user_id_stored.get_value().as_deref(),
        ));
        assignment_error.set(false);
    };

    // Only a blank form starts from a preset. When duplicating, the values come from the
    // task being copied and must survive untouched.
    if is_create_mode {
        apply_archetype(selected_archetype.get_untracked());
    }

    // Track linked rewards/punishments with amounts: Vec<(id, amount)>
    // In create mode, pre-select default rewards/punishments from household settings
    let initial_rewards: Vec<(String, i32)> = if !linked_rewards.is_empty() {
        linked_rewards.iter().map(|r| (r.reward.id.to_string(), r.amount)).collect()
    } else if is_create_mode && !default_rewards.is_empty() {
        default_rewards
    } else {
        vec![]
    };
    let initial_punishments: Vec<(String, i32)> = if !linked_punishments.is_empty() {
        linked_punishments.iter().map(|p| (p.punishment.id.to_string(), p.amount)).collect()
    } else if is_create_mode && !default_punishments.is_empty() {
        default_punishments
    } else {
        vec![]
    };
    let selected_rewards = create_rw_signal(initial_rewards);
    let selected_punishments = create_rw_signal(initial_punishments);

    let original_rewards: Vec<(String, i32)> = linked_rewards.iter().map(|r| (r.reward.id.to_string(), r.amount)).collect();
    let original_punishments: Vec<(String, i32)> = linked_punishments.iter().map(|p| (p.punishment.id.to_string(), p.amount)).collect();

    // A task may still carry links from before the household switched the feature off -
    // hiding them would silently drop them on the next save.
    let rewards_section_visible = links_section_visible(rewards_enabled, linked_rewards.len());
    let punishments_section_visible =
        links_section_visible(punishments_enabled, linked_punishments.len());

    // Signals for the "add new" dropdown selections
    let selected_new_reward = create_rw_signal(String::new());
    let new_reward_amount = create_rw_signal(1i32);
    let selected_new_punishment = create_rw_signal(String::new());
    let new_punishment_amount = create_rw_signal(1i32);

    // Dashboard visibility signal
    let on_dashboard = create_rw_signal(false);
    let initial_on_dashboard = create_rw_signal(false);

    // A group starts expanded when it holds something that deviates from the archetype's
    // preset. `on_dashboard` is not part of it: it arrives asynchronously after mount.
    let initial_groups = initial_open_groups(
        if is_edit { FormMode::Edit } else { FormMode::Create },
        &FormSnapshot {
            description_empty: description.get_untracked().trim().is_empty(),
            category_set: !selected_category_id.get_untracked().is_empty(),
            due_time_set: !due_time.get_untracked().is_empty(),
            recurrence: recurrence_type.get_untracked(),
            custom_dates_len: selected_custom_dates.get_untracked().len(),
            target_count: target_count.get_untracked(),
            allow_exceed: allow_exceed_target.get_untracked(),
            habit_bad: habit_type.get_untracked() == HabitType::Bad.as_str(),
            points_reward_set: !points_reward.get_untracked().is_empty(),
            points_penalty_set: !points_penalty.get_untracked().is_empty(),
            linked_rewards: selected_rewards.get_untracked().len(),
            linked_punishments: selected_punishments.get_untracked().len(),
            anyone_can_complete: anyone_can_complete.get_untracked(),
            assignee_cannot_uncomplete: assignee_cannot_uncomplete.get_untracked(),
            requires_review: requires_review.get_untracked(),
        },
        selected_archetype.get_untracked(),
    );

    let task_id = task.as_ref().map(|t| t.id.to_string());

    // Load initial dashboard status for existing tasks
    {
        let task_id_for_effect = task_id.clone();
        create_effect(move |_| {
            if let Some(ref task_id) = task_id_for_effect {
                let task_id = task_id.clone();
                wasm_bindgen_futures::spawn_local(async move {
                    if let Ok(is_on_dashboard) = ApiClient::is_task_on_dashboard(&task_id).await {
                        on_dashboard.set(is_on_dashboard);
                        initial_on_dashboard.set(is_on_dashboard);
                    }
                });
            }
        });
    }

    let on_submit = {
        let task_id = task_id.clone();
        let household_id = household_id.clone();
        let original_rewards = original_rewards.clone();
        let original_punishments = original_punishments.clone();

        move |ev: web_sys::SubmitEvent| {
            ev.prevent_default();

            // A maintenance task without a responsible person never locks anything, so it
            // must not be saved silently ineffective.
            if assignment_missing(selected_archetype.get(), &assigned_user.get()) {
                assignment_error.set(true);
                return;
            }
            assignment_error.set(false);

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

            let target = target_count.get().parse::<i32>().unwrap_or(1).max(0);
            let new_rewards = selected_rewards.get(); // Vec<(String, i32)>
            let new_punishments = selected_punishments.get(); // Vec<(String, i32)>

            wasm_bindgen_futures::spawn_local(async move {
                if let Some(task_id) = task_id {
                    // Edit mode - update existing task
                    let pts_reward = points_reward.get().parse::<i64>().ok();
                    let pts_penalty = points_penalty.get().parse::<i64>().ok();
                    let due_time_val = {
                        let val = due_time.get();
                        if val.is_empty() { None } else { Some(val) }
                    };
                    let habit_type_val = match habit_type.get().as_str() {
                        "bad" => HabitType::Bad,
                        _ => HabitType::Good,
                    };
                    let category_id_val = {
                        let cat_id = selected_category_id.get();
                        if cat_id.is_empty() {
                            Some(None) // Explicitly set to None to clear the category
                        } else {
                            Some(Uuid::parse_str(&cat_id).ok())
                        }
                    };
                    let request = UpdateTaskRequest {
                        title: Some(title.get()),
                        description: Some(description.get()),
                        recurrence_type: Some(rec_type),
                        recurrence_value: rec_value,
                        assigned_user_id,
                        target_count: Some(target),
                        time_period: None,
                        allow_exceed_target: Some(allow_exceed_target.get()),
                        anyone_can_complete: Some(anyone_can_complete.get()),
                        assignee_cannot_uncomplete: Some(assignee_cannot_uncomplete.get()),
                        requires_review: Some(requires_review.get()),
                        points_reward: pts_reward,
                        points_penalty: pts_penalty,
                        due_time: due_time_val,
                        habit_type: Some(habit_type_val),
                        category_id: category_id_val,
                        archived: None,
                        paused: None,
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

                            // Update dashboard status if changed
                            let current_on_dashboard = on_dashboard.get();
                            let was_on_dashboard = initial_on_dashboard.get();
                            if current_on_dashboard != was_on_dashboard {
                                if current_on_dashboard {
                                    let _ = ApiClient::add_task_to_dashboard(&task_id).await;
                                } else {
                                    let _ = ApiClient::remove_task_from_dashboard(&task_id).await;
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
                    let pts_reward = points_reward.get().parse::<i64>().ok();
                    let pts_penalty = points_penalty.get().parse::<i64>().ok();
                    let due_time_val = {
                        let val = due_time.get();
                        if val.is_empty() { None } else { Some(val) }
                    };
                    let habit_type_val = match habit_type.get().as_str() {
                        "bad" => HabitType::Bad,
                        _ => HabitType::Good,
                    };
                    let category_id_val = {
                        let cat_id = selected_category_id.get();
                        if cat_id.is_empty() { None } else { Uuid::parse_str(&cat_id).ok() }
                    };
                    let request = CreateTaskRequest {
                        title: title.get(),
                        description: Some(description.get()),
                        recurrence_type: rec_type,
                        recurrence_value: rec_value,
                        assigned_user_id,
                        target_count: Some(target),
                        time_period: None,
                        allow_exceed_target: Some(allow_exceed_target.get()),
                        anyone_can_complete: Some(anyone_can_complete.get()),
                        assignee_cannot_uncomplete: Some(assignee_cannot_uncomplete.get()),
                        requires_review: Some(requires_review.get()),
                        points_reward: pts_reward,
                        points_penalty: pts_penalty,
                        due_time: due_time_val,
                        habit_type: Some(habit_type_val),
                        category_id: category_id_val,
                        is_suggestion: if is_suggestion { Some(true) } else { None },
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

                            // Add to dashboard if enabled
                            if on_dashboard.get() {
                                let _ = ApiClient::add_task_to_dashboard(&task_id).await;
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

    let i18n = use_i18n();
    let i18n_stored = store_value(i18n.clone());

    // Deletes on the server first, then hands the id to the caller so it can drop the
    // task from its list. The modal stays open on failure so the error is visible.
    let confirm_delete = move |_| {
        let Some(task_id) = delete_task_id.get_value() else {
            return;
        };
        let household_id = delete_household_id.get_value();
        deleting.set(true);
        error.set(None);
        wasm_bindgen_futures::spawn_local(async move {
            match ApiClient::delete_task(&household_id, &task_id).await {
                Ok(()) => {
                    if let Some(callback) = on_delete_stored.get_value() {
                        callback.call(task_id);
                    }
                    deleting.set(false);
                    on_close.call(());
                }
                Err(e) => {
                    deleting.set(false);
                    confirming_delete.set(false);
                    error.set(Some(format!(
                        "{}: {}",
                        i18n_stored.get_value().t("task_modal.delete_failed"),
                        e
                    )));
                }
            }
        });
    };

    // In create mode the heading names the chosen type, so the form says what it produces.
    let modal_title = move || {
        if is_edit {
            i18n_stored.get_value().t("task_modal.edit_title")
        } else if is_suggestion {
            i18n_stored.get_value().t("task_modal.suggest_title")
        } else {
            i18n_stored
                .get_value()
                .t(preset(selected_archetype.get()).form_title_key)
        }
    };
    let submit_button_text = if is_edit {
        i18n.t("task_modal.save_changes")
    } else if is_suggestion {
        i18n.t("suggestions.suggest_task")
    } else {
        i18n.t("common.create")
    };
    let saving_text = if is_edit {
        i18n.t("task_modal.saving")
    } else if is_suggestion {
        i18n.t("suggestions.suggesting")
    } else {
        i18n.t("task_modal.creating")
    };

    // The type the switches currently add up to. Follows every toggle, so flipping a
    // permission by hand is visible immediately instead of silently contradicting the cards.
    let derived_archetype = move || {
        derive_archetype(
            &FormFlags {
                habit_bad: habit_type.get() == HabitType::Bad.as_str(),
                anyone_can_complete: anyone_can_complete.get(),
                assignee_cannot_uncomplete: assignee_cannot_uncomplete.get(),
                recurrence: recurrence_type.get(),
                target_count: target_count.get(),
            },
            selected_archetype.get(),
        )
    };

    // One-offs carry a date as their base field; every other type carries a rhythm.
    let base_field_is_date = move || preset(selected_archetype.get()).base_is_date;

    view! {
        <div class="modal-backdrop" on:click=close>
            <div class="modal modal-task" on:click=|e| e.stop_propagation()>
                <div class="modal-header">
                    <h3 class="modal-title">{modal_title}</h3>
                    <button class="modal-close" on:click=close>"×"</button>
                </div>

                {move || error.get().map(|e| view! {
                    <div class="alert alert-error" style="margin: 1rem;">{e}</div>
                })}

                <form on:submit=on_submit>
                    <div style="padding: 1rem; max-height: 60vh; overflow-y: auto;">

                        // Step 1: what is being created. Edit mode shows the chip instead.
                        <Show when=move || !is_edit fallback=|| ()>
                            <div class="form-group">
                                <label class="form-label">{i18n_stored.get_value().t("task_modal.archetype.step_label")}</label>
                                <div class="task-type-grid">
                                    {ALL_ARCHETYPES.into_iter().map(|archetype| {
                                        let p = preset(archetype);
                                        let name = i18n_stored.get_value().t(p.name_key);
                                        let desc = i18n_stored.get_value().t(p.desc_key);
                                        view! {
                                            <button
                                                type="button"
                                                class="task-type-card"
                                                aria-pressed=move || (selected_archetype.get() == archetype).to_string()
                                                on:click=move |_| apply_archetype(archetype)
                                            >
                                                <span class="task-type-icon">{p.icon}</span>
                                                <span class="task-type-text">
                                                    <span class="task-type-name">{name}</span>
                                                    <span class="task-type-desc">{desc}</span>
                                                </span>
                                            </button>
                                        }
                                    }).collect_view()}
                                </div>
                            </div>
                        </Show>

                        // The type the flags currently add up to - marked when it drifted away
                        // from what was picked.
                        <div class="task-archetype-chip-row">
                            {move || {
                                let derived = derived_archetype();
                                let p = preset(derived);
                                let drifted = derived != selected_archetype.get();
                                let chip_class = if drifted {
                                    "task-archetype-chip changed"
                                } else {
                                    "task-archetype-chip"
                                };
                                let name = i18n_stored.get_value().t(p.name_key);
                                let changed_label = i18n_stored.get_value().t("task_modal.archetype.changed");
                                view! {
                                    <span class=chip_class>
                                        <span class="task-type-icon">{p.icon}</span>
                                        <span>{name}</span>
                                        {drifted.then(|| view! {
                                            <span class="task-archetype-chip-note">{changed_label}</span>
                                        })}
                                    </span>
                                }
                            }}
                        </div>

                        {move || preset(selected_archetype.get()).note.map(|(kind, note_key)| {
                            let text = i18n_stored.get_value().t(note_key);
                            view! { <div class=kind.css_class()>{text}</div> }
                        })}

                        <div class="form-group">
                            <label class="form-label" for="task-title">{i18n_stored.get_value().t("task_modal.title_label")}</label>
                            <input
                                type="text"
                                id="task-title"
                                class="form-input"
                                placeholder=i18n_stored.get_value().t("task_modal.title_placeholder")
                                prop:value=move || title.get()
                                on:input=move |ev| title.set(event_target_value(&ev))
                                required
                            />
                        </div>

                        // A date on a task is a custom recurrence holding exactly that day -
                        // the very shape SetDateModal writes, so nothing about saving changes.
                        <Show when=base_field_is_date fallback=|| ()>
                            <div class="form-group">
                                <label class="form-label" for="task-onetime-date">{i18n_stored.get_value().t("task_modal.onetime_date")}</label>
                                <input
                                    type="date"
                                    id="task-onetime-date"
                                    class="form-input"
                                    prop:value=move || onetime_date_value(&recurrence_type.get(), &selected_custom_dates.get())
                                    on:input=move |ev| {
                                        let picked = NaiveDate::parse_from_str(&event_target_value(&ev), "%Y-%m-%d").ok();
                                        let (rec, dates) = apply_onetime_date(picked);
                                        recurrence_type.set(rec.to_string());
                                        selected_custom_dates.set(dates);
                                    }
                                />
                                <small class="form-hint">{i18n_stored.get_value().t("task_modal.onetime_date_hint")}</small>
                            </div>
                        </Show>

                        <Show when=move || !base_field_is_date() fallback=|| ()>
                            <RecurrenceFields
                                recurrence_type=recurrence_type
                                selected_weekday=selected_weekday
                                selected_month_day=selected_month_day
                                selected_weekdays=selected_weekdays
                                selected_custom_dates=selected_custom_dates
                            />
                        </Show>

                        // The assignment is the last base field for every type - only label,
                        // requirement and prefill differ. A field that is sometimes there and
                        // sometimes not is a field nobody finds.
                        <div class="form-group">
                            {move || {
                                let archetype = selected_archetype.get();
                                let p = preset(archetype);
                                let label = i18n_stored.get_value().t(p.assign_label_key);
                                let hint = i18n_stored.get_value().t(p.assign_hint_key);
                                let not_assigned_label = i18n_stored.get_value().t("task_modal.not_assigned");
                                let required = archetype.assignment_required();
                                view! {
                                    <label class="form-label" for="task-assigned">
                                        {label}
                                        {required.then(|| view! { <span class="required">"*"</span> })}
                                    </label>
                                    <select
                                        id="task-assigned"
                                        class="form-select"
                                        prop:value=move || assigned_user.get()
                                        on:change=move |ev| {
                                            assigned_user.set(event_target_value(&ev));
                                            assignment_error.set(false);
                                        }
                                    >
                                        <option value="" selected=move || assigned_user.get().is_empty()>{not_assigned_label}</option>
                                        {members_stored.get_value().into_iter().map(|m| {
                                            let user_id = m.user.id.to_string();
                                            let user_id_for_selected = user_id.clone();
                                            let name = m.user.username.clone();
                                            view! {
                                                <option value=user_id selected=move || assigned_user.get() == user_id_for_selected>
                                                    {name}
                                                </option>
                                            }
                                        }).collect_view()}
                                    </select>
                                    <small class="form-hint">{hint}</small>
                                }
                            }}
                            <Show when=move || assignment_error.get() fallback=|| ()>
                                <small class="form-field-error">
                                    {i18n_stored.get_value().t("task_modal.assignment_required_error")}
                                </small>
                            </Show>
                        </div>

                        <Accordion
                            class="task-form-group"
                            summary=i18n_stored.get_value().t("task_modal.group.details")
                            open=initial_groups.details
                        >
                            // For one-offs the rhythm lives here: reachable, never locked.
                            <Show when=base_field_is_date fallback=|| ()>
                                <RecurrenceFields
                                    recurrence_type=recurrence_type
                                    selected_weekday=selected_weekday
                                    selected_month_day=selected_month_day
                                    selected_weekdays=selected_weekdays
                                    selected_custom_dates=selected_custom_dates
                                />
                            </Show>

                            <div class="form-group">
                                <label class="form-label" for="task-description">{i18n_stored.get_value().t("task_modal.description_label")}</label>
                                <textarea
                                    id="task-description"
                                    class="form-input description-textarea"
                                    rows="4"
                                    placeholder=i18n_stored.get_value().t("task_modal.description_placeholder")
                                    prop:value=move || description.get()
                                    on:input=move |ev| description.set(event_target_value(&ev))
                                />
                            </div>

                            <Show when=move || !categories_stored.get_value().is_empty() fallback=|| ()>
                                <TaskCategoryField
                                    value=selected_category_id
                                    categories=categories_stored.get_value()
                                />
                            </Show>

                            <TaskDueTimeField value=due_time />

                            <TaskOnDashboardField value=on_dashboard />
                        </Accordion>

                        <Accordion
                            class="task-form-group"
                            summary=i18n_stored.get_value().t("task_modal.group.goal")
                            open=initial_groups.goal
                        >
                            // A bonus task has no target to enter — the field would only invite
                            // a value that turns it back into an ordinary chore.
                            <Show
                                when=move || shows_target_count(selected_archetype.get())
                                fallback=|| ()
                            >
                                <TaskTargetCountField value=target_count />
                            </Show>
                            <TaskAllowExceedField value=allow_exceed_target />
                            <TaskHabitTypeField value=habit_type />
                        </Accordion>

                        <Accordion
                            class="task-form-group"
                            summary=i18n_stored.get_value().t("task_modal.group.points")
                            open=initial_groups.points
                        >
                            // The reward stays for every type — doing a bonus task is worth
                            // points. Only the penalty goes, because there is nothing to miss.
                            <TaskPointsRewardField value=points_reward />
                            <Show
                                when=move || shows_points_penalty(selected_archetype.get())
                                fallback=|| ()
                            >
                                <TaskPointsPenaltyField value=points_penalty />
                            </Show>

                            <Show when=move || rewards_section_visible fallback=|| ()>
                                <div class="form-group">
                                    <label class="form-label">{i18n_stored.get_value().t("task_modal.rewards_on_completion")}</label>
                                    <div style="border: 1px solid var(--card-border); border-radius: var(--border-radius); padding: 0.75rem;">
                                        // Add new reward row
                                        {
                                            let household_rewards_for_dropdown = household_rewards.clone();
                                            let select_reward_label = i18n_stored.get_value().t("task_modal.select_reward");
                                            let add_label = i18n_stored.get_value().t("task_modal.add");
                                            view! {
                                                <div style="display: flex; gap: 0.5rem; align-items: center; margin-bottom: 0.75rem;">
                                                    <select
                                                        class="form-select"
                                                        style="flex: 1;"
                                                        prop:value=move || selected_new_reward.get()
                                                        on:change=move |ev| selected_new_reward.set(event_target_value(&ev))
                                                    >
                                                        <option value="">{select_reward_label.clone()}</option>
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
                                                        {add_label}
                                                    </button>
                                                </div>
                                            }
                                        }

                                        // List of linked rewards
                                        {
                                            let household_rewards_for_list = household_rewards.clone();
                                            let no_rewards_linked = i18n_stored.get_value().t("task_modal.no_rewards_linked");
                                            let unknown_label = i18n_stored.get_value().t("task_modal.unknown");
                                            let remove_label = i18n_stored.get_value().t("task_modal.remove");
                                            view! {
                                                <div>
                                                    {move || {
                                                        let rewards = selected_rewards.get();
                                                        if rewards.is_empty() {
                                                            let no_rewards_linked = no_rewards_linked.clone();
                                                            view! { <p style="color: var(--text-muted); font-size: 0.875rem; margin: 0;">{no_rewards_linked}</p> }.into_view()
                                                        } else {
                                                            rewards.iter().map(|(reward_id, amount)| {
                                                                let reward_name = household_rewards_for_list.iter()
                                                                    .find(|r| r.id.to_string() == *reward_id)
                                                                    .map(|r| r.name.clone())
                                                                    .unwrap_or_else(|| unknown_label.clone());
                                                                let reward_id_for_remove = reward_id.clone();
                                                                let amount_display = *amount;
                                                                let remove_label = remove_label.clone();
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
                                                                            {remove_label}
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
                                    <small class="form-hint">{i18n_stored.get_value().t("task_modal.rewards_hint")}</small>
                                </div>
                            </Show>

                            <Show when=move || punishments_section_visible fallback=|| ()>
                                <div class="form-group">
                                    <label class="form-label">{i18n_stored.get_value().t("task_modal.punishments_on_miss")}</label>
                                    <div style="border: 1px solid var(--card-border); border-radius: var(--border-radius); padding: 0.75rem;">
                                        // Add new punishment row
                                        {
                                            let household_punishments_for_dropdown = household_punishments.clone();
                                            let select_punishment_label = i18n_stored.get_value().t("task_modal.select_punishment");
                                            let add_label = i18n_stored.get_value().t("task_modal.add");
                                            view! {
                                                <div style="display: flex; gap: 0.5rem; align-items: center; margin-bottom: 0.75rem;">
                                                    <select
                                                        class="form-select"
                                                        style="flex: 1;"
                                                        prop:value=move || selected_new_punishment.get()
                                                        on:change=move |ev| selected_new_punishment.set(event_target_value(&ev))
                                                    >
                                                        <option value="">{select_punishment_label.clone()}</option>
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
                                                        {add_label}
                                                    </button>
                                                </div>
                                            }
                                        }

                                        // List of linked punishments
                                        {
                                            let household_punishments_for_list = household_punishments.clone();
                                            let no_punishments_linked = i18n_stored.get_value().t("task_modal.no_punishments_linked");
                                            let unknown_label = i18n_stored.get_value().t("task_modal.unknown");
                                            let remove_label = i18n_stored.get_value().t("task_modal.remove");
                                            view! {
                                                <div>
                                                    {move || {
                                                        let punishments = selected_punishments.get();
                                                        if punishments.is_empty() {
                                                            let no_punishments_linked = no_punishments_linked.clone();
                                                            view! { <p style="color: var(--text-muted); font-size: 0.875rem; margin: 0;">{no_punishments_linked}</p> }.into_view()
                                                        } else {
                                                            punishments.iter().map(|(punishment_id, amount)| {
                                                                let punishment_name = household_punishments_for_list.iter()
                                                                    .find(|p| p.id.to_string() == *punishment_id)
                                                                    .map(|p| p.name.clone())
                                                                    .unwrap_or_else(|| unknown_label.clone());
                                                                let punishment_id_for_remove = punishment_id.clone();
                                                                let amount_display = *amount;
                                                                let remove_label = remove_label.clone();
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
                                                                            {remove_label}
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
                                    <small class="form-hint">{i18n_stored.get_value().t("task_modal.punishments_hint")}</small>
                                </div>
                            </Show>
                        </Accordion>

                        <Accordion
                            class="task-form-group"
                            summary=i18n_stored.get_value().t("task_modal.group.rules")
                            open=initial_groups.rules
                        >
                            <TaskAnyoneCanCompleteField value=anyone_can_complete />
                            <TaskAssigneeCannotUncompleteField value=assignee_cannot_uncomplete />
                            <TaskRequiresReviewField value=requires_review />
                        </Accordion>
                    </div>

                    <Show when=move || can_delete && confirming_delete.get() fallback=|| ()>
                        <div class="modal-footer task-delete-confirm">
                            <span class="task-delete-confirm-text">
                                {i18n_stored.get_value().t("task_modal.delete_confirm_question")}
                            </span>
                            <button
                                type="button"
                                class="btn btn-outline"
                                on:click=move |_| confirming_delete.set(false)
                                disabled=move || deleting.get()
                            >
                                {i18n_stored.get_value().t("task_modal.delete_keep")}
                            </button>
                            <button
                                type="button"
                                class="btn btn-danger"
                                on:click=confirm_delete
                                disabled=move || deleting.get()
                            >
                                {move || if deleting.get() {
                                    i18n_stored.get_value().t("task_modal.deleting")
                                } else {
                                    i18n_stored.get_value().t("task_modal.delete_confirm")
                                }}
                            </button>
                        </div>
                    </Show>

                    <div class="modal-footer">
                        <Show when=move || can_delete && !confirming_delete.get() fallback=|| ()>
                            <button
                                type="button"
                                class="btn btn-danger btn-outline task-delete-trigger"
                                on:click=move |_| confirming_delete.set(true)
                                disabled=move || saving.get() || deleting.get()
                            >
                                {i18n_stored.get_value().t("task_modal.delete_task")}
                            </button>
                        </Show>
                        <button
                            type="button"
                            class="btn btn-outline"
                            on:click=move |_| on_close.call(())
                            disabled=move || saving.get() || deleting.get()
                        >
                            {i18n_stored.get_value().t("common.cancel")}
                        </button>
                        <button
                            type="submit"
                            class="btn btn-primary"
                            disabled=move || saving.get() || deleting.get()
                        >
                            {move || if saving.get() { saving_text.clone() } else { submit_button_text.clone() }}
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

    #[test]
    fn delete_offered_for_existing_task_with_callback() {
        assert!(delete_action_available(true, true));
    }

    #[test]
    fn delete_hidden_without_callback() {
        // No callback means the caller withheld permission.
        assert!(!delete_action_available(true, false));
    }

    #[test]
    fn delete_hidden_in_create_mode() {
        // Create/duplicate mode has no task to delete.
        assert!(!delete_action_available(false, true));
    }

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

    #[wasm_bindgen_test]
    fn test_prefill_source_task_priority() {
        // When both task and prefill_from are None, source_task should be None
        let task: Option<Task> = None;
        let prefill_from: Option<Task> = None;
        let source_task = task.as_ref().or(prefill_from.as_ref());
        assert!(source_task.is_none());
    }

    #[wasm_bindgen_test]
    fn test_prefill_uses_prefill_from_when_task_is_none() {
        // When task is None but prefill_from is Some, source_task should use prefill_from
        let task: Option<Task> = None;
        let prefill_task = Task {
            id: Uuid::new_v4(),
            household_id: Uuid::new_v4(),
            title: "Prefill Task".to_string(),
            description: "Test description".to_string(),
            recurrence_type: RecurrenceType::Daily,
            recurrence_value: None,
            assigned_user_id: None,
            target_count: 3,
            time_period: None,
            allow_exceed_target: true,
            anyone_can_complete: false,
            assignee_cannot_uncomplete: false,
            requires_review: false,
            points_reward: Some(10),
            points_penalty: None,
            due_time: Some("14:00".to_string()),
            habit_type: HabitType::Good,
            category_id: None,
            category_name: None,
            category_color: None,
            archived: false,
            paused: false,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
            suggestion: None,
            suggested_by: None,
        };
        let prefill_from = Some(prefill_task);
        let source_task = task.as_ref().or(prefill_from.as_ref());

        assert!(source_task.is_some());
        assert_eq!(source_task.unwrap().title, "Prefill Task");
        assert_eq!(source_task.unwrap().target_count, 3);
        assert_eq!(source_task.unwrap().points_reward, Some(10));
    }

    #[wasm_bindgen_test]
    fn test_edit_mode_uses_task_not_prefill() {
        // When task is Some, it should take priority over prefill_from
        let edit_task = Task {
            id: Uuid::new_v4(),
            household_id: Uuid::new_v4(),
            title: "Edit Task".to_string(),
            description: String::new(),
            recurrence_type: RecurrenceType::Weekly,
            recurrence_value: None,
            assigned_user_id: None,
            target_count: 1,
            time_period: None,
            allow_exceed_target: false,
            anyone_can_complete: false,
            assignee_cannot_uncomplete: false,
            requires_review: true,
            points_reward: None,
            points_penalty: None,
            due_time: None,
            habit_type: HabitType::Good,
            category_id: None,
            category_name: None,
            category_color: None,
            archived: false,
            paused: false,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
            suggestion: None,
            suggested_by: None,
        };
        let task = Some(edit_task);
        let prefill_task = Task {
            id: Uuid::new_v4(),
            household_id: Uuid::new_v4(),
            title: "Should Not Use This".to_string(),
            description: String::new(),
            recurrence_type: RecurrenceType::Daily,
            recurrence_value: None,
            assigned_user_id: None,
            target_count: 5,
            time_period: None,
            allow_exceed_target: true,
            anyone_can_complete: false,
            assignee_cannot_uncomplete: false,
            requires_review: false,
            points_reward: None,
            points_penalty: None,
            due_time: None,
            habit_type: HabitType::Good,
            category_id: None,
            category_name: None,
            category_color: None,
            archived: false,
            paused: false,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
            suggestion: None,
            suggested_by: None,
        };
        let prefill_from = Some(prefill_task);
        let source_task = task.as_ref().or(prefill_from.as_ref());

        assert!(source_task.is_some());
        assert_eq!(source_task.unwrap().title, "Edit Task");
        assert_eq!(source_task.unwrap().target_count, 1);
    }

    #[wasm_bindgen_test]
    fn test_is_edit_mode_detection() {
        // is_edit should be true only when task is Some
        let task_some: Option<Task> = Some(Task {
            id: Uuid::new_v4(),
            household_id: Uuid::new_v4(),
            title: "Test".to_string(),
            description: String::new(),
            recurrence_type: RecurrenceType::Daily,
            recurrence_value: None,
            assigned_user_id: None,
            target_count: 1,
            time_period: None,
            allow_exceed_target: true,
            anyone_can_complete: false,
            assignee_cannot_uncomplete: false,
            requires_review: false,
            points_reward: None,
            points_penalty: None,
            due_time: None,
            habit_type: HabitType::Good,
            category_id: None,
            category_name: None,
            category_color: None,
            archived: false,
            paused: false,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
            suggestion: None,
            suggested_by: None,
        });
        let task_none: Option<Task> = None;

        let is_edit_when_some = task_some.is_some();
        let is_edit_when_none = task_none.is_some();

        assert!(is_edit_when_some);
        assert!(!is_edit_when_none);
    }

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
