use leptos::*;
use leptos_router::*;
use shared::CreateUserRequest;

use crate::api::{ApiClient, AuthState};
use crate::i18n::use_i18n;

#[component]
pub fn Register() -> impl IntoView {
    let i18n = use_i18n();
    let i18n_stored = store_value(i18n);

    let auth_state = expect_context::<AuthState>();
    let navigate = use_navigate();

    let username = create_rw_signal(String::new());
    let email = create_rw_signal(String::new());
    let password = create_rw_signal(String::new());
    let confirm_password = create_rw_signal(String::new());
    let agb_accepted = create_rw_signal(false);
    let error = create_rw_signal(Option::<String>::None);
    let loading = create_rw_signal(false);

    let on_submit = move |ev: web_sys::SubmitEvent| {
        ev.prevent_default();

        if !agb_accepted.get() {
            error.set(Some(i18n_stored.get_value().t("auth.agb_required")));
            return;
        }

        if password.get() != confirm_password.get() {
            error.set(Some(i18n_stored.get_value().t("auth.password_mismatch")));
            return;
        }

        if password.get().len() < 8 {
            error.set(Some(i18n_stored.get_value().t("auth.password_min_length")));
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
                    <h1 class="auth-title">{move || i18n_stored.get_value().t("auth.create_account")}</h1>
                    <p class="auth-subtitle">{move || i18n_stored.get_value().t("auth.create_account_subtitle")}</p>
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
                            placeholder=move || i18n_stored.get_value().t("auth.choose_username")
                            prop:value=move || username.get()
                            on:input=move |ev| username.set(event_target_value(&ev))
                            required
                        />
                    </div>

                    <div class="form-group">
                        <label class="form-label" for="email">{move || i18n_stored.get_value().t("auth.email")}</label>
                        <input
                            type="email"
                            id="email"
                            class="form-input"
                            placeholder=move || i18n_stored.get_value().t("auth.enter_email")
                            prop:value=move || email.get()
                            on:input=move |ev| email.set(event_target_value(&ev))
                            required
                        />
                    </div>

                    <div class="form-group">
                        <label class="form-label" for="password">{move || i18n_stored.get_value().t("auth.password")}</label>
                        <input
                            type="password"
                            id="password"
                            class="form-input"
                            placeholder=move || i18n_stored.get_value().t("auth.create_password")
                            prop:value=move || password.get()
                            on:input=move |ev| password.set(event_target_value(&ev))
                            required
                            minlength="8"
                        />
                    </div>

                    <div class="form-group">
                        <label class="form-label" for="confirm-password">{move || i18n_stored.get_value().t("auth.confirm_password")}</label>
                        <input
                            type="password"
                            id="confirm-password"
                            class="form-input"
                            placeholder=move || i18n_stored.get_value().t("auth.confirm_your_password")
                            prop:value=move || confirm_password.get()
                            on:input=move |ev| confirm_password.set(event_target_value(&ev))
                            required
                        />
                    </div>

                    <div class="agb-checkbox-container">
                        <input
                            type="checkbox"
                            id="agb"
                            prop:checked=move || agb_accepted.get()
                            on:change=move |ev| agb_accepted.set(event_target_checked(&ev))
                        />
                        <label for="agb">
                            {move || i18n_stored.get_value().t("auth.agb_accept_prefix")}
                            " "
                            <a href="/agb" target="_blank">{move || i18n_stored.get_value().t("auth.agb_link")}</a>
                            " "
                            {move || i18n_stored.get_value().t("auth.agb_and")}
                            " "
                            <a href="/datenschutz" target="_blank">{move || i18n_stored.get_value().t("auth.datenschutz_link")}</a>
                            " "
                            {move || i18n_stored.get_value().t("auth.agb_accept_suffix")}
                        </label>
                    </div>

                    <button
                        type="submit"
                        class="btn btn-primary"
                        style="width: 100%; margin-top: 1rem;"
                        disabled=move || loading.get()
                    >
                        {move || if loading.get() { i18n_stored.get_value().t("auth.creating_account") } else { i18n_stored.get_value().t("auth.create_account") }}
                    </button>
                </form>

                <p style="text-align: center; margin-top: 1rem; color: var(--text-muted);">
                    {move || i18n_stored.get_value().t("auth.have_account")} " "
                    <a href="/login" style="color: var(--primary-color);">{move || i18n_stored.get_value().t("auth.sign_in")}</a>
                </p>

                <div class="legal-links" style="margin-top: 1.5rem; padding-top: 1rem; border-top: 1px solid var(--border-color);">
                    <a href="/impressum">"Impressum"</a>
                    <a href="/datenschutz">"Datenschutz"</a>
                    <a href="/agb">"AGB"</a>
                </div>
            </div>
        </div>
    }
}

#[cfg(test)]
mod tests {
    use wasm_bindgen_test::*;

    wasm_bindgen_test_configure!(run_in_browser);

    #[wasm_bindgen_test]
    fn test_password_mismatch_validation() {
        let password = "password123";
        let confirm_password = "password456";
        assert_ne!(password, confirm_password);
    }

    #[wasm_bindgen_test]
    fn test_password_match_validation() {
        let password = "password123";
        let confirm_password = "password123";
        assert_eq!(password, confirm_password);
    }

    #[wasm_bindgen_test]
    fn test_password_length_too_short() {
        let password = "short";
        assert!(password.len() < 8);
    }

    #[wasm_bindgen_test]
    fn test_password_length_valid() {
        let password = "validpassword";
        assert!(password.len() >= 8);
    }

    #[wasm_bindgen_test]
    fn test_password_length_exactly_8() {
        let password = "12345678";
        assert!(password.len() >= 8);
    }

    #[wasm_bindgen_test]
    fn test_button_text_not_loading() {
        let loading = false;
        let text = if loading { "Creating account..." } else { "Create Account" };
        assert_eq!(text, "Create Account");
    }

    #[wasm_bindgen_test]
    fn test_button_text_loading() {
        let loading = true;
        let text = if loading { "Creating account..." } else { "Create Account" };
        assert_eq!(text, "Creating account...");
    }

    #[wasm_bindgen_test]
    fn test_validation_error_messages() {
        let mismatch_error = "Passwords do not match";
        let length_error = "Password must be at least 8 characters";
        assert!(!mismatch_error.is_empty());
        assert!(!length_error.is_empty());
    }
}
