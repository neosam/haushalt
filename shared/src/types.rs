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
    pub refresh_token: String,
    pub user: User,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RefreshTokenRequest {
    pub refresh_token: String,
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum HierarchyType {
    /// Everyone can manage tasks, rewards, and punishments
    Equals,
    /// Only Owner and Admin can manage (default, current behavior)
    #[default]
    Organized,
    /// Owner and Admin can manage, but only Members can be assigned tasks
    Hierarchy,
}

impl HierarchyType {
    pub fn as_str(&self) -> &'static str {
        match self {
            HierarchyType::Equals => "equals",
            HierarchyType::Organized => "organized",
            HierarchyType::Hierarchy => "hierarchy",
        }
    }

    /// Check if a role can manage tasks/rewards/punishments in this hierarchy
    pub fn can_manage(&self, role: &Role) -> bool {
        match self {
            HierarchyType::Equals => true, // Everyone can manage
            HierarchyType::Organized | HierarchyType::Hierarchy => {
                role.can_manage_tasks() // Owner + Admin only
            }
        }
    }

    /// Check if a role can be assigned to tasks in this hierarchy
    pub fn can_be_assigned(&self, role: &Role) -> bool {
        match self {
            HierarchyType::Equals | HierarchyType::Organized => true,
            HierarchyType::Hierarchy => matches!(role, Role::Member),
        }
    }
}

impl FromStr for HierarchyType {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "equals" => Ok(HierarchyType::Equals),
            "organized" => Ok(HierarchyType::Organized),
            "hierarchy" => Ok(HierarchyType::Hierarchy),
            _ => Err(()),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HouseholdSettings {
    pub household_id: Uuid,
    pub dark_mode: bool,
    pub role_label_owner: String,
    pub role_label_admin: String,
    pub role_label_member: String,
    pub hierarchy_type: HierarchyType,
    pub timezone: String,
    pub rewards_enabled: bool,
    pub punishments_enabled: bool,
    pub chat_enabled: bool,
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
            hierarchy_type: HierarchyType::default(),
            timezone: "UTC".to_string(),
            rewards_enabled: false,
            punishments_enabled: false,
            chat_enabled: false,
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
    pub hierarchy_type: Option<HierarchyType>,
    pub timezone: Option<String>,
    pub rewards_enabled: Option<bool>,
    pub punishments_enabled: Option<bool>,
    pub chat_enabled: Option<bool>,
}

// ============================================================================
// User Settings Types
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserSettings {
    pub user_id: Uuid,
    pub language: String,
    pub updated_at: DateTime<Utc>,
}

impl Default for UserSettings {
    fn default() -> Self {
        Self {
            user_id: Uuid::nil(),
            language: "en".to_string(),
            updated_at: Utc::now(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateUserSettingsRequest {
    pub language: Option<String>,
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
// Task Category Types
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskCategory {
    pub id: Uuid,
    pub household_id: Uuid,
    pub name: String,
    pub color: Option<String>,
    pub sort_order: i32,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateTaskCategoryRequest {
    pub name: String,
    pub color: Option<String>,
    pub sort_order: Option<i32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateTaskCategoryRequest {
    pub name: Option<String>,
    pub color: Option<String>,
    pub sort_order: Option<i32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskCategoriesResponse {
    pub categories: Vec<TaskCategory>,
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

/// Type of habit determining reward/punishment behavior
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum HabitType {
    /// Normal habit: completion = reward, missed = punishment
    #[default]
    Good,
    /// Bad habit: completion = punishment, missed = reward
    Bad,
}

impl HabitType {
    pub fn as_str(&self) -> &'static str {
        match self {
            HabitType::Good => "good",
            HabitType::Bad => "bad",
        }
    }

    /// Returns true if consequences should be inverted (Bad habit)
    pub fn is_inverted(&self) -> bool {
        matches!(self, HabitType::Bad)
    }
}

impl FromStr for HabitType {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "good" => Ok(HabitType::Good),
            "bad" => Ok(HabitType::Bad),
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
    /// When true, task completions require owner/admin approval before being finalized.
    pub requires_review: bool,
    /// Points awarded when this task is completed
    pub points_reward: Option<i64>,
    /// Points deducted when this task is missed
    pub points_penalty: Option<i64>,
    /// Due time in "HH:MM" format. None means end of day (23:59)
    pub due_time: Option<String>,
    /// Type of habit: Good (normal) or Bad (inverted consequences)
    pub habit_type: HabitType,
    /// Optional category for grouping tasks
    pub category_id: Option<Uuid>,
    /// Category name (populated when loading task with category)
    pub category_name: Option<String>,
    /// Whether the task is archived (hidden from active lists)
    pub archived: bool,
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
    /// When true, completions require owner/admin approval.
    pub requires_review: Option<bool>,
    /// Points awarded when this task is completed
    pub points_reward: Option<i64>,
    /// Points deducted when this task is missed
    pub points_penalty: Option<i64>,
    /// Due time in "HH:MM" format. None means end of day (23:59)
    pub due_time: Option<String>,
    /// Type of habit: Good (normal) or Bad (inverted consequences)
    pub habit_type: Option<HabitType>,
    /// Optional category for grouping tasks
    pub category_id: Option<Uuid>,
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
    pub requires_review: Option<bool>,
    /// Points awarded when this task is completed
    pub points_reward: Option<i64>,
    /// Points deducted when this task is missed
    pub points_penalty: Option<i64>,
    /// Due time in "HH:MM" format. None means end of day (23:59)
    pub due_time: Option<String>,
    /// Type of habit: Good (normal) or Bad (inverted consequences)
    pub habit_type: Option<HabitType>,
    /// Optional category for grouping tasks (use Some(None) to clear the category)
    pub category_id: Option<Option<Uuid>>,
    /// Whether the task is archived
    pub archived: Option<bool>,
}

/// Status of a task completion
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum CompletionStatus {
    /// Completion is approved (default for tasks without review, or after owner approval)
    #[default]
    Approved,
    /// Completion is awaiting owner/admin review
    Pending,
}

impl CompletionStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            CompletionStatus::Approved => "approved",
            CompletionStatus::Pending => "pending",
        }
    }
}

impl FromStr for CompletionStatus {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "approved" => Ok(CompletionStatus::Approved),
            "pending" => Ok(CompletionStatus::Pending),
            _ => Err(()),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskCompletion {
    pub id: Uuid,
    pub task_id: Uuid,
    pub user_id: Uuid,
    pub completed_at: DateTime<Utc>,
    pub due_date: NaiveDate,
    pub status: CompletionStatus,
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
    /// Tasks with target_count 0 are never considered "met" (they're free-form)
    pub fn is_target_met(&self) -> bool {
        self.task.target_count > 0 && self.completions_today >= self.task.target_count
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
    pub requires_confirmation: bool,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateRewardRequest {
    pub name: String,
    pub description: Option<String>,
    pub point_cost: Option<i64>,
    pub is_purchasable: bool,
    pub requires_confirmation: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateRewardRequest {
    pub name: Option<String>,
    pub description: Option<String>,
    pub point_cost: Option<i64>,
    pub is_purchasable: Option<bool>,
    pub requires_confirmation: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserReward {
    pub id: Uuid,
    pub user_id: Uuid,
    pub reward_id: Uuid,
    pub household_id: Uuid,
    pub amount: i32,
    pub redeemed_amount: i32,
    pub pending_redemption: i32,
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
    pub requires_confirmation: bool,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreatePunishmentRequest {
    pub name: String,
    pub description: Option<String>,
    pub requires_confirmation: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdatePunishmentRequest {
    pub name: Option<String>,
    pub description: Option<String>,
    pub requires_confirmation: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserPunishment {
    pub id: Uuid,
    pub user_id: Uuid,
    pub punishment_id: Uuid,
    pub household_id: Uuid,
    pub amount: i32,
    pub completed_amount: i32,
    pub pending_completion: i32,
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
// Pending Confirmation Types (for Rewards/Punishments)
// ============================================================================

/// A pending reward redemption awaiting confirmation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PendingRewardRedemption {
    pub user_reward: UserReward,
    pub reward: Reward,
    pub user: User,
}

/// A pending punishment completion awaiting confirmation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PendingPunishmentCompletion {
    pub user_punishment: UserPunishment,
    pub punishment: Punishment,
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
// Activity Log Types
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ActivityType {
    // Task events
    TaskCreated,
    TaskUpdated,
    TaskDeleted,
    TaskAssigned,
    TaskCompleted,
    TaskMissed,
    TaskCompletionApproved,
    TaskCompletionRejected,

    // Reward events
    RewardCreated,
    RewardDeleted,
    RewardAssigned,
    RewardPurchased,
    RewardRedeemed,
    RewardRedemptionApproved,
    RewardRedemptionRejected,

    // Punishment events
    PunishmentCreated,
    PunishmentDeleted,
    PunishmentAssigned,
    PunishmentCompleted,
    PunishmentCompletionApproved,
    PunishmentCompletionRejected,

    // Points events
    PointsAdjusted,

    // Membership events
    MemberJoined,
    MemberLeft,
    MemberRoleChanged,
    InvitationSent,

    // Settings events
    SettingsChanged,
}

impl ActivityType {
    pub fn as_str(&self) -> &'static str {
        match self {
            ActivityType::TaskCreated => "task_created",
            ActivityType::TaskUpdated => "task_updated",
            ActivityType::TaskDeleted => "task_deleted",
            ActivityType::TaskAssigned => "task_assigned",
            ActivityType::TaskCompleted => "task_completed",
            ActivityType::TaskMissed => "task_missed",
            ActivityType::TaskCompletionApproved => "task_completion_approved",
            ActivityType::TaskCompletionRejected => "task_completion_rejected",
            ActivityType::RewardCreated => "reward_created",
            ActivityType::RewardDeleted => "reward_deleted",
            ActivityType::RewardAssigned => "reward_assigned",
            ActivityType::RewardPurchased => "reward_purchased",
            ActivityType::RewardRedeemed => "reward_redeemed",
            ActivityType::RewardRedemptionApproved => "reward_redemption_approved",
            ActivityType::RewardRedemptionRejected => "reward_redemption_rejected",
            ActivityType::PunishmentCreated => "punishment_created",
            ActivityType::PunishmentDeleted => "punishment_deleted",
            ActivityType::PunishmentAssigned => "punishment_assigned",
            ActivityType::PunishmentCompleted => "punishment_completed",
            ActivityType::PunishmentCompletionApproved => "punishment_completion_approved",
            ActivityType::PunishmentCompletionRejected => "punishment_completion_rejected",
            ActivityType::PointsAdjusted => "points_adjusted",
            ActivityType::MemberJoined => "member_joined",
            ActivityType::MemberLeft => "member_left",
            ActivityType::MemberRoleChanged => "member_role_changed",
            ActivityType::InvitationSent => "invitation_sent",
            ActivityType::SettingsChanged => "settings_changed",
        }
    }
}

impl FromStr for ActivityType {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "task_created" => Ok(ActivityType::TaskCreated),
            "task_updated" => Ok(ActivityType::TaskUpdated),
            "task_deleted" => Ok(ActivityType::TaskDeleted),
            "task_assigned" => Ok(ActivityType::TaskAssigned),
            "task_completed" => Ok(ActivityType::TaskCompleted),
            "task_missed" => Ok(ActivityType::TaskMissed),
            "task_completion_approved" => Ok(ActivityType::TaskCompletionApproved),
            "task_completion_rejected" => Ok(ActivityType::TaskCompletionRejected),
            "reward_created" => Ok(ActivityType::RewardCreated),
            "reward_deleted" => Ok(ActivityType::RewardDeleted),
            "reward_assigned" => Ok(ActivityType::RewardAssigned),
            "reward_purchased" => Ok(ActivityType::RewardPurchased),
            "reward_redeemed" => Ok(ActivityType::RewardRedeemed),
            "reward_redemption_approved" => Ok(ActivityType::RewardRedemptionApproved),
            "reward_redemption_rejected" => Ok(ActivityType::RewardRedemptionRejected),
            "punishment_created" => Ok(ActivityType::PunishmentCreated),
            "punishment_deleted" => Ok(ActivityType::PunishmentDeleted),
            "punishment_assigned" => Ok(ActivityType::PunishmentAssigned),
            "punishment_completed" => Ok(ActivityType::PunishmentCompleted),
            "punishment_completion_approved" => Ok(ActivityType::PunishmentCompletionApproved),
            "punishment_completion_rejected" => Ok(ActivityType::PunishmentCompletionRejected),
            "points_adjusted" => Ok(ActivityType::PointsAdjusted),
            "member_joined" => Ok(ActivityType::MemberJoined),
            "member_left" => Ok(ActivityType::MemberLeft),
            "member_role_changed" => Ok(ActivityType::MemberRoleChanged),
            "invitation_sent" => Ok(ActivityType::InvitationSent),
            "settings_changed" => Ok(ActivityType::SettingsChanged),
            _ => Err(()),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActivityLog {
    pub id: Uuid,
    pub household_id: Uuid,
    pub actor_id: Uuid,
    pub affected_user_id: Option<Uuid>,
    pub activity_type: ActivityType,
    pub entity_type: Option<String>,
    pub entity_id: Option<Uuid>,
    pub details: Option<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActivityLogWithUsers {
    pub log: ActivityLog,
    pub actor: User,
    pub affected_user: Option<User>,
}

// ============================================================================
// Pending Review Types
// ============================================================================

/// A pending task completion awaiting review, with task and user details
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PendingReview {
    pub completion: TaskCompletion,
    pub task: Task,
    pub user: User,
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
// Chat Message Types
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub id: Uuid,
    pub household_id: Uuid,
    pub user_id: Uuid,
    pub content: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub is_deleted: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessageWithUser {
    pub message: ChatMessage,
    pub user: User,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateChatMessageRequest {
    pub content: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateChatMessageRequest {
    pub content: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListChatMessagesRequest {
    pub limit: Option<i64>,
    pub before: Option<Uuid>,
}

// ============================================================================
// WebSocket Message Types
// ============================================================================

/// Messages sent from client to server via WebSocket
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "payload")]
pub enum WsClientMessage {
    /// Authenticate with JWT token
    Authenticate { token: String },
    /// Join a household chat room
    JoinRoom { household_id: Uuid },
    /// Leave the current chat room
    LeaveRoom,
    /// Send a new chat message
    SendMessage { content: String },
    /// Edit an existing message
    EditMessage { message_id: Uuid, content: String },
    /// Delete a message
    DeleteMessage { message_id: Uuid },
    /// Ping to keep connection alive
    Ping,
}

/// Messages sent from server to client via WebSocket
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "payload")]
pub enum WsServerMessage {
    /// Authentication successful
    Authenticated { user_id: Uuid, username: String },
    /// Error occurred
    Error { code: String, message: String },
    /// Successfully joined a chat room
    JoinedRoom { household_id: Uuid },
    /// Successfully left the chat room
    LeftRoom,
    /// New message received
    NewMessage { message: ChatMessageWithUser },
    /// Message was edited
    MessageEdited { message: ChatMessageWithUser },
    /// Message was deleted
    MessageDeleted { message_id: Uuid, household_id: Uuid },
    /// Pong response to ping
    Pong,
}

// ============================================================================
// Note Types
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Note {
    pub id: Uuid,
    pub household_id: Uuid,
    pub user_id: Uuid,
    pub title: String,
    pub content: String,
    pub is_shared: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NoteWithUser {
    pub note: Note,
    pub user: User,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateNoteRequest {
    pub title: String,
    pub content: Option<String>,
    pub is_shared: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateNoteRequest {
    pub title: Option<String>,
    pub content: Option<String>,
    pub is_shared: Option<bool>,
}

// ============================================================================
// Announcement Types
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Announcement {
    pub id: Uuid,
    pub household_id: Uuid,
    pub created_by: Uuid,
    pub title: String,
    pub content: String,
    pub starts_at: Option<DateTime<Utc>>,
    pub ends_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateAnnouncementRequest {
    pub title: String,
    pub content: Option<String>,
    pub starts_at: Option<DateTime<Utc>>,
    pub ends_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateAnnouncementRequest {
    pub title: Option<String>,
    pub content: Option<String>,
    /// None = no change, Some(None) = clear, Some(Some(dt)) = set
    pub starts_at: Option<Option<DateTime<Utc>>>,
    /// None = no change, Some(None) = clear, Some(Some(dt)) = set
    pub ends_at: Option<Option<DateTime<Utc>>>,
}

// ============================================================================
// Dashboard Tasks
// ============================================================================

/// Response containing the list of task IDs that should appear on the user's dashboard
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DashboardTasksResponse {
    pub task_ids: Vec<Uuid>,
}

/// Check if a specific task is on the user's dashboard
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IsTaskOnDashboardResponse {
    pub on_dashboard: bool,
}

/// A dashboard task with its household information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DashboardTaskWithHousehold {
    pub task_with_status: TaskWithStatus,
    pub household_id: Uuid,
    pub household_name: String,
}

/// Response containing dashboard tasks with their full status information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DashboardTasksWithStatusResponse {
    pub tasks: Vec<DashboardTaskWithHousehold>,
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
    fn test_hierarchy_type_from_str() {
        assert_eq!("equals".parse(), Ok(HierarchyType::Equals));
        assert_eq!("ORGANIZED".parse(), Ok(HierarchyType::Organized));
        assert_eq!("Hierarchy".parse(), Ok(HierarchyType::Hierarchy));
        assert!("invalid".parse::<HierarchyType>().is_err());
    }

    #[test]
    fn test_hierarchy_type_can_manage() {
        // Equals: everyone can manage
        assert!(HierarchyType::Equals.can_manage(&Role::Owner));
        assert!(HierarchyType::Equals.can_manage(&Role::Admin));
        assert!(HierarchyType::Equals.can_manage(&Role::Member));

        // Organized: only Owner and Admin can manage
        assert!(HierarchyType::Organized.can_manage(&Role::Owner));
        assert!(HierarchyType::Organized.can_manage(&Role::Admin));
        assert!(!HierarchyType::Organized.can_manage(&Role::Member));

        // Hierarchy: only Owner and Admin can manage
        assert!(HierarchyType::Hierarchy.can_manage(&Role::Owner));
        assert!(HierarchyType::Hierarchy.can_manage(&Role::Admin));
        assert!(!HierarchyType::Hierarchy.can_manage(&Role::Member));
    }

    #[test]
    fn test_hierarchy_type_can_be_assigned() {
        // Equals: everyone can be assigned
        assert!(HierarchyType::Equals.can_be_assigned(&Role::Owner));
        assert!(HierarchyType::Equals.can_be_assigned(&Role::Admin));
        assert!(HierarchyType::Equals.can_be_assigned(&Role::Member));

        // Organized: everyone can be assigned
        assert!(HierarchyType::Organized.can_be_assigned(&Role::Owner));
        assert!(HierarchyType::Organized.can_be_assigned(&Role::Admin));
        assert!(HierarchyType::Organized.can_be_assigned(&Role::Member));

        // Hierarchy: only Members can be assigned
        assert!(!HierarchyType::Hierarchy.can_be_assigned(&Role::Owner));
        assert!(!HierarchyType::Hierarchy.can_be_assigned(&Role::Admin));
        assert!(HierarchyType::Hierarchy.can_be_assigned(&Role::Member));
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
                requires_review: false,
                points_reward: None,
                points_penalty: None,
                due_time: None,
                habit_type: HabitType::Good,
                category_id: None,
                category_name: None,
                archived: false,
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

    #[test]
    fn test_activity_type_from_str() {
        assert_eq!("task_created".parse(), Ok(ActivityType::TaskCreated));
        assert_eq!("TASK_COMPLETED".parse(), Ok(ActivityType::TaskCompleted));
        assert_eq!("reward_assigned".parse(), Ok(ActivityType::RewardAssigned));
        assert_eq!("points_adjusted".parse(), Ok(ActivityType::PointsAdjusted));
        assert_eq!("member_joined".parse(), Ok(ActivityType::MemberJoined));
        assert_eq!("settings_changed".parse(), Ok(ActivityType::SettingsChanged));
        assert!("invalid".parse::<ActivityType>().is_err());
    }

    #[test]
    fn test_activity_type_as_str() {
        assert_eq!(ActivityType::TaskCreated.as_str(), "task_created");
        assert_eq!(ActivityType::RewardPurchased.as_str(), "reward_purchased");
        assert_eq!(ActivityType::PunishmentAssigned.as_str(), "punishment_assigned");
        assert_eq!(ActivityType::MemberRoleChanged.as_str(), "member_role_changed");
    }

    #[test]
    fn test_habit_type_from_str() {
        assert_eq!("good".parse(), Ok(HabitType::Good));
        assert_eq!("GOOD".parse(), Ok(HabitType::Good));
        assert_eq!("bad".parse(), Ok(HabitType::Bad));
        assert_eq!("BAD".parse(), Ok(HabitType::Bad));
        assert!("invalid".parse::<HabitType>().is_err());
    }

    #[test]
    fn test_habit_type_as_str() {
        assert_eq!(HabitType::Good.as_str(), "good");
        assert_eq!(HabitType::Bad.as_str(), "bad");
    }

    #[test]
    fn test_habit_type_is_inverted() {
        assert!(!HabitType::Good.is_inverted());
        assert!(HabitType::Bad.is_inverted());
    }

    #[test]
    fn test_habit_type_default() {
        assert_eq!(HabitType::default(), HabitType::Good);
    }
}
