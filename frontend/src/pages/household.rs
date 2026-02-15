use leptos::*;
use leptos_router::*;
use shared::{CreateInvitationRequest, Household, Invitation, LeaderboardEntry, MemberWithUser, Role, TaskWithStatus};

use crate::api::ApiClient;
use crate::components::loading::Loading;
use crate::components::modal::Modal;
use crate::components::points_display::PointsBadge;
use crate::components::task_card::TaskList;

#[component]
pub fn HouseholdPage() -> impl IntoView {
    let params = use_params_map();
    let household_id = move || params.with(|p| p.get("id").cloned().unwrap_or_default());

    let household = create_rw_signal(Option::<Household>::None);
    let members = create_rw_signal(Vec::<MemberWithUser>::new());
    let tasks = create_rw_signal(Vec::<TaskWithStatus>::new());
    let leaderboard = create_rw_signal(Vec::<LeaderboardEntry>::new());
    let invitations = create_rw_signal(Vec::<Invitation>::new());
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

            // Load members and check current user role
            if let Ok(m) = ApiClient::list_members(&id).await {
                // Check if current user can manage members
                if let Ok(current_user) = ApiClient::get_current_user().await {
                    let can_manage = m.iter().any(|member| {
                        member.user.id == current_user.id && member.membership.role.can_manage_members()
                    });
                    current_user_can_manage.set(can_manage);
                }
                members.set(m);
            }

            // Load due tasks
            if let Ok(t) = ApiClient::get_due_tasks(&id).await {
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

            loading.set(false);
        });
    });

    let on_complete_task = Callback::new(move |task_id: String| {
        let id = household_id();
        wasm_bindgen_futures::spawn_local(async move {
            if ApiClient::complete_task(&id, &task_id).await.is_ok() {
                // Refresh tasks
                if let Ok(t) = ApiClient::get_due_tasks(&id).await {
                    tasks.set(t);
                }
                // Refresh leaderboard
                if let Ok(l) = ApiClient::get_leaderboard(&id).await {
                    leaderboard.set(l);
                }
            }
        });
    });

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
                    <div class="dashboard-header">
                        <h1 class="dashboard-title">{h.name}</h1>
                        <div style="display: flex; gap: 0.5rem; margin-top: 1rem;">
                            <a href=format!("/households/{}/tasks", id.clone()) class="btn btn-outline">"Tasks"</a>
                            <a href=format!("/households/{}/rewards", id.clone()) class="btn btn-outline">"Rewards"</a>
                            <a href=format!("/households/{}/punishments", id.clone()) class="btn btn-outline">"Punishments"</a>
                            <a href=format!("/households/{}/point-conditions", id.clone()) class="btn btn-outline">"Points"</a>
                        </div>
                    </div>

                    <div class="grid grid-2">
                        <div>
                            <TaskList tasks=tasks.get() on_complete=on_complete_task />
                        </div>

                        <div>
                            <div class="card">
                                <div class="card-header">
                                    <h3 class="card-title">"Leaderboard"</h3>
                                </div>
                                {move || {
                                    let lb = leaderboard.get();
                                    if lb.is_empty() {
                                        view! {
                                            <div class="empty-state">
                                                <p>"No members yet"</p>
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
                                                                    {entry.tasks_completed} " tasks completed"
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
                                    <h3 class="card-title">"Members"</h3>
                                    <Show when=move || current_user_can_manage.get() fallback=|| ()>
                                        <button
                                            class="btn btn-primary"
                                            style="padding: 0.25rem 0.75rem; font-size: 0.875rem;"
                                            on:click=move |_| show_invite_modal.set(true)
                                        >
                                            "+ Invite"
                                        </button>
                                    </Show>
                                </div>
                                {move || {
                                    let m = members.get();
                                    view! {
                                        <div>
                                            {m.into_iter().map(|member| {
                                                let badge_class = match member.membership.role {
                                                    shared::Role::Owner => "badge badge-owner",
                                                    shared::Role::Admin => "badge badge-admin",
                                                    shared::Role::Member => "badge badge-member",
                                                };
                                                let role_text = match member.membership.role {
                                                    shared::Role::Owner => "Owner",
                                                    shared::Role::Admin => "Admin",
                                                    shared::Role::Member => "Member",
                                                };
                                                view! {
                                                    <div style="display: flex; justify-content: space-between; align-items: center; padding: 0.75rem 0; border-bottom: 1px solid var(--border-color);">
                                                        <div>
                                                            <span style="font-weight: 500;">{member.user.username}</span>
                                                            <span class=badge_class style="margin-left: 0.5rem;">{role_text}</span>
                                                        </div>
                                                        <PointsBadge points=member.membership.points />
                                                    </div>
                                                }
                                            }).collect_view()}
                                        </div>
                                    }
                                }}

                                // Pending Invitations section
                                <Show when=move || current_user_can_manage.get() && !invitations.get().is_empty() fallback=|| ()>
                                    <div style="margin-top: 1rem; padding-top: 1rem; border-top: 2px solid var(--border-color);">
                                        <h4 style="font-size: 0.875rem; color: var(--text-muted); margin-bottom: 0.5rem;">"Pending Invitations"</h4>
                                        {move || {
                                            invitations.get().into_iter().map(|inv| {
                                                let inv_id = inv.id.to_string();
                                                let cancel_id = inv_id.clone();
                                                let role_badge = if inv.role == Role::Admin {
                                                    "badge badge-admin"
                                                } else {
                                                    "badge badge-member"
                                                };
                                                let role_text = if inv.role == Role::Admin { "Admin" } else { "Member" };
                                                view! {
                                                    <div style="display: flex; justify-content: space-between; align-items: center; padding: 0.5rem 0; border-bottom: 1px solid var(--border-color); opacity: 0.7;">
                                                        <div>
                                                            <span style="font-weight: 500;">{inv.email.clone()}</span>
                                                            <span class=role_badge style="margin-left: 0.5rem;">{role_text}</span>
                                                            <span style="margin-left: 0.5rem; font-size: 0.75rem; color: var(--text-muted);">"(pending)"</span>
                                                        </div>
                                                        <button
                                                            class="btn btn-outline"
                                                            style="padding: 0.125rem 0.5rem; font-size: 0.75rem;"
                                                            on:click=move |_| on_cancel_invitation(cancel_id.clone())
                                                        >
                                                            "Cancel"
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
                <Modal title="Invite Member" on_close=move |_| show_invite_modal.set(false)>
                    {move || invite_error.get().map(|e| view! {
                        <div class="alert alert-error" style="margin-bottom: 1rem;">{e}</div>
                    })}

                    <form on:submit=on_invite_submit>
                        <div class="form-group">
                            <label class="form-label" for="invite-email">"Email Address"</label>
                            <input
                                type="email"
                                id="invite-email"
                                class="form-input"
                                placeholder="user@example.com"
                                prop:value=move || invite_email.get()
                                on:input=move |ev| invite_email.set(event_target_value(&ev))
                                required
                            />
                            <small class="form-hint">"Enter the email of the user you want to invite"</small>
                        </div>

                        <div class="form-group">
                            <label class="form-label" for="invite-role">"Role"</label>
                            <select
                                id="invite-role"
                                class="form-select"
                                prop:value=move || invite_role.get()
                                on:change=move |ev| invite_role.set(event_target_value(&ev))
                            >
                                <option value="member">"Member"</option>
                                <option value="admin">"Admin"</option>
                            </select>
                            <small class="form-hint">"Admins can manage tasks, rewards, and invite other members"</small>
                        </div>

                        <div class="modal-footer">
                            <button
                                type="button"
                                class="btn btn-outline"
                                on:click=move |_| show_invite_modal.set(false)
                                disabled=move || inviting.get()
                            >
                                "Cancel"
                            </button>
                            <button
                                type="submit"
                                class="btn btn-primary"
                                disabled=move || inviting.get()
                            >
                                {move || if inviting.get() { "Sending..." } else { "Send Invitation" }}
                            </button>
                        </div>
                    </form>
                </Modal>
            </Show>
        </Show>
    }
}
