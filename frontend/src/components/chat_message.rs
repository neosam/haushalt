use leptos::*;
use shared::ChatMessageWithUser;
use uuid::Uuid;

use crate::utils::format_time;

/// A single chat message display
#[component]
pub fn ChatMessage(
    message: ChatMessageWithUser,
    current_user_id: Uuid,
    on_edit: Callback<(Uuid, String)>,
    on_delete: Callback<Uuid>,
    #[prop(default = "UTC".to_string())] timezone: String,
) -> impl IntoView {
    let (editing, set_editing) = create_signal(false);
    let original_content = store_value(message.message.content.clone());
    let (edit_content, set_edit_content) = create_signal(message.message.content.clone());

    let is_own_message = message.message.user_id == current_user_id;
    let is_deleted = message.message.is_deleted;
    let message_id = message.message.id;
    let content_display = message.message.content.clone();
    let username = message.user.username.clone();

    let formatted_time = {
        let created = message.message.created_at;
        let updated = message.message.updated_at;
        let was_edited = updated > created;

        let time_str = format_time(created, &timezone);
        if was_edited {
            format!("{} (edited)", time_str)
        } else {
            time_str
        }
    };

    let handle_save = move |_: web_sys::MouseEvent| {
        let content = edit_content.get();
        if !content.trim().is_empty() {
            on_edit.call((message_id, content));
            set_editing.set(false);
        }
    };

    let handle_cancel = move |_: web_sys::MouseEvent| {
        set_edit_content.set(original_content.get_value());
        set_editing.set(false);
    };

    let handle_delete = move |_: web_sys::MouseEvent| {
        on_delete.call(message_id);
    };

    let handle_start_edit = move |_: web_sys::MouseEvent| {
        set_editing.set(true);
    };

    view! {
        <div class=move || {
            if is_own_message {
                "chat-message chat-message-own"
            } else {
                "chat-message"
            }
        }>
            <div class="chat-message-header">
                <span class="chat-message-author">{username}</span>
                <span class="chat-message-time">{formatted_time}</span>
            </div>

            {move || {
                if editing.get() {
                    view! {
                        <div class="chat-message-edit">
                            <textarea
                                class="chat-edit-input"
                                prop:value=move || edit_content.get()
                                on:input=move |ev| set_edit_content.set(event_target_value(&ev))
                                rows="2"
                            />
                            <div class="chat-edit-buttons">
                                <button class="btn btn-primary btn-sm" on:click=handle_save>"Save"</button>
                                <button class="btn btn-secondary btn-sm" on:click=handle_cancel>"Cancel"</button>
                            </div>
                        </div>
                    }.into_view()
                } else {
                    let content = content_display.clone();
                    view! {
                        <div class="chat-message-content">
                            {if is_deleted {
                                view! { <em class="chat-message-deleted">"[Message deleted]"</em> }.into_view()
                            } else {
                                view! { <span>{content}</span> }.into_view()
                            }}
                        </div>
                    }.into_view()
                }
            }}

            {move || {
                if is_own_message && !is_deleted && !editing.get() {
                    view! {
                        <div class="chat-message-actions">
                            <button
                                class="btn-icon"
                                title="Edit"
                                on:click=handle_start_edit
                            >
                                "Edit"
                            </button>
                            <button
                                class="btn-icon btn-danger-text"
                                title="Delete"
                                on:click=handle_delete
                            >
                                "Delete"
                            </button>
                        </div>
                    }.into_view()
                } else {
                    view! {}.into_view()
                }
            }}
        </div>
    }
}
