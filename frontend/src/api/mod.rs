pub mod websocket;

use gloo_net::http::Request;
use gloo_storage::{LocalStorage, Storage};
use leptos::*;
use serde::{de::DeserializeOwned, Serialize};
use shared::{
    ActivityLogWithUsers, AdjustPointsRequest, AdjustPointsResponse, ApiError, ApiSuccess, AuthResponse,
    ChatMessageWithUser, CreateChatMessageRequest, CreateHouseholdRequest, CreateInvitationRequest,
    CreateNoteRequest, CreatePointConditionRequest, CreatePunishmentRequest, CreateRewardRequest,
    CreateTaskRequest, CreateUserRequest, Household, HouseholdMembership, HouseholdSettings, Invitation,
    InvitationWithHousehold, InviteUserRequest, LeaderboardEntry, LoginRequest, MemberWithUser, Note,
    NoteWithUser, PendingPunishmentCompletion, PendingReview, PendingRewardRedemption, PointCondition,
    Punishment, Reward, Task, TaskCompletion, TaskPunishmentLink, TaskRewardLink, TaskWithStatus,
    UpdateChatMessageRequest, UpdateHouseholdSettingsRequest, UpdateNoteRequest, UpdatePunishmentRequest,
    UpdateRewardRequest, UpdateTaskRequest, User, UserPunishment, UserPunishmentWithUser, UserReward,
    UserRewardWithUser,
};

const API_BASE: &str = "/api";
const TOKEN_KEY: &str = "auth_token";

#[derive(Clone)]
pub struct AuthState {
    pub token: RwSignal<Option<String>>,
    pub user: RwSignal<Option<User>>,
}

impl Default for AuthState {
    fn default() -> Self {
        Self::new()
    }
}

impl AuthState {
    pub fn new() -> Self {
        let stored_token: Option<String> = LocalStorage::get(TOKEN_KEY).ok();

        Self {
            token: create_rw_signal(stored_token),
            user: create_rw_signal(None),
        }
    }

    pub fn is_authenticated(&self) -> bool {
        self.token.get().is_some()
    }

    pub fn set_auth(&self, response: AuthResponse) {
        LocalStorage::set(TOKEN_KEY, &response.token).ok();
        self.token.set(Some(response.token));
        self.user.set(Some(response.user));
    }

    pub fn logout(&self) {
        LocalStorage::delete(TOKEN_KEY);
        self.token.set(None);
        self.user.set(None);
    }

    pub fn get_token(&self) -> Option<String> {
        self.token.get()
    }
}

pub struct ApiClient;

impl ApiClient {
    fn get_token() -> Option<String> {
        LocalStorage::get(TOKEN_KEY).ok()
    }

    async fn request<T: DeserializeOwned>(
        method: &str,
        path: &str,
        body: Option<impl Serialize>,
        auth: bool,
    ) -> Result<T, String> {
        let url = format!("{}{}", API_BASE, path);

        let mut request = match method {
            "GET" => Request::get(&url),
            "POST" => Request::post(&url),
            "PUT" => Request::put(&url),
            "DELETE" => Request::delete(&url),
            _ => return Err("Invalid method".to_string()),
        };

        if auth {
            if let Some(token) = Self::get_token() {
                request = request.header("Authorization", &format!("Bearer {}", token));
            }
        }

        let response = if let Some(body) = body {
            request
                .header("Content-Type", "application/json")
                .json(&body)
                .map_err(|e| e.to_string())?
                .send()
                .await
                .map_err(|e| e.to_string())?
        } else {
            request
                .send()
                .await
                .map_err(|e| e.to_string())?
        };

        if response.ok() {
            let result: ApiSuccess<T> = response.json().await.map_err(|e| e.to_string())?;
            Ok(result.data)
        } else {
            let error: ApiError = response
                .json()
                .await
                .unwrap_or(ApiError {
                    error: "unknown".to_string(),
                    message: "An unknown error occurred".to_string(),
                });
            Err(error.message)
        }
    }

    // Auth endpoints
    pub async fn register(request: CreateUserRequest) -> Result<AuthResponse, String> {
        Self::request("POST", "/auth/register", Some(request), false).await
    }

    pub async fn login(request: LoginRequest) -> Result<AuthResponse, String> {
        Self::request("POST", "/auth/login", Some(request), false).await
    }

    pub async fn get_current_user() -> Result<User, String> {
        Self::request::<User>("GET", "/auth/me", None::<()>, true).await
    }

    // Household endpoints
    pub async fn list_households() -> Result<Vec<Household>, String> {
        Self::request::<Vec<Household>>("GET", "/households", None::<()>, true).await
    }

    pub async fn create_household(request: CreateHouseholdRequest) -> Result<Household, String> {
        Self::request("POST", "/households", Some(request), true).await
    }

    pub async fn get_household(id: &str) -> Result<Household, String> {
        Self::request::<Household>("GET", &format!("/households/{}", id), None::<()>, true).await
    }

    pub async fn delete_household(id: &str) -> Result<(), String> {
        Self::request::<()>("DELETE", &format!("/households/{}", id), None::<()>, true).await
    }

    pub async fn list_members(household_id: &str) -> Result<Vec<MemberWithUser>, String> {
        Self::request::<Vec<MemberWithUser>>(
            "GET",
            &format!("/households/{}/members", household_id),
            None::<()>,
            true,
        )
        .await
    }

    pub async fn invite_member(household_id: &str, request: InviteUserRequest) -> Result<HouseholdMembership, String> {
        Self::request(
            "POST",
            &format!("/households/{}/invite", household_id),
            Some(request),
            true,
        )
        .await
    }

    pub async fn get_leaderboard(household_id: &str) -> Result<Vec<LeaderboardEntry>, String> {
        Self::request::<Vec<LeaderboardEntry>>(
            "GET",
            &format!("/households/{}/leaderboard", household_id),
            None::<()>,
            true,
        )
        .await
    }

    // Household settings endpoints
    pub async fn get_household_settings(household_id: &str) -> Result<HouseholdSettings, String> {
        Self::request::<HouseholdSettings>(
            "GET",
            &format!("/households/{}/settings", household_id),
            None::<()>,
            true,
        )
        .await
    }

    pub async fn update_household_settings(
        household_id: &str,
        request: UpdateHouseholdSettingsRequest,
    ) -> Result<HouseholdSettings, String> {
        Self::request(
            "PUT",
            &format!("/households/{}/settings", household_id),
            Some(request),
            true,
        )
        .await
    }

    // Task endpoints
    pub async fn list_tasks(household_id: &str) -> Result<Vec<Task>, String> {
        Self::request::<Vec<Task>>(
            "GET",
            &format!("/households/{}/tasks", household_id),
            None::<()>,
            true,
        )
        .await
    }

    pub async fn get_due_tasks(household_id: &str) -> Result<Vec<TaskWithStatus>, String> {
        Self::request::<Vec<TaskWithStatus>>(
            "GET",
            &format!("/households/{}/tasks/due", household_id),
            None::<()>,
            true,
        )
        .await
    }

    pub async fn get_all_tasks_with_status(household_id: &str) -> Result<Vec<TaskWithStatus>, String> {
        Self::request::<Vec<TaskWithStatus>>(
            "GET",
            &format!("/households/{}/tasks/all", household_id),
            None::<()>,
            true,
        )
        .await
    }

    pub async fn get_my_assigned_tasks(household_id: &str) -> Result<Vec<Task>, String> {
        Self::request::<Vec<Task>>(
            "GET",
            &format!("/households/{}/tasks/assigned-to-me", household_id),
            None::<()>,
            true,
        )
        .await
    }

    pub async fn create_task(household_id: &str, request: CreateTaskRequest) -> Result<Task, String> {
        Self::request(
            "POST",
            &format!("/households/{}/tasks", household_id),
            Some(request),
            true,
        )
        .await
    }

    pub async fn complete_task(household_id: &str, task_id: &str) -> Result<TaskCompletion, String> {
        Self::request::<TaskCompletion>(
            "POST",
            &format!("/households/{}/tasks/{}/complete", household_id, task_id),
            None::<()>,
            true,
        )
        .await
    }

    pub async fn uncomplete_task(household_id: &str, task_id: &str) -> Result<(), String> {
        Self::request::<()>(
            "POST",
            &format!("/households/{}/tasks/{}/uncomplete", household_id, task_id),
            None::<()>,
            true,
        )
        .await
    }

    pub async fn delete_task(household_id: &str, task_id: &str) -> Result<(), String> {
        Self::request::<()>(
            "DELETE",
            &format!("/households/{}/tasks/{}", household_id, task_id),
            None::<()>,
            true,
        )
        .await
    }

    pub async fn update_task(
        household_id: &str,
        task_id: &str,
        request: UpdateTaskRequest,
    ) -> Result<Task, String> {
        Self::request(
            "PUT",
            &format!("/households/{}/tasks/{}", household_id, task_id),
            Some(request),
            true,
        )
        .await
    }

    // Task rewards/punishments endpoints
    pub async fn get_task_rewards(household_id: &str, task_id: &str) -> Result<Vec<TaskRewardLink>, String> {
        Self::request::<Vec<TaskRewardLink>>(
            "GET",
            &format!("/households/{}/tasks/{}/rewards", household_id, task_id),
            None::<()>,
            true,
        )
        .await
    }

    pub async fn add_task_reward(
        household_id: &str,
        task_id: &str,
        reward_id: &str,
        amount: i32,
    ) -> Result<(), String> {
        Self::request::<()>(
            "POST",
            &format!(
                "/households/{}/tasks/{}/rewards/{}?amount={}",
                household_id, task_id, reward_id, amount
            ),
            None::<()>,
            true,
        )
        .await
    }

    pub async fn remove_task_reward(
        household_id: &str,
        task_id: &str,
        reward_id: &str,
    ) -> Result<(), String> {
        Self::request::<()>(
            "DELETE",
            &format!(
                "/households/{}/tasks/{}/rewards/{}",
                household_id, task_id, reward_id
            ),
            None::<()>,
            true,
        )
        .await
    }

    pub async fn get_task_punishments(
        household_id: &str,
        task_id: &str,
    ) -> Result<Vec<TaskPunishmentLink>, String> {
        Self::request::<Vec<TaskPunishmentLink>>(
            "GET",
            &format!("/households/{}/tasks/{}/punishments", household_id, task_id),
            None::<()>,
            true,
        )
        .await
    }

    pub async fn add_task_punishment(
        household_id: &str,
        task_id: &str,
        punishment_id: &str,
        amount: i32,
    ) -> Result<(), String> {
        Self::request::<()>(
            "POST",
            &format!(
                "/households/{}/tasks/{}/punishments/{}?amount={}",
                household_id, task_id, punishment_id, amount
            ),
            None::<()>,
            true,
        )
        .await
    }

    pub async fn remove_task_punishment(
        household_id: &str,
        task_id: &str,
        punishment_id: &str,
    ) -> Result<(), String> {
        Self::request::<()>(
            "DELETE",
            &format!(
                "/households/{}/tasks/{}/punishments/{}",
                household_id, task_id, punishment_id
            ),
            None::<()>,
            true,
        )
        .await
    }

    // Task review endpoints
    pub async fn get_pending_reviews(household_id: &str) -> Result<Vec<PendingReview>, String> {
        Self::request::<Vec<PendingReview>>(
            "GET",
            &format!("/households/{}/tasks/pending-reviews", household_id),
            None::<()>,
            true,
        )
        .await
    }

    pub async fn approve_completion(household_id: &str, completion_id: &str) -> Result<TaskCompletion, String> {
        Self::request::<TaskCompletion>(
            "POST",
            &format!(
                "/households/{}/tasks/completions/{}/approve",
                household_id, completion_id
            ),
            None::<()>,
            true,
        )
        .await
    }

    pub async fn reject_completion(household_id: &str, completion_id: &str) -> Result<(), String> {
        Self::request::<()>(
            "POST",
            &format!(
                "/households/{}/tasks/completions/{}/reject",
                household_id, completion_id
            ),
            None::<()>,
            true,
        )
        .await
    }

    // Point condition endpoints
    pub async fn list_point_conditions(household_id: &str) -> Result<Vec<PointCondition>, String> {
        Self::request::<Vec<PointCondition>>(
            "GET",
            &format!("/households/{}/point-conditions", household_id),
            None::<()>,
            true,
        )
        .await
    }

    pub async fn create_point_condition(
        household_id: &str,
        request: CreatePointConditionRequest,
    ) -> Result<PointCondition, String> {
        Self::request(
            "POST",
            &format!("/households/{}/point-conditions", household_id),
            Some(request),
            true,
        )
        .await
    }

    pub async fn delete_point_condition(household_id: &str, condition_id: &str) -> Result<(), String> {
        Self::request::<()>(
            "DELETE",
            &format!("/households/{}/point-conditions/{}", household_id, condition_id),
            None::<()>,
            true,
        )
        .await
    }

    // Reward endpoints
    pub async fn list_rewards(household_id: &str) -> Result<Vec<Reward>, String> {
        Self::request::<Vec<Reward>>(
            "GET",
            &format!("/households/{}/rewards", household_id),
            None::<()>,
            true,
        )
        .await
    }

    pub async fn create_reward(household_id: &str, request: CreateRewardRequest) -> Result<Reward, String> {
        Self::request(
            "POST",
            &format!("/households/{}/rewards", household_id),
            Some(request),
            true,
        )
        .await
    }

    pub async fn update_reward(household_id: &str, reward_id: &str, request: UpdateRewardRequest) -> Result<Reward, String> {
        Self::request(
            "PUT",
            &format!("/households/{}/rewards/{}", household_id, reward_id),
            Some(request),
            true,
        )
        .await
    }

    pub async fn purchase_reward(household_id: &str, reward_id: &str) -> Result<UserReward, String> {
        Self::request::<UserReward>(
            "POST",
            &format!("/households/{}/rewards/{}/purchase", household_id, reward_id),
            None::<()>,
            true,
        )
        .await
    }

    pub async fn delete_reward(household_id: &str, reward_id: &str) -> Result<(), String> {
        Self::request::<()>(
            "DELETE",
            &format!("/households/{}/rewards/{}", household_id, reward_id),
            None::<()>,
            true,
        )
        .await
    }

    pub async fn list_user_rewards(household_id: &str) -> Result<Vec<UserReward>, String> {
        Self::request::<Vec<UserReward>>(
            "GET",
            &format!("/households/{}/rewards/user-rewards", household_id),
            None::<()>,
            true,
        )
        .await
    }

    pub async fn list_all_user_rewards(household_id: &str) -> Result<Vec<UserRewardWithUser>, String> {
        Self::request::<Vec<UserRewardWithUser>>(
            "GET",
            &format!("/households/{}/rewards/user-rewards/all", household_id),
            None::<()>,
            true,
        )
        .await
    }

    pub async fn delete_user_reward(household_id: &str, user_reward_id: &str) -> Result<(), String> {
        Self::request::<()>(
            "DELETE",
            &format!("/households/{}/rewards/user-rewards/{}", household_id, user_reward_id),
            None::<()>,
            true,
        )
        .await
    }

    pub async fn redeem_reward(household_id: &str, user_reward_id: &str) -> Result<UserReward, String> {
        Self::request::<UserReward>(
            "POST",
            &format!("/households/{}/rewards/user-rewards/{}/redeem", household_id, user_reward_id),
            None::<()>,
            true,
        )
        .await
    }

    // Reward confirmation endpoints
    pub async fn get_pending_reward_redemptions(household_id: &str) -> Result<Vec<PendingRewardRedemption>, String> {
        Self::request::<Vec<PendingRewardRedemption>>(
            "GET",
            &format!("/households/{}/rewards/pending-confirmations", household_id),
            None::<()>,
            true,
        )
        .await
    }

    pub async fn approve_reward_redemption(household_id: &str, user_reward_id: &str) -> Result<UserReward, String> {
        Self::request::<UserReward>(
            "POST",
            &format!("/households/{}/rewards/user-rewards/{}/approve", household_id, user_reward_id),
            None::<()>,
            true,
        )
        .await
    }

    pub async fn reject_reward_redemption(household_id: &str, user_reward_id: &str) -> Result<UserReward, String> {
        Self::request::<UserReward>(
            "POST",
            &format!("/households/{}/rewards/user-rewards/{}/reject", household_id, user_reward_id),
            None::<()>,
            true,
        )
        .await
    }

    // Punishment endpoints
    pub async fn list_punishments(household_id: &str) -> Result<Vec<Punishment>, String> {
        Self::request::<Vec<Punishment>>(
            "GET",
            &format!("/households/{}/punishments", household_id),
            None::<()>,
            true,
        )
        .await
    }

    pub async fn create_punishment(
        household_id: &str,
        request: CreatePunishmentRequest,
    ) -> Result<Punishment, String> {
        Self::request(
            "POST",
            &format!("/households/{}/punishments", household_id),
            Some(request),
            true,
        )
        .await
    }

    pub async fn update_punishment(
        household_id: &str,
        punishment_id: &str,
        request: UpdatePunishmentRequest,
    ) -> Result<Punishment, String> {
        Self::request(
            "PUT",
            &format!("/households/{}/punishments/{}", household_id, punishment_id),
            Some(request),
            true,
        )
        .await
    }

    pub async fn delete_punishment(household_id: &str, punishment_id: &str) -> Result<(), String> {
        Self::request::<()>(
            "DELETE",
            &format!("/households/{}/punishments/{}", household_id, punishment_id),
            None::<()>,
            true,
        )
        .await
    }

    pub async fn list_user_punishments(household_id: &str) -> Result<Vec<UserPunishment>, String> {
        Self::request::<Vec<UserPunishment>>(
            "GET",
            &format!("/households/{}/punishments/user-punishments", household_id),
            None::<()>,
            true,
        )
        .await
    }

    pub async fn list_all_user_punishments(household_id: &str) -> Result<Vec<UserPunishmentWithUser>, String> {
        Self::request::<Vec<UserPunishmentWithUser>>(
            "GET",
            &format!("/households/{}/punishments/user-punishments/all", household_id),
            None::<()>,
            true,
        )
        .await
    }

    pub async fn delete_user_punishment(household_id: &str, user_punishment_id: &str) -> Result<(), String> {
        Self::request::<()>(
            "DELETE",
            &format!("/households/{}/punishments/user-punishments/{}", household_id, user_punishment_id),
            None::<()>,
            true,
        )
        .await
    }

    // Invitation endpoints (household admin)
    pub async fn create_invitation(
        household_id: &str,
        request: CreateInvitationRequest,
    ) -> Result<Invitation, String> {
        Self::request(
            "POST",
            &format!("/households/{}/invite", household_id),
            Some(request),
            true,
        )
        .await
    }

    pub async fn list_household_invitations(household_id: &str) -> Result<Vec<Invitation>, String> {
        Self::request::<Vec<Invitation>>(
            "GET",
            &format!("/households/{}/invitations", household_id),
            None::<()>,
            true,
        )
        .await
    }

    pub async fn cancel_invitation(
        household_id: &str,
        invitation_id: &str,
    ) -> Result<(), String> {
        Self::request::<()>(
            "DELETE",
            &format!("/households/{}/invitations/{}", household_id, invitation_id),
            None::<()>,
            true,
        )
        .await
    }

    // User invitation endpoints
    pub async fn get_my_invitations() -> Result<Vec<InvitationWithHousehold>, String> {
        Self::request::<Vec<InvitationWithHousehold>>(
            "GET",
            "/invitations",
            None::<()>,
            true,
        )
        .await
    }

    pub async fn accept_invitation(invitation_id: &str) -> Result<HouseholdMembership, String> {
        Self::request::<HouseholdMembership>(
            "POST",
            &format!("/invitations/{}/accept", invitation_id),
            None::<()>,
            true,
        )
        .await
    }

    pub async fn decline_invitation(invitation_id: &str) -> Result<(), String> {
        Self::request::<()>(
            "POST",
            &format!("/invitations/{}/decline", invitation_id),
            None::<()>,
            true,
        )
        .await
    }

    // Member management endpoints
    pub async fn adjust_member_points(
        household_id: &str,
        user_id: &str,
        request: AdjustPointsRequest,
    ) -> Result<AdjustPointsResponse, String> {
        Self::request(
            "POST",
            &format!("/households/{}/members/{}/points", household_id, user_id),
            Some(request),
            true,
        )
        .await
    }

    pub async fn assign_reward(
        household_id: &str,
        reward_id: &str,
        user_id: &str,
    ) -> Result<UserReward, String> {
        Self::request::<UserReward>(
            "POST",
            &format!("/households/{}/rewards/{}/assign/{}", household_id, reward_id, user_id),
            None::<()>,
            true,
        )
        .await
    }

    pub async fn assign_punishment(
        household_id: &str,
        punishment_id: &str,
        user_id: &str,
    ) -> Result<UserPunishment, String> {
        Self::request::<UserPunishment>(
            "POST",
            &format!("/households/{}/punishments/{}/assign/{}", household_id, punishment_id, user_id),
            None::<()>,
            true,
        )
        .await
    }

    pub async fn unassign_reward(
        household_id: &str,
        reward_id: &str,
        user_id: &str,
    ) -> Result<(), String> {
        Self::request::<()>(
            "POST",
            &format!("/households/{}/rewards/{}/unassign/{}", household_id, reward_id, user_id),
            None::<()>,
            true,
        )
        .await
    }

    pub async fn unassign_punishment(
        household_id: &str,
        punishment_id: &str,
        user_id: &str,
    ) -> Result<(), String> {
        Self::request::<()>(
            "POST",
            &format!("/households/{}/punishments/{}/unassign/{}", household_id, punishment_id, user_id),
            None::<()>,
            true,
        )
        .await
    }

    pub async fn complete_punishment(
        household_id: &str,
        user_punishment_id: &str,
    ) -> Result<UserPunishment, String> {
        Self::request::<UserPunishment>(
            "POST",
            &format!("/households/{}/punishments/user-punishments/{}/complete", household_id, user_punishment_id),
            None::<()>,
            true,
        )
        .await
    }

    // Punishment confirmation endpoints
    pub async fn get_pending_punishment_completions(household_id: &str) -> Result<Vec<PendingPunishmentCompletion>, String> {
        Self::request::<Vec<PendingPunishmentCompletion>>(
            "GET",
            &format!("/households/{}/punishments/pending-confirmations", household_id),
            None::<()>,
            true,
        )
        .await
    }

    pub async fn approve_punishment_completion(household_id: &str, user_punishment_id: &str) -> Result<UserPunishment, String> {
        Self::request::<UserPunishment>(
            "POST",
            &format!("/households/{}/punishments/user-punishments/{}/approve", household_id, user_punishment_id),
            None::<()>,
            true,
        )
        .await
    }

    pub async fn reject_punishment_completion(household_id: &str, user_punishment_id: &str) -> Result<UserPunishment, String> {
        Self::request::<UserPunishment>(
            "POST",
            &format!("/households/{}/punishments/user-punishments/{}/reject", household_id, user_punishment_id),
            None::<()>,
            true,
        )
        .await
    }

    // Activity log endpoints
    pub async fn list_activities(
        household_id: &str,
        limit: Option<i64>,
    ) -> Result<Vec<ActivityLogWithUsers>, String> {
        let url = if let Some(limit) = limit {
            format!("/households/{}/activities?limit={}", household_id, limit)
        } else {
            format!("/households/{}/activities", household_id)
        };
        Self::request::<Vec<ActivityLogWithUsers>>("GET", &url, None::<()>, true).await
    }

    // Chat endpoints
    pub async fn list_chat_messages(
        household_id: &str,
        limit: Option<i64>,
        before: Option<&str>,
    ) -> Result<Vec<ChatMessageWithUser>, String> {
        let mut url = format!("/households/{}/chat", household_id);
        let mut params = Vec::new();
        if let Some(limit) = limit {
            params.push(format!("limit={}", limit));
        }
        if let Some(before) = before {
            params.push(format!("before={}", before));
        }
        if !params.is_empty() {
            url = format!("{}?{}", url, params.join("&"));
        }
        Self::request::<Vec<ChatMessageWithUser>>("GET", &url, None::<()>, true).await
    }

    pub async fn send_chat_message(
        household_id: &str,
        content: &str,
    ) -> Result<ChatMessageWithUser, String> {
        Self::request(
            "POST",
            &format!("/households/{}/chat", household_id),
            Some(CreateChatMessageRequest {
                content: content.to_string(),
            }),
            true,
        )
        .await
    }

    pub async fn update_chat_message(
        household_id: &str,
        message_id: &str,
        content: &str,
    ) -> Result<ChatMessageWithUser, String> {
        Self::request(
            "PUT",
            &format!("/households/{}/chat/{}", household_id, message_id),
            Some(UpdateChatMessageRequest {
                content: content.to_string(),
            }),
            true,
        )
        .await
    }

    pub async fn delete_chat_message(
        household_id: &str,
        message_id: &str,
    ) -> Result<(), String> {
        Self::request::<()>(
            "DELETE",
            &format!("/households/{}/chat/{}", household_id, message_id),
            None::<()>,
            true,
        )
        .await
    }

    // Notes endpoints
    pub async fn list_notes(household_id: &str) -> Result<Vec<NoteWithUser>, String> {
        Self::request::<Vec<NoteWithUser>>(
            "GET",
            &format!("/households/{}/notes", household_id),
            None::<()>,
            true,
        )
        .await
    }

    pub async fn create_note(
        household_id: &str,
        request: CreateNoteRequest,
    ) -> Result<Note, String> {
        Self::request(
            "POST",
            &format!("/households/{}/notes", household_id),
            Some(request),
            true,
        )
        .await
    }

    pub async fn get_note(household_id: &str, note_id: &str) -> Result<Note, String> {
        Self::request::<Note>(
            "GET",
            &format!("/households/{}/notes/{}", household_id, note_id),
            None::<()>,
            true,
        )
        .await
    }

    pub async fn update_note(
        household_id: &str,
        note_id: &str,
        request: UpdateNoteRequest,
    ) -> Result<Note, String> {
        Self::request(
            "PUT",
            &format!("/households/{}/notes/{}", household_id, note_id),
            Some(request),
            true,
        )
        .await
    }

    pub async fn delete_note(household_id: &str, note_id: &str) -> Result<(), String> {
        Self::request::<()>(
            "DELETE",
            &format!("/households/{}/notes/{}", household_id, note_id),
            None::<()>,
            true,
        )
        .await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use wasm_bindgen_test::*;

    wasm_bindgen_test_configure!(run_in_browser);

    #[wasm_bindgen_test]
    fn test_api_base_constant() {
        assert_eq!(API_BASE, "/api");
    }

    #[wasm_bindgen_test]
    fn test_token_key_constant() {
        assert_eq!(TOKEN_KEY, "auth_token");
    }

    #[wasm_bindgen_test]
    fn test_url_format_households() {
        let url = format!("{}/households", API_BASE);
        assert_eq!(url, "/api/households");
    }

    #[wasm_bindgen_test]
    fn test_url_format_household_tasks() {
        let household_id = "abc-123";
        let url = format!("{}/households/{}/tasks", API_BASE, household_id);
        assert_eq!(url, "/api/households/abc-123/tasks");
    }

    #[wasm_bindgen_test]
    fn test_url_format_task_complete() {
        let household_id = "house-1";
        let task_id = "task-1";
        let url = format!(
            "{}/households/{}/tasks/{}/complete",
            API_BASE, household_id, task_id
        );
        assert_eq!(url, "/api/households/house-1/tasks/task-1/complete");
    }

    #[wasm_bindgen_test]
    fn test_url_format_task_rewards() {
        let household_id = "h1";
        let task_id = "t1";
        let reward_id = "r1";
        let url = format!(
            "{}/households/{}/tasks/{}/rewards/{}",
            API_BASE, household_id, task_id, reward_id
        );
        assert_eq!(url, "/api/households/h1/tasks/t1/rewards/r1");
    }

    #[wasm_bindgen_test]
    fn test_url_format_invitations() {
        let invitation_id = "inv-123";
        let url = format!("{}/invitations/{}/accept", API_BASE, invitation_id);
        assert_eq!(url, "/api/invitations/inv-123/accept");
    }

    #[wasm_bindgen_test]
    fn test_url_format_member_points() {
        let household_id = "h1";
        let user_id = "u1";
        let url = format!(
            "{}/households/{}/members/{}/points",
            API_BASE, household_id, user_id
        );
        assert_eq!(url, "/api/households/h1/members/u1/points");
    }
}
