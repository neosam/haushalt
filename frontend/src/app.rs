use leptos::*;
use leptos_router::*;

use crate::api::{ApiClient, AuthState};
use crate::components::navbar::Navbar;
use crate::i18n::{provide_i18n, use_i18n};
use crate::pages::{
    activity::ActivityPage, chat::ChatPage, dashboard::Dashboard, household::HouseholdPage,
    household_settings::HouseholdSettingsPage, journal::JournalPage, login::Login, notes::NotesPage,
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
                        <Route path="/households/:id/journal" view=JournalPage />
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

    // Check for auth failure on each render
    let auth_state_check = auth_state.clone();
    create_effect(move |_| {
        // This will clear the auth state if a refresh failure occurred
        auth_state_check.check_and_clear_auth_failed();
    });

    // Load user settings and update language on authentication
    let auth_state_effect = auth_state.clone();
    create_effect(move |_| {
        if auth_state_effect.is_authenticated() {
            let i18n = i18n.clone();
            let auth_state_for_error = auth_state_effect.clone();
            wasm_bindgen_futures::spawn_local(async move {
                match ApiClient::get_user_settings().await {
                    Ok(settings) => {
                        i18n.set_language(&settings.language);
                    }
                    Err(_) => {
                        // Check if auth failed during the request
                        auth_state_for_error.check_and_clear_auth_failed();
                    }
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
