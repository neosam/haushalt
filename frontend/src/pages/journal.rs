use leptos::*;
use leptos_router::*;
use shared::{HouseholdSettings, JournalEntry, JournalEntryWithUser, User};
use uuid::Uuid;

use crate::api::ApiClient;
use crate::components::journal_entry_card::JournalEntryCard;
use crate::components::journal_modal::JournalModal;
use crate::components::loading::Loading;
use crate::i18n::use_i18n;

#[component]
pub fn JournalPage() -> impl IntoView {
    let i18n = use_i18n();
    let i18n_stored = store_value(i18n);

    let params = use_params_map();
    let household_id = move || params.with(|p| p.get("id").cloned().unwrap_or_default());

    let entries = create_rw_signal(Vec::<JournalEntryWithUser>::new());
    let current_user = create_rw_signal(Option::<User>::None);
    let settings = create_rw_signal(Option::<HouseholdSettings>::None);
    let loading = create_rw_signal(true);
    let error = create_rw_signal(Option::<String>::None);
    let success = create_rw_signal(Option::<String>::None);

    // Filter state
    let show_shared = create_rw_signal(true);
    let show_private = create_rw_signal(true);

    // Modal state: None = closed, Some(None) = create mode, Some(Some(entry)) = edit mode
    let modal_entry = create_rw_signal(Option::<Option<JournalEntry>>::None);

    // Load entries
    create_effect(move |_| {
        let id = household_id();
        if id.is_empty() {
            return;
        }

        let id_for_entries = id.clone();
        let id_for_user = id.clone();
        let id_for_settings = id.clone();

        // Load journal entries
        wasm_bindgen_futures::spawn_local(async move {
            match ApiClient::list_journal_entries(&id_for_entries).await {
                Ok(e) => {
                    entries.set(e);
                    loading.set(false);
                }
                Err(e) => {
                    error.set(Some(e));
                    loading.set(false);
                }
            }
        });

        // Load current user
        wasm_bindgen_futures::spawn_local(async move {
            let _ = id_for_user;
            if let Ok(user) = ApiClient::get_current_user().await {
                current_user.set(Some(user));
            }
        });

        // Load settings for dark mode
        wasm_bindgen_futures::spawn_local(async move {
            if let Ok(s) = ApiClient::get_household_settings(&id_for_settings).await {
                apply_dark_mode(s.dark_mode);
                settings.set(Some(s));
            }
        });
    });

    let reload_entries = move || {
        let id = household_id();
        if id.is_empty() {
            return;
        }
        wasm_bindgen_futures::spawn_local(async move {
            if let Ok(e) = ApiClient::list_journal_entries(&id).await {
                entries.set(e);
            }
        });
    };

    let on_edit = move |entry: JournalEntryWithUser| {
        modal_entry.set(Some(Some(entry.entry)));
    };

    let on_delete = move |entry_id: Uuid| {
        let id = household_id();
        let entry_id_str = entry_id.to_string();
        wasm_bindgen_futures::spawn_local(async move {
            match ApiClient::delete_journal_entry(&id, &entry_id_str).await {
                Ok(()) => {
                    entries.update(|e| e.retain(|entry| entry.entry.id != entry_id));
                    success.set(Some(i18n_stored.get_value().t("journal.deleted")));
                }
                Err(e) => error.set(Some(e)),
            }
        });
    };

    let on_modal_save = move |_saved_entry: JournalEntry| {
        modal_entry.set(None);
        success.set(Some(i18n_stored.get_value().t("journal.saved")));
        // Reload to get the updated entry with user info
        reload_entries();
    };

    let on_modal_close = move |_| {
        modal_entry.set(None);
    };

    let open_create_modal = move |_| {
        modal_entry.set(Some(None));
    };

    // Filtered entries
    let filtered_entries = move || {
        entries
            .get()
            .into_iter()
            .filter(|entry| {
                if entry.entry.is_shared {
                    show_shared.get()
                } else {
                    show_private.get()
                }
            })
            .collect::<Vec<_>>()
    };

    view! {
        <div class="dashboard-header">
            <h1 class="dashboard-title">{i18n_stored.get_value().t("journal.title")}</h1>
            <button class="btn btn-primary" on:click=open_create_modal>
                {i18n_stored.get_value().t("journal.new_entry")}
            </button>
        </div>

        {move || error.get().map(|e| view! {
            <div class="alert alert-error">{e}</div>
        })}

        {move || success.get().map(|s| view! {
            <div class="alert alert-success">{s}
                <button
                    class="alert-dismiss"
                    on:click=move |_| success.set(None)
                >"×"</button>
            </div>
        })}

        // Filter controls
        <div class="filter-controls">
            <label class="filter-checkbox">
                <input
                    type="checkbox"
                    prop:checked=move || show_shared.get()
                    on:change=move |ev| show_shared.set(event_target_checked(&ev))
                />
                <span>{i18n_stored.get_value().t("journal.shared")}</span>
            </label>
            <label class="filter-checkbox">
                <input
                    type="checkbox"
                    prop:checked=move || show_private.get()
                    on:change=move |ev| show_private.set(event_target_checked(&ev))
                />
                <span>{i18n_stored.get_value().t("journal.private")}</span>
            </label>
        </div>

        <Show when=move || loading.get() fallback=|| ()>
            <Loading />
        </Show>

        <Show when=move || !loading.get() fallback=|| ()>
            <div class="notes-list journal-list">
                {move || {
                    let entries_vec = filtered_entries();
                    let user_id = current_user.get().map(|u| u.id).unwrap_or(Uuid::nil());

                    if entries_vec.is_empty() {
                        view! {
                            <div class="empty-state">
                                <p>{i18n_stored.get_value().t("journal.no_entries")}</p>
                                <p class="empty-state-hint">{i18n_stored.get_value().t("journal.first_entry")}</p>
                            </div>
                        }.into_view()
                    } else {
                        entries_vec.into_iter().map(|entry| {
                            view! {
                                <JournalEntryCard
                                    entry=entry
                                    current_user_id=user_id
                                    i18n=i18n_stored
                                    on_edit=Callback::new(on_edit)
                                    on_delete=Callback::new(on_delete)
                                />
                            }
                        }).collect_view()
                    }
                }}
            </div>
        </Show>

        // Modal
        {move || modal_entry.get().map(|entry_opt| {
            view! {
                <JournalModal
                    entry=entry_opt
                    household_id=household_id()
                    i18n=i18n_stored
                    on_close=Callback::new(on_modal_close)
                    on_save=Callback::new(on_modal_save)
                />
            }
        })}
    }
}

/// Apply dark mode class to document body
fn apply_dark_mode(enabled: bool) {
    if let Some(window) = web_sys::window() {
        if let Some(document) = window.document() {
            if let Some(body) = document.body() {
                if enabled {
                    let _ = body.class_list().add_1("dark-mode");
                } else {
                    let _ = body.class_list().remove_1("dark-mode");
                }
            }
        }
    }
}
