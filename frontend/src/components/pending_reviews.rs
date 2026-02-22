use leptos::*;
use shared::PendingReview;

use crate::api::ApiClient;
use crate::i18n::use_i18n;
use crate::utils::create_remove_action_handler;

#[component]
pub fn PendingReviews(
    household_id: String,
    #[prop(into)] on_review_complete: Callback<()>,
) -> impl IntoView {
    let i18n = use_i18n();
    let i18n_stored = store_value(i18n);

    let reviews = create_rw_signal(Vec::<PendingReview>::new());
    let loading = create_rw_signal(true);
    let error = create_rw_signal(Option::<String>::None);
    let processing = create_rw_signal(Option::<String>::None); // Track which completion is being processed

    // Fetch pending reviews
    {
        let household_id = household_id.clone();
        create_effect(move |_| {
            let household_id = household_id.clone();
            wasm_bindgen_futures::spawn_local(async move {
                match ApiClient::get_pending_reviews(&household_id).await {
                    Ok(data) => {
                        reviews.set(data);
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

    let id_matcher = |r: &PendingReview| r.completion.id.to_string();

    let approve_completion = create_remove_action_handler(
        household_id.clone(),
        processing,
        reviews,
        error,
        on_review_complete,
        id_matcher,
        |hid, cid| async move {
            ApiClient::approve_completion(&hid, &cid).await.map(|_| ())
        },
    );

    let reject_completion = create_remove_action_handler(
        household_id.clone(),
        processing,
        reviews,
        error,
        on_review_complete,
        id_matcher,
        |hid, cid| async move {
            ApiClient::reject_completion(&hid, &cid).await.map(|_| ())
        },
    );

    view! {
        {
            let approve_completion = approve_completion.clone();
            let reject_completion = reject_completion.clone();
            move || {
            // Hide entire component when not loading and empty (no pending reviews)
            // Also hide on "forbidden" errors (e.g., Solo Mode active)
            let is_forbidden = error.get().as_ref().is_some_and(|e| e.to_lowercase().contains("forbidden") || e.contains("permission"));
            if !loading.get() && (reviews.get().is_empty() || is_forbidden) && (error.get().is_none() || is_forbidden) {
                return ().into_view();
            }

            let approve_completion = approve_completion.clone();
            let reject_completion = reject_completion.clone();
            view! {
                <div class="card">
                    <div class="card-header">
                        <h3 class="card-title">{i18n_stored.get_value().t("pending_reviews.title")}</h3>
                    </div>

                    {move || error.get().map(|e| view! {
                        <div class="alert alert-error" style="margin: 1rem;">{e}</div>
                    })}

                    {move || {
                        if loading.get() {
                            view! { <div class="empty-state"><p>{i18n_stored.get_value().t("common.loading")}</p></div> }.into_view()
                        } else {
                            let current_reviews = reviews.get();
                        let completed_by_label = i18n_stored.get_value().t("pending_reviews.completed_by");
                        let approve_label = i18n_stored.get_value().t("pending_reviews.approve");
                        let reject_label = i18n_stored.get_value().t("pending_reviews.reject");

                        current_reviews.into_iter().map(|review| {
                            let completion_id = review.completion.id.to_string();
                            let completion_id_for_approve = completion_id.clone();
                            let completion_id_for_reject = completion_id.clone();
                            let completion_id_check_1 = completion_id.clone();
                            let completion_id_check_2 = completion_id.clone();
                            let completion_id_check_3 = completion_id.clone();
                            let completion_id_check_4 = completion_id.clone();
                            let approve = approve_completion.clone();
                            let reject = reject_completion.clone();
                            let completed_at = review.completion.completed_at.format("%b %d, %H:%M").to_string();
                            let completed_by_label = completed_by_label.clone();
                            let approve_label = approve_label.clone();
                            let reject_label = reject_label.clone();

                            view! {
                                <div class="pending-review-item">
                                    <div class="pending-review-content">
                                        <div class="pending-review-task">{review.task.title.clone()}</div>
                                        <div class="pending-review-meta">
                                            {completed_by_label.clone()} " "
                                            <strong>{review.user.username.clone()}</strong>
                                            " - "{completed_at}
                                        </div>
                                    </div>
                                    <div class="pending-review-actions">
                                        <button
                                            class="btn btn-success"
                                            style="padding: 0.25rem 0.75rem; font-size: 0.875rem;"
                                            disabled=move || processing.get() == Some(completion_id_check_1.clone())
                                            on:click=move |_| approve(completion_id_for_approve.clone())
                                        >
                                            {
                                                let approve_label = approve_label.clone();
                                                move || if processing.get() == Some(completion_id_check_2.clone()) { "...".to_string() } else { approve_label.clone() }
                                            }
                                        </button>
                                        <button
                                            class="btn btn-danger"
                                            style="padding: 0.25rem 0.75rem; font-size: 0.875rem;"
                                            disabled=move || processing.get() == Some(completion_id_check_3.clone())
                                            on:click=move |_| reject(completion_id_for_reject.clone())
                                        >
                                            {
                                                let reject_label = reject_label.clone();
                                                move || if processing.get() == Some(completion_id_check_4.clone()) { "...".to_string() } else { reject_label.clone() }
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
