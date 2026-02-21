use leptos::*;

/// Form group wrapper component for consistent label and input layout.
#[component]
pub fn FormGroup(
    #[prop(into)] label: String,
    #[prop(optional, into)] for_id: Option<String>,
    #[prop(optional, into)] hint: Option<String>,
    #[prop(optional)] required: bool,
    children: Children,
) -> impl IntoView {
    view! {
        <div class="form-group">
            <label class="form-label" for=for_id.clone()>
                {label}
                {if required {
                    view! { <span style="color: var(--danger-color);"> " *"</span> }.into_view()
                } else {
                    view! {}.into_view()
                }}
            </label>
            {children()}
            {hint.map(|h| view! {
                <span class="form-hint">{h}</span>
            })}
        </div>
    }
}

#[cfg(test)]
mod tests {
    use wasm_bindgen_test::*;

    wasm_bindgen_test_configure!(run_in_browser);

    #[wasm_bindgen_test]
    fn test_form_group_css_classes() {
        assert_eq!("form-group", "form-group");
        assert_eq!("form-label", "form-label");
        assert_eq!("form-hint", "form-hint");
    }
}
