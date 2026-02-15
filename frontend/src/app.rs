use leptos::*;
use leptos_router::*;

use crate::api::AuthState;
use crate::components::navbar::Navbar;
use crate::pages::{
    activity::ActivityPage, chat::ChatPage, dashboard::Dashboard, household::HouseholdPage,
    household_settings::HouseholdSettingsPage, login::Login, punishments::PunishmentsPage,
    register::Register, rewards::RewardsPage, settings::SettingsPage, tasks::TasksPage,
};

#[component]
pub fn App() -> impl IntoView {
    let auth_state = AuthState::new();
    provide_context(auth_state.clone());

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
                        <Route path="/households/:id/chat" view=ChatPage />
                        <Route path="/households/:id/activity" view=ActivityPage />
                        <Route path="/households/:id/settings" view=HouseholdSettingsPage />
                        <Route path="/settings" view=SettingsPage />
                    </Route>
                </Routes>
            </main>
        </Router>
    }
}

#[component]
fn AuthenticatedLayout() -> impl IntoView {
    let auth_state = expect_context::<AuthState>();

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
