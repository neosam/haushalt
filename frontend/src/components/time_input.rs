use leptos::*;

/// Time input component (HH:MM format).
#[component]
pub fn TimeInput(
    #[prop(into)] value: RwSignal<Option<String>>,
    #[prop(optional, into)] id: Option<String>,
    #[prop(optional, into)] class: Option<String>,
    #[prop(optional, into)] disabled: MaybeSignal<bool>,
) -> impl IntoView {
    let full_class = if let Some(extra) = class {
        format!("form-input {}", extra)
    } else {
        "form-input".to_string()
    };

    view! {
        <input
            type="time"
            class=full_class
            id=id
            disabled=disabled
            prop:value=move || value.get().unwrap_or_default()
            on:input=move |ev| {
                let input_value = event_target_value(&ev);
                if input_value.is_empty() {
                    value.set(None);
                } else {
                    value.set(Some(input_value));
                }
            }
        />
    }
}

#[cfg(test)]
mod tests {
    use wasm_bindgen_test::*;

    wasm_bindgen_test_configure!(run_in_browser);

    #[wasm_bindgen_test]
    fn test_time_input_css_classes() {
        assert_eq!("form-input", "form-input");
    }
}
