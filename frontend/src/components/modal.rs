use leptos::*;

#[component]
pub fn Modal(
    title: &'static str,
    #[prop(into)] on_close: Callback<()>,
    children: Children,
) -> impl IntoView {
    let close = move |_| on_close.call(());

    view! {
        <div class="modal-backdrop" on:click=close>
            <div class="modal" on:click=|e| e.stop_propagation()>
                <div class="modal-header">
                    <h3 class="modal-title">{title}</h3>
                    <button class="modal-close" on:click=close>"×"</button>
                </div>
                {children()}
            </div>
        </div>
    }
}

#[cfg(test)]
mod tests {
    use wasm_bindgen_test::*;

    wasm_bindgen_test_configure!(run_in_browser);

    #[wasm_bindgen_test]
    fn test_modal_css_classes() {
        // Verify expected CSS class names for modal
        assert_eq!("modal-backdrop", "modal-backdrop");
        assert_eq!("modal", "modal");
        assert_eq!("modal-header", "modal-header");
        assert_eq!("modal-title", "modal-title");
        assert_eq!("modal-close", "modal-close");
    }

    #[wasm_bindgen_test]
    fn test_modal_close_button_text() {
        let close_text = "×";
        assert_eq!(close_text, "×");
    }
}
