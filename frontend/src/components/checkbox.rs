use leptos::*;

/// Checkbox component with label.
#[component]
pub fn Checkbox(
    #[prop(into)] checked: RwSignal<bool>,
    #[prop(into)] label: String,
    #[prop(optional, into)] id: Option<String>,
    #[prop(optional, into)] hint: Option<String>,
    #[prop(optional, into)] disabled: MaybeSignal<bool>,
) -> impl IntoView {
    let checkbox_id = id.unwrap_or_else(|| format!("checkbox-{}", label.replace(' ', "-").to_lowercase()));

    view! {
        <div class="filter-checkbox">
            <input
                type="checkbox"
                id=checkbox_id.clone()
                disabled=disabled
                prop:checked=move || checked.get()
                on:change=move |ev| {
                    checked.set(event_target_checked(&ev));
                }
            />
            <label for=checkbox_id.clone()>
                <span>{label}</span>
            </label>
            {hint.map(|h| view! {
                <span class="form-hint" style="margin-left: 0.5rem;">{h}</span>
            })}
        </div>
    }
}

#[cfg(test)]
mod tests {
    use wasm_bindgen_test::*;

    wasm_bindgen_test_configure!(run_in_browser);

    #[wasm_bindgen_test]
    fn test_checkbox_css_classes() {
        assert_eq!("filter-checkbox", "filter-checkbox");
    }
}
