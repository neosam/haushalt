use leptos::*;

#[derive(Default, Clone, Copy, PartialEq)]
pub enum ProgressVariant {
    #[default]
    Auto,    // Color based on percentage
    Success,
    Warning,
    Danger,
    Primary,
}

/// Progress bar component.
#[component]
pub fn ProgressBar(
    #[prop(into)] value: MaybeSignal<f32>,
    #[prop(optional)] variant: ProgressVariant,
    #[prop(optional, into)] height: Option<String>,
    #[prop(optional, into)] class: Option<String>,
) -> impl IntoView {
    let height = height.unwrap_or_else(|| "10px".to_string());

    let bar_color = move || {
        let v = value.get();
        match variant {
            ProgressVariant::Auto => {
                if v >= 80.0 {
                    "var(--success-color)"
                } else if v >= 50.0 {
                    "var(--warning-color)"
                } else {
                    "var(--danger-color)"
                }
            }
            ProgressVariant::Success => "var(--success-color)",
            ProgressVariant::Warning => "var(--warning-color)",
            ProgressVariant::Danger => "var(--danger-color)",
            ProgressVariant::Primary => "var(--primary-color)",
        }
    };

    let container_class = if let Some(extra) = class {
        format!("progress-bar-container {}", extra)
    } else {
        "progress-bar-container".to_string()
    };

    let container_style = format!(
        "background: var(--border-color); border-radius: 4px; height: {}; overflow: hidden;",
        height
    );

    view! {
        <div class=container_class style=container_style>
            <div style=move || format!(
                "background: {}; width: {:.1}%; height: {}; display: block; transition: width 0.3s;",
                bar_color(),
                value.get().clamp(0.0, 100.0),
                height
            )></div>
        </div>
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use wasm_bindgen_test::*;

    wasm_bindgen_test_configure!(run_in_browser);

    #[wasm_bindgen_test]
    fn test_progress_variant_classes() {
        assert!(matches!(ProgressVariant::Auto, ProgressVariant::Auto));
        assert!(matches!(ProgressVariant::Success, ProgressVariant::Success));
        assert!(matches!(ProgressVariant::Warning, ProgressVariant::Warning));
        assert!(matches!(ProgressVariant::Danger, ProgressVariant::Danger));
    }
}
