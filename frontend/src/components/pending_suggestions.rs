use leptos::*;
use shared::Task;

use crate::api::ApiClient;
use crate::i18n::use_i18n;

#[component]
pub fn PendingSuggestions(
    household_id: String,
    /// Map of user IDs to usernames for displaying suggester
    #[prop(default = vec![])] members: Vec<shared::MemberWithUser>,
    #[prop(into)] on_suggestion_handled: Callback<()>,
) -> impl IntoView {
    let i18n = use_i18n();
    let i18n_stored = store_value(i18n);

    let suggestions = create_rw_signal(Vec::<Task>::new());
    let loading = create_rw_signal(true);
    let error = create_rw_signal(Option::<String>::None);
    let processing = create_rw_signal(Option::<String>::None); // Track which suggestion is being processed
    let members_stored = store_value(members);

    // Fetch pending suggestions
    {
        let household_id = household_id.clone();
        create_effect(move |_| {
            let household_id = household_id.clone();
            wasm_bindgen_futures::spawn_local(async move {
                match ApiClient::list_suggestions(&household_id).await {
                    Ok(data) => {
                        suggestions.set(data);
                        loading.set(false);
                    }
                    Err(e) => {
                        error.set(Some(e));
                        loading.set(false);
                    }
                }
            });
        });
    }

    let approve_suggestion = {
        let household_id = household_id.clone();
        move |task_id: String| {
            let household_id = household_id.clone();
            processing.set(Some(task_id.clone()));

            wasm_bindgen_futures::spawn_local(async move {
                match ApiClient::approve_suggestion(&household_id, &task_id).await {
                    Ok(_) => {
                        // Remove from local list
                        suggestions.update(|s| {
                            s.retain(|suggestion| suggestion.id.to_string() != task_id);
                        });
                        processing.set(None);
                        on_suggestion_handled.call(());
                    }
                    Err(e) => {
                        error.set(Some(e));
                        processing.set(None);
                    }
                }
            });
        }
    };

    let deny_suggestion = {
        let household_id = household_id.clone();
        move |task_id: String| {
            let household_id = household_id.clone();
            processing.set(Some(task_id.clone()));

            wasm_bindgen_futures::spawn_local(async move {
                match ApiClient::deny_suggestion(&household_id, &task_id).await {
                    Ok(_) => {
                        // Remove from local list
                        suggestions.update(|s| {
                            s.retain(|suggestion| suggestion.id.to_string() != task_id);
                        });
                        processing.set(None);
                        on_suggestion_handled.call(());
                    }
                    Err(e) => {
                        error.set(Some(e));
                        processing.set(None);
                    }
                }
            });
        }
    };

    let approve_suggestion = std::rc::Rc::new(approve_suggestion);
    let deny_suggestion = std::rc::Rc::new(deny_suggestion);

    view! {
        {
            let approve_suggestion = approve_suggestion.clone();
            let deny_suggestion = deny_suggestion.clone();
            move || {
            // Hide entire component when not loading and empty (no pending suggestions)
            // Also hide on "forbidden" errors (e.g., Solo Mode active)
            let is_forbidden = error.get().as_ref().is_some_and(|e| e.to_lowercase().contains("forbidden") || e.contains("permission"));
            if !loading.get() && (suggestions.get().is_empty() || is_forbidden) && (error.get().is_none() || is_forbidden) {
                return ().into_view();
            }

            let approve_suggestion = approve_suggestion.clone();
            let deny_suggestion = deny_suggestion.clone();
            view! {
                <div class="card">
                    <div class="card-header">
                        <h3 class="card-title">{i18n_stored.get_value().t("suggestions.title")}</h3>
                    </div>

                    {move || error.get().map(|e| view! {
                        <div class="alert alert-error" style="margin: 1rem;">{e}</div>
                    })}

                    {move || {
                        if loading.get() {
                            view! { <div class="empty-state"><p>{i18n_stored.get_value().t("common.loading")}</p></div> }.into_view()
                        } else {
                            let current_suggestions = suggestions.get();
                            let suggested_by_label = i18n_stored.get_value().t("suggestions.suggested_by");
                            let approve_label = i18n_stored.get_value().t("suggestions.approve");
                            let deny_label = i18n_stored.get_value().t("suggestions.deny");

                        current_suggestions.into_iter().map(|suggestion| {
                            let task_id = suggestion.id.to_string();
                            let task_id_for_approve = task_id.clone();
                            let task_id_for_deny = task_id.clone();
                            let task_id_check_1 = task_id.clone();
                            let task_id_check_2 = task_id.clone();
                            let task_id_check_3 = task_id.clone();
                            let task_id_check_4 = task_id.clone();
                            let approve = approve_suggestion.clone();
                            let deny = deny_suggestion.clone();
                            let created_at = suggestion.created_at.format("%b %d, %H:%M").to_string();
                            let suggested_by_label = suggested_by_label.clone();
                            let approve_label = approve_label.clone();
                            let deny_label = deny_label.clone();

                            // Look up suggester username
                            let suggester_name = suggestion.suggested_by
                                .and_then(|uid| {
                                    members_stored.get_value().iter()
                                        .find(|m| m.user.id == uid)
                                        .map(|m| m.user.username.clone())
                                })
                                .unwrap_or_else(|| "Unknown".to_string());

                            view! {
                                <div class="pending-review-item">
                                    <div class="pending-review-content">
                                        <div class="pending-review-task">{suggestion.title.clone()}</div>
                                        <div class="pending-review-meta">
                                            {suggested_by_label.clone()} " "
                                            <strong>{suggester_name}</strong>
                                            " - "{created_at}
                                        </div>
                                    </div>
                                    <div class="pending-review-actions">
                                        <button
                                            class="btn btn-success"
                                            style="padding: 0.25rem 0.75rem; font-size: 0.875rem;"
                                            disabled=move || processing.get() == Some(task_id_check_1.clone())
                                            on:click=move |_| approve(task_id_for_approve.clone())
                                        >
                                            {
                                                let approve_label = approve_label.clone();
                                                move || if processing.get() == Some(task_id_check_2.clone()) { "...".to_string() } else { approve_label.clone() }
                                            }
                                        </button>
                                        <button
                                            class="btn btn-danger"
                                            style="padding: 0.25rem 0.75rem; font-size: 0.875rem;"
                                            disabled=move || processing.get() == Some(task_id_check_3.clone())
                                            on:click=move |_| deny(task_id_for_deny.clone())
                                        >
                                            {
                                                let deny_label = deny_label.clone();
                                                move || if processing.get() == Some(task_id_check_4.clone()) { "...".to_string() } else { deny_label.clone() }
                                            }
                                        </button>
                                    </div>
                                </div>
                            }
                        }).collect_view().into_view()
                        }
                    }}
                </div>
            }.into_view()
        }}
    }
}
