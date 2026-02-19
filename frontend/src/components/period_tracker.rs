use leptos::*;
use shared::{PeriodDisplay, PeriodStatus};

/// Displays recent period results as a habit tracker row
/// Shows icons: ✓ completed, ✗ failed, - skipped
/// Hover tooltip shows the date
#[component]
pub fn PeriodTracker(
    /// Recent periods (oldest first, for left-to-right display)
    periods: Vec<PeriodDisplay>,
    /// Whether to show "in progress" indicator for today
    #[prop(default = false)]
    show_in_progress: bool,
) -> impl IntoView {
    if periods.is_empty() && !show_in_progress {
        return view! {}.into_view();
    }

    view! {
        <div class="period-tracker">
            {periods.into_iter().map(|p| {
                let date_str = p.period_start.format("%d.%m.%Y").to_string();
                let (icon, class) = match p.status {
                    PeriodStatus::Completed => ("✓", "period-completed"),
                    PeriodStatus::Failed => ("✗", "period-failed"),
                    PeriodStatus::Skipped => ("-", "period-skipped"),
                };
                view! {
                    <span class=format!("period-icon {}", class) title=date_str>
                        {icon}
                    </span>
                }
            }).collect_view()}
            {show_in_progress.then(|| view! {
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
) -> impl IntoView {
    if periods.is_empty() && !show_in_progress {
        return view! {}.into_view();
    }

    view! {
        <div class="period-tracker period-tracker-compact">
            {periods.into_iter().map(|p| {
                let date_str = p.period_start.format("%d.%m.%Y").to_string();
                let (icon, class) = match p.status {
                    PeriodStatus::Completed => ("✓", "period-completed"),
                    PeriodStatus::Failed => ("✗", "period-failed"),
                    PeriodStatus::Skipped => ("-", "period-skipped"),
                };
                view! {
                    <span class=format!("period-icon {}", class) title=date_str>
                        {icon}
                    </span>
                }
            }).collect_view()}
            {show_in_progress.then(|| view! {
                <span class="period-icon period-in-progress" title="Heute">
                    "○"
                </span>
            })}
        </div>
    }.into_view()
}
