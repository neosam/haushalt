use leptos::*;
use shared::TaskWithStatus;
use std::time::Duration;

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

    // Debounce state
    let is_debouncing = create_rw_signal(false);

    let on_plus = move |_| {
        if !is_target_met && !is_debouncing.get() {
            is_debouncing.set(true);

            // Set 1-second timeout
            let task_id_clone = task_id.clone();
            set_timeout(
                move || {
                    on_complete.call(task_id_clone);
                    is_debouncing.set(false);
                },
                Duration::from_secs(1)
            );
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
                    class=move || if is_debouncing.get() { "btn btn-primary btn-debouncing" } else { "btn btn-primary" }
                    style="padding: 0.25rem 0.75rem; font-size: 1rem; min-width: 32px;"
                    disabled=move || is_target_met || is_debouncing.get()
                    on:click=on_plus
                >
                    {move || if is_debouncing.get() { "..." } else { "+" }}
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
                            view! { <TaskCard task=task on_complete=on_complete on_uncomplete=on_uncomplete /> }
                        }).collect_view()}
                    </div>
                }.into_any()
            }}
        </div>
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use shared::{RecurrenceType, Task};
    use uuid::Uuid;
    use wasm_bindgen_test::*;

    wasm_bindgen_test_configure!(run_in_browser);

    fn create_test_task(completions: i32, target: i32) -> TaskWithStatus {
        TaskWithStatus {
            task: Task {
                id: Uuid::new_v4(),
                household_id: Uuid::new_v4(),
                title: "Test Task".to_string(),
                description: "Test description".to_string(),
                recurrence_type: RecurrenceType::Daily,
                recurrence_value: None,
                assigned_user_id: None,
                target_count: target,
                time_period: None,
                created_at: Utc::now(),
                updated_at: Utc::now(),
            },
            completions_today: completions,
            current_streak: 0,
            last_completion: None,
        }
    }

    #[wasm_bindgen_test]
    fn test_task_with_status_is_target_met_true() {
        let task = create_test_task(3, 3);
        assert!(task.is_target_met());
    }

    #[wasm_bindgen_test]
    fn test_task_with_status_is_target_met_false() {
        let task = create_test_task(2, 3);
        assert!(!task.is_target_met());
    }

    #[wasm_bindgen_test]
    fn test_task_with_status_remaining() {
        let task = create_test_task(1, 3);
        assert_eq!(task.remaining(), 2);
    }

    #[wasm_bindgen_test]
    fn test_task_with_status_remaining_zero_when_complete() {
        let task = create_test_task(3, 3);
        assert_eq!(task.remaining(), 0);
    }

    #[wasm_bindgen_test]
    fn test_task_with_status_remaining_over_target() {
        let task = create_test_task(5, 3);
        assert_eq!(task.remaining(), 0);
    }

    #[wasm_bindgen_test]
    fn test_progress_display_format() {
        let completions = 2;
        let target = 5;
        let progress_display = format!("{}/{}", completions, target);
        assert_eq!(progress_display, "2/5");
    }

    #[wasm_bindgen_test]
    fn test_card_class_completed() {
        let task = create_test_task(3, 3);
        let is_target_met = task.is_target_met();
        let card_class = if is_target_met {
            "task-item task-completed"
        } else {
            "task-item"
        };
        assert_eq!(card_class, "task-item task-completed");
    }

    #[wasm_bindgen_test]
    fn test_card_class_incomplete() {
        let task = create_test_task(1, 3);
        let is_target_met = task.is_target_met();
        let card_class = if is_target_met {
            "task-item task-completed"
        } else {
            "task-item"
        };
        assert_eq!(card_class, "task-item");
    }

    #[wasm_bindgen_test]
    fn test_has_completions_true() {
        let task = create_test_task(1, 3);
        let has_completions = task.completions_today > 0;
        assert!(has_completions);
    }

    #[wasm_bindgen_test]
    fn test_has_completions_false() {
        let task = create_test_task(0, 3);
        let has_completions = task.completions_today > 0;
        assert!(!has_completions);
    }

    #[wasm_bindgen_test]
    fn test_streak_display() {
        let task = TaskWithStatus {
            task: Task {
                id: Uuid::new_v4(),
                household_id: Uuid::new_v4(),
                title: "Test Task".to_string(),
                description: "".to_string(),
                recurrence_type: RecurrenceType::Daily,
                recurrence_value: None,
                assigned_user_id: None,
                target_count: 1,
                time_period: None,
                created_at: Utc::now(),
                updated_at: Utc::now(),
            },
            completions_today: 0,
            current_streak: 5,
            last_completion: None,
        };
        let streak_text = if task.current_streak > 0 {
            format!(" | Streak: {}", task.current_streak)
        } else {
            String::new()
        };
        assert_eq!(streak_text, " | Streak: 5");
    }

    #[wasm_bindgen_test]
    fn test_streak_display_zero() {
        let task = create_test_task(0, 1);
        let streak_text = if task.current_streak > 0 {
            format!(" | Streak: {}", task.current_streak)
        } else {
            String::new()
        };
        assert_eq!(streak_text, "");
    }
}
