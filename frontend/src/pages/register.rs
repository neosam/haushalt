use leptos::*;
use leptos_router::*;
use shared::CreateUserRequest;

use crate::api::{ApiClient, AuthState};

#[component]
pub fn Register() -> impl IntoView {
    let auth_state = expect_context::<AuthState>();
    let navigate = use_navigate();

    let username = create_rw_signal(String::new());
    let email = create_rw_signal(String::new());
    let password = create_rw_signal(String::new());
    let confirm_password = create_rw_signal(String::new());
    let error = create_rw_signal(Option::<String>::None);
    let loading = create_rw_signal(false);

    let on_submit = move |ev: web_sys::SubmitEvent| {
        ev.prevent_default();

        if password.get() != confirm_password.get() {
            error.set(Some("Passwords do not match".to_string()));
            return;
        }

        if password.get().len() < 8 {
            error.set(Some("Password must be at least 8 characters".to_string()));
            return;
        }

        let nav = navigate.clone();
        let auth = auth_state.clone();

        loading.set(true);
        error.set(None);

        let request = CreateUserRequest {
            username: username.get(),
            email: email.get(),
            password: password.get(),
        };

        wasm_bindgen_futures::spawn_local(async move {
            match ApiClient::register(request).await {
                Ok(response) => {
                    auth.set_auth(response);
                    nav("/", Default::default());
                }
                Err(e) => {
                    error.set(Some(e));
                    loading.set(false);
                }
            }
        });
    };

    view! {
        <div class="auth-container">
            <div class="auth-card card">
                <div class="auth-header">
                    <h1 class="auth-title">"Create Account"</h1>
                    <p class="auth-subtitle">"Sign up to get started"</p>
                </div>

                {move || error.get().map(|e| view! {
                    <div class="alert alert-error">{e}</div>
                })}

                <form on:submit=on_submit>
                    <div class="form-group">
                        <label class="form-label" for="username">"Username"</label>
                        <input
                            type="text"
                            id="username"
                            class="form-input"
                            placeholder="Choose a username"
                            prop:value=move || username.get()
                            on:input=move |ev| username.set(event_target_value(&ev))
                            required
                        />
                    </div>

                    <div class="form-group">
                        <label class="form-label" for="email">"Email"</label>
                        <input
                            type="email"
                            id="email"
                            class="form-input"
                            placeholder="Enter your email"
                            prop:value=move || email.get()
                            on:input=move |ev| email.set(event_target_value(&ev))
                            required
                        />
                    </div>

                    <div class="form-group">
                        <label class="form-label" for="password">"Password"</label>
                        <input
                            type="password"
                            id="password"
                            class="form-input"
                            placeholder="Create a password (min 8 characters)"
                            prop:value=move || password.get()
                            on:input=move |ev| password.set(event_target_value(&ev))
                            required
                            minlength="8"
                        />
                    </div>

                    <div class="form-group">
                        <label class="form-label" for="confirm-password">"Confirm Password"</label>
                        <input
                            type="password"
                            id="confirm-password"
                            class="form-input"
                            placeholder="Confirm your password"
                            prop:value=move || confirm_password.get()
                            on:input=move |ev| confirm_password.set(event_target_value(&ev))
                            required
                        />
                    </div>

                    <button
                        type="submit"
                        class="btn btn-primary"
                        style="width: 100%; margin-top: 1rem;"
                        disabled=move || loading.get()
                    >
                        {move || if loading.get() { "Creating account..." } else { "Create Account" }}
                    </button>
                </form>

                <p style="text-align: center; margin-top: 1rem; color: var(--text-muted);">
                    "Already have an account? "
                    <a href="/login" style="color: var(--primary-color);">"Sign in"</a>
                </p>
            </div>
        </div>
    }
}
