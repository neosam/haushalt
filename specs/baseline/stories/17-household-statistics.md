# Household Statistics

## Overview

Weekly and monthly statistics for household members, calculated from task period results. Statistics summarize each member's task completion performance over time.

---

## US-STAT-001: Configure Week Start Day

**As a** household Owner
**I want to** configure which day of the week the household's week starts
**So that** statistics align with the household's cultural norms

### Acceptance Criteria
- New household setting: `week_start_day` (integer 0-6, where 0=Monday, 6=Sunday)
- Default value: 0 (Monday)
- Only Owner can modify this setting
- Setting is visible in household settings page

---

## US-STAT-002: Calculate Weekly Statistics

**As a** system
**I want to** calculate weekly statistics for each household member
**So that** members can review their weekly performance

### Acceptance Criteria
- Statistics are calculated for the previous week on the configured week start day
- A week is defined as 7 days ending on the day before the week start day
- Statistics are stored per member, per week
- Calculation includes:
  - Total tasks assigned to the member (based on `assigned_user_id` or hierarchy rules)
  - Total expected completions (sum of period results where member was responsible)
  - Total actual completions (sum of `completed` period results)
  - Per-task breakdown: task name, expected count, actual count, completion rate
  - Overall completion rate: (total completions / total expected) * 100
- Only includes tasks where the member is "on hierarchy" (assignable based on household hierarchy type)
- Archived tasks are excluded from new calculations but historical data is preserved

### Data Structure
```
WeeklyStatistics {
    id: UUID
    household_id: UUID
    user_id: UUID
    week_start: Date          // First day of the week
    week_end: Date            // Last day of the week
    total_expected: i32       // Total expected completions
    total_completed: i32      // Total actual completions
    completion_rate: f32      // Percentage (0.0 - 100.0)
    task_stats: Vec<TaskStat> // Per-task breakdown
    calculated_at: DateTime
}

TaskStat {
    task_id: UUID
    task_title: String
    expected: i32
    completed: i32
    completion_rate: f32
}
```

---

## US-STAT-003: Calculate Monthly Statistics

**As a** system
**I want to** calculate monthly statistics for each household member
**So that** members can review their monthly performance

### Acceptance Criteria
- Statistics are calculated for the previous month on the 1st of each month
- A month is defined as the calendar month (1st to last day)
- Statistics are stored per member, per month
- Calculation follows same structure as weekly statistics
- Only includes tasks where the member is "on hierarchy"

### Data Structure
```
MonthlyStatistics {
    id: UUID
    household_id: UUID
    user_id: UUID
    month: Date               // First day of the month (YYYY-MM-01)
    total_expected: i32
    total_completed: i32
    completion_rate: f32
    task_stats: Vec<TaskStat>
    calculated_at: DateTime
}
```

---

## US-STAT-004: Background Job for Statistics Calculation

**As a** system
**I want to** automatically calculate statistics on schedule
**So that** statistics are always up to date

### Acceptance Criteria
- Background job runs daily (similar to period finalization job)
- Checks each household's timezone and week_start_day
- On week start day: calculate previous week's statistics for all members
- On 1st of month: calculate previous month's statistics for all members
- Idempotent: re-running does not create duplicate records
- Statistics are only calculated once (check if already exists before calculating)

---

## US-STAT-005: View Weekly Statistics Page

**As a** household member
**I want to** view weekly statistics for the household
**So that** I can see how everyone performed

### Acceptance Criteria
- New "Statistics" tab in household navigation
- Default view shows current/most recent week
- Displays statistics for all members (filtered by hierarchy visibility rules)
- For each member shows:
  - Username
  - Overall completion rate (with visual indicator, e.g., progress bar)
  - Total completed / total expected
- Expandable detail view per member shows per-task breakdown
- Members can only see their own detailed task breakdown unless they are Owner/Admin

---

## US-STAT-006: View Monthly Statistics Page

**As a** household member
**I want to** view monthly statistics for the household
**So that** I can see long-term performance trends

### Acceptance Criteria
- Toggle between weekly and monthly view on statistics page
- Monthly view follows same structure as weekly view
- Shows month name and year in header

---

## US-STAT-007: Browse Historical Statistics

**As a** household member
**I want to** browse through past weeks and months
**So that** I can review historical performance

### Acceptance Criteria
- Navigation controls to go to previous/next week or month
- Date picker or dropdown to jump to specific period
- Shows "No statistics available" for periods before tracking started
- Cannot navigate to future periods

---

## US-STAT-008: API Endpoints for Statistics

**As a** developer
**I want to** access statistics via API
**So that** the frontend can display them

### Acceptance Criteria

#### GET /api/households/{id}/statistics/weekly
- Query params: `week_start` (date, optional - defaults to most recent)
- Returns weekly statistics for all visible members
- Respects hierarchy visibility rules

#### GET /api/households/{id}/statistics/monthly
- Query params: `month` (YYYY-MM, optional - defaults to most recent)
- Returns monthly statistics for all visible members

#### GET /api/households/{id}/statistics/weekly/{user_id}
- Returns detailed weekly statistics for specific user
- Only accessible by the user themselves or Owner/Admin

#### GET /api/households/{id}/statistics/monthly/{user_id}
- Returns detailed monthly statistics for specific user
- Only accessible by the user themselves or Owner/Admin

---

## US-STAT-009: Statistics Calculation Logic

**As a** system
**I want to** correctly calculate statistics from period results
**So that** statistics accurately reflect member performance

### Acceptance Criteria

#### Determining Task Responsibility
For each task period, determine if a member was responsible:
1. If task has `assigned_user_id`: only that user is responsible
2. If task has no assignment (based on hierarchy type):
   - **Equals**: All members share responsibility (divide by member count)
   - **Organized**: All members share responsibility
   - **Hierarchy**: Only members with role "Member" are responsible (divide by Member count)

#### Counting Expected vs Completed
- **Expected**: Count of period results where member was responsible
- **Completed**: Count of period results with status = 'completed' where member was responsible
- **Skipped** periods are excluded from both counts (don't affect the rate)
- **Failed** periods count as expected but not completed

#### Handling Shared Tasks
When a task is shared (no specific assignee):
- Each eligible member gets `1 / eligible_count` credit for expected
- If any member completes, all eligible members get `1 / eligible_count` credit for completed
- This prevents shared tasks from unfairly inflating/deflating individual rates

---

## US-STAT-010: Statistics Data Retention

**As a** system administrator
**I want to** define data retention policy for statistics
**So that** storage is managed appropriately

### Acceptance Criteria
- Weekly statistics are retained for 2 years
- Monthly statistics are retained indefinitely
- Old weekly statistics can be purged via admin command
- Household deletion cascades to delete all statistics

---

## Database Schema

```sql
-- Household setting addition
ALTER TABLE household_settings ADD COLUMN week_start_day INTEGER NOT NULL DEFAULT 0;

-- Weekly statistics
CREATE TABLE weekly_statistics (
    id TEXT PRIMARY KEY NOT NULL,
    household_id TEXT NOT NULL REFERENCES households(id) ON DELETE CASCADE,
    user_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    week_start DATE NOT NULL,
    week_end DATE NOT NULL,
    total_expected INTEGER NOT NULL,
    total_completed INTEGER NOT NULL,
    completion_rate REAL NOT NULL,
    calculated_at DATETIME NOT NULL,

    UNIQUE(household_id, user_id, week_start)
);

CREATE INDEX idx_weekly_stats_household ON weekly_statistics(household_id);
CREATE INDEX idx_weekly_stats_user ON weekly_statistics(user_id);
CREATE INDEX idx_weekly_stats_week ON weekly_statistics(week_start);

-- Weekly statistics task breakdown
CREATE TABLE weekly_statistics_tasks (
    id TEXT PRIMARY KEY NOT NULL,
    weekly_statistics_id TEXT NOT NULL REFERENCES weekly_statistics(id) ON DELETE CASCADE,
    task_id TEXT NOT NULL,
    task_title TEXT NOT NULL,
    expected INTEGER NOT NULL,
    completed INTEGER NOT NULL,
    completion_rate REAL NOT NULL
);

CREATE INDEX idx_weekly_stats_tasks_parent ON weekly_statistics_tasks(weekly_statistics_id);

-- Monthly statistics
CREATE TABLE monthly_statistics (
    id TEXT PRIMARY KEY NOT NULL,
    household_id TEXT NOT NULL REFERENCES households(id) ON DELETE CASCADE,
    user_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    month DATE NOT NULL,  -- First day of month
    total_expected INTEGER NOT NULL,
    total_completed INTEGER NOT NULL,
    completion_rate REAL NOT NULL,
    calculated_at DATETIME NOT NULL,

    UNIQUE(household_id, user_id, month)
);

CREATE INDEX idx_monthly_stats_household ON monthly_statistics(household_id);
CREATE INDEX idx_monthly_stats_user ON monthly_statistics(user_id);
CREATE INDEX idx_monthly_stats_month ON monthly_statistics(month);

-- Monthly statistics task breakdown
CREATE TABLE monthly_statistics_tasks (
    id TEXT PRIMARY KEY NOT NULL,
    monthly_statistics_id TEXT NOT NULL REFERENCES monthly_statistics(id) ON DELETE CASCADE,
    task_id TEXT NOT NULL,
    task_title TEXT NOT NULL,
    expected INTEGER NOT NULL,
    completed INTEGER NOT NULL,
    completion_rate REAL NOT NULL
);

CREATE INDEX idx_monthly_stats_tasks_parent ON monthly_statistics_tasks(monthly_statistics_id);
```

---

## Implementation Notes

### Phase 1: Backend Foundation
1. Add `week_start_day` to household settings
2. Create database migrations for statistics tables
3. Implement statistics calculation service
4. Add background job for automatic calculation

### Phase 2: API Layer
1. Implement statistics API endpoints
2. Add authorization checks based on hierarchy

### Phase 3: Frontend
1. Add Statistics tab to household navigation
2. Implement weekly statistics view
3. Implement monthly statistics view
4. Add navigation controls for browsing history
