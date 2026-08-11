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
use crate::utils::timezone::today_in_tz;

const DATE_FORMAT: &str = "%Y-%m-%d";

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

    // Range recalculation state
    let range_from = create_rw_signal(String::new());
    let range_to = create_rw_signal(String::new());
    let recalculating = create_rw_signal(false);
    let recalculate_result = create_rw_signal(Option::<String>::None);
    let show_recalculate = create_rw_signal(false);

    // Bumped whenever stored statistics change, so the display reloads
    let reload_trigger = create_rw_signal(0u32);

    let week_start_day = create_memo(move |_| {
        settings.get().map(|s| s.week_start_day).unwrap_or(0)
    });

    // Load settings and the lists of already calculated periods
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
            loading.set(false);
        });

        refresh_available_periods(id, available_weeks, available_months);
    });

    // Preselect the running period once settings are known. Any period can be picked
    // afterwards — including ones that have never been calculated.
    create_effect(move |_| {
        let Some(s) = settings.get() else {
            return;
        };

        let today = today_in_tz(&s.timezone);

        if selected_week.get_untracked().is_none() {
            selected_week.set(Some(shared::week_start_for(today, s.week_start_day)));
        }
        if selected_month.get_untracked().is_none() {
            selected_month.set(Some(shared::month_start_for(today)));
        }
        if range_to.get_untracked().is_empty() {
            range_to.set(today.format(DATE_FORMAT).to_string());
        }
        if range_from.get_untracked().is_empty() {
            let three_months_ago = shared::month_start_for(today - chrono::Duration::days(90));
            range_from.set(three_months_ago.format(DATE_FORMAT).to_string());
        }
    });

    // Load statistics when selection changes
    create_effect(move |_| {
        let id = household_id();
        reload_trigger.get();
        if id.is_empty() {
            return;
        }

        if let Some(week) = selected_week.get() {
            let week_str = week.format(DATE_FORMAT).to_string();
            wasm_bindgen_futures::spawn_local(async move {
                match ApiClient::get_weekly_statistics(&id, Some(&week_str)).await {
                    Ok(stats) => weekly_stats.set(Some(stats)),
                    Err(e) => error.set(Some(e)),
                }
            });
        }
    });

    create_effect(move |_| {
        let id = household_id();
        reload_trigger.get();
        if id.is_empty() {
            return;
        }

        if let Some(month) = selected_month.get() {
            let month_str = month.format(DATE_FORMAT).to_string();
            wasm_bindgen_futures::spawn_local(async move {
                match ApiClient::get_monthly_statistics(&id, Some(&month_str)).await {
                    Ok(stats) => monthly_stats.set(Some(stats)),
                    Err(e) => error.set(Some(e)),
                }
            });
        }
    });

    // Calculate statistics for the selected period
    let on_calculate = move |_| {
        let id = household_id();
        calculating.set(true);
        error.set(None);
        recalculate_result.set(None);

        match current_view.get() {
            StatisticsView::Weekly => {
                let week = selected_week.get().map(|w| w.format(DATE_FORMAT).to_string());
                wasm_bindgen_futures::spawn_local(async move {
                    match ApiClient::calculate_weekly_statistics(&id, week.as_deref()).await {
                        Ok(stats) => {
                            weekly_stats.set(Some(stats));
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
                let month = selected_month.get().map(|m| m.format(DATE_FORMAT).to_string());
                wasm_bindgen_futures::spawn_local(async move {
                    match ApiClient::calculate_monthly_statistics(&id, month.as_deref()).await {
                        Ok(stats) => {
                            monthly_stats.set(Some(stats));
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

    // Recalculate every period covering the given range
    let on_recalculate = move |_| {
        let id = household_id();
        let from = range_from.get();
        let to = range_to.get();

        if from.is_empty() || to.is_empty() {
            return;
        }

        recalculating.set(true);
        error.set(None);
        recalculate_result.set(None);

        let weekly = current_view.get() == StatisticsView::Weekly;
        // Read the templates up front — the async block runs outside the reactive scope
        let done_template = i18n_stored.get_value().t("statistics.recalculate_done");
        let none_message = i18n_stored.get_value().t("statistics.recalculate_none");

        wasm_bindgen_futures::spawn_local(async move {
            let result = if weekly {
                ApiClient::recalculate_weekly_statistics(&id, &from, &to).await
            } else {
                ApiClient::recalculate_monthly_statistics(&id, &from, &to).await
            };

            match result {
                Ok(response) => {
                    recalculate_result.set(Some(format_recalculation_result(
                        &response,
                        &done_template,
                        &none_message,
                    )));
                    refresh_available_periods(id, available_weeks, available_months);
                    reload_trigger.update(|n| *n += 1);
                }
                Err(e) => error.set(Some(e)),
            }
            recalculating.set(false);
        });
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

                    // Free period picker — any week/month, calculated or not
                    <div style="display: flex; align-items: center; gap: 0.5rem;">
                        <label class="form-label" for="statistics-period" style="margin: 0;">
                            {i18n_stored.get_value().t("statistics.period")}
                        </label>
                        <input
                            id="statistics-period"
                            type="date"
                            class="form-input"
                            style="width: auto;"
                            prop:value=move || {
                                let selected = if current_view.get() == StatisticsView::Weekly {
                                    selected_week.get()
                                } else {
                                    selected_month.get()
                                };
                                selected.map(|d| d.format(DATE_FORMAT).to_string()).unwrap_or_default()
                            }
                            on:change=move |ev| {
                                let Ok(date) = NaiveDate::parse_from_str(&event_target_value(&ev), DATE_FORMAT) else {
                                    return;
                                };
                                if current_view.get() == StatisticsView::Weekly {
                                    selected_week.set(Some(shared::week_start_for(date, week_start_day.get())));
                                } else {
                                    selected_month.set(Some(shared::month_start_for(date)));
                                }
                            }
                        />
                    </div>

                    // Quick jump to a period that already has data
                    {move || {
                        let weekly = current_view.get() == StatisticsView::Weekly;
                        let periods = if weekly { available_weeks.get() } else { available_months.get() };

                        if periods.is_empty() {
                            return view! {}.into_view();
                        }

                        let selected = if weekly { selected_week.get() } else { selected_month.get() };
                        // A freely picked period is usually not in the list — keep the
                        // placeholder selected then, so the dropdown never claims otherwise
                        let is_listed = selected.map(|s| periods.contains(&s)).unwrap_or(false);

                        view! {
                            <select
                                class="form-select"
                                style="width: auto; min-width: 200px;"
                                on:change=move |ev| {
                                    let Ok(date) = NaiveDate::parse_from_str(&event_target_value(&ev), DATE_FORMAT) else {
                                        return;
                                    };
                                    if current_view.get() == StatisticsView::Weekly {
                                        selected_week.set(Some(date));
                                    } else {
                                        selected_month.set(Some(date));
                                    }
                                }
                            >
                                <option value="" selected=!is_listed disabled>
                                    {i18n_stored.get_value().t("statistics.existing_periods")}
                                </option>
                                {periods.into_iter().map(|period| {
                                    let value = period.format(DATE_FORMAT).to_string();
                                    let display = if weekly {
                                        format_week_display(&period)
                                    } else {
                                        format_month_display(&period)
                                    };
                                    view! {
                                        <option value=value selected=selected == Some(period)>
                                            {display}
                                        </option>
                                    }
                                }).collect_view()}
                            </select>
                        }.into_view()
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

                // Recalculate a whole range of periods at once
                <div style="margin-top: 1rem; border-top: 1px solid var(--border-color); padding-top: 0.75rem;">
                    <Button
                        variant=ButtonVariant::Secondary
                        on_click=Callback::new(move |_| show_recalculate.update(|open| *open = !*open))
                    >
                        {move || format!(
                            "{} {}",
                            if show_recalculate.get() { "▾" } else { "▸" },
                            i18n_stored.get_value().t("statistics.recalculate_range")
                        )}
                    </Button>

                    <Show when=move || show_recalculate.get() fallback=|| ()>
                        <div style="display: flex; gap: 1rem; align-items: flex-end; flex-wrap: wrap; margin-top: 0.75rem;">
                            <div>
                                <label class="form-label" for="statistics-range-from">
                                    {i18n_stored.get_value().t("statistics.range_from")}
                                </label>
                                <input
                                    id="statistics-range-from"
                                    type="date"
                                    class="form-input"
                                    style="width: auto;"
                                    prop:value=move || range_from.get()
                                    on:change=move |ev| range_from.set(event_target_value(&ev))
                                />
                            </div>
                            <div>
                                <label class="form-label" for="statistics-range-to">
                                    {i18n_stored.get_value().t("statistics.range_to")}
                                </label>
                                <input
                                    id="statistics-range-to"
                                    type="date"
                                    class="form-input"
                                    style="width: auto;"
                                    prop:value=move || range_to.get()
                                    on:change=move |ev| range_to.set(event_target_value(&ev))
                                />
                            </div>
                            <Button
                                disabled=MaybeSignal::derive(move || recalculating.get())
                                on_click=Callback::new(on_recalculate)
                            >
                                {move || if recalculating.get() {
                                    i18n_stored.get_value().t("statistics.calculating")
                                } else {
                                    i18n_stored.get_value().t("statistics.recalculate")
                                }}
                            </Button>
                        </div>

                        <p style="margin: 0.5rem 0 0; font-size: 0.9em; color: var(--text-muted);">
                            {i18n_stored.get_value().t("statistics.recalculate_hint")}
                        </p>

                        {move || recalculate_result.get().map(|message| view! {
                            <p style="margin: 0.5rem 0 0; color: var(--success-color);">{message}</p>
                        })}
                    </Show>
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
                view! { <NoMemberData i18n=i18n /> }.into_view()
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
                view! { <NoMemberData i18n=i18n /> }.into_view()
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

/// Shown when a period has no stored member statistics — either because nothing was
/// calculated yet, or because there was nothing to count.
#[component]
fn NoMemberData(i18n: StoredValue<crate::i18n::I18nContext>) -> impl IntoView {
    view! {
        <p>{i18n.get_value().t("statistics.no_member_data")}</p>
        <p style="color: var(--text-muted);">{i18n.get_value().t("statistics.click_calculate")}</p>
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

/// Reload both lists of periods that already have stored statistics
fn refresh_available_periods(
    household_id: String,
    available_weeks: RwSignal<Vec<NaiveDate>>,
    available_months: RwSignal<Vec<NaiveDate>>,
) {
    wasm_bindgen_futures::spawn_local(async move {
        if let Ok(weeks) = ApiClient::list_available_weeks(&household_id).await {
            available_weeks.set(weeks);
        }
        if let Ok(months) = ApiClient::list_available_months(&household_id).await {
            available_months.set(months);
        }
    });
}

/// Turn a recalculation response into the confirmation shown to the user
fn format_recalculation_result(
    response: &shared::RecalculateStatisticsResponse,
    done_template: &str,
    none_message: &str,
) -> String {
    match (response.first_period, response.last_period) {
        (Some(first), Some(last)) => format!(
            "{} ({} - {})",
            done_template.replace("{count}", &response.periods_calculated.to_string()),
            first.format("%d.%m.%Y"),
            last.format("%d.%m.%Y")
        ),
        _ => none_message.to_string(),
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
