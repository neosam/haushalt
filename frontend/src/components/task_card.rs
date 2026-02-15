use leptos::*;
use shared::TaskWithStatus;

#[component]
pub fn TaskCard(
    task: TaskWithStatus,
    #[prop(into)] on_complete: Callback<String>,
    #[prop(into)] on_uncomplete: Callback<String>,
) -> impl IntoView {
    let is_target_met = task.is_target_met();
    let task_id = task.task.id.to_string();
    let task_id_for_minus = task_id.clone();
    let completions = task.completions_today;
    let target = task.task.target_count;
    let has_completions = completions > 0;

    let on_plus = move |_| {
        if !is_target_met {
            on_complete.call(task_id.clone());
        }
    };

    let on_minus = move |_| {
        if has_completions {
            on_uncomplete.call(task_id_for_minus.clone());
        }
    };

    let card_class = if is_target_met {
        "task-item task-completed"
    } else {
        "task-item"
    };

    // Progress display as fraction (e.g., "2/3")
    let progress_display = format!("{}/{}", completions, target);

    view! {
        <div class=card_class>
            <div class="task-content" style="flex: 1;">
                <div class="task-title">{task.task.title.clone()}</div>
                <div class="task-meta">
                    {format!("{:?}", task.task.recurrence_type)}
                    {if task.current_streak > 0 {
                        format!(" | Streak: {}", task.current_streak)
                    } else {
                        String::new()
                    }}
                </div>
            </div>
            <div style="display: flex; align-items: center; gap: 0.5rem;">
                <button
                    class="btn btn-outline"
                    style="padding: 0.25rem 0.75rem; font-size: 1rem; min-width: 32px;"
                    disabled=!has_completions
                    on:click=on_minus
                >
                    "-"
                </button>
                <span style="font-size: 0.875rem; color: var(--text-muted); min-width: 2rem; text-align: center;">
                    {progress_display}
                </span>
                <button
                    class="btn btn-primary"
                    style="padding: 0.25rem 0.75rem; font-size: 1rem; min-width: 32px;"
                    disabled=is_target_met
                    on:click=on_plus
                >
                    "+"
                </button>
            </div>
        </div>
    }
}

#[component]
pub fn TaskList(
    tasks: Vec<TaskWithStatus>,
    #[prop(into)] on_complete: Callback<String>,
    #[prop(into)] on_uncomplete: Callback<String>,
) -> impl IntoView {
    view! {
        <div class="card">
            <div class="card-header">
                <h3 class="card-title">"Today's Tasks"</h3>
            </div>
            {if tasks.is_empty() {
                view! {
                    <div class="empty-state">
                        <p>"No tasks due today!"</p>
                    </div>
                }.into_any()
            } else {
                view! {
                    <div>
                        {tasks.into_iter().map(|task| {
                            let complete_callback = on_complete.clone();
                            let uncomplete_callback = on_uncomplete.clone();
                            view! { <TaskCard task=task on_complete=complete_callback on_uncomplete=uncomplete_callback /> }
                        }).collect_view()}
                    </div>
                }.into_any()
            }}
        </div>
    }
}
