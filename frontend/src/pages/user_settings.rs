use leptos::*;
use shared::UpdateUserSettingsRequest;

use crate::api::ApiClient;
use crate::components::loading::Loading;
use crate::i18n::{supported_languages, use_i18n};

#[component]
pub fn UserSettingsPage() -> impl IntoView {
    let i18n = use_i18n();

    let loading = create_rw_signal(true);
    let saving = create_rw_signal(false);
    let error = create_rw_signal(Option::<String>::None);
    let success = create_rw_signal(Option::<String>::None);
    let selected_language = create_rw_signal(String::new());

    // Load user settings
    create_effect(move |_| {
        wasm_bindgen_futures::spawn_local(async move {
            match ApiClient::get_user_settings().await {
                Ok(settings) => {
                    selected_language.set(settings.language);
                    loading.set(false);
                }
                Err(e) => {
                    error.set(Some(e));
                    loading.set(false);
                }
            }
        });
    });

    // Store i18n context for use in closures
    let i18n_stored = store_value(i18n);

    view! {
        <div class="dashboard-header">
            <h1 class="dashboard-title">{move || i18n_stored.get_value().t("settings.user_settings")}</h1>
        </div>

        {move || error.get().map(|e| view! {
            <div class="alert alert-error">{e}
                <button class="alert-dismiss" on:click=move |_| error.set(None)>"×"</button>
            </div>
        })}

        {move || success.get().map(|s| view! {
            <div class="alert alert-success">{s}
                <button class="alert-dismiss" on:click=move |_| success.set(None)>"×"</button>
            </div>
        })}

        <Show when=move || loading.get() fallback=|| ()>
            <Loading />
        </Show>

        <Show when=move || !loading.get() fallback=|| ()>
            <div class="card">
                <form on:submit=move |ev: web_sys::SubmitEvent| {
                    ev.prevent_default();
                    saving.set(true);
                    error.set(None);
                    success.set(None);

                    let language = selected_language.get();
                    let i18n_clone = i18n_stored.get_value();

                    wasm_bindgen_futures::spawn_local(async move {
                        let request = UpdateUserSettingsRequest {
                            language: Some(language.clone()),
                        };

                        match ApiClient::update_user_settings(request).await {
                            Ok(settings) => {
                                // Update i18n context with the new language
                                i18n_clone.set_language(&settings.language);
                                success.set(Some(i18n_clone.t("settings.saved")));
                                saving.set(false);
                            }
                            Err(e) => {
                                error.set(Some(e));
                                saving.set(false);
                            }
                        }
                    });
                }>
                    <div class="card-header">
                        <h3 class="card-title">{move || i18n_stored.get_value().t("settings.language")}</h3>
                    </div>
                    <div style="padding: 1rem;">
                        <div class="form-group">
                            <label class="form-label" for="language">{move || i18n_stored.get_value().t("settings.language")}</label>
                            <select
                                id="language"
                                class="form-input"
                                prop:value=move || selected_language.get()
                                on:change=move |ev| selected_language.set(event_target_value(&ev))
                            >
                                {supported_languages().into_iter().map(|(code, name)| {
                                    let code_value = code.to_string();
                                    let code_selected = code.to_string();
                                    view! {
                                        <option
                                            value=code_value
                                            selected=move || selected_language.get() == code_selected
                                        >
                                            {name}
                                        </option>
                                    }
                                }).collect_view()}
                            </select>
                        </div>
                    </div>
                    <div class="card-footer" style="padding: 1rem; border-top: 1px solid var(--border-color);">
                        <button
                            type="submit"
                            class="btn btn-primary"
                            disabled=move || saving.get()
                        >
                            {move || if saving.get() {
                                i18n_stored.get_value().t("common.saving")
                            } else {
                                i18n_stored.get_value().t("common.save")
                            }}
                        </button>
                    </div>
                </form>
            </div>
        </Show>
    }
}
