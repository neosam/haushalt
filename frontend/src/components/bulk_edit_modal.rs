//! Bulk edit for tasks: apply a handful of fields to many tasks at once.
//!
//! Split out of `task_modal.rs`, which used to carry this mode as a second branch
//! next to create/edit/duplicate/suggest.

use leptos::*;
use shared::{
    HabitType, MemberWithUser, RecurrenceType, RecurrenceValue, TaskCategory, UpdateTaskRequest,
};
use uuid::Uuid;

use crate::api::ApiClient;
use crate::components::calendar_picker::CalendarPicker;
use crate::components::task_fields::*;
use crate::i18n::use_i18n;

/// Snapshot of the bulk edit form: which fields are ticked and with which value.
///
/// Deliberately free of signals so that building the request can be tested without
/// a browser. Raw values arrive exactly as the user typed them into the form fields
/// (`*_raw`); parsing happens here in one place.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct BulkEditForm {
    pub apply_category: bool,
    pub category_id_raw: String,
    pub apply_assigned_user: bool,
    pub assigned_user_raw: String,
    pub apply_recurrence: bool,
    pub recurrence_type_raw: String,
    pub weekday: u8,
    pub month_day: u8,
    pub weekdays: Vec<u8>,
    pub custom_dates: Vec<chrono::NaiveDate>,
    pub apply_target_count: bool,
    pub target_count_raw: String,
    pub apply_allow_exceed: bool,
    pub allow_exceed_target: bool,
    pub apply_anyone_can_complete: bool,
    pub anyone_can_complete: bool,
    pub apply_assignee_cannot_uncomplete: bool,
    pub assignee_cannot_uncomplete: bool,
    pub apply_requires_review: bool,
    pub requires_review: bool,
    pub apply_points_reward: bool,
    pub points_reward_raw: String,
    pub apply_points_penalty: bool,
    pub points_penalty_raw: String,
    pub apply_due_time: bool,
    pub due_time_raw: String,
    pub apply_habit_type: bool,
    pub habit_type_raw: String,
    pub apply_paused: bool,
    pub paused: bool,
}

/// Builds the update request from the ticked fields. Unticked fields stay `None`, so
/// the backend leaves them untouched.
///
/// `title` and `description` are always `None` — bulk editing them would give every
/// selected task the same name.
///
/// Not included: dashboard visibility. It does not hang off the task update but off
/// its own endpoints and is handled in the component after the update.
pub fn build_bulk_update_request(form: &BulkEditForm) -> UpdateTaskRequest {
    let category_id = if form.apply_category {
        if form.category_id_raw.is_empty() {
            Some(None)
        } else {
            Some(Uuid::parse_str(&form.category_id_raw).ok())
        }
    } else {
        None
    };

    // Double option, then flattened: an empty selection therefore cannot clear the
    // assignment. Carried over unchanged from the previous behaviour.
    let assigned_user_id = if form.apply_assigned_user {
        if form.assigned_user_raw.is_empty() {
            Some(None)
        } else {
            Some(Uuid::parse_str(&form.assigned_user_raw).ok())
        }
    } else {
        None
    }
    .flatten();

    let recurrence_type = if form.apply_recurrence {
        Some(match form.recurrence_type_raw.as_str() {
            "onetime" => RecurrenceType::OneTime,
            "daily" => RecurrenceType::Daily,
            "weekly" => RecurrenceType::Weekly,
            "monthly" => RecurrenceType::Monthly,
            "weekdays" => RecurrenceType::Weekdays,
            "custom" => RecurrenceType::Custom,
            _ => RecurrenceType::Daily,
        })
    } else {
        None
    };

    let recurrence_value = if form.apply_recurrence {
        match form.recurrence_type_raw.as_str() {
            "weekly" => Some(RecurrenceValue::WeekDay(form.weekday)),
            "monthly" => Some(RecurrenceValue::MonthDay(form.month_day)),
            "weekdays" => Some(RecurrenceValue::Weekdays(form.weekdays.clone())),
            "custom" => Some(RecurrenceValue::CustomDates(form.custom_dates.clone())),
            _ => None, // onetime and daily need no value
        }
    } else {
        None
    };

    UpdateTaskRequest {
        title: None,
        description: None,
        recurrence_type,
        recurrence_value,
        assigned_user_id,
        // Clamped to 0, not to 1: "-5" becomes Some(0).
        target_count: form
            .apply_target_count
            .then(|| form.target_count_raw.parse::<i32>().unwrap_or(1).max(0)),
        time_period: None,
        allow_exceed_target: form.apply_allow_exceed.then_some(form.allow_exceed_target),
        anyone_can_complete: form
            .apply_anyone_can_complete
            .then_some(form.anyone_can_complete),
        assignee_cannot_uncomplete: form
            .apply_assignee_cannot_uncomplete
            .then_some(form.assignee_cannot_uncomplete),
        requires_review: form.apply_requires_review.then_some(form.requires_review),
        points_reward: form
            .apply_points_reward
            .then(|| form.points_reward_raw.parse::<i64>().ok())
            .flatten(),
        points_penalty: form
            .apply_points_penalty
            .then(|| form.points_penalty_raw.parse::<i64>().ok())
            .flatten(),
        // Same as the assignment: an empty time does not clear the due time.
        due_time: form
            .apply_due_time
            .then(|| {
                if form.due_time_raw.is_empty() {
                    None
                } else {
                    Some(form.due_time_raw.clone())
                }
            })
            .flatten(),
        habit_type: if form.apply_habit_type {
            Some(match form.habit_type_raw.as_str() {
                "bad" => HabitType::Bad,
                _ => HabitType::Good,
            })
        } else {
            None
        },
        category_id,
        archived: None,
        paused: form.apply_paused.then_some(form.paused),
    }
}

/// Applies a selection of fields to many tasks at once.
///
/// Every field is opt-in via its own "apply" checkbox; unticked fields are left alone
/// on every selected task. Title and description are not offered at all.
#[component]
pub fn BulkEditModal(
    /// Ids of the selected tasks that get updated together
    bulk_task_ids: Vec<String>,
    household_id: String,
    members: Vec<MemberWithUser>,
    #[prop(default = vec![])] categories: Vec<TaskCategory>,
    #[prop(into)] on_close: Callback<()>,
    /// Receives the number of successfully updated tasks
    #[prop(into)] on_bulk_save: Callback<usize>,
) -> impl IntoView {
    let bulk_task_count = bulk_task_ids.len();

    let members_stored = store_value(members);
    let categories_stored = store_value(categories);

    let error = create_rw_signal(Option::<String>::None);
    let saving = create_rw_signal(false);

    // Form fields
    let selected_category_id = create_rw_signal(String::new());
    // Quirk carried over from the old TaskModal create path: with exactly one
    // assignable member that member is preselected — which also happens here.
    let initial_assigned_user = {
        let members_val = members_stored.get_value();
        if members_val.len() == 1 {
            members_val[0].user.id.to_string()
        } else {
            String::new()
        }
    };
    let assigned_user = create_rw_signal(initial_assigned_user);
    let recurrence_type = create_rw_signal("daily".to_string());
    let target_count = create_rw_signal("1".to_string());
    let allow_exceed_target = create_rw_signal(true);
    let anyone_can_complete = create_rw_signal(false);
    let assignee_cannot_uncomplete = create_rw_signal(false);
    let requires_review = create_rw_signal(false);
    let on_dashboard = create_rw_signal(false);
    let habit_type = create_rw_signal("good".to_string());
    let points_reward = create_rw_signal(String::new());
    let points_penalty = create_rw_signal(String::new());
    let due_time = create_rw_signal(String::new());
    let paused = create_rw_signal(false);

    // Recurrence value signals
    let bulk_selected_weekday = create_rw_signal(1u8); // Monday
    let bulk_selected_month_day = create_rw_signal(1u8);
    let bulk_selected_weekdays = create_rw_signal(Vec::<u8>::new());
    let selected_custom_dates = create_rw_signal(Vec::<chrono::NaiveDate>::new());

    // "Apply" signals - which fields to update
    let apply_category = create_rw_signal(false);
    let apply_assigned_user = create_rw_signal(false);
    let apply_target_count = create_rw_signal(false);
    let apply_allow_exceed = create_rw_signal(false);
    let apply_anyone_can_complete = create_rw_signal(false);
    let apply_assignee_cannot_uncomplete = create_rw_signal(false);
    let apply_requires_review = create_rw_signal(false);
    let apply_on_dashboard = create_rw_signal(false);
    let apply_habit_type = create_rw_signal(false);
    let apply_points_reward = create_rw_signal(false);
    let apply_points_penalty = create_rw_signal(false);
    let apply_due_time = create_rw_signal(false);
    let apply_paused = create_rw_signal(false);
    let apply_recurrence = create_rw_signal(false);

    // Progress state
    let bulk_progress = create_rw_signal((0usize, 0usize)); // (completed, total)
    let bulk_errors = create_rw_signal(Vec::<String>::new());

    let on_bulk_submit = {
        let household_id = household_id.clone();
        let bulk_task_ids = bulk_task_ids.clone();

        move |ev: web_sys::SubmitEvent| {
            ev.prevent_default();
            saving.set(true);
            error.set(None);
            bulk_progress.set((0, bulk_task_ids.len()));
            bulk_errors.set(vec![]);

            let hid = household_id.clone();
            let ids = bulk_task_ids.clone();

            wasm_bindgen_futures::spawn_local(async move {
                let mut success_count = 0;
                let mut error_list = vec![];

                for (idx, task_id) in ids.iter().enumerate() {
                    // Read fresh per task, mirroring the previous inline code that
                    // rebuilt the request on every iteration.
                    let form = BulkEditForm {
                        apply_category: apply_category.get(),
                        category_id_raw: selected_category_id.get(),
                        apply_assigned_user: apply_assigned_user.get(),
                        assigned_user_raw: assigned_user.get(),
                        apply_recurrence: apply_recurrence.get(),
                        recurrence_type_raw: recurrence_type.get(),
                        weekday: bulk_selected_weekday.get(),
                        month_day: bulk_selected_month_day.get(),
                        weekdays: bulk_selected_weekdays.get(),
                        custom_dates: selected_custom_dates.get(),
                        apply_target_count: apply_target_count.get(),
                        target_count_raw: target_count.get(),
                        apply_allow_exceed: apply_allow_exceed.get(),
                        allow_exceed_target: allow_exceed_target.get(),
                        apply_anyone_can_complete: apply_anyone_can_complete.get(),
                        anyone_can_complete: anyone_can_complete.get(),
                        apply_assignee_cannot_uncomplete: apply_assignee_cannot_uncomplete.get(),
                        assignee_cannot_uncomplete: assignee_cannot_uncomplete.get(),
                        apply_requires_review: apply_requires_review.get(),
                        requires_review: requires_review.get(),
                        apply_points_reward: apply_points_reward.get(),
                        points_reward_raw: points_reward.get(),
                        apply_points_penalty: apply_points_penalty.get(),
                        points_penalty_raw: points_penalty.get(),
                        apply_due_time: apply_due_time.get(),
                        due_time_raw: due_time.get(),
                        apply_habit_type: apply_habit_type.get(),
                        habit_type_raw: habit_type.get(),
                        apply_paused: apply_paused.get(),
                        paused: paused.get(),
                    };
                    let request = build_bulk_update_request(&form);

                    match ApiClient::update_task(&hid, task_id, request).await {
                        Ok(_) => {
                            // Dashboard visibility has its own endpoints, so it is not
                            // part of the update request.
                            if apply_on_dashboard.get() {
                                let should_be_on_dashboard = on_dashboard.get();
                                if should_be_on_dashboard {
                                    let _ = ApiClient::add_task_to_dashboard(task_id).await;
                                } else {
                                    let _ = ApiClient::remove_task_from_dashboard(task_id).await;
                                }
                            }
                            success_count += 1;
                        }
                        Err(e) => {
                            error_list.push(format!("Task {}: {}", &task_id[..8], e));
                        }
                    }

                    bulk_progress.set((idx + 1, ids.len()));
                }

                saving.set(false);
                bulk_errors.set(error_list.clone());

                if error_list.is_empty() {
                    on_bulk_save.call(success_count);
                }
            });
        }
    };

    let close = move |_| on_close.call(());

    let i18n = use_i18n();
    let i18n_stored = store_value(i18n.clone());

    let modal_title = i18n
        .t("tasks.bulk_edit_title")
        .replace("{count}", &bulk_task_count.to_string());
    let submit_button_text = i18n.t("tasks.edit_selected");
    // Quirk carried over: the old title/button cascade treated bulk edit as neither
    // edit nor suggestion, so it fell through to the create branch. The button
    // therefore reads "Creating…" while a bulk save runs.
    let saving_text = i18n.t("task_modal.creating");

    let members_for_bulk = members_stored.get_value();
    let categories_for_bulk = categories_stored.get_value();

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

                // Progress indicator
                {move || {
                    if saving.get() {
                        let (completed, total) = bulk_progress.get();
                        let percent = if total > 0 { (completed * 100) / total } else { 0 };
                        Some(view! {
                            <div class="bulk-edit-progress" style="margin: 1rem;">
                                <div style="margin-bottom: 0.5rem;">
                                    {i18n_stored.get_value().t("tasks.bulk_edit_progress")
                                        .replace("{current}", &completed.to_string())
                                        .replace("{total}", &total.to_string())}
                                </div>
                                <div class="bulk-edit-progress-bar">
                                    <div class="bulk-edit-progress-fill" style=format!("width: {}%", percent)></div>
                                </div>
                            </div>
                        })
                    } else {
                        None
                    }
                }}

                // Per-task errors
                {move || {
                    let errors = bulk_errors.get();
                    if !errors.is_empty() {
                        Some(view! {
                            <div class="alert alert-error" style="margin: 1rem;">
                                <div style="font-weight: 500; margin-bottom: 0.5rem;">
                                    {i18n_stored.get_value().t("tasks.bulk_edit_partial")
                                        .replace("{success}", &(bulk_task_count - errors.len()).to_string())
                                        .replace("{total}", &bulk_task_count.to_string())
                                        .replace("{failed}", &errors.len().to_string())}
                                </div>
                                <ul style="margin: 0; padding-left: 1rem;">
                                    {errors.iter().map(|e| view! { <li>{e}</li> }).collect_view()}
                                </ul>
                            </div>
                        })
                    } else {
                        None
                    }
                }}

                <form on:submit=on_bulk_submit>
                    <div style="padding: 1rem; max-height: 60vh; overflow-y: auto;">
                        <div class="alert alert-info" style="margin-bottom: 1rem;">
                            {i18n_stored.get_value().t("tasks.bulk_edit_hint")}
                        </div>

                        // Field order matches the regular edit dialog
                        // Category
                        <BulkEditField label=i18n_stored.get_value().t("task_modal.category") apply=apply_category>
                            <TaskCategoryField value=selected_category_id categories=categories_for_bulk hide_label=true />
                        </BulkEditField>

                        // Recurrence
                        <BulkEditField label=i18n_stored.get_value().t("task_modal.recurrence_label") apply=apply_recurrence>
                            <TaskRecurrenceTypeField value=recurrence_type hide_label=true />
                        </BulkEditField>

                        // Conditional recurrence value fields based on selected type
                        <Show when=move || apply_recurrence.get() && recurrence_type.get() == "weekly" fallback=|| ()>
                            <div class="form-group" style="margin-left: 1.5rem;">
                                <TaskWeekdayField value=bulk_selected_weekday hide_label=false />
                            </div>
                        </Show>

                        <Show when=move || apply_recurrence.get() && recurrence_type.get() == "monthly" fallback=|| ()>
                            <div class="form-group" style="margin-left: 1.5rem;">
                                <TaskMonthDayField value=bulk_selected_month_day hide_label=false />
                            </div>
                        </Show>

                        <Show when=move || apply_recurrence.get() && recurrence_type.get() == "weekdays" fallback=|| ()>
                            <div class="form-group" style="margin-left: 1.5rem;">
                                <TaskWeekdaysField value=bulk_selected_weekdays hide_label=false />
                            </div>
                        </Show>

                        <Show when=move || apply_recurrence.get() && recurrence_type.get() == "custom" fallback=|| ()>
                            <div class="form-group" style="margin-left: 1.5rem;">
                                <label class="form-label">{i18n_stored.get_value().t("task_modal.custom_dates")}</label>
                                <CalendarPicker selected_dates=selected_custom_dates />
                                <small class="form-hint">{i18n_stored.get_value().t("task_modal.custom_dates_hint")}</small>
                            </div>
                        </Show>

                        // Target Count
                        <BulkEditField label=i18n_stored.get_value().t("task_modal.target_count") apply=apply_target_count>
                            <TaskTargetCountField value=target_count hide_label=true />
                        </BulkEditField>

                        // Allow Exceed Target
                        <BulkEditField label=i18n_stored.get_value().t("task_modal.allow_exceed") apply=apply_allow_exceed>
                            <TaskAllowExceedField value=allow_exceed_target hide_label=true />
                        </BulkEditField>

                        // Anyone Can Complete
                        <BulkEditField label=i18n_stored.get_value().t("task_modal.anyone_can_complete") apply=apply_anyone_can_complete>
                            <TaskAnyoneCanCompleteField value=anyone_can_complete hide_label=true />
                        </BulkEditField>

                        // Assignee Cannot Uncomplete
                        <BulkEditField label=i18n_stored.get_value().t("task_modal.assignee_cannot_uncomplete") apply=apply_assignee_cannot_uncomplete>
                            <TaskAssigneeCannotUncompleteField value=assignee_cannot_uncomplete hide_label=true />
                        </BulkEditField>

                        // Requires Review
                        <BulkEditField label=i18n_stored.get_value().t("task_modal.require_review") apply=apply_requires_review>
                            <TaskRequiresReviewField value=requires_review hide_label=true />
                        </BulkEditField>

                        // Show on Dashboard
                        <BulkEditField label=i18n_stored.get_value().t("task_modal.show_on_dashboard") apply=apply_on_dashboard>
                            <TaskOnDashboardField value=on_dashboard hide_label=true />
                        </BulkEditField>

                        // Habit Type
                        <BulkEditField label=i18n_stored.get_value().t("task_modal.habit_type_label") apply=apply_habit_type>
                            <TaskHabitTypeField value=habit_type hide_label=true />
                        </BulkEditField>

                        // Points Reward
                        <BulkEditField label=i18n_stored.get_value().t("task_modal.points_reward") apply=apply_points_reward>
                            <TaskPointsRewardField value=points_reward hide_label=true />
                        </BulkEditField>

                        // Points Penalty
                        <BulkEditField label=i18n_stored.get_value().t("task_modal.points_penalty") apply=apply_points_penalty>
                            <TaskPointsPenaltyField value=points_penalty hide_label=true />
                        </BulkEditField>

                        // Due Time
                        <BulkEditField label=i18n_stored.get_value().t("task_modal.due_time") apply=apply_due_time>
                            <TaskDueTimeField value=due_time hide_label=true />
                        </BulkEditField>

                        // Assigned User
                        <BulkEditField label=i18n_stored.get_value().t("task_modal.assigned_to") apply=apply_assigned_user>
                            <TaskAssignedUserField value=assigned_user members=members_for_bulk hide_label=true />
                        </BulkEditField>

                        // Paused (bulk-edit specific)
                        <BulkEditField label=i18n_stored.get_value().t("tasks.paused") apply=apply_paused>
                            <TaskPausedField value=paused hide_label=true />
                        </BulkEditField>
                    </div>

                    <div class="modal-footer">
                        <button
                            type="button"
                            class="btn btn-outline"
                            on:click=move |_| on_close.call(())
                            disabled=move || saving.get()
                        >
                            {i18n_stored.get_value().t("common.cancel")}
                        </button>
                        <button
                            type="submit"
                            class="btn btn-primary"
                            disabled=move || saving.get()
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

    const SAMPLE_UUID: &str = "550e8400-e29b-41d4-a716-446655440000";

    fn form() -> BulkEditForm {
        BulkEditForm::default()
    }

    /// Everything the user ticked off, so that "always None" claims can be checked
    /// against a maximally filled form.
    fn fully_applied_form() -> BulkEditForm {
        BulkEditForm {
            apply_category: true,
            category_id_raw: SAMPLE_UUID.to_string(),
            apply_assigned_user: true,
            assigned_user_raw: SAMPLE_UUID.to_string(),
            apply_recurrence: true,
            recurrence_type_raw: "weekly".to_string(),
            weekday: 3,
            month_day: 5,
            weekdays: Vec::new(),
            custom_dates: Vec::new(),
            apply_target_count: true,
            target_count_raw: "7".to_string(),
            apply_allow_exceed: true,
            allow_exceed_target: true,
            apply_anyone_can_complete: true,
            anyone_can_complete: true,
            apply_assignee_cannot_uncomplete: true,
            assignee_cannot_uncomplete: true,
            apply_requires_review: true,
            requires_review: true,
            apply_points_reward: true,
            points_reward_raw: "42".to_string(),
            apply_points_penalty: true,
            points_penalty_raw: "13".to_string(),
            apply_due_time: true,
            due_time_raw: "07:30".to_string(),
            apply_habit_type: true,
            habit_type_raw: "bad".to_string(),
            apply_paused: true,
            paused: true,
        }
    }

    #[test]
    fn nothing_applied_leaves_every_field_untouched() {
        let request = build_bulk_update_request(&form());

        assert!(request.title.is_none());
        assert!(request.description.is_none());
        assert!(request.recurrence_type.is_none());
        assert!(request.recurrence_value.is_none());
        assert!(request.assigned_user_id.is_none());
        assert!(request.target_count.is_none());
        assert!(request.time_period.is_none());
        assert!(request.allow_exceed_target.is_none());
        assert!(request.anyone_can_complete.is_none());
        assert!(request.assignee_cannot_uncomplete.is_none());
        assert!(request.requires_review.is_none());
        assert!(request.points_reward.is_none());
        assert!(request.points_penalty.is_none());
        assert!(request.due_time.is_none());
        assert!(request.habit_type.is_none());
        assert!(request.category_id.is_none());
        assert!(request.archived.is_none());
        assert!(request.paused.is_none());
    }

    #[test]
    fn title_and_description_are_never_bulk_updated() {
        // Renaming every selected task alike is never what the user wants.
        let request = build_bulk_update_request(&fully_applied_form());

        assert!(request.title.is_none());
        assert!(request.description.is_none());
    }

    #[test]
    fn time_period_and_archived_are_never_bulk_updated() {
        let request = build_bulk_update_request(&fully_applied_form());

        assert!(request.time_period.is_none());
        assert!(request.archived.is_none());
    }

    #[test]
    fn empty_category_clears_the_category() {
        let request = build_bulk_update_request(&BulkEditForm {
            apply_category: true,
            ..form()
        });

        assert_eq!(request.category_id, Some(None));
    }

    #[test]
    fn valid_category_uuid_is_applied() {
        let request = build_bulk_update_request(&BulkEditForm {
            apply_category: true,
            category_id_raw: SAMPLE_UUID.to_string(),
            ..form()
        });

        assert_eq!(request.category_id, Some(Uuid::parse_str(SAMPLE_UUID).ok()));
    }

    #[test]
    fn unparsable_category_clears_the_category() {
        // A garbage id is indistinguishable from "no category" here.
        let request = build_bulk_update_request(&BulkEditForm {
            apply_category: true,
            category_id_raw: "not-a-uuid".to_string(),
            ..form()
        });

        assert_eq!(request.category_id, Some(None));
    }

    #[test]
    fn empty_assignment_cannot_clear_the_assignee() {
        // Quirk carried over: the flattened double option swallows the clear.
        let request = build_bulk_update_request(&BulkEditForm {
            apply_assigned_user: true,
            ..form()
        });

        assert!(request.assigned_user_id.is_none());
    }

    #[test]
    fn valid_assignee_uuid_is_applied() {
        let request = build_bulk_update_request(&BulkEditForm {
            apply_assigned_user: true,
            assigned_user_raw: SAMPLE_UUID.to_string(),
            ..form()
        });

        assert_eq!(request.assigned_user_id, Uuid::parse_str(SAMPLE_UUID).ok());
    }

    #[test]
    fn empty_due_time_cannot_clear_the_due_time() {
        // Same flattening quirk as the assignment.
        let request = build_bulk_update_request(&BulkEditForm {
            apply_due_time: true,
            ..form()
        });

        assert!(request.due_time.is_none());
    }

    #[test]
    fn due_time_is_applied_verbatim() {
        let request = build_bulk_update_request(&BulkEditForm {
            apply_due_time: true,
            due_time_raw: "07:30".to_string(),
            ..form()
        });

        assert_eq!(request.due_time.as_deref(), Some("07:30"));
    }

    #[test]
    fn unparsable_target_count_falls_back_to_one() {
        let request = build_bulk_update_request(&BulkEditForm {
            apply_target_count: true,
            target_count_raw: "invalid".to_string(),
            ..form()
        });

        assert_eq!(request.target_count, Some(1));
    }

    #[test]
    fn negative_target_count_is_clamped_to_zero() {
        // Quirk carried over: `.max(0)`, not `.max(1)`.
        let request = build_bulk_update_request(&BulkEditForm {
            apply_target_count: true,
            target_count_raw: "-5".to_string(),
            ..form()
        });

        assert_eq!(request.target_count, Some(0));
    }

    #[test]
    fn empty_points_reward_is_skipped() {
        let request = build_bulk_update_request(&BulkEditForm {
            apply_points_reward: true,
            ..form()
        });

        assert!(request.points_reward.is_none());
    }

    #[test]
    fn points_reward_is_parsed() {
        let request = build_bulk_update_request(&BulkEditForm {
            apply_points_reward: true,
            points_reward_raw: "42".to_string(),
            ..form()
        });

        assert_eq!(request.points_reward, Some(42));
    }

    #[test]
    fn points_penalty_is_parsed() {
        let request = build_bulk_update_request(&BulkEditForm {
            apply_points_penalty: true,
            points_penalty_raw: "13".to_string(),
            ..form()
        });

        assert_eq!(request.points_penalty, Some(13));
    }

    #[test]
    fn weekly_recurrence_carries_the_selected_weekday() {
        let request = build_bulk_update_request(&BulkEditForm {
            apply_recurrence: true,
            recurrence_type_raw: "weekly".to_string(),
            weekday: 4,
            ..form()
        });

        assert_eq!(request.recurrence_type, Some(RecurrenceType::Weekly));
        assert_eq!(request.recurrence_value, Some(RecurrenceValue::WeekDay(4)));
    }

    #[test]
    fn monthly_recurrence_carries_the_selected_day_of_month() {
        let request = build_bulk_update_request(&BulkEditForm {
            apply_recurrence: true,
            recurrence_type_raw: "monthly".to_string(),
            month_day: 17,
            ..form()
        });

        assert_eq!(request.recurrence_type, Some(RecurrenceType::Monthly));
        assert_eq!(request.recurrence_value, Some(RecurrenceValue::MonthDay(17)));
    }

    #[test]
    fn weekdays_recurrence_carries_the_selected_days() {
        let request = build_bulk_update_request(&BulkEditForm {
            apply_recurrence: true,
            recurrence_type_raw: "weekdays".to_string(),
            weekdays: [1, 3, 5].to_vec(),
            ..form()
        });

        assert_eq!(request.recurrence_type, Some(RecurrenceType::Weekdays));
        assert_eq!(
            request.recurrence_value,
            Some(RecurrenceValue::Weekdays([1, 3, 5].to_vec()))
        );
    }

    #[test]
    fn daily_recurrence_needs_no_value() {
        let request = build_bulk_update_request(&BulkEditForm {
            apply_recurrence: true,
            recurrence_type_raw: "daily".to_string(),
            ..form()
        });

        assert_eq!(request.recurrence_type, Some(RecurrenceType::Daily));
        assert!(request.recurrence_value.is_none());
    }

    #[test]
    fn onetime_recurrence_needs_no_value() {
        let request = build_bulk_update_request(&BulkEditForm {
            apply_recurrence: true,
            recurrence_type_raw: "onetime".to_string(),
            ..form()
        });

        assert_eq!(request.recurrence_type, Some(RecurrenceType::OneTime));
        assert!(request.recurrence_value.is_none());
    }

    #[test]
    fn unknown_recurrence_falls_back_to_daily() {
        let request = build_bulk_update_request(&BulkEditForm {
            apply_recurrence: true,
            recurrence_type_raw: "fortnightly".to_string(),
            ..form()
        });

        assert_eq!(request.recurrence_type, Some(RecurrenceType::Daily));
        assert!(request.recurrence_value.is_none());
    }

    #[test]
    fn recurrence_is_ignored_when_not_applied() {
        let request = build_bulk_update_request(&BulkEditForm {
            recurrence_type_raw: "weekly".to_string(),
            weekday: 4,
            ..form()
        });

        assert!(request.recurrence_type.is_none());
        assert!(request.recurrence_value.is_none());
    }

    #[test]
    fn pausing_is_applied() {
        let request = build_bulk_update_request(&BulkEditForm {
            apply_paused: true,
            paused: true,
            ..form()
        });

        assert_eq!(request.paused, Some(true));
    }

    #[test]
    fn unpausing_is_applied() {
        let request = build_bulk_update_request(&BulkEditForm {
            apply_paused: true,
            ..form()
        });

        assert_eq!(request.paused, Some(false));
    }

    #[test]
    fn bad_habit_type_is_recognised() {
        let request = build_bulk_update_request(&BulkEditForm {
            apply_habit_type: true,
            habit_type_raw: "bad".to_string(),
            ..form()
        });

        assert_eq!(request.habit_type, Some(HabitType::Bad));
    }

    #[test]
    fn any_other_habit_type_means_good() {
        let request = build_bulk_update_request(&BulkEditForm {
            apply_habit_type: true,
            habit_type_raw: "sideways".to_string(),
            ..form()
        });

        assert_eq!(request.habit_type, Some(HabitType::Good));
    }

    #[test]
    fn booleans_are_applied_independently() {
        let request = build_bulk_update_request(&BulkEditForm {
            apply_allow_exceed: true,
            apply_anyone_can_complete: true,
            anyone_can_complete: true,
            apply_requires_review: true,
            ..form()
        });

        assert_eq!(request.allow_exceed_target, Some(false));
        assert_eq!(request.anyone_can_complete, Some(true));
        assert_eq!(request.requires_review, Some(false));
        assert!(request.assignee_cannot_uncomplete.is_none());
    }
}
