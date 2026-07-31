// Test utilities for service layer testing
// Provides database setup, fixture creation, and assertion helpers

use chrono::{NaiveDate, Utc};
use sqlx::{SqlitePool, Sqlite, Pool};
use uuid::Uuid;

use shared::{
    CompletionStatus, HabitType, PeriodStatus, RecurrenceType,
    RecurrenceValue, Role, SuggestionStatus, Task, TimePeriod,
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

    // Household settings table (22-column production shape — must stay column-for-column
    // identical to HouseholdSettingsRow so `SELECT * FROM household_settings` + FromRow works)
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS household_settings (
            household_id TEXT PRIMARY KEY NOT NULL REFERENCES households(id),
            dark_mode BOOLEAN NOT NULL DEFAULT 0,
            role_label_owner TEXT NOT NULL DEFAULT 'Owner',
            role_label_admin TEXT NOT NULL DEFAULT 'Admin',
            role_label_member TEXT NOT NULL DEFAULT 'Member',
            hierarchy_type TEXT NOT NULL DEFAULT 'organized',
            timezone TEXT NOT NULL DEFAULT 'UTC',
            rewards_enabled BOOLEAN NOT NULL DEFAULT 0,
            punishments_enabled BOOLEAN NOT NULL DEFAULT 0,
            chat_enabled BOOLEAN NOT NULL DEFAULT 0,
            vacation_mode BOOLEAN NOT NULL DEFAULT 0,
            vacation_start DATE,
            vacation_end DATE,
            auto_archive_days INTEGER DEFAULT 7,
            allow_task_suggestions BOOLEAN NOT NULL DEFAULT 1,
            week_start_day INTEGER NOT NULL DEFAULT 0,
            default_points_reward INTEGER,
            default_points_penalty INTEGER,
            solo_mode BOOLEAN NOT NULL DEFAULT 0,
            solo_mode_exit_requested_at DATETIME,
            solo_mode_previous_hierarchy_type TEXT,
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
            anyone_can_complete BOOLEAN NOT NULL DEFAULT FALSE,
            assignee_cannot_uncomplete BOOLEAN NOT NULL DEFAULT FALSE,
            requires_review BOOLEAN NOT NULL DEFAULT FALSE,
            points_reward INTEGER,
            points_penalty INTEGER,
            due_time TEXT,
            habit_type TEXT NOT NULL DEFAULT 'good' CHECK(habit_type IN ('good', 'bad')),
            category_id TEXT REFERENCES task_categories(id),
            archived BOOLEAN NOT NULL DEFAULT FALSE,
            paused BOOLEAN NOT NULL DEFAULT FALSE,
            suggestion TEXT CHECK(suggestion IN ('suggested', 'approved', 'denied')),
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

    // Missed task penalties table — tracks which tasks have been processed for missed
    // penalties on which dates, preventing duplicate penalties across background job runs
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS missed_task_penalties (
            task_id TEXT NOT NULL,
            due_date DATE NOT NULL,
            processed_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
            PRIMARY KEY (task_id, due_date),
            FOREIGN KEY (task_id) REFERENCES tasks(id) ON DELETE CASCADE
        )
        "#,
    )
    .execute(pool)
    .await
    .unwrap();

    sqlx::query("CREATE INDEX IF NOT EXISTS idx_missed_task_penalties_date ON missed_task_penalties(due_date)")
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
            reward_type TEXT NOT NULL DEFAULT 'standard',
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
            punishment_type TEXT NOT NULL DEFAULT 'standard',
            created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
        )
        "#,
    )
    .execute(pool)
    .await
    .unwrap();

    // Household default rewards (junction table for multiple default rewards per household)
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS household_default_rewards (
            household_id TEXT NOT NULL REFERENCES households(id) ON DELETE CASCADE,
            reward_id TEXT NOT NULL REFERENCES rewards(id) ON DELETE CASCADE,
            amount INTEGER NOT NULL DEFAULT 1,
            PRIMARY KEY (household_id, reward_id)
        )
        "#,
    )
    .execute(pool)
    .await
    .unwrap();

    // Household default punishments (junction table for multiple default punishments per household)
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS household_default_punishments (
            household_id TEXT NOT NULL REFERENCES households(id) ON DELETE CASCADE,
            punishment_id TEXT NOT NULL REFERENCES punishments(id) ON DELETE CASCADE,
            amount INTEGER NOT NULL DEFAULT 1,
            PRIMARY KEY (household_id, punishment_id)
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

    // Public reports (Phase 6) — must stay in sync with
    // migrations/20240150000000_public_reports.sql, which this schema mirrors.
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS public_reports (
            id TEXT PRIMARY KEY NOT NULL,
            user_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
            name TEXT NOT NULL,
            token TEXT NOT NULL UNIQUE,
            language TEXT NOT NULL DEFAULT 'en',
            enabled INTEGER NOT NULL DEFAULT 1,
            created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
            updated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
        )
        "#,
    )
    .execute(pool)
    .await
    .unwrap();

    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS public_report_households (
            report_id TEXT NOT NULL REFERENCES public_reports(id) ON DELETE CASCADE,
            household_id TEXT NOT NULL REFERENCES households(id) ON DELETE CASCADE,
            PRIMARY KEY (report_id, household_id)
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
///
/// The owner's email is generated per call, so any number of households can live in the
/// same pool. It used to be a fixed `owner@test.com`, which made a second household fail
/// on the UNIQUE constraint and forced callers to hand-roll their own fixture.
pub async fn create_test_household_with_name(pool: &SqlitePool, name: &str) -> Uuid {
    let owner_email = format!("owner-{}@test.com", Uuid::new_v4());
    create_test_household_with_owner(pool, name, &owner_email).await
}

/// Create a test household whose owner has a specific email — for tests that need to
/// address the owner afterwards.
pub async fn create_test_household_with_owner(
    pool: &SqlitePool,
    name: &str,
    owner_email: &str,
) -> Uuid {
    let id = Uuid::new_v4();
    let owner_id = create_test_user(pool, owner_email, Role::Owner).await;

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

    // Create default household settings (no created_at column in production schema)
    sqlx::query(
        r#"INSERT INTO household_settings (household_id, timezone, hierarchy_type, vacation_mode, auto_archive_days, updated_at)
        VALUES (?, 'UTC', 'democratic', FALSE, 30, ?)"#,
    )
    .bind(id.to_string())
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
    anyone_can_complete: bool,
    assignee_cannot_uncomplete: bool,
    requires_review: bool,
    points_reward: Option<i64>,
    points_penalty: Option<i64>,
    due_time: Option<String>,
    habit_type: HabitType,
    category_id: Option<Uuid>,
    archived: bool,
    paused: bool,
    suggestion: Option<SuggestionStatus>,
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

    pub fn with_anyone_can_complete(mut self, anyone: bool) -> Self {
        self.anyone_can_complete = anyone;
        self
    }

    pub fn with_assignee_cannot_uncomplete(mut self, restricted: bool) -> Self {
        self.assignee_cannot_uncomplete = restricted;
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

    pub fn with_suggestion(mut self, suggestion: SuggestionStatus) -> Self {
        self.suggestion = Some(suggestion);
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
                anyone_can_complete, assignee_cannot_uncomplete, requires_review,
                points_reward, points_penalty,
                due_time, habit_type, category_id, archived, paused, suggestion,
                created_at, updated_at
            )
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
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
        .bind(self.anyone_can_complete)
        .bind(self.assignee_cannot_uncomplete)
        .bind(self.requires_review)
        .bind(self.points_reward)
        .bind(self.points_penalty)
        .bind(&self.due_time)
        .bind(self.habit_type.as_str())
        .bind(self.category_id.map(|c| c.to_string()))
        .bind(self.archived)
        .bind(self.paused)
        .bind(self.suggestion.map(|s| s.as_str()))
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
            anyone_can_complete: self.anyone_can_complete,
            assignee_cannot_uncomplete: self.assignee_cannot_uncomplete,
            requires_review: self.requires_review,
            points_reward: self.points_reward,
            points_penalty: self.points_penalty,
            due_time: self.due_time,
            habit_type: self.habit_type,
            category_id: self.category_id,
            category_name: None,
            category_color: None,
            archived: self.archived,
            paused: self.paused,
            suggestion: self.suggestion,
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
        anyone_can_complete: false,
        assignee_cannot_uncomplete: false,
        requires_review: false,
        points_reward: None,
        points_penalty: None,
        due_time: None,
        habit_type: HabitType::Good,
        category_id: None,
        archived: false,
        paused: false,
        suggestion: None,
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

/// Set household vacation start/end dates (independent of the vacation_mode toggle)
pub async fn set_vacation_dates(
    pool: &SqlitePool,
    household_id: &Uuid,
    start: Option<NaiveDate>,
    end: Option<NaiveDate>,
) {
    sqlx::query(
        "UPDATE household_settings SET vacation_start = ?, vacation_end = ? WHERE household_id = ?",
    )
    .bind(start)
    .bind(end)
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

/// Insert a missed-task penalty row, as `process_missed_tasks` would
pub async fn insert_missed_task_penalty(pool: &SqlitePool, task_id: &Uuid, due_date: NaiveDate) {
    sqlx::query("INSERT INTO missed_task_penalties (task_id, due_date) VALUES (?, ?)")
        .bind(task_id.to_string())
        .bind(due_date)
        .execute(pool)
        .await
        .unwrap();
}

// ============================================================================
// Harness Smoke Tests (Phase 2.1 Wave 0)
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::services::household_settings;

    #[tokio::test]
    async fn test_get_or_create_settings_works_against_test_schema() {
        let pool = create_test_pool().await;
        let household_id = create_test_household(&pool).await;

        let settings = household_settings::get_or_create_settings(&pool, &household_id)
            .await
            .unwrap();

        assert_eq!(settings.timezone, "UTC");
        assert!(!settings.vacation_mode);
    }

    #[tokio::test]
    async fn test_get_or_create_settings_creates_row_when_absent() {
        let pool = create_test_pool().await;
        // Create a household WITHOUT going through create_test_household_with_name, so no
        // household_settings row exists yet — exercises the INSERT branch.
        let owner_id = create_test_user(&pool, "no-settings-owner@test.com", Role::Owner).await;
        let household_id = Uuid::new_v4();
        let now = Utc::now();
        sqlx::query(
            "INSERT INTO households (id, name, owner_id, created_at, updated_at) VALUES (?, ?, ?, ?, ?)",
        )
        .bind(household_id.to_string())
        .bind("No Settings Household")
        .bind(owner_id.to_string())
        .bind(now)
        .bind(now)
        .execute(&pool)
        .await
        .unwrap();

        let result = household_settings::get_or_create_settings(&pool, &household_id).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_insert_missed_task_penalty() {
        let pool = create_test_pool().await;
        let household_id = create_test_household(&pool).await;
        let task = create_test_task(&pool, &household_id).build().await;
        let due_date = NaiveDate::from_ymd_opt(2026, 7, 25).unwrap();

        insert_missed_task_penalty(&pool, &task.id, due_date).await;

        let count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM missed_task_penalties WHERE task_id = ? AND due_date = ?",
        )
        .bind(task.id.to_string())
        .bind(due_date)
        .fetch_one(&pool)
        .await
        .unwrap();

        assert_eq!(count, 1);
    }

    #[tokio::test]
    async fn test_task_with_approved_suggestion_insertable() {
        let pool = create_test_pool().await;
        let household_id = create_test_household(&pool).await;

        let task = create_test_task(&pool, &household_id)
            .with_suggestion(SuggestionStatus::Approved)
            .build()
            .await;

        assert_eq!(task.suggestion, Some(SuggestionStatus::Approved));

        let suggestion: Option<String> =
            sqlx::query_scalar("SELECT suggestion FROM tasks WHERE id = ?")
                .bind(task.id.to_string())
                .fetch_one(&pool)
                .await
                .unwrap();
        assert_eq!(suggestion.as_deref(), Some("approved"));
    }

    #[tokio::test]
    async fn test_task_anyone_can_complete_insertable_and_defaults_off() {
        let pool = create_test_pool().await;
        let household_id = create_test_household(&pool).await;

        // Default is off, mirroring the production schema default
        let default_task = create_test_task(&pool, &household_id).build().await;
        assert!(!default_task.anyone_can_complete);

        let task = create_test_task(&pool, &household_id)
            .with_anyone_can_complete(true)
            .build()
            .await;

        assert!(task.anyone_can_complete);

        let stored: bool = sqlx::query_scalar("SELECT anyone_can_complete FROM tasks WHERE id = ?")
            .bind(task.id.to_string())
            .fetch_one(&pool)
            .await
            .unwrap();
        assert!(stored);
    }

    #[tokio::test]
    async fn test_task_assignee_cannot_uncomplete_insertable_and_defaults_off() {
        let pool = create_test_pool().await;
        let household_id = create_test_household(&pool).await;

        // Default is off, mirroring the production schema default
        let default_task = create_test_task(&pool, &household_id).build().await;
        assert!(!default_task.assignee_cannot_uncomplete);

        let task = create_test_task(&pool, &household_id)
            .with_assignee_cannot_uncomplete(true)
            .build()
            .await;

        assert!(task.assignee_cannot_uncomplete);

        let stored: bool =
            sqlx::query_scalar("SELECT assignee_cannot_uncomplete FROM tasks WHERE id = ?")
                .bind(task.id.to_string())
                .fetch_one(&pool)
                .await
                .unwrap();
        assert!(stored);
    }

    #[tokio::test]
    async fn test_set_vacation_mode_and_dates() {
        let pool = create_test_pool().await;
        let household_id = create_test_household(&pool).await;

        set_vacation_mode(&pool, &household_id, true).await;
        let start = NaiveDate::from_ymd_opt(2026, 7, 1).unwrap();
        let end = NaiveDate::from_ymd_opt(2026, 7, 31).unwrap();
        set_vacation_dates(&pool, &household_id, Some(start), Some(end)).await;

        let settings = household_settings::get_or_create_settings(&pool, &household_id)
            .await
            .unwrap();

        let inside_range = NaiveDate::from_ymd_opt(2026, 7, 15).unwrap();
        let outside_range = NaiveDate::from_ymd_opt(2026, 8, 15).unwrap();

        assert!(household_settings::is_household_on_vacation(
            &settings,
            inside_range
        ));
        assert!(!household_settings::is_household_on_vacation(
            &settings,
            outside_range
        ));
    }
}
