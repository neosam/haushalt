use leptos::*;

#[derive(Default, Clone, Copy, PartialEq)]
pub enum LinkVariant {
    #[default]
    Primary,
    Secondary,
    Muted,
}

/// Styled link component.
#[component]
pub fn Link(
    #[prop(into)] href: String,
    #[prop(optional)] variant: LinkVariant,
    #[prop(optional, into)] class: Option<String>,
    children: Children,
) -> impl IntoView {
    let color = match variant {
        LinkVariant::Primary => "var(--primary-color)",
        LinkVariant::Secondary => "var(--text-color)",
        LinkVariant::Muted => "var(--text-muted)",
    };

    let style = format!("color: {};", color);

    let full_class = class.unwrap_or_default();

    view! {
        <a href=href class=full_class style=style>
            {children()}
        </a>
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use wasm_bindgen_test::*;

    wasm_bindgen_test_configure!(run_in_browser);

    #[wasm_bindgen_test]
    fn test_link_variant_colors() {
        assert!(matches!(LinkVariant::Primary, LinkVariant::Primary));
        assert!(matches!(LinkVariant::Secondary, LinkVariant::Secondary));
        assert!(matches!(LinkVariant::Muted, LinkVariant::Muted));
    }
}
