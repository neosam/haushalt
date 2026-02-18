use chrono::NaiveDate;
use leptos::*;
use shared::{CreateJournalEntryRequest, JournalEntry, UpdateJournalEntryRequest};

use crate::api::ApiClient;
use crate::components::markdown::MarkdownViewReactive;
use crate::i18n::I18nContext;

#[component]
pub fn JournalModal(
    entry: Option<JournalEntry>,
    household_id: String,
    i18n: StoredValue<I18nContext>,
    #[prop(into)] on_close: Callback<()>,
    #[prop(into)] on_save: Callback<JournalEntry>,
) -> impl IntoView {
    let is_edit = entry.is_some();
    let error = create_rw_signal(Option::<String>::None);
    let saving = create_rw_signal(false);
    let preview_mode = create_rw_signal(false);

    // Get today's date for default
    let today = chrono::Utc::now().date_naive();
    let today_str = today.format("%Y-%m-%d").to_string();

    // Form fields - initialize based on mode
    let title = create_rw_signal(entry.as_ref().map(|e| e.title.clone()).unwrap_or_default());
    let content = create_rw_signal(entry.as_ref().map(|e| e.content.clone()).unwrap_or_default());
    let entry_date = create_rw_signal(
        entry
            .as_ref()
            .map(|e| e.entry_date.format("%Y-%m-%d").to_string())
            .unwrap_or_else(|| today_str.clone()),
    );
    let is_shared = create_rw_signal(entry.as_ref().map(|e| e.is_shared).unwrap_or(false));

    let entry_id = entry.as_ref().map(|e| e.id.to_string());

    let on_submit = {
        let entry_id = entry_id.clone();
        let household_id = household_id.clone();

        move |ev: web_sys::SubmitEvent| {
            ev.prevent_default();

            if content.get().trim().is_empty() {
                error.set(Some(i18n.get_value().t("journal.content_required")));
                return;
            }

            // Parse the date
            let parsed_date = match NaiveDate::parse_from_str(&entry_date.get(), "%Y-%m-%d") {
                Ok(d) => d,
                Err(_) => {
                    error.set(Some(i18n.get_value().t("journal.invalid_date")));
                    return;
                }
            };

            saving.set(true);
            error.set(None);

            let entry_id = entry_id.clone();
            let household_id = household_id.clone();

            wasm_bindgen_futures::spawn_local(async move {
                if let Some(entry_id) = entry_id {
                    // Edit mode - update existing entry
                    let request = UpdateJournalEntryRequest {
                        title: Some(title.get()),
                        content: Some(content.get()),
                        entry_date: Some(parsed_date),
                        is_shared: Some(is_shared.get()),
                    };

                    match ApiClient::update_journal_entry(&household_id, &entry_id, request).await {
                        Ok(updated_entry) => {
                            saving.set(false);
                            on_save.call(updated_entry);
                        }
                        Err(e) => {
                            error.set(Some(e));
                            saving.set(false);
                        }
                    }
                } else {
                    // Create mode - create new entry
                    let title_value = title.get();
                    let request = CreateJournalEntryRequest {
                        title: if title_value.is_empty() {
                            None
                        } else {
                            Some(title_value)
                        },
                        content: content.get(),
                        entry_date: Some(parsed_date),
                        is_shared: is_shared.get(),
                    };

                    match ApiClient::create_journal_entry(&household_id, request).await {
                        Ok(created_entry) => {
                            saving.set(false);
                            on_save.call(created_entry);
                        }
                        Err(e) => {
                            error.set(Some(e));
                            saving.set(false);
                        }
                    }
                }
            });
        }
    };

    let close = move |_| on_close.call(());

    let toggle_preview = move |_: web_sys::MouseEvent| {
        preview_mode.update(|v| *v = !*v);
    };

    let modal_title = if is_edit {
        i18n.get_value().t("journal.edit_entry")
    } else {
        i18n.get_value().t("journal.new_entry")
    };

    let submit_button_text = i18n.get_value().t("common.save");
    let saving_text = i18n.get_value().t("common.saving");
    let cancel_text = i18n.get_value().t("common.cancel");
    let title_label = i18n.get_value().t("journal.entry_title");
    let content_label = i18n.get_value().t("journal.entry_content");
    let date_label = i18n.get_value().t("journal.entry_date");
    let share_label = i18n.get_value().t("journal.share_with_household");
    let preview_text = i18n.get_value().t("common.preview");
    let edit_text = i18n.get_value().t("common.edit");

    let content_signal = Signal::derive(move || content.get());

    view! {
        <div class="modal-backdrop" on:click=close>
            <div class="modal modal-large" on:click=|e| e.stop_propagation()>
                <div class="modal-header">
                    <h3 class="modal-title">{modal_title}</h3>
                    <button class="modal-close" on:click=close>"×"</button>
                </div>

                {move || error.get().map(|e| view! {
                    <div class="alert alert-error" style="margin: 1rem;">{e}</div>
                })}

                <form on:submit=on_submit>
                    <div style="padding: 1rem;">
                        <div class="form-group">
                            <label class="form-label" for="journal-title">{title_label.clone()}</label>
                            <input
                                type="text"
                                id="journal-title"
                                class="form-input"
                                placeholder=""
                                prop:value=move || title.get()
                                on:input=move |ev| title.set(event_target_value(&ev))
                            />
                        </div>

                        <div class="form-group">
                            <label class="form-label" for="journal-date">{date_label.clone()}</label>
                            <input
                                type="date"
                                id="journal-date"
                                class="form-input"
                                prop:value=move || entry_date.get()
                                on:input=move |ev| entry_date.set(event_target_value(&ev))
                                required
                            />
                        </div>

                        <div class="form-group">
                            <div style="display: flex; justify-content: space-between; align-items: center; margin-bottom: 0.5rem;">
                                <label class="form-label" for="journal-content" style="margin-bottom: 0;">{content_label.clone()}</label>
                                <button
                                    type="button"
                                    class="btn btn-outline btn-sm"
                                    on:click=toggle_preview
                                >
                                    {move || if preview_mode.get() { edit_text.clone() } else { preview_text.clone() }}
                                </button>
                            </div>

                            <Show
                                when=move || !preview_mode.get()
                                fallback=move || view! {
                                    <div class="note-preview">
                                        <MarkdownViewReactive content=content_signal />
                                    </div>
                                }
                            >
                                <textarea
                                    id="journal-content"
                                    class="form-input note-textarea"
                                    placeholder=""
                                    rows="12"
                                    prop:value=move || content.get()
                                    on:input=move |ev| content.set(event_target_value(&ev))
                                    required
                                />
                            </Show>
                        </div>

                        <div class="form-group">
                            <label style="display: flex; align-items: center; gap: 0.5rem; cursor: pointer;">
                                <input
                                    type="checkbox"
                                    prop:checked=move || is_shared.get()
                                    on:change=move |ev| is_shared.set(event_target_checked(&ev))
                                />
                                <span>{share_label.clone()}</span>
                            </label>
                        </div>
                    </div>

                    <div class="modal-footer">
                        <button
                            type="button"
                            class="btn btn-outline"
                            on:click=move |_| on_close.call(())
                            disabled=move || saving.get()
                        >
                            {cancel_text.clone()}
                        </button>
                        <button
                            type="submit"
                            class="btn btn-primary"
                            disabled=move || saving.get()
                        >
                            {move || if saving.get() { saving_text.clone() } else { submit_button_text.clone() }}
                        </button>
                    </div>
                </form>
            </div>
        </div>
    }
}
