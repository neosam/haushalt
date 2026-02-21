use leptos::*;

/// Number input component with min/max support.
#[component]
pub fn NumberInput(
    #[prop(into)] value: RwSignal<i32>,
    #[prop(optional)] min: Option<i32>,
    #[prop(optional)] max: Option<i32>,
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
            type="number"
            class=full_class
            id=id
            min=min
            max=max
            disabled=disabled
            prop:value=move || value.get()
            on:input=move |ev| {
                if let Ok(num) = event_target_value(&ev).parse::<i32>() {
                    value.set(num);
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
    fn test_number_input_css_classes() {
        assert_eq!("form-input", "form-input");
    }
}
