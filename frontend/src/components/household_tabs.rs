use leptos::*;

#[derive(Clone, Copy, PartialEq)]
pub enum HouseholdTab {
    Overview,
    Tasks,
    Rewards,
    Punishments,
}

impl HouseholdTab {
    fn label(&self) -> &'static str {
        match self {
            HouseholdTab::Overview => "Overview",
            HouseholdTab::Tasks => "Tasks",
            HouseholdTab::Rewards => "Rewards",
            HouseholdTab::Punishments => "Punishments",
        }
    }

    fn path(&self, household_id: &str) -> String {
        match self {
            HouseholdTab::Overview => format!("/households/{}", household_id),
            HouseholdTab::Tasks => format!("/households/{}/tasks", household_id),
            HouseholdTab::Rewards => format!("/households/{}/rewards", household_id),
            HouseholdTab::Punishments => format!("/households/{}/punishments", household_id),
        }
    }
}

#[component]
pub fn HouseholdTabs(
    household_id: String,
    active_tab: HouseholdTab,
) -> impl IntoView {
    let tabs = [
        HouseholdTab::Overview,
        HouseholdTab::Tasks,
        HouseholdTab::Rewards,
        HouseholdTab::Punishments,
    ];

    view! {
        <nav class="household-tabs">
            {tabs.into_iter().map(|tab| {
                let href = tab.path(&household_id);
                let is_active = tab == active_tab;
                let class = if is_active { "tab-link active" } else { "tab-link" };
                view! {
                    <a href=href class=class>
                        {tab.label()}
                    </a>
                }
            }).collect_view()}
        </nav>
    }
}
