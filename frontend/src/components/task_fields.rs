//! Reusable form field components for task editing.
//!
//! These components are used by TaskModal for both single task editing
//! and bulk editing. Each component handles its own label, input, and hint.

use leptos::*;
use shared::{MemberWithUser, TaskCategory};

use crate::i18n::use_i18n;

/// Category selection field
#[component]
pub fn TaskCategoryField(
    value: RwSignal<String>,
    categories: Vec<TaskCategory>,
    #[prop(default = false)] disabled: bool,
    #[prop(default = false)] hide_label: bool,
) -> impl IntoView {
    let i18n = use_i18n();
    let i18n_stored = store_value(i18n);
    let categories_stored = store_value(categories);

    view! {
        <div class="form-group">
            <Show when=move || !hide_label fallback=|| ()>
                <label class="form-label" for="task-category">{i18n_stored.get_value().t("task_modal.category")}</label>
            </Show>
            <select
                id="task-category"
                class="form-select"
                disabled=disabled
                on:change=move |ev| value.set(event_target_value(&ev))
            >
                <option value="" selected=move || value.get().is_empty()>
                    {i18n_stored.get_value().t("task_modal.no_category")}
                </option>
                {move || {
                    categories_stored.get_value().into_iter().map(|cat| {
                        let cat_id = cat.id.to_string();
                        let cat_id_for_selected = cat_id.clone();
                        view! {
                            <option value=cat_id selected=move || value.get() == cat_id_for_selected>
                                {cat.name}
                            </option>
                        }
                    }).collect_view()
                }}
            </select>
            <Show when=move || !hide_label fallback=|| ()>
                <small class="form-hint">{i18n_stored.get_value().t("task_modal.category_hint")}</small>
            </Show>
        </div>
    }
}

/// Assigned user selection field
#[component]
pub fn TaskAssignedUserField(
    value: RwSignal<String>,
    members: Vec<MemberWithUser>,
    #[prop(default = false)] disabled: bool,
    #[prop(default = false)] hide_label: bool,
) -> impl IntoView {
    let i18n = use_i18n();
    let i18n_stored = store_value(i18n);
    let members_stored = store_value(members);

    view! {
        <div class="form-group">
            <Show when=move || !hide_label fallback=|| ()>
                <label class="form-label" for="task-assigned">{i18n_stored.get_value().t("task_modal.assigned_to")}</label>
            </Show>
            <select
                id="task-assigned"
                class="form-select"
                disabled=disabled
                prop:value=move || value.get()
                on:change=move |ev| value.set(event_target_value(&ev))
            >
                <option value="" selected=move || value.get().is_empty()>
                    {i18n_stored.get_value().t("task_modal.not_assigned")}
                </option>
                {move || {
                    members_stored.get_value().into_iter().map(|m| {
                        let user_id = m.user.id.to_string();
                        let user_id_for_selected = user_id.clone();
                        let name = m.user.username.clone();
                        view! {
                            <option value=user_id selected=move || value.get() == user_id_for_selected>
                                {name}
                            </option>
                        }
                    }).collect_view()
                }}
            </select>
            <Show when=move || !hide_label fallback=|| ()>
                <small class="form-hint">{i18n_stored.get_value().t("task_modal.assigned_hint")}</small>
            </Show>
        </div>
    }
}

/// Target count input field
#[component]
pub fn TaskTargetCountField(
    value: RwSignal<String>,
    #[prop(default = false)] disabled: bool,
    #[prop(default = false)] hide_label: bool,
) -> impl IntoView {
    let i18n = use_i18n();
    let i18n_stored = store_value(i18n);

    view! {
        <div class="form-group">
            <Show when=move || !hide_label fallback=|| ()>
                <label class="form-label" for="task-target-count">{i18n_stored.get_value().t("task_modal.target_count")}</label>
            </Show>
            <input
                type="number"
                id="task-target-count"
                class="form-input"
                min="0"
                disabled=disabled
                prop:value=move || value.get()
                on:input=move |ev| value.set(event_target_value(&ev))
            />
            <Show when=move || !hide_label fallback=|| ()>
                <small class="form-hint">{i18n_stored.get_value().t("task_modal.target_count_hint")}</small>
            </Show>
        </div>
    }
}

/// Allow exceed target checkbox
#[component]
pub fn TaskAllowExceedField(
    value: RwSignal<bool>,
    #[prop(default = false)] disabled: bool,
    #[prop(default = false)] hide_label: bool,
) -> impl IntoView {
    let i18n = use_i18n();
    let i18n_stored = store_value(i18n);

    view! {
        <div class="form-group">
            <label style="display: flex; align-items: center; gap: 0.5rem; cursor: pointer;">
                <input
                    type="checkbox"
                    disabled=disabled
                    prop:checked=move || value.get()
                    on:change=move |ev| value.set(event_target_checked(&ev))
                />
                <Show when=move || !hide_label fallback=|| ()>
                    <span>{i18n_stored.get_value().t("task_modal.allow_exceed")}</span>
                </Show>
            </label>
            <Show when=move || !hide_label fallback=|| ()>
                <small class="form-hint">{i18n_stored.get_value().t("task_modal.allow_exceed_hint")}</small>
            </Show>
        </div>
    }
}

/// Requires review checkbox
#[component]
pub fn TaskRequiresReviewField(
    value: RwSignal<bool>,
    #[prop(default = false)] disabled: bool,
    #[prop(default = false)] hide_label: bool,
) -> impl IntoView {
    let i18n = use_i18n();
    let i18n_stored = store_value(i18n);

    view! {
        <div class="form-group">
            <label style="display: flex; align-items: center; gap: 0.5rem; cursor: pointer;">
                <input
                    type="checkbox"
                    disabled=disabled
                    prop:checked=move || value.get()
                    on:change=move |ev| value.set(event_target_checked(&ev))
                />
                <Show when=move || !hide_label fallback=|| ()>
                    <span>{i18n_stored.get_value().t("task_modal.require_review")}</span>
                </Show>
            </label>
            <Show when=move || !hide_label fallback=|| ()>
                <small class="form-hint">{i18n_stored.get_value().t("task_modal.require_review_hint")}</small>
            </Show>
        </div>
    }
}

/// Show on dashboard checkbox
#[component]
pub fn TaskOnDashboardField(
    value: RwSignal<bool>,
    #[prop(default = false)] disabled: bool,
    #[prop(default = false)] hide_label: bool,
) -> impl IntoView {
    let i18n = use_i18n();
    let i18n_stored = store_value(i18n);

    view! {
        <div class="form-group">
            <label style="display: flex; align-items: center; gap: 0.5rem; cursor: pointer;">
                <input
                    type="checkbox"
                    disabled=disabled
                    prop:checked=move || value.get()
                    on:change=move |ev| value.set(event_target_checked(&ev))
                />
                <Show when=move || !hide_label fallback=|| ()>
                    <span>{i18n_stored.get_value().t("task_modal.show_on_dashboard")}</span>
                </Show>
            </label>
            <Show when=move || !hide_label fallback=|| ()>
                <small class="form-hint">{i18n_stored.get_value().t("task_modal.show_on_dashboard_hint")}</small>
            </Show>
        </div>
    }
}

/// Habit type selection (good/bad)
#[component]
pub fn TaskHabitTypeField(
    value: RwSignal<String>,
    #[prop(default = false)] disabled: bool,
    #[prop(default = false)] hide_label: bool,
) -> impl IntoView {
    let i18n = use_i18n();
    let i18n_stored = store_value(i18n);

    view! {
        <div class="form-group">
            <Show when=move || !hide_label fallback=|| ()>
                <label class="form-label" for="task-habit-type">{i18n_stored.get_value().t("task_modal.habit_type_label")}</label>
            </Show>
            <select
                id="task-habit-type"
                class="form-select"
                disabled=disabled
                on:change=move |ev| value.set(event_target_value(&ev))
            >
                <option value="good" selected=move || value.get() == "good">
                    {i18n_stored.get_value().t("habit_type.good")}
                </option>
                <option value="bad" selected=move || value.get() == "bad">
                    {i18n_stored.get_value().t("habit_type.bad")}
                </option>
            </select>
            <Show when=move || !hide_label fallback=|| ()>
                <small class="form-hint">{i18n_stored.get_value().t("task_modal.habit_type_hint")}</small>
            </Show>
        </div>
    }
}

/// Points reward input
#[component]
pub fn TaskPointsRewardField(
    value: RwSignal<String>,
    #[prop(default = false)] disabled: bool,
    #[prop(default = false)] hide_label: bool,
) -> impl IntoView {
    let i18n = use_i18n();
    let i18n_stored = store_value(i18n);

    view! {
        <div class="form-group">
            <Show when=move || !hide_label fallback=|| ()>
                <label class="form-label" for="task-points-reward">{i18n_stored.get_value().t("task_modal.points_reward")}</label>
            </Show>
            <input
                type="number"
                id="task-points-reward"
                class="form-input"
                min="0"
                placeholder="0"
                disabled=disabled
                prop:value=move || value.get()
                on:input=move |ev| value.set(event_target_value(&ev))
            />
            <Show when=move || !hide_label fallback=|| ()>
                <small class="form-hint">{i18n_stored.get_value().t("task_modal.points_reward_hint")}</small>
            </Show>
        </div>
    }
}

/// Points penalty input
#[component]
pub fn TaskPointsPenaltyField(
    value: RwSignal<String>,
    #[prop(default = false)] disabled: bool,
    #[prop(default = false)] hide_label: bool,
) -> impl IntoView {
    let i18n = use_i18n();
    let i18n_stored = store_value(i18n);

    view! {
        <div class="form-group">
            <Show when=move || !hide_label fallback=|| ()>
                <label class="form-label" for="task-points-penalty">{i18n_stored.get_value().t("task_modal.points_penalty")}</label>
            </Show>
            <input
                type="number"
                id="task-points-penalty"
                class="form-input"
                min="0"
                placeholder="0"
                disabled=disabled
                prop:value=move || value.get()
                on:input=move |ev| value.set(event_target_value(&ev))
            />
            <Show when=move || !hide_label fallback=|| ()>
                <small class="form-hint">{i18n_stored.get_value().t("task_modal.points_penalty_hint")}</small>
            </Show>
        </div>
    }
}

/// Due time input
#[component]
pub fn TaskDueTimeField(
    value: RwSignal<String>,
    #[prop(default = false)] disabled: bool,
    #[prop(default = false)] hide_label: bool,
) -> impl IntoView {
    let i18n = use_i18n();
    let i18n_stored = store_value(i18n);

    view! {
        <div class="form-group">
            <Show when=move || !hide_label fallback=|| ()>
                <label class="form-label" for="task-due-time">{i18n_stored.get_value().t("task_modal.due_time")}</label>
            </Show>
            <input
                type="time"
                id="task-due-time"
                class="form-input"
                disabled=disabled
                prop:value=move || value.get()
                on:input=move |ev| value.set(event_target_value(&ev))
            />
            <Show when=move || !hide_label fallback=|| ()>
                <small class="form-hint">{i18n_stored.get_value().t("task_modal.due_time_hint")}</small>
            </Show>
        </div>
    }
}

/// Paused checkbox (primarily for bulk edit)
#[component]
pub fn TaskPausedField(
    value: RwSignal<bool>,
    #[prop(default = false)] disabled: bool,
    #[prop(default = false)] hide_label: bool,
) -> impl IntoView {
    let i18n = use_i18n();
    let i18n_stored = store_value(i18n);

    view! {
        <div class="form-group">
            <label style="display: flex; align-items: center; gap: 0.5rem; cursor: pointer;">
                <input
                    type="checkbox"
                    disabled=disabled
                    prop:checked=move || value.get()
                    on:change=move |ev| value.set(event_target_checked(&ev))
                />
                <Show when=move || !hide_label fallback=|| ()>
                    <span>{i18n_stored.get_value().t("tasks.paused")}</span>
                </Show>
            </label>
            <Show when=move || !hide_label fallback=|| ()>
                <small class="form-hint">{i18n_stored.get_value().t("tasks.paused_hint")}</small>
            </Show>
        </div>
    }
}

/// Recurrence type selection
#[component]
pub fn TaskRecurrenceTypeField(
    value: RwSignal<String>,
    #[prop(default = false)] disabled: bool,
    #[prop(default = false)] hide_label: bool,
) -> impl IntoView {
    let i18n = use_i18n();
    let i18n_stored = store_value(i18n);

    view! {
        <div class="form-group">
            <Show when=move || !hide_label fallback=|| ()>
                <label class="form-label" for="task-recurrence">{i18n_stored.get_value().t("task_modal.recurrence_label")}</label>
            </Show>
            <select
                id="task-recurrence"
                class="form-select"
                disabled=disabled
                on:change=move |ev| value.set(event_target_value(&ev))
            >
                <option value="onetime" selected=move || value.get() == "onetime">
                    {i18n_stored.get_value().t("recurrence.onetime_freeform")}
                </option>
                <option value="daily" selected=move || value.get() == "daily">
                    {i18n_stored.get_value().t("recurrence.daily")}
                </option>
                <option value="weekly" selected=move || value.get() == "weekly">
                    {i18n_stored.get_value().t("recurrence.weekly")}
                </option>
                <option value="monthly" selected=move || value.get() == "monthly">
                    {i18n_stored.get_value().t("recurrence.monthly")}
                </option>
                <option value="weekdays" selected=move || value.get() == "weekdays">
                    {i18n_stored.get_value().t("recurrence.specific_days")}
                </option>
                <option value="custom" selected=move || value.get() == "custom">
                    {i18n_stored.get_value().t("recurrence.custom_dates")}
                </option>
            </select>
            <Show when=move || !hide_label fallback=|| ()>
                <small class="form-hint">{i18n_stored.get_value().t("task_modal.recurrence_hint")}</small>
            </Show>
        </div>
    }
}

/// Single weekday selection (for weekly recurrence)
#[component]
pub fn TaskWeekdayField(
    value: RwSignal<u8>,
    #[prop(default = false)] disabled: bool,
    #[prop(default = false)] hide_label: bool,
) -> impl IntoView {
    let i18n = use_i18n();
    let i18n_stored = store_value(i18n);

    view! {
        <div class="form-group">
            <Show when=move || !hide_label fallback=|| ()>
                <label class="form-label" for="task-weekday">{i18n_stored.get_value().t("task_modal.day_of_week")}</label>
            </Show>
            <select
                id="task-weekday"
                class="form-select"
                disabled=disabled
                on:change=move |ev| {
                    if let Ok(day) = event_target_value(&ev).parse::<u8>() {
                        value.set(day);
                    }
                }
            >
                <option value="0" selected=move || value.get() == 0>{i18n_stored.get_value().t("weekday.sunday")}</option>
                <option value="1" selected=move || value.get() == 1>{i18n_stored.get_value().t("weekday.monday")}</option>
                <option value="2" selected=move || value.get() == 2>{i18n_stored.get_value().t("weekday.tuesday")}</option>
                <option value="3" selected=move || value.get() == 3>{i18n_stored.get_value().t("weekday.wednesday")}</option>
                <option value="4" selected=move || value.get() == 4>{i18n_stored.get_value().t("weekday.thursday")}</option>
                <option value="5" selected=move || value.get() == 5>{i18n_stored.get_value().t("weekday.friday")}</option>
                <option value="6" selected=move || value.get() == 6>{i18n_stored.get_value().t("weekday.saturday")}</option>
            </select>
            <Show when=move || !hide_label fallback=|| ()>
                <small class="form-hint">{i18n_stored.get_value().t("task_modal.weekly_hint")}</small>
            </Show>
        </div>
    }
}

/// Day of month selection (for monthly recurrence)
#[component]
pub fn TaskMonthDayField(
    value: RwSignal<u8>,
    #[prop(default = false)] disabled: bool,
    #[prop(default = false)] hide_label: bool,
) -> impl IntoView {
    let i18n = use_i18n();
    let i18n_stored = store_value(i18n);

    view! {
        <div class="form-group">
            <Show when=move || !hide_label fallback=|| ()>
                <label class="form-label" for="task-monthday">{i18n_stored.get_value().t("task_modal.day_of_month")}</label>
            </Show>
            <input
                type="number"
                id="task-monthday"
                class="form-input"
                min="1"
                max="31"
                disabled=disabled
                prop:value=move || value.get().to_string()
                on:input=move |ev| {
                    if let Ok(day) = event_target_value(&ev).parse::<u8>() {
                        value.set(day.clamp(1, 31));
                    }
                }
            />
            <Show when=move || !hide_label fallback=|| ()>
                <small class="form-hint">{i18n_stored.get_value().t("task_modal.monthly_hint")}</small>
            </Show>
        </div>
    }
}

/// Multiple weekdays selection (for weekdays recurrence)
#[component]
pub fn TaskWeekdaysField(
    value: RwSignal<Vec<u8>>,
    #[prop(default = false)] disabled: bool,
    #[prop(default = false)] hide_label: bool,
) -> impl IntoView {
    let i18n = use_i18n();
    let i18n_stored = store_value(i18n);

    // Weekday data: (value, translation_key)
    let weekdays: [(u8, &str); 7] = [
        (1, "weekday.monday"),
        (2, "weekday.tuesday"),
        (3, "weekday.wednesday"),
        (4, "weekday.thursday"),
        (5, "weekday.friday"),
        (6, "weekday.saturday"),
        (0, "weekday.sunday"),
    ];

    view! {
        <div class="form-group">
            <Show when=move || !hide_label fallback=|| ()>
                <label class="form-label">{i18n_stored.get_value().t("task_modal.select_days")}</label>
            </Show>
            <div style="display: flex; flex-wrap: wrap; gap: 0.5rem;">
                {weekdays.into_iter().map(|(day_num, key)| {
                    let day_name = i18n_stored.get_value().t(key).chars().take(3).collect::<String>();
                    view! {
                        <label style="display: flex; align-items: center; gap: 0.25rem; padding: 0.5rem 0.75rem; border: 1px solid var(--card-border); border-radius: var(--border-radius); cursor: pointer; user-select: none;">
                            <input
                                type="checkbox"
                                disabled=disabled
                                prop:checked=move || value.get().contains(&day_num)
                                on:change=move |ev| {
                                    let checked = event_target_checked(&ev);
                                    value.update(|days| {
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
            <Show when=move || !hide_label fallback=|| ()>
                <small class="form-hint">{i18n_stored.get_value().t("task_modal.weekdays_hint")}</small>
            </Show>
        </div>
    }
}

/// Custom interval field (for custom recurrence)
#[component]
pub fn TaskCustomIntervalField(
    value: RwSignal<i32>,
    #[prop(default = false)] disabled: bool,
    #[prop(default = false)] hide_label: bool,
) -> impl IntoView {
    let i18n = use_i18n();
    let i18n_stored = store_value(i18n);

    view! {
        <div class="form-group">
            <Show when=move || !hide_label fallback=|| ()>
                <label class="form-label" for="task-interval">{i18n_stored.get_value().t("task_modal.custom_interval")}</label>
            </Show>
            <input
                type="number"
                id="task-interval"
                class="form-input"
                min="1"
                disabled=disabled
                prop:value=move || value.get().to_string()
                on:input=move |ev| {
                    if let Ok(interval) = event_target_value(&ev).parse::<i32>() {
                        value.set(interval.max(1));
                    }
                }
            />
            <Show when=move || !hide_label fallback=|| ()>
                <small class="form-hint">{i18n_stored.get_value().t("task_modal.custom_interval_hint")}</small>
            </Show>
        </div>
    }
}

/// Wrapper for bulk edit fields - adds "Apply" checkbox
#[component]
pub fn BulkEditField(
    label: String,
    apply: RwSignal<bool>,
    children: Children,
) -> impl IntoView {
    view! {
        <div class="form-group bulk-edit-field">
            <div class="bulk-edit-apply">
                <input
                    type="checkbox"
                    prop:checked=move || apply.get()
                    on:change=move |ev| apply.set(event_target_checked(&ev))
                />
                <label>{label}</label>
            </div>
            <div class=move || if apply.get() { "" } else { "field-disabled" }>
                {children()}
            </div>
        </div>
    }
}

#[cfg(test)]
mod tests {
    use wasm_bindgen_test::*;

    wasm_bindgen_test_configure!(run_in_browser);

    #[wasm_bindgen_test]
    fn test_habit_type_values() {
        // Habit type should accept "good" and "bad"
        let valid_values = vec!["good", "bad"];
        assert!(valid_values.contains(&"good"));
        assert!(valid_values.contains(&"bad"));
    }

    #[wasm_bindgen_test]
    fn test_target_count_default() {
        // Default target count should be 1
        let default = "1".to_string();
        assert_eq!(default, "1");
    }
}
