//! Bulk edit for tasks: apply a handful of fields to many tasks at once.
//!
//! Split out of `task_modal.rs`, which used to carry this mode as a second branch
//! next to create/edit/duplicate/suggest.

use shared::{HabitType, RecurrenceType, RecurrenceValue, UpdateTaskRequest};
use uuid::Uuid;

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
