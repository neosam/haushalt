use leptos::*;

#[derive(Default, Clone, Copy, PartialEq)]
pub enum ActionBarAlign {
    #[default]
    End,
    Start,
    Between,
    Center,
}

/// Action bar for grouping buttons.
#[component]
pub fn ActionBar(
    #[prop(optional)] align: ActionBarAlign,
    #[prop(optional, into)] class: Option<String>,
    children: Children,
) -> impl IntoView {
    let justify = match align {
        ActionBarAlign::End => "flex-end",
        ActionBarAlign::Start => "flex-start",
        ActionBarAlign::Between => "space-between",
        ActionBarAlign::Center => "center",
    };

    let style = format!(
        "display: flex; gap: 0.5rem; align-items: center; justify-content: {};",
        justify
    );

    let full_class = class.unwrap_or_default();

    view! {
        <div class=full_class style=style>
            {children()}
        </div>
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use wasm_bindgen_test::*;

    wasm_bindgen_test_configure!(run_in_browser);

    #[wasm_bindgen_test]
    fn test_action_bar_align_variants() {
        assert!(matches!(ActionBarAlign::End, ActionBarAlign::End));
        assert!(matches!(ActionBarAlign::Start, ActionBarAlign::Start));
        assert!(matches!(ActionBarAlign::Between, ActionBarAlign::Between));
        assert!(matches!(ActionBarAlign::Center, ActionBarAlign::Center));
    }
}
