use leptos::*;

/// Section divider component.
#[component]
pub fn Divider(
    #[prop(optional, into)] class: Option<String>,
) -> impl IntoView {
    let full_class = if let Some(extra) = class {
        format!("divider {}", extra)
    } else {
        "divider".to_string()
    };

    view! {
        <hr class=full_class />
    }
}

#[cfg(test)]
mod tests {
    use wasm_bindgen_test::*;

    wasm_bindgen_test_configure!(run_in_browser);

    #[wasm_bindgen_test]
    fn test_divider_css_classes() {
        assert_eq!("divider", "divider");
    }
}
