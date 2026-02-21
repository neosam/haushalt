use chrono::NaiveDate;
use leptos::*;
use leptos_router::*;
use shared::{HouseholdSettings, MemberStatistic, MonthlyStatisticsResponse, WeeklyStatisticsResponse};

use crate::api::ApiClient;
use crate::components::loading::Loading;
use crate::components::{
    Accordion, Alert, AlertVariant, Button, ButtonVariant, Card, ProgressBar,
};
use crate::i18n::use_i18n;

#[derive(Clone, Copy, PartialEq)]
enum StatisticsView {
    Weekly,
    Monthly,
}

#[component]
pub fn StatisticsPage() -> impl IntoView {
    let i18n = use_i18n();
    let i18n_stored = store_value(i18n);

    let params = use_params_map();
    let household_id = move || params.with(|p| p.get("id").cloned().unwrap_or_default());

    let settings = create_rw_signal(Option::<HouseholdSettings>::None);
    let loading = create_rw_signal(true);
    let calculating = create_rw_signal(false);
    let error = create_rw_signal(Option::<String>::None);

    // Current view (weekly or monthly)
    let current_view = create_rw_signal(StatisticsView::Weekly);

    // Weekly state
    let weekly_stats = create_rw_signal(Option::<WeeklyStatisticsResponse>::None);
    let available_weeks = create_rw_signal(Vec::<NaiveDate>::new());
    let selected_week = create_rw_signal(Option::<NaiveDate>::None);

    // Monthly state
    let monthly_stats = create_rw_signal(Option::<MonthlyStatisticsResponse>::None);
    let available_months = create_rw_signal(Vec::<NaiveDate>::new());
    let selected_month = create_rw_signal(Option::<NaiveDate>::None);

    // Load settings
    create_effect(move |_| {
        let id = household_id();
        if id.is_empty() {
            return;
        }

        let id_clone = id.clone();
        wasm_bindgen_futures::spawn_local(async move {
            if let Ok(s) = ApiClient::get_household_settings(&id_clone).await {
                apply_dark_mode(s.dark_mode);
                settings.set(Some(s));
            }
        });

        // Load available weeks and months
        let id_for_weeks = id.clone();
        let id_for_months = id.clone();

        wasm_bindgen_futures::spawn_local(async move {
            if let Ok(weeks) = ApiClient::list_available_weeks(&id_for_weeks).await {
                available_weeks.set(weeks.clone());
                if let Some(first) = weeks.first() {
                    selected_week.set(Some(*first));
                }
            }
            loading.set(false);
        });

        wasm_bindgen_futures::spawn_local(async move {
            if let Ok(months) = ApiClient::list_available_months(&id_for_months).await {
                available_months.set(months.clone());
                if let Some(first) = months.first() {
                    selected_month.set(Some(*first));
                }
            }
        });
    });

    // Load statistics when selection changes
    create_effect(move |_| {
        let id = household_id();
        if id.is_empty() {
            return;
        }

        if let Some(week) = selected_week.get() {
            let id_clone = id.clone();
            let week_str = week.format("%Y-%m-%d").to_string();
            wasm_bindgen_futures::spawn_local(async move {
                match ApiClient::get_weekly_statistics(&id_clone, Some(&week_str)).await {
                    Ok(stats) => weekly_stats.set(Some(stats)),
                    Err(e) => error.set(Some(e)),
                }
            });
        }
    });

    create_effect(move |_| {
        let id = household_id();
        if id.is_empty() {
            return;
        }

        if let Some(month) = selected_month.get() {
            let id_clone = id.clone();
            let month_str = month.format("%Y-%m-%d").to_string();
            wasm_bindgen_futures::spawn_local(async move {
                match ApiClient::get_monthly_statistics(&id_clone, Some(&month_str)).await {
                    Ok(stats) => monthly_stats.set(Some(stats)),
                    Err(e) => error.set(Some(e)),
                }
            });
        }
    });

    // Calculate statistics action
    let on_calculate = move |_| {
        let id = household_id();
        calculating.set(true);
        error.set(None);

        let view = current_view.get();
        match view {
            StatisticsView::Weekly => {
                let week = selected_week.get().map(|w| w.format("%Y-%m-%d").to_string());
                wasm_bindgen_futures::spawn_local(async move {
                    match ApiClient::calculate_weekly_statistics(&id, week.as_deref()).await {
                        Ok(stats) => {
                            weekly_stats.set(Some(stats));
                            // Refresh available weeks
                            if let Ok(weeks) = ApiClient::list_available_weeks(&id).await {
                                available_weeks.set(weeks);
                            }
                        }
                        Err(e) => error.set(Some(e)),
                    }
                    calculating.set(false);
                });
            }
            StatisticsView::Monthly => {
                let month = selected_month.get().map(|m| m.format("%Y-%m-%d").to_string());
                wasm_bindgen_futures::spawn_local(async move {
                    match ApiClient::calculate_monthly_statistics(&id, month.as_deref()).await {
                        Ok(stats) => {
                            monthly_stats.set(Some(stats));
                            // Refresh available months
                            if let Ok(months) = ApiClient::list_available_months(&id).await {
                                available_months.set(months);
                            }
                        }
                        Err(e) => error.set(Some(e)),
                    }
                    calculating.set(false);
                });
            }
        }
    };

    view! {
        <div class="dashboard-header">
            <h1 class="dashboard-title">{i18n_stored.get_value().t("statistics.title")}</h1>
        </div>

        {move || error.get().map(|e| view! {
            <Alert variant=AlertVariant::Error>{e}</Alert>
        })}

        <Show when=move || loading.get() fallback=|| ()>
            <Loading />
        </Show>

        <Show when=move || !loading.get() fallback=|| ()>
            // View switcher
            <Card style="margin-bottom: 1rem;">
                <div style="display: flex; gap: 1rem; align-items: center; flex-wrap: wrap;">
                    <div style="display: flex; gap: 0.5rem;">
                        <Button
                            variant=MaybeSignal::derive(move || if current_view.get() == StatisticsView::Weekly { ButtonVariant::Primary } else { ButtonVariant::Secondary })
                            on_click=Callback::new(move |_| current_view.set(StatisticsView::Weekly))
                        >
                            {i18n_stored.get_value().t("statistics.weekly")}
                        </Button>
                        <Button
                            variant=MaybeSignal::derive(move || if current_view.get() == StatisticsView::Monthly { ButtonVariant::Primary } else { ButtonVariant::Secondary })
                            on_click=Callback::new(move |_| current_view.set(StatisticsView::Monthly))
                        >
                            {i18n_stored.get_value().t("statistics.monthly")}
                        </Button>
                    </div>

                    // Period selector
                    {move || {
                        if current_view.get() == StatisticsView::Weekly {
                            let weeks = available_weeks.get();
                            view! {
                                <select
                                    class="form-select"
                                    style="width: auto; min-width: 200px;"
                                    on:change=move |ev| {
                                        if let Ok(date) = NaiveDate::parse_from_str(&event_target_value(&ev), "%Y-%m-%d") {
                                            selected_week.set(Some(date));
                                        }
                                    }
                                >
                                    {weeks.into_iter().map(|week| {
                                        let week_str = week.format("%Y-%m-%d").to_string();
                                        let display = format_week_display(&week);
                                        view! {
                                            <option
                                                value=week_str.clone()
                                                selected=move || selected_week.get() == Some(week)
                                            >
                                                {display}
                                            </option>
                                        }
                                    }).collect_view()}
                                </select>
                            }.into_view()
                        } else {
                            let months = available_months.get();
                            view! {
                                <select
                                    class="form-select"
                                    style="width: auto; min-width: 200px;"
                                    on:change=move |ev| {
                                        if let Ok(date) = NaiveDate::parse_from_str(&event_target_value(&ev), "%Y-%m-%d") {
                                            selected_month.set(Some(date));
                                        }
                                    }
                                >
                                    {months.into_iter().map(|month| {
                                        let month_str = month.format("%Y-%m-%d").to_string();
                                        let display = format_month_display(&month);
                                        view! {
                                            <option
                                                value=month_str.clone()
                                                selected=move || selected_month.get() == Some(month)
                                            >
                                                {display}
                                            </option>
                                        }
                                    }).collect_view()}
                                </select>
                            }.into_view()
                        }
                    }}

                    <Button
                        disabled=MaybeSignal::derive(move || calculating.get())
                        on_click=Callback::new(on_calculate)
                    >
                        {move || if calculating.get() {
                            i18n_stored.get_value().t("statistics.calculating")
                        } else {
                            i18n_stored.get_value().t("statistics.calculate")
                        }}
                    </Button>
                </div>
            </Card>

            // Statistics display
            {move || {
                if current_view.get() == StatisticsView::Weekly {
                    if let Some(stats) = weekly_stats.get() {
                        view! { <WeeklyStatsView stats=stats i18n=i18n_stored /> }.into_view()
                    } else {
                        view! {
                            <Card class="empty-state">
                                <p>{i18n_stored.get_value().t("statistics.no_weekly_data")}</p>
                                <p>{i18n_stored.get_value().t("statistics.click_calculate")}</p>
                            </Card>
                        }.into_view()
                    }
                } else if let Some(stats) = monthly_stats.get() {
                    view! { <MonthlyStatsView stats=stats i18n=i18n_stored /> }.into_view()
                } else {
                    view! {
                        <Card class="empty-state">
                            <p>{i18n_stored.get_value().t("statistics.no_monthly_data")}</p>
                            <p>{i18n_stored.get_value().t("statistics.click_calculate")}</p>
                        </Card>
                    }.into_view()
                }
            }}
        </Show>
    }
}

#[component]
fn WeeklyStatsView(
    stats: WeeklyStatisticsResponse,
    i18n: StoredValue<crate::i18n::I18nContext>,
) -> impl IntoView {
    let week_range = format!(
        "{} - {}",
        stats.week_start.format("%d.%m.%Y"),
        stats.week_end.format("%d.%m.%Y")
    );

    let title = format!("{} {}", i18n.get_value().t("statistics.week_of"), week_range);

    view! {
        <Card title=title>
            {if stats.members.is_empty() {
                view! {
                    <p>{i18n.get_value().t("statistics.no_member_data")}</p>
                }.into_view()
            } else {
                view! {
                    {stats.members.into_iter().map(|member| {
                        view! { <MemberStatsCard member=member i18n=i18n /> }
                    }).collect_view()}
                }.into_view()
            }}
        </Card>
    }
}

#[component]
fn MonthlyStatsView(
    stats: MonthlyStatisticsResponse,
    i18n: StoredValue<crate::i18n::I18nContext>,
) -> impl IntoView {
    let title = stats.month.format("%B %Y").to_string();

    view! {
        <Card title=title>
            {if stats.members.is_empty() {
                view! {
                    <p>{i18n.get_value().t("statistics.no_member_data")}</p>
                }.into_view()
            } else {
                view! {
                    {stats.members.into_iter().map(|member| {
                        view! { <MemberStatsCard member=member i18n=i18n /> }
                    }).collect_view()}
                }.into_view()
            }}
        </Card>
    }
}

#[component]
fn MemberStatsCard(
    member: MemberStatistic,
    i18n: StoredValue<crate::i18n::I18nContext>,
) -> impl IntoView {
    let completion_color = if member.completion_rate >= 80.0 {
        "var(--success-color)"
    } else if member.completion_rate >= 50.0 {
        "var(--warning-color)"
    } else {
        "var(--danger-color)"
    };

    view! {
        <div style="padding: 1rem; margin-bottom: 1rem; background: var(--card-color); border: 1px solid var(--border-color); border-radius: 8px;">
            <div style="display: flex; justify-content: space-between; align-items: center; margin-bottom: 0.75rem;">
                <strong style="font-size: 1.1em;">{&member.username}</strong>
                <span style=format!("color: {}; font-weight: bold; font-size: 1.2em;", completion_color)>
                    {format!("{:.1}%", member.completion_rate)}
                </span>
            </div>
            <div style="font-size: 0.9em; color: var(--text-muted); margin-bottom: 0.75rem;">
                {i18n.get_value().t("statistics.completed")} ": "
                {member.total_completed} " / " {member.total_expected}
            </div>

            // Progress bar
            <div style="margin-bottom: 1rem;">
                <ProgressBar value=member.completion_rate />
            </div>

            // Task breakdown
            {if !member.task_stats.is_empty() {
                let summary = format!(
                    "{} ({} {})",
                    i18n.get_value().t("statistics.task_breakdown"),
                    member.task_stats.len(),
                    i18n.get_value().t("statistics.tasks")
                );
                view! {
                    <Accordion summary=summary>
                        {member.task_stats.into_iter().map(|task| {
                            let task_color = if task.completion_rate >= 80.0 {
                                "var(--success-color)"
                            } else if task.completion_rate >= 50.0 {
                                "var(--warning-color)"
                            } else {
                                "var(--danger-color)"
                            };
                            view! {
                                <div style="display: flex; justify-content: space-between; padding: 0.5rem 0; border-bottom: 1px solid var(--border-color);">
                                    <span>{&task.task_title}</span>
                                    <span style=format!("color: {};", task_color)>
                                        {task.completed} "/" {task.expected}
                                        " (" {format!("{:.0}%", task.completion_rate)} ")"
                                    </span>
                                </div>
                            }
                        }).collect_view()}
                    </Accordion>
                }.into_view()
            } else {
                view! {}.into_view()
            }}
        </div>
    }
}

fn format_week_display(week_start: &NaiveDate) -> String {
    let week_end = *week_start + chrono::Duration::days(6);
    format!(
        "{} - {}",
        week_start.format("%d.%m.%Y"),
        week_end.format("%d.%m.%Y")
    )
}

fn format_month_display(month: &NaiveDate) -> String {
    month.format("%B %Y").to_string()
}

fn apply_dark_mode(enabled: bool) {
    if let Some(window) = web_sys::window() {
        if let Some(document) = window.document() {
            if let Some(body) = document.body() {
                if enabled {
                    let _ = body.class_list().add_1("dark-mode");
                } else {
                    let _ = body.class_list().remove_1("dark-mode");
                }
            }
        }
    }
}
