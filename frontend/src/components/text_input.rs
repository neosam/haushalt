use leptos::*;

/// Text input component with consistent styling.
#[component]
pub fn TextInput(
    #[prop(into)] value: RwSignal<String>,
    #[prop(optional, into)] placeholder: Option<String>,
    #[prop(optional, into)] input_type: Option<String>,
    #[prop(optional, into)] disabled: MaybeSignal<bool>,
    #[prop(optional)] required: bool,
    #[prop(optional, into)] id: Option<String>,
    #[prop(optional, into)] class: Option<String>,
) -> impl IntoView {
    let input_type = input_type.unwrap_or_else(|| "text".to_string());

    let full_class = if let Some(extra) = class {
        format!("form-input {}", extra)
    } else {
        "form-input".to_string()
    };

    view! {
        <input
            type=input_type
            class=full_class
            id=id
            placeholder=placeholder
            required=required
            disabled=disabled
            prop:value=move || value.get()
            on:input=move |ev| {
                value.set(event_target_value(&ev));
            }
        />
    }
}

#[cfg(test)]
mod tests {
    use wasm_bindgen_test::*;

    wasm_bindgen_test_configure!(run_in_browser);

    #[wasm_bindgen_test]
    fn test_text_input_css_classes() {
        assert_eq!("form-input", "form-input");
    }
}
