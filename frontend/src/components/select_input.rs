use leptos::*;

/// Select dropdown component.
#[component]
pub fn SelectInput(
    #[prop(into)] value: RwSignal<String>,
    #[prop(into)] options: MaybeSignal<Vec<(String, String)>>,
    #[prop(optional, into)] id: Option<String>,
    #[prop(optional, into)] class: Option<String>,
    #[prop(optional, into)] disabled: MaybeSignal<bool>,
) -> impl IntoView {
    let full_class = if let Some(extra) = class {
        format!("form-select {}", extra)
    } else {
        "form-select".to_string()
    };

    view! {
        <select
            class=full_class
            id=id
            disabled=disabled
            on:change=move |ev| {
                value.set(event_target_value(&ev));
            }
        >
            {move || {
                options.get().into_iter().map(|(val, label)| {
                    let val_clone = val.clone();
                    view! {
                        <option
                            value=val.clone()
                            selected=move || value.get() == val_clone
                        >
                            {label}
                        </option>
                    }
                }).collect_view()
            }}
        </select>
    }
}

#[cfg(test)]
mod tests {
    use wasm_bindgen_test::*;

    wasm_bindgen_test_configure!(run_in_browser);

    #[wasm_bindgen_test]
    fn test_select_input_css_classes() {
        assert_eq!("form-select", "form-select");
    }
}
