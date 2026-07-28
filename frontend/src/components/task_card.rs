use chrono::{Datelike, NaiveDate, Utc, Weekday};
use leptos::leptos_dom::helpers::TimeoutHandle;
use leptos::*;
use shared::{Archetype, RecurrenceType, TaskWithStatus};
use std::collections::{BTreeMap, HashMap, HashSet};
use std::time::Duration;

use crate::components::context_menu::{ContextMenu, ContextMenuAction};
use crate::components::period_tracker::PeriodTrackerCompact;
use crate::components::task_card_model::{
    accent_class, can_complete_pending, can_undo_pending, card_action, effective_completions,
    type_badge, CardAction, COUNTER_FLUSH_MS,
};
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
    #[prop(into)] on_complete: Callback<(String, i32)>,
    #[prop(into)] on_uncomplete: Callback<(String, i32)>,
    #[prop(default = "UTC".to_string())] timezone: String,
    #[prop(optional)] household_name: Option<String>,
    #[prop(optional)] household_id: Option<String>,
    #[prop(optional)] on_dashboard: Option<bool>,
    #[prop(optional, into)] on_toggle_dashboard: Option<Callback<(String, bool)>>,
    #[prop(optional, into)] on_click_title: Option<Callback<(String, String)>>,
    #[prop(default = Vec::new())] context_actions: Vec<ContextMenuAction>,
) -> impl IntoView {
    let i18n = use_i18n();
    let i18n_stored = store_value(i18n);

    let is_target_met = task.is_target_met();
    // Whether the user may undo a completion: false for the assignee of a task flagged
    // assignee_cannot_uncomplete - somebody else has to clear it
    let can_uncomplete = task.can_uncomplete();
    let is_user_assigned = task.is_user_assigned;
    // Whether to show the +/- controls at all: the assignee, or anyone when the task allows it
    let is_completable_by_user = task.is_completable_by_user();
    let task_id = task.task.id.to_string();
    let task_id_for_dashboard = task_id.clone();
    let task_id_for_title = task_id.clone();
    let household_id_for_title = household_id.clone();
    let completions = task.completions_today;
    let target = task.task.target_count;
    let allow_exceed_target = task.task.allow_exceed_target;

    // Dashboard toggle state (reactive for immediate UI feedback)
    let is_on_dashboard = create_rw_signal(on_dashboard.unwrap_or(false));

    // Taps land here first and are only sent once the tapping stops. The count on screen follows
    // this signal, not the server, so the button reacts instantly - the previous version
    // deferred every single tap by a second and dropped anything tapped in between.
    let pending_delta = create_rw_signal(0i32);
    let flush_timer = store_value(None::<TimeoutHandle>);
    // Stored rather than cloned so `flush` stays `Copy` and both handlers can hold it.
    let task_id_stored = store_value(task_id.clone());

    // Sends what has piled up and clears the slate. One call carries the whole burst, so five
    // taps cost one round trip and one list reload instead of five of each.
    let flush = move || {
        let delta = pending_delta.get_untracked();
        if delta == 0 {
            return;
        }
        pending_delta.set(0);
        let id = task_id_stored.get_value();
        if delta > 0 {
            on_complete.call((id, delta));
        } else {
            on_uncomplete.call((id, -delta));
        }
    };

    // Every tap restarts the clock: the burst is sent as a whole once it ends.
    let restart_timer = move || {
        flush_timer.update_value(|handle| {
            if let Some(handle) = handle.take() {
                handle.clear();
            }
            *handle =
                set_timeout_with_handle(flush, Duration::from_millis(COUNTER_FLUSH_MS)).ok();
        });
    };

    // The count as the user sees it: server state plus whatever is still queued.
    let shown_completions = move || effective_completions(completions, pending_delta.get());
    let plus_enabled = move || {
        can_complete_pending(
            is_completable_by_user,
            allow_exceed_target,
            target,
            shown_completions(),
        )
    };
    let minus_enabled = move || can_undo_pending(can_uncomplete, shown_completions());

    let on_plus = move |_| {
        if plus_enabled() {
            pending_delta.update(|d| *d += 1);
            restart_timer();
        }
    };

    let on_minus = move |_| {
        if minus_enabled() {
            pending_delta.update(|d| *d -= 1);
            restart_timer();
        }
    };

    // Leaving the card with taps still queued must not swallow them - send them on the way out.
    // The callbacks belong to the parent list, which outlives this card.
    on_cleanup(move || {
        flush_timer.update_value(|handle| {
            if let Some(handle) = handle.take() {
                handle.clear();
            }
        });
        flush();
    });

    let archetype = task.task.archetype();
    let card_class = if is_target_met {
        format!("task-item task-completed {}", accent_class(archetype))
    } else {
        format!("task-item {}", accent_class(archetype))
    };

    // A task without a target is a bonus task: it is counted, not measured. Showing "3/0"
    // would invent a goal it does not have, so the count stands on its own. Reads the optimistic
    // count so the number moves with the tap, not with the round trip.
    let is_bonus = archetype == Archetype::Bonus;
    let progress_display = move || {
        if target > 0 {
            format!("{}/{}", shown_completions(), target)
        } else {
            format!("{} ×", shown_completions())
        }
    };

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

    // Bad habit indicator (drives the inverted colours of the period tracker)
    let is_bad_habit = task.task.habit_type.is_inverted();

    // Type badge: icon, translated label and the badge's colour class. OneOff and Routine carry
    // none - they are the default shape of a task, so labelling them would be noise.
    let type_badge_parts = type_badge(archetype).map(|(icon, label_key)| {
        let class = match archetype {
            Archetype::BadHabit | Archetype::Maintenance => "badge badge-sm badge-danger",
            _ => "badge badge-sm badge-assigned",
        };
        (icon, i18n_stored.get_value().t(label_key), class)
    });

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

    // Explain the disabled "-" button when the assignee is not allowed to undo the completion
    let cannot_uncomplete_title = if can_uncomplete {
        String::new()
    } else {
        i18n_stored.get_value().t("task_card.cannot_uncomplete")
    };

    let action = card_action(
        archetype,
        target,
        completions,
        can_uncomplete,
        is_completable_by_user,
    );

    // The mockup's central point: instead of a permanently disabled button whose explanation
    // hides in a `title` attribute, the card states in plain words what was logged and why this
    // user cannot clear it. `last_completion` carries the when; the API does not expose who, so
    // the text names the count and the date.
    let locked_notice = (action == CardAction::Locked).then(|| {
        let date = task
            .last_completion
            .map(|ts| ts.with_timezone(&Utc).date_naive())
            .map(|d| d.format("%d.%m.%Y").to_string());
        let key = if date.is_some() {
            "task_card.locked_with_date"
        } else {
            "task_card.locked"
        };
        let mut text = i18n_stored
            .get_value()
            .t(key)
            .replace("{count}", &completions.to_string());
        if let Some(date) = date {
            text = text.replace("{date}", &date);
        }
        text
    });

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
                        <PeriodTrackerCompact periods=recent_periods.clone() show_in_progress=true is_bad_habit=is_bad_habit is_bonus=is_bonus />
                    }.into_view()
                } else {
                    ().into_view()
                }}
                {if type_badge_parts.is_some() || is_user_assigned {
                    let assigned_label = i18n_stored.get_value().t("tasks.assigned_to_you");
                    view! {
                        <div style="display: flex; flex-wrap: wrap; gap: 0.25rem; margin-top: 0.25rem;">
                            // One badge per archetype instead of the old bad-habit-only one,
                            // which knew exactly one of the six types.
                            {type_badge_parts.map(|(icon, label, class)| view! {
                                <span class=class>{format!("{} {}", icon, label)}</span>
                            })}
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
                {locked_notice.map(|text| view! {
                    <div class="task-card-locked">{text}</div>
                })}
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
                // What the card offers depends on the archetype (wording, colour) and on the
                // target (counter vs. single button) - see task_card_model::card_action.
                {match action {
                    CardAction::Counter => view! {
                        <button
                            class="btn btn-outline"
                            style="padding: 0.25rem 0.75rem; font-size: 1rem; min-width: 32px;"
                            disabled=move || !minus_enabled()
                            title=cannot_uncomplete_title.clone()
                            on:click=on_minus
                        >
                            "-"
                        </button>
                        <span style="font-size: 0.875rem; color: var(--text-muted); min-width: 2rem; text-align: center;">
                            {progress_display}
                        </span>
                        <button
                            class="btn btn-primary"
                            style="padding: 0.25rem 0.75rem; font-size: 1rem; min-width: 32px;"
                            disabled=move || !plus_enabled()
                            on:click=on_plus
                        >
                            "+"
                        </button>
                    }.into_view(),
                    CardAction::Single { label_key, style } => {
                        let action_label = i18n_stored.get_value().t(label_key);
                        let button_class = style.css_class();
                        view! {
                            // The undo only appears once there is something to undo - a button
                            // that can never do anything is exactly what this rework removes.
                            <Show when=move || minus_enabled() fallback=|| ()>
                                <button
                                    class="btn btn-outline"
                                    style="padding: 0.25rem 0.75rem; font-size: 1rem; min-width: 32px;"
                                    on:click=on_minus
                                >
                                    "-"
                                </button>
                            </Show>
                            <Show when=move || { shown_completions() > 0 } fallback=|| ()>
                                <span style="font-size: 0.875rem; color: var(--text-muted); min-width: 2rem; text-align: center;">
                                    {progress_display}
                                </span>
                            </Show>
                            <button
                                class=button_class
                                disabled=move || !plus_enabled()
                                on:click=on_plus
                            >
                                {action_label.clone()}
                            </button>
                        }.into_view()
                    }
                    CardAction::Locked | CardAction::ReadOnly => view! {
                        <span style="font-size: 0.875rem; color: var(--text-muted); min-width: 2rem; text-align: center;">
                            {progress_display}
                        </span>
                    }.into_view(),
                }}
                // Context menu (optional)
                {if !context_actions.is_empty() {
                    view! { <ContextMenu actions=context_actions /> }.into_view()
                } else {
                    ().into_view()
                }}
            </div>
        </div>
    }
}

#[component]
pub fn TaskList(
    tasks: Vec<TaskWithStatus>,
    #[prop(into)] on_complete: Callback<(String, i32)>,
    #[prop(into)] on_uncomplete: Callback<(String, i32)>,
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

    /// Whether this date group can be collapsed.
    /// Today always stays open so the tasks that matter right now are never hidden.
    fn is_collapsible(&self) -> bool {
        !matches!(self, DueDateGroup::Today)
    }

    /// Whether this date group starts expanded.
    /// Only today does, so the list opens on today's tasks.
    fn starts_expanded(&self) -> bool {
        matches!(self, DueDateGroup::Today)
    }

    /// Stable identity of the group, independent of translation and task contents.
    /// The open/closed state is remembered under this key so it survives the re-render
    /// that follows every list refresh.
    fn state_key(&self) -> String {
        match self {
            DueDateGroup::Today => "today".to_string(),
            DueDateGroup::Tomorrow => "tomorrow".to_string(),
            DueDateGroup::Weekday(days_until, _) => format!("weekday-{}", days_until),
            DueDateGroup::Later(date) => format!("later-{}", date),
            DueDateGroup::NoSchedule => "no-schedule".to_string(),
        }
    }
}

/// Remembered open/closed state of the collapsible groups, keyed by [`DueDateGroup::state_key`]
/// and [`category_state_key`]. Lives on the page so it outlives the list's re-renders.
pub type GroupStates = HashMap<String, bool>;

/// Stable identity of a category sub-group within a date group.
fn category_state_key(date_key: &str, category_name: &str) -> String {
    format!("{}/{}", date_key, category_name)
}

/// Open state of a group: what the user last chose, otherwise the group's default.
fn group_open_state(states: &GroupStates, key: &str, default_open: bool) -> bool {
    states.get(key).copied().unwrap_or(default_open)
}

/// Record the group's new open state after the browser toggled its `<details>`.
/// Writing without reading reactively keeps the toggle from re-rendering the list.
fn remember_group_state(states: Option<RwSignal<GroupStates>>, key: &str, ev: &web_sys::Event) {
    let Some(states) = states else { return };
    // `toggle` does not bubble, so the target is always the `<details>` that changed.
    let is_open = event_target::<web_sys::HtmlDetailsElement>(ev).open();
    states.update_untracked(|s| {
        s.insert(key.to_string(), is_open);
    });
}

/// A category sub-group within a date group.
pub struct CategoryGroup {
    pub name: String,
    /// Configured category color as a CSS color string, if the category has one.
    pub color: Option<String>,
    pub tasks: Vec<TaskWithHousehold>,
}

/// Group tasks by category within a date group.
/// Categories come out alphabetically, tasks alphabetically within each category,
/// and the uncategorized tasks last under `other_label`.
fn group_tasks_by_category(tasks: Vec<TaskWithHousehold>, other_label: &str) -> Vec<CategoryGroup> {
    let mut by_category: BTreeMap<String, Vec<TaskWithHousehold>> = BTreeMap::new();
    let mut uncategorized: Vec<TaskWithHousehold> = Vec::new();

    for task in tasks {
        if let Some(cat_name) = task.category_name() {
            by_category.entry(cat_name.clone()).or_default().push(task);
        } else {
            uncategorized.push(task);
        }
    }

    // BTreeMap already yields the categories in alphabetical order.
    let mut result: Vec<CategoryGroup> = by_category
        .into_iter()
        .map(|(name, mut tasks)| {
            tasks.sort_by_key(|a| a.title().to_lowercase());
            // Every task of a category carries the same color; take the first one that has it.
            let color = tasks.iter().find_map(|t| t.category_color().cloned());
            CategoryGroup { name, color, tasks }
        })
        .collect();

    if !uncategorized.is_empty() {
        uncategorized.sort_by_key(|a| a.title().to_lowercase());
        result.push(CategoryGroup {
            name: other_label.to_string(),
            color: None,
            tasks: uncategorized,
        });
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
    #[prop(into)] on_complete: Callback<(String, i32)>,
    #[prop(into)] on_uncomplete: Callback<(String, i32)>,
    #[prop(default = "UTC".to_string())] timezone: String,
    #[prop(optional)] dashboard_task_ids: Option<HashSet<String>>,
    #[prop(optional, into)] on_toggle_dashboard: Option<Callback<(String, bool)>>,
    #[prop(optional, into)] on_click_title: Option<Callback<(String, String)>>,
    #[prop(optional, into)] on_edit: Option<Callback<(String, String)>>,
    #[prop(optional, into)] on_set_date: Option<Callback<(String, String)>>,
    /// Callback for pause/unpause: (task_id, household_id, is_currently_paused)
    #[prop(optional, into)] on_pause: Option<Callback<(String, String, bool)>>,
    /// When true, hides the Edit action (Solo Mode - only Set Date allowed)
    #[prop(default = false)] solo_mode: bool,
    /// Remembered open/closed state of the groups. Every list refresh rebuilds this
    /// component, so without a signal that lives on the page a group the user opened
    /// would snap shut again - and the task they just edited would vanish with it.
    #[prop(optional)] group_states: Option<RwSignal<GroupStates>>,
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
                            let is_collapsible = group.is_collapsible();
                            let date_key = group.state_key();
                            // Read untracked: the open state must not make this view depend on
                            // the signal, or every toggle would rebuild the list underneath it.
                            let group_open = match group_states {
                                Some(states) => states.with_untracked(|s| {
                                    group_open_state(s, &date_key, group.starts_expanded())
                                }),
                                None => group.starts_expanded(),
                            };
                            let date_key_for_toggle = date_key.clone();
                            let tz_inner = tz.clone();
                            let dashboard_ids_inner = dashboard_ids.clone();
                            let other_label_inner = other_label_view.clone();
                            // Sub-group by category
                            let category_groups = group_tasks_by_category(group_tasks, &other_label_inner);
                            let has_multiple_categories = category_groups.len() > 1 || (category_groups.len() == 1 && category_groups[0].name != other_label_inner);
                            let group_task_count = category_groups.iter().map(|g| g.tasks.len()).sum::<usize>();

                            let category_views = category_groups.into_iter().map(|cat_group| {
                                let tz_cat = tz_inner.clone();
                                let dashboard_ids_cat = dashboard_ids_inner.clone();
                                let show_category_header = has_multiple_categories;
                                let CategoryGroup { name: cat_name, color: cat_color, tasks: cat_tasks } = cat_group;
                                let cat_task_count = cat_tasks.len();
                                let cat_key = category_state_key(&date_key, &cat_name);
                                // Category groups default to open; only an explicit collapse by
                                // the user is remembered.
                                let cat_open = match group_states {
                                    Some(states) => states.with_untracked(|s| {
                                        group_open_state(s, &cat_key, true)
                                    }),
                                    None => true,
                                };
                                let cat_key_for_toggle = cat_key.clone();
                                let task_views = cat_tasks.into_iter().map(|twh| {
                                    let tz_task = tz_cat.clone();
                                    let task_id = twh.task_id();
                                    let is_on_dashboard = dashboard_ids_cat.as_ref()
                                        .map(|ids| ids.contains(&task_id))
                                        .unwrap_or(false);
                                    // Extract household info from the task wrapper
                                    let hh_id = twh.household_id.clone();
                                    let hh_name = twh.household_name.clone();

                                    // Build context menu actions
                                    let mut ctx_actions: Vec<ContextMenuAction> = Vec::new();
                                    let is_no_schedule = twh.task.next_due_date.is_none();

                                    // Edit action (hidden in Solo Mode)
                                    if !solo_mode {
                                        if let (Some(edit_cb), Some(ref hid)) = (on_edit, &hh_id) {
                                            let edit_label = i18n_stored.get_value().t("task_card.edit");
                                            let tid = task_id.clone();
                                            let hid_clone = hid.clone();
                                            ctx_actions.push(ContextMenuAction {
                                                label: edit_label,
                                                on_click: Callback::new(move |_| edit_cb.call((tid.clone(), hid_clone.clone()))),
                                                danger: false,
                                            });
                                        }
                                    }

                                    // Set date action (only for tasks without schedule)
                                    if is_no_schedule {
                                        if let (Some(set_date_cb), Some(ref hid)) = (on_set_date, &hh_id) {
                                            let set_date_label = i18n_stored.get_value().t("task_card.set_date");
                                            let tid = task_id.clone();
                                            let hid_clone = hid.clone();
                                            ctx_actions.push(ContextMenuAction {
                                                label: set_date_label,
                                                on_click: Callback::new(move |_| set_date_cb.call((tid.clone(), hid_clone.clone()))),
                                                danger: false,
                                            });
                                        }
                                    }

                                    // Pause/Unpause action (hidden in Solo Mode)
                                    if !solo_mode {
                                        if let (Some(pause_cb), Some(ref hid)) = (on_pause, &hh_id) {
                                            let is_paused = twh.task.task.paused;
                                            let pause_label = if is_paused {
                                                i18n_stored.get_value().t("task_card.unpause")
                                            } else {
                                                i18n_stored.get_value().t("task_card.pause")
                                            };
                                            let tid = task_id.clone();
                                            let hid_clone = hid.clone();
                                            ctx_actions.push(ContextMenuAction {
                                                label: pause_label,
                                                on_click: Callback::new(move |_| pause_cb.call((tid.clone(), hid_clone.clone(), is_paused))),
                                                danger: false,
                                            });
                                        }
                                    }

                                    let context_actions = ctx_actions;

                                    // Render TaskCard with appropriate props based on available data
                                    // Match on household info (both must be Some to display household)
                                    match (on_toggle_dashboard, on_click_title, hh_id, hh_name) {
                                        // With household info
                                        (Some(toggle_cb), Some(title_cb), Some(hid), Some(name)) => {
                                            view! { <TaskCard task=twh.task on_complete=on_complete on_uncomplete=on_uncomplete timezone=tz_task household_name=name household_id=hid on_dashboard=is_on_dashboard on_toggle_dashboard=toggle_cb on_click_title=title_cb context_actions=context_actions /> }.into_view()
                                        }
                                        (Some(toggle_cb), None, Some(hid), Some(name)) => {
                                            view! { <TaskCard task=twh.task on_complete=on_complete on_uncomplete=on_uncomplete timezone=tz_task household_name=name household_id=hid on_dashboard=is_on_dashboard on_toggle_dashboard=toggle_cb context_actions=context_actions /> }.into_view()
                                        }
                                        (None, Some(title_cb), Some(hid), Some(name)) => {
                                            view! { <TaskCard task=twh.task on_complete=on_complete on_uncomplete=on_uncomplete timezone=tz_task household_name=name household_id=hid on_click_title=title_cb context_actions=context_actions /> }.into_view()
                                        }
                                        (None, None, Some(hid), Some(name)) => {
                                            view! { <TaskCard task=twh.task on_complete=on_complete on_uncomplete=on_uncomplete timezone=tz_task household_name=name household_id=hid context_actions=context_actions /> }.into_view()
                                        }
                                        // With household_id only (for title click callback)
                                        (Some(toggle_cb), Some(title_cb), Some(hid), None) => {
                                            view! { <TaskCard task=twh.task on_complete=on_complete on_uncomplete=on_uncomplete timezone=tz_task household_id=hid on_dashboard=is_on_dashboard on_toggle_dashboard=toggle_cb on_click_title=title_cb context_actions=context_actions /> }.into_view()
                                        }
                                        (None, Some(title_cb), Some(hid), None) => {
                                            view! { <TaskCard task=twh.task on_complete=on_complete on_uncomplete=on_uncomplete timezone=tz_task household_id=hid on_click_title=title_cb context_actions=context_actions /> }.into_view()
                                        }
                                        // Without household info
                                        (Some(toggle_cb), Some(title_cb), None, _) => {
                                            view! { <TaskCard task=twh.task on_complete=on_complete on_uncomplete=on_uncomplete timezone=tz_task on_dashboard=is_on_dashboard on_toggle_dashboard=toggle_cb on_click_title=title_cb context_actions=context_actions /> }.into_view()
                                        }
                                        (Some(toggle_cb), None, _, _) => {
                                            view! { <TaskCard task=twh.task on_complete=on_complete on_uncomplete=on_uncomplete timezone=tz_task on_dashboard=is_on_dashboard on_toggle_dashboard=toggle_cb context_actions=context_actions /> }.into_view()
                                        }
                                        (None, Some(title_cb), _, _) => {
                                            view! { <TaskCard task=twh.task on_complete=on_complete on_uncomplete=on_uncomplete timezone=tz_task on_click_title=title_cb context_actions=context_actions /> }.into_view()
                                        }
                                        _ => {
                                            view! { <TaskCard task=twh.task on_complete=on_complete on_uncomplete=on_uncomplete timezone=tz_task context_actions=context_actions /> }.into_view()
                                        }
                                    }
                                }).collect_view();

                                if show_category_header {
                                    // Left bar in the configured category color; categories
                                    // without a color fall back to the neutral border color.
                                    let bar_color = cat_color.unwrap_or_else(|| "var(--border-color)".to_string());
                                    let header_style = format!(
                                        "font-weight: 500; font-size: 0.75rem; padding: 0.5rem 1rem; color: var(--text-muted); background: var(--bg-secondary); border-bottom: 1px solid var(--border-color); border-left: 4px solid {};",
                                        bar_color
                                    );
                                    view! {
                                        <details
                                            class="category-group collapsible-group"
                                            open=cat_open
                                            on:toggle=move |ev| remember_group_state(group_states, &cat_key_for_toggle, &ev)
                                            style="border: 1px solid var(--border-color); border-radius: var(--border-radius); margin-bottom: 0.5rem; overflow: hidden;"
                                        >
                                            <summary style=header_style>
                                                <span class="group-chevron">"\u{25b8}"</span>
                                                <span>{cat_name}</span>
                                                <span class="group-task-count">{cat_task_count}</span>
                                            </summary>
                                            {task_views}
                                        </details>
                                    }.into_view()
                                } else {
                                    view! { <div class="category-group">{task_views}</div> }.into_view()
                                }
                            }).collect_view();

                            let content = view! { <div style="margin-top: 0.5rem;">{category_views}</div> };

                            if is_collapsible {
                                view! {
                                    <details
                                        class="task-group collapsible-group"
                                        open=group_open
                                        on:toggle=move |ev| remember_group_state(group_states, &date_key_for_toggle, &ev)
                                        style="margin-bottom: 1rem;"
                                    >
                                        <summary style="font-weight: 500; font-size: 0.875rem; padding: 0.5rem 1rem; background: rgba(79, 70, 229, 0.15); color: var(--primary-color); border-radius: var(--border-radius);">
                                            <span class="group-chevron">"\u{25b8}"</span>
                                            <span>{title}</span>
                                            <span class="group-task-count">{group_task_count}</span>
                                        </summary>
                                        {content}
                                    </details>
                                }.into_view()
                            } else {
                                view! {
                                    <div class="task-group" style="margin-bottom: 1.5rem;">
                                        <div style="font-weight: 600; font-size: 1rem; padding: 0.5rem 1rem; background: var(--primary-color); color: white; border-radius: var(--border-radius);">
                                            {title}
                                        </div>
                                        {content}
                                    </div>
                                }.into_view()
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
#[derive(Clone, PartialEq)]
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

    /// Get the configured category color from the inner task.
    pub fn category_color(&self) -> Option<&String> {
        self.task.task.category_color.as_ref()
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

    // Tests for anyone_can_complete: the +/- controls are gated on is_completable_by_user()

    fn create_test_task_for_other_user(anyone_can_complete: bool) -> TaskWithStatus {
        let mut task = create_test_task_with_exceed(0, 3, true);
        task.task.assigned_user_id = Some(Uuid::new_v4());
        task.task.anyone_can_complete = anyone_can_complete;
        task.is_user_assigned = false;
        task
    }

    #[wasm_bindgen_test]
    fn test_non_assignee_can_complete_when_anyone_can_complete() {
        let task = create_test_task_for_other_user(true);
        assert!(task.is_completable_by_user());
        assert!(task.can_complete());
    }

    #[wasm_bindgen_test]
    fn test_non_assignee_cannot_complete_by_default() {
        let task = create_test_task_for_other_user(false);
        assert!(!task.is_completable_by_user());
        assert!(!task.can_complete());
    }

    // Tests for assignee_cannot_uncomplete: the "-" button is gated on can_uncomplete()

    fn create_restricted_test_task(is_user_assigned: bool) -> TaskWithStatus {
        let mut task = create_test_task_with_exceed(1, 3, true);
        task.task.assigned_user_id = Some(Uuid::new_v4());
        task.task.assignee_cannot_uncomplete = true;
        task.is_user_assigned = is_user_assigned;
        task
    }

    #[wasm_bindgen_test]
    fn test_assignee_cannot_uncomplete_hides_minus_for_assignee() {
        let task = create_restricted_test_task(true);
        // The assignee still sees the controls and may check the task off ...
        assert!(task.is_completable_by_user());
        assert!(task.can_complete());
        // ... but the "-" button is disabled for them
        assert!(!task.can_uncomplete());
    }

    #[wasm_bindgen_test]
    fn test_assignee_cannot_uncomplete_keeps_minus_for_other_member() {
        let task = create_restricted_test_task(false);
        assert!(task.can_complete());
        assert!(task.can_uncomplete());
    }

    // Tests for category grouping: name, color and ordering

    fn create_task_in_category(title: &str, category: Option<(&str, Option<&str>)>) -> TaskWithHousehold {
        let mut task = create_test_task(0, 1);
        task.task.title = title.to_string();
        if let Some((name, color)) = category {
            task.task.category_id = Some(Uuid::new_v4());
            task.task.category_name = Some(name.to_string());
            task.task.category_color = color.map(|c| c.to_string());
        }
        TaskWithHousehold::from_task(task)
    }

    #[test]
    fn test_group_by_category_uses_configured_color() {
        let tasks = vec![create_task_in_category("Spülen", Some(("Küche", Some("#FF8800"))))];
        let groups = group_tasks_by_category(tasks, "Sonstige");

        assert_eq!(groups.len(), 1);
        assert_eq!(groups[0].name, "Küche");
        assert_eq!(groups[0].color.as_deref(), Some("#FF8800"));
    }

    #[test]
    fn test_group_by_category_without_color_stays_none() {
        let tasks = vec![create_task_in_category("Spülen", Some(("Küche", None)))];
        let groups = group_tasks_by_category(tasks, "Sonstige");

        assert_eq!(groups[0].color, None);
    }

    #[test]
    fn test_group_by_category_picks_color_from_first_task_that_has_one() {
        // Only tasks loaded through the category join carry the color
        let tasks = vec![
            create_task_in_category("A Task", Some(("Küche", None))),
            create_task_in_category("B Task", Some(("Küche", Some("#00AA55")))),
        ];
        let groups = group_tasks_by_category(tasks, "Sonstige");

        assert_eq!(groups.len(), 1);
        assert_eq!(groups[0].tasks.len(), 2);
        assert_eq!(groups[0].color.as_deref(), Some("#00AA55"));
    }

    #[test]
    fn test_group_by_category_uncategorized_goes_last_without_color() {
        let tasks = vec![
            create_task_in_category("Ohne", None),
            create_task_in_category("Mit", Some(("Küche", Some("#FF8800")))),
        ];
        let groups = group_tasks_by_category(tasks, "Sonstige");

        assert_eq!(groups.len(), 2);
        assert_eq!(groups[0].name, "Küche");
        assert_eq!(groups[1].name, "Sonstige");
        assert_eq!(groups[1].color, None);
    }

    #[test]
    fn test_group_by_category_sorts_categories_and_tasks() {
        let tasks = vec![
            create_task_in_category("zebra", Some(("Wohnzimmer", None))),
            create_task_in_category("Apfel", Some(("Wohnzimmer", None))),
            create_task_in_category("Spülen", Some(("Küche", None))),
        ];
        let groups = group_tasks_by_category(tasks, "Sonstige");

        assert_eq!(groups[0].name, "Küche");
        assert_eq!(groups[1].name, "Wohnzimmer");
        assert_eq!(groups[1].tasks[0].title(), "Apfel");
        assert_eq!(groups[1].tasks[1].title(), "zebra");
    }

    // Tests for collapsible date groups

    #[test]
    fn test_today_is_not_collapsible_and_starts_expanded() {
        assert!(!DueDateGroup::Today.is_collapsible());
        assert!(DueDateGroup::Today.starts_expanded());
    }

    #[test]
    fn test_other_date_groups_are_collapsible_and_start_collapsed() {
        let groups = [
            DueDateGroup::Tomorrow,
            DueDateGroup::Weekday(3, "dates.wednesday".to_string()),
            DueDateGroup::Later(NaiveDate::from_ymd_opt(2026, 8, 15).unwrap()),
            DueDateGroup::NoSchedule,
        ];

        for group in groups {
            assert!(group.is_collapsible(), "{:?} should be collapsible", group);
            assert!(!group.starts_expanded(), "{:?} should start collapsed", group);
        }
    }

    // Tests for the remembered open/closed state of the groups

    #[test]
    fn test_state_key_is_distinct_per_group() {
        let keys = [
            DueDateGroup::Today.state_key(),
            DueDateGroup::Tomorrow.state_key(),
            DueDateGroup::Weekday(3, "dates.wednesday".to_string()).state_key(),
            DueDateGroup::Weekday(5, "dates.friday".to_string()).state_key(),
            DueDateGroup::Later(NaiveDate::from_ymd_opt(2026, 8, 15).unwrap()).state_key(),
            DueDateGroup::NoSchedule.state_key(),
        ];

        let unique: HashSet<&String> = keys.iter().collect();
        assert_eq!(unique.len(), keys.len(), "every group needs its own key: {:?}", keys);
    }

    /// The key must not depend on the translated title - otherwise switching the language
    /// would silently forget which groups were open.
    #[test]
    fn test_state_key_ignores_weekday_label() {
        let monday = DueDateGroup::Weekday(3, "dates.monday".to_string());
        let mittwoch = DueDateGroup::Weekday(3, "dates.wednesday".to_string());

        assert_eq!(monday.state_key(), mittwoch.state_key());
    }

    #[test]
    fn test_category_state_key_is_scoped_to_its_date_group() {
        let today = category_state_key(&DueDateGroup::Today.state_key(), "Küche");
        let tomorrow = category_state_key(&DueDateGroup::Tomorrow.state_key(), "Küche");

        assert_ne!(today, tomorrow);
        assert!(today.starts_with("today/"));
    }

    #[test]
    fn test_group_open_state_falls_back_to_default() {
        let states = GroupStates::new();

        assert!(group_open_state(&states, "today", true));
        assert!(!group_open_state(&states, "tomorrow", false));
    }

    #[test]
    fn test_group_open_state_prefers_remembered_choice() {
        let mut states = GroupStates::new();
        states.insert("tomorrow".to_string(), true);
        states.insert("today".to_string(), false);

        // The user opened "tomorrow", which defaults to collapsed ...
        assert!(group_open_state(&states, "tomorrow", false));
        // ... and closed "today", which defaults to expanded.
        assert!(!group_open_state(&states, "today", true));
    }
}
