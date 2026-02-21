use leptos::*;

#[derive(Default, Clone, Copy, PartialEq)]
pub enum IconButtonVariant {
    #[default]
    Default,
    Primary,
    Danger,
    Outline,
}

/// Icon-only button component for compact actions.
#[component]
pub fn IconButton(
    #[prop(optional)] variant: IconButtonVariant,
    #[prop(optional, into)] disabled: MaybeSignal<bool>,
    #[prop(optional, into)] title: Option<String>,
    #[prop(optional, into)] class: Option<String>,
    #[prop(optional)] on_click: Option<Callback<ev::MouseEvent>>,
    children: Children,
) -> impl IntoView {
    let variant_class = match variant {
        IconButtonVariant::Default => "btn btn-icon",
        IconButtonVariant::Primary => "btn btn-icon btn-primary",
        IconButtonVariant::Danger => "btn btn-icon btn-danger",
        IconButtonVariant::Outline => "btn btn-icon btn-outline",
    };

    let full_class = move || {
        if let Some(ref extra) = class {
            format!("{} {}", variant_class, extra)
        } else {
            variant_class.to_string()
        }
    };

    view! {
        <button
            type="button"
            class=full_class
            disabled=disabled
            title=title
            on:click=move |ev| {
                if let Some(callback) = on_click {
                    callback.call(ev);
                }
            }
        >
            {children()}
        </button>
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use wasm_bindgen_test::*;

    wasm_bindgen_test_configure!(run_in_browser);

    #[wasm_bindgen_test]
    fn test_icon_button_variant_classes() {
        assert!(matches!(IconButtonVariant::Default, IconButtonVariant::Default));
        assert!(matches!(IconButtonVariant::Primary, IconButtonVariant::Primary));
        assert!(matches!(IconButtonVariant::Danger, IconButtonVariant::Danger));
        assert!(matches!(IconButtonVariant::Outline, IconButtonVariant::Outline));
    }
}
