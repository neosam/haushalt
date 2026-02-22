use chrono::Utc;
use leptos::*;
use shared::{PeriodDisplay, PeriodStatus};

/// Check if today already has a completed/failed/skipped entry in periods
fn today_has_entry(periods: &[PeriodDisplay]) -> bool {
    let today = Utc::now().date_naive();
    periods.iter().any(|p| p.period_start == today)
}

/// Displays recent period results as a habit tracker row
/// Shows icons: ✓ completed, ✗ failed, - skipped
/// Hover tooltip shows the date
/// For bad habits, colors are inverted (completed = bad/red, failed = good/green)
#[component]
pub fn PeriodTracker(
    /// Recent periods (oldest first, for left-to-right display)
    periods: Vec<PeriodDisplay>,
    /// Whether to show "in progress" indicator for today
    #[prop(default = false)]
    show_in_progress: bool,
    /// Whether this is a bad habit (inverts completed/failed colors)
    #[prop(default = false)]
    is_bad_habit: bool,
) -> impl IntoView {
    // Don't show in-progress if today already has an entry
    let effective_show_in_progress = show_in_progress && !today_has_entry(&periods);

    if periods.is_empty() && !effective_show_in_progress {
        return view! {}.into_view();
    }

    view! {
        <div class="period-tracker">
            {periods.into_iter().map(|p| {
                let date_str = p.period_start.format("%d.%m.%Y").to_string();
                let (icon, class) = match p.status {
                    PeriodStatus::Completed => {
                        if is_bad_habit {
                            ("✓", "period-failed") // Bad: completed bad habit = red
                        } else {
                            ("✓", "period-completed")
                        }
                    }
                    PeriodStatus::Failed => {
                        if is_bad_habit {
                            ("✗", "period-completed") // Good: resisted bad habit = green
                        } else {
                            ("✗", "period-failed")
                        }
                    }
                    PeriodStatus::Skipped => ("-", "period-skipped"),
                };
                view! {
                    <span class=format!("period-icon {}", class) title=date_str>
                        {icon}
                    </span>
                }
            }).collect_view()}
            {effective_show_in_progress.then(|| view! {
                <span class="period-icon period-in-progress" title="Heute">
                    "○"
                </span>
            })}
        </div>
    }.into_view()
}

/// Compact version for list views (smaller icons)
#[component]
pub fn PeriodTrackerCompact(
    periods: Vec<PeriodDisplay>,
    #[prop(default = false)]
    show_in_progress: bool,
    /// Whether this is a bad habit (inverts completed/failed colors)
    #[prop(default = false)]
    is_bad_habit: bool,
) -> impl IntoView {
    // Don't show in-progress if today already has an entry
    let effective_show_in_progress = show_in_progress && !today_has_entry(&periods);

    if periods.is_empty() && !effective_show_in_progress {
        return view! {}.into_view();
    }

    view! {
        <div class="period-tracker period-tracker-compact">
            {periods.into_iter().map(|p| {
                let date_str = p.period_start.format("%d.%m.%Y").to_string();
                let (icon, class) = match p.status {
                    PeriodStatus::Completed => {
                        if is_bad_habit {
                            ("✓", "period-failed") // Bad: completed bad habit = red
                        } else {
                            ("✓", "period-completed")
                        }
                    }
                    PeriodStatus::Failed => {
                        if is_bad_habit {
                            ("✗", "period-completed") // Good: resisted bad habit = green
                        } else {
                            ("✗", "period-failed")
                        }
                    }
                    PeriodStatus::Skipped => ("-", "period-skipped"),
                };
                view! {
                    <span class=format!("period-icon {}", class) title=date_str>
                        {icon}
                    </span>
                }
            }).collect_view()}
            {effective_show_in_progress.then(|| view! {
                <span class="period-icon period-in-progress" title="Heute">
                    "○"
                </span>
            })}
        </div>
    }.into_view()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_today_has_entry_returns_true_when_today_exists() {
        let today = Utc::now().date_naive();
        let periods = vec![PeriodDisplay {
            period_start: today,
            status: PeriodStatus::Completed,
        }];
        assert!(today_has_entry(&periods));
    }

    #[test]
    fn test_today_has_entry_returns_false_when_today_missing() {
        let yesterday = Utc::now().date_naive() - chrono::Duration::days(1);
        let periods = vec![PeriodDisplay {
            period_start: yesterday,
            status: PeriodStatus::Completed,
        }];
        assert!(!today_has_entry(&periods));
    }

    #[test]
    fn test_today_has_entry_returns_false_for_empty_periods() {
        let periods: Vec<PeriodDisplay> = vec![];
        assert!(!today_has_entry(&periods));
    }

    #[test]
    fn test_today_has_entry_with_multiple_periods() {
        let today = Utc::now().date_naive();
        let yesterday = today - chrono::Duration::days(1);
        let periods = vec![
            PeriodDisplay {
                period_start: yesterday,
                status: PeriodStatus::Completed,
            },
            PeriodDisplay {
                period_start: today,
                status: PeriodStatus::Failed,
            },
        ];
        assert!(today_has_entry(&periods));
    }
}
