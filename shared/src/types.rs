use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
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

    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "owner" => Some(Role::Owner),
            "admin" => Some(Role::Admin),
            "member" => Some(Role::Member),
            _ => None,
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
}

impl RecurrenceType {
    pub fn as_str(&self) -> &'static str {
        match self {
            RecurrenceType::Daily => "daily",
            RecurrenceType::Weekly => "weekly",
            RecurrenceType::Monthly => "monthly",
            RecurrenceType::Weekdays => "weekdays",
            RecurrenceType::Custom => "custom",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "daily" => Some(RecurrenceType::Daily),
            "weekly" => Some(RecurrenceType::Weekly),
            "monthly" => Some(RecurrenceType::Monthly),
            "weekdays" => Some(RecurrenceType::Weekdays),
            "custom" => Some(RecurrenceType::Custom),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Task {
    pub id: Uuid,
    pub household_id: Uuid,
    pub title: String,
    pub description: String,
    pub recurrence_type: RecurrenceType,
    pub recurrence_value: Option<RecurrenceValue>,
    pub assigned_user_id: Option<Uuid>,
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
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateTaskRequest {
    pub title: Option<String>,
    pub description: Option<String>,
    pub recurrence_type: Option<RecurrenceType>,
    pub recurrence_value: Option<RecurrenceValue>,
    pub assigned_user_id: Option<Uuid>,
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
    pub is_completed_today: bool,
    pub current_streak: i32,
    pub last_completion: Option<DateTime<Utc>>,
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

    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "task_complete" => Some(ConditionType::TaskComplete),
            "task_missed" => Some(ConditionType::TaskMissed),
            "streak" => Some(ConditionType::Streak),
            "streak_broken" => Some(ConditionType::StreakBroken),
            _ => None,
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
    pub assigned_by: Option<Uuid>,
    pub is_purchased: bool,
    pub redeemed: bool,
    pub assigned_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserRewardWithDetails {
    pub user_reward: UserReward,
    pub reward: Reward,
    pub assigned_by_user: Option<User>,
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
    pub assigned_by: Uuid,
    pub task_completion_id: Option<Uuid>,
    pub completed: bool,
    pub assigned_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserPunishmentWithDetails {
    pub user_punishment: UserPunishment,
    pub punishment: Punishment,
    pub assigned_by_user: User,
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
        assert_eq!(Role::from_str("owner"), Some(Role::Owner));
        assert_eq!(Role::from_str("ADMIN"), Some(Role::Admin));
        assert_eq!(Role::from_str("Member"), Some(Role::Member));
        assert_eq!(Role::from_str("invalid"), None);
    }

    #[test]
    fn test_recurrence_type_from_str() {
        assert_eq!(RecurrenceType::from_str("daily"), Some(RecurrenceType::Daily));
        assert_eq!(RecurrenceType::from_str("WEEKLY"), Some(RecurrenceType::Weekly));
        assert_eq!(RecurrenceType::from_str("Monthly"), Some(RecurrenceType::Monthly));
        assert_eq!(RecurrenceType::from_str("weekdays"), Some(RecurrenceType::Weekdays));
        assert_eq!(RecurrenceType::from_str("custom"), Some(RecurrenceType::Custom));
        assert_eq!(RecurrenceType::from_str("invalid"), None);
    }

    #[test]
    fn test_condition_type_from_str() {
        assert_eq!(ConditionType::from_str("task_complete"), Some(ConditionType::TaskComplete));
        assert_eq!(ConditionType::from_str("TASK_MISSED"), Some(ConditionType::TaskMissed));
        assert_eq!(ConditionType::from_str("streak"), Some(ConditionType::Streak));
        assert_eq!(ConditionType::from_str("streak_broken"), Some(ConditionType::StreakBroken));
        assert_eq!(ConditionType::from_str("invalid"), None);
    }

    #[test]
    fn test_api_success() {
        let success = ApiSuccess::new("test data");
        assert_eq!(success.data, "test data");
    }
}
