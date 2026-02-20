use leptos::*;
use shared::{Household, HouseholdSettings, Role};

use crate::components::modal::Modal;
use crate::i18n::use_i18n;

/// Whether the user is creating a task or suggesting one
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TaskAction {
    Create,
    Suggest,
}

/// Household with the user's role, settings, and action for permission checking
#[derive(Clone, Debug)]
pub struct EligibleHousehold {
    pub household: Household,
    pub role: Role,
    pub settings: HouseholdSettings,
    pub action: TaskAction,
}

#[component]
pub fn HouseholdPickerModal(
    households: Vec<EligibleHousehold>,
    #[prop(into)] on_select: Callback<EligibleHousehold>,
    #[prop(into)] on_close: Callback<()>,
) -> impl IntoView {
    let i18n = use_i18n();
    let i18n_stored = store_value(i18n);
    let households_stored = store_value(households);

    view! {
        <Modal
            title=i18n_stored.get_value().t("quick_task.select_household")
            on_close=on_close
        >
            <div class="modal-body">
                <ul class="household-picker-list">
                    {move || {
                        let create_label = i18n_stored.get_value().t("quick_task.action_create");
                        let suggest_label = i18n_stored.get_value().t("quick_task.action_suggest");

                        households_stored.get_value().into_iter().map(|eh| {
                            let eh_clone = eh.clone();
                            let action_label = match eh.action {
                                TaskAction::Create => create_label.clone(),
                                TaskAction::Suggest => suggest_label.clone(),
                            };
                            let action_class = match eh.action {
                                TaskAction::Create => "household-picker-action create",
                                TaskAction::Suggest => "household-picker-action suggest",
                            };
                            view! {
                                <li
                                    class="household-picker-item"
                                    on:pointerup=move |_| on_select.call(eh_clone.clone())
                                >
                                    <div class="household-picker-name">{eh.household.name.clone()}</div>
                                    <div class="household-picker-meta">
                                        <span class="household-picker-role">{eh.role.as_str()}</span>
                                        <span class=action_class>{action_label}</span>
                                    </div>
                                </li>
                            }
                        }).collect_view()
                    }}
                </ul>
            </div>
        </Modal>
    }
}

#[cfg(test)]
mod tests {
    use wasm_bindgen_test::*;

    wasm_bindgen_test_configure!(run_in_browser);

    #[wasm_bindgen_test]
    fn test_household_picker_css_classes() {
        // Verify expected CSS class names
        assert_eq!("household-picker-list", "household-picker-list");
        assert_eq!("household-picker-item", "household-picker-item");
        assert_eq!("household-picker-name", "household-picker-name");
        assert_eq!("household-picker-role", "household-picker-role");
    }
}
