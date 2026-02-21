use chrono::NaiveDate;
use leptos::*;
use leptos_router::*;
use shared::{HierarchyType, HouseholdSettings, Role, UpdateHouseholdSettingsRequest};

use crate::api::ApiClient;
use crate::components::loading::Loading;
use crate::components::{
    Alert, AlertVariant, Button, ButtonVariant, Card, Divider, SectionHeader,
};
use crate::i18n::use_i18n;
use crate::utils::COMMON_TIMEZONES;

#[component]
pub fn HouseholdSettingsPage() -> impl IntoView {
    let i18n = use_i18n();
    let i18n_stored = store_value(i18n);

    let params = use_params_map();
    let household_id = move || params.with(|p| p.get("id").cloned().unwrap_or_default());

    let settings = create_rw_signal(Option::<HouseholdSettings>::None);
    let loading = create_rw_signal(true);
    let saving = create_rw_signal(false);
    let error = create_rw_signal(Option::<String>::None);
    let success = create_rw_signal(Option::<String>::None);

    // Check if current user is owner
    let is_owner = create_rw_signal(false);

    // Form state
    let dark_mode = create_rw_signal(false);
    let role_label_owner = create_rw_signal(String::new());
    let role_label_admin = create_rw_signal(String::new());
    let role_label_member = create_rw_signal(String::new());
    let hierarchy_type = create_rw_signal(HierarchyType::Organized);
    let timezone = create_rw_signal("UTC".to_string());
    let rewards_enabled = create_rw_signal(false);
    let punishments_enabled = create_rw_signal(false);
    let chat_enabled = create_rw_signal(false);
    let vacation_mode = create_rw_signal(false);
    let vacation_start = create_rw_signal(Option::<NaiveDate>::None);
    let vacation_end = create_rw_signal(Option::<NaiveDate>::None);
    let auto_archive_days = create_rw_signal(Option::<i32>::Some(7));
    let allow_task_suggestions = create_rw_signal(true);
    let week_start_day = create_rw_signal(0i32); // 0 = Monday

    // Load settings and check permissions
    create_effect(move |_| {
        let id = household_id();
        if id.is_empty() {
            return;
        }

        let id_for_settings = id.clone();
        let id_for_members = id.clone();

        // Load settings
        wasm_bindgen_futures::spawn_local(async move {
            match ApiClient::get_household_settings(&id_for_settings).await {
                Ok(s) => {
                    dark_mode.set(s.dark_mode);
                    role_label_owner.set(s.role_label_owner.clone());
                    role_label_admin.set(s.role_label_admin.clone());
                    role_label_member.set(s.role_label_member.clone());
                    hierarchy_type.set(s.hierarchy_type);
                    timezone.set(s.timezone.clone());
                    rewards_enabled.set(s.rewards_enabled);
                    punishments_enabled.set(s.punishments_enabled);
                    chat_enabled.set(s.chat_enabled);
                    vacation_mode.set(s.vacation_mode);
                    vacation_start.set(s.vacation_start);
                    vacation_end.set(s.vacation_end);
                    auto_archive_days.set(s.auto_archive_days);
                    allow_task_suggestions.set(s.allow_task_suggestions);
                    week_start_day.set(s.week_start_day);
                    settings.set(Some(s));
                }
                Err(e) => error.set(Some(e)),
            }
            loading.set(false);
        });

        // Check if current user is owner
        wasm_bindgen_futures::spawn_local(async move {
            if let Ok(members) = ApiClient::list_members(&id_for_members).await {
                if let Ok(current_user) = ApiClient::get_current_user().await {
                    let owner = members.iter().any(|m| {
                        m.user.id == current_user.id && m.membership.role == Role::Owner
                    });
                    is_owner.set(owner);
                }
            }
        });
    });

    let on_save = move |ev: web_sys::SubmitEvent| {
        ev.prevent_default();

        let id = household_id();
        saving.set(true);
        error.set(None);
        success.set(None);

        let request = UpdateHouseholdSettingsRequest {
            dark_mode: Some(dark_mode.get()),
            role_label_owner: Some(role_label_owner.get()),
            role_label_admin: Some(role_label_admin.get()),
            role_label_member: Some(role_label_member.get()),
            hierarchy_type: Some(hierarchy_type.get()),
            timezone: Some(timezone.get()),
            rewards_enabled: Some(rewards_enabled.get()),
            punishments_enabled: Some(punishments_enabled.get()),
            chat_enabled: Some(chat_enabled.get()),
            vacation_mode: Some(vacation_mode.get()),
            vacation_start: Some(vacation_start.get()),
            vacation_end: Some(vacation_end.get()),
            auto_archive_days: Some(auto_archive_days.get()),
            allow_task_suggestions: Some(allow_task_suggestions.get()),
            week_start_day: Some(week_start_day.get()),
        };

        wasm_bindgen_futures::spawn_local(async move {
            match ApiClient::update_household_settings(&id, request).await {
                Ok(s) => {
                    settings.set(Some(s.clone()));
                    success.set(Some(i18n_stored.get_value().t("settings.saved")));

                    // Apply dark mode immediately
                    apply_dark_mode(s.dark_mode);
                }
                Err(e) => {
                    error.set(Some(e));
                }
            }
            saving.set(false);
        });
    };

    view! {
        <div class="dashboard-header">
            <h1 class="dashboard-title">{i18n_stored.get_value().t("settings.household_settings")}</h1>
        </div>

        <Show when=move || loading.get() fallback=|| ()>
            <Loading />
        </Show>

        <Show when=move || !loading.get() fallback=|| ()>
            {move || error.get().map(|e| view! {
                <Alert variant=AlertVariant::Error>{e}</Alert>
            })}

            {move || success.get().map(|s| view! {
                <Alert variant=AlertVariant::Success>{s}</Alert>
            })}

            <Card>
                <Show
                    when=move || is_owner.get()
                    fallback=move || view! {
                        <div class="empty-state">
                            <p>{i18n_stored.get_value().t("settings.owner_only")}</p>
                        </div>
                    }
                >
                    <form on:submit=on_save>
                        <div class="form-group">
                            <label class="form-label" for="hierarchy-type">{i18n_stored.get_value().t("settings.household_structure")}</label>
                            <select
                                id="hierarchy-type"
                                class="form-select"
                                on:change=move |ev| {
                                    let value = event_target_value(&ev);
                                    let ht = match value.as_str() {
                                        "equals" => HierarchyType::Equals,
                                        "hierarchy" => HierarchyType::Hierarchy,
                                        _ => HierarchyType::Organized,
                                    };
                                    hierarchy_type.set(ht);
                                }
                            >
                                <option value="equals" selected=move || hierarchy_type.get() == HierarchyType::Equals>
                                    {i18n_stored.get_value().t("hierarchy.equals")} " - " {i18n_stored.get_value().t("hierarchy.equals_desc")}
                                </option>
                                <option value="organized" selected=move || hierarchy_type.get() == HierarchyType::Organized>
                                    {i18n_stored.get_value().t("hierarchy.organized")} " - " {i18n_stored.get_value().t("hierarchy.organized_desc")}
                                </option>
                                <option value="hierarchy" selected=move || hierarchy_type.get() == HierarchyType::Hierarchy>
                                    {i18n_stored.get_value().t("hierarchy.hierarchy")} " - " {i18n_stored.get_value().t("hierarchy.hierarchy_desc")}
                                </option>
                            </select>
                            <small class="form-hint">{i18n_stored.get_value().t("settings.structure_hint")}</small>
                        </div>

                        <Divider />

                        <div class="form-group">
                            <label class="form-label" for="timezone">{i18n_stored.get_value().t("settings.timezone")}</label>
                            <select
                                id="timezone"
                                class="form-select"
                                on:change=move |ev| {
                                    timezone.set(event_target_value(&ev));
                                }
                            >
                                {COMMON_TIMEZONES.iter().map(|(tz_id, tz_name)| {
                                    let tz_id = *tz_id;
                                    let tz_name = *tz_name;
                                    view! {
                                        <option
                                            value=tz_id
                                            selected=move || timezone.get() == tz_id
                                        >
                                            {tz_name}
                                        </option>
                                    }
                                }).collect_view()}
                            </select>
                            <small class="form-hint">{i18n_stored.get_value().t("settings.timezone_hint")}</small>
                        </div>

                        <div class="form-group">
                            <label class="form-label" for="week_start_day">{i18n_stored.get_value().t("settings.week_start_day")}</label>
                            <select
                                id="week_start_day"
                                class="form-select"
                                on:change=move |ev| {
                                    if let Ok(val) = event_target_value(&ev).parse::<i32>() {
                                        week_start_day.set(val);
                                    }
                                }
                            >
                                <option value="0" selected=move || week_start_day.get() == 0>{i18n_stored.get_value().t("weekday.monday")}</option>
                                <option value="1" selected=move || week_start_day.get() == 1>{i18n_stored.get_value().t("weekday.tuesday")}</option>
                                <option value="2" selected=move || week_start_day.get() == 2>{i18n_stored.get_value().t("weekday.wednesday")}</option>
                                <option value="3" selected=move || week_start_day.get() == 3>{i18n_stored.get_value().t("weekday.thursday")}</option>
                                <option value="4" selected=move || week_start_day.get() == 4>{i18n_stored.get_value().t("weekday.friday")}</option>
                                <option value="5" selected=move || week_start_day.get() == 5>{i18n_stored.get_value().t("weekday.saturday")}</option>
                                <option value="6" selected=move || week_start_day.get() == 6>{i18n_stored.get_value().t("weekday.sunday")}</option>
                            </select>
                            <small class="form-hint">{i18n_stored.get_value().t("settings.week_start_day_hint")}</small>
                        </div>

                        <Divider />

                        <div class="form-group">
                            <label class="form-label">{i18n_stored.get_value().t("settings.theme")}</label>
                            <div style="display: flex; align-items: center; gap: 0.5rem;">
                                <input
                                    type="checkbox"
                                    id="dark-mode"
                                    prop:checked=move || dark_mode.get()
                                    on:change=move |ev| {
                                        dark_mode.set(event_target_checked(&ev));
                                    }
                                />
                                <label for="dark-mode">{i18n_stored.get_value().t("settings.enable_dark_mode")}</label>
                            </div>
                            <small class="form-hint">{i18n_stored.get_value().t("settings.dark_mode_hint")}</small>
                        </div>

                        <Divider />

                        <SectionHeader>{i18n_stored.get_value().t("settings.optional_features")}</SectionHeader>

                        <div class="form-group">
                            <div style="display: flex; align-items: center; gap: 0.5rem;">
                                <input
                                    type="checkbox"
                                    id="rewards-enabled"
                                    prop:checked=move || rewards_enabled.get()
                                    on:change=move |ev| {
                                        rewards_enabled.set(event_target_checked(&ev));
                                    }
                                />
                                <label for="rewards-enabled">{i18n_stored.get_value().t("settings.enable_rewards")}</label>
                            </div>
                            <small class="form-hint">{i18n_stored.get_value().t("settings.rewards_hint")}</small>
                        </div>

                        <div class="form-group">
                            <div style="display: flex; align-items: center; gap: 0.5rem;">
                                <input
                                    type="checkbox"
                                    id="punishments-enabled"
                                    prop:checked=move || punishments_enabled.get()
                                    on:change=move |ev| {
                                        punishments_enabled.set(event_target_checked(&ev));
                                    }
                                />
                                <label for="punishments-enabled">{i18n_stored.get_value().t("settings.enable_punishments")}</label>
                            </div>
                            <small class="form-hint">{i18n_stored.get_value().t("settings.punishments_hint")}</small>
                        </div>

                        <div class="form-group">
                            <div style="display: flex; align-items: center; gap: 0.5rem;">
                                <input
                                    type="checkbox"
                                    id="chat-enabled"
                                    prop:checked=move || chat_enabled.get()
                                    on:change=move |ev| {
                                        chat_enabled.set(event_target_checked(&ev));
                                    }
                                />
                                <label for="chat-enabled">{i18n_stored.get_value().t("settings.enable_chat")}</label>
                            </div>
                            <small class="form-hint">{i18n_stored.get_value().t("settings.chat_hint")}</small>
                        </div>

                        <div class="form-group">
                            <div style="display: flex; align-items: center; gap: 0.5rem;">
                                <input
                                    type="checkbox"
                                    id="allow-task-suggestions"
                                    prop:checked=move || allow_task_suggestions.get()
                                    on:change=move |ev| {
                                        allow_task_suggestions.set(event_target_checked(&ev));
                                    }
                                />
                                <label for="allow-task-suggestions">{i18n_stored.get_value().t("settings.allow_task_suggestions")}</label>
                            </div>
                            <small class="form-hint">{i18n_stored.get_value().t("settings.task_suggestions_hint")}</small>
                        </div>

                        <Divider />

                        <SectionHeader>{i18n_stored.get_value().t("settings.vacation_mode")}</SectionHeader>

                        <div class="form-group">
                            <div style="display: flex; align-items: center; gap: 0.5rem;">
                                <input
                                    type="checkbox"
                                    id="vacation-mode"
                                    prop:checked=move || vacation_mode.get()
                                    on:change=move |ev| {
                                        vacation_mode.set(event_target_checked(&ev));
                                    }
                                />
                                <label for="vacation-mode">{i18n_stored.get_value().t("settings.enable_vacation_mode")}</label>
                            </div>
                            <small class="form-hint">{i18n_stored.get_value().t("settings.vacation_mode_hint")}</small>
                        </div>

                        <Show when=move || vacation_mode.get() fallback=|| ()>
                            <div style="margin-left: 1.5rem; padding-left: 1rem; border-left: 2px solid var(--border-color);">
                                <div class="form-group">
                                    <label class="form-label" for="vacation-start">{i18n_stored.get_value().t("settings.vacation_start")}</label>
                                    <input
                                        type="date"
                                        id="vacation-start"
                                        class="form-input"
                                        prop:value=move || vacation_start.get().map(|d| d.to_string()).unwrap_or_default()
                                        on:input=move |ev| {
                                            let value = event_target_value(&ev);
                                            if value.is_empty() {
                                                vacation_start.set(None);
                                            } else if let Ok(date) = NaiveDate::parse_from_str(&value, "%Y-%m-%d") {
                                                vacation_start.set(Some(date));
                                            }
                                        }
                                    />
                                    <small class="form-hint">{i18n_stored.get_value().t("settings.vacation_start_hint")}</small>
                                </div>

                                <div class="form-group">
                                    <label class="form-label" for="vacation-end">{i18n_stored.get_value().t("settings.vacation_end")}</label>
                                    <input
                                        type="date"
                                        id="vacation-end"
                                        class="form-input"
                                        prop:value=move || vacation_end.get().map(|d| d.to_string()).unwrap_or_default()
                                        on:input=move |ev| {
                                            let value = event_target_value(&ev);
                                            if value.is_empty() {
                                                vacation_end.set(None);
                                            } else if let Ok(date) = NaiveDate::parse_from_str(&value, "%Y-%m-%d") {
                                                vacation_end.set(Some(date));
                                            }
                                        }
                                    />
                                    <small class="form-hint">{i18n_stored.get_value().t("settings.vacation_end_hint")}</small>
                                </div>
                            </div>
                        </Show>

                        <Divider />

                        <SectionHeader>{i18n_stored.get_value().t("settings.task_cleanup")}</SectionHeader>

                        <div class="form-group">
                            <div style="display: flex; align-items: center; gap: 0.5rem;">
                                <input
                                    type="checkbox"
                                    id="auto-archive-enabled"
                                    prop:checked=move || auto_archive_days.get().map(|d| d > 0).unwrap_or(false)
                                    on:change=move |ev| {
                                        let checked = event_target_checked(&ev);
                                        if checked {
                                            auto_archive_days.set(Some(7));
                                        } else {
                                            auto_archive_days.set(None);
                                        }
                                    }
                                />
                                <label for="auto-archive-enabled">{i18n_stored.get_value().t("settings.enable_auto_archive")}</label>
                            </div>
                            <small class="form-hint">{i18n_stored.get_value().t("settings.auto_archive_hint")}</small>
                        </div>

                        <Show when=move || auto_archive_days.get().map(|d| d > 0).unwrap_or(false) fallback=|| ()>
                            <div style="margin-left: 1.5rem; padding-left: 1rem; border-left: 2px solid var(--border-color);">
                                <div class="form-group">
                                    <label class="form-label" for="auto-archive-days">{i18n_stored.get_value().t("settings.auto_archive_days")}</label>
                                    <input
                                        type="number"
                                        id="auto-archive-days"
                                        class="form-input"
                                        min="1"
                                        max="90"
                                        prop:value=move || auto_archive_days.get().unwrap_or(7).to_string()
                                        on:input=move |ev| {
                                            let value = event_target_value(&ev);
                                            if let Ok(days) = value.parse::<i32>() {
                                                if (1..=90).contains(&days) {
                                                    auto_archive_days.set(Some(days));
                                                }
                                            }
                                        }
                                    />
                                    <small class="form-hint">{i18n_stored.get_value().t("settings.auto_archive_days_hint")}</small>
                                </div>
                            </div>
                        </Show>

                        <Divider />

                        <SectionHeader>{i18n_stored.get_value().t("settings.custom_role_labels")}</SectionHeader>
                        <p style="color: var(--text-muted); margin-bottom: 1rem; font-size: 0.875rem;">
                            {i18n_stored.get_value().t("settings.role_labels_hint")}
                        </p>

                        <div class="form-group">
                            <label class="form-label" for="label-owner">{i18n_stored.get_value().t("settings.owner_label")}</label>
                            <input
                                type="text"
                                id="label-owner"
                                class="form-input"
                                placeholder=i18n_stored.get_value().t("roles.owner")
                                prop:value=move || role_label_owner.get()
                                on:input=move |ev| role_label_owner.set(event_target_value(&ev))
                            />
                        </div>

                        <div class="form-group">
                            <label class="form-label" for="label-admin">{i18n_stored.get_value().t("settings.admin_label")}</label>
                            <input
                                type="text"
                                id="label-admin"
                                class="form-input"
                                placeholder=i18n_stored.get_value().t("roles.admin")
                                prop:value=move || role_label_admin.get()
                                on:input=move |ev| role_label_admin.set(event_target_value(&ev))
                            />
                        </div>

                        <div class="form-group">
                            <label class="form-label" for="label-member">{i18n_stored.get_value().t("settings.member_label")}</label>
                            <input
                                type="text"
                                id="label-member"
                                class="form-input"
                                placeholder=i18n_stored.get_value().t("roles.member")
                                prop:value=move || role_label_member.get()
                                on:input=move |ev| role_label_member.set(event_target_value(&ev))
                            />
                        </div>

                        <div style="margin-top: 2rem;">
                            <Button
                                variant=ButtonVariant::Primary
                                button_type="submit"
                                disabled=MaybeSignal::derive(move || saving.get())
                            >
                                {move || if saving.get() { i18n_stored.get_value().t("common.saving") } else { i18n_stored.get_value().t("settings.save_settings") }}
                            </Button>
                        </div>
                    </form>
                </Show>
            </Card>
        </Show>
    }
}

/// Apply dark mode class to document body
fn apply_dark_mode(enabled: bool) {
    if let Some(window) = web_sys::window() {
        if let Some(document) = window.document() {
            if let Some(body) = document.body() {
                if enabled {
                    let _ = body.class_list().add_1("dark-mode");
                } else {
                    let _ = body.class_list().remove_1("dark-mode");
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use wasm_bindgen_test::*;

    wasm_bindgen_test_configure!(run_in_browser);

    #[wasm_bindgen_test]
    fn test_default_role_labels() {
        let owner = "Owner";
        let admin = "Admin";
        let member = "Member";
        assert_eq!(owner, "Owner");
        assert_eq!(admin, "Admin");
        assert_eq!(member, "Member");
    }

    #[wasm_bindgen_test]
    fn test_custom_role_labels() {
        let owner = "Parent";
        let admin = "Guardian";
        let member = "Child";
        assert_eq!(owner, "Parent");
        assert_eq!(admin, "Guardian");
        assert_eq!(member, "Child");
    }
}
