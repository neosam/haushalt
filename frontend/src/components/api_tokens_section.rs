//! Programmatic API tokens, managed in the user settings.
//!
//! A token lets an external system call the household API on the creator's behalf, bound to
//! one household and a read/write permission. The plaintext secret is shown exactly ONCE,
//! right after creation — the section makes that moment prominent and treats the value as a
//! secret (copy button, warning, no persistence). Everything else edits a token in place.

use leptos::*;
use shared::{ApiToken, CreateApiTokenRequest, Household, UpdateApiTokenRequest};
use uuid::Uuid;

use crate::api::ApiClient;
use crate::components::loading::Loading;
use crate::components::{Button, ButtonSize, ButtonVariant};
use crate::i18n::use_i18n;
use crate::utils::copy_to_clipboard;

/// How long the copy button flashes its "copied" label, in milliseconds.
const COPIED_FLASH_MS: u32 = 2000;

/// The household name for an id, for display next to a token whose binding is immutable.
fn household_name(households: &[Household], household_id: Uuid) -> Option<String> {
    households
        .iter()
        .find(|h| h.id == household_id)
        .map(|h| h.name.clone())
}

#[cfg(all(test, not(target_arch = "wasm32")))]
mod tests {
    use super::*;
    use chrono::Utc;

    fn household(id: Uuid, name: &str) -> Household {
        Household {
            id,
            name: name.to_string(),
            owner_id: Uuid::new_v4(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }

    #[test]
    fn test_household_name_resolves_and_falls_back() {
        let a = Uuid::new_v4();
        let b = Uuid::new_v4();
        let households = [household(a, "Kitchen"), household(b, "Studio")];

        assert_eq!(household_name(&households, b).as_deref(), Some("Studio"));
        // An id not in the list (e.g. a household the user has since left) resolves to None,
        // so the caller can fall back to showing the raw id rather than an empty label.
        assert_eq!(household_name(&households, Uuid::new_v4()), None);
    }
}

#[component]
pub fn ApiTokensSection() -> impl IntoView {
    let i18n = use_i18n();
    let i18n_stored = store_value(i18n);
    let t = move |key: &str| i18n_stored.get_value().t(key);

    let tokens = create_rw_signal(Vec::<ApiToken>::new());
    let households = create_rw_signal(Vec::<Household>::new());
    let loading = create_rw_signal(true);
    let busy = create_rw_signal(false);
    let error = create_rw_signal(Option::<String>::None);
    let success = create_rw_signal(Option::<String>::None);

    let new_name = create_rw_signal(String::new());
    let new_household = create_rw_signal(Option::<Uuid>::None);
    let new_can_write = create_rw_signal(false);

    // The plaintext secret of the token just created — the only time it ever exists here.
    let created_secret = create_rw_signal(Option::<String>::None);
    let secret_copied = create_rw_signal(false);
    let confirm_delete = create_rw_signal(Option::<Uuid>::None);

    create_effect(move |_| {
        wasm_bindgen_futures::spawn_local(async move {
            match ApiClient::list_api_tokens().await {
                Ok(loaded) => tokens.set(loaded),
                Err(e) => error.set(Some(e)),
            }
            if let Ok(loaded) = ApiClient::list_households().await {
                // Default the create form to the first household so a token can be made in
                // one click when the user belongs to exactly one.
                if new_household.get_untracked().is_none() {
                    new_household.set(loaded.first().map(|h| h.id));
                }
                households.set(loaded);
            }
            loading.set(false);
        });
    });

    // Replace one token in the list, keeping its position.
    let replace = move |updated: ApiToken| {
        tokens.update(|list| {
            if let Some(slot) = list.iter_mut().find(|token| token.id == updated.id) {
                *slot = updated;
            }
        });
    };

    // The single write path every field edit funnels through.
    let apply = move |token_id: Uuid, request: UpdateApiTokenRequest, message: String| {
        busy.set(true);
        error.set(None);
        success.set(None);
        wasm_bindgen_futures::spawn_local(async move {
            match ApiClient::update_api_token(&token_id.to_string(), request).await {
                Ok(updated) => {
                    replace(updated);
                    success.set(Some(message));
                }
                Err(e) => error.set(Some(e)),
            }
            busy.set(false);
        });
    };

    let create = move |_| {
        let name = new_name.get_untracked().trim().to_string();
        if name.is_empty() {
            error.set(Some(t("api_tokens.name_required")));
            return;
        }
        let Some(household_id) = new_household.get_untracked() else {
            error.set(Some(t("api_tokens.no_households")));
            return;
        };

        busy.set(true);
        error.set(None);
        success.set(None);
        created_secret.set(None);
        let can_write = new_can_write.get_untracked();
        let created_message = t("api_tokens.created");
        wasm_bindgen_futures::spawn_local(async move {
            let request = CreateApiTokenRequest {
                household_id,
                name,
                can_write: Some(can_write),
            };
            match ApiClient::create_api_token(request).await {
                Ok(created) => {
                    tokens.update(|list| list.insert(0, created.token));
                    new_name.set(String::new());
                    new_can_write.set(false);
                    // Surface the plaintext once — the user must copy it now.
                    secret_copied.set(false);
                    created_secret.set(Some(created.secret));
                    success.set(Some(created_message));
                }
                Err(e) => error.set(Some(e)),
            }
            busy.set(false);
        });
    };

    let copy_secret = move |_| {
        let Some(secret) = created_secret.get_untracked() else {
            return;
        };
        wasm_bindgen_futures::spawn_local(async move {
            match copy_to_clipboard(&secret).await {
                Ok(()) => {
                    secret_copied.set(true);
                    gloo_timers::future::TimeoutFuture::new(COPIED_FLASH_MS).await;
                    secret_copied.set(false);
                }
                Err(e) => error.set(Some(e)),
            }
        });
    };

    let delete = move |token_id: Uuid| {
        busy.set(true);
        error.set(None);
        success.set(None);
        confirm_delete.set(None);
        let message = t("api_tokens.deleted");
        wasm_bindgen_futures::spawn_local(async move {
            match ApiClient::delete_api_token(&token_id.to_string()).await {
                Ok(()) => {
                    tokens.update(|list| list.retain(|token| token.id != token_id));
                    success.set(Some(message));
                }
                Err(e) => error.set(Some(e)),
            }
            busy.set(false);
        });
    };

    view! {
        <div class="card">
            <div class="card-header">
                <h3 class="card-title">{move || t("api_tokens.title")}</h3>
            </div>

            <div class="api-tokens">
                <p class="form-hint">{move || t("api_tokens.hint")}</p>

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

                // The one-time secret. Shown only right after a create, treated as sensitive.
                {move || created_secret.get().map(|secret| view! {
                    <div class="alert alert-success api-token-secret">
                        <p class="api-token-secret-warning">{move || t("api_tokens.secret_warning")}</p>
                        <div class="api-token-secret-row">
                            <code class="api-token-secret-value">{secret.clone()}</code>
                            <Button
                                variant=ButtonVariant::Outline
                                size=ButtonSize::Small
                                on_click=Callback::new(copy_secret)
                            >
                                {move || if secret_copied.get() {
                                    t("report.copied")
                                } else {
                                    t("report.copy_button")
                                }}
                            </Button>
                        </div>
                        <button class="alert-dismiss" on:click=move |_| created_secret.set(None)>"×"</button>
                    </div>
                })}

                <Show when=move || loading.get() fallback=|| ()>
                    <Loading />
                </Show>

                <Show when=move || !loading.get() fallback=|| ()>
                    <Show
                        when=move || !households.get().is_empty()
                        fallback=move || view! {
                            <p class="form-hint">{move || t("api_tokens.no_households")}</p>
                        }
                    >
                        <div class="api-token-create">
                            <input
                                type="text"
                                class="form-input"
                                placeholder=move || t("api_tokens.name_placeholder")
                                prop:value=move || new_name.get()
                                on:input=move |ev| new_name.set(event_target_value(&ev))
                            />
                            <select
                                class="form-input api-token-household"
                                on:change=move |ev| {
                                    new_household.set(Uuid::parse_str(&event_target_value(&ev)).ok());
                                }
                            >
                                {move || households.get().into_iter().map(|household| {
                                    let selected = new_household.get() == Some(household.id);
                                    view! {
                                        <option value=household.id.to_string() selected=selected>
                                            {household.name}
                                        </option>
                                    }
                                }).collect_view()}
                            </select>
                            <label class="api-token-toggle">
                                <input
                                    type="checkbox"
                                    prop:checked=move || new_can_write.get()
                                    on:change=move |ev| new_can_write.set(event_target_checked(&ev))
                                />
                                <span>{move || t("api_tokens.can_write")}</span>
                            </label>
                            <Button
                                variant=ButtonVariant::Primary
                                disabled=Signal::derive(move || busy.get())
                                on_click=Callback::new(create)
                            >
                                {move || t("api_tokens.create")}
                            </Button>
                        </div>
                    </Show>

                    <Show
                        when=move || !tokens.get().is_empty()
                        fallback=move || view! {
                            <p class="form-hint">{move || t("api_tokens.empty")}</p>
                        }
                    >
                        <For
                            each=move || tokens.get()
                            key=|token| (
                                token.id,
                                token.enabled,
                                token.can_write,
                                token.name.clone(),
                                token.household_id,
                            )
                            children=move |token| {
                                let token_id = token.id;
                                let prefix = token.token_prefix.clone();
                                let household_label = household_name(&households.get(), token.household_id)
                                    .unwrap_or_else(|| token.household_id.to_string());
                                let last_used = token.last_used_at
                                    .map(|dt| dt.format("%Y-%m-%d %H:%M UTC").to_string())
                                    .unwrap_or_else(|| t("api_tokens.never_used"));

                                view! {
                                    <div class="api-token">
                                        <div class="api-token-row">
                                            <input
                                                type="text"
                                                class="form-input"
                                                prop:value=token.name.clone()
                                                on:change=move |ev| {
                                                    let name = event_target_value(&ev).trim().to_string();
                                                    apply(
                                                        token_id,
                                                        UpdateApiTokenRequest {
                                                            name: Some(name),
                                                            ..Default::default()
                                                        },
                                                        t("api_tokens.saved"),
                                                    );
                                                }
                                            />
                                            <code class="api-token-prefix">{prefix}</code>
                                        </div>

                                        <p class="form-hint api-token-meta">
                                            <span class="api-token-household-label">{household_label}</span>
                                            " · "
                                            <span>{move || t("api_tokens.last_used")}": "{last_used.clone()}</span>
                                        </p>

                                        <label class="api-token-toggle">
                                            <input
                                                type="checkbox"
                                                prop:checked=token.can_write
                                                on:change=move |ev| {
                                                    apply(
                                                        token_id,
                                                        UpdateApiTokenRequest {
                                                            can_write: Some(event_target_checked(&ev)),
                                                            ..Default::default()
                                                        },
                                                        t("api_tokens.saved"),
                                                    );
                                                }
                                            />
                                            <span>{move || t("api_tokens.can_write")}</span>
                                        </label>

                                        <label class="api-token-toggle">
                                            <input
                                                type="checkbox"
                                                prop:checked=token.enabled
                                                on:change=move |ev| {
                                                    apply(
                                                        token_id,
                                                        UpdateApiTokenRequest {
                                                            enabled: Some(event_target_checked(&ev)),
                                                            ..Default::default()
                                                        },
                                                        t("api_tokens.saved"),
                                                    );
                                                }
                                            />
                                            <span>{move || t("api_tokens.enabled")}</span>
                                        </label>

                                        <div class="api-token-actions">
                                            <Show
                                                when=move || confirm_delete.get() == Some(token_id)
                                                fallback=move || view! {
                                                    <Button
                                                        variant=ButtonVariant::Danger
                                                        size=ButtonSize::Small
                                                        disabled=Signal::derive(move || busy.get())
                                                        on_click=Callback::new(move |_| confirm_delete.set(Some(token_id)))
                                                    >
                                                        {move || t("common.delete")}
                                                    </Button>
                                                }
                                            >
                                                <span class="form-hint">{move || t("api_tokens.delete_confirm")}</span>
                                                <Button
                                                    variant=ButtonVariant::Danger
                                                    size=ButtonSize::Small
                                                    disabled=Signal::derive(move || busy.get())
                                                    on_click=Callback::new(move |_| delete(token_id))
                                                >
                                                    {move || t("common.delete")}
                                                </Button>
                                                <Button
                                                    variant=ButtonVariant::Secondary
                                                    size=ButtonSize::Small
                                                    on_click=Callback::new(move |_| confirm_delete.set(None))
                                                >
                                                    {move || t("common.cancel")}
                                                </Button>
                                            </Show>
                                        </div>
                                    </div>
                                }
                            }
                        />
                    </Show>
                </Show>
            </div>
        </div>
    }
}
