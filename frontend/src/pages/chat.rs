use leptos::*;
use leptos_router::*;
use shared::{ChatMessageWithUser, HouseholdSettings, User};
use uuid::Uuid;

use crate::api::ApiClient;
use crate::components::chat_message::ChatMessage;
use crate::components::loading::Loading;
use crate::i18n::use_i18n;

#[component]
pub fn ChatPage() -> impl IntoView {
    let i18n = use_i18n();
    let i18n_stored = store_value(i18n);

    let params = use_params_map();

    // Get household_id once at component creation
    let household_id_initial = params.with_untracked(|p| p.get("id").cloned().unwrap_or_default());

    // State
    let messages = create_rw_signal(Vec::<ChatMessageWithUser>::new());
    let current_user = create_rw_signal(Option::<User>::None);
    let settings = create_rw_signal(Option::<HouseholdSettings>::None);
    let loading = create_rw_signal(true);
    let error = create_rw_signal(Option::<String>::None);
    let new_message = create_rw_signal(String::new());
    let sending = create_rw_signal(false);

    // Store household_id for use in closures
    let household_id = store_value(household_id_initial.clone());

    // Load initial data
    if !household_id_initial.is_empty() {
        let id_for_user = household_id_initial.clone();
        let id_for_settings = household_id_initial.clone();
        let id_for_messages = household_id_initial.clone();

        // Load current user
        wasm_bindgen_futures::spawn_local(async move {
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

        // Load initial messages
        wasm_bindgen_futures::spawn_local(async move {
            match ApiClient::list_chat_messages(&id_for_messages, Some(50), None).await {
                Ok(mut msgs) => {
                    // Reverse to show oldest first
                    msgs.reverse();
                    messages.set(msgs);
                    loading.set(false);
                }
                Err(e) => {
                    error.set(Some(e));
                    loading.set(false);
                }
            }
        });

        // Start polling for new messages
        let id_for_polling = id_for_user.clone();
        set_interval(
            move || {
                let id = id_for_polling.clone();
                wasm_bindgen_futures::spawn_local(async move {
                    if let Ok(mut new_msgs) = ApiClient::list_chat_messages(&id, Some(50), None).await {
                        new_msgs.reverse();
                        messages.set(new_msgs);
                    }
                });
            },
            std::time::Duration::from_secs(3),
        );
    }

    // Send message handler
    let do_send_message = move || {
        let content = new_message.get();
        if content.trim().is_empty() || sending.get() {
            return;
        }

        sending.set(true);
        let id = household_id.get_value();
        let content_clone = content.clone();

        wasm_bindgen_futures::spawn_local(async move {
            match ApiClient::send_chat_message(&id, &content_clone).await {
                Ok(msg) => {
                    messages.update(|msgs| {
                        if !msgs.iter().any(|m| m.message.id == msg.message.id) {
                            msgs.push(msg);
                        }
                    });
                    new_message.set(String::new());
                }
                Err(e) => {
                    error.set(Some(e));
                }
            }
            sending.set(false);
        });
    };

    let send_message = move |_: web_sys::MouseEvent| {
        do_send_message();
    };

    let handle_keydown = move |ev: web_sys::KeyboardEvent| {
        if ev.key() == "Enter" && !ev.shift_key() {
            ev.prevent_default();
            do_send_message();
        }
    };

    let on_edit = Callback::new(move |(message_id, content): (Uuid, String)| {
        let id = household_id.get_value();
        let msg_id = message_id.to_string();

        wasm_bindgen_futures::spawn_local(async move {
            match ApiClient::update_chat_message(&id, &msg_id, &content).await {
                Ok(updated_msg) => {
                    messages.update(|msgs| {
                        if let Some(m) = msgs.iter_mut().find(|m| m.message.id == message_id) {
                            *m = updated_msg;
                        }
                    });
                }
                Err(e) => {
                    error.set(Some(e));
                }
            }
        });
    });

    let on_delete = Callback::new(move |message_id: Uuid| {
        let id = household_id.get_value();
        let msg_id = message_id.to_string();

        wasm_bindgen_futures::spawn_local(async move {
            match ApiClient::delete_chat_message(&id, &msg_id).await {
                Ok(()) => {
                    messages.update(|msgs| {
                        if let Some(m) = msgs.iter_mut().find(|m| m.message.id == message_id) {
                            m.message.is_deleted = true;
                        }
                    });
                }
                Err(e) => {
                    error.set(Some(e));
                }
            }
        });
    });

    view! {
        <div class="dashboard-header">
            <h1 class="dashboard-title">{i18n_stored.get_value().t("chat.title")}</h1>
        </div>

        {move || error.get().map(|e| view! {
            <div class="alert alert-error">{e}</div>
        })}

        <Show when=move || loading.get() fallback=|| ()>
            <Loading />
        </Show>

        <Show when=move || !loading.get() fallback=|| ()>
            <div class="chat-container">
                <div class="chat-messages">
                    {move || {
                        let msgs = messages.get();
                        let user_id = current_user.get().map(|u| u.id).unwrap_or(Uuid::nil());

                        if msgs.is_empty() {
                            view! {
                                <div class="chat-empty">
                                    <p>{i18n_stored.get_value().t("chat.start_conversation")}</p>
                                </div>
                            }.into_view()
                        } else {
                            let tz = settings.get().map(|s| s.timezone).unwrap_or_else(|| "UTC".to_string());
                            msgs.into_iter().map(|msg| {
                                let tz = tz.clone();
                                view! {
                                    <ChatMessage
                                        message=msg
                                        current_user_id=user_id
                                        on_edit=on_edit
                                        on_delete=on_delete
                                        timezone=tz
                                    />
                                }
                            }).collect_view()
                        }
                    }}
                </div>

                <div class="chat-input-area">
                    <textarea
                        class="chat-input"
                        placeholder=i18n_stored.get_value().t("chat.placeholder")
                        prop:value=move || new_message.get()
                        on:input=move |ev| new_message.set(event_target_value(&ev))
                        on:keydown=handle_keydown
                        rows="2"
                    />
                    <button
                        class="btn btn-primary chat-send-btn"
                        on:click=send_message
                        disabled=move || sending.get() || new_message.get().trim().is_empty()
                    >
                        {move || if sending.get() { i18n_stored.get_value().t("chat.sending") } else { i18n_stored.get_value().t("chat.send") }}
                    </button>
                </div>
            </div>
        </Show>
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
