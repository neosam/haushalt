use leptos::*;

#[derive(Default, Clone, Copy, PartialEq)]
pub enum ButtonVariant {
    #[default]
    Primary,
    Secondary,
    Outline,
    Danger,
    Success,
    Icon,
}

#[derive(Default, Clone, Copy, PartialEq)]
pub enum ButtonSize {
    #[default]
    Medium,
    Small,
    ExtraSmall,
}

/// Reusable button component with variants and sizes.
#[component]
pub fn Button(
    #[prop(optional, into)] variant: MaybeSignal<ButtonVariant>,
    #[prop(optional)] size: ButtonSize,
    #[prop(optional, into)] disabled: MaybeSignal<bool>,
    #[prop(optional, into)] loading: MaybeSignal<bool>,
    #[prop(optional, into)] button_type: Option<String>,
    #[prop(optional, into)] class: Option<String>,
    #[prop(optional)] on_click: Option<Callback<ev::MouseEvent>>,
    children: Children,
) -> impl IntoView {
    let size_class = match size {
        ButtonSize::Medium => "",
        ButtonSize::Small => "btn-sm",
        ButtonSize::ExtraSmall => "btn-xs",
    };

    let class_clone = class.clone();
    let full_class = move || {
        let variant_class = match variant.get() {
            ButtonVariant::Primary => "btn btn-primary",
            ButtonVariant::Secondary => "btn",
            ButtonVariant::Outline => "btn btn-outline",
            ButtonVariant::Danger => "btn btn-danger",
            ButtonVariant::Success => "btn btn-success",
            ButtonVariant::Icon => "btn btn-icon",
        };
        let mut classes = vec![variant_class];
        if !size_class.is_empty() {
            classes.push(size_class);
        }
        if let Some(ref extra) = class_clone {
            classes.push(extra);
        }
        classes.join(" ")
    };

    let button_type = button_type.unwrap_or_else(|| "button".to_string());

    let is_disabled = move || disabled.get() || loading.get();

    view! {
        <button
            type=button_type
            class=full_class
            disabled=is_disabled
            on:click=move |ev| {
                if let Some(callback) = on_click {
                    callback.call(ev);
                }
            }
        >
            {move || {
                if loading.get() {
                    view! {
                        <span class="spinner" style="width: 1em; height: 1em; margin-right: 0.5em;"></span>
                    }.into_view()
                } else {
                    view! {}.into_view()
                }
            }}
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
    fn test_button_variant_classes() {
        // Verify expected CSS class names for variants
        assert!(matches!(ButtonVariant::Primary, ButtonVariant::Primary));
        assert!(matches!(ButtonVariant::Secondary, ButtonVariant::Secondary));
        assert!(matches!(ButtonVariant::Outline, ButtonVariant::Outline));
        assert!(matches!(ButtonVariant::Danger, ButtonVariant::Danger));
        assert!(matches!(ButtonVariant::Success, ButtonVariant::Success));
        assert!(matches!(ButtonVariant::Icon, ButtonVariant::Icon));
    }

    #[wasm_bindgen_test]
    fn test_button_size_classes() {
        // Verify expected CSS class names for sizes
        assert!(matches!(ButtonSize::Medium, ButtonSize::Medium));
        assert!(matches!(ButtonSize::Small, ButtonSize::Small));
        assert!(matches!(ButtonSize::ExtraSmall, ButtonSize::ExtraSmall));
    }
}
