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
