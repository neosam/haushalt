use gloo_net::http::Request;
use gloo_storage::{LocalStorage, Storage};
use leptos::*;
use serde::{de::DeserializeOwned, Serialize};
use shared::{
    AdjustPointsRequest, AdjustPointsResponse, ApiError, ApiSuccess, AuthResponse,
    CreateHouseholdRequest, CreateInvitationRequest, CreatePointConditionRequest,
    CreatePunishmentRequest, CreateRewardRequest, CreateTaskRequest, CreateUserRequest, Household,
    HouseholdMembership, Invitation, InvitationWithHousehold, InviteUserRequest, LeaderboardEntry,
    LoginRequest, MemberWithUser, PointCondition, Punishment, Reward, Task, TaskCompletion,
    TaskWithStatus, UpdateTaskRequest, User, UserPunishment, UserPunishmentWithUser, UserReward,
    UserRewardWithUser,
};

const API_BASE: &str = "/api";
const TOKEN_KEY: &str = "auth_token";

#[derive(Clone)]
pub struct AuthState {
    pub token: RwSignal<Option<String>>,
    pub user: RwSignal<Option<User>>,
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
    pub async fn get_task_rewards(household_id: &str, task_id: &str) -> Result<Vec<Reward>, String> {
        Self::request::<Vec<Reward>>(
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
    ) -> Result<(), String> {
        Self::request::<()>(
            "POST",
            &format!(
                "/households/{}/tasks/{}/rewards/{}",
                household_id, task_id, reward_id
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
    ) -> Result<Vec<Punishment>, String> {
        Self::request::<Vec<Punishment>>(
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
    ) -> Result<(), String> {
        Self::request::<()>(
            "POST",
            &format!(
                "/households/{}/tasks/{}/punishments/{}",
                household_id, task_id, punishment_id
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

    pub async fn redeem_reward(household_id: &str, user_reward_id: &str) -> Result<UserReward, String> {
        Self::request::<UserReward>(
            "POST",
            &format!("/households/{}/rewards/user-rewards/{}/redeem", household_id, user_reward_id),
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
}
