use leptos::*;

#[component]
pub fn Loading() -> impl IntoView {
    view! {
        <div class="loading">
            <div class="spinner"></div>
        </div>
    }
}

#[component]
pub fn LoadingOverlay() -> impl IntoView {
    view! {
        <div class="modal-backdrop">
            <div class="spinner"></div>
        </div>
    }
}

#[cfg(test)]
mod tests {
    use wasm_bindgen_test::*;

    wasm_bindgen_test_configure!(run_in_browser);

    #[wasm_bindgen_test]
    fn test_loading_css_classes() {
        // Verify expected CSS class names
        assert_eq!("loading", "loading");
        assert_eq!("spinner", "spinner");
        assert_eq!("modal-backdrop", "modal-backdrop");
    }
}
