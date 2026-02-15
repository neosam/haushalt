use leptos::*;
use leptos_router::*;
use shared::{HouseholdSettings, Note, NoteWithUser, User};
use uuid::Uuid;

use crate::api::ApiClient;
use crate::components::household_tabs::{HouseholdTab, HouseholdTabs};
use crate::components::loading::Loading;
use crate::components::note_card::NoteCard;
use crate::components::note_modal::NoteModal;
use crate::i18n::use_i18n;

#[component]
pub fn NotesPage() -> impl IntoView {
    let i18n = use_i18n();
    let i18n_stored = store_value(i18n);

    let params = use_params_map();
    let household_id = move || params.with(|p| p.get("id").cloned().unwrap_or_default());

    let notes = create_rw_signal(Vec::<NoteWithUser>::new());
    let current_user = create_rw_signal(Option::<User>::None);
    let settings = create_rw_signal(Option::<HouseholdSettings>::None);
    let loading = create_rw_signal(true);
    let error = create_rw_signal(Option::<String>::None);
    let success = create_rw_signal(Option::<String>::None);

    // Filter state
    let show_shared = create_rw_signal(true);
    let show_private = create_rw_signal(true);

    // Modal state: None = closed, Some(None) = create mode, Some(Some(note)) = edit mode
    let modal_note = create_rw_signal(Option::<Option<Note>>::None);

    // Load notes
    create_effect(move |_| {
        let id = household_id();
        if id.is_empty() {
            return;
        }

        let id_for_notes = id.clone();
        let id_for_user = id.clone();
        let id_for_settings = id.clone();

        // Load notes
        wasm_bindgen_futures::spawn_local(async move {
            match ApiClient::list_notes(&id_for_notes).await {
                Ok(n) => {
                    notes.set(n);
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

    let reload_notes = move || {
        let id = household_id();
        if id.is_empty() {
            return;
        }
        wasm_bindgen_futures::spawn_local(async move {
            if let Ok(n) = ApiClient::list_notes(&id).await {
                notes.set(n);
            }
        });
    };

    let on_edit = move |note: NoteWithUser| {
        modal_note.set(Some(Some(note.note)));
    };

    let on_delete = move |note_id: Uuid| {
        let id = household_id();
        let note_id_str = note_id.to_string();
        wasm_bindgen_futures::spawn_local(async move {
            match ApiClient::delete_note(&id, &note_id_str).await {
                Ok(()) => {
                    notes.update(|n| n.retain(|note| note.note.id != note_id));
                    success.set(Some(i18n_stored.get_value().t("notes.deleted")));
                }
                Err(e) => error.set(Some(e)),
            }
        });
    };

    let on_modal_save = move |_saved_note: Note| {
        modal_note.set(None);
        success.set(Some(i18n_stored.get_value().t("notes.saved")));
        // Reload to get the updated note with user info
        reload_notes();
    };

    let on_modal_close = move |_| {
        modal_note.set(None);
    };

    let open_create_modal = move |_| {
        modal_note.set(Some(None));
    };

    // Filtered notes
    let filtered_notes = move || {
        notes
            .get()
            .into_iter()
            .filter(|note| {
                if note.note.is_shared {
                    show_shared.get()
                } else {
                    show_private.get()
                }
            })
            .collect::<Vec<_>>()
    };

    let hid = household_id();

    view! {
        {
            let hid = hid.clone();
            move || view! { <HouseholdTabs household_id=hid.clone() active_tab=HouseholdTab::Notes settings=settings.get() /> }
        }

        <div class="dashboard-header">
            <h1 class="dashboard-title">{i18n_stored.get_value().t("notes.title")}</h1>
            <button class="btn btn-primary" on:click=open_create_modal>
                {i18n_stored.get_value().t("notes.new_note")}
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
                <span>{i18n_stored.get_value().t("notes.shared")}</span>
            </label>
            <label class="filter-checkbox">
                <input
                    type="checkbox"
                    prop:checked=move || show_private.get()
                    on:change=move |ev| show_private.set(event_target_checked(&ev))
                />
                <span>{i18n_stored.get_value().t("notes.private")}</span>
            </label>
        </div>

        <Show when=move || loading.get() fallback=|| ()>
            <Loading />
        </Show>

        <Show when=move || !loading.get() fallback=|| ()>
            <div class="notes-list">
                {move || {
                    let notes_vec = filtered_notes();
                    let user_id = current_user.get().map(|u| u.id).unwrap_or(Uuid::nil());

                    if notes_vec.is_empty() {
                        view! {
                            <div class="empty-state">
                                <p>{i18n_stored.get_value().t("notes.first_note")}</p>
                            </div>
                        }.into_view()
                    } else {
                        notes_vec.into_iter().map(|note| {
                            let on_edit = on_edit.clone();
                            view! {
                                <NoteCard
                                    note=note
                                    current_user_id=user_id
                                    on_edit=Callback::new(move |n| on_edit(n))
                                    on_delete=Callback::new(move |id| on_delete(id))
                                />
                            }
                        }).collect_view()
                    }
                }}
            </div>
        </Show>

        // Modal
        {move || modal_note.get().map(|note_opt| {
            view! {
                <NoteModal
                    note=note_opt
                    household_id=household_id()
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
