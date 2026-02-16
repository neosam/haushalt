use chrono::{Datelike, NaiveDate, Weekday};
use leptos::*;
use shared::{RecurrenceType, TaskWithStatus};
use std::collections::{BTreeMap, HashSet};
use std::time::Duration;

use crate::i18n::{use_i18n, I18nContext};
use crate::utils::timezone::today_in_tz;

/// Get the translation key for a weekday
fn weekday_translation_key(weekday: Weekday) -> &'static str {
    match weekday {
        Weekday::Mon => "weekday.monday",
        Weekday::Tue => "weekday.tuesday",
        Weekday::Wed => "weekday.wednesday",
        Weekday::Thu => "weekday.thursday",
        Weekday::Fri => "weekday.friday",
        Weekday::Sat => "weekday.saturday",
        Weekday::Sun => "weekday.sunday",
    }
}

/// Get the translation key for a recurrence type
fn recurrence_type_translation_key(recurrence_type: &RecurrenceType) -> &'static str {
    match recurrence_type {
        RecurrenceType::Daily => "recurrence.daily",
        RecurrenceType::Weekly => "recurrence.weekly",
        RecurrenceType::Monthly => "recurrence.monthly",
        RecurrenceType::Weekdays => "recurrence.weekdays",
        RecurrenceType::Custom => "recurrence.custom",
        RecurrenceType::OneTime => "recurrence.onetime",
    }
}

/// Format a next due date for display with translations
fn format_next_due_date(date: NaiveDate, today: NaiveDate, i18n: &I18nContext) -> String {
    let days_until = (date - today).num_days();

    match days_until {
        0 => i18n.t("dates.today"),
        1 => i18n.t("dates.tomorrow"),
        2..=6 => i18n.t(weekday_translation_key(date.weekday())),
        _ => {
            // Show date
            date.format("%b %d").to_string()
        }
    }
}

#[component]
pub fn TaskCard(
    task: TaskWithStatus,
    #[prop(into)] on_complete: Callback<String>,
    #[prop(into)] on_uncomplete: Callback<String>,
    #[prop(default = "UTC".to_string())] timezone: String,
    #[prop(optional)] household_name: Option<String>,
    #[prop(optional)] on_dashboard: Option<bool>,
    #[prop(optional, into)] on_toggle_dashboard: Option<Callback<(String, bool)>>,
) -> impl IntoView {
    let i18n = use_i18n();
    let i18n_stored = store_value(i18n);

    let is_target_met = task.is_target_met();
    let can_complete = task.can_complete();
    let task_id = task.task.id.to_string();
    let task_id_for_minus = task_id.clone();
    let task_id_for_dashboard = task_id.clone();
    let completions = task.completions_today;
    let target = task.task.target_count;
    let has_completions = completions > 0;

    // Dashboard toggle state (reactive for immediate UI feedback)
    let is_on_dashboard = create_rw_signal(on_dashboard.unwrap_or(false));

    // Debounce state
    let is_debouncing = create_rw_signal(false);

    let on_plus = move |_| {
        if can_complete && !is_debouncing.get() {
            is_debouncing.set(true);

            // Set 1-second timeout
            let task_id_clone = task_id.clone();
            set_timeout(
                move || {
                    on_complete.call(task_id_clone);
                    is_debouncing.set(false);
                },
                Duration::from_secs(1)
            );
        }
    };

    let on_minus = move |_| {
        if has_completions {
            on_uncomplete.call(task_id_for_minus.clone());
        }
    };

    let card_class = if is_target_met {
        "task-item task-completed"
    } else {
        "task-item"
    };

    // Progress display as fraction (e.g., "2/3")
    let progress_display = format!("{}/{}", completions, target);

    // Format next due date using household timezone
    let today = today_in_tz(&timezone);
    let next_due_display = task.next_due_date.map(|d| format_next_due_date(d, today, &i18n_stored.get_value()));

    // Format due time (e.g., "14:00")
    let due_time_display = task.task.due_time.clone()
        .map(|time| format!(" ({})", time))
        .unwrap_or_default();

    // Format due label with time
    let due_label = i18n_stored.get_value().t("dates.due");
    let due_display = next_due_display.map(|due| format!(" | {}: {}{}", due_label, due, due_time_display)).unwrap_or_default();

    // Format streak label
    let streak_label = i18n_stored.get_value().t("dates.streak");
    let streak_display = if task.current_streak > 0 {
        format!(" | {}: {}", streak_label, task.current_streak)
    } else {
        String::new()
    };

    // Translate recurrence type
    let recurrence_display = i18n_stored.get_value().t(recurrence_type_translation_key(&task.task.recurrence_type));

    // Bad habit indicator
    let is_bad_habit = task.task.habit_type.is_inverted();
    let bad_habit_label = i18n_stored.get_value().t("habit_type.bad_short");

    // Household name prefix for meta line
    let household_prefix = household_name.map(|name| format!("{} | ", name)).unwrap_or_default();

    // Dashboard toggle handler
    let show_dashboard_toggle = on_toggle_dashboard.is_some();
    let on_dashboard_click = move |_| {
        if let Some(callback) = on_toggle_dashboard {
            let new_state = !is_on_dashboard.get();
            is_on_dashboard.set(new_state);
            callback.call((task_id_for_dashboard.clone(), new_state));
        }
    };

    let dashboard_toggle_title_on = i18n_stored.get_value().t("task_card.remove_from_dashboard");
    let dashboard_toggle_title_off = i18n_stored.get_value().t("task_card.add_to_dashboard");

    view! {
        <div class=card_class>
            <div class="task-content" style="flex: 1;">
                <div class="task-title" style="display: flex; align-items: center; gap: 0.5rem;">
                    {task.task.title.clone()}
                    {if is_bad_habit {
                        view! {
                            <span style="font-size: 0.7rem; padding: 0.1rem 0.4rem; background: var(--danger-color); color: white; border-radius: var(--border-radius); font-weight: 500;">
                                {bad_habit_label}
                            </span>
                        }.into_view()
                    } else {
                        ().into_view()
                    }}
                </div>
                <div class="task-meta">
                    {household_prefix}
                    {recurrence_display}
                    {due_display}
                    {streak_display}
                </div>
            </div>
            <div style="display: flex; align-items: center; gap: 0.5rem;">
                // Dashboard toggle button (star icon)
                {if show_dashboard_toggle {
                    let title_on = dashboard_toggle_title_on.clone();
                    let title_off = dashboard_toggle_title_off.clone();
                    view! {
                        <button
                            class="btn btn-outline"
                            style="padding: 0.25rem 0.5rem; font-size: 1rem; min-width: 32px;"
                            title=move || if is_on_dashboard.get() { title_on.clone() } else { title_off.clone() }
                            on:click=on_dashboard_click.clone()
                        >
                            {move || if is_on_dashboard.get() { "★" } else { "☆" }}
                        </button>
                    }.into_view()
                } else {
                    ().into_view()
                }}
                <button
                    class="btn btn-outline"
                    style="padding: 0.25rem 0.75rem; font-size: 1rem; min-width: 32px;"
                    disabled=!has_completions
                    on:click=on_minus
                >
                    "-"
                </button>
                <span style="font-size: 0.875rem; color: var(--text-muted); min-width: 2rem; text-align: center;">
                    {progress_display}
                </span>
                <button
                    class=move || if is_debouncing.get() { "btn btn-primary btn-debouncing" } else { "btn btn-primary" }
                    style="padding: 0.25rem 0.75rem; font-size: 1rem; min-width: 32px;"
                    disabled=move || !can_complete || is_debouncing.get()
                    on:click=on_plus
                >
                    {move || if is_debouncing.get() { "..." } else { "+" }}
                </button>
            </div>
        </div>
    }
}

#[component]
pub fn TaskList(
    tasks: Vec<TaskWithStatus>,
    #[prop(into)] on_complete: Callback<String>,
    #[prop(into)] on_uncomplete: Callback<String>,
    #[prop(default = "UTC".to_string())] timezone: String,
) -> impl IntoView {
    let i18n = use_i18n();
    let i18n_stored = store_value(i18n);

    view! {
        <div class="card">
            <div class="card-header">
                <h3 class="card-title">{i18n_stored.get_value().t("dates.today")} " - " {i18n_stored.get_value().t("tasks.title")}</h3>
            </div>
            {if tasks.is_empty() {
                view! {
                    <div class="empty-state">
                        <p>{i18n_stored.get_value().t("tasks.no_tasks")}</p>
                    </div>
                }.into_any()
            } else {
                let tz = timezone.clone();
                view! {
                    <div>
                        {tasks.into_iter().map(|task| {
                            let tz = tz.clone();
                            view! { <TaskCard task=task on_complete=on_complete on_uncomplete=on_uncomplete timezone=tz /> }
                        }).collect_view()}
                    </div>
                }.into_any()
            }}
        </div>
    }
}

/// Group key for organizing tasks by due date
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
enum DueDateGroup {
    Today,
    Tomorrow,
    Weekday(u32, String), // (days_until, weekday_name)
    Later(NaiveDate),
    NoSchedule,
}

impl DueDateGroup {
    fn from_date(date: Option<NaiveDate>, today: NaiveDate) -> Self {
        match date {
            None => DueDateGroup::NoSchedule,
            Some(d) => {
                let days_until = (d - today).num_days();
                match days_until {
                    0 => DueDateGroup::Today,
                    1 => DueDateGroup::Tomorrow,
                    2..=6 => {
                        // Store the weekday for later translation
                        let weekday_key = weekday_translation_key(d.weekday()).to_string();
                        DueDateGroup::Weekday(days_until as u32, weekday_key)
                    }
                    _ => DueDateGroup::Later(d),
                }
            }
        }
    }

    fn title(&self, i18n: &I18nContext) -> String {
        match self {
            DueDateGroup::Today => i18n.t("dates.today"),
            DueDateGroup::Tomorrow => i18n.t("dates.tomorrow"),
            DueDateGroup::Weekday(_, key) => i18n.t(key),
            DueDateGroup::Later(date) => date.format("%b %d").to_string(),
            DueDateGroup::NoSchedule => i18n.t("dates.no_schedule"),
        }
    }
}

/// Sub-group TaskWithStatus by category within a date group
fn group_tasks_by_category(tasks: Vec<TaskWithStatus>, other_label: &str) -> Vec<(String, Vec<TaskWithStatus>)> {
    let mut by_category: BTreeMap<String, Vec<TaskWithStatus>> = BTreeMap::new();
    let mut uncategorized: Vec<TaskWithStatus> = Vec::new();

    for task in tasks {
        if let Some(ref cat_name) = task.task.category_name {
            by_category.entry(cat_name.clone()).or_default().push(task);
        } else {
            uncategorized.push(task);
        }
    }

    let mut result: Vec<(String, Vec<TaskWithStatus>)> = by_category.into_iter().collect();
    result.sort_by(|a, b| a.0.cmp(&b.0));
    if !uncategorized.is_empty() {
        result.push((other_label.to_string(), uncategorized));
    }
    result
}

#[component]
pub fn GroupedTaskList(
    tasks: Vec<TaskWithStatus>,
    #[prop(into)] on_complete: Callback<String>,
    #[prop(into)] on_uncomplete: Callback<String>,
    #[prop(default = "UTC".to_string())] timezone: String,
    #[prop(optional)] dashboard_task_ids: Option<HashSet<String>>,
    #[prop(optional, into)] on_toggle_dashboard: Option<Callback<(String, bool)>>,
) -> impl IntoView {
    let i18n = use_i18n();
    let i18n_stored = store_value(i18n);

    let today = today_in_tz(&timezone);
    let other_label = i18n_stored.get_value().t("categories.other");

    // Group tasks by their due date
    let mut grouped: BTreeMap<DueDateGroup, Vec<TaskWithStatus>> = BTreeMap::new();

    for task in tasks {
        let group = DueDateGroup::from_date(task.next_due_date, today);
        grouped.entry(group).or_default().push(task);
    }

    let groups: Vec<(DueDateGroup, Vec<TaskWithStatus>)> = grouped.into_iter().collect();

    view! {
        <div class="card">
            <div class="card-header">
                <h3 class="card-title">{i18n_stored.get_value().t("tasks.title")}</h3>
            </div>
            {if groups.is_empty() {
                view! {
                    <div class="empty-state">
                        <p>{i18n_stored.get_value().t("tasks.no_tasks")}</p>
                    </div>
                }.into_any()
            } else {
                let tz = timezone.clone();
                let dashboard_ids = dashboard_task_ids.clone();
                let other_label_view = other_label.clone();
                view! {
                    <div>
                        {groups.into_iter().map(|(group, group_tasks)| {
                            let title = group.title(&i18n_stored.get_value());
                            let is_today = matches!(group, DueDateGroup::Today);
                            let tz_inner = tz.clone();
                            let dashboard_ids_inner = dashboard_ids.clone();
                            let other_label_inner = other_label_view.clone();
                            // Sub-group by category
                            let category_groups = group_tasks_by_category(group_tasks, &other_label_inner);
                            let has_multiple_categories = category_groups.len() > 1 || (category_groups.len() == 1 && category_groups[0].0 != other_label_inner);
                            view! {
                                <div class="task-group" style=if is_today { "margin-bottom: 1.5rem;" } else { "margin-bottom: 1rem;" }>
                                    <div style=if is_today {
                                        "font-weight: 600; font-size: 1rem; padding: 0.5rem 1rem; background: var(--primary-color); color: white; border-radius: var(--border-radius);"
                                    } else {
                                        "font-weight: 500; font-size: 0.875rem; padding: 0.5rem 1rem; background: var(--bg-muted); color: var(--text-muted); border-radius: var(--border-radius);"
                                    }>
                                        {title}
                                    </div>
                                    <div style="margin-top: 0.5rem;">
                                        {category_groups.into_iter().map(|(cat_name, cat_tasks)| {
                                            let tz_cat = tz_inner.clone();
                                            let dashboard_ids_cat = dashboard_ids_inner.clone();
                                            let show_category_header = has_multiple_categories;
                                            view! {
                                                <div class="category-group" style=if show_category_header {
                                                    "border: 1px solid var(--border-color); border-radius: var(--border-radius); margin-bottom: 0.5rem; overflow: hidden;"
                                                } else {
                                                    ""
                                                }>
                                                    {if show_category_header {
                                                        view! {
                                                            <div style="font-weight: 500; font-size: 0.75rem; padding: 0.5rem 1rem; color: var(--text-muted); background: var(--bg-secondary); border-bottom: 1px solid var(--border-color);">
                                                                {cat_name}
                                                            </div>
                                                        }.into_view()
                                                    } else {
                                                        ().into_view()
                                                    }}
                                                    {cat_tasks.into_iter().map(|task| {
                                                        let tz_task = tz_cat.clone();
                                                        let task_id = task.task.id.to_string();
                                                        // Only pass dashboard props if the feature is enabled
                                                        if let (Some(ids), Some(callback)) = (&dashboard_ids_cat, on_toggle_dashboard) {
                                                            let is_on_dashboard = ids.contains(&task_id);
                                                            view! { <TaskCard task=task on_complete=on_complete on_uncomplete=on_uncomplete timezone=tz_task on_dashboard=is_on_dashboard on_toggle_dashboard=callback /> }.into_view()
                                                        } else {
                                                            view! { <TaskCard task=task on_complete=on_complete on_uncomplete=on_uncomplete timezone=tz_task /> }.into_view()
                                                        }
                                                    }).collect_view()}
                                                </div>
                                            }
                                        }).collect_view()}
                                    </div>
                                </div>
                            }
                        }).collect_view()}
                    </div>
                }.into_any()
            }}
        </div>
    }
}

/// Task with associated household name for dashboard display
#[derive(Clone)]
pub struct TaskWithHousehold {
    pub task: TaskWithStatus,
    pub household_name: String,
    pub household_id: String,
}

/// Sub-group tasks by category within a date group
fn group_by_category(tasks: Vec<TaskWithHousehold>, other_label: &str) -> Vec<(String, Vec<TaskWithHousehold>)> {
    let mut by_category: BTreeMap<String, Vec<TaskWithHousehold>> = BTreeMap::new();
    let mut uncategorized: Vec<TaskWithHousehold> = Vec::new();

    for task in tasks {
        if let Some(ref cat_name) = task.task.task.category_name {
            by_category.entry(cat_name.clone()).or_default().push(task);
        } else {
            uncategorized.push(task);
        }
    }

    let mut result: Vec<(String, Vec<TaskWithHousehold>)> = by_category.into_iter().collect();
    // Sort categories alphabetically
    result.sort_by(|a, b| a.0.cmp(&b.0));
    // Add uncategorized tasks at the end
    if !uncategorized.is_empty() {
        result.push((other_label.to_string(), uncategorized));
    }
    result
}

#[component]
pub fn DashboardGroupedTaskList(
    tasks: Vec<TaskWithHousehold>,
    #[prop(into)] on_complete: Callback<String>,
    #[prop(into)] on_uncomplete: Callback<String>,
    #[prop(default = "UTC".to_string())] timezone: String,
) -> impl IntoView {
    let i18n = use_i18n();
    let i18n_stored = store_value(i18n);

    let today = today_in_tz(&timezone);
    let other_label = i18n_stored.get_value().t("categories.other");

    // Group tasks by their due date
    let mut grouped: BTreeMap<DueDateGroup, Vec<TaskWithHousehold>> = BTreeMap::new();

    for task in tasks {
        let group = DueDateGroup::from_date(task.task.next_due_date, today);
        grouped.entry(group).or_default().push(task);
    }

    let groups: Vec<(DueDateGroup, Vec<TaskWithHousehold>)> = grouped.into_iter().collect();

    view! {
        <div class="card">
            <div class="card-header">
                <h3 class="card-title">{i18n_stored.get_value().t("tasks.title")}</h3>
            </div>
            {if groups.is_empty() {
                view! {
                    <div class="empty-state">
                        <p>{i18n_stored.get_value().t("tasks.no_tasks")}</p>
                    </div>
                }.into_any()
            } else {
                let tz = timezone.clone();
                let other_label_view = other_label.clone();
                view! {
                    <div>
                        {groups.into_iter().map(|(group, group_tasks)| {
                            let title = group.title(&i18n_stored.get_value());
                            let is_today = matches!(group, DueDateGroup::Today);
                            let tz_inner = tz.clone();
                            let other_label_inner = other_label_view.clone();
                            // Sub-group by category
                            let category_groups = group_by_category(group_tasks, &other_label_inner);
                            let has_multiple_categories = category_groups.len() > 1 || (category_groups.len() == 1 && category_groups[0].0 != other_label_inner);
                            view! {
                                <div class="task-group" style=if is_today { "margin-bottom: 1.5rem;" } else { "margin-bottom: 1rem;" }>
                                    <div style=if is_today {
                                        "font-weight: 600; font-size: 1rem; padding: 0.5rem 1rem; background: var(--primary-color); color: white; border-radius: var(--border-radius);"
                                    } else {
                                        "font-weight: 500; font-size: 0.875rem; padding: 0.5rem 1rem; background: var(--bg-muted); color: var(--text-muted); border-radius: var(--border-radius);"
                                    }>
                                        {title}
                                    </div>
                                    <div style="margin-top: 0.5rem;">
                                        {category_groups.into_iter().map(|(cat_name, cat_tasks)| {
                                            let tz_cat = tz_inner.clone();
                                            let show_category_header = has_multiple_categories;
                                            view! {
                                                <div class="category-group" style=if show_category_header {
                                                    "border: 1px solid var(--border-color); border-radius: var(--border-radius); margin-bottom: 0.5rem; overflow: hidden;"
                                                } else {
                                                    ""
                                                }>
                                                    {if show_category_header {
                                                        view! {
                                                            <div style="font-weight: 500; font-size: 0.75rem; padding: 0.5rem 1rem; color: var(--text-muted); background: var(--bg-secondary); border-bottom: 1px solid var(--border-color);">
                                                                {cat_name}
                                                            </div>
                                                        }.into_view()
                                                    } else {
                                                        ().into_view()
                                                    }}
                                                    {cat_tasks.into_iter().map(|twh| {
                                                        let tz_task = tz_cat.clone();
                                                        view! { <TaskCard task=twh.task on_complete=on_complete on_uncomplete=on_uncomplete timezone=tz_task household_name=twh.household_name /> }
                                                    }).collect_view()}
                                                </div>
                                            }
                                        }).collect_view()}
                                    </div>
                                </div>
                            }
                        }).collect_view()}
                    </div>
                }.into_any()
            }}
        </div>
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use shared::{HabitType, RecurrenceType, Task};
    use uuid::Uuid;
    use wasm_bindgen_test::*;

    wasm_bindgen_test_configure!(run_in_browser);

    fn create_test_task(completions: i32, target: i32) -> TaskWithStatus {
        create_test_task_with_exceed(completions, target, true)
    }

    fn create_test_task_with_exceed(completions: i32, target: i32, allow_exceed: bool) -> TaskWithStatus {
        TaskWithStatus {
            task: Task {
                id: Uuid::new_v4(),
                household_id: Uuid::new_v4(),
                title: "Test Task".to_string(),
                description: "Test description".to_string(),
                recurrence_type: RecurrenceType::Daily,
                recurrence_value: None,
                assigned_user_id: None,
                target_count: target,
                time_period: None,
                allow_exceed_target: allow_exceed,
                requires_review: false,
                points_reward: None,
                points_penalty: None,
                due_time: None,
                habit_type: HabitType::Good,
                category_id: None,
                category_name: None,
                created_at: Utc::now(),
                updated_at: Utc::now(),
            },
            completions_today: completions,
            current_streak: 0,
            last_completion: None,
            next_due_date: None,
        }
    }

    #[wasm_bindgen_test]
    fn test_task_with_status_is_target_met_true() {
        let task = create_test_task(3, 3);
        assert!(task.is_target_met());
    }

    #[wasm_bindgen_test]
    fn test_task_with_status_is_target_met_false() {
        let task = create_test_task(2, 3);
        assert!(!task.is_target_met());
    }

    #[wasm_bindgen_test]
    fn test_task_with_status_remaining() {
        let task = create_test_task(1, 3);
        assert_eq!(task.remaining(), 2);
    }

    #[wasm_bindgen_test]
    fn test_task_with_status_remaining_zero_when_complete() {
        let task = create_test_task(3, 3);
        assert_eq!(task.remaining(), 0);
    }

    #[wasm_bindgen_test]
    fn test_task_with_status_remaining_over_target() {
        let task = create_test_task(5, 3);
        assert_eq!(task.remaining(), 0);
    }

    #[wasm_bindgen_test]
    fn test_progress_display_format() {
        let completions = 2;
        let target = 5;
        let progress_display = format!("{}/{}", completions, target);
        assert_eq!(progress_display, "2/5");
    }

    #[wasm_bindgen_test]
    fn test_card_class_completed() {
        let task = create_test_task(3, 3);
        let is_target_met = task.is_target_met();
        let card_class = if is_target_met {
            "task-item task-completed"
        } else {
            "task-item"
        };
        assert_eq!(card_class, "task-item task-completed");
    }

    #[wasm_bindgen_test]
    fn test_card_class_incomplete() {
        let task = create_test_task(1, 3);
        let is_target_met = task.is_target_met();
        let card_class = if is_target_met {
            "task-item task-completed"
        } else {
            "task-item"
        };
        assert_eq!(card_class, "task-item");
    }

    #[wasm_bindgen_test]
    fn test_has_completions_true() {
        let task = create_test_task(1, 3);
        let has_completions = task.completions_today > 0;
        assert!(has_completions);
    }

    #[wasm_bindgen_test]
    fn test_has_completions_false() {
        let task = create_test_task(0, 3);
        let has_completions = task.completions_today > 0;
        assert!(!has_completions);
    }

    #[wasm_bindgen_test]
    fn test_streak_display() {
        let task = TaskWithStatus {
            task: Task {
                id: Uuid::new_v4(),
                household_id: Uuid::new_v4(),
                title: "Test Task".to_string(),
                description: "".to_string(),
                recurrence_type: RecurrenceType::Daily,
                recurrence_value: None,
                assigned_user_id: None,
                target_count: 1,
                time_period: None,
                allow_exceed_target: true,
                requires_review: false,
                points_reward: None,
                points_penalty: None,
                due_time: None,
                habit_type: HabitType::Good,
                category_id: None,
                category_name: None,
                created_at: Utc::now(),
                updated_at: Utc::now(),
            },
            completions_today: 0,
            current_streak: 5,
            last_completion: None,
            next_due_date: None,
        };
        let streak_text = if task.current_streak > 0 {
            format!(" | Streak: {}", task.current_streak)
        } else {
            String::new()
        };
        assert_eq!(streak_text, " | Streak: 5");
    }

    #[wasm_bindgen_test]
    fn test_streak_display_zero() {
        let task = create_test_task(0, 1);
        let streak_text = if task.current_streak > 0 {
            format!(" | Streak: {}", task.current_streak)
        } else {
            String::new()
        };
        assert_eq!(streak_text, "");
    }

    // Tests for can_complete / allow_exceed_target functionality

    #[wasm_bindgen_test]
    fn test_can_complete_target_not_met() {
        // Can always complete if target not yet met
        let task = create_test_task_with_exceed(1, 3, false);
        assert!(task.can_complete());
    }

    #[wasm_bindgen_test]
    fn test_can_complete_target_met_allow_exceed() {
        // Can complete beyond target when allow_exceed_target is true
        let task = create_test_task_with_exceed(3, 3, true);
        assert!(task.can_complete());
    }

    #[wasm_bindgen_test]
    fn test_can_complete_target_met_no_exceed() {
        // Cannot complete beyond target when allow_exceed_target is false
        let task = create_test_task_with_exceed(3, 3, false);
        assert!(!task.can_complete());
    }

    #[wasm_bindgen_test]
    fn test_can_complete_over_target_allow_exceed() {
        // Can continue completing when already over target with allow_exceed_target true
        let task = create_test_task_with_exceed(5, 3, true);
        assert!(task.can_complete());
    }

    #[wasm_bindgen_test]
    fn test_can_complete_over_target_no_exceed() {
        // Cannot complete when already over target with allow_exceed_target false
        let task = create_test_task_with_exceed(5, 3, false);
        assert!(!task.can_complete());
    }

}
