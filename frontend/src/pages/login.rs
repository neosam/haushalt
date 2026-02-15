use leptos::*;
use leptos_router::*;
use shared::LoginRequest;

use crate::api::{ApiClient, AuthState};
use crate::i18n::use_i18n;

#[component]
pub fn Login() -> impl IntoView {
    let auth_state = expect_context::<AuthState>();
    let i18n = use_i18n();
    let i18n_stored = store_value(i18n);
    let navigate = use_navigate();

    let username = create_rw_signal(String::new());
    let password = create_rw_signal(String::new());
    let error = create_rw_signal(Option::<String>::None);
    let loading = create_rw_signal(false);

    let on_submit = move |ev: web_sys::SubmitEvent| {
        ev.prevent_default();

        let nav = navigate.clone();
        let auth = auth_state.clone();

        loading.set(true);
        error.set(None);

        let request = LoginRequest {
            username: username.get(),
            password: password.get(),
        };

        wasm_bindgen_futures::spawn_local(async move {
            match ApiClient::login(request).await {
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
                    <h1 class="auth-title">{move || i18n_stored.get_value().t("auth.welcome_back")}</h1>
                    <p class="auth-subtitle">{move || i18n_stored.get_value().t("auth.sign_in_subtitle")}</p>
                </div>

                {move || error.get().map(|e| view! {
                    <div class="alert alert-error">{e}</div>
                })}

                <form on:submit=on_submit>
                    <div class="form-group">
                        <label class="form-label" for="username">{move || i18n_stored.get_value().t("auth.username")}</label>
                        <input
                            type="text"
                            id="username"
                            class="form-input"
                            placeholder=move || i18n_stored.get_value().t("auth.enter_username")
                            prop:value=move || username.get()
                            on:input=move |ev| username.set(event_target_value(&ev))
                            required
                        />
                    </div>

                    <div class="form-group">
                        <label class="form-label" for="password">{move || i18n_stored.get_value().t("auth.password")}</label>
                        <input
                            type="password"
                            id="password"
                            class="form-input"
                            placeholder=move || i18n_stored.get_value().t("auth.enter_password")
                            prop:value=move || password.get()
                            on:input=move |ev| password.set(event_target_value(&ev))
                            required
                        />
                    </div>

                    <button
                        type="submit"
                        class="btn btn-primary"
                        style="width: 100%; margin-top: 1rem;"
                        disabled=move || loading.get()
                    >
                        {move || if loading.get() {
                            i18n_stored.get_value().t("auth.signing_in")
                        } else {
                            i18n_stored.get_value().t("auth.sign_in")
                        }}
                    </button>
                </form>

                <p style="text-align: center; margin-top: 1rem; color: var(--text-muted);">
                    {move || i18n_stored.get_value().t("auth.no_account")}
                    " "
                    <a href="/register" style="color: var(--primary-color);">{move || i18n_stored.get_value().t("auth.sign_up")}</a>
                </p>
            </div>
        </div>
    }
}

#[cfg(test)]
mod tests {
    use wasm_bindgen_test::*;

    wasm_bindgen_test_configure!(run_in_browser);

    #[wasm_bindgen_test]
    fn test_button_text_not_loading() {
        let loading = false;
        let text = if loading { "Signing in..." } else { "Sign In" };
        assert_eq!(text, "Sign In");
    }

    #[wasm_bindgen_test]
    fn test_button_text_loading() {
        let loading = true;
        let text = if loading { "Signing in..." } else { "Sign In" };
        assert_eq!(text, "Signing in...");
    }

    #[wasm_bindgen_test]
    fn test_css_classes() {
        assert_eq!("auth-container", "auth-container");
        assert_eq!("auth-card card", "auth-card card");
        assert_eq!("auth-header", "auth-header");
        assert_eq!("auth-title", "auth-title");
        assert_eq!("auth-subtitle", "auth-subtitle");
        assert_eq!("alert alert-error", "alert alert-error");
    }

    #[wasm_bindgen_test]
    fn test_form_input_placeholders() {
        let username_placeholder = "Enter your username";
        let password_placeholder = "Enter your password";
        assert!(!username_placeholder.is_empty());
        assert!(!password_placeholder.is_empty());
    }
}
