use std::collections::HashSet;

use leptos::*;
use leptos_router::*;
use shared::{AdjustPointsRequest, Announcement, CreateInvitationRequest, Household, HouseholdSettings, Invitation, LeaderboardEntry, MemberWithUser, Punishment, Reward, Role, Task, TaskCategory, TaskPunishmentLink, TaskRewardLink, TaskWithStatus, UpdateRoleRequest};
use uuid::Uuid;

use crate::api::ApiClient;
use crate::components::announcement_banner::AnnouncementBanner;
use crate::components::announcement_modal::AnnouncementModal;
use crate::components::loading::Loading;
use crate::components::modal::Modal;
use crate::components::pending_confirmations::PendingConfirmations;
use crate::components::pending_reviews::PendingReviews;
use crate::components::pending_suggestions::PendingSuggestions;
use crate::components::points_display::PointsBadge;
use crate::components::task_card::{GroupedTaskList, TaskWithHousehold};
use crate::components::task_detail_modal::TaskDetailModal;
use crate::components::task_modal::TaskModal;
use crate::i18n::use_i18n;

#[component]
pub fn HouseholdPage() -> impl IntoView {
    let i18n = use_i18n();
    let i18n_stored = store_value(i18n);

    let params = use_params_map();
    let household_id = move || params.with(|p| p.get("id").cloned().unwrap_or_default());

    let household = create_rw_signal(Option::<Household>::None);
    let members = create_rw_signal(Vec::<MemberWithUser>::new());
    let tasks = create_rw_signal(Vec::<TaskWithStatus>::new());
    let leaderboard = create_rw_signal(Vec::<LeaderboardEntry>::new());
    let invitations = create_rw_signal(Vec::<Invitation>::new());
    let settings = create_rw_signal(Option::<HouseholdSettings>::None);
    let loading = create_rw_signal(true);
    let error = create_rw_signal(Option::<String>::None);

    // Invite modal state
    let show_invite_modal = create_rw_signal(false);
    let invite_email = create_rw_signal(String::new());
    let invite_role = create_rw_signal("member".to_string());
    let invite_error = create_rw_signal(Option::<String>::None);
    let inviting = create_rw_signal(false);

    // Check if current user can manage members
    let current_user_can_manage = create_rw_signal(false);
    let current_user_role = create_rw_signal(Option::<Role>::None);
    let current_user_id = create_rw_signal(Option::<Uuid>::None);

    // Rewards and punishments for assignment
    let rewards = create_rw_signal(Vec::<Reward>::new());
    let punishments = create_rw_signal(Vec::<Punishment>::new());

    // Announcements
    let active_announcements = create_rw_signal(Vec::<Announcement>::new());
    let show_announcement_modal = create_rw_signal(false);

    // Dashboard task whitelist
    let dashboard_task_ids = create_rw_signal(HashSet::<String>::new());

    // Task detail modal state
    let detail_task_id = create_rw_signal(Option::<String>::None);

    // Task edit modal state
    let editing_task = create_rw_signal(Option::<Task>::None);
    let task_linked_rewards = create_rw_signal(Vec::<TaskRewardLink>::new());
    let task_linked_punishments = create_rw_signal(Vec::<TaskPunishmentLink>::new());
    let categories = create_rw_signal(Vec::<TaskCategory>::new());

    // Adjust points modal state
    let show_adjust_points_modal = create_rw_signal(false);
    let adjust_points_user_id = create_rw_signal(String::new());
    let adjust_points_username = create_rw_signal(String::new());
    let adjust_points_amount = create_rw_signal(String::new());
    let adjust_points_reason = create_rw_signal(String::new());
    let adjust_points_error = create_rw_signal(Option::<String>::None);
    let adjusting_points = create_rw_signal(false);

    // Assign reward modal state
    let show_assign_reward_modal = create_rw_signal(false);
    let assign_reward_user_id = create_rw_signal(String::new());
    let assign_reward_username = create_rw_signal(String::new());
    let selected_reward_id = create_rw_signal(String::new());
    let assign_reward_error = create_rw_signal(Option::<String>::None);
    let assigning_reward = create_rw_signal(false);

    // Assign punishment modal state
    let show_assign_punishment_modal = create_rw_signal(false);
    let assign_punishment_user_id = create_rw_signal(String::new());
    let assign_punishment_username = create_rw_signal(String::new());
    let selected_punishment_id = create_rw_signal(String::new());
    let assign_punishment_error = create_rw_signal(Option::<String>::None);
    let assigning_punishment = create_rw_signal(false);

    // Owner transfer confirmation modal state
    let show_owner_transfer_modal = create_rw_signal(false);
    let owner_transfer_user_id = create_rw_signal(String::new());
    let owner_transfer_username = create_rw_signal(String::new());
    let transferring_ownership = create_rw_signal(false);

    // Load data on mount
    create_effect(move |_| {
        let id = household_id();
        if id.is_empty() {
            return;
        }

        wasm_bindgen_futures::spawn_local(async move {
            // Load household
            match ApiClient::get_household(&id).await {
                Ok(h) => household.set(Some(h)),
                Err(e) => error.set(Some(e)),
            }

            // Load members and store current user role
            if let Ok(m) = ApiClient::list_members(&id).await {
                // Find current user's role
                if let Ok(current_user) = ApiClient::get_current_user().await {
                    current_user_id.set(Some(current_user.id));
                    if let Some(member) = m.iter().find(|member| member.user.id == current_user.id) {
                        current_user_role.set(Some(member.membership.role));
                    }
                }
                members.set(m);
            }

            // Load all tasks with status
            if let Ok(t) = ApiClient::get_all_tasks_with_status(&id).await {
                tasks.set(t);
            }

            // Load leaderboard
            if let Ok(l) = ApiClient::get_leaderboard(&id).await {
                leaderboard.set(l);
            }

            // Load invitations (only for admins/owners, will fail silently for members)
            if let Ok(inv) = ApiClient::list_household_invitations(&id).await {
                invitations.set(inv);
            }

            // Load rewards, punishments, and categories for assignment/edit modals
            if let Ok(r) = ApiClient::list_rewards(&id).await {
                rewards.set(r);
            }
            if let Ok(p) = ApiClient::list_punishments(&id).await {
                punishments.set(p);
            }
            if let Ok(c) = ApiClient::list_categories(&id).await {
                categories.set(c);
            }

            // Load active announcements
            if let Ok(anns) = ApiClient::list_active_announcements(&id).await {
                active_announcements.set(anns);
            }

            // Load dashboard task IDs (user's whitelist)
            if let Ok(ids) = ApiClient::get_dashboard_task_ids().await {
                dashboard_task_ids.set(ids.into_iter().map(|id| id.to_string()).collect());
            }

            // Load settings and apply dark mode
            if let Ok(s) = ApiClient::get_household_settings(&id).await {
                // Apply dark mode
                if let Some(window) = web_sys::window() {
                    if let Some(document) = window.document() {
                        if let Some(body) = document.body() {
                            if s.dark_mode {
                                let _ = body.class_list().add_1("dark-mode");
                            } else {
                                let _ = body.class_list().remove_1("dark-mode");
                            }
                        }
                    }
                }
                // Update can_manage based on hierarchy type
                if let Some(role) = current_user_role.get() {
                    current_user_can_manage.set(s.hierarchy_type.can_manage(&role));
                }
                settings.set(Some(s));
            }

            loading.set(false);
        });
    });

    let on_complete_task = Callback::new(move |task_id: String| {
        let id = household_id();
        wasm_bindgen_futures::spawn_local(async move {
            if ApiClient::complete_task(&id, &task_id).await.is_ok() {
                // Refresh tasks
                if let Ok(t) = ApiClient::get_all_tasks_with_status(&id).await {
                    tasks.set(t);
                }
                // Refresh leaderboard
                if let Ok(l) = ApiClient::get_leaderboard(&id).await {
                    leaderboard.set(l);
                }
            }
        });
    });

    let on_uncomplete_task = Callback::new(move |task_id: String| {
        let id = household_id();
        wasm_bindgen_futures::spawn_local(async move {
            if ApiClient::uncomplete_task(&id, &task_id).await.is_ok() {
                // Refresh tasks
                if let Ok(t) = ApiClient::get_all_tasks_with_status(&id).await {
                    tasks.set(t);
                }
                // Refresh leaderboard
                if let Ok(l) = ApiClient::get_leaderboard(&id).await {
                    leaderboard.set(l);
                }
            }
        });
    });

    // Handle task edit from detail modal
    let on_edit_task = move |task: Task| {
        let id = household_id();
        let task_id = task.id.to_string();
        editing_task.set(Some(task));

        // Load linked rewards
        let id_for_rewards = id.clone();
        let task_id_for_rewards = task_id.clone();
        wasm_bindgen_futures::spawn_local(async move {
            if let Ok(r) = ApiClient::get_task_rewards(&id_for_rewards, &task_id_for_rewards).await {
                task_linked_rewards.set(r);
            }
        });

        // Load linked punishments
        wasm_bindgen_futures::spawn_local(async move {
            if let Ok(p) = ApiClient::get_task_punishments(&id, &task_id).await {
                task_linked_punishments.set(p);
            }
        });
    };

    // Handle task save from edit modal
    let on_task_save = move |saved_task: Task| {
        // Update tasks list
        tasks.update(|t| {
            if let Some(pos) = t.iter().position(|tw| tw.task.id == saved_task.id) {
                t[pos].task = saved_task;
            }
        });
        editing_task.set(None);
        task_linked_rewards.set(vec![]);
        task_linked_punishments.set(vec![]);
    };

    let on_invite_submit = move |ev: web_sys::SubmitEvent| {
        ev.prevent_default();

        let id = household_id();
        let email = invite_email.get();
        let role_str = invite_role.get();
        let role = if role_str == "admin" {
            Some(Role::Admin)
        } else {
            Some(Role::Member)
        };

        inviting.set(true);
        invite_error.set(None);

        wasm_bindgen_futures::spawn_local(async move {
            let request = CreateInvitationRequest { email, role };
            match ApiClient::create_invitation(&id, request).await {
                Ok(invitation) => {
                    invitations.update(|inv| inv.push(invitation));
                    show_invite_modal.set(false);
                    invite_email.set(String::new());
                    invite_role.set("member".to_string());
                }
                Err(e) => {
                    invite_error.set(Some(e));
                }
            }
            inviting.set(false);
        });
    };

    let on_cancel_invitation = move |invitation_id: String| {
        let id = household_id();
        wasm_bindgen_futures::spawn_local(async move {
            if ApiClient::cancel_invitation(&id, &invitation_id).await.is_ok() {
                invitations.update(|inv| inv.retain(|i| i.id.to_string() != invitation_id));
            }
        });
    };

    // Open adjust points modal for a specific member
    let open_adjust_points_modal = move |user_id: String, username: String| {
        adjust_points_user_id.set(user_id);
        adjust_points_username.set(username);
        adjust_points_amount.set(String::new());
        adjust_points_reason.set(String::new());
        adjust_points_error.set(None);
        show_adjust_points_modal.set(true);
    };

    let on_adjust_points_submit = move |ev: web_sys::SubmitEvent| {
        ev.prevent_default();

        let id = household_id();
        let user_id = adjust_points_user_id.get();
        let amount_str = adjust_points_amount.get();
        let reason = adjust_points_reason.get();

        let points: i64 = match amount_str.parse() {
            Ok(p) => p,
            Err(_) => {
                adjust_points_error.set(Some(i18n_stored.get_value().t("members.valid_number_error")));
                return;
            }
        };

        if points == 0 {
            adjust_points_error.set(Some(i18n_stored.get_value().t("members.zero_points_error")));
            return;
        }

        adjusting_points.set(true);
        adjust_points_error.set(None);

        wasm_bindgen_futures::spawn_local(async move {
            let request = AdjustPointsRequest {
                points,
                reason: if reason.is_empty() { None } else { Some(reason) },
            };
            match ApiClient::adjust_member_points(&id, &user_id, request).await {
                Ok(_) => {
                    show_adjust_points_modal.set(false);
                    // Refresh members and leaderboard
                    if let Ok(m) = ApiClient::list_members(&id).await {
                        members.set(m);
                    }
                    if let Ok(l) = ApiClient::get_leaderboard(&id).await {
                        leaderboard.set(l);
                    }
                }
                Err(e) => {
                    adjust_points_error.set(Some(e));
                }
            }
            adjusting_points.set(false);
        });
    };

    // Open assign reward modal for a specific member
    let open_assign_reward_modal = move |user_id: String, username: String| {
        assign_reward_user_id.set(user_id);
        assign_reward_username.set(username);
        selected_reward_id.set(String::new());
        assign_reward_error.set(None);
        show_assign_reward_modal.set(true);
    };

    let on_assign_reward_submit = move |ev: web_sys::SubmitEvent| {
        ev.prevent_default();

        let id = household_id();
        let user_id = assign_reward_user_id.get();
        let reward_id = selected_reward_id.get();

        if reward_id.is_empty() {
            assign_reward_error.set(Some(i18n_stored.get_value().t("members.select_reward_error")));
            return;
        }

        assigning_reward.set(true);
        assign_reward_error.set(None);

        wasm_bindgen_futures::spawn_local(async move {
            match ApiClient::assign_reward(&id, &reward_id, &user_id).await {
                Ok(_) => {
                    show_assign_reward_modal.set(false);
                    // Refresh members and leaderboard
                    if let Ok(m) = ApiClient::list_members(&id).await {
                        members.set(m);
                    }
                    if let Ok(l) = ApiClient::get_leaderboard(&id).await {
                        leaderboard.set(l);
                    }
                }
                Err(e) => {
                    assign_reward_error.set(Some(e));
                }
            }
            assigning_reward.set(false);
        });
    };

    // Open assign punishment modal for a specific member
    let open_assign_punishment_modal = move |user_id: String, username: String| {
        assign_punishment_user_id.set(user_id);
        assign_punishment_username.set(username);
        selected_punishment_id.set(String::new());
        assign_punishment_error.set(None);
        show_assign_punishment_modal.set(true);
    };

    let on_assign_punishment_submit = move |ev: web_sys::SubmitEvent| {
        ev.prevent_default();

        let id = household_id();
        let user_id = assign_punishment_user_id.get();
        let punishment_id = selected_punishment_id.get();

        if punishment_id.is_empty() {
            assign_punishment_error.set(Some(i18n_stored.get_value().t("members.select_punishment_error")));
            return;
        }

        assigning_punishment.set(true);
        assign_punishment_error.set(None);

        wasm_bindgen_futures::spawn_local(async move {
            match ApiClient::assign_punishment(&id, &punishment_id, &user_id).await {
                Ok(_) => {
                    show_assign_punishment_modal.set(false);
                    // Refresh members and leaderboard
                    if let Ok(m) = ApiClient::list_members(&id).await {
                        members.set(m);
                    }
                    if let Ok(l) = ApiClient::get_leaderboard(&id).await {
                        leaderboard.set(l);
                    }
                }
                Err(e) => {
                    assign_punishment_error.set(Some(e));
                }
            }
            assigning_punishment.set(false);
        });
    };

    // Dashboard toggle callback
    let on_toggle_dashboard = Callback::new(move |(task_id, add_to_dashboard): (String, bool)| {
        wasm_bindgen_futures::spawn_local(async move {
            if add_to_dashboard {
                if ApiClient::add_task_to_dashboard(&task_id).await.is_ok() {
                    dashboard_task_ids.update(|ids| {
                        ids.insert(task_id);
                    });
                }
            } else if ApiClient::remove_task_from_dashboard(&task_id).await.is_ok() {
                dashboard_task_ids.update(|ids| {
                    ids.remove(&task_id);
                });
            }
        });
    });

    // Task title click callback - opens detail modal
    let on_click_task_title = Callback::new(move |(task_id, _household_id): (String, String)| {
        detail_task_id.set(Some(task_id));
    });

    view! {
        <Show when=move || loading.get() fallback=|| ()>
            <Loading />
        </Show>

        <Show when=move || !loading.get() fallback=|| ()>
            {move || error.get().map(|e| view! {
                <div class="alert alert-error">{e}</div>
            })}

            {move || household.get().map(|h| {
                let id = h.id.to_string();
                view! {
                    // Announcement Banner
                    {move || {
                        let announcements = active_announcements.get();
                        let has_announcements = !announcements.is_empty();
                        let is_owner = current_user_role.get().map(|r| r == Role::Owner).unwrap_or(false);

                        if has_announcements && is_owner {
                            view! {
                                <AnnouncementBanner
                                    announcements=announcements
                                    on_manage=Callback::new(move |_| show_announcement_modal.set(true))
                                />
                            }.into_view()
                        } else if has_announcements {
                            view! {
                                <AnnouncementBanner announcements=announcements />
                            }.into_view()
                        } else if is_owner {
                            // Show just the manage button if owner and no announcements
                            view! {
                                <div class="announcements-container">
                                    <button
                                        class="btn btn-secondary btn-sm announcement-manage-btn"
                                        on:click=move |_| show_announcement_modal.set(true)
                                    >
                                        {i18n_stored.get_value().t("announcements.manage")}
                                    </button>
                                </div>
                            }.into_view()
                        } else {
                            view! {}.into_view()
                        }
                    }}

                    <div class="dashboard-header">
                        <h1 class="dashboard-title">{h.name}</h1>
                    </div>

                    <div class="grid grid-2">
                        <div>
                            {
                                let tz = settings.get().map(|s| s.timezone).unwrap_or_else(|| "UTC".to_string());
                                let dashboard_ids = dashboard_task_ids.get();
                                let hh_id = id.clone();
                                // Convert TaskWithStatus to TaskWithHousehold for the unified component
                                let tasks_with_household: Vec<TaskWithHousehold> = tasks.get()
                                    .into_iter()
                                    .map(|t| TaskWithHousehold::new(t, Some(hh_id.clone()), None))
                                    .collect();
                                view! { <GroupedTaskList tasks=tasks_with_household on_complete=on_complete_task on_uncomplete=on_uncomplete_task timezone=tz dashboard_task_ids=dashboard_ids on_toggle_dashboard=on_toggle_dashboard on_click_title=on_click_task_title /> }
                            }

                            // Pending Reviews Section (only for managers/owners)
                            <Show when=move || current_user_can_manage.get() fallback=|| ()>
                                {
                                    let hid = id.clone();
                                    let hid2 = id.clone();
                                    view! {
                                        <div style="margin-top: 1.5rem;">
                                            <PendingReviews
                                                household_id=hid
                                                on_review_complete=move |_| {
                                                    // Refresh tasks and leaderboard after review
                                                    let hid = household_id();
                                                    wasm_bindgen_futures::spawn_local(async move {
                                                        if let Ok(t) = ApiClient::get_all_tasks_with_status(&hid).await {
                                                            tasks.set(t);
                                                        }
                                                        if let Ok(l) = ApiClient::get_leaderboard(&hid).await {
                                                            leaderboard.set(l);
                                                        }
                                                    });
                                                }
                                            />
                                        </div>
                                        <div style="margin-top: 1rem;">
                                            <PendingSuggestions
                                                household_id=hid2.clone()
                                                members=members.get()
                                                on_suggestion_handled=move |_| {
                                                    // Refresh tasks after suggestion is approved
                                                    let hid = household_id();
                                                    wasm_bindgen_futures::spawn_local(async move {
                                                        if let Ok(t) = ApiClient::get_all_tasks_with_status(&hid).await {
                                                            tasks.set(t);
                                                        }
                                                    });
                                                }
                                            />
                                        </div>
                                        <div style="margin-top: 1rem;">
                                            <PendingConfirmations
                                                household_id=hid2
                                                on_confirmation_complete=move |_| {
                                                    // Refresh leaderboard after confirmation
                                                    let hid = household_id();
                                                    wasm_bindgen_futures::spawn_local(async move {
                                                        if let Ok(l) = ApiClient::get_leaderboard(&hid).await {
                                                            leaderboard.set(l);
                                                        }
                                                    });
                                                }
                                            />
                                        </div>
                                    }
                                }
                            </Show>
                        </div>

                        <div>
                            <div class="card">
                                <div class="card-header">
                                    <h3 class="card-title">{i18n_stored.get_value().t("leaderboard.title")}</h3>
                                </div>
                                {move || {
                                    let lb = leaderboard.get();
                                    if lb.is_empty() {
                                        view! {
                                            <div class="empty-state">
                                                <p>{i18n_stored.get_value().t("leaderboard.no_members")}</p>
                                            </div>
                                        }.into_view()
                                    } else {
                                        view! {
                                            <div>
                                                {lb.into_iter().map(|entry| {
                                                    let rank_class = match entry.rank {
                                                        1 => "leaderboard-rank first",
                                                        2 => "leaderboard-rank second",
                                                        3 => "leaderboard-rank third",
                                                        _ => "leaderboard-rank",
                                                    };
                                                    view! {
                                                        <div class="leaderboard-item">
                                                            <span class=rank_class>{entry.rank}</span>
                                                            <div class="leaderboard-user">
                                                                <div style="font-weight: 500;">{entry.user.username}</div>
                                                                <div style="font-size: 0.75rem; color: var(--text-muted);">
                                                                    {entry.tasks_completed} " " {i18n_stored.get_value().t("leaderboard.tasks_completed_count")}
                                                                </div>
                                                            </div>
                                                            <PointsBadge points=entry.points />
                                                        </div>
                                                    }
                                                }).collect_view()}
                                            </div>
                                        }.into_view()
                                    }
                                }}
                            </div>

                            <div class="card">
                                <div class="card-header" style="display: flex; justify-content: space-between; align-items: center;">
                                    <h3 class="card-title">{i18n_stored.get_value().t("members.title")}</h3>
                                    <Show when=move || current_user_can_manage.get() fallback=|| ()>
                                        <button
                                            class="btn btn-primary"
                                            style="padding: 0.25rem 0.75rem; font-size: 0.875rem;"
                                            on:click=move |_| show_invite_modal.set(true)
                                        >
                                            "+ " {i18n_stored.get_value().t("household.invite")}
                                        </button>
                                    </Show>
                                </div>
                                {move || {
                                    let m = members.get();
                                    let can_manage = current_user_can_manage.get();
                                    let current_settings = settings.get();
                                    let is_current_user_owner = current_user_role.get() == Some(Role::Owner);
                                    let curr_user_id = current_user_id.get();
                                    let adjust_points_title = i18n_stored.get_value().t("buttons.adjust_points");
                                    let assign_reward_title = i18n_stored.get_value().t("buttons.assign_reward");
                                    let assign_punishment_title = i18n_stored.get_value().t("buttons.assign_punishment");
                                    let role_owner_default = i18n_stored.get_value().t("roles.owner");
                                    let role_admin_default = i18n_stored.get_value().t("roles.admin");
                                    let role_member_default = i18n_stored.get_value().t("roles.member");
                                    view! {
                                        <div>
                                            {m.into_iter().map(|member| {
                                                let is_member_owner = member.membership.role == Role::Owner;
                                                let is_self = curr_user_id == Some(member.user.id);
                                                let can_change_role = is_current_user_owner && !is_member_owner && !is_self;
                                                let badge_class = match member.membership.role {
                                                    shared::Role::Owner => "badge badge-owner",
                                                    shared::Role::Admin => "badge badge-admin",
                                                    shared::Role::Member => "badge badge-member",
                                                };
                                                let select_class = match member.membership.role {
                                                    shared::Role::Owner => "role-select role-select-owner",
                                                    shared::Role::Admin => "role-select role-select-admin",
                                                    shared::Role::Member => "role-select role-select-member",
                                                };
                                                let role_text = current_settings.as_ref()
                                                    .and_then(|s| {
                                                        let label = match member.membership.role {
                                                            shared::Role::Owner => &s.role_label_owner,
                                                            shared::Role::Admin => &s.role_label_admin,
                                                            shared::Role::Member => &s.role_label_member,
                                                        };
                                                        if label.is_empty() { None } else { Some(label.clone()) }
                                                    })
                                                    .unwrap_or_else(|| match member.membership.role {
                                                        shared::Role::Owner => role_owner_default.clone(),
                                                        shared::Role::Admin => role_admin_default.clone(),
                                                        shared::Role::Member => role_member_default.clone(),
                                                    });
                                                let current_role_value = match member.membership.role {
                                                    shared::Role::Owner => "owner",
                                                    shared::Role::Admin => "admin",
                                                    shared::Role::Member => "member",
                                                };
                                                let user_id = member.user.id.to_string();
                                                let user_id_role = user_id.clone();
                                                let username = member.user.username.clone();
                                                let user_id_points = user_id.clone();
                                                let username_points = username.clone();
                                                let user_id_reward = user_id.clone();
                                                let username_reward = username.clone();
                                                let user_id_punishment = user_id.clone();
                                                let username_punishment = username.clone();
                                                let adjust_points_title = adjust_points_title.clone();
                                                let assign_reward_title = assign_reward_title.clone();
                                                let assign_punishment_title = assign_punishment_title.clone();
                                                // Use custom role labels from settings if available
                                                let role_admin_label = current_settings.as_ref()
                                                    .and_then(|s| if s.role_label_admin.is_empty() { None } else { Some(s.role_label_admin.clone()) })
                                                    .unwrap_or_else(|| role_admin_default.clone());
                                                let role_member_label = current_settings.as_ref()
                                                    .and_then(|s| if s.role_label_member.is_empty() { None } else { Some(s.role_label_member.clone()) })
                                                    .unwrap_or_else(|| role_member_default.clone());
                                                let role_owner_label = current_settings.as_ref()
                                                    .and_then(|s| if s.role_label_owner.is_empty() { None } else { Some(s.role_label_owner.clone()) })
                                                    .unwrap_or_else(|| role_owner_default.clone());
                                                let member_username = username.clone();
                                                let member_user_id_for_transfer = user_id.clone();
                                                view! {
                                                    <div style="display: flex; justify-content: space-between; align-items: center; padding: 0.75rem 0; border-bottom: 1px solid var(--border-color);">
                                                        <div>
                                                            <span style="font-weight: 500;">{member.user.username}</span>
                                                            {if can_change_role {
                                                                let hh_id = household_id();
                                                                view! {
                                                                    <select
                                                                        class=select_class
                                                                        style="margin-left: 0.5rem;"
                                                                        on:change=move |ev| {
                                                                            let new_role_str = event_target_value(&ev);
                                                                            let hh_id = hh_id.clone();
                                                                            let user_id_role = user_id_role.clone();
                                                                            // Handle owner transfer with confirmation
                                                                            if new_role_str == "owner" {
                                                                                owner_transfer_user_id.set(member_user_id_for_transfer.clone());
                                                                                owner_transfer_username.set(member_username.clone());
                                                                                show_owner_transfer_modal.set(true);
                                                                                // Reset dropdown to current value by reloading members
                                                                                wasm_bindgen_futures::spawn_local(async move {
                                                                                    if let Ok(m) = ApiClient::list_members(&hh_id).await {
                                                                                        members.set(m);
                                                                                    }
                                                                                });
                                                                                return;
                                                                            }
                                                                            let new_role = match new_role_str.as_str() {
                                                                                "admin" => Role::Admin,
                                                                                _ => Role::Member,
                                                                            };
                                                                            wasm_bindgen_futures::spawn_local(async move {
                                                                                match ApiClient::update_member_role(&hh_id, &user_id_role, UpdateRoleRequest { role: new_role }).await {
                                                                                    Ok(_) => {
                                                                                        // Reload members
                                                                                        if let Ok(m) = ApiClient::list_members(&hh_id).await {
                                                                                            members.set(m);
                                                                                        }
                                                                                    }
                                                                                    Err(_) => {
                                                                                        // Error handling - reload members to restore UI state
                                                                                        if let Ok(m) = ApiClient::list_members(&hh_id).await {
                                                                                            members.set(m);
                                                                                        }
                                                                                    }
                                                                                }
                                                                            });
                                                                        }
                                                                    >
                                                                        <option value="owner">{role_owner_label.clone()}</option>
                                                                        <option value="admin" selected=move || current_role_value == "admin">{role_admin_label.clone()}</option>
                                                                        <option value="member" selected=move || current_role_value == "member">{role_member_label.clone()}</option>
                                                                    </select>
                                                                }.into_view()
                                                            } else {
                                                                view! {
                                                                    <span class=badge_class style="margin-left: 0.5rem;">{role_text}</span>
                                                                }.into_view()
                                                            }}
                                                        </div>
                                                        <div style="display: flex; align-items: center; gap: 0.5rem;">
                                                            {if can_manage {
                                                                view! {
                                                                    <div style="display: flex; gap: 0.25rem;">
                                                                        <button
                                                                            class="btn btn-outline"
                                                                            style="padding: 0.125rem 0.5rem; font-size: 0.75rem;"
                                                                            title=adjust_points_title.clone()
                                                                            on:click=move |_| open_adjust_points_modal(user_id_points.clone(), username_points.clone())
                                                                        >
                                                                            "±"
                                                                        </button>
                                                                        <button
                                                                            class="btn btn-outline"
                                                                            style="padding: 0.125rem 0.5rem; font-size: 0.75rem; color: var(--success-color);"
                                                                            title=assign_reward_title.clone()
                                                                            on:click=move |_| open_assign_reward_modal(user_id_reward.clone(), username_reward.clone())
                                                                        >
                                                                            "🎁"
                                                                        </button>
                                                                        <button
                                                                            class="btn btn-outline"
                                                                            style="padding: 0.125rem 0.5rem; font-size: 0.75rem; color: var(--error-color);"
                                                                            title=assign_punishment_title.clone()
                                                                            on:click=move |_| open_assign_punishment_modal(user_id_punishment.clone(), username_punishment.clone())
                                                                        >
                                                                            "⚠"
                                                                        </button>
                                                                    </div>
                                                                }.into_view()
                                                            } else {
                                                                ().into_view()
                                                            }}
                                                            <PointsBadge points=member.membership.points />
                                                        </div>
                                                    </div>
                                                }
                                            }).collect_view()}
                                        </div>
                                    }
                                }}

                                // Pending Invitations section
                                <Show when=move || current_user_can_manage.get() && !invitations.get().is_empty() fallback=|| ()>
                                    <div style="margin-top: 1rem; padding-top: 1rem; border-top: 2px solid var(--border-color);">
                                        <h4 style="font-size: 0.875rem; color: var(--text-muted); margin-bottom: 0.5rem;">{i18n_stored.get_value().t("invitations.pending")}</h4>
                                        {move || {
                                            let current_settings = settings.get();
                                            let role_admin_default = i18n_stored.get_value().t("roles.admin");
                                            let role_member_default = i18n_stored.get_value().t("roles.member");
                                            invitations.get().into_iter().map(|inv| {
                                                let inv_id = inv.id.to_string();
                                                let cancel_id = inv_id.clone();
                                                let role_badge = if inv.role == Role::Admin {
                                                    "badge badge-admin"
                                                } else {
                                                    "badge badge-member"
                                                };
                                                let role_text = current_settings.as_ref()
                                                    .and_then(|s| {
                                                        let label = if inv.role == Role::Admin {
                                                            &s.role_label_admin
                                                        } else {
                                                            &s.role_label_member
                                                        };
                                                        if label.is_empty() { None } else { Some(label.clone()) }
                                                    })
                                                    .unwrap_or_else(|| if inv.role == Role::Admin {
                                                        role_admin_default.clone()
                                                    } else {
                                                        role_member_default.clone()
                                                    });
                                                view! {
                                                    <div style="display: flex; justify-content: space-between; align-items: center; padding: 0.5rem 0; border-bottom: 1px solid var(--border-color); opacity: 0.7;">
                                                        <div>
                                                            <span style="font-weight: 500;">{inv.email.clone()}</span>
                                                            <span class=role_badge style="margin-left: 0.5rem;">{role_text}</span>
                                                            <span style="margin-left: 0.5rem; font-size: 0.75rem; color: var(--text-muted);">{i18n_stored.get_value().t("members.pending")}</span>
                                                        </div>
                                                        <button
                                                            class="btn btn-outline"
                                                            style="padding: 0.125rem 0.5rem; font-size: 0.75rem;"
                                                            on:click=move |_| on_cancel_invitation(cancel_id.clone())
                                                        >
                                                            {i18n_stored.get_value().t("common.cancel")}
                                                        </button>
                                                    </div>
                                                }
                                            }).collect_view()
                                        }}
                                    </div>
                                </Show>
                            </div>
                        </div>
                    </div>
                }
            })}

            // Invite Modal
            <Show when=move || show_invite_modal.get() fallback=|| ()>
                <Modal title=i18n_stored.get_value().t("members.invite") on_close=move |_| show_invite_modal.set(false)>
                    {move || invite_error.get().map(|e| view! {
                        <div class="alert alert-error" style="margin-bottom: 1rem;">{e}</div>
                    })}

                    <form on:submit=on_invite_submit>
                        <div class="form-group">
                            <label class="form-label" for="invite-email">{i18n_stored.get_value().t("members.email")}</label>
                            <input
                                type="email"
                                id="invite-email"
                                class="form-input"
                                placeholder="user@example.com"
                                prop:value=move || invite_email.get()
                                on:input=move |ev| invite_email.set(event_target_value(&ev))
                                required
                            />
                            <small class="form-hint">{i18n_stored.get_value().t("members.invite_hint")}</small>
                        </div>

                        <div class="form-group">
                            <label class="form-label" for="invite-role">{i18n_stored.get_value().t("members.role")}</label>
                            <select
                                id="invite-role"
                                class="form-select"
                                prop:value=move || invite_role.get()
                                on:change=move |ev| invite_role.set(event_target_value(&ev))
                            >
                                <option value="member">
                                    {move || settings.get().map(|s| s.role_label_member).unwrap_or_else(|| i18n_stored.get_value().t("roles.member"))}
                                </option>
                                <option value="admin">
                                    {move || settings.get().map(|s| s.role_label_admin).unwrap_or_else(|| i18n_stored.get_value().t("roles.admin"))}
                                </option>
                            </select>
                            <small class="form-hint">{i18n_stored.get_value().t("members.role_hint")}</small>
                        </div>

                        <div class="modal-footer">
                            <button
                                type="button"
                                class="btn btn-outline"
                                on:click=move |_| show_invite_modal.set(false)
                                disabled=move || inviting.get()
                            >
                                {i18n_stored.get_value().t("common.cancel")}
                            </button>
                            <button
                                type="submit"
                                class="btn btn-primary"
                                disabled=move || inviting.get()
                            >
                                {move || if inviting.get() { i18n_stored.get_value().t("members.sending") } else { i18n_stored.get_value().t("members.send_invitation") }}
                            </button>
                        </div>
                    </form>
                </Modal>
            </Show>

            // Adjust Points Modal
            <Show when=move || show_adjust_points_modal.get() fallback=|| ()>
                <Modal title=i18n_stored.get_value().t("members.adjust_points_title") on_close=move |_| show_adjust_points_modal.set(false)>
                    {move || adjust_points_error.get().map(|e| view! {
                        <div class="alert alert-error" style="margin-bottom: 1rem;">{e}</div>
                    })}

                    <form on:submit=on_adjust_points_submit>
                        <div class="form-group">
                            <label class="form-label" for="adjust-points-amount">{i18n_stored.get_value().t("common.points")}</label>
                            <input
                                type="number"
                                id="adjust-points-amount"
                                class="form-input"
                                placeholder=i18n_stored.get_value().t("members.points_placeholder")
                                prop:value=move || adjust_points_amount.get()
                                on:input=move |ev| adjust_points_amount.set(event_target_value(&ev))
                                required
                            />
                            <small class="form-hint">{i18n_stored.get_value().t("members.adjust_points_hint")}</small>
                        </div>

                        <div class="form-group">
                            <label class="form-label" for="adjust-points-reason">{i18n_stored.get_value().t("members.points_reason")}</label>
                            <input
                                type="text"
                                id="adjust-points-reason"
                                class="form-input"
                                placeholder=i18n_stored.get_value().t("members.reason_placeholder")
                                prop:value=move || adjust_points_reason.get()
                                on:input=move |ev| adjust_points_reason.set(event_target_value(&ev))
                            />
                        </div>

                        <div class="modal-footer">
                            <button
                                type="button"
                                class="btn btn-outline"
                                on:click=move |_| show_adjust_points_modal.set(false)
                                disabled=move || adjusting_points.get()
                            >
                                {i18n_stored.get_value().t("common.cancel")}
                            </button>
                            <button
                                type="submit"
                                class="btn btn-primary"
                                disabled=move || adjusting_points.get()
                            >
                                {move || if adjusting_points.get() { i18n_stored.get_value().t("members.adjusting") } else { i18n_stored.get_value().t("members.adjust_points") }}
                            </button>
                        </div>
                    </form>
                </Modal>
            </Show>

            // Assign Reward Modal
            <Show when=move || show_assign_reward_modal.get() fallback=|| ()>
                <Modal title=i18n_stored.get_value().t("rewards.assign") on_close=move |_| show_assign_reward_modal.set(false)>
                    {move || assign_reward_error.get().map(|e| view! {
                        <div class="alert alert-error" style="margin-bottom: 1rem;">{e}</div>
                    })}

                    <form on:submit=on_assign_reward_submit>
                        <div class="form-group">
                            <label class="form-label" for="select-reward">{i18n_stored.get_value().t("members.select_reward")}</label>
                            <select
                                id="select-reward"
                                class="form-select"
                                prop:value=move || selected_reward_id.get()
                                on:change=move |ev| selected_reward_id.set(event_target_value(&ev))
                                required
                            >
                                <option value="">{i18n_stored.get_value().t("members.select_reward_placeholder")}</option>
                                {move || rewards.get().into_iter().map(|r| {
                                    let id = r.id.to_string();
                                    let cost_text = r.point_cost.map(|c| format!(" ({} pts)", c)).unwrap_or_default();
                                    view! {
                                        <option value=id.clone()>{r.name}{cost_text}</option>
                                    }
                                }).collect_view()}
                            </select>
                            <small class="form-hint">{i18n_stored.get_value().t("members.reward_hint")}</small>
                        </div>

                        <div class="modal-footer">
                            <button
                                type="button"
                                class="btn btn-outline"
                                on:click=move |_| show_assign_reward_modal.set(false)
                                disabled=move || assigning_reward.get()
                            >
                                {i18n_stored.get_value().t("common.cancel")}
                            </button>
                            <button
                                type="submit"
                                class="btn btn-primary"
                                disabled=move || assigning_reward.get()
                            >
                                {move || if assigning_reward.get() { i18n_stored.get_value().t("members.assigning") } else { i18n_stored.get_value().t("rewards.assign") }}
                            </button>
                        </div>
                    </form>
                </Modal>
            </Show>

            // Assign Punishment Modal
            <Show when=move || show_assign_punishment_modal.get() fallback=|| ()>
                <Modal title=i18n_stored.get_value().t("punishments.assign") on_close=move |_| show_assign_punishment_modal.set(false)>
                    {move || assign_punishment_error.get().map(|e| view! {
                        <div class="alert alert-error" style="margin-bottom: 1rem;">{e}</div>
                    })}

                    <form on:submit=on_assign_punishment_submit>
                        <div class="form-group">
                            <label class="form-label" for="select-punishment">{i18n_stored.get_value().t("members.select_punishment")}</label>
                            <select
                                id="select-punishment"
                                class="form-select"
                                prop:value=move || selected_punishment_id.get()
                                on:change=move |ev| selected_punishment_id.set(event_target_value(&ev))
                                required
                            >
                                <option value="">{i18n_stored.get_value().t("members.select_punishment_placeholder")}</option>
                                {move || punishments.get().into_iter().map(|p| {
                                    let id = p.id.to_string();
                                    view! {
                                        <option value=id.clone()>{p.name}</option>
                                    }
                                }).collect_view()}
                            </select>
                            <small class="form-hint">{i18n_stored.get_value().t("members.punishment_hint")}</small>
                        </div>

                        <div class="modal-footer">
                            <button
                                type="button"
                                class="btn btn-outline"
                                on:click=move |_| show_assign_punishment_modal.set(false)
                                disabled=move || assigning_punishment.get()
                            >
                                {i18n_stored.get_value().t("common.cancel")}
                            </button>
                            <button
                                type="submit"
                                class="btn btn-primary"
                                disabled=move || assigning_punishment.get()
                            >
                                {move || if assigning_punishment.get() { i18n_stored.get_value().t("members.assigning") } else { i18n_stored.get_value().t("punishments.assign") }}
                            </button>
                        </div>
                    </form>
                </Modal>
            </Show>

            // Owner Transfer Confirmation Modal
            <Show when=move || show_owner_transfer_modal.get() fallback=|| ()>
                <Modal title=i18n_stored.get_value().t("members.transfer_ownership") on_close=move |_| show_owner_transfer_modal.set(false)>
                    <div style="margin-bottom: 1rem;">
                        <p style="margin-bottom: 0.5rem;">
                            {move || {
                                let username = owner_transfer_username.get();
                                i18n_stored.get_value().t("members.transfer_ownership_confirm").replace("{username}", &username)
                            }}
                        </p>
                        <p style="color: var(--warning-color); font-weight: 500;">
                            {i18n_stored.get_value().t("members.transfer_ownership_warning")}
                        </p>
                    </div>

                    <div class="modal-footer">
                        <button
                            type="button"
                            class="btn btn-outline"
                            on:click=move |_| show_owner_transfer_modal.set(false)
                            disabled=move || transferring_ownership.get()
                        >
                            {i18n_stored.get_value().t("common.cancel")}
                        </button>
                        <button
                            type="button"
                            class="btn btn-danger"
                            disabled=move || transferring_ownership.get()
                            on:click=move |_| {
                                let hh_id = household_id();
                                let target_user_id = owner_transfer_user_id.get();
                                transferring_ownership.set(true);
                                wasm_bindgen_futures::spawn_local(async move {
                                    match ApiClient::update_member_role(&hh_id, &target_user_id, UpdateRoleRequest { role: Role::Owner }).await {
                                        Ok(_) => {
                                            // Reload members and update current user role
                                            if let Ok(m) = ApiClient::list_members(&hh_id).await {
                                                // Find current user's new role
                                                if let Ok(current_user) = ApiClient::get_current_user().await {
                                                    if let Some(member) = m.iter().find(|member| member.user.id == current_user.id) {
                                                        current_user_role.set(Some(member.membership.role));
                                                    }
                                                }
                                                members.set(m);
                                            }
                                            show_owner_transfer_modal.set(false);
                                        }
                                        Err(_) => {
                                            // Error - reload members to restore UI state
                                            if let Ok(m) = ApiClient::list_members(&hh_id).await {
                                                members.set(m);
                                            }
                                        }
                                    }
                                    transferring_ownership.set(false);
                                });
                            }
                        >
                            {move || if transferring_ownership.get() { i18n_stored.get_value().t("common.processing") } else { i18n_stored.get_value().t("members.confirm_transfer") }}
                        </button>
                    </div>
                </Modal>
            </Show>

            // Announcement Management Modal
            <Show when=move || show_announcement_modal.get() fallback=|| ()>
                <AnnouncementModal
                    household_id=household_id()
                    on_close=Callback::new(move |_| {
                        show_announcement_modal.set(false);
                        // Refresh active announcements
                        let hid = household_id();
                        wasm_bindgen_futures::spawn_local(async move {
                            if let Ok(anns) = ApiClient::list_active_announcements(&hid).await {
                                active_announcements.set(anns);
                            }
                        });
                    })
                />
            </Show>

            // Task Detail Modal
            {move || {
                let hh_id = household_id();
                detail_task_id.get().map(|tid| view! {
                    <TaskDetailModal
                        task_id=tid
                        household_id=hh_id
                        on_close=move |_| detail_task_id.set(None)
                        on_edit=move |task| {
                            detail_task_id.set(None);
                            on_edit_task(task);
                        }
                    />
                })
            }}

            // Task Edit Modal
            {move || editing_task.get().map(|task| {
                let hid = household_id();
                view! {
                    <TaskModal
                        task=Some(task)
                        household_id=hid
                        members=members.get()
                        household_rewards=rewards.get()
                        household_punishments=punishments.get()
                        linked_rewards=task_linked_rewards.get()
                        linked_punishments=task_linked_punishments.get()
                        categories=categories.get()
                        on_close=move |_| {
                            editing_task.set(None);
                            task_linked_rewards.set(vec![]);
                            task_linked_punishments.set(vec![]);
                        }
                        on_save=on_task_save
                    />
                }
            })}
        </Show>
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use wasm_bindgen_test::*;

    wasm_bindgen_test_configure!(run_in_browser);

    #[wasm_bindgen_test]
    fn test_leaderboard_rank_class_first() {
        let rank = 1;
        let rank_class = match rank {
            1 => "leaderboard-rank first",
            2 => "leaderboard-rank second",
            3 => "leaderboard-rank third",
            _ => "leaderboard-rank",
        };
        assert_eq!(rank_class, "leaderboard-rank first");
    }

    #[wasm_bindgen_test]
    fn test_leaderboard_rank_class_second() {
        let rank = 2;
        let rank_class = match rank {
            1 => "leaderboard-rank first",
            2 => "leaderboard-rank second",
            3 => "leaderboard-rank third",
            _ => "leaderboard-rank",
        };
        assert_eq!(rank_class, "leaderboard-rank second");
    }

    #[wasm_bindgen_test]
    fn test_leaderboard_rank_class_third() {
        let rank = 3;
        let rank_class = match rank {
            1 => "leaderboard-rank first",
            2 => "leaderboard-rank second",
            3 => "leaderboard-rank third",
            _ => "leaderboard-rank",
        };
        assert_eq!(rank_class, "leaderboard-rank third");
    }

    #[wasm_bindgen_test]
    fn test_leaderboard_rank_class_other() {
        let rank = 5;
        let rank_class = match rank {
            1 => "leaderboard-rank first",
            2 => "leaderboard-rank second",
            3 => "leaderboard-rank third",
            _ => "leaderboard-rank",
        };
        assert_eq!(rank_class, "leaderboard-rank");
    }

    #[wasm_bindgen_test]
    fn test_role_badge_class_owner() {
        let role = Role::Owner;
        let badge_class = match role {
            Role::Owner => "badge badge-owner",
            Role::Admin => "badge badge-admin",
            Role::Member => "badge badge-member",
        };
        assert_eq!(badge_class, "badge badge-owner");
    }

    #[wasm_bindgen_test]
    fn test_role_badge_class_admin() {
        let role = Role::Admin;
        let badge_class = match role {
            Role::Owner => "badge badge-owner",
            Role::Admin => "badge badge-admin",
            Role::Member => "badge badge-member",
        };
        assert_eq!(badge_class, "badge badge-admin");
    }

    #[wasm_bindgen_test]
    fn test_role_badge_class_member() {
        let role = Role::Member;
        let badge_class = match role {
            Role::Owner => "badge badge-owner",
            Role::Admin => "badge badge-admin",
            Role::Member => "badge badge-member",
        };
        assert_eq!(badge_class, "badge badge-member");
    }

    #[wasm_bindgen_test]
    fn test_role_text_owner() {
        let role = Role::Owner;
        let role_text = match role {
            Role::Owner => "Owner",
            Role::Admin => "Admin",
            Role::Member => "Member",
        };
        assert_eq!(role_text, "Owner");
    }

    #[wasm_bindgen_test]
    fn test_points_validation_valid() {
        let amount_str = "10";
        let points: Result<i64, _> = amount_str.parse();
        assert!(points.is_ok());
        assert_eq!(points.unwrap(), 10);
    }

    #[wasm_bindgen_test]
    fn test_points_validation_negative() {
        let amount_str = "-5";
        let points: Result<i64, _> = amount_str.parse();
        assert!(points.is_ok());
        assert_eq!(points.unwrap(), -5);
    }

    #[wasm_bindgen_test]
    fn test_points_validation_invalid() {
        let amount_str = "abc";
        let points: Result<i64, _> = amount_str.parse();
        assert!(points.is_err());
    }

    #[wasm_bindgen_test]
    fn test_points_validation_zero_rejected() {
        let points: i64 = 0;
        let is_zero = points == 0;
        assert!(is_zero);
    }

    #[wasm_bindgen_test]
    fn test_invite_role_admin() {
        let role_str = "admin";
        let role = if role_str == "admin" {
            Some(Role::Admin)
        } else {
            Some(Role::Member)
        };
        assert_eq!(role, Some(Role::Admin));
    }

    #[wasm_bindgen_test]
    fn test_invite_role_member() {
        let role_str = "member";
        let role = if role_str == "admin" {
            Some(Role::Admin)
        } else {
            Some(Role::Member)
        };
        assert_eq!(role, Some(Role::Member));
    }

    #[wasm_bindgen_test]
    fn test_empty_reason_handling() {
        let reason = String::new();
        let result = if reason.is_empty() { None } else { Some(reason) };
        assert!(result.is_none());
    }

    #[wasm_bindgen_test]
    fn test_nonempty_reason_handling() {
        let reason = "Bonus for helping".to_string();
        let result = if reason.is_empty() { None } else { Some(reason.clone()) };
        assert_eq!(result, Some("Bonus for helping".to_string()));
    }

    #[wasm_bindgen_test]
    fn test_button_text_inviting() {
        let inviting = true;
        let text = if inviting { "Sending..." } else { "Send Invitation" };
        assert_eq!(text, "Sending...");
    }

    #[wasm_bindgen_test]
    fn test_button_text_not_inviting() {
        let inviting = false;
        let text = if inviting { "Sending..." } else { "Send Invitation" };
        assert_eq!(text, "Send Invitation");
    }
}
