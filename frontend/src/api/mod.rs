pub mod websocket;

use gloo_net::http::Request;
use gloo_storage::{LocalStorage, Storage};
use leptos::*;
use serde::{de::DeserializeOwned, Serialize};
use shared::{
    ActivityLogWithUsers, AdjustPointsRequest, AdjustPointsResponse, Announcement, ApiError, ApiSuccess,
    AuthResponse, ChatMessageWithUser, CreateAnnouncementRequest, CreateChatMessageRequest,
    CreateHouseholdRequest, CreateInvitationRequest, CreateJournalEntryRequest, CreateNoteRequest,
    CreatePointConditionRequest, CreatePunishmentRequest, CreateRewardRequest, CreateTaskRequest,
    CreateUserRequest, Household, HouseholdMembership, HouseholdSettings, Invitation, InvitationWithHousehold,
    InviteUserRequest, JournalEntry, JournalEntryWithUser, LeaderboardEntry, LoginRequest, MemberWithUser,
    Note, NoteWithUser, PendingPunishmentCompletion, PendingReview, PendingRewardRedemption, PointCondition,
    Punishment, RandomPickResult, RandomRewardPickResult, RefreshTokenRequest, Reward, Task, TaskCompletion,
    TaskPunishmentLink, TaskRewardLink, TaskWithDetails, TaskWithStatus, UpdateAnnouncementRequest,
    UpdateChatMessageRequest, UpdateHouseholdSettingsRequest, UpdateJournalEntryRequest, UpdateNoteRequest,
    UpdatePunishmentRequest, UpdateRewardRequest, UpdateRoleRequest, UpdateTaskRequest,
    UpdateUserSettingsRequest, User, UserPunishment, UserPunishmentWithUser, UserReward, UserRewardWithUser,
    UserSettings,
};

use std::sync::atomic::{AtomicBool, Ordering};

const API_BASE: &str = "/api";
const TOKEN_KEY: &str = "auth_token";
const REFRESH_TOKEN_KEY: &str = "refresh_token";

/// Global flag to signal that authentication has failed and user should re-login
static AUTH_FAILED: AtomicBool = AtomicBool::new(false);

/// Global flag to prevent concurrent token refresh attempts
static REFRESH_IN_PROGRESS: AtomicBool = AtomicBool::new(false);

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
        // Check if auth has failed globally (e.g., refresh token expired)
        if AUTH_FAILED.load(Ordering::Relaxed) {
            return false;
        }
        self.token.get().is_some()
    }

    /// Check and clear the auth failed flag, returning true if it was set
    pub fn check_and_clear_auth_failed(&self) -> bool {
        if AUTH_FAILED.swap(false, Ordering::Relaxed) {
            self.token.set(None);
            self.user.set(None);
            true
        } else {
            false
        }
    }

    pub fn set_auth(&self, response: AuthResponse) {
        LocalStorage::set(TOKEN_KEY, &response.token).ok();
        LocalStorage::set(REFRESH_TOKEN_KEY, &response.refresh_token).ok();
        self.token.set(Some(response.token));
        self.user.set(Some(response.user));
    }

    pub fn logout(&self) {
        // Try to call server-side logout (fire and forget)
        if let Some(refresh_token) = Self::get_refresh_token() {
            wasm_bindgen_futures::spawn_local(async move {
                let _ = ApiClient::logout(refresh_token).await;
            });
        }
        LocalStorage::delete(TOKEN_KEY);
        LocalStorage::delete(REFRESH_TOKEN_KEY);
        self.token.set(None);
        self.user.set(None);
    }

    fn get_refresh_token() -> Option<String> {
        LocalStorage::get(REFRESH_TOKEN_KEY).ok()
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

    fn get_refresh_token() -> Option<String> {
        LocalStorage::get(REFRESH_TOKEN_KEY).ok()
    }

    fn store_tokens(token: &str, refresh_token: &str) {
        LocalStorage::set(TOKEN_KEY, token).ok();
        LocalStorage::set(REFRESH_TOKEN_KEY, refresh_token).ok();
        // Clear any auth failure flag since we have valid tokens
        AUTH_FAILED.store(false, Ordering::Relaxed);
    }

    fn clear_tokens() {
        LocalStorage::delete(TOKEN_KEY);
        LocalStorage::delete(REFRESH_TOKEN_KEY);
    }

    /// Attempt to refresh tokens, ensuring only one refresh happens at a time.
    /// If a refresh is already in progress, waits briefly and checks if tokens are now valid.
    async fn try_refresh_token() -> Result<(), String> {
        // If refresh is already in progress, wait and check if tokens are valid
        if REFRESH_IN_PROGRESS.load(Ordering::Relaxed) {
            // Wait a bit for the other refresh to complete
            gloo_timers::future::TimeoutFuture::new(100).await;
            // Check if we now have a valid token (other refresh succeeded)
            if Self::get_token().is_some() {
                return Ok(());
            }
            // Still no token - the other refresh must have failed
            return Err("Refresh failed".to_string());
        }

        // Mark refresh as in progress
        REFRESH_IN_PROGRESS.store(true, Ordering::Relaxed);

        let result = Self::do_refresh().await;

        // Mark refresh as complete
        REFRESH_IN_PROGRESS.store(false, Ordering::Relaxed);

        result
    }

    /// Perform the actual token refresh
    async fn do_refresh() -> Result<(), String> {
        let refresh_token = Self::get_refresh_token()
            .ok_or_else(|| "No refresh token".to_string())?;

        match Self::refresh_token_request(refresh_token).await {
            Ok(auth_response) => {
                Self::store_tokens(&auth_response.token, &auth_response.refresh_token);
                Ok(())
            }
            Err(e) => Err(e),
        }
    }

    /// Execute a single HTTP request without retry logic
    async fn execute_request<T: DeserializeOwned>(
        method: &str,
        path: &str,
        body_json: Option<String>,
        auth: bool,
    ) -> Result<(T, u16), String> {
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

        let response = if let Some(json) = body_json {
            request
                .header("Content-Type", "application/json")
                .body(json)
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

        let status = response.status();
        if response.ok() {
            let result: ApiSuccess<T> = response.json().await.map_err(|e| e.to_string())?;
            Ok((result.data, status))
        } else {
            let error: ApiError = response
                .json()
                .await
                .unwrap_or(ApiError {
                    error: "unknown".to_string(),
                    message: "An unknown error occurred".to_string(),
                });
            Err(format!("{}|{}", status, error.message))
        }
    }

    async fn request<T: DeserializeOwned>(
        method: &str,
        path: &str,
        body: Option<impl Serialize>,
        auth: bool,
    ) -> Result<T, String> {
        // Serialize body once so we can retry if needed
        let body_json = body.and_then(|b| serde_json::to_string(&b).ok());

        // First attempt
        match Self::execute_request::<T>(method, path, body_json.clone(), auth).await {
            Ok((data, _)) => Ok(data),
            Err(e) => {
                // Check if it's a 401 and we should try refresh
                if auth && e.starts_with("401|") {
                    // Try synchronized refresh (prevents race conditions with concurrent requests)
                    if Self::try_refresh_token().await.is_ok() {
                        // Retry with new token
                        return match Self::execute_request::<T>(method, path, body_json, auth).await {
                            Ok((data, _)) => Ok(data),
                            Err(e2) => {
                                // Extract error message from "status|message" format
                                let msg = e2.split('|').nth(1).unwrap_or(&e2);
                                Err(msg.to_string())
                            }
                        };
                    }
                    // Refresh failed, clear tokens and signal auth failure
                    Self::clear_tokens();
                    AUTH_FAILED.store(true, Ordering::Relaxed);
                    return Err("Session expired. Please log in again.".to_string());
                }
                // Not a 401, return the error message
                let msg = e.split('|').nth(1).unwrap_or(&e);
                Err(msg.to_string())
            }
        }
    }

    async fn refresh_token_request(refresh_token: String) -> Result<AuthResponse, String> {
        let url = format!("{}/auth/refresh", API_BASE);
        let response = Request::post(&url)
            .header("Content-Type", "application/json")
            .json(&RefreshTokenRequest { refresh_token })
            .map_err(|e| e.to_string())?
            .send()
            .await
            .map_err(|e| e.to_string())?;

        if response.ok() {
            let result: ApiSuccess<AuthResponse> = response.json().await.map_err(|e| e.to_string())?;
            Ok(result.data)
        } else {
            Err("Failed to refresh token".to_string())
        }
    }

    async fn logout(refresh_token: String) -> Result<(), String> {
        let url = format!("{}/auth/logout", API_BASE);
        let response = Request::post(&url)
            .header("Content-Type", "application/json")
            .json(&RefreshTokenRequest { refresh_token })
            .map_err(|e| e.to_string())?
            .send()
            .await
            .map_err(|e| e.to_string())?;

        if response.ok() {
            Ok(())
        } else {
            Err("Failed to logout".to_string())
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

    /// Get full task details including statistics for the detail view
    pub async fn get_task_details(
        household_id: &str,
        task_id: &str,
    ) -> Result<TaskWithDetails, String> {
        Self::request::<TaskWithDetails>(
            "GET",
            &format!("/households/{}/tasks/{}/details", household_id, task_id),
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

    pub async fn archive_task(household_id: &str, task_id: &str) -> Result<Task, String> {
        Self::request::<Task>(
            "POST",
            &format!("/households/{}/tasks/{}/archive", household_id, task_id),
            None::<()>,
            true,
        )
        .await
    }

    pub async fn unarchive_task(household_id: &str, task_id: &str) -> Result<Task, String> {
        Self::request::<Task>(
            "POST",
            &format!("/households/{}/tasks/{}/unarchive", household_id, task_id),
            None::<()>,
            true,
        )
        .await
    }

    pub async fn pause_task(household_id: &str, task_id: &str) -> Result<Task, String> {
        Self::request::<Task>(
            "POST",
            &format!("/households/{}/tasks/{}/pause", household_id, task_id),
            None::<()>,
            true,
        )
        .await
    }

    pub async fn unpause_task(household_id: &str, task_id: &str) -> Result<Task, String> {
        Self::request::<Task>(
            "POST",
            &format!("/households/{}/tasks/{}/unpause", household_id, task_id),
            None::<()>,
            true,
        )
        .await
    }

    pub async fn list_archived_tasks(household_id: &str) -> Result<Vec<Task>, String> {
        Self::request::<Vec<Task>>(
            "GET",
            &format!("/households/{}/tasks/archived", household_id),
            None::<()>,
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

    // Task category endpoints
    pub async fn list_categories(
        household_id: &str,
    ) -> Result<Vec<shared::TaskCategory>, String> {
        let response: shared::TaskCategoriesResponse = Self::request(
            "GET",
            &format!("/households/{}/categories", household_id),
            None::<()>,
            true,
        )
        .await?;
        Ok(response.categories)
    }

    pub async fn create_category(
        household_id: &str,
        request: shared::CreateTaskCategoryRequest,
    ) -> Result<shared::TaskCategory, String> {
        Self::request(
            "POST",
            &format!("/households/{}/categories", household_id),
            Some(request),
            true,
        )
        .await
    }

    pub async fn update_category(
        household_id: &str,
        category_id: &str,
        request: shared::UpdateTaskCategoryRequest,
    ) -> Result<shared::TaskCategory, String> {
        Self::request(
            "PUT",
            &format!("/households/{}/categories/{}", household_id, category_id),
            Some(request),
            true,
        )
        .await
    }

    pub async fn delete_category(household_id: &str, category_id: &str) -> Result<(), String> {
        Self::request::<()>(
            "DELETE",
            &format!("/households/{}/categories/{}", household_id, category_id),
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

    // Task suggestion endpoints
    pub async fn list_suggestions(household_id: &str) -> Result<Vec<Task>, String> {
        Self::request::<Vec<Task>>(
            "GET",
            &format!("/households/{}/tasks/suggestions", household_id),
            None::<()>,
            true,
        )
        .await
    }

    pub async fn approve_suggestion(household_id: &str, task_id: &str) -> Result<Task, String> {
        Self::request::<Task>(
            "POST",
            &format!("/households/{}/tasks/{}/approve", household_id, task_id),
            None::<()>,
            true,
        )
        .await
    }

    pub async fn deny_suggestion(household_id: &str, task_id: &str) -> Result<Task, String> {
        Self::request::<Task>(
            "POST",
            &format!("/households/{}/tasks/{}/deny", household_id, task_id),
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

    /// Get the options linked to a random choice reward
    pub async fn get_reward_options(household_id: &str, reward_id: &str) -> Result<Vec<Reward>, String> {
        Self::request::<Vec<Reward>>(
            "GET",
            &format!("/households/{}/rewards/{}/options", household_id, reward_id),
            None::<()>,
            true,
        )
        .await
    }

    /// Pick a random reward from a user's random choice reward assignment
    pub async fn pick_random_reward(household_id: &str, user_reward_id: &str) -> Result<RandomRewardPickResult, String> {
        Self::request::<RandomRewardPickResult>(
            "POST",
            &format!("/households/{}/rewards/user-rewards/{}/pick", household_id, user_reward_id),
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

    pub async fn update_member_role(
        household_id: &str,
        user_id: &str,
        request: UpdateRoleRequest,
    ) -> Result<HouseholdMembership, String> {
        Self::request(
            "PUT",
            &format!("/households/{}/members/{}/role", household_id, user_id),
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

    /// Get the options linked to a random choice punishment
    pub async fn get_punishment_options(household_id: &str, punishment_id: &str) -> Result<Vec<Punishment>, String> {
        Self::request::<Vec<Punishment>>(
            "GET",
            &format!("/households/{}/punishments/{}/options", household_id, punishment_id),
            None::<()>,
            true,
        )
        .await
    }

    /// Pick a random punishment from a user's random choice punishment assignment
    pub async fn pick_random_punishment(household_id: &str, user_punishment_id: &str) -> Result<RandomPickResult, String> {
        Self::request::<RandomPickResult>(
            "POST",
            &format!("/households/{}/punishments/user-punishments/{}/pick", household_id, user_punishment_id),
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

    // Journal endpoints
    pub async fn list_journal_entries(household_id: &str) -> Result<Vec<JournalEntryWithUser>, String> {
        Self::request::<Vec<JournalEntryWithUser>>(
            "GET",
            &format!("/households/{}/journal", household_id),
            None::<()>,
            true,
        )
        .await
    }

    pub async fn create_journal_entry(
        household_id: &str,
        request: CreateJournalEntryRequest,
    ) -> Result<JournalEntry, String> {
        Self::request(
            "POST",
            &format!("/households/{}/journal", household_id),
            Some(request),
            true,
        )
        .await
    }

    pub async fn get_journal_entry(household_id: &str, entry_id: &str) -> Result<JournalEntry, String> {
        Self::request::<JournalEntry>(
            "GET",
            &format!("/households/{}/journal/{}", household_id, entry_id),
            None::<()>,
            true,
        )
        .await
    }

    pub async fn update_journal_entry(
        household_id: &str,
        entry_id: &str,
        request: UpdateJournalEntryRequest,
    ) -> Result<JournalEntry, String> {
        Self::request(
            "PUT",
            &format!("/households/{}/journal/{}", household_id, entry_id),
            Some(request),
            true,
        )
        .await
    }

    pub async fn delete_journal_entry(household_id: &str, entry_id: &str) -> Result<(), String> {
        Self::request::<()>(
            "DELETE",
            &format!("/households/{}/journal/{}", household_id, entry_id),
            None::<()>,
            true,
        )
        .await
    }

    // Announcement endpoints
    pub async fn list_announcements(household_id: &str) -> Result<Vec<Announcement>, String> {
        Self::request::<Vec<Announcement>>(
            "GET",
            &format!("/households/{}/announcements", household_id),
            None::<()>,
            true,
        )
        .await
    }

    pub async fn list_active_announcements(household_id: &str) -> Result<Vec<Announcement>, String> {
        Self::request::<Vec<Announcement>>(
            "GET",
            &format!("/households/{}/announcements/active", household_id),
            None::<()>,
            true,
        )
        .await
    }

    pub async fn create_announcement(
        household_id: &str,
        request: CreateAnnouncementRequest,
    ) -> Result<Announcement, String> {
        Self::request(
            "POST",
            &format!("/households/{}/announcements", household_id),
            Some(request),
            true,
        )
        .await
    }

    pub async fn update_announcement(
        household_id: &str,
        announcement_id: &str,
        request: UpdateAnnouncementRequest,
    ) -> Result<Announcement, String> {
        Self::request(
            "PUT",
            &format!("/households/{}/announcements/{}", household_id, announcement_id),
            Some(request),
            true,
        )
        .await
    }

    pub async fn delete_announcement(household_id: &str, announcement_id: &str) -> Result<(), String> {
        Self::request::<()>(
            "DELETE",
            &format!("/households/{}/announcements/{}", household_id, announcement_id),
            None::<()>,
            true,
        )
        .await
    }

    // User settings endpoints
    pub async fn get_user_settings() -> Result<UserSettings, String> {
        Self::request::<UserSettings>("GET", "/users/me/settings", None::<()>, true).await
    }

    pub async fn update_user_settings(
        request: UpdateUserSettingsRequest,
    ) -> Result<UserSettings, String> {
        Self::request("PUT", "/users/me/settings", Some(request), true).await
    }

    // Dashboard task whitelist endpoints
    pub async fn get_dashboard_task_ids() -> Result<Vec<uuid::Uuid>, String> {
        let response: shared::DashboardTasksResponse =
            Self::request("GET", "/dashboard/tasks", None::<()>, true).await?;
        Ok(response.task_ids)
    }

    pub async fn is_task_on_dashboard(task_id: &str) -> Result<bool, String> {
        let response: shared::IsTaskOnDashboardResponse =
            Self::request("GET", &format!("/dashboard/tasks/{}", task_id), None::<()>, true).await?;
        Ok(response.on_dashboard)
    }

    pub async fn add_task_to_dashboard(task_id: &str) -> Result<(), String> {
        Self::request::<()>("POST", &format!("/dashboard/tasks/{}", task_id), None::<()>, true).await
    }

    pub async fn remove_task_from_dashboard(task_id: &str) -> Result<(), String> {
        Self::request::<()>("DELETE", &format!("/dashboard/tasks/{}", task_id), None::<()>, true).await
    }

    pub async fn get_dashboard_tasks_with_status(
    ) -> Result<Vec<shared::DashboardTaskWithHousehold>, String> {
        let response: shared::DashboardTasksWithStatusResponse =
            Self::request("GET", "/dashboard/tasks/details", None::<()>, true).await?;
        Ok(response.tasks)
    }

    /// Get all tasks from all households the user is a member of
    /// Used by the "Show all" toggle on the dashboard
    pub async fn get_all_tasks_across_households(
    ) -> Result<Vec<shared::DashboardTaskWithHousehold>, String> {
        let response: shared::DashboardTasksWithStatusResponse =
            Self::request("GET", "/dashboard/tasks/all", None::<()>, true).await?;
        Ok(response.tasks)
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
