use leptos::*;
use shared::PendingReview;

use crate::api::ApiClient;

#[component]
pub fn PendingReviews(
    household_id: String,
    #[prop(into)] on_review_complete: Callback<()>,
) -> impl IntoView {
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

    let approve_completion = {
        let household_id = household_id.clone();
        move |completion_id: String| {
            let household_id = household_id.clone();
            processing.set(Some(completion_id.clone()));

            wasm_bindgen_futures::spawn_local(async move {
                match ApiClient::approve_completion(&household_id, &completion_id).await {
                    Ok(_) => {
                        // Remove from local list
                        reviews.update(|r| {
                            r.retain(|review| review.completion.id.to_string() != completion_id);
                        });
                        processing.set(None);
                        on_review_complete.call(());
                    }
                    Err(e) => {
                        error.set(Some(e));
                        processing.set(None);
                    }
                }
            });
        }
    };

    let reject_completion = {
        let household_id = household_id.clone();
        move |completion_id: String| {
            let household_id = household_id.clone();
            processing.set(Some(completion_id.clone()));

            wasm_bindgen_futures::spawn_local(async move {
                match ApiClient::reject_completion(&household_id, &completion_id).await {
                    Ok(_) => {
                        // Remove from local list
                        reviews.update(|r| {
                            r.retain(|review| review.completion.id.to_string() != completion_id);
                        });
                        processing.set(None);
                        on_review_complete.call(());
                    }
                    Err(e) => {
                        error.set(Some(e));
                        processing.set(None);
                    }
                }
            });
        }
    };

    view! {
        <div class="card">
            <div class="card-header">
                <h3 class="card-title">"Pending Reviews"</h3>
            </div>

            {move || error.get().map(|e| view! {
                <div class="alert alert-error" style="margin: 1rem;">{e}</div>
            })}

            {move || {
                if loading.get() {
                    view! { <div class="empty-state"><p>"Loading..."</p></div> }.into_view()
                } else {
                    let current_reviews = reviews.get();
                    if current_reviews.is_empty() {
                        view! { <div class="empty-state"><p>"No pending reviews"</p></div> }.into_view()
                    } else {
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

                            view! {
                                <div class="pending-review-item">
                                    <div class="pending-review-content">
                                        <div class="pending-review-task">{review.task.title.clone()}</div>
                                        <div class="pending-review-meta">
                                            "Completed by "
                                            <strong>{review.user.username.clone()}</strong>
                                            " on "{completed_at}
                                        </div>
                                    </div>
                                    <div class="pending-review-actions">
                                        <button
                                            class="btn btn-success"
                                            style="padding: 0.25rem 0.75rem; font-size: 0.875rem;"
                                            disabled=move || processing.get() == Some(completion_id_check_1.clone())
                                            on:click=move |_| approve(completion_id_for_approve.clone())
                                        >
                                            {move || if processing.get() == Some(completion_id_check_2.clone()) { "..." } else { "Approve" }}
                                        </button>
                                        <button
                                            class="btn btn-danger"
                                            style="padding: 0.25rem 0.75rem; font-size: 0.875rem;"
                                            disabled=move || processing.get() == Some(completion_id_check_3.clone())
                                            on:click=move |_| reject(completion_id_for_reject.clone())
                                        >
                                            {move || if processing.get() == Some(completion_id_check_4.clone()) { "..." } else { "Reject" }}
                                        </button>
                                    </div>
                                </div>
                            }
                        }).collect_view().into_view()
                    }
                }
            }}
        </div>
    }
}
