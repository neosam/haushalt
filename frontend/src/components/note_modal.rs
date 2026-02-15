use leptos::*;
use shared::{CreateNoteRequest, Note, UpdateNoteRequest};

use crate::api::ApiClient;
use crate::components::markdown::MarkdownViewReactive;

#[component]
pub fn NoteModal(
    note: Option<Note>,
    household_id: String,
    #[prop(into)] on_close: Callback<()>,
    #[prop(into)] on_save: Callback<Note>,
) -> impl IntoView {
    let is_edit = note.is_some();
    let error = create_rw_signal(Option::<String>::None);
    let saving = create_rw_signal(false);
    let preview_mode = create_rw_signal(false);

    // Form fields - initialize based on mode
    let title = create_rw_signal(note.as_ref().map(|n| n.title.clone()).unwrap_or_default());
    let content = create_rw_signal(note.as_ref().map(|n| n.content.clone()).unwrap_or_default());
    let is_shared = create_rw_signal(note.as_ref().map(|n| n.is_shared).unwrap_or(false));

    let note_id = note.as_ref().map(|n| n.id.to_string());

    let on_submit = {
        let note_id = note_id.clone();
        let household_id = household_id.clone();

        move |ev: web_sys::SubmitEvent| {
            ev.prevent_default();

            if title.get().trim().is_empty() {
                error.set(Some("Note title is required".to_string()));
                return;
            }

            saving.set(true);
            error.set(None);

            let note_id = note_id.clone();
            let household_id = household_id.clone();

            wasm_bindgen_futures::spawn_local(async move {
                if let Some(note_id) = note_id {
                    // Edit mode - update existing note
                    let request = UpdateNoteRequest {
                        title: Some(title.get()),
                        content: Some(content.get()),
                        is_shared: Some(is_shared.get()),
                    };

                    match ApiClient::update_note(&household_id, &note_id, request).await {
                        Ok(updated_note) => {
                            saving.set(false);
                            on_save.call(updated_note);
                        }
                        Err(e) => {
                            error.set(Some(e));
                            saving.set(false);
                        }
                    }
                } else {
                    // Create mode - create new note
                    let request = CreateNoteRequest {
                        title: title.get(),
                        content: Some(content.get()),
                        is_shared: is_shared.get(),
                    };

                    match ApiClient::create_note(&household_id, request).await {
                        Ok(created_note) => {
                            saving.set(false);
                            on_save.call(created_note);
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

    let modal_title = if is_edit { "Edit Note" } else { "Create Note" };
    let submit_button_text = if is_edit { "Save Changes" } else { "Create" };
    let saving_text = if is_edit { "Saving..." } else { "Creating..." };

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
                            <label class="form-label" for="note-title">"Title"</label>
                            <input
                                type="text"
                                id="note-title"
                                class="form-input"
                                placeholder="Note title"
                                prop:value=move || title.get()
                                on:input=move |ev| title.set(event_target_value(&ev))
                                required
                            />
                        </div>

                        <div class="form-group">
                            <div style="display: flex; justify-content: space-between; align-items: center; margin-bottom: 0.5rem;">
                                <label class="form-label" for="note-content" style="margin-bottom: 0;">"Content (Markdown)"</label>
                                <button
                                    type="button"
                                    class="btn btn-outline btn-sm"
                                    on:click=toggle_preview
                                >
                                    {move || if preview_mode.get() { "Edit" } else { "Preview" }}
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
                                    id="note-content"
                                    class="form-input note-textarea"
                                    placeholder="Write your note using Markdown...

# Heading
- List item
**bold** and *italic*"
                                    rows="12"
                                    prop:value=move || content.get()
                                    on:input=move |ev| content.set(event_target_value(&ev))
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
                                <span>"Share with household members"</span>
                            </label>
                            <p class="form-hint">
                                {move || if is_shared.get() {
                                    "All household members can see this note"
                                } else {
                                    "Only you can see this note"
                                }}
                            </p>
                        </div>
                    </div>

                    <div class="modal-footer">
                        <button
                            type="button"
                            class="btn btn-outline"
                            on:click=move |_| on_close.call(())
                            disabled=move || saving.get()
                        >
                            "Cancel"
                        </button>
                        <button
                            type="submit"
                            class="btn btn-primary"
                            disabled=move || saving.get()
                        >
                            {move || if saving.get() { saving_text } else { submit_button_text }}
                        </button>
                    </div>
                </form>
            </div>
        </div>
    }
}
