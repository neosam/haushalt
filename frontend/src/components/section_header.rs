use leptos::*;

#[derive(Default, Clone, Copy, PartialEq)]
pub enum HeaderLevel {
    H2,
    #[default]
    H3,
    H4,
}

/// Section header component with consistent styling.
#[component]
pub fn SectionHeader(
    #[prop(optional)] level: HeaderLevel,
    #[prop(optional, into)] class: Option<String>,
    children: Children,
) -> impl IntoView {
    let style = "margin-bottom: 1rem;";

    let full_class = class.unwrap_or_default();

    match level {
        HeaderLevel::H2 => view! {
            <h2 class=full_class style=style>{children()}</h2>
        }.into_view(),
        HeaderLevel::H3 => view! {
            <h3 class=full_class style=style>{children()}</h3>
        }.into_view(),
        HeaderLevel::H4 => view! {
            <h4 class=full_class style=style>{children()}</h4>
        }.into_view(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use wasm_bindgen_test::*;

    wasm_bindgen_test_configure!(run_in_browser);

    #[wasm_bindgen_test]
    fn test_header_level_variants() {
        assert!(matches!(HeaderLevel::H2, HeaderLevel::H2));
        assert!(matches!(HeaderLevel::H3, HeaderLevel::H3));
        assert!(matches!(HeaderLevel::H4, HeaderLevel::H4));
    }
}
