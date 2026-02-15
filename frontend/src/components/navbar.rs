use leptos::*;
use leptos_router::*;

use crate::api::AuthState;
use crate::i18n::use_i18n;

#[component]
pub fn Navbar() -> impl IntoView {
    let auth_state = expect_context::<AuthState>();
    let i18n = use_i18n();
    let navigate = use_navigate();

    // Mobile menu state
    let is_menu_open = create_rw_signal(false);

    let toggle_menu = move |_| {
        is_menu_open.update(|open| *open = !*open);
    };

    let close_menu = move |_| {
        is_menu_open.set(false);
    };

    let on_logout = move |_| {
        is_menu_open.set(false);
        auth_state.logout();
        navigate("/login", Default::default());
    };

    let i18n_brand = i18n.clone();
    let i18n_dashboard = i18n.clone();
    let i18n_settings = i18n.clone();
    let i18n_logout = i18n.clone();

    view! {
        <nav class="navbar">
            <div class="container navbar-content">
                <a href="/" class="navbar-brand">{move || i18n_brand.t("nav.app_name")}</a>

                // Hamburger button (mobile only)
                <button class="navbar-toggle" on:click=toggle_menu aria-label="Toggle menu">
                    <span class="hamburger-line"></span>
                    <span class="hamburger-line"></span>
                    <span class="hamburger-line"></span>
                </button>

                // Navigation links
                <div class=move || if is_menu_open.get() { "navbar-links navbar-links--open" } else { "navbar-links" }>
                    <a href="/" on:click=close_menu>{move || i18n_dashboard.t("nav.dashboard")}</a>
                    <a href="/user-settings" on:click=close_menu>{move || i18n_settings.t("nav.settings")}</a>
                    <button class="btn btn-outline" on:click=on_logout>
                        {move || i18n_logout.t("nav.logout")}
                    </button>
                </div>
            </div>
        </nav>
    }
}
