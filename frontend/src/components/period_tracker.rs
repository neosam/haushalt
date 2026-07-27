use chrono::Utc;
use leptos::*;
use shared::{PeriodDisplay, PeriodStatus};

/// Check if today already has a completed/failed/skipped entry in periods
fn today_has_entry(periods: &[PeriodDisplay]) -> bool {
    let today = Utc::now().date_naive();
    periods.iter().any(|p| p.period_start == today)
}

/// How one period is rendered: its icon and the CSS class carrying the colour.
///
/// Bad habits invert the verdict — indulging is the failure, resisting is the success. Bonus
/// tasks have no verdict to invert: doing them counts, not doing them is simply nothing. They
/// are never marked red, because there is nothing they could have failed at. New periods no
/// longer arrive as `Failed` at all (see `background_jobs.rs`), but rows finalized before that
/// fix still exist and must not turn a bonus task's history red retroactively.
fn period_appearance(
    status: PeriodStatus,
    is_bad_habit: bool,
    is_bonus: bool,
) -> (&'static str, &'static str) {
    match status {
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
            } else if is_bonus {
                ("-", "period-skipped") // Nothing was owed, so nothing was missed
            } else {
                ("✗", "period-failed")
            }
        }
        PeriodStatus::Skipped => ("-", "period-skipped"),
    }
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
    /// Whether this is a bonus task (never marked as failed)
    #[prop(default = false)]
    is_bonus: bool,
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
                let (icon, class) = period_appearance(p.status, is_bad_habit, is_bonus);
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
    /// Whether this is a bonus task (never marked as failed)
    #[prop(default = false)]
    is_bonus: bool,
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
                let (icon, class) = period_appearance(p.status, is_bad_habit, is_bonus);
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

    /// The whole point of the bonus archetype: it can never have failed at anything, so its
    /// history must never contain a red mark.
    #[test]
    fn test_bonus_failed_period_renders_neutral_not_red() {
        let (icon, class) = period_appearance(PeriodStatus::Failed, false, true);
        assert_eq!(class, "period-skipped");
        assert_eq!(icon, "-");
    }

    #[test]
    fn test_bonus_completed_period_stays_green() {
        let (icon, class) = period_appearance(PeriodStatus::Completed, false, true);
        assert_eq!(class, "period-completed");
        assert_eq!(icon, "✓");
    }

    #[test]
    fn test_ordinary_failed_period_stays_red() {
        let (_, class) = period_appearance(PeriodStatus::Failed, false, false);
        assert_eq!(class, "period-failed");
    }

    /// A bad habit keeps its inverted reading even if it somehow carries no target.
    #[test]
    fn test_bad_habit_inversion_wins_over_bonus() {
        let (_, completed) = period_appearance(PeriodStatus::Completed, true, true);
        let (_, failed) = period_appearance(PeriodStatus::Failed, true, true);
        assert_eq!(completed, "period-failed");
        assert_eq!(failed, "period-completed");
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
