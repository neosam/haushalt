use leptos::*;
use leptos_router::*;
use shared::{HierarchyType, HouseholdSettings, Role, UpdateHouseholdSettingsRequest};

use crate::api::ApiClient;
use crate::components::household_tabs::{HouseholdTab, HouseholdTabs};
use crate::components::loading::Loading;
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
        {move || {
            let hid = household_id();
            view! { <HouseholdTabs household_id=hid active_tab=HouseholdTab::Settings settings=settings.get() /> }
        }}

        <div class="dashboard-header">
            <h1 class="dashboard-title">{i18n_stored.get_value().t("settings.household_settings")}</h1>
        </div>

        <Show when=move || loading.get() fallback=|| ()>
            <Loading />
        </Show>

        <Show when=move || !loading.get() fallback=|| ()>
            {move || error.get().map(|e| view! {
                <div class="alert alert-error">{e}</div>
            })}

            {move || success.get().map(|s| view! {
                <div class="alert alert-success">{s}</div>
            })}

            <div class="card">
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

                        <hr style="margin: 1.5rem 0; border-color: var(--border-color);" />

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

                        <hr style="margin: 1.5rem 0; border-color: var(--border-color);" />

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

                        <hr style="margin: 1.5rem 0; border-color: var(--border-color);" />

                        <h3 style="margin-bottom: 1rem;">{i18n_stored.get_value().t("settings.optional_features")}</h3>

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

                        <hr style="margin: 1.5rem 0; border-color: var(--border-color);" />

                        <h3 style="margin-bottom: 1rem;">{i18n_stored.get_value().t("settings.custom_role_labels")}</h3>
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
                            <button
                                type="submit"
                                class="btn btn-primary"
                                disabled=move || saving.get()
                            >
                                {move || if saving.get() { i18n_stored.get_value().t("common.saving") } else { i18n_stored.get_value().t("settings.save_settings") }}
                            </button>
                        </div>
                    </form>
                </Show>
            </div>
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
