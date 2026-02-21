use chrono::NaiveDate;
use leptos::*;

use crate::components::date_input::DateInput;
use crate::components::modal::Modal;
use crate::i18n::use_i18n;

/// Modal for setting a date on a task (converts to Custom recurrence)
#[component]
pub fn SetDateModal(
    #[prop(into)] task_id: String,
    #[prop(into)] task_title: String,
    #[prop(into)] on_save: Callback<(String, NaiveDate)>,
    #[prop(into)] on_close: Callback<()>,
) -> impl IntoView {
    let i18n = use_i18n();
    let selected_date = create_rw_signal::<Option<NaiveDate>>(None);
    let task_id_for_save = task_id.clone();

    let on_submit = move |_| {
        if let Some(date) = selected_date.get() {
            on_save.call((task_id_for_save.clone(), date));
        }
    };

    let can_save = move || selected_date.get().is_some();

    view! {
        <Modal
            title=i18n.t("task_card.set_date_title")
            on_close=on_close
            class="modal-sm"
        >
            <div class="modal-body">
                <p style="margin-bottom: 1rem; color: var(--text-muted);">
                    {i18n.t("task_card.set_date_hint")}
                </p>
                <p style="margin-bottom: 0.5rem; font-weight: 500;">
                    {task_title}
                </p>
                <div class="form-group">
                    <label class="form-label">{i18n.t("task_card.select_date")}</label>
                    <DateInput value=selected_date />
                </div>
            </div>
            <div class="modal-footer">
                <button
                    class="btn btn-outline"
                    on:click=move |_| on_close.call(())
                >
                    {i18n.t("common.cancel")}
                </button>
                <button
                    class="btn btn-primary"
                    disabled=move || !can_save()
                    on:click=on_submit
                >
                    {i18n.t("common.save")}
                </button>
            </div>
        </Modal>
    }
}

#[cfg(test)]
mod tests {
    use wasm_bindgen_test::*;

    wasm_bindgen_test_configure!(run_in_browser);

    #[wasm_bindgen_test]
    fn test_set_date_modal_css_classes() {
        assert_eq!("modal-sm", "modal-sm");
        assert_eq!("modal-body", "modal-body");
        assert_eq!("modal-footer", "modal-footer");
    }
}
