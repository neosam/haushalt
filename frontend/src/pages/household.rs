use leptos::*;
use leptos_router::*;
use shared::{Household, LeaderboardEntry, MemberWithUser, TaskWithStatus};

use crate::api::ApiClient;
use crate::components::loading::Loading;
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
    let loading = create_rw_signal(true);
    let error = create_rw_signal(Option::<String>::None);

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

            // Load members
            if let Ok(m) = ApiClient::list_members(&id).await {
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
                                <div class="card-header">
                                    <h3 class="card-title">"Members"</h3>
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
                            </div>
                        </div>
                    </div>
                }
            })}
        </Show>
    }
}
