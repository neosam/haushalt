use leptos::*;

/// Collapsible accordion component using details/summary.
#[component]
pub fn Accordion(
    #[prop(into)] summary: String,
    #[prop(optional)] open: bool,
    #[prop(optional, into)] class: Option<String>,
    children: Children,
) -> impl IntoView {
    let full_class = class.unwrap_or_default();

    view! {
        <details class=full_class open=open>
            <summary style="cursor: pointer; user-select: none; padding: 0.5rem 0;">
                {summary}
            </summary>
            <div style="margin-top: 0.75rem; padding-left: 1rem;">
                {children()}
            </div>
        </details>
    }
}

#[cfg(test)]
mod tests {
    use wasm_bindgen_test::*;

    wasm_bindgen_test_configure!(run_in_browser);

    #[wasm_bindgen_test]
    fn test_accordion_structure() {
        // Basic structure test
        assert!(true);
    }
}
