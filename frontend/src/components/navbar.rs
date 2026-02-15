use leptos::*;
use leptos_router::*;

use crate::api::AuthState;

#[component]
pub fn Navbar() -> impl IntoView {
    let auth_state = expect_context::<AuthState>();
    let navigate = use_navigate();

    let on_logout = move |_| {
        auth_state.logout();
        navigate("/login", Default::default());
    };

    view! {
        <nav class="navbar">
            <div class="container navbar-content">
                <a href="/" class="navbar-brand">"Household Manager"</a>
                <div class="navbar-links">
                    <a href="/">"Dashboard"</a>
                    <a href="/settings">"Settings"</a>
                    <button class="btn btn-outline" on:click=on_logout>
                        "Logout"
                    </button>
                </div>
            </div>
        </nav>
    }
}
