use leptos::*;

#[derive(Default, Clone, Copy, PartialEq)]
pub enum AlertVariant {
    #[default]
    Info,
    Success,
    Warning,
    Error,
}

/// Alert component for displaying messages.
#[component]
pub fn Alert(
    #[prop(optional)] variant: AlertVariant,
    #[prop(optional)] dismissible: bool,
    #[prop(optional)] on_dismiss: Option<Callback<()>>,
    #[prop(optional, into)] class: Option<String>,
    children: Children,
) -> impl IntoView {
    let variant_class = match variant {
        AlertVariant::Info => "alert",
        AlertVariant::Success => "alert alert-success",
        AlertVariant::Warning => "alert alert-warning",
        AlertVariant::Error => "alert alert-error",
    };

    let full_class = if let Some(extra) = class {
        format!("{} {}", variant_class, extra)
    } else {
        variant_class.to_string()
    };

    view! {
        <div class=full_class>
            {children()}
            {if dismissible {
                view! {
                    <button
                        class="alert-dismiss"
                        type="button"
                        on:click=move |_| {
                            if let Some(callback) = on_dismiss {
                                callback.call(());
                            }
                        }
                    >
                        "×"
                    </button>
                }.into_view()
            } else {
                view! {}.into_view()
            }}
        </div>
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use wasm_bindgen_test::*;

    wasm_bindgen_test_configure!(run_in_browser);

    #[wasm_bindgen_test]
    fn test_alert_variant_classes() {
        assert!(matches!(AlertVariant::Info, AlertVariant::Info));
        assert!(matches!(AlertVariant::Success, AlertVariant::Success));
        assert!(matches!(AlertVariant::Warning, AlertVariant::Warning));
        assert!(matches!(AlertVariant::Error, AlertVariant::Error));
    }
}
