use leptos::*;
use leptos_router::*;
use shared::{CreateTaskRequest, RecurrenceType, Task};

use crate::api::ApiClient;
use crate::components::loading::Loading;
use crate::components::modal::Modal;

#[component]
pub fn TasksPage() -> impl IntoView {
    let params = use_params_map();
    let household_id = move || params.with(|p| p.get("id").cloned().unwrap_or_default());

    let tasks = create_rw_signal(Vec::<Task>::new());
    let loading = create_rw_signal(true);
    let error = create_rw_signal(Option::<String>::None);
    let show_create_modal = create_rw_signal(false);

    // Form fields
    let title = create_rw_signal(String::new());
    let description = create_rw_signal(String::new());
    let recurrence_type = create_rw_signal("daily".to_string());

    // Load tasks
    create_effect(move |_| {
        let id = household_id();
        if id.is_empty() {
            return;
        }

        wasm_bindgen_futures::spawn_local(async move {
            match ApiClient::list_tasks(&id).await {
                Ok(t) => {
                    tasks.set(t);
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
        let rec_type = match recurrence_type.get().as_str() {
            "daily" => RecurrenceType::Daily,
            "weekly" => RecurrenceType::Weekly,
            "monthly" => RecurrenceType::Monthly,
            "weekdays" => RecurrenceType::Weekdays,
            _ => RecurrenceType::Daily,
        };

        let request = CreateTaskRequest {
            title: title.get(),
            description: Some(description.get()),
            recurrence_type: rec_type,
            recurrence_value: None,
            assigned_user_id: None,
        };

        wasm_bindgen_futures::spawn_local(async move {
            match ApiClient::create_task(&id, request).await {
                Ok(task) => {
                    tasks.update(|t| t.push(task));
                    show_create_modal.set(false);
                    title.set(String::new());
                    description.set(String::new());
                }
                Err(e) => error.set(Some(e)),
            }
        });
    };

    let on_delete = move |task_id: String| {
        let id = household_id();
        wasm_bindgen_futures::spawn_local(async move {
            if ApiClient::delete_task(&id, &task_id).await.is_ok() {
                tasks.update(|t| t.retain(|task| task.id.to_string() != task_id));
            }
        });
    };

    view! {
        <div class="dashboard-header">
            <h1 class="dashboard-title">"Tasks"</h1>
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
                    "+ Create Task"
                </button>
            </div>

            {move || {
                let t = tasks.get();
                if t.is_empty() {
                    view! {
                        <div class="card empty-state">
                            <p>"No tasks yet."</p>
                            <p>"Create your first task to get started!"</p>
                        </div>
                    }.into_view()
                } else {
                    view! {
                        <div class="card">
                            {t.into_iter().map(|task| {
                                let task_id = task.id.to_string();
                                let delete_id = task_id.clone();
                                view! {
                                    <div class="task-item">
                                        <div class="task-content">
                                            <div class="task-title">{task.title}</div>
                                            <div class="task-meta">
                                                {format!("{:?}", task.recurrence_type)}
                                                {if !task.description.is_empty() {
                                                    format!(" | {}", task.description)
                                                } else {
                                                    String::new()
                                                }}
                                            </div>
                                        </div>
                                        <button
                                            class="btn btn-danger"
                                            style="padding: 0.25rem 0.5rem; font-size: 0.75rem;"
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
            <Modal title="Create Task" on_close=move |_| show_create_modal.set(false)>
                <form on:submit=on_create>
                    <div class="form-group">
                        <label class="form-label" for="task-title">"Title"</label>
                        <input
                            type="text"
                            id="task-title"
                            class="form-input"
                            placeholder="e.g., Take out the trash"
                            prop:value=move || title.get()
                            on:input=move |ev| title.set(event_target_value(&ev))
                            required
                        />
                    </div>

                    <div class="form-group">
                        <label class="form-label" for="task-description">"Description"</label>
                        <input
                            type="text"
                            id="task-description"
                            class="form-input"
                            placeholder="Optional description"
                            prop:value=move || description.get()
                            on:input=move |ev| description.set(event_target_value(&ev))
                        />
                    </div>

                    <div class="form-group">
                        <label class="form-label" for="recurrence">"Recurrence"</label>
                        <select
                            id="recurrence"
                            class="form-select"
                            prop:value=move || recurrence_type.get()
                            on:change=move |ev| recurrence_type.set(event_target_value(&ev))
                        >
                            <option value="daily">"Daily"</option>
                            <option value="weekly">"Weekly"</option>
                            <option value="monthly">"Monthly"</option>
                            <option value="weekdays">"Weekdays (Mon-Fri)"</option>
                        </select>
                    </div>

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
