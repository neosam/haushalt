use leptos::*;
use leptos_router::*;

use crate::api::{ApiClient, AuthState};
use crate::components::navbar::Navbar;
use crate::i18n::{provide_i18n, use_i18n};
use crate::pages::{
    activity::ActivityPage, chat::ChatPage, dashboard::Dashboard, household::HouseholdPage,
    household_settings::HouseholdSettingsPage, login::Login, notes::NotesPage,
    punishments::PunishmentsPage, register::Register, rewards::RewardsPage,
    settings::SettingsPage, tasks::TasksPage, user_settings::UserSettingsPage,
};

#[component]
pub fn App() -> impl IntoView {
    let auth_state = AuthState::new();
    provide_context(auth_state.clone());

    // Provide i18n context with default language
    // The language will be updated when user settings are loaded
    provide_i18n("en".to_string());

    view! {
        <Router>
            <main>
                <Routes>
                    <Route path="/login" view=Login />
                    <Route path="/register" view=Register />
                    <Route path="/" view=AuthenticatedLayout>
                        <Route path="" view=Dashboard />
                        <Route path="/households/:id" view=HouseholdPage />
                        <Route path="/households/:id/tasks" view=TasksPage />
                        <Route path="/households/:id/rewards" view=RewardsPage />
                        <Route path="/households/:id/punishments" view=PunishmentsPage />
                        <Route path="/households/:id/notes" view=NotesPage />
                        <Route path="/households/:id/chat" view=ChatPage />
                        <Route path="/households/:id/activity" view=ActivityPage />
                        <Route path="/households/:id/settings" view=HouseholdSettingsPage />
                        <Route path="/settings" view=SettingsPage />
                        <Route path="/user-settings" view=UserSettingsPage />
                    </Route>
                </Routes>
            </main>
        </Router>
    }
}

#[component]
fn AuthenticatedLayout() -> impl IntoView {
    let auth_state = expect_context::<AuthState>();
    let i18n = use_i18n();

    // Load user settings and update language on authentication
    let auth_state_effect = auth_state.clone();
    create_effect(move |_| {
        if auth_state_effect.is_authenticated() {
            let i18n = i18n.clone();
            wasm_bindgen_futures::spawn_local(async move {
                if let Ok(settings) = ApiClient::get_user_settings().await {
                    i18n.set_language(&settings.language);
                }
            });
        }
    });

    view! {
        <Show
            when=move || auth_state.is_authenticated()
            fallback=|| view! { <RedirectToLogin /> }
        >
            <Navbar />
            <div class="container">
                <Outlet />
            </div>
        </Show>
    }
}

#[component]
fn RedirectToLogin() -> impl IntoView {
    let navigate = use_navigate();
    navigate("/login", Default::default());
    view! {}
}
