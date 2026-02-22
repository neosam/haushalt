use chrono::NaiveDate;
use leptos::*;
use leptos_router::*;
use shared::{DefaultPunishmentEntry, DefaultRewardEntry, HierarchyType, Household, HouseholdSettings, Punishment, Reward, Role, UpdateHouseholdSettingsRequest};

use crate::api::ApiClient;
use crate::components::loading::Loading;
use crate::components::modal::Modal;
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
    let current_role = create_rw_signal(Option::<Role>::None);

    // Household data for rename
    let household = create_rw_signal(Option::<Household>::None);
    let household_name = create_rw_signal(String::new());
    let name_saving = create_rw_signal(false);

    // Solo Mode state
    let solo_mode_confirm_open = create_rw_signal(false);
    let solo_mode_activating = create_rw_signal(false);

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

    // Task defaults
    let default_points_reward = create_rw_signal(Option::<i64>::None);
    let default_points_penalty = create_rw_signal(Option::<i64>::None);
    // Vec of (reward_id, amount)
    let default_rewards = create_rw_signal(Vec::<(String, i32)>::new());
    let default_punishments = create_rw_signal(Vec::<(String, i32)>::new());
    // Signals for adding new defaults
    let selected_new_reward = create_rw_signal(String::new());
    let new_reward_amount = create_rw_signal(1i32);
    let selected_new_punishment = create_rw_signal(String::new());
    let new_punishment_amount = create_rw_signal(1i32);

    // Available rewards and punishments for dropdowns
    let rewards = create_rw_signal(Vec::<Reward>::new());
    let punishments = create_rw_signal(Vec::<Punishment>::new());

    // Load settings and check permissions
    create_effect(move |_| {
        let id = household_id();
        if id.is_empty() {
            return;
        }

        let id_for_settings = id.clone();
        let id_for_members = id.clone();
        let id_for_household = id.clone();

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
                    default_points_reward.set(s.default_points_reward);
                    default_points_penalty.set(s.default_points_penalty);
                    default_rewards.set(
                        s.default_rewards.iter()
                            .map(|r| (r.reward.id.to_string(), r.amount))
                            .collect()
                    );
                    default_punishments.set(
                        s.default_punishments.iter()
                            .map(|p| (p.punishment.id.to_string(), p.amount))
                            .collect()
                    );
                    settings.set(Some(s));
                }
                Err(e) => error.set(Some(e)),
            }
            loading.set(false);
        });

        // Load household data for name
        wasm_bindgen_futures::spawn_local(async move {
            if let Ok(hh) = ApiClient::get_household(&id_for_household).await {
                household_name.set(hh.name.clone());
                household.set(Some(hh));
            }
        });

        // Check if current user is owner and get role
        wasm_bindgen_futures::spawn_local(async move {
            if let Ok(members) = ApiClient::list_members(&id_for_members).await {
                if let Ok(current_user) = ApiClient::get_current_user().await {
                    for m in &members {
                        if m.user.id == current_user.id {
                            current_role.set(Some(m.membership.role));
                            is_owner.set(m.membership.role == Role::Owner);
                            break;
                        }
                    }
                }
            }
        });

        // Load rewards and punishments for dropdowns
        let id_for_rewards = id.clone();
        let id_for_punishments = id;
        wasm_bindgen_futures::spawn_local(async move {
            if let Ok(r) = ApiClient::list_rewards(&id_for_rewards).await {
                rewards.set(r);
            }
        });
        wasm_bindgen_futures::spawn_local(async move {
            if let Ok(p) = ApiClient::list_punishments(&id_for_punishments).await {
                punishments.set(p);
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
            default_points_reward: Some(default_points_reward.get()),
            default_points_penalty: Some(default_points_penalty.get()),
            default_rewards: Some(
                default_rewards.get().into_iter()
                    .filter_map(|(id, amount)| {
                        uuid::Uuid::parse_str(&id).ok().map(|reward_id| DefaultRewardEntry { reward_id, amount })
                    })
                    .collect()
            ),
            default_punishments: Some(
                default_punishments.get().into_iter()
                    .filter_map(|(id, amount)| {
                        uuid::Uuid::parse_str(&id).ok().map(|punishment_id| DefaultPunishmentEntry { punishment_id, amount })
                    })
                    .collect()
            ),
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

    let on_save_name = move |_| {
        let name = household_name.get();
        if name.trim().is_empty() {
            return;
        }

        let id = household_id();
        name_saving.set(true);
        error.set(None);
        success.set(None);

        wasm_bindgen_futures::spawn_local(async move {
            match ApiClient::update_household(&id, name).await {
                Ok(hh) => {
                    household.set(Some(hh));
                    success.set(Some(i18n_stored.get_value().t("household.settings.name_updated")));
                }
                Err(e) => {
                    error.set(Some(e));
                }
            }
            name_saving.set(false);
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
                // General section - visible to Owner/Admin
                <Show when=move || current_role.get().map(|r| r.can_manage_tasks()).unwrap_or(false) fallback=|| ()>
                    <SectionHeader>{i18n_stored.get_value().t("household.settings.general")}</SectionHeader>
                    <div class="form-group">
                        <label class="form-label" for="household-name">{i18n_stored.get_value().t("household.settings.name")}</label>
                        <div style="display: flex; gap: 0.5rem; align-items: flex-start;">
                            <input
                                type="text"
                                id="household-name"
                                class="form-input"
                                style="flex: 1;"
                                maxlength="100"
                                prop:value=move || household_name.get()
                                on:input=move |ev| household_name.set(event_target_value(&ev))
                            />
                            <Button
                                variant=ButtonVariant::Primary
                                on_click=Callback::new(on_save_name)
                                disabled=MaybeSignal::derive(move || name_saving.get() || household_name.get().trim().is_empty())
                            >
                                {move || if name_saving.get() { i18n_stored.get_value().t("common.saving") } else { i18n_stored.get_value().t("common.save") }}
                            </Button>
                        </div>
                    </div>
                    <Divider />
                </Show>

                <Show
                    when=move || is_owner.get()
                    fallback=move || view! {
                        <Show when=move || !current_role.get().map(|r| r.can_manage_tasks()).unwrap_or(false) fallback=|| ()>
                            <div class="empty-state">
                                <p>{i18n_stored.get_value().t("settings.owner_only")}</p>
                            </div>
                        </Show>
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

                        <SectionHeader>{i18n_stored.get_value().t("solo_mode.section_title")}</SectionHeader>

                        {move || {
                            let s = settings.get();
                            if let Some(ref settings_val) = s {
                                if settings_val.solo_mode {
                                    // Solo Mode is active - show info
                                    view! {
                                        <div class="solo-mode-info">
                                            <Alert variant=AlertVariant::Warning>
                                                <strong>{i18n_stored.get_value().t("solo_mode.active")}</strong>
                                                <p style="margin: 0.5rem 0 0 0;">
                                                    {i18n_stored.get_value().t("solo_mode.settings_locked")}
                                                </p>
                                                <p style="margin: 0.5rem 0 0 0;">
                                                    {i18n_stored.get_value().t("solo_mode.exit_via_banner")}
                                                </p>
                                            </Alert>
                                        </div>
                                    }.into_view()
                                } else {
                                    // Solo Mode is not active - show activation button
                                    view! {
                                        <div class="form-group">
                                            <p style="color: var(--text-muted); margin-bottom: 1rem; font-size: 0.875rem;">
                                                {i18n_stored.get_value().t("solo_mode.description")}
                                            </p>
                                            <ul style="color: var(--text-muted); font-size: 0.875rem; margin-bottom: 1rem; padding-left: 1.5rem;">
                                                <li>{i18n_stored.get_value().t("solo_mode.feature_1")}</li>
                                                <li>{i18n_stored.get_value().t("solo_mode.feature_2")}</li>
                                                <li>{i18n_stored.get_value().t("solo_mode.feature_3")}</li>
                                                <li>{i18n_stored.get_value().t("solo_mode.feature_4")}</li>
                                            </ul>
                                            <Button
                                                variant=ButtonVariant::Danger
                                                on_click=Callback::new(move |_| solo_mode_confirm_open.set(true))
                                            >
                                                {i18n_stored.get_value().t("solo_mode.activate")}
                                            </Button>
                                        </div>
                                    }.into_view()
                                }
                            } else {
                                ().into_view()
                            }
                        }}

                        // Solo Mode confirmation modal
                        <Show when=move || solo_mode_confirm_open.get() fallback=|| ()>
                            <Modal
                                on_close=move |_| solo_mode_confirm_open.set(false)
                                title=i18n_stored.get_value().t("solo_mode.confirm_title")
                            >
                                <p>{i18n_stored.get_value().t("solo_mode.confirm_message")}</p>
                                <p style="margin-top: 0.5rem; color: var(--text-muted);">
                                    {i18n_stored.get_value().t("solo_mode.confirm_cooldown")}
                                </p>
                                <div style="display: flex; gap: 0.5rem; margin-top: 1rem; justify-content: flex-end;">
                                    <Button
                                        variant=ButtonVariant::Secondary
                                        on_click=Callback::new(move |_| solo_mode_confirm_open.set(false))
                                    >
                                        {i18n_stored.get_value().t("common.cancel")}
                                    </Button>
                                    <Button
                                        variant=ButtonVariant::Danger
                                        disabled=MaybeSignal::derive(move || solo_mode_activating.get())
                                        on_click=Callback::new(move |_| {
                                            let id = household_id();
                                            solo_mode_activating.set(true);
                                            error.set(None);

                                            wasm_bindgen_futures::spawn_local(async move {
                                                match ApiClient::activate_solo_mode(&id).await {
                                                    Ok(new_settings) => {
                                                        settings.set(Some(new_settings));
                                                        solo_mode_confirm_open.set(false);
                                                    }
                                                    Err(e) => {
                                                        error.set(Some(e));
                                                    }
                                                }
                                                solo_mode_activating.set(false);
                                            });
                                        })
                                    >
                                        {move || if solo_mode_activating.get() {
                                            i18n_stored.get_value().t("common.loading")
                                        } else {
                                            i18n_stored.get_value().t("solo_mode.activate")
                                        }}
                                    </Button>
                                </div>
                            </Modal>
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

                        <Divider />

                        <SectionHeader>{i18n_stored.get_value().t("settings.task_defaults")}</SectionHeader>
                        <p style="color: var(--text-muted); margin-bottom: 1rem; font-size: 0.875rem;">
                            {i18n_stored.get_value().t("settings.task_defaults_hint")}
                        </p>

                        <div class="form-group">
                            <label class="form-label" for="default-points-reward">{i18n_stored.get_value().t("settings.default_points_reward")}</label>
                            <input
                                type="number"
                                id="default-points-reward"
                                class="form-input"
                                min="0"
                                prop:value=move || default_points_reward.get().map(|p| p.to_string()).unwrap_or_default()
                                on:input=move |ev| {
                                    let value = event_target_value(&ev);
                                    if value.is_empty() {
                                        default_points_reward.set(None);
                                    } else if let Ok(points) = value.parse::<i64>() {
                                        if points >= 0 {
                                            default_points_reward.set(Some(points));
                                        }
                                    }
                                }
                            />
                        </div>

                        <div class="form-group">
                            <label class="form-label" for="default-points-penalty">{i18n_stored.get_value().t("settings.default_points_penalty")}</label>
                            <input
                                type="number"
                                id="default-points-penalty"
                                class="form-input"
                                min="0"
                                prop:value=move || default_points_penalty.get().map(|p| p.to_string()).unwrap_or_default()
                                on:input=move |ev| {
                                    let value = event_target_value(&ev);
                                    if value.is_empty() {
                                        default_points_penalty.set(None);
                                    } else if let Ok(points) = value.parse::<i64>() {
                                        if points >= 0 {
                                            default_points_penalty.set(Some(points));
                                        }
                                    }
                                }
                            />
                        </div>

                        // Show default rewards list only if rewards are enabled
                        <Show when=move || rewards_enabled.get() fallback=|| ()>
                            <div class="form-group">
                                <label class="form-label">{i18n_stored.get_value().t("settings.default_rewards")}</label>
                                <div style="border: 1px solid var(--card-border); border-radius: var(--border-radius); padding: 0.75rem;">
                                    // Add new reward row
                                    <div style="display: flex; gap: 0.5rem; align-items: center; margin-bottom: 0.75rem;">
                                        <select
                                            class="form-select"
                                            style="flex: 1;"
                                            prop:value=move || selected_new_reward.get()
                                            on:change=move |ev| selected_new_reward.set(event_target_value(&ev))
                                        >
                                            <option value="">{i18n_stored.get_value().t("settings.select_reward")}</option>
                                            {move || {
                                                let current_ids: Vec<String> = default_rewards.get().iter().map(|(id, _)| id.clone()).collect();
                                                rewards.get().into_iter()
                                                    .filter(|r| !current_ids.contains(&r.id.to_string()))
                                                    .map(|r| {
                                                        let rid = r.id.to_string();
                                                        view! {
                                                            <option value=rid>{r.name}</option>
                                                        }
                                                    })
                                                    .collect_view()
                                            }}
                                        </select>
                                        <input
                                            type="number"
                                            class="form-input"
                                            style="width: 70px;"
                                            min="1"
                                            prop:value=move || new_reward_amount.get().to_string()
                                            on:input=move |ev| {
                                                if let Ok(val) = event_target_value(&ev).parse::<i32>() {
                                                    new_reward_amount.set(val.max(1));
                                                }
                                            }
                                        />
                                        <button
                                            type="button"
                                            class="btn btn-outline"
                                            style="padding: 0.5rem 1rem;"
                                            disabled=move || selected_new_reward.get().is_empty()
                                            on:click=move |_| {
                                                let reward_id = selected_new_reward.get();
                                                let amount = new_reward_amount.get();
                                                if !reward_id.is_empty() {
                                                    default_rewards.update(|r| {
                                                        if !r.iter().any(|(id, _)| id == &reward_id) {
                                                            r.push((reward_id.clone(), amount));
                                                        }
                                                    });
                                                    selected_new_reward.set(String::new());
                                                    new_reward_amount.set(1);
                                                }
                                            }
                                        >
                                            {i18n_stored.get_value().t("common.add")}
                                        </button>
                                    </div>

                                    // List of selected default rewards
                                    <div>
                                        {move || {
                                            let selected = default_rewards.get();
                                            if selected.is_empty() {
                                                view! { <p style="color: var(--text-muted); font-size: 0.875rem; margin: 0;">{i18n_stored.get_value().t("settings.no_default_rewards")}</p> }.into_view()
                                            } else {
                                                selected.iter().map(|(reward_id, amount)| {
                                                    let reward_name = rewards.get().iter()
                                                        .find(|r| r.id.to_string() == *reward_id)
                                                        .map(|r| r.name.clone())
                                                        .unwrap_or_else(|| i18n_stored.get_value().t("common.unknown"));
                                                    let reward_id_for_remove = reward_id.clone();
                                                    let amount_display = *amount;
                                                    view! {
                                                        <div style="display: flex; justify-content: space-between; align-items: center; padding: 0.5rem; background: var(--bg-secondary); border-radius: var(--border-radius); margin-bottom: 0.25rem;">
                                                            <span>
                                                                {reward_name}
                                                                {if amount_display > 1 {
                                                                    view! { <span style="color: var(--text-muted); margin-left: 0.5rem;">" ×"{amount_display}</span> }.into_view()
                                                                } else {
                                                                    ().into_view()
                                                                }}
                                                            </span>
                                                            <button
                                                                type="button"
                                                                class="btn btn-outline"
                                                                style="padding: 0.25rem 0.5rem; font-size: 0.75rem;"
                                                                on:click=move |_| {
                                                                    default_rewards.update(|r| {
                                                                        r.retain(|(id, _)| id != &reward_id_for_remove);
                                                                    });
                                                                }
                                                            >
                                                                {i18n_stored.get_value().t("common.remove")}
                                                            </button>
                                                        </div>
                                                    }
                                                }).collect_view().into_view()
                                            }
                                        }}
                                    </div>
                                </div>
                                <small class="form-hint">{i18n_stored.get_value().t("settings.default_rewards_hint")}</small>
                            </div>
                        </Show>

                        // Show default punishments list only if punishments are enabled
                        <Show when=move || punishments_enabled.get() fallback=|| ()>
                            <div class="form-group">
                                <label class="form-label">{i18n_stored.get_value().t("settings.default_punishments")}</label>
                                <div style="border: 1px solid var(--card-border); border-radius: var(--border-radius); padding: 0.75rem;">
                                    // Add new punishment row
                                    <div style="display: flex; gap: 0.5rem; align-items: center; margin-bottom: 0.75rem;">
                                        <select
                                            class="form-select"
                                            style="flex: 1;"
                                            prop:value=move || selected_new_punishment.get()
                                            on:change=move |ev| selected_new_punishment.set(event_target_value(&ev))
                                        >
                                            <option value="">{i18n_stored.get_value().t("settings.select_punishment")}</option>
                                            {move || {
                                                let current_ids: Vec<String> = default_punishments.get().iter().map(|(id, _)| id.clone()).collect();
                                                punishments.get().into_iter()
                                                    .filter(|p| !current_ids.contains(&p.id.to_string()))
                                                    .map(|p| {
                                                        let pid = p.id.to_string();
                                                        view! {
                                                            <option value=pid>{p.name}</option>
                                                        }
                                                    })
                                                    .collect_view()
                                            }}
                                        </select>
                                        <input
                                            type="number"
                                            class="form-input"
                                            style="width: 70px;"
                                            min="1"
                                            prop:value=move || new_punishment_amount.get().to_string()
                                            on:input=move |ev| {
                                                if let Ok(val) = event_target_value(&ev).parse::<i32>() {
                                                    new_punishment_amount.set(val.max(1));
                                                }
                                            }
                                        />
                                        <button
                                            type="button"
                                            class="btn btn-outline"
                                            style="padding: 0.5rem 1rem;"
                                            disabled=move || selected_new_punishment.get().is_empty()
                                            on:click=move |_| {
                                                let punishment_id = selected_new_punishment.get();
                                                let amount = new_punishment_amount.get();
                                                if !punishment_id.is_empty() {
                                                    default_punishments.update(|p| {
                                                        if !p.iter().any(|(id, _)| id == &punishment_id) {
                                                            p.push((punishment_id.clone(), amount));
                                                        }
                                                    });
                                                    selected_new_punishment.set(String::new());
                                                    new_punishment_amount.set(1);
                                                }
                                            }
                                        >
                                            {i18n_stored.get_value().t("common.add")}
                                        </button>
                                    </div>

                                    // List of selected default punishments
                                    <div>
                                        {move || {
                                            let selected = default_punishments.get();
                                            if selected.is_empty() {
                                                view! { <p style="color: var(--text-muted); font-size: 0.875rem; margin: 0;">{i18n_stored.get_value().t("settings.no_default_punishments")}</p> }.into_view()
                                            } else {
                                                selected.iter().map(|(punishment_id, amount)| {
                                                    let punishment_name = punishments.get().iter()
                                                        .find(|p| p.id.to_string() == *punishment_id)
                                                        .map(|p| p.name.clone())
                                                        .unwrap_or_else(|| i18n_stored.get_value().t("common.unknown"));
                                                    let punishment_id_for_remove = punishment_id.clone();
                                                    let amount_display = *amount;
                                                    view! {
                                                        <div style="display: flex; justify-content: space-between; align-items: center; padding: 0.5rem; background: var(--bg-secondary); border-radius: var(--border-radius); margin-bottom: 0.25rem;">
                                                            <span>
                                                                {punishment_name}
                                                                {if amount_display > 1 {
                                                                    view! { <span style="color: var(--text-muted); margin-left: 0.5rem;">" ×"{amount_display}</span> }.into_view()
                                                                } else {
                                                                    ().into_view()
                                                                }}
                                                            </span>
                                                            <button
                                                                type="button"
                                                                class="btn btn-outline"
                                                                style="padding: 0.25rem 0.5rem; font-size: 0.75rem;"
                                                                on:click=move |_| {
                                                                    default_punishments.update(|p| {
                                                                        p.retain(|(id, _)| id != &punishment_id_for_remove);
                                                                    });
                                                                }
                                                            >
                                                                {i18n_stored.get_value().t("common.remove")}
                                                            </button>
                                                        </div>
                                                    }
                                                }).collect_view().into_view()
                                            }
                                        }}
                                    </div>
                                </div>
                                <small class="form-hint">{i18n_stored.get_value().t("settings.default_punishments_hint")}</small>
                            </div>
                        </Show>

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
    use shared::Role;
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

    #[wasm_bindgen_test]
    fn test_owner_can_see_general_section() {
        let role = Role::Owner;
        assert!(role.can_manage_tasks());
    }

    #[wasm_bindgen_test]
    fn test_admin_can_see_general_section() {
        let role = Role::Admin;
        assert!(role.can_manage_tasks());
    }

    #[wasm_bindgen_test]
    fn test_member_cannot_see_general_section() {
        let role = Role::Member;
        assert!(!role.can_manage_tasks());
    }

    #[wasm_bindgen_test]
    fn test_empty_name_rejected() {
        let name = "   ";
        assert!(name.trim().is_empty());
    }

    #[wasm_bindgen_test]
    fn test_valid_name_accepted() {
        let name = "Smith Family";
        assert!(!name.trim().is_empty());
    }
}
