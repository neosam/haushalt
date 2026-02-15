use leptos::*;

#[component]
pub fn PointsBadge(points: i64) -> impl IntoView {
    view! {
        <span class="points-badge">
            {points} " pts"
        </span>
    }
}

#[component]
pub fn PointsCard(points: i64, label: &'static str) -> impl IntoView {
    view! {
        <div class="card stat-card">
            <div class="stat-value">{points}</div>
            <div class="stat-label">{label}</div>
        </div>
    }
}
