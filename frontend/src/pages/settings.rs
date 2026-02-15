use leptos::*;

use crate::api::AuthState;

#[component]
pub fn SettingsPage() -> impl IntoView {
    let auth_state = expect_context::<AuthState>();

    view! {
        <div class="dashboard-header">
            <h1 class="dashboard-title">"Settings"</h1>
        </div>

        <div class="card">
            <h3 class="card-title">"Account Information"</h3>
            {move || {
                auth_state.user.get().map(|user| view! {
                    <div style="margin-top: 1rem;">
                        <div class="form-group">
                            <label class="form-label">"Username"</label>
                            <p>{user.username}</p>
                        </div>
                        <div class="form-group">
                            <label class="form-label">"Email"</label>
                            <p>{user.email}</p>
                        </div>
                    </div>
                })
            }}
        </div>

        <div class="card">
            <h3 class="card-title">"About"</h3>
            <p style="color: var(--text-muted); margin-top: 1rem;">
                "Household Manager - A full-stack Rust application for managing household tasks, rewards, and punishments."
            </p>
        </div>
    }
}
