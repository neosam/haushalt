use leptos::*;
use shared::{HabitType, RecurrenceType, RecurrenceValue, Task, TaskStatistics, TaskWithDetails};

use crate::api::ApiClient;
use crate::components::markdown::MarkdownView;
use crate::components::modal::Modal;
use crate::i18n::use_i18n;

/// Format a recurrence pattern as human-readable text
fn format_recurrence(task: &Task, i18n: &crate::i18n::I18nContext) -> String {
    match task.recurrence_type {
        RecurrenceType::OneTime => i18n.t("tasks.recurrence.onetime"),
        RecurrenceType::Daily => i18n.t("tasks.recurrence.daily"),
        RecurrenceType::Weekly => {
            if let Some(RecurrenceValue::WeekDay(day)) = &task.recurrence_value {
                let day_name = match day {
                    1 => i18n.t("dates.monday"),
                    2 => i18n.t("dates.tuesday"),
                    3 => i18n.t("dates.wednesday"),
                    4 => i18n.t("dates.thursday"),
                    5 => i18n.t("dates.friday"),
                    6 => i18n.t("dates.saturday"),
                    0 | 7 => i18n.t("dates.sunday"),
                    _ => "".to_string(),
                };
                format!("{} {}", i18n.t("tasks.recurrence.every"), day_name)
            } else {
                i18n.t("tasks.recurrence.weekly")
            }
        }
        RecurrenceType::Monthly => {
            if let Some(RecurrenceValue::MonthDay(day)) = &task.recurrence_value {
                format!("{} {}", i18n.t("tasks.recurrence.monthly_on"), day)
            } else {
                i18n.t("tasks.recurrence.monthly")
            }
        }
        RecurrenceType::Weekdays => {
            if let Some(RecurrenceValue::Weekdays(days)) = &task.recurrence_value {
                let day_names: Vec<String> = days
                    .iter()
                    .map(|d| match d {
                        1 => i18n.t("dates.mon"),
                        2 => i18n.t("dates.tue"),
                        3 => i18n.t("dates.wed"),
                        4 => i18n.t("dates.thu"),
                        5 => i18n.t("dates.fri"),
                        6 => i18n.t("dates.sat"),
                        0 | 7 => i18n.t("dates.sun"),
                        _ => "".to_string(),
                    })
                    .collect();
                format!("{} {}", i18n.t("tasks.recurrence.every"), day_names.join(", "))
            } else {
                i18n.t("tasks.recurrence.weekdays")
            }
        }
        RecurrenceType::Custom => i18n.t("tasks.recurrence.custom"),
    }
}

/// Format a completion rate as a percentage string
fn format_rate(rate: Option<f64>, completed: i32, total: i32) -> String {
    if let Some(r) = rate {
        format!("{:.0}% ({}/{})", r, completed, total)
    } else {
        "-".to_string()
    }
}

#[component]
fn TaskDetailContent(
    task: Task,
    stats: TaskStatistics,
    assigned_user: Option<shared::User>,
    linked_rewards: Vec<shared::TaskRewardLink>,
    linked_punishments: Vec<shared::TaskPunishmentLink>,
) -> impl IntoView {
    let i18n = use_i18n();

    let recurrence_text = format_recurrence(&task, &i18n);
    let is_bad_habit = task.habit_type == HabitType::Bad;
    let has_description = !task.description.is_empty();
    let has_due_time = task.due_time.is_some();
    let has_assigned_user = assigned_user.is_some();
    let has_category = task.category_name.is_some();
    let has_last_completed = stats.last_completed.is_some();
    let has_next_due = stats.next_due.is_some();
    let has_linked_rewards = !linked_rewards.is_empty();
    let has_linked_punishments = !linked_punishments.is_empty();

    // Pre-compute all the values we need
    let description = task.description.clone();
    let due_time = task.due_time.clone().unwrap_or_default();
    let target_display = format!(
        "{}{}",
        task.target_count,
        task.time_period.map(|p| format!(" / {:?}", p)).unwrap_or_default()
    );
    let assigned_username = assigned_user.map(|u| u.username).unwrap_or_default();
    let category_name = task.category_name.clone().unwrap_or_default();
    let last_completed_str = stats.last_completed.map(|d| d.format("%Y-%m-%d %H:%M").to_string()).unwrap_or_default();
    let next_due_str = stats.next_due.map(|d| d.to_string()).unwrap_or_default();

    let points_completion = if is_bad_habit {
        task.points_penalty.map(|p| format!("-{}", p)).unwrap_or_else(|| "-".to_string())
    } else {
        task.points_reward.map(|p| format!("+{}", p)).unwrap_or_else(|| "-".to_string())
    };
    let points_miss = if is_bad_habit {
        task.points_reward.map(|p| format!("+{}", p)).unwrap_or_else(|| "-".to_string())
    } else {
        task.points_penalty.map(|p| format!("-{}", p)).unwrap_or_else(|| "-".to_string())
    };

    let points_completion_class = if is_bad_habit { "detail-value points-negative" } else { "detail-value points-positive" };
    let points_miss_class = if is_bad_habit { "detail-value points-positive" } else { "detail-value points-negative" };

    view! {
        // Description section
        {if has_description {
            Some(view! {
                <section class="detail-section">
                    <h4>{i18n.t("tasks.detail.description")}</h4>
                    <MarkdownView content=description.clone() />
                </section>
            })
        } else {
            None
        }}

        // Task info section
        <section class="detail-section">
            <h4>{i18n.t("tasks.detail.info")}</h4>
            <div class="detail-grid">
                <div class="detail-item">
                    <span class="detail-label">{i18n.t("tasks.detail.type")}</span>
                    <span class="detail-value">
                        {if is_bad_habit {
                            view! { <span class="badge badge-warning">{i18n.t("tasks.habit.bad")}</span> }.into_view()
                        } else {
                            view! { <span class="badge badge-success">{i18n.t("tasks.habit.good")}</span> }.into_view()
                        }}
                    </span>
                </div>
                <div class="detail-item">
                    <span class="detail-label">{i18n.t("tasks.detail.recurrence")}</span>
                    <span class="detail-value">{recurrence_text}</span>
                </div>
                {if has_due_time {
                    Some(view! {
                        <div class="detail-item">
                            <span class="detail-label">{i18n.t("tasks.detail.due_time")}</span>
                            <span class="detail-value">{due_time.clone()}</span>
                        </div>
                    })
                } else {
                    None
                }}
                <div class="detail-item">
                    <span class="detail-label">{i18n.t("tasks.detail.target")}</span>
                    <span class="detail-value">{target_display}</span>
                </div>
                {if has_assigned_user {
                    Some(view! {
                        <div class="detail-item">
                            <span class="detail-label">{i18n.t("tasks.detail.assigned_to")}</span>
                            <span class="detail-value">{assigned_username.clone()}</span>
                        </div>
                    })
                } else {
                    None
                }}
                {if has_category {
                    Some(view! {
                        <div class="detail-item">
                            <span class="detail-label">{i18n.t("tasks.detail.category")}</span>
                            <span class="detail-value">{category_name.clone()}</span>
                        </div>
                    })
                } else {
                    None
                }}
            </div>
        </section>

        // Statistics section
        <section class="detail-section">
            <h4>{i18n.t("tasks.detail.statistics")}</h4>
            <div class="stats-cards">
                <div class="stat-card">
                    <div class="stat-label">{i18n.t("tasks.detail.rate_week")}</div>
                    <div class="stat-value">{format_rate(stats.completion_rate_week, stats.periods_completed_week, stats.periods_total_week)}</div>
                </div>
                <div class="stat-card">
                    <div class="stat-label">{i18n.t("tasks.detail.rate_month")}</div>
                    <div class="stat-value">{format_rate(stats.completion_rate_month, stats.periods_completed_month, stats.periods_total_month)}</div>
                </div>
                <div class="stat-card">
                    <div class="stat-label">{i18n.t("tasks.detail.rate_all_time")}</div>
                    <div class="stat-value">{format_rate(stats.completion_rate_all_time, stats.periods_completed_all_time, stats.periods_total_all_time)}</div>
                </div>
            </div>
            <div class="detail-grid stats-grid">
                <div class="detail-item">
                    <span class="detail-label">{i18n.t("tasks.detail.current_streak")}</span>
                    <span class="detail-value streak-value">{stats.current_streak}</span>
                </div>
                <div class="detail-item">
                    <span class="detail-label">{i18n.t("tasks.detail.best_streak")}</span>
                    <span class="detail-value">{stats.best_streak}</span>
                </div>
                <div class="detail-item">
                    <span class="detail-label">{i18n.t("tasks.detail.total_completions")}</span>
                    <span class="detail-value">{stats.total_completions}</span>
                </div>
                {if has_last_completed {
                    Some(view! {
                        <div class="detail-item">
                            <span class="detail-label">{i18n.t("tasks.detail.last_completed")}</span>
                            <span class="detail-value">{last_completed_str.clone()}</span>
                        </div>
                    })
                } else {
                    None
                }}
                {if has_next_due {
                    Some(view! {
                        <div class="detail-item">
                            <span class="detail-label">{i18n.t("tasks.detail.next_due")}</span>
                            <span class="detail-value">{next_due_str.clone()}</span>
                        </div>
                    })
                } else {
                    None
                }}
            </div>
        </section>

        // Points section
        <section class="detail-section">
            <h4>{i18n.t("tasks.detail.points")}</h4>
            <div class="points-display">
                <div class="point-item">
                    <span class="detail-label">{i18n.t("tasks.detail.points_on_completion")}</span>
                    <span class=points_completion_class>{points_completion}</span>
                </div>
                <div class="point-item">
                    <span class="detail-label">{i18n.t("tasks.detail.points_on_miss")}</span>
                    <span class=points_miss_class>{points_miss}</span>
                </div>
            </div>
        </section>

        // Linked rewards section
        {if has_linked_rewards {
            Some(view! {
                <section class="detail-section">
                    <h4>{i18n.t("tasks.detail.linked_rewards")}</h4>
                    <ul class="linked-items">
                        {linked_rewards.into_iter().map(|link| {
                            let amount_display = if link.amount > 1 {
                                Some(format!(" x{}", link.amount))
                            } else {
                                None
                            };
                            view! {
                                <li class="linked-item reward-item">
                                    <span class="item-name">{link.reward.name}</span>
                                    {amount_display}
                                </li>
                            }
                        }).collect::<Vec<_>>()}
                    </ul>
                </section>
            })
        } else {
            None
        }}

        // Linked punishments section
        {if has_linked_punishments {
            Some(view! {
                <section class="detail-section">
                    <h4>{i18n.t("tasks.detail.linked_punishments")}</h4>
                    <ul class="linked-items">
                        {linked_punishments.into_iter().map(|link| {
                            let amount_display = if link.amount > 1 {
                                Some(format!(" x{}", link.amount))
                            } else {
                                None
                            };
                            view! {
                                <li class="linked-item punishment-item">
                                    <span class="item-name">{link.punishment.name}</span>
                                    {amount_display}
                                </li>
                            }
                        }).collect::<Vec<_>>()}
                    </ul>
                </section>
            })
        } else {
            None
        }}
    }
}

#[component]
pub fn TaskDetailModal(
    task_id: String,
    household_id: String,
    #[prop(into)] on_close: Callback<()>,
    #[prop(into)] on_edit: Callback<Task>,
) -> impl IntoView {
    let i18n = use_i18n();
    let i18n_stored = store_value(i18n.clone());
    let loading = create_rw_signal(true);
    let error = create_rw_signal(Option::<String>::None);
    let details = create_rw_signal(Option::<TaskWithDetails>::None);

    // Load task details on mount
    {
        let task_id = task_id.clone();
        let household_id = household_id.clone();
        create_effect(move |_| {
            let task_id = task_id.clone();
            let household_id = household_id.clone();
            wasm_bindgen_futures::spawn_local(async move {
                match ApiClient::get_task_details(&household_id, &task_id).await {
                    Ok(d) => {
                        details.set(Some(d));
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

    let handle_edit = move |_| {
        if let Some(d) = details.get() {
            on_edit.call(d.task);
        }
    };

    let title = i18n.t("tasks.detail.title");
    let close_text = i18n.t("common.close");
    let edit_text = i18n.t("common.edit");

    view! {
        <Modal title=title on_close=on_close>
            <div class="modal-body task-detail-modal">
                {move || {
                    let i18n = i18n_stored.get_value();
                    if loading.get() {
                        view! { <div class="loading">{i18n.t("common.loading")}</div> }.into_view()
                    } else if let Some(err) = error.get() {
                        view! { <div class="error-message">{err}</div> }.into_view()
                    } else if let Some(d) = details.get() {
                        view! {
                            <TaskDetailContent
                                task=d.task
                                stats=d.statistics
                                assigned_user=d.assigned_user
                                linked_rewards=d.linked_rewards
                                linked_punishments=d.linked_punishments
                            />
                        }.into_view()
                    } else {
                        view! { <div class="error-message">"No data"</div> }.into_view()
                    }
                }}
            </div>
            <div class="modal-footer">
                <button class="btn btn-secondary" on:click=move |_| on_close.call(())>
                    {close_text.clone()}
                </button>
                <button class="btn btn-primary" on:click=handle_edit>
                    {edit_text.clone()}
                </button>
            </div>
        </Modal>
    }
}
