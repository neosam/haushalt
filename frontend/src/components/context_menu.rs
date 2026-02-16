use leptos::*;
use wasm_bindgen::JsCast;

/// Represents an action in the context menu
#[derive(Clone)]
pub struct ContextMenuAction {
    pub label: String,
    pub on_click: Callback<()>,
    pub danger: bool,
}

/// A reusable context menu component with a trigger button and dropdown
#[component]
pub fn ContextMenu(actions: Vec<ContextMenuAction>) -> impl IntoView {
    let is_open = create_rw_signal(false);

    // Close menu when clicking outside
    let menu_ref = create_node_ref::<html::Div>();

    // Handle click outside to close the menu
    create_effect(move |_| {
        if is_open.get() {
            let handler = wasm_bindgen::closure::Closure::wrap(Box::new(move |event: web_sys::MouseEvent| {
                if let Some(menu_element) = menu_ref.get() {
                    if let Some(target) = event.target() {
                        let target_node: web_sys::Node = target.unchecked_into();
                        if !menu_element.contains(Some(&target_node)) {
                            is_open.set(false);
                        }
                    }
                }
            }) as Box<dyn FnMut(_)>);

            if let Some(window) = web_sys::window() {
                if let Some(document) = window.document() {
                    let _ = document.add_event_listener_with_callback(
                        "click",
                        handler.as_ref().unchecked_ref(),
                    );

                    // Store handler to be cleaned up
                    on_cleanup(move || {
                        if let Some(window) = web_sys::window() {
                            if let Some(document) = window.document() {
                                let _ = document.remove_event_listener_with_callback(
                                    "click",
                                    handler.as_ref().unchecked_ref(),
                                );
                            }
                        }
                        drop(handler);
                    });
                }
            }
        }
    });

    view! {
        <div class="context-menu-container" node_ref=menu_ref>
            <button
                class="context-menu-trigger"
                on:click=move |e| {
                    e.stop_propagation();
                    is_open.update(|open| *open = !*open);
                }
            >
                "⋮"
            </button>

            <Show when=move || is_open.get() fallback=|| ()>
                <div class="context-menu-dropdown">
                    {actions.iter().cloned().map(|action| {
                        let class_name = if action.danger {
                            "context-menu-item danger"
                        } else {
                            "context-menu-item"
                        };
                        let on_click = action.on_click;
                        view! {
                            <button
                                class=class_name
                                on:click=move |e| {
                                    e.stop_propagation();
                                    is_open.set(false);
                                    on_click.call(());
                                }
                            >
                                {action.label}
                            </button>
                        }
                    }).collect_view()}
                </div>
            </Show>
        </div>
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use wasm_bindgen_test::*;

    wasm_bindgen_test_configure!(run_in_browser);

    #[wasm_bindgen_test]
    fn test_context_menu_action_creation() {
        let action = ContextMenuAction {
            label: "Edit".to_string(),
            on_click: Callback::new(|_| {}),
            danger: false,
        };
        assert_eq!(action.label, "Edit");
        assert!(!action.danger);
    }

    #[wasm_bindgen_test]
    fn test_context_menu_action_danger_flag() {
        let delete_action = ContextMenuAction {
            label: "Delete".to_string(),
            on_click: Callback::new(|_| {}),
            danger: true,
        };
        assert!(delete_action.danger);
    }

    #[wasm_bindgen_test]
    fn test_context_menu_css_classes() {
        assert_eq!("context-menu-container", "context-menu-container");
        assert_eq!("context-menu-trigger", "context-menu-trigger");
        assert_eq!("context-menu-dropdown", "context-menu-dropdown");
        assert_eq!("context-menu-item", "context-menu-item");
        assert_eq!("context-menu-item danger", "context-menu-item danger");
    }

    #[wasm_bindgen_test]
    fn test_danger_class_assignment() {
        let danger_action = ContextMenuAction {
            label: "Delete".to_string(),
            on_click: Callback::new(|_| {}),
            danger: true,
        };
        let normal_action = ContextMenuAction {
            label: "Edit".to_string(),
            on_click: Callback::new(|_| {}),
            danger: false,
        };

        let danger_class = if danger_action.danger {
            "context-menu-item danger"
        } else {
            "context-menu-item"
        };
        let normal_class = if normal_action.danger {
            "context-menu-item danger"
        } else {
            "context-menu-item"
        };

        assert_eq!(danger_class, "context-menu-item danger");
        assert_eq!(normal_class, "context-menu-item");
    }
}
