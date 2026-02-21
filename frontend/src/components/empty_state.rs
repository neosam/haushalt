use leptos::*;

/// Empty state display component.
#[component]
pub fn EmptyState(
    #[prop(optional, into)] icon: Option<String>,
    #[prop(optional, into)] class: Option<String>,
    children: Children,
) -> impl IntoView {
    let full_class = if let Some(extra) = class {
        format!("empty-state {}", extra)
    } else {
        "empty-state".to_string()
    };

    view! {
        <div class=full_class>
            {icon.map(|i| view! {
                <span class="empty-state-icon">{i}</span>
            })}
            {children()}
        </div>
    }
}

#[cfg(test)]
mod tests {
    use wasm_bindgen_test::*;

    wasm_bindgen_test_configure!(run_in_browser);

    #[wasm_bindgen_test]
    fn test_empty_state_css_classes() {
        assert_eq!("empty-state", "empty-state");
        assert_eq!("empty-state-icon", "empty-state-icon");
    }
}
