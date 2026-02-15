use leptos::*;
use chrono::NaiveDate;

#[component]
pub fn CalendarPicker(
    #[prop(into)] selected_dates: RwSignal<Vec<NaiveDate>>,
) -> impl IntoView {
    let date_input = create_rw_signal(String::new());
    let error = create_rw_signal(Option::<String>::None);

    let add_date = move |_| {
        let input = date_input.get();
        if input.is_empty() {
            error.set(Some("Please select a date".to_string()));
            return;
        }

        match NaiveDate::parse_from_str(&input, "%Y-%m-%d") {
            Ok(date) => {
                selected_dates.update(|dates| {
                    if !dates.contains(&date) {
                        dates.push(date);
                        dates.sort();
                    }
                });
                date_input.set(String::new());
                error.set(None);
            }
            Err(_) => {
                error.set(Some("Invalid date format. Use YYYY-MM-DD".to_string()));
            }
        }
    };

    view! {
        <div class="calendar-picker">
            <div class="form-group">
                <label class="form-label">"Add Custom Date"</label>
                <div style="display: flex; gap: 0.5rem;">
                    <input
                        type="date"
                        class="form-input"
                        prop:value=move || date_input.get()
                        on:input=move |ev| date_input.set(event_target_value(&ev))
                    />
                    <button
                        type="button"
                        class="btn btn-outline"
                        on:click=add_date
                    >
                        "Add"
                    </button>
                </div>
                {move || error.get().map(|e| view! {
                    <small style="color: var(--error-color, #dc3545);">{e}</small>
                })}
            </div>

            <div class="selected-dates">
                <strong>"Selected Dates:"</strong>
                {move || {
                    let dates = selected_dates.get();
                    if dates.is_empty() {
                        view! { <p style="color: var(--text-muted, #6c757d);">"No dates selected"</p> }.into_view()
                    } else {
                        view! {
                            <ul style="list-style: none; padding: 0; margin-top: 0.5rem;">
                                {dates.into_iter().map(|date| {
                    let date_str = date.format("%Y-%m-%d").to_string();
                    let date_for_remove = date;
                                    view! {
                                        <li style="display: flex; justify-content: space-between; align-items: center; padding: 0.25rem 0; border-bottom: 1px solid var(--border-color, #dee2e6);">
                                            <span>{date_str}</span>
                                            <button
                                                type="button"
                                                class="btn btn-outline"
                                                style="padding: 0.125rem 0.5rem; font-size: 0.75rem;"
                                                on:click=move |_| {
                                                    selected_dates.update(|dates| {
                                                        dates.retain(|d| *d != date_for_remove);
                                                    });
                                                }
                                            >
                                                "Remove"
                                            </button>
                                        </li>
                                    }
                                }).collect_view()}
                            </ul>
                        }.into_view()
                    }
                }}
            </div>
        </div>
    }
}
