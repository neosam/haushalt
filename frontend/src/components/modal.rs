use leptos::*;

#[component]
pub fn Modal(
    title: &'static str,
    #[prop(into)] on_close: Callback<()>,
    children: Children,
) -> impl IntoView {
    let close = move |_| on_close.call(());

    view! {
        <div class="modal-backdrop" on:click=close.clone()>
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
