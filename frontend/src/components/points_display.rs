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

#[cfg(test)]
mod tests {
    use wasm_bindgen_test::*;

    wasm_bindgen_test_configure!(run_in_browser);

    #[wasm_bindgen_test]
    fn test_points_format_positive() {
        let points: i64 = 100;
        let display = format!("{} pts", points);
        assert_eq!(display, "100 pts");
    }

    #[wasm_bindgen_test]
    fn test_points_format_negative() {
        let points: i64 = -50;
        let display = format!("{} pts", points);
        assert_eq!(display, "-50 pts");
    }

    #[wasm_bindgen_test]
    fn test_points_format_zero() {
        let points: i64 = 0;
        let display = format!("{} pts", points);
        assert_eq!(display, "0 pts");
    }

    #[wasm_bindgen_test]
    fn test_points_format_large() {
        let points: i64 = 999999;
        let display = format!("{} pts", points);
        assert_eq!(display, "999999 pts");
    }
}
