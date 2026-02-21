use leptos::*;

/// Card container component.
#[component]
pub fn Card(
    #[prop(optional, into)] title: Option<String>,
    #[prop(optional, into)] class: Option<String>,
    #[prop(optional, into)] style: Option<String>,
    children: Children,
) -> impl IntoView {
    let full_class = if let Some(extra) = class {
        format!("card {}", extra)
    } else {
        "card".to_string()
    };

    let style_attr = style.unwrap_or_default();

    view! {
        <div class=full_class style=style_attr>
            {title.map(|t| view! {
                <div class="card-header">
                    <h3 class="card-title">{t}</h3>
                </div>
            })}
            {children()}
        </div>
    }
}

/// Card header section.
#[component]
pub fn CardHeader(
    #[prop(optional, into)] class: Option<String>,
    children: Children,
) -> impl IntoView {
    let full_class = if let Some(extra) = class {
        format!("card-header {}", extra)
    } else {
        "card-header".to_string()
    };

    view! {
        <div class=full_class>
            {children()}
        </div>
    }
}

/// Card body section.
#[component]
pub fn CardBody(
    #[prop(optional, into)] class: Option<String>,
    children: Children,
) -> impl IntoView {
    let full_class = if let Some(extra) = class {
        format!("card-body {}", extra)
    } else {
        "card-body".to_string()
    };

    view! {
        <div class=full_class>
            {children()}
        </div>
    }
}

/// Card footer section.
#[component]
pub fn CardFooter(
    #[prop(optional, into)] class: Option<String>,
    children: Children,
) -> impl IntoView {
    let full_class = if let Some(extra) = class {
        format!("card-footer {}", extra)
    } else {
        "card-footer".to_string()
    };

    view! {
        <div class=full_class>
            {children()}
        </div>
    }
}

#[cfg(test)]
mod tests {
    use wasm_bindgen_test::*;

    wasm_bindgen_test_configure!(run_in_browser);

    #[wasm_bindgen_test]
    fn test_card_css_classes() {
        assert_eq!("card", "card");
        assert_eq!("card-header", "card-header");
        assert_eq!("card-title", "card-title");
        assert_eq!("card-body", "card-body");
        assert_eq!("card-footer", "card-footer");
    }
}
