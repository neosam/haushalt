// Test utilities for service layer testing
// Provides database setup, fixture creation, and assertion helpers

use chrono::{NaiveDate, Utc};
use sqlx::{SqlitePool, Sqlite, Pool};
use uuid::Uuid;

use shared::{
    CompletionStatus, HabitType, PeriodStatus, RecurrenceType,
    RecurrenceValue, Role, Task, TimePeriod,
};

// ============================================================================
// Database Setup (Tasks 1.2 - 1.3)
// ============================================================================

/// Create an in-memory SQLite database pool for testing
/// Runs all migrations automatically
pub async fn create_test_pool() -> Pool<Sqlite> {
    let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();
    run_migrations(&pool).await;
    pool
}

/// Run all database migrations on a test database
pub async fn run_migrations(pool: &SqlitePool) {
    // Note: Using sqlx::migrate!() here would require the migrations to be in the right path
    // For tests, we'll create the tables manually based on the actual schema
    create_test_schema(pool).await;
}

async fn create_test_schema(pool: &SqlitePool) {
    // Users table
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS users (
            id TEXT PRIMARY KEY NOT NULL,
            username TEXT NOT NULL UNIQUE,
            email TEXT NOT NULL UNIQUE,
            password_hash TEXT,
            oidc_subject TEXT,
            oidc_provider TEXT,
            created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
            updated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
        )
        "#,
    )
    .execute(pool)
    .await
    .unwrap();

    sqlx::query("CREATE INDEX IF NOT EXISTS idx_users_username ON users(username)")
        .execute(pool)
        .await
        .unwrap();

    // Households table
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS households (
            id TEXT PRIMARY KEY NOT NULL,
            name TEXT NOT NULL,
            owner_id TEXT NOT NULL REFERENCES users(id),
            created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
            updated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
        )
        "#,
    )
    .execute(pool)
    .await
    .unwrap();

    // Household settings table
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS household_settings (
            household_id TEXT PRIMARY KEY NOT NULL REFERENCES households(id),
            timezone TEXT NOT NULL DEFAULT 'UTC',
            hierarchy_type TEXT NOT NULL DEFAULT 'democratic',
            vacation_mode BOOLEAN NOT NULL DEFAULT FALSE,
            auto_archive_days INTEGER NOT NULL DEFAULT 30,
            created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
            updated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
        )
        "#,
    )
    .execute(pool)
    .await
    .unwrap();

    // User settings table
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS user_settings (
            user_id TEXT PRIMARY KEY NOT NULL REFERENCES users(id),
            language TEXT NOT NULL DEFAULT 'en',
            theme TEXT NOT NULL DEFAULT 'light',
            notifications_enabled BOOLEAN NOT NULL DEFAULT TRUE,
            created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
            updated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
        )
        "#,
    )
    .execute(pool)
    .await
    .unwrap();

    // Household memberships table
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS household_memberships (
            id TEXT PRIMARY KEY NOT NULL,
            household_id TEXT NOT NULL REFERENCES households(id),
            user_id TEXT NOT NULL REFERENCES users(id),
            role TEXT NOT NULL DEFAULT 'member' CHECK(role IN ('owner', 'admin', 'member')),
            points INTEGER NOT NULL DEFAULT 0,
            joined_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
            UNIQUE(household_id, user_id)
        )
        "#,
    )
    .execute(pool)
    .await
    .unwrap();

    sqlx::query("CREATE INDEX IF NOT EXISTS idx_memberships_household ON household_memberships(household_id)")
        .execute(pool)
        .await
        .unwrap();

    // Task categories table
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS task_categories (
            id TEXT PRIMARY KEY NOT NULL,
            household_id TEXT NOT NULL REFERENCES households(id),
            name TEXT NOT NULL,
            color TEXT NOT NULL DEFAULT '#3B82F6',
            icon TEXT,
            created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
            updated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
            UNIQUE(household_id, name)
        )
        "#,
    )
    .execute(pool)
    .await
    .unwrap();

    // Tasks table
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS tasks (
            id TEXT PRIMARY KEY NOT NULL,
            household_id TEXT NOT NULL REFERENCES households(id),
            title TEXT NOT NULL,
            description TEXT NOT NULL DEFAULT '',
            recurrence_type TEXT NOT NULL DEFAULT 'daily' CHECK(recurrence_type IN ('daily', 'weekly', 'monthly', 'weekdays', 'custom', 'onetime')),
            recurrence_value TEXT,
            assigned_user_id TEXT REFERENCES users(id),
            target_count INTEGER NOT NULL DEFAULT 1,
            time_period TEXT CHECK(time_period IN ('day', 'week', 'month', 'year', 'none')),
            allow_exceed_target BOOLEAN NOT NULL DEFAULT TRUE,
            requires_review BOOLEAN NOT NULL DEFAULT FALSE,
            points_reward INTEGER,
            points_penalty INTEGER,
            due_time TEXT,
            habit_type TEXT NOT NULL DEFAULT 'good' CHECK(habit_type IN ('good', 'bad')),
            category_id TEXT REFERENCES task_categories(id),
            archived BOOLEAN NOT NULL DEFAULT FALSE,
            paused BOOLEAN NOT NULL DEFAULT FALSE,
            suggestion TEXT CHECK(suggestion IN ('suggested', 'accepted', 'rejected')),
            suggested_by TEXT REFERENCES users(id),
            created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
            updated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
        )
        "#,
    )
    .execute(pool)
    .await
    .unwrap();

    sqlx::query("CREATE INDEX IF NOT EXISTS idx_tasks_household ON tasks(household_id)")
        .execute(pool)
        .await
        .unwrap();

    // Task completions table
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS task_completions (
            id TEXT PRIMARY KEY NOT NULL,
            task_id TEXT NOT NULL REFERENCES tasks(id),
            user_id TEXT NOT NULL REFERENCES users(id),
            completed_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
            due_date DATE NOT NULL,
            status TEXT NOT NULL DEFAULT 'approved' CHECK(status IN ('pending', 'approved', 'rejected'))
        )
        "#,
    )
    .execute(pool)
    .await
    .unwrap();

    sqlx::query("CREATE INDEX IF NOT EXISTS idx_completions_task ON task_completions(task_id)")
        .execute(pool)
        .await
        .unwrap();

    // Task period results table
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS task_period_results (
            id TEXT PRIMARY KEY NOT NULL,
            task_id TEXT NOT NULL REFERENCES tasks(id),
            period_start DATE NOT NULL,
            period_end DATE NOT NULL,
            status TEXT NOT NULL CHECK(status IN ('completed', 'failed', 'skipped')),
            completions_count INTEGER NOT NULL,
            target_count INTEGER NOT NULL,
            finalized_at DATETIME NOT NULL,
            finalized_by TEXT NOT NULL DEFAULT 'system',
            notes TEXT
        )
        "#,
    )
    .execute(pool)
    .await
    .unwrap();

    sqlx::query("CREATE UNIQUE INDEX IF NOT EXISTS idx_period_results_task_date ON task_period_results(task_id, period_start)")
        .execute(pool)
        .await
        .unwrap();

    // Point conditions table
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS point_conditions (
            id TEXT PRIMARY KEY NOT NULL,
            household_id TEXT NOT NULL REFERENCES households(id),
            name TEXT NOT NULL,
            description TEXT,
            points INTEGER NOT NULL,
            created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
        )
        "#,
    )
    .execute(pool)
    .await
    .unwrap();

    // Task consequences table
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS task_consequences (
            id TEXT PRIMARY KEY,
            task_id TEXT NOT NULL,
            consequence_type TEXT NOT NULL,
            trigger_type TEXT NOT NULL,
            consequence_id TEXT NOT NULL,
            created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
        )
        "#,
    )
    .execute(pool)
    .await
    .unwrap();

    // Rewards table
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS rewards (
            id TEXT PRIMARY KEY NOT NULL,
            household_id TEXT NOT NULL REFERENCES households(id),
            name TEXT NOT NULL,
            description TEXT NOT NULL DEFAULT '',
            point_cost INTEGER,
            is_purchasable BOOLEAN NOT NULL DEFAULT FALSE,
            requires_confirmation BOOLEAN NOT NULL DEFAULT FALSE,
            created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
        )
        "#,
    )
    .execute(pool)
    .await
    .unwrap();

    // Punishments table
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS punishments (
            id TEXT PRIMARY KEY NOT NULL,
            household_id TEXT NOT NULL REFERENCES households(id),
            name TEXT NOT NULL,
            description TEXT NOT NULL DEFAULT '',
            requires_confirmation BOOLEAN NOT NULL DEFAULT FALSE,
            created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
        )
        "#,
    )
    .execute(pool)
    .await
    .unwrap();

    // Task-reward associations
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS task_rewards (
            task_id TEXT NOT NULL REFERENCES tasks(id),
            reward_id TEXT NOT NULL REFERENCES rewards(id),
            amount INTEGER NOT NULL DEFAULT 1,
            PRIMARY KEY (task_id, reward_id)
        )
        "#,
    )
    .execute(pool)
    .await
    .unwrap();

    // Task-punishment associations
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS task_punishments (
            task_id TEXT NOT NULL REFERENCES tasks(id),
            punishment_id TEXT NOT NULL REFERENCES punishments(id),
            amount INTEGER NOT NULL DEFAULT 1,
            PRIMARY KEY (task_id, punishment_id)
        )
        "#,
    )
    .execute(pool)
    .await
    .unwrap();

    // User rewards table
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS user_rewards (
            id TEXT PRIMARY KEY NOT NULL,
            user_id TEXT NOT NULL REFERENCES users(id),
            reward_id TEXT NOT NULL REFERENCES rewards(id),
            household_id TEXT NOT NULL REFERENCES households(id),
            assigned_by TEXT REFERENCES users(id),
            is_purchased BOOLEAN NOT NULL DEFAULT FALSE,
            redeemed BOOLEAN NOT NULL DEFAULT FALSE,
            assigned_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
        )
        "#,
    )
    .execute(pool)
    .await
    .unwrap();

    // User punishments table
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS user_punishments (
            id TEXT PRIMARY KEY NOT NULL,
            user_id TEXT NOT NULL REFERENCES users(id),
            punishment_id TEXT NOT NULL REFERENCES punishments(id),
            household_id TEXT NOT NULL REFERENCES households(id),
            assigned_by TEXT NOT NULL REFERENCES users(id),
            task_completion_id TEXT REFERENCES task_completions(id),
            completed BOOLEAN NOT NULL DEFAULT FALSE,
            assigned_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
        )
        "#,
    )
    .execute(pool)
    .await
    .unwrap();

    // Activity logs table
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS activity_logs (
            id TEXT PRIMARY KEY NOT NULL,
            household_id TEXT NOT NULL REFERENCES households(id),
            user_id TEXT REFERENCES users(id),
            activity_type TEXT NOT NULL,
            entity_type TEXT,
            entity_id TEXT,
            metadata TEXT,
            created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
        )
        "#,
    )
    .execute(pool)
    .await
    .unwrap();
}

// ============================================================================
// Fixture Creation (Tasks 1.4 - 1.7)
// ============================================================================

/// Create a test household with default settings
pub async fn create_test_household(pool: &SqlitePool) -> Uuid {
    create_test_household_with_name(pool, "Test Household").await
}

/// Create a test household with a specific name
pub async fn create_test_household_with_name(pool: &SqlitePool, name: &str) -> Uuid {
    let id = Uuid::new_v4();
    let owner_id = create_test_user(pool, "owner@test.com", Role::Owner).await;

    let now = Utc::now();
    sqlx::query(
        "INSERT INTO households (id, name, owner_id, created_at, updated_at) VALUES (?, ?, ?, ?, ?)",
    )
    .bind(id.to_string())
    .bind(name)
    .bind(owner_id.to_string())
    .bind(now)
    .bind(now)
    .execute(pool)
    .await
    .unwrap();

    // Create default household settings
    sqlx::query(
        r#"INSERT INTO household_settings (household_id, timezone, hierarchy_type, vacation_mode, auto_archive_days, created_at, updated_at)
        VALUES (?, 'UTC', 'democratic', FALSE, 30, ?, ?)"#,
    )
    .bind(id.to_string())
    .bind(now)
    .bind(now)
    .execute(pool)
    .await
    .unwrap();

    id
}

/// Create a test user with a specific role
pub async fn create_test_user(pool: &SqlitePool, email: &str, _role: Role) -> Uuid {
    let id = Uuid::new_v4();
    let username = email.split('@').next().unwrap();
    let now = Utc::now();

    sqlx::query(
        "INSERT INTO users (id, username, email, password_hash, created_at, updated_at) VALUES (?, ?, ?, ?, ?, ?)",
    )
    .bind(id.to_string())
    .bind(username)
    .bind(email)
    .bind("test_password_hash")
    .bind(now)
    .bind(now)
    .execute(pool)
    .await
    .unwrap();

    id
}

/// Create a test membership linking user to household
pub async fn create_test_membership(
    pool: &SqlitePool,
    household_id: &Uuid,
    user_id: &Uuid,
    role: Role,
) -> Uuid {
    let id = Uuid::new_v4();
    let now = Utc::now();

    sqlx::query(
        "INSERT INTO household_memberships (id, household_id, user_id, role, points, joined_at) VALUES (?, ?, ?, ?, 0, ?)",
    )
    .bind(id.to_string())
    .bind(household_id.to_string())
    .bind(user_id.to_string())
    .bind(role.as_str())
    .bind(now)
    .execute(pool)
    .await
    .unwrap();

    id
}

/// Builder for creating test tasks with fluent API
pub struct TestTaskBuilder {
    pool: SqlitePool,
    household_id: Uuid,
    title: String,
    description: Option<String>,
    recurrence_type: RecurrenceType,
    recurrence_value: Option<RecurrenceValue>,
    assigned_user_id: Option<Uuid>,
    target_count: i32,
    time_period: Option<TimePeriod>,
    allow_exceed_target: bool,
    requires_review: bool,
    points_reward: Option<i64>,
    points_penalty: Option<i64>,
    due_time: Option<String>,
    habit_type: HabitType,
    category_id: Option<Uuid>,
    archived: bool,
    paused: bool,
}

impl TestTaskBuilder {
    pub fn with_title(mut self, title: &str) -> Self {
        self.title = title.to_string();
        self
    }

    pub fn with_description(mut self, description: &str) -> Self {
        self.description = Some(description.to_string());
        self
    }

    pub fn with_recurrence(mut self, recurrence_type: RecurrenceType) -> Self {
        self.recurrence_type = recurrence_type;
        self
    }

    pub fn with_recurrence_value(mut self, recurrence_value: RecurrenceValue) -> Self {
        self.recurrence_value = Some(recurrence_value);
        self
    }

    pub fn with_assigned_user(mut self, user_id: Uuid) -> Self {
        self.assigned_user_id = Some(user_id);
        self
    }

    pub fn with_target_count(mut self, count: i32) -> Self {
        self.target_count = count;
        self
    }

    pub fn with_time_period(mut self, period: TimePeriod) -> Self {
        self.time_period = Some(period);
        self
    }

    pub fn with_allow_exceed_target(mut self, allow: bool) -> Self {
        self.allow_exceed_target = allow;
        self
    }

    pub fn with_requires_review(mut self, requires: bool) -> Self {
        self.requires_review = requires;
        self
    }

    pub fn with_points(mut self, reward: i64, penalty: i64) -> Self {
        self.points_reward = Some(reward);
        self.points_penalty = Some(penalty);
        self
    }

    pub fn with_points_reward(mut self, reward: i64) -> Self {
        self.points_reward = Some(reward);
        self
    }

    pub fn with_points_penalty(mut self, penalty: i64) -> Self {
        self.points_penalty = Some(penalty);
        self
    }

    pub fn with_due_time(mut self, time: &str) -> Self {
        self.due_time = Some(time.to_string());
        self
    }

    pub fn with_habit_type(mut self, habit_type: HabitType) -> Self {
        self.habit_type = habit_type;
        self
    }

    pub fn with_category(mut self, category_id: Uuid) -> Self {
        self.category_id = Some(category_id);
        self
    }

    pub fn with_archived(mut self, archived: bool) -> Self {
        self.archived = archived;
        self
    }

    pub fn with_paused(mut self, paused: bool) -> Self {
        self.paused = paused;
        self
    }

    pub async fn build(self) -> Task {
        let id = Uuid::new_v4();
        let now = Utc::now();

        let recurrence_value_json = self
            .recurrence_value
            .as_ref()
            .map(|v| serde_json::to_string(v).unwrap());

        let time_period_str = self.time_period.as_ref().map(|p| p.as_str());

        sqlx::query(
            r#"
            INSERT INTO tasks (
                id, household_id, title, description, recurrence_type, recurrence_value,
                assigned_user_id, target_count, time_period, allow_exceed_target,
                requires_review, points_reward, points_penalty, due_time, habit_type,
                category_id, archived, paused, created_at, updated_at
            )
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(id.to_string())
        .bind(self.household_id.to_string())
        .bind(&self.title)
        .bind(self.description.as_deref().unwrap_or(""))
        .bind(self.recurrence_type.as_str())
        .bind(&recurrence_value_json)
        .bind(self.assigned_user_id.map(|u| u.to_string()))
        .bind(self.target_count)
        .bind(time_period_str)
        .bind(self.allow_exceed_target)
        .bind(self.requires_review)
        .bind(self.points_reward)
        .bind(self.points_penalty)
        .bind(&self.due_time)
        .bind(self.habit_type.as_str())
        .bind(self.category_id.map(|c| c.to_string()))
        .bind(self.archived)
        .bind(self.paused)
        .bind(now)
        .bind(now)
        .execute(&self.pool)
        .await
        .unwrap();

        Task {
            id,
            household_id: self.household_id,
            title: self.title,
            description: self.description.unwrap_or_default(),
            recurrence_type: self.recurrence_type,
            recurrence_value: self.recurrence_value,
            assigned_user_id: self.assigned_user_id,
            target_count: self.target_count,
            time_period: self.time_period,
            allow_exceed_target: self.allow_exceed_target,
            requires_review: self.requires_review,
            points_reward: self.points_reward,
            points_penalty: self.points_penalty,
            due_time: self.due_time,
            habit_type: self.habit_type,
            category_id: self.category_id,
            category_name: None,
            archived: self.archived,
            paused: self.paused,
            suggestion: None,
            suggested_by: None,
            created_at: now,
            updated_at: now,
        }
    }
}

/// Create a test task builder with fluent API
pub fn create_test_task(pool: &SqlitePool, household_id: &Uuid) -> TestTaskBuilder {
    TestTaskBuilder {
        pool: pool.clone(),
        household_id: *household_id,
        title: "Test Task".to_string(),
        description: None,
        recurrence_type: RecurrenceType::Daily,
        recurrence_value: None,
        assigned_user_id: None,
        target_count: 1,
        time_period: None,
        allow_exceed_target: true,
        requires_review: false,
        points_reward: None,
        points_penalty: None,
        due_time: None,
        habit_type: HabitType::Good,
        category_id: None,
        archived: false,
        paused: false,
    }
}

// ============================================================================
// Assertion Helpers (Tasks 2.1 - 2.6)
// ============================================================================

/// Assert that a task completion exists with the given status
pub async fn assert_completion_exists(
    pool: &SqlitePool,
    task_id: &Uuid,
    user_id: &Uuid,
    status: CompletionStatus,
) {
    let count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM task_completions WHERE task_id = ? AND user_id = ? AND status = ?",
    )
    .bind(task_id.to_string())
    .bind(user_id.to_string())
    .bind(status.as_str())
    .fetch_one(pool)
    .await
    .unwrap();

    assert!(
        count > 0,
        "Expected completion for task {} by user {} with status {:?}, but found none",
        task_id,
        user_id,
        status
    );
}

/// Assert that a task completion does NOT exist
pub async fn assert_completion_not_exists(pool: &SqlitePool, task_id: &Uuid, user_id: &Uuid) {
    let count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM task_completions WHERE task_id = ? AND user_id = ?",
    )
    .bind(task_id.to_string())
    .bind(user_id.to_string())
    .fetch_one(pool)
    .await
    .unwrap();

    assert_eq!(
        count, 0,
        "Expected no completion for task {} by user {}, but found {}",
        task_id, user_id, count
    );
}

/// Assert that a period result exists with the given status
pub async fn assert_period_result(
    pool: &SqlitePool,
    task_id: &Uuid,
    period_start: NaiveDate,
    status: PeriodStatus,
) {
    let result_status: Option<String> = sqlx::query_scalar(
        "SELECT status FROM task_period_results WHERE task_id = ? AND period_start = ?",
    )
    .bind(task_id.to_string())
    .bind(period_start)
    .fetch_optional(pool)
    .await
    .unwrap();

    assert!(
        result_status.is_some(),
        "Expected period result for task {} on {}, but found none",
        task_id,
        period_start
    );

    let result_status = result_status.unwrap();
    let actual_status: PeriodStatus = result_status.parse().unwrap();

    assert_eq!(
        actual_status, status,
        "Expected period status {:?} for task {} on {}, but found {:?}",
        status, task_id, period_start, actual_status
    );
}

/// Assert streak values for a task
pub async fn assert_streak(pool: &SqlitePool, task_id: &Uuid, current: i32, best: i32) {
    use crate::services::period_results::{calculate_best_streak, calculate_current_streak};

    let current_streak = calculate_current_streak(pool, task_id).await.unwrap();
    let best_streak = calculate_best_streak(pool, task_id).await.unwrap();

    assert_eq!(
        current_streak, current,
        "Expected current streak {} for task {}, but found {}",
        current, task_id, current_streak
    );

    assert_eq!(
        best_streak, best,
        "Expected best streak {} for task {}, but found {}",
        best, task_id, best_streak
    );
}

/// Assert points balance for a user in a household
pub async fn assert_points_balance(
    pool: &SqlitePool,
    household_id: &Uuid,
    user_id: &Uuid,
    expected_points: i32,
) {
    let points: i32 = sqlx::query_scalar(
        "SELECT points FROM household_memberships WHERE household_id = ? AND user_id = ?",
    )
    .bind(household_id.to_string())
    .bind(user_id.to_string())
    .fetch_one(pool)
    .await
    .unwrap();

    assert_eq!(
        points, expected_points,
        "Expected {} points for user {} in household {}, but found {}",
        expected_points, user_id, household_id, points
    );
}

/// Assert that an activity log entry exists
pub async fn assert_activity_logged(
    pool: &SqlitePool,
    household_id: &Uuid,
    activity_type: &str,
) {
    let count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM activity_logs WHERE household_id = ? AND activity_type = ?",
    )
    .bind(household_id.to_string())
    .bind(activity_type)
    .fetch_one(pool)
    .await
    .unwrap();

    assert!(
        count > 0,
        "Expected activity log of type '{}' for household {}, but found none",
        activity_type, household_id
    );
}

// ============================================================================
// Additional Helper Functions
// ============================================================================

/// Get the current points balance for a user
pub async fn get_user_points(
    pool: &SqlitePool,
    household_id: &Uuid,
    user_id: &Uuid,
) -> i32 {
    sqlx::query_scalar(
        "SELECT points FROM household_memberships WHERE household_id = ? AND user_id = ?",
    )
    .bind(household_id.to_string())
    .bind(user_id.to_string())
    .fetch_one(pool)
    .await
    .unwrap()
}

/// Update user points directly (for test setup)
pub async fn set_user_points(
    pool: &SqlitePool,
    household_id: &Uuid,
    user_id: &Uuid,
    points: i32,
) {
    sqlx::query(
        "UPDATE household_memberships SET points = ? WHERE household_id = ? AND user_id = ?",
    )
    .bind(points)
    .bind(household_id.to_string())
    .bind(user_id.to_string())
    .execute(pool)
    .await
    .unwrap();
}

/// Set household timezone for testing timezone-dependent behavior
pub async fn set_household_timezone(
    pool: &SqlitePool,
    household_id: &Uuid,
    timezone: &str,
) {
    sqlx::query("UPDATE household_settings SET timezone = ? WHERE household_id = ?")
        .bind(timezone)
        .bind(household_id.to_string())
        .execute(pool)
        .await
        .unwrap();
}

/// Set household vacation mode
pub async fn set_vacation_mode(
    pool: &SqlitePool,
    household_id: &Uuid,
    enabled: bool,
) {
    sqlx::query("UPDATE household_settings SET vacation_mode = ? WHERE household_id = ?")
        .bind(enabled)
        .bind(household_id.to_string())
        .execute(pool)
        .await
        .unwrap();
}

/// Create a test reward
pub async fn create_test_reward(
    pool: &SqlitePool,
    household_id: &Uuid,
    name: &str,
    point_cost: Option<i32>,
) -> Uuid {
    let id = Uuid::new_v4();
    let now = Utc::now();

    sqlx::query(
        "INSERT INTO rewards (id, household_id, name, description, point_cost, is_purchasable, requires_confirmation, created_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(id.to_string())
    .bind(household_id.to_string())
    .bind(name)
    .bind("")
    .bind(point_cost)
    .bind(point_cost.is_some())
    .bind(false)
    .bind(now)
    .execute(pool)
    .await
    .unwrap();

    id
}

/// Create a test punishment
pub async fn create_test_punishment(
    pool: &SqlitePool,
    household_id: &Uuid,
    name: &str,
) -> Uuid {
    let id = Uuid::new_v4();
    let now = Utc::now();

    sqlx::query(
        "INSERT INTO punishments (id, household_id, name, description, requires_confirmation, created_at) VALUES (?, ?, ?, ?, ?, ?)",
    )
    .bind(id.to_string())
    .bind(household_id.to_string())
    .bind(name)
    .bind("")
    .bind(false)
    .bind(now)
    .execute(pool)
    .await
    .unwrap();

    id
}

/// Link a reward to a task
pub async fn link_task_reward(
    pool: &SqlitePool,
    task_id: &Uuid,
    reward_id: &Uuid,
    amount: i32,
) {
    sqlx::query(
        "INSERT INTO task_rewards (task_id, reward_id, amount) VALUES (?, ?, ?)",
    )
    .bind(task_id.to_string())
    .bind(reward_id.to_string())
    .bind(amount)
    .execute(pool)
    .await
    .unwrap();
}

/// Link a punishment to a task
pub async fn link_task_punishment(
    pool: &SqlitePool,
    task_id: &Uuid,
    punishment_id: &Uuid,
    amount: i32,
) {
    sqlx::query(
        "INSERT INTO task_punishments (task_id, punishment_id, amount) VALUES (?, ?, ?)",
    )
    .bind(task_id.to_string())
    .bind(punishment_id.to_string())
    .bind(amount)
    .execute(pool)
    .await
    .unwrap();
}
