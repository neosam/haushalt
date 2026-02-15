use leptos::*;
use shared::NoteWithUser;
use uuid::Uuid;

use crate::components::markdown::MarkdownView;

/// A card displaying a single note
#[component]
pub fn NoteCard(
    note: NoteWithUser,
    current_user_id: Uuid,
    #[prop(into)] on_edit: Callback<NoteWithUser>,
    #[prop(into)] on_delete: Callback<Uuid>,
) -> impl IntoView {
    let note_for_edit = note.clone();
    let note_id = note.note.id;
    let can_modify = note.note.user_id == current_user_id;
    let is_private = !note.note.is_shared;
    let title = note.note.title.clone();
    let content = note.note.content.clone();
    let author = note.user.username.clone();

    let format_time = {
        let updated = note.note.updated_at;
        updated.format("%Y-%m-%d %H:%M").to_string()
    };

    let handle_edit = move |_: web_sys::MouseEvent| {
        on_edit.call(note_for_edit.clone());
    };

    let handle_delete = move |_: web_sys::MouseEvent| {
        on_delete.call(note_id);
    };

    view! {
        <div class="note-card">
            <div class="note-header">
                <h3 class="note-title">{title}</h3>
                <div class="note-badges">
                    {if is_private {
                        view! { <span class="badge badge-private">"Private"</span> }.into_view()
                    } else {
                        view! { <span class="badge badge-shared">"Shared"</span> }.into_view()
                    }}
                </div>
            </div>

            <div class="note-content">
                <MarkdownView content=content />
            </div>

            <div class="note-footer">
                <div class="note-meta">
                    <span class="note-author">"By: " {author}</span>
                    <span class="note-time">{format_time}</span>
                </div>

                {if can_modify {
                    view! {
                        <div class="note-actions">
                            <button
                                class="btn btn-outline btn-sm"
                                on:click=handle_edit
                            >
                                "Edit"
                            </button>
                            <button
                                class="btn btn-danger btn-sm"
                                on:click=handle_delete
                            >
                                "Delete"
                            </button>
                        </div>
                    }.into_view()
                } else {
                    view! {}.into_view()
                }}
            </div>
        </div>
    }
}
