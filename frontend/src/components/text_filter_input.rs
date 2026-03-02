use leptos::*;

/// Text filter input component that prevents input focus loss during typing.
///
/// This component is separated from parent pages to ensure stable rendering.
/// The parent signal updates trigger re-renders, but this component remains
/// independent to preserve input focus state.
#[component]
pub fn TextFilterInput(
    /// Callback that receives the current filter text
    on_change: Callback<String>,
    /// Placeholder text for the input
    placeholder: String,
    /// Optional id for label association
    #[prop(optional)]
    id: Option<String>,
) -> impl IntoView {
    // Store the callback to prevent it from being recreated
    let on_change_stored = store_value(on_change);
    let input_id = id.unwrap_or_default();
    let aria_label = placeholder.clone();

    view! {
        <div class="filter-controls">
            <input
                type="text"
                class="text-filter-input"
                placeholder=placeholder
                id=input_id
                aria-label=aria_label
                on:input=move |ev| {
                    // CRITICAL: Use untrack to prevent reactive tracking
                    //
                    // Without untrack, accessing signals inside this event handler
                    // would create reactive dependencies. When those signals update,
                    // Leptos would re-render the parent component, which recreates
                    // the input element, causing focus loss during typing.
                    //
                    // By using untrack, we break the reactive dependency chain:
                    // - The input value changes don't trigger parent re-renders
                    // - The input element remains stable in the DOM
                    // - User focus is preserved while typing
                    //
                    // The callback pattern (store_value + untrack) ensures:
                    // 1. The callback itself is stable across renders
                    // 2. Calling the callback doesn't create reactive dependencies
                    // 3. Parent signals can still update independently without affecting input focus
                    untrack(move || {
                        let value = event_target_value(&ev);
                        on_change_stored.get_value().call(value);
                    });
                }
            />
        </div>
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use wasm_bindgen::JsCast;
    use wasm_bindgen_test::*;
    use web_sys::HtmlInputElement;

    wasm_bindgen_test_configure!(run_in_browser);

    fn get_test_container() -> web_sys::HtmlElement {
        let document = web_sys::window().unwrap().document().unwrap();
        let container = document.create_element("div").unwrap();
        container.set_id("test-container");
        document.body().unwrap().append_child(&container).unwrap();
        container.unchecked_into()
    }

    fn cleanup_test_container() {
        let document = web_sys::window().unwrap().document().unwrap();
        if let Some(container) = document.get_element_by_id("test-container") {
            container.remove();
        }
    }

    #[wasm_bindgen_test]
    fn test_renders_with_correct_placeholder() {
        let runtime = create_runtime();
        let container = get_test_container();
        let callback = Callback::new(|_: String| {});

        mount_to(container.clone().unchecked_into(), move || {
            view! { <TextFilterInput on_change=callback placeholder="Search tasks".to_string() /> }
        });

        let input: HtmlInputElement = container
            .query_selector("input.text-filter-input")
            .unwrap()
            .unwrap()
            .unchecked_into();
        assert_eq!(input.placeholder(), "Search tasks");

        cleanup_test_container();
        runtime.dispose();
    }

    #[wasm_bindgen_test]
    fn test_renders_with_id_attribute() {
        let runtime = create_runtime();
        let container = get_test_container();
        let callback = Callback::new(|_: String| {});

        mount_to(container.clone().unchecked_into(), move || {
            view! { <TextFilterInput on_change=callback placeholder="Search".to_string() id="my-filter".to_string() /> }
        });

        let input: HtmlInputElement = container
            .query_selector("#my-filter")
            .unwrap()
            .unwrap()
            .unchecked_into();
        assert_eq!(input.id(), "my-filter");

        cleanup_test_container();
        runtime.dispose();
    }

    #[wasm_bindgen_test]
    fn test_renders_with_aria_label() {
        let runtime = create_runtime();
        let container = get_test_container();
        let callback = Callback::new(|_: String| {});

        mount_to(container.clone().unchecked_into(), move || {
            view! { <TextFilterInput on_change=callback placeholder="Filter by title".to_string() /> }
        });

        let input: HtmlInputElement = container
            .query_selector("input.text-filter-input")
            .unwrap()
            .unwrap()
            .unchecked_into();
        assert_eq!(
            input.get_attribute("aria-label").unwrap(),
            "Filter by title"
        );

        cleanup_test_container();
        runtime.dispose();
    }

    #[wasm_bindgen_test]
    fn test_callback_called_on_input() {
        let runtime = create_runtime();
        let container = get_test_container();

        let received_value = create_rw_signal(String::new());
        let callback = Callback::new(move |value: String| {
            received_value.set(value);
        });

        mount_to(container.clone().unchecked_into(), move || {
            view! { <TextFilterInput on_change=callback placeholder="Search".to_string() /> }
        });

        let input: HtmlInputElement = container
            .query_selector("input.text-filter-input")
            .unwrap()
            .unwrap()
            .unchecked_into();

        // Simulate typing by setting value and dispatching input event
        input.set_value("test query");
        let event = web_sys::Event::new("input").unwrap();
        input.dispatch_event(&event).unwrap();

        assert_eq!(received_value.get_untracked(), "test query");

        cleanup_test_container();
        runtime.dispose();
    }

    #[wasm_bindgen_test]
    fn test_focus_preserved_after_input_event() {
        let runtime = create_runtime();
        let container = get_test_container();

        let callback = Callback::new(|_: String| {});

        mount_to(container.clone().unchecked_into(), move || {
            view! { <TextFilterInput on_change=callback placeholder="Search".to_string() id="focus-test".to_string() /> }
        });

        let input: HtmlInputElement = container
            .query_selector("#focus-test")
            .unwrap()
            .unwrap()
            .unchecked_into();

        // Focus the input
        input.focus().unwrap();

        // Simulate input event
        input.set_value("typing");
        let event = web_sys::Event::new("input").unwrap();
        input.dispatch_event(&event).unwrap();

        // Verify focus is preserved
        let document = web_sys::window().unwrap().document().unwrap();
        let active = document.active_element().unwrap();
        assert_eq!(active.id(), "focus-test");

        cleanup_test_container();
        runtime.dispose();
    }

    #[wasm_bindgen_test]
    fn test_renders_inside_filter_controls_div() {
        let runtime = create_runtime();
        let container = get_test_container();
        let callback = Callback::new(|_: String| {});

        mount_to(container.clone().unchecked_into(), move || {
            view! { <TextFilterInput on_change=callback placeholder="Search".to_string() /> }
        });

        let wrapper = container.query_selector("div.filter-controls").unwrap();
        assert!(wrapper.is_some());

        let input = container.query_selector("div.filter-controls > input.text-filter-input").unwrap();
        assert!(input.is_some());

        cleanup_test_container();
        runtime.dispose();
    }
}
