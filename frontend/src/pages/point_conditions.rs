use leptos::*;
use leptos_router::*;
use shared::{ConditionType, CreatePointConditionRequest, PointCondition};

use crate::api::ApiClient;
use crate::components::loading::Loading;
use crate::components::modal::Modal;

#[component]
pub fn PointConditionsPage() -> impl IntoView {
    let params = use_params_map();
    let household_id = move || params.with(|p| p.get("id").cloned().unwrap_or_default());

    let conditions = create_rw_signal(Vec::<PointCondition>::new());
    let loading = create_rw_signal(true);
    let error = create_rw_signal(Option::<String>::None);
    let show_create_modal = create_rw_signal(false);

    // Form fields
    let name = create_rw_signal(String::new());
    let condition_type = create_rw_signal("task_complete".to_string());
    let points_value = create_rw_signal(String::new());
    let streak_threshold = create_rw_signal(String::new());

    // Load conditions
    create_effect(move |_| {
        let id = household_id();
        if id.is_empty() {
            return;
        }

        wasm_bindgen_futures::spawn_local(async move {
            match ApiClient::list_point_conditions(&id).await {
                Ok(c) => {
                    conditions.set(c);
                    loading.set(false);
                }
                Err(e) => {
                    error.set(Some(e));
                    loading.set(false);
                }
            }
        });
    });

    let on_create = move |ev: web_sys::SubmitEvent| {
        ev.prevent_default();

        let id = household_id();
        let cond_type = match condition_type.get().as_str() {
            "task_complete" => ConditionType::TaskComplete,
            "task_missed" => ConditionType::TaskMissed,
            "streak" => ConditionType::Streak,
            "streak_broken" => ConditionType::StreakBroken,
            _ => ConditionType::TaskComplete,
        };

        let points: i64 = points_value.get().parse().unwrap_or(0);
        let threshold: Option<i32> = streak_threshold.get().parse().ok();

        let request = CreatePointConditionRequest {
            name: name.get(),
            condition_type: cond_type,
            points_value: points,
            streak_threshold: threshold,
            multiplier: None,
            task_id: None,
        };

        wasm_bindgen_futures::spawn_local(async move {
            match ApiClient::create_point_condition(&id, request).await {
                Ok(condition) => {
                    conditions.update(|c| c.push(condition));
                    show_create_modal.set(false);
                    name.set(String::new());
                    points_value.set(String::new());
                    streak_threshold.set(String::new());
                }
                Err(e) => error.set(Some(e)),
            }
        });
    };

    let on_delete = move |condition_id: String| {
        let id = household_id();
        wasm_bindgen_futures::spawn_local(async move {
            if ApiClient::delete_point_condition(&id, &condition_id).await.is_ok() {
                conditions.update(|c| c.retain(|cond| cond.id.to_string() != condition_id));
            }
        });
    };

    view! {
        <div class="dashboard-header">
            <h1 class="dashboard-title">"Point Conditions"</h1>
            <a href=move || format!("/households/{}", household_id()) style="color: var(--text-muted);">
                "← Back to household"
            </a>
        </div>

        {move || error.get().map(|e| view! {
            <div class="alert alert-error">{e}</div>
        })}

        <Show when=move || loading.get() fallback=|| ()>
            <Loading />
        </Show>

        <Show when=move || !loading.get() fallback=|| ()>
            <div style="margin-bottom: 1rem;">
                <button class="btn btn-primary" on:click=move |_| show_create_modal.set(true)>
                    "+ Create Condition"
                </button>
            </div>

            {move || {
                let c = conditions.get();
                if c.is_empty() {
                    view! {
                        <div class="card empty-state">
                            <p>"No point conditions yet."</p>
                            <p>"Define how members earn or lose points!"</p>
                        </div>
                    }.into_view()
                } else {
                    view! {
                        <div class="grid grid-3">
                            {c.into_iter().map(|condition| {
                                let condition_id = condition.id.to_string();
                                let delete_id = condition_id.clone();
                                let type_badge = match condition.condition_type {
                                    ConditionType::TaskComplete => ("badge badge-owner", "Task Complete"),
                                    ConditionType::TaskMissed => ("badge badge-member", "Task Missed"),
                                    ConditionType::Streak => ("badge badge-admin", "Streak"),
                                    ConditionType::StreakBroken => ("badge badge-member", "Streak Broken"),
                                };
                                view! {
                                    <div class="card">
                                        <div style="display: flex; justify-content: space-between; align-items: start; margin-bottom: 0.5rem;">
                                            <h3 class="card-title">{condition.name}</h3>
                                            <span class=type_badge.0>{type_badge.1}</span>
                                        </div>
                                        <div style="font-size: 1.5rem; font-weight: 700; margin: 1rem 0;">
                                            {if condition.points_value >= 0 {
                                                format!("+{} pts", condition.points_value)
                                            } else {
                                                format!("{} pts", condition.points_value)
                                            }}
                                        </div>
                                        {condition.streak_threshold.map(|t| view! {
                                            <p style="color: var(--text-muted); font-size: 0.875rem;">
                                                "Streak threshold: " {t}
                                            </p>
                                        })}
                                        <button
                                            class="btn btn-danger"
                                            style="width: 100%; margin-top: 0.5rem; padding: 0.25rem 0.5rem; font-size: 0.75rem;"
                                            on:click=move |_| on_delete(delete_id.clone())
                                        >
                                            "Delete"
                                        </button>
                                    </div>
                                }
                            }).collect_view()}
                        </div>
                    }.into_view()
                }
            }}
        </Show>

        <Show when=move || show_create_modal.get() fallback=|| ()>
            <Modal title="Create Point Condition" on_close=move |_| show_create_modal.set(false)>
                <form on:submit=on_create>
                    <div class="form-group">
                        <label class="form-label" for="condition-name">"Name"</label>
                        <input
                            type="text"
                            id="condition-name"
                            class="form-input"
                            placeholder="e.g., Task Completion Bonus"
                            prop:value=move || name.get()
                            on:input=move |ev| name.set(event_target_value(&ev))
                            required
                        />
                    </div>

                    <div class="form-group">
                        <label class="form-label" for="condition-type">"Condition Type"</label>
                        <select
                            id="condition-type"
                            class="form-select"
                            prop:value=move || condition_type.get()
                            on:change=move |ev| condition_type.set(event_target_value(&ev))
                        >
                            <option value="task_complete">"Task Completed"</option>
                            <option value="task_missed">"Task Missed"</option>
                            <option value="streak">"Streak Reached"</option>
                            <option value="streak_broken">"Streak Broken"</option>
                        </select>
                    </div>

                    <div class="form-group">
                        <label class="form-label" for="points-value">"Points Value"</label>
                        <input
                            type="number"
                            id="points-value"
                            class="form-input"
                            placeholder="10 (positive for rewards, negative for penalties)"
                            prop:value=move || points_value.get()
                            on:input=move |ev| points_value.set(event_target_value(&ev))
                            required
                        />
                    </div>

                    <Show when=move || condition_type.get() == "streak" || condition_type.get() == "streak_broken" fallback=|| ()>
                        <div class="form-group">
                            <label class="form-label" for="streak-threshold">"Streak Threshold"</label>
                            <input
                                type="number"
                                id="streak-threshold"
                                class="form-input"
                                placeholder="7"
                                min="1"
                                prop:value=move || streak_threshold.get()
                                on:input=move |ev| streak_threshold.set(event_target_value(&ev))
                            />
                        </div>
                    </Show>

                    <div class="modal-footer">
                        <button type="button" class="btn btn-outline" on:click=move |_| show_create_modal.set(false)>
                            "Cancel"
                        </button>
                        <button type="submit" class="btn btn-primary">
                            "Create"
                        </button>
                    </div>
                </form>
            </Modal>
        </Show>
    }
}
