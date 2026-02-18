use leptos::*;
use shared::JournalEntryWithUser;
use uuid::Uuid;

use crate::components::markdown::MarkdownView;
use crate::i18n::I18nContext;

/// A card displaying a single journal entry
#[component]
pub fn JournalEntryCard(
    entry: JournalEntryWithUser,
    current_user_id: Uuid,
    i18n: StoredValue<I18nContext>,
    #[prop(into)] on_edit: Callback<JournalEntryWithUser>,
    #[prop(into)] on_delete: Callback<Uuid>,
) -> impl IntoView {
    let entry_for_edit = entry.clone();
    let entry_id = entry.entry.id;
    let can_modify = entry.entry.user_id == current_user_id;
    let is_private = !entry.entry.is_shared;
    let title = entry.entry.title.clone();
    let content = entry.entry.content.clone();
    let author = entry.user.username.clone();
    let entry_date = entry.entry.entry_date.format("%B %d, %Y").to_string();

    let shared_label = i18n.get_value().t("journal.shared");
    let private_label = i18n.get_value().t("journal.private");
    let edit_label = i18n.get_value().t("common.edit");
    let delete_label = i18n.get_value().t("common.delete");

    let handle_edit = move |_: web_sys::MouseEvent| {
        on_edit.call(entry_for_edit.clone());
    };

    let handle_delete = move |_: web_sys::MouseEvent| {
        on_delete.call(entry_id);
    };

    view! {
        <div class="note-card journal-entry-card">
            <div class="note-header">
                <div class="journal-entry-title-row">
                    {if !title.is_empty() {
                        view! { <h3 class="note-title">{title}</h3> }.into_view()
                    } else {
                        view! {}.into_view()
                    }}
                    <span class="journal-entry-date">{entry_date}</span>
                </div>
                <div class="note-badges">
                    {if is_private {
                        view! { <span class="badge badge-private">{private_label}</span> }.into_view()
                    } else {
                        view! { <span class="badge badge-shared">{shared_label}</span> }.into_view()
                    }}
                </div>
            </div>

            <div class="note-content">
                <MarkdownView content=content />
            </div>

            <div class="note-footer">
                <div class="note-meta">
                    <span class="note-author">{author}</span>
                </div>

                {if can_modify {
                    view! {
                        <div class="note-actions">
                            <button
                                class="btn btn-outline btn-sm"
                                on:click=handle_edit
                            >
                                {edit_label}
                            </button>
                            <button
                                class="btn btn-danger btn-sm"
                                on:click=handle_delete
                            >
                                {delete_label}
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
