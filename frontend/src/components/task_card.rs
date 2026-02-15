use leptos::*;
use shared::TaskWithStatus;

#[component]
pub fn TaskCard(
    task: TaskWithStatus,
    #[prop(into)] on_complete: Callback<String>,
) -> impl IntoView {
    let is_completed = task.is_completed_today;
    let task_id = task.task.id.to_string();

    let on_click = move |_| {
        if !is_completed {
            on_complete.call(task_id.clone());
        }
    };

    let card_class = if is_completed {
        "task-item task-completed"
    } else {
        "task-item"
    };

    view! {
        <div class=card_class>
            <input
                type="checkbox"
                class="task-checkbox"
                checked=is_completed
                disabled=is_completed
                on:change=on_click
            />
            <div class="task-content">
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
        </div>
    }
}

#[component]
pub fn TaskList(
    tasks: Vec<TaskWithStatus>,
    #[prop(into)] on_complete: Callback<String>,
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
                            let callback = on_complete.clone();
                            view! { <TaskCard task=task on_complete=callback /> }
                        }).collect_view()}
                    </div>
                }.into_any()
            }}
        </div>
    }
}
