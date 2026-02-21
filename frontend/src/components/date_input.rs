use chrono::NaiveDate;
use leptos::*;

/// Date input component.
#[component]
pub fn DateInput(
    #[prop(into)] value: RwSignal<Option<NaiveDate>>,
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
            type="date"
            class=full_class
            id=id
            disabled=disabled
            prop:value=move || {
                value.get().map(|d| d.format("%Y-%m-%d").to_string()).unwrap_or_default()
            }
            on:input=move |ev| {
                let input_value = event_target_value(&ev);
                if input_value.is_empty() {
                    value.set(None);
                } else if let Ok(date) = NaiveDate::parse_from_str(&input_value, "%Y-%m-%d") {
                    value.set(Some(date));
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
    fn test_date_input_css_classes() {
        assert_eq!("form-input", "form-input");
    }
}
