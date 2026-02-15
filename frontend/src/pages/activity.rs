use leptos::*;
use leptos_router::*;
use shared::{ActivityLogWithUsers, ActivityType, HouseholdSettings};

use crate::api::ApiClient;
use crate::components::household_tabs::{HouseholdTab, HouseholdTabs};
use crate::components::loading::Loading;
use crate::i18n::{use_i18n, I18nContext};
use crate::utils::format_datetime;

#[component]
pub fn ActivityPage() -> impl IntoView {
    let i18n = use_i18n();
    let i18n_stored = store_value(i18n);

    let params = use_params_map();
    let household_id = move || params.with(|p| p.get("id").cloned().unwrap_or_default());

    let activities = create_rw_signal(Vec::<ActivityLogWithUsers>::new());
    let settings = create_rw_signal(Option::<HouseholdSettings>::None);
    let loading = create_rw_signal(true);
    let error = create_rw_signal(Option::<String>::None);

    // Load activities and settings
    create_effect(move |_| {
        let id = household_id();
        if id.is_empty() {
            return;
        }

        let id_for_activities = id.clone();
        let id_for_settings = id.clone();

        // Load activities
        wasm_bindgen_futures::spawn_local(async move {
            match ApiClient::list_activities(&id_for_activities, Some(100)).await {
                Ok(a) => {
                    activities.set(a);
                    loading.set(false);
                }
                Err(e) => {
                    error.set(Some(e));
                    loading.set(false);
                }
            }
        });

        // Load settings for dark mode
        wasm_bindgen_futures::spawn_local(async move {
            if let Ok(s) = ApiClient::get_household_settings(&id_for_settings).await {
                apply_dark_mode(s.dark_mode);
                settings.set(Some(s));
            }
        });
    });

    view! {
        {move || {
            let hid = household_id();
            view! { <HouseholdTabs household_id=hid active_tab=HouseholdTab::Activity settings=settings.get() /> }
        }}

        <div class="dashboard-header">
            <h1 class="dashboard-title">{i18n_stored.get_value().t("activity.title")}</h1>
        </div>

        {move || error.get().map(|e| view! {
            <div class="alert alert-error">{e}</div>
        })}

        <Show when=move || loading.get() fallback=|| ()>
            <Loading />
        </Show>

        <Show when=move || !loading.get() fallback=|| ()>
            {move || {
                let a = activities.get();
                if a.is_empty() {
                    view! {
                        <div class="card empty-state">
                            <p>{i18n_stored.get_value().t("activity.no_activity")}</p>
                            <p>{i18n_stored.get_value().t("activity.will_appear")}</p>
                        </div>
                    }.into_view()
                } else {
                    let tz = settings.get().map(|s| s.timezone).unwrap_or_else(|| "UTC".to_string());
                    view! {
                        <div class="card">
                            {a.into_iter().map(|activity| {
                                let description = format_activity_description(&activity, &i18n_stored.get_value());
                                let timestamp = format_datetime(activity.log.created_at, &tz);

                                view! {
                                    <div class="task-item">
                                        <div class="task-content">
                                            <div class="task-title">{description}</div>
                                            <div class="task-meta">{timestamp}</div>
                                        </div>
                                    </div>
                                }
                            }).collect_view()}
                        </div>
                    }.into_view()
                }
            }}
        </Show>
    }
}

/// Helper to replace placeholders in a translation string
fn replace_placeholders(template: &str, replacements: &[(&str, &str)]) -> String {
    let mut result = template.to_string();
    for (placeholder, value) in replacements {
        result = result.replace(placeholder, value);
    }
    result
}

/// Format a human-readable description of an activity with translations
fn format_activity_description(activity: &ActivityLogWithUsers, i18n: &I18nContext) -> String {
    let actor = &activity.actor.username;
    let affected = activity.affected_user.as_ref().map(|u| u.username.as_str());

    // Try to extract entity name from details JSON if available
    let entity_name = activity.log.details.as_ref()
        .and_then(|d| {
            // Simple extraction - look for "title" or "name" fields in JSON
            if let Some(start) = d.find("\"title\":\"") {
                let start = start + 9;
                if let Some(end) = d[start..].find('"') {
                    return Some(&d[start..start + end]);
                }
            }
            if let Some(start) = d.find("\"name\":\"") {
                let start = start + 8;
                if let Some(end) = d[start..].find('"') {
                    return Some(&d[start..start + end]);
                }
            }
            None
        })
        .unwrap_or("");

    match activity.log.activity_type {
        // Task events
        ActivityType::TaskCreated => {
            if entity_name.is_empty() {
                replace_placeholders(&i18n.t("activity.task_created_no_name"), &[("{actor}", actor)])
            } else {
                replace_placeholders(&i18n.t("activity.task_created"), &[("{actor}", actor), ("{name}", entity_name)])
            }
        }
        ActivityType::TaskUpdated => {
            if entity_name.is_empty() {
                replace_placeholders(&i18n.t("activity.task_updated_no_name"), &[("{actor}", actor)])
            } else {
                replace_placeholders(&i18n.t("activity.task_updated"), &[("{actor}", actor), ("{name}", entity_name)])
            }
        }
        ActivityType::TaskDeleted => {
            if entity_name.is_empty() {
                replace_placeholders(&i18n.t("activity.task_deleted_no_name"), &[("{actor}", actor)])
            } else {
                replace_placeholders(&i18n.t("activity.task_deleted"), &[("{actor}", actor), ("{name}", entity_name)])
            }
        }
        ActivityType::TaskAssigned => {
            if let Some(to) = affected {
                if entity_name.is_empty() {
                    replace_placeholders(&i18n.t("activity.task_assigned_no_name"), &[("{actor}", actor), ("{user}", to)])
                } else {
                    replace_placeholders(&i18n.t("activity.task_assigned"), &[("{actor}", actor), ("{name}", entity_name), ("{user}", to)])
                }
            } else {
                replace_placeholders(&i18n.t("activity.task_assigned_no_user"), &[("{actor}", actor)])
            }
        }
        ActivityType::TaskCompleted => {
            if entity_name.is_empty() {
                replace_placeholders(&i18n.t("activity.task_completed_no_name"), &[("{actor}", actor)])
            } else {
                replace_placeholders(&i18n.t("activity.task_completed"), &[("{actor}", actor), ("{name}", entity_name)])
            }
        }
        ActivityType::TaskMissed => {
            if let Some(user) = affected {
                if entity_name.is_empty() {
                    replace_placeholders(&i18n.t("activity.task_missed_no_name"), &[("{user}", user)])
                } else {
                    replace_placeholders(&i18n.t("activity.task_missed"), &[("{user}", user), ("{name}", entity_name)])
                }
            } else {
                i18n.t("activity.task_missed_no_user")
            }
        }
        ActivityType::TaskCompletionApproved => {
            if let Some(user) = affected {
                if entity_name.is_empty() {
                    replace_placeholders(&i18n.t("activity.task_completion_approved_no_name"), &[("{actor}", actor), ("{user}", user)])
                } else {
                    replace_placeholders(&i18n.t("activity.task_completion_approved"), &[("{actor}", actor), ("{user}", user), ("{name}", entity_name)])
                }
            } else {
                replace_placeholders(&i18n.t("activity.task_completion_approved_no_user"), &[("{actor}", actor)])
            }
        }
        ActivityType::TaskCompletionRejected => {
            if let Some(user) = affected {
                if entity_name.is_empty() {
                    replace_placeholders(&i18n.t("activity.task_completion_rejected_no_name"), &[("{actor}", actor), ("{user}", user)])
                } else {
                    replace_placeholders(&i18n.t("activity.task_completion_rejected"), &[("{actor}", actor), ("{user}", user), ("{name}", entity_name)])
                }
            } else {
                replace_placeholders(&i18n.t("activity.task_completion_rejected_no_user"), &[("{actor}", actor)])
            }
        }

        // Reward events
        ActivityType::RewardCreated => {
            if entity_name.is_empty() {
                replace_placeholders(&i18n.t("activity.reward_created_no_name"), &[("{actor}", actor)])
            } else {
                replace_placeholders(&i18n.t("activity.reward_created"), &[("{actor}", actor), ("{name}", entity_name)])
            }
        }
        ActivityType::RewardDeleted => {
            if entity_name.is_empty() {
                replace_placeholders(&i18n.t("activity.reward_deleted_no_name"), &[("{actor}", actor)])
            } else {
                replace_placeholders(&i18n.t("activity.reward_deleted"), &[("{actor}", actor), ("{name}", entity_name)])
            }
        }
        ActivityType::RewardAssigned => {
            if let Some(to) = affected {
                if entity_name.is_empty() {
                    replace_placeholders(&i18n.t("activity.reward_assigned_no_name"), &[("{actor}", actor), ("{user}", to)])
                } else {
                    replace_placeholders(&i18n.t("activity.reward_assigned"), &[("{actor}", actor), ("{name}", entity_name), ("{user}", to)])
                }
            } else {
                replace_placeholders(&i18n.t("activity.reward_assigned_no_user"), &[("{actor}", actor)])
            }
        }
        ActivityType::RewardPurchased => {
            if entity_name.is_empty() {
                replace_placeholders(&i18n.t("activity.reward_purchased_no_name"), &[("{actor}", actor)])
            } else {
                replace_placeholders(&i18n.t("activity.reward_purchased"), &[("{actor}", actor), ("{name}", entity_name)])
            }
        }
        ActivityType::RewardRedeemed => {
            if entity_name.is_empty() {
                replace_placeholders(&i18n.t("activity.reward_redeemed_no_name"), &[("{actor}", actor)])
            } else {
                replace_placeholders(&i18n.t("activity.reward_redeemed"), &[("{actor}", actor), ("{name}", entity_name)])
            }
        }

        // Punishment events
        ActivityType::PunishmentCreated => {
            if entity_name.is_empty() {
                replace_placeholders(&i18n.t("activity.punishment_created_no_name"), &[("{actor}", actor)])
            } else {
                replace_placeholders(&i18n.t("activity.punishment_created"), &[("{actor}", actor), ("{name}", entity_name)])
            }
        }
        ActivityType::PunishmentDeleted => {
            if entity_name.is_empty() {
                replace_placeholders(&i18n.t("activity.punishment_deleted_no_name"), &[("{actor}", actor)])
            } else {
                replace_placeholders(&i18n.t("activity.punishment_deleted"), &[("{actor}", actor), ("{name}", entity_name)])
            }
        }
        ActivityType::PunishmentAssigned => {
            if let Some(to) = affected {
                if entity_name.is_empty() {
                    replace_placeholders(&i18n.t("activity.punishment_assigned_no_name"), &[("{actor}", actor), ("{user}", to)])
                } else {
                    replace_placeholders(&i18n.t("activity.punishment_assigned"), &[("{actor}", actor), ("{name}", entity_name), ("{user}", to)])
                }
            } else {
                replace_placeholders(&i18n.t("activity.punishment_assigned_no_user"), &[("{actor}", actor)])
            }
        }
        ActivityType::PunishmentCompleted => {
            if entity_name.is_empty() {
                replace_placeholders(&i18n.t("activity.punishment_completed_no_name"), &[("{actor}", actor)])
            } else {
                replace_placeholders(&i18n.t("activity.punishment_completed"), &[("{actor}", actor), ("{name}", entity_name)])
            }
        }

        // Reward confirmation events
        ActivityType::RewardRedemptionApproved => {
            if let Some(user) = affected {
                if entity_name.is_empty() {
                    replace_placeholders(&i18n.t("activity.reward_redemption_approved_no_name"), &[("{actor}", actor), ("{user}", user)])
                } else {
                    replace_placeholders(&i18n.t("activity.reward_redemption_approved"), &[("{actor}", actor), ("{user}", user), ("{name}", entity_name)])
                }
            } else {
                replace_placeholders(&i18n.t("activity.reward_redemption_approved_no_user"), &[("{actor}", actor)])
            }
        }
        ActivityType::RewardRedemptionRejected => {
            if let Some(user) = affected {
                if entity_name.is_empty() {
                    replace_placeholders(&i18n.t("activity.reward_redemption_rejected_no_name"), &[("{actor}", actor), ("{user}", user)])
                } else {
                    replace_placeholders(&i18n.t("activity.reward_redemption_rejected"), &[("{actor}", actor), ("{user}", user), ("{name}", entity_name)])
                }
            } else {
                replace_placeholders(&i18n.t("activity.reward_redemption_rejected_no_user"), &[("{actor}", actor)])
            }
        }

        // Punishment confirmation events
        ActivityType::PunishmentCompletionApproved => {
            if let Some(user) = affected {
                if entity_name.is_empty() {
                    replace_placeholders(&i18n.t("activity.punishment_completion_approved_no_name"), &[("{actor}", actor), ("{user}", user)])
                } else {
                    replace_placeholders(&i18n.t("activity.punishment_completion_approved"), &[("{actor}", actor), ("{user}", user), ("{name}", entity_name)])
                }
            } else {
                replace_placeholders(&i18n.t("activity.punishment_completion_approved_no_user"), &[("{actor}", actor)])
            }
        }
        ActivityType::PunishmentCompletionRejected => {
            if let Some(user) = affected {
                if entity_name.is_empty() {
                    replace_placeholders(&i18n.t("activity.punishment_completion_rejected_no_name"), &[("{actor}", actor), ("{user}", user)])
                } else {
                    replace_placeholders(&i18n.t("activity.punishment_completion_rejected"), &[("{actor}", actor), ("{user}", user), ("{name}", entity_name)])
                }
            } else {
                replace_placeholders(&i18n.t("activity.punishment_completion_rejected_no_user"), &[("{actor}", actor)])
            }
        }

        // Points events
        ActivityType::PointsAdjusted => {
            if let Some(user) = affected {
                // Try to extract points from details
                let points = activity.log.details.as_ref()
                    .and_then(|d| {
                        if let Some(start) = d.find("\"points\":") {
                            let start = start + 9;
                            let end = d[start..].find([',', '}']).unwrap_or(d.len() - start);
                            d[start..start + end].trim().parse::<i64>().ok()
                        } else {
                            None
                        }
                    });

                if let Some(pts) = points {
                    let pts_str = if pts >= 0 {
                        format!("+{}", pts)
                    } else {
                        pts.to_string()
                    };
                    if pts >= 0 {
                        replace_placeholders(&i18n.t("activity.points_adjusted_positive"), &[("{actor}", actor), ("{user}", user), ("{points}", &pts.to_string())])
                    } else {
                        replace_placeholders(&i18n.t("activity.points_adjusted_negative"), &[("{actor}", actor), ("{user}", user), ("{points}", &pts_str)])
                    }
                } else {
                    replace_placeholders(&i18n.t("activity.points_adjusted_no_amount"), &[("{actor}", actor), ("{user}", user)])
                }
            } else {
                replace_placeholders(&i18n.t("activity.points_adjusted_no_user"), &[("{actor}", actor)])
            }
        }

        // Membership events
        ActivityType::MemberJoined => {
            replace_placeholders(&i18n.t("activity.member_joined"), &[("{actor}", actor)])
        }
        ActivityType::MemberLeft => {
            if let Some(user) = affected {
                replace_placeholders(&i18n.t("activity.member_removed"), &[("{user}", user), ("{actor}", actor)])
            } else {
                replace_placeholders(&i18n.t("activity.member_left"), &[("{actor}", actor)])
            }
        }
        ActivityType::MemberRoleChanged => {
            if let Some(user) = affected {
                replace_placeholders(&i18n.t("activity.member_role_changed"), &[("{actor}", actor), ("{user}", user)])
            } else {
                replace_placeholders(&i18n.t("activity.member_role_changed_no_user"), &[("{actor}", actor)])
            }
        }
        ActivityType::InvitationSent => {
            // Try to extract email from details
            let email = activity.log.details.as_ref()
                .and_then(|d| {
                    if let Some(start) = d.find("\"email\":\"") {
                        let start = start + 9;
                        if let Some(end) = d[start..].find('"') {
                            return Some(&d[start..start + end]);
                        }
                    }
                    None
                })
                .unwrap_or("");

            if email.is_empty() {
                replace_placeholders(&i18n.t("activity.invitation_sent_no_email"), &[("{actor}", actor)])
            } else {
                replace_placeholders(&i18n.t("activity.invitation_sent"), &[("{actor}", actor), ("{email}", email)])
            }
        }

        // Settings events
        ActivityType::SettingsChanged => {
            replace_placeholders(&i18n.t("activity.settings_changed"), &[("{actor}", actor)])
        }
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
    use super::*;
    use chrono::Utc;
    use shared::{ActivityLog, User};
    use uuid::Uuid;
    use wasm_bindgen_test::*;

    wasm_bindgen_test_configure!(run_in_browser);

    fn create_test_user(name: &str) -> User {
        User {
            id: Uuid::new_v4(),
            username: name.to_string(),
            email: format!("{}@test.com", name),
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }

    fn create_test_activity(
        activity_type: ActivityType,
        actor: User,
        affected: Option<User>,
        details: Option<String>,
    ) -> ActivityLogWithUsers {
        ActivityLogWithUsers {
            log: ActivityLog {
                id: Uuid::new_v4(),
                household_id: Uuid::new_v4(),
                actor_id: actor.id,
                affected_user_id: affected.as_ref().map(|u| u.id),
                activity_type,
                entity_type: None,
                entity_id: None,
                details,
                created_at: Utc::now(),
            },
            actor,
            affected_user: affected,
        }
    }

    #[wasm_bindgen_test]
    fn test_format_task_created() {
        let actor = create_test_user("Alice");
        let activity = create_test_activity(
            ActivityType::TaskCreated,
            actor,
            None,
            Some(r#"{"title":"Clean room"}"#.to_string()),
        );
        let desc = format_activity_description(&activity);
        assert_eq!(desc, "Alice created task 'Clean room'");
    }

    #[wasm_bindgen_test]
    fn test_format_task_assigned() {
        let actor = create_test_user("Alice");
        let affected = create_test_user("Bob");
        let activity = create_test_activity(
            ActivityType::TaskAssigned,
            actor,
            Some(affected),
            Some(r#"{"title":"Do dishes"}"#.to_string()),
        );
        let desc = format_activity_description(&activity);
        assert_eq!(desc, "Alice assigned task 'Do dishes' to Bob");
    }

    #[wasm_bindgen_test]
    fn test_format_points_adjusted() {
        let actor = create_test_user("Alice");
        let affected = create_test_user("Bob");
        let activity = create_test_activity(
            ActivityType::PointsAdjusted,
            actor,
            Some(affected),
            Some(r#"{"points":10}"#.to_string()),
        );
        let desc = format_activity_description(&activity);
        assert_eq!(desc, "Alice adjusted Bob's points by +10");
    }

    #[wasm_bindgen_test]
    fn test_format_member_joined() {
        let actor = create_test_user("Charlie");
        let activity = create_test_activity(ActivityType::MemberJoined, actor, None, None);
        let desc = format_activity_description(&activity);
        assert_eq!(desc, "Charlie joined the household");
    }

    #[wasm_bindgen_test]
    fn test_format_settings_changed() {
        let actor = create_test_user("Admin");
        let activity = create_test_activity(ActivityType::SettingsChanged, actor, None, None);
        let desc = format_activity_description(&activity);
        assert_eq!(desc, "Admin changed household settings");
    }
}
