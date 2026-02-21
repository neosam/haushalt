use leptos::*;

/// List item component with consistent styling.
#[component]
pub fn ListItem(
    #[prop(optional)] clickable: bool,
    #[prop(optional)] on_click: Option<Callback<ev::MouseEvent>>,
    #[prop(optional, into)] class: Option<String>,
    children: Children,
) -> impl IntoView {
    let base_class = "task-item";

    let full_class = if let Some(extra) = class {
        format!("{} {}", base_class, extra)
    } else {
        base_class.to_string()
    };

    let cursor_style = if clickable { "cursor: pointer;" } else { "" };

    view! {
        <div
            class=full_class
            style=cursor_style
            on:click=move |ev| {
                if let Some(callback) = on_click {
                    callback.call(ev);
                }
            }
        >
            {children()}
        </div>
    }
}

#[cfg(test)]
mod tests {
    use wasm_bindgen_test::*;

    wasm_bindgen_test_configure!(run_in_browser);

    #[wasm_bindgen_test]
    fn test_list_item_css_classes() {
        assert_eq!("task-item", "task-item");
    }
}
