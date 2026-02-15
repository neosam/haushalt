use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use std::str::FromStr;
use uuid::Uuid;

// ============================================================================
// User Types
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    pub id: Uuid,
    pub username: String,
    pub email: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateUserRequest {
    pub username: String,
    pub email: String,
    pub password: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoginRequest {
    pub username: String,
    pub password: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthResponse {
    pub token: String,
    pub user: User,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateUserRequest {
    pub username: Option<String>,
    pub email: Option<String>,
}

// ============================================================================
// Household Types
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Household {
    pub id: Uuid,
    pub name: String,
    pub owner_id: Uuid,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateHouseholdRequest {
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateHouseholdRequest {
    pub name: Option<String>,
}

// ============================================================================
// Household Settings Types
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HouseholdSettings {
    pub household_id: Uuid,
    pub dark_mode: bool,
    pub role_label_owner: String,
    pub role_label_admin: String,
    pub role_label_member: String,
    pub updated_at: DateTime<Utc>,
}

impl Default for HouseholdSettings {
    fn default() -> Self {
        Self {
            household_id: Uuid::nil(),
            dark_mode: false,
            role_label_owner: "Owner".to_string(),
            role_label_admin: "Admin".to_string(),
            role_label_member: "Member".to_string(),
            updated_at: Utc::now(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateHouseholdSettingsRequest {
    pub dark_mode: Option<bool>,
    pub role_label_owner: Option<String>,
    pub role_label_admin: Option<String>,
    pub role_label_member: Option<String>,
}

// ============================================================================
// Membership Types
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Role {
    Owner,
    Admin,
    Member,
}

impl Role {
    pub fn as_str(&self) -> &'static str {
        match self {
            Role::Owner => "owner",
            Role::Admin => "admin",
            Role::Member => "member",
        }
    }

    pub fn can_manage_members(&self) -> bool {
        matches!(self, Role::Owner | Role::Admin)
    }

    pub fn can_manage_tasks(&self) -> bool {
        matches!(self, Role::Owner | Role::Admin)
    }

    pub fn can_manage_rewards(&self) -> bool {
        matches!(self, Role::Owner | Role::Admin)
    }

    pub fn can_manage_roles(&self) -> bool {
        matches!(self, Role::Owner)
    }

    pub fn can_delete_household(&self) -> bool {
        matches!(self, Role::Owner)
    }
}

impl FromStr for Role {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "owner" => Ok(Role::Owner),
            "admin" => Ok(Role::Admin),
            "member" => Ok(Role::Member),
            _ => Err(()),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HouseholdMembership {
    pub id: Uuid,
    pub household_id: Uuid,
    pub user_id: Uuid,
    pub role: Role,
    pub points: i64,
    pub joined_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemberWithUser {
    pub membership: HouseholdMembership,
    pub user: User,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InviteUserRequest {
    pub email: String,
    pub role: Option<Role>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateRoleRequest {
    pub role: Role,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdjustPointsRequest {
    pub points: i64,
    pub reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdjustPointsResponse {
    pub new_points: i64,
}

// ============================================================================
// Task Types
// ============================================================================

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum RecurrenceType {
    Daily,
    Weekly,
    Monthly,
    Weekdays,
    Custom,
    #[serde(rename = "onetime", alias = "none")]
    OneTime,
}

impl RecurrenceType {
    pub fn as_str(&self) -> &'static str {
        match self {
            RecurrenceType::Daily => "daily",
            RecurrenceType::Weekly => "weekly",
            RecurrenceType::Monthly => "monthly",
            RecurrenceType::Weekdays => "weekdays",
            RecurrenceType::Custom => "custom",
            RecurrenceType::OneTime => "onetime",
        }
    }
}

impl FromStr for RecurrenceType {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "daily" => Ok(RecurrenceType::Daily),
            "weekly" => Ok(RecurrenceType::Weekly),
            "monthly" => Ok(RecurrenceType::Monthly),
            "weekdays" => Ok(RecurrenceType::Weekdays),
            "custom" => Ok(RecurrenceType::Custom),
            "onetime" | "none" => Ok(RecurrenceType::OneTime), // backward compat for "none"
            _ => Err(()),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RecurrenceValue {
    /// For weekly: day of week (0 = Sunday, 1 = Monday, etc.)
    WeekDay(u8),
    /// For monthly: day of month (1-31)
    MonthDay(u8),
    /// For weekdays: array of weekday numbers (0-6)
    Weekdays(Vec<u8>),
    /// For custom: array of specific dates
    CustomDates(Vec<NaiveDate>),
    /// For daily: no value needed
    None,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TimePeriod {
    Day,
    Week,
    Month,
    Year,
    None,
}

impl TimePeriod {
    pub fn as_str(&self) -> &'static str {
        match self {
            TimePeriod::Day => "day",
            TimePeriod::Week => "week",
            TimePeriod::Month => "month",
            TimePeriod::Year => "year",
            TimePeriod::None => "none",
        }
    }
}

impl FromStr for TimePeriod {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "day" => Ok(TimePeriod::Day),
            "week" => Ok(TimePeriod::Week),
            "month" => Ok(TimePeriod::Month),
            "year" => Ok(TimePeriod::Year),
            "none" => Ok(TimePeriod::None),
            _ => Err(()),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Task {
    pub id: Uuid,
    pub household_id: Uuid,
    pub title: String,
    pub description: String,
    pub recurrence_type: RecurrenceType,
    pub recurrence_value: Option<RecurrenceValue>,
    pub assigned_user_id: Option<Uuid>,
    pub target_count: i32,
    pub time_period: Option<TimePeriod>,
    /// When true, users can track completions beyond the target count.
    /// When false, the complete button is disabled once target is reached.
    pub allow_exceed_target: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateTaskRequest {
    pub title: String,
    pub description: Option<String>,
    pub recurrence_type: RecurrenceType,
    pub recurrence_value: Option<RecurrenceValue>,
    pub assigned_user_id: Option<Uuid>,
    pub target_count: Option<i32>,
    pub time_period: Option<TimePeriod>,
    /// When true (default), users can track completions beyond the target count.
    pub allow_exceed_target: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateTaskRequest {
    pub title: Option<String>,
    pub description: Option<String>,
    pub recurrence_type: Option<RecurrenceType>,
    pub recurrence_value: Option<RecurrenceValue>,
    pub assigned_user_id: Option<Uuid>,
    pub target_count: Option<i32>,
    pub time_period: Option<TimePeriod>,
    pub allow_exceed_target: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskCompletion {
    pub id: Uuid,
    pub task_id: Uuid,
    pub user_id: Uuid,
    pub completed_at: DateTime<Utc>,
    pub due_date: NaiveDate,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskWithStatus {
    pub task: Task,
    pub completions_today: i32,
    pub current_streak: i32,
    pub last_completion: Option<DateTime<Utc>>,
    /// Next date when this task is due (None for OneTime tasks)
    pub next_due_date: Option<NaiveDate>,
}

impl TaskWithStatus {
    /// Returns true if the target for the current period is met
    pub fn is_target_met(&self) -> bool {
        self.completions_today >= self.task.target_count
    }

    /// Returns remaining completions needed to meet the target
    pub fn remaining(&self) -> i32 {
        (self.task.target_count - self.completions_today).max(0)
    }

    /// Returns true if the user can add more completions
    /// This is false when target is met AND allow_exceed_target is false
    pub fn can_complete(&self) -> bool {
        self.task.allow_exceed_target || !self.is_target_met()
    }
}

// ============================================================================
// Point Condition Types
// ============================================================================

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConditionType {
    TaskComplete,
    TaskMissed,
    Streak,
    StreakBroken,
}

impl ConditionType {
    pub fn as_str(&self) -> &'static str {
        match self {
            ConditionType::TaskComplete => "task_complete",
            ConditionType::TaskMissed => "task_missed",
            ConditionType::Streak => "streak",
            ConditionType::StreakBroken => "streak_broken",
        }
    }
}

impl FromStr for ConditionType {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "task_complete" => Ok(ConditionType::TaskComplete),
            "task_missed" => Ok(ConditionType::TaskMissed),
            "streak" => Ok(ConditionType::Streak),
            "streak_broken" => Ok(ConditionType::StreakBroken),
            _ => Err(()),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PointCondition {
    pub id: Uuid,
    pub household_id: Uuid,
    pub name: String,
    pub condition_type: ConditionType,
    pub points_value: i64,
    pub streak_threshold: Option<i32>,
    pub multiplier: Option<f64>,
    pub task_id: Option<Uuid>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreatePointConditionRequest {
    pub name: String,
    pub condition_type: ConditionType,
    pub points_value: i64,
    pub streak_threshold: Option<i32>,
    pub multiplier: Option<f64>,
    pub task_id: Option<Uuid>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdatePointConditionRequest {
    pub name: Option<String>,
    pub condition_type: Option<ConditionType>,
    pub points_value: Option<i64>,
    pub streak_threshold: Option<i32>,
    pub multiplier: Option<f64>,
    pub task_id: Option<Uuid>,
}

// ============================================================================
// Reward Types
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Reward {
    pub id: Uuid,
    pub household_id: Uuid,
    pub name: String,
    pub description: String,
    pub point_cost: Option<i64>,
    pub is_purchasable: bool,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateRewardRequest {
    pub name: String,
    pub description: Option<String>,
    pub point_cost: Option<i64>,
    pub is_purchasable: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateRewardRequest {
    pub name: Option<String>,
    pub description: Option<String>,
    pub point_cost: Option<i64>,
    pub is_purchasable: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserReward {
    pub id: Uuid,
    pub user_id: Uuid,
    pub reward_id: Uuid,
    pub household_id: Uuid,
    pub amount: i32,
    pub redeemed_amount: i32,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserRewardWithDetails {
    pub user_reward: UserReward,
    pub reward: Reward,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserRewardWithUser {
    pub user_reward: UserReward,
    pub user: User,
}

// ============================================================================
// Punishment Types
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Punishment {
    pub id: Uuid,
    pub household_id: Uuid,
    pub name: String,
    pub description: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreatePunishmentRequest {
    pub name: String,
    pub description: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdatePunishmentRequest {
    pub name: Option<String>,
    pub description: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserPunishment {
    pub id: Uuid,
    pub user_id: Uuid,
    pub punishment_id: Uuid,
    pub household_id: Uuid,
    pub amount: i32,
    pub completed_amount: i32,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserPunishmentWithDetails {
    pub user_punishment: UserPunishment,
    pub punishment: Punishment,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserPunishmentWithUser {
    pub user_punishment: UserPunishment,
    pub user: User,
}

// ============================================================================
// Invitation Types
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum InvitationStatus {
    Pending,
    Accepted,
    Declined,
    Expired,
}

impl InvitationStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            InvitationStatus::Pending => "pending",
            InvitationStatus::Accepted => "accepted",
            InvitationStatus::Declined => "declined",
            InvitationStatus::Expired => "expired",
        }
    }
}

impl FromStr for InvitationStatus {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "pending" => Ok(InvitationStatus::Pending),
            "accepted" => Ok(InvitationStatus::Accepted),
            "declined" => Ok(InvitationStatus::Declined),
            "expired" => Ok(InvitationStatus::Expired),
            _ => Err(()),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Invitation {
    pub id: Uuid,
    pub household_id: Uuid,
    pub email: String,
    pub role: Role,
    pub invited_by: Uuid,
    pub status: InvitationStatus,
    pub created_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
    pub responded_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InvitationWithHousehold {
    pub invitation: Invitation,
    pub household: Household,
    pub invited_by_user: User,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateInvitationRequest {
    pub email: String,
    pub role: Option<Role>,
}

// ============================================================================
// Task-Reward/Punishment Association Types
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskReward {
    pub task_id: Uuid,
    pub reward_id: Uuid,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskPunishment {
    pub task_id: Uuid,
    pub punishment_id: Uuid,
}

/// Reward linked to a task with amount (how many times to apply)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskRewardLink {
    pub reward: Reward,
    pub amount: i32,
}

/// Punishment linked to a task with amount (how many times to apply)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskPunishmentLink {
    pub punishment: Punishment,
    pub amount: i32,
}

// ============================================================================
// Extended Task Types
// ============================================================================

/// Task with linked rewards and punishments
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskWithConfig {
    pub task: Task,
    pub linked_rewards: Vec<TaskRewardLink>,
    pub linked_punishments: Vec<TaskPunishmentLink>,
}

/// Result of task completion including points and rewards assigned
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskCompletionResult {
    pub completion: TaskCompletion,
    pub points_awarded: i64,
    pub rewards_assigned: Vec<Reward>,
}

/// Report from missed task processing
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MissedTaskReport {
    pub processed_at: DateTime<Utc>,
    pub tasks_checked: i64,
    pub missed_tasks: i64,
    pub punishments_assigned: i64,
    pub points_deducted: i64,
}

// ============================================================================
// Leaderboard Types
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LeaderboardEntry {
    pub user: User,
    pub points: i64,
    pub rank: i32,
    pub tasks_completed: i64,
    pub current_streak: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PointsHistoryEntry {
    pub id: Uuid,
    pub points_change: i64,
    pub reason: String,
    pub task_id: Option<Uuid>,
    pub created_at: DateTime<Utc>,
}

// ============================================================================
// API Response Types
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiError {
    pub error: String,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiSuccess<T> {
    pub data: T,
}

impl<T> ApiSuccess<T> {
    pub fn new(data: T) -> Self {
        Self { data }
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_role_permissions() {
        assert!(Role::Owner.can_manage_members());
        assert!(Role::Owner.can_manage_tasks());
        assert!(Role::Owner.can_manage_rewards());
        assert!(Role::Owner.can_manage_roles());
        assert!(Role::Owner.can_delete_household());

        assert!(Role::Admin.can_manage_members());
        assert!(Role::Admin.can_manage_tasks());
        assert!(Role::Admin.can_manage_rewards());
        assert!(!Role::Admin.can_manage_roles());
        assert!(!Role::Admin.can_delete_household());

        assert!(!Role::Member.can_manage_members());
        assert!(!Role::Member.can_manage_tasks());
        assert!(!Role::Member.can_manage_rewards());
        assert!(!Role::Member.can_manage_roles());
        assert!(!Role::Member.can_delete_household());
    }

    #[test]
    fn test_role_from_str() {
        assert_eq!("owner".parse(), Ok(Role::Owner));
        assert_eq!("ADMIN".parse(), Ok(Role::Admin));
        assert_eq!("Member".parse(), Ok(Role::Member));
        assert!("invalid".parse::<Role>().is_err());
    }

    #[test]
    fn test_recurrence_type_from_str() {
        assert_eq!("daily".parse(), Ok(RecurrenceType::Daily));
        assert_eq!("WEEKLY".parse(), Ok(RecurrenceType::Weekly));
        assert_eq!("Monthly".parse(), Ok(RecurrenceType::Monthly));
        assert_eq!("weekdays".parse(), Ok(RecurrenceType::Weekdays));
        assert_eq!("custom".parse(), Ok(RecurrenceType::Custom));
        assert_eq!("none".parse(), Ok(RecurrenceType::OneTime));
        assert!("invalid".parse::<RecurrenceType>().is_err());
    }

    #[test]
    fn test_condition_type_from_str() {
        assert_eq!("task_complete".parse(), Ok(ConditionType::TaskComplete));
        assert_eq!("TASK_MISSED".parse(), Ok(ConditionType::TaskMissed));
        assert_eq!("streak".parse(), Ok(ConditionType::Streak));
        assert_eq!("streak_broken".parse(), Ok(ConditionType::StreakBroken));
        assert!("invalid".parse::<ConditionType>().is_err());
    }

    #[test]
    fn test_api_success() {
        let success = ApiSuccess::new("test data");
        assert_eq!(success.data, "test data");
    }

    #[test]
    fn test_invitation_status_from_str() {
        assert_eq!("pending".parse(), Ok(InvitationStatus::Pending));
        assert_eq!("ACCEPTED".parse(), Ok(InvitationStatus::Accepted));
        assert_eq!("Declined".parse(), Ok(InvitationStatus::Declined));
        assert_eq!("expired".parse(), Ok(InvitationStatus::Expired));
        assert!("invalid".parse::<InvitationStatus>().is_err());
    }

    fn create_task_with_status(completions: i32, target: i32, allow_exceed: bool) -> TaskWithStatus {
        TaskWithStatus {
            task: Task {
                id: Uuid::new_v4(),
                household_id: Uuid::new_v4(),
                title: "Test Task".to_string(),
                description: String::new(),
                recurrence_type: RecurrenceType::Daily,
                recurrence_value: None,
                assigned_user_id: None,
                target_count: target,
                time_period: None,
                allow_exceed_target: allow_exceed,
                created_at: Utc::now(),
                updated_at: Utc::now(),
            },
            completions_today: completions,
            current_streak: 0,
            last_completion: None,
            next_due_date: None,
        }
    }

    #[test]
    fn test_task_with_status_is_target_met() {
        // Not met
        let task = create_task_with_status(1, 3, true);
        assert!(!task.is_target_met());

        // Exactly met
        let task = create_task_with_status(3, 3, true);
        assert!(task.is_target_met());

        // Exceeded
        let task = create_task_with_status(5, 3, true);
        assert!(task.is_target_met());
    }

    #[test]
    fn test_task_with_status_remaining() {
        // Some remaining
        let task = create_task_with_status(1, 3, true);
        assert_eq!(task.remaining(), 2);

        // None remaining (met)
        let task = create_task_with_status(3, 3, true);
        assert_eq!(task.remaining(), 0);

        // None remaining (exceeded) - should not go negative
        let task = create_task_with_status(5, 3, true);
        assert_eq!(task.remaining(), 0);
    }

    #[test]
    fn test_task_with_status_can_complete_target_not_met() {
        // Can always complete when target not met, regardless of allow_exceed_target
        let task_allow = create_task_with_status(1, 3, true);
        assert!(task_allow.can_complete());

        let task_restrict = create_task_with_status(1, 3, false);
        assert!(task_restrict.can_complete());
    }

    #[test]
    fn test_task_with_status_can_complete_target_met_allow_exceed() {
        // Can complete when target met and allow_exceed_target is true
        let task = create_task_with_status(3, 3, true);
        assert!(task.can_complete());
    }

    #[test]
    fn test_task_with_status_can_complete_target_met_no_exceed() {
        // Cannot complete when target met and allow_exceed_target is false
        let task = create_task_with_status(3, 3, false);
        assert!(!task.can_complete());
    }

    #[test]
    fn test_task_with_status_can_complete_exceeded_allow() {
        // Can continue completing when already exceeded with allow_exceed_target true
        let task = create_task_with_status(5, 3, true);
        assert!(task.can_complete());
    }

    #[test]
    fn test_task_with_status_can_complete_exceeded_no_exceed() {
        // Cannot complete when already exceeded with allow_exceed_target false
        let task = create_task_with_status(5, 3, false);
        assert!(!task.can_complete());
    }
}
