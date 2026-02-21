use chrono::{Datelike, NaiveDate, Weekday};
use leptos::*;
use shared::{RecurrenceType, TaskWithStatus};
use std::collections::{BTreeMap, HashSet};
use std::time::Duration;

use crate::components::period_tracker::PeriodTrackerCompact;
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
    #[prop(optional)] household_id: Option<String>,
    #[prop(optional)] on_dashboard: Option<bool>,
    #[prop(optional, into)] on_toggle_dashboard: Option<Callback<(String, bool)>>,
    #[prop(optional, into)] on_click_title: Option<Callback<(String, String)>>,
) -> impl IntoView {
    let i18n = use_i18n();
    let i18n_stored = store_value(i18n);

    let is_target_met = task.is_target_met();
    let can_complete = task.can_complete();
    let is_user_assigned = task.is_user_assigned;
    let task_id = task.task.id.to_string();
    let task_id_for_minus = task_id.clone();
    let task_id_for_dashboard = task_id.clone();
    let task_id_for_title = task_id.clone();
    let household_id_for_title = household_id.clone();
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

    // Recent periods for habit tracker display
    let recent_periods = task.recent_periods.clone();
    let has_recent_periods = !recent_periods.is_empty();

    // Household name and link for meta line
    let household_name_display = household_name.clone();
    let household_id_for_link = household_id.clone();

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

    // Title click handler
    let title_clickable = on_click_title.is_some() && household_id_for_title.is_some();
    let on_title_click = move |_| {
        if let (Some(callback), Some(ref hid)) = (on_click_title, &household_id_for_title) {
            callback.call((task_id_for_title.clone(), hid.clone()));
        }
    };

    let task_title = task.task.title.clone();

    view! {
        <div class=card_class>
            <div class="task-content" style="flex: 1;">
                <div class="task-title">
                    {if title_clickable {
                        view! {
                            <span class="task-title-clickable" on:pointerup=on_title_click.clone()>
                                {task_title.clone()}
                            </span>
                        }.into_view()
                    } else {
                        view! { <span>{task_title.clone()}</span> }.into_view()
                    }}
                </div>
                <div class="task-meta">
                    {if let (Some(name), Some(hid)) = (household_name_display.clone(), household_id_for_link.clone()) {
                        let navigate = leptos_router::use_navigate();
                        let hid_clone = hid.clone();
                        view! {
                            <span
                                class="household-link"
                                on:pointerup=move |_| {
                                    navigate(&format!("/households/{}", hid_clone), Default::default());
                                }
                            >{name}</span>
                            " | "
                        }.into_view()
                    } else {
                        ().into_view()
                    }}
                    {recurrence_display}
                    {due_display}
                    {streak_display}
                </div>
                {if has_recent_periods {
                    view! {
                        <PeriodTrackerCompact periods=recent_periods.clone() show_in_progress=true is_bad_habit=is_bad_habit />
                    }.into_view()
                } else {
                    ().into_view()
                }}
                {if is_bad_habit || is_user_assigned {
                    let assigned_label = i18n_stored.get_value().t("tasks.assigned_to_you");
                    view! {
                        <div style="display: flex; flex-wrap: wrap; gap: 0.25rem; margin-top: 0.25rem;">
                            {if is_bad_habit {
                                view! {
                                    <span class="badge badge-sm badge-danger">{bad_habit_label}</span>
                                }.into_view()
                            } else {
                                ().into_view()
                            }}
                            {if is_user_assigned {
                                view! {
                                    <span class="badge badge-sm badge-assigned">{assigned_label}</span>
                                }.into_view()
                            } else {
                                ().into_view()
                            }}
                        </div>
                    }.into_view()
                } else {
                    ().into_view()
                }}
            </div>
            <div class="task-actions">
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
                // Only show +/- buttons if user is assigned to the task
                {if is_user_assigned {
                    view! {
                        <button
                            class="btn btn-outline"
                            style="padding: 0.25rem 0.75rem; font-size: 1rem; min-width: 32px;"
                            disabled=!has_completions
                            on:click=on_minus
                        >
                            "-"
                        </button>
                        <span style="font-size: 0.875rem; color: var(--text-muted); min-width: 2rem; text-align: center;">
                            {progress_display.clone()}
                        </span>
                        <button
                            class=move || if is_debouncing.get() { "btn btn-primary btn-debouncing" } else { "btn btn-primary" }
                            style="padding: 0.25rem 0.75rem; font-size: 1rem; min-width: 32px;"
                            disabled=move || !can_complete || is_debouncing.get()
                            on:click=on_plus
                        >
                            {move || if is_debouncing.get() { "..." } else { "+" }}
                        </button>
                    }.into_view()
                } else {
                    // Just show progress without buttons
                    view! {
                        <span style="font-size: 0.875rem; color: var(--text-muted); min-width: 2rem; text-align: center;">
                            {progress_display.clone()}
                        </span>
                    }.into_view()
                }}
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

/// Group tasks by category within a date group.
/// Returns a sorted list of (category_name, tasks) tuples.
fn group_tasks_by_category(tasks: Vec<TaskWithHousehold>, other_label: &str) -> Vec<(String, Vec<TaskWithHousehold>)> {
    let mut by_category: BTreeMap<String, Vec<TaskWithHousehold>> = BTreeMap::new();
    let mut uncategorized: Vec<TaskWithHousehold> = Vec::new();

    for task in tasks {
        if let Some(cat_name) = task.category_name() {
            by_category.entry(cat_name.clone()).or_default().push(task);
        } else {
            uncategorized.push(task);
        }
    }

    let mut result: Vec<(String, Vec<TaskWithHousehold>)> = by_category.into_iter().collect();
    // Sort categories alphabetically
    result.sort_by(|a, b| a.0.cmp(&b.0));
    // Sort tasks alphabetically within each category
    for (_, category_tasks) in &mut result {
        category_tasks.sort_by_key(|a| a.title().to_lowercase());
    }
    if !uncategorized.is_empty() {
        // Sort uncategorized tasks alphabetically
        uncategorized.sort_by_key(|a| a.title().to_lowercase());
        result.push((other_label.to_string(), uncategorized));
    }
    result
}

/// Unified grouped task list component.
/// Displays tasks grouped by due date (Today, Tomorrow, Weekday, Later, No Schedule)
/// and sub-grouped by category within each date group.
///
/// This component handles both single-household context (household page) and
/// multi-household context (dashboard) based on whether tasks have household info.
#[component]
pub fn GroupedTaskList(
    tasks: Vec<TaskWithHousehold>,
    #[prop(into)] on_complete: Callback<String>,
    #[prop(into)] on_uncomplete: Callback<String>,
    #[prop(default = "UTC".to_string())] timezone: String,
    #[prop(optional)] dashboard_task_ids: Option<HashSet<String>>,
    #[prop(optional, into)] on_toggle_dashboard: Option<Callback<(String, bool)>>,
    #[prop(optional, into)] on_click_title: Option<Callback<(String, String)>>,
) -> impl IntoView {
    let i18n = use_i18n();
    let i18n_stored = store_value(i18n);

    let today = today_in_tz(&timezone);
    let other_label = i18n_stored.get_value().t("categories.other");

    // Group tasks by their due date
    let mut grouped: BTreeMap<DueDateGroup, Vec<TaskWithHousehold>> = BTreeMap::new();

    for task in tasks {
        let group = DueDateGroup::from_date(task.next_due_date(), today);
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
                                        "font-weight: 500; font-size: 0.875rem; padding: 0.5rem 1rem; background: rgba(79, 70, 229, 0.15); color: var(--primary-color); border-radius: var(--border-radius);"
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
                                                    {cat_tasks.into_iter().map(|twh| {
                                                        let tz_task = tz_cat.clone();
                                                        let task_id = twh.task_id();
                                                        let is_on_dashboard = dashboard_ids_cat.as_ref()
                                                            .map(|ids| ids.contains(&task_id))
                                                            .unwrap_or(false);
                                                        // Extract household info from the task wrapper
                                                        let hh_id = twh.household_id.clone();
                                                        let hh_name = twh.household_name.clone();
                                                        // Render TaskCard with appropriate props based on available data
                                                        // Match on household info (both must be Some to display household)
                                                        match (on_toggle_dashboard, on_click_title, hh_id, hh_name) {
                                                            // With household info
                                                            (Some(toggle_cb), Some(title_cb), Some(hid), Some(name)) => {
                                                                view! { <TaskCard task=twh.task on_complete=on_complete on_uncomplete=on_uncomplete timezone=tz_task household_name=name household_id=hid on_dashboard=is_on_dashboard on_toggle_dashboard=toggle_cb on_click_title=title_cb /> }.into_view()
                                                            }
                                                            (Some(toggle_cb), None, Some(hid), Some(name)) => {
                                                                view! { <TaskCard task=twh.task on_complete=on_complete on_uncomplete=on_uncomplete timezone=tz_task household_name=name household_id=hid on_dashboard=is_on_dashboard on_toggle_dashboard=toggle_cb /> }.into_view()
                                                            }
                                                            (None, Some(title_cb), Some(hid), Some(name)) => {
                                                                view! { <TaskCard task=twh.task on_complete=on_complete on_uncomplete=on_uncomplete timezone=tz_task household_name=name household_id=hid on_click_title=title_cb /> }.into_view()
                                                            }
                                                            (None, None, Some(hid), Some(name)) => {
                                                                view! { <TaskCard task=twh.task on_complete=on_complete on_uncomplete=on_uncomplete timezone=tz_task household_name=name household_id=hid /> }.into_view()
                                                            }
                                                            // With household_id only (for title click callback)
                                                            (Some(toggle_cb), Some(title_cb), Some(hid), None) => {
                                                                view! { <TaskCard task=twh.task on_complete=on_complete on_uncomplete=on_uncomplete timezone=tz_task household_id=hid on_dashboard=is_on_dashboard on_toggle_dashboard=toggle_cb on_click_title=title_cb /> }.into_view()
                                                            }
                                                            (None, Some(title_cb), Some(hid), None) => {
                                                                view! { <TaskCard task=twh.task on_complete=on_complete on_uncomplete=on_uncomplete timezone=tz_task household_id=hid on_click_title=title_cb /> }.into_view()
                                                            }
                                                            // Without household info
                                                            (Some(toggle_cb), Some(title_cb), None, _) => {
                                                                view! { <TaskCard task=twh.task on_complete=on_complete on_uncomplete=on_uncomplete timezone=tz_task on_dashboard=is_on_dashboard on_toggle_dashboard=toggle_cb on_click_title=title_cb /> }.into_view()
                                                            }
                                                            (Some(toggle_cb), None, _, _) => {
                                                                view! { <TaskCard task=twh.task on_complete=on_complete on_uncomplete=on_uncomplete timezone=tz_task on_dashboard=is_on_dashboard on_toggle_dashboard=toggle_cb /> }.into_view()
                                                            }
                                                            (None, Some(title_cb), _, _) => {
                                                                view! { <TaskCard task=twh.task on_complete=on_complete on_uncomplete=on_uncomplete timezone=tz_task on_click_title=title_cb /> }.into_view()
                                                            }
                                                            _ => {
                                                                view! { <TaskCard task=twh.task on_complete=on_complete on_uncomplete=on_uncomplete timezone=tz_task /> }.into_view()
                                                            }
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

/// Task with associated household information for display.
/// This is the unified type used by GroupedTaskList.
#[derive(Clone)]
pub struct TaskWithHousehold {
    pub task: TaskWithStatus,
    pub household_name: Option<String>,
    pub household_id: Option<String>,
}

impl TaskWithHousehold {
    /// Create a TaskWithHousehold from a TaskWithStatus with household info.
    pub fn new(task: TaskWithStatus, household_id: Option<String>, household_name: Option<String>) -> Self {
        Self {
            task,
            household_id,
            household_name,
        }
    }

    /// Create a TaskWithHousehold from a TaskWithStatus without household info.
    /// Used when displaying tasks within a single household context.
    pub fn from_task(task: TaskWithStatus) -> Self {
        Self {
            task,
            household_id: None,
            household_name: None,
        }
    }

    /// Create a TaskWithHousehold with required household info (for dashboard).
    pub fn with_household(task: TaskWithStatus, household_id: String, household_name: String) -> Self {
        Self {
            task,
            household_id: Some(household_id),
            household_name: Some(household_name),
        }
    }

    /// Get the next due date from the inner task.
    pub fn next_due_date(&self) -> Option<NaiveDate> {
        self.task.next_due_date
    }

    /// Get the category name from the inner task.
    pub fn category_name(&self) -> Option<&String> {
        self.task.task.category_name.as_ref()
    }

    /// Get the task title from the inner task.
    pub fn title(&self) -> &str {
        &self.task.task.title
    }

    /// Get the task ID from the inner task.
    pub fn task_id(&self) -> String {
        self.task.task.id.to_string()
    }
}

/// Deprecated: Use GroupedTaskList with TaskWithHousehold instead.
/// This type alias is kept for backwards compatibility but will be removed.
#[deprecated(note = "Use GroupedTaskList with TaskWithHousehold::with_household instead")]
pub type DashboardGroupedTaskList = ();

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
                archived: false,
                paused: false,
                created_at: Utc::now(),
                updated_at: Utc::now(),
                suggestion: None,
                suggested_by: None,
            },
            completions_today: completions,
            current_streak: 0,
            last_completion: None,
            next_due_date: None,
            is_user_assigned: true,
            recent_periods: Vec::new(),
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
                archived: false,
                paused: false,
                created_at: Utc::now(),
                updated_at: Utc::now(),
                suggestion: None,
                suggested_by: None,
            },
            completions_today: 0,
            current_streak: 5,
            last_completion: None,
            next_due_date: None,
            is_user_assigned: true,
            recent_periods: Vec::new(),
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
