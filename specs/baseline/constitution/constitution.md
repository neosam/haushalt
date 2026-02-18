# Household Manager - Constitution

## 1. Purpose & Vision

**Household Manager** is a full-stack Rust application for collaborative task and habit tracking for families or shared living situations.

### Core Goals
- Delegation and tracking of household tasks and habits
- Gamification through points, rewards, and punishments
- Role-based access control for flexible hierarchies
- Real-time communication and activity tracking
- Multi-household support

### Target Users
- Families organizing chores
- Shared living situations (roommates, co-ops)
- Small teams managing collaborative tasks

---

## 2. Architecture

### 2.1 Workspace Structure

```
/backend/     - Actix-web Server (REST API + WebSocket)
/frontend/    - Leptos CSR WASM application
/shared/      - Shared API types
```

### 2.2 Layered Architecture (Backend)

```
Handlers (HTTP Endpoints)
    ↓
Services (Business Logic)
    ↓
Database Layer (SQLx + SQLite)
```

### 2.3 Technology Stack

| Component | Technology |
|-----------|------------|
| Backend | Rust, Actix-web |
| Frontend | Rust, Leptos (CSR), WASM |
| Database | SQLite, SQLx |
| Auth | JWT (HS256), Argon2 |
| Real-time | WebSocket |
| Build | Nix, Trunk |

---

## 3. Domain Model

### 3.1 Core Entities

#### User
- Global identity with username, email, password (Argon2)
- Language preference (en, de)
- Can be member of multiple households

#### Household
- Organization unit (family, shared apartment)
- Has one owner
- Contains members with roles
- Customizable settings

#### HouseholdMembership
- Links User to Household
- Stores role and points
- Unique: One user per household

### 3.2 Role Hierarchy

| Role | Manage Members | Manage Tasks | Manage Rewards | Change Roles | Delete Household |
|------|---------------|--------------|----------------|--------------|------------------|
| Owner | ✓ | ✓ | ✓ | ✓ | ✓ |
| Admin | ✓ | ✓ | ✓ | ✗ | ✗ |
| Member | ✗ | ✗ | ✗ | ✗ | ✗ |

### 3.3 Hierarchy Types

| Type | Who Can Manage | Who Can Be Assigned |
|------|----------------|---------------------|
| Equals | Everyone | Everyone |
| Organized (Default) | Owner/Admin | Everyone |
| Hierarchy | Owner/Admin | Members only |

---

## 4. Task System

### 4.1 Task Properties

| Property | Description |
|----------|-------------|
| `title`, `description` | Name and details |
| `recurrence_type` | Daily, Weekly, Monthly, Weekdays, Custom, OneTime |
| `recurrence_value` | JSON-encoded recurrence details |
| `target_count` | Number of completions per period |
| `time_period` | Day, Week, Month, Year |
| `allow_exceed_target` | Allow more than target? |
| `requires_review` | Approval required? |
| `points_reward` | Points on completion |
| `points_penalty` | Point deduction on miss |
| `habit_type` | Good (normal) or Bad (inverted) |
| `assigned_user_id` | Optional: Assigned user |
| `category_id` | Optional: Category |
| `due_time` | Optional: Due time (HH:MM) |
| `archived` | Is task archived? (hidden from active lists) |

### 4.2 Recurrence Types

| Type | Description |
|------|-------------|
| Daily | Every day |
| Weekly | Weekly on specific day |
| Monthly | Monthly on specific day |
| Weekdays | Mon-Fri (customizable) |
| Custom | Specific date list |
| OneTime | Once, always due |

### 4.3 Completion Rules

1. **One completion per day per user per task**
2. **Target enforcement**:
   - `target_count = 0`: Unlimited
   - `target_count > 0` + `allow_exceed_target = false`: Stop at target
   - `target_count > 0` + `allow_exceed_target = true`: Unlimited
3. **Review workflow**: With `requires_review = true` status is "Pending" until approved
4. **Assignment**: Assigned tasks only completable by assigned user

### 4.4 Habit Types

| Type | On Completion | On Miss |
|------|---------------|---------|
| Good | Reward/Points | Penalty/Deduction |
| Bad | Penalty/Deduction | Reward/Points |

---

## 5. Points & Rewards System

### 5.1 Point Sources

- Automatic on task completion (`points_reward`)
- Automatic on task miss (`points_penalty`)
- Streak bonuses via PointConditions
- Manual adjustment by Admin

### 5.2 PointCondition Types

| Type | Trigger |
|------|---------|
| TaskComplete | Task completed |
| TaskMissed | Task missed |
| Streak | Streak threshold reached |
| StreakBroken | Streak broken |

### 5.3 Rewards

- Name, description, point cost
- `is_purchasable`: Purchasable with points?
- `requires_confirmation`: Approval required?
- Linkable to tasks (automatic assignment)

### 5.4 Punishments

- Name, description
- `requires_confirmation`: Approval required?
- `punishment_type`: Type of punishment (extensible enum)
- Linkable to tasks (automatic assignment on miss)
- Not purchasable (admin assignment only)

#### Punishment Types

| Type | Description |
|------|-------------|
| Standard | Default type - describes what the punishment is |
| RandomChoice | Container for other punishments, user picks randomly |

#### Random Choice Punishments

- `punishment_type = random_choice`: Punishment is a container for other punishments
- Links to multiple other punishments as options (minimum 2)
- Self-reference is allowed (punishment can include itself as an option)
- When assigned, user sees "Pick one" button
- Clicking randomly selects one option and assigns it to the user
- Options can include other random choice punishments (nesting allowed)
- If nested random choice is selected, user picks again
- Success notification shows which punishment was selected

---

## 6. Authentication & Authorization

### 6.1 Auth Flow

1. Registration with username + password (Argon2 hash)
2. Login returns access token (short-lived) + refresh token (long-lived)
3. Frontend stores in LocalStorage
4. On 401: Automatic token refresh
5. Refresh token rotation (single use)

### 6.2 JWT Structure

```json
{
  "sub": "user_id",
  "exp": 1234567890,
  "iat": 1234567800
}
```

### 6.3 Authorization Checks

1. **Membership**: User must be household member
2. **Role check**: Action requires specific role
3. **Hierarchy check**: HierarchyType allows action for role

---

## 7. Database Schema

### 7.1 Core Tables

#### users
```sql
id TEXT PRIMARY KEY,
username TEXT UNIQUE,
email TEXT UNIQUE,
password_hash TEXT,
created_at DATETIME,
updated_at DATETIME
```

#### households
```sql
id TEXT PRIMARY KEY,
name TEXT,
owner_id TEXT REFERENCES users(id),
created_at DATETIME,
updated_at DATETIME
```

#### household_memberships
```sql
id TEXT PRIMARY KEY,
household_id TEXT REFERENCES households(id),
user_id TEXT REFERENCES users(id),
role TEXT CHECK(role IN ('owner', 'admin', 'member')),
points INTEGER DEFAULT 0,
joined_at DATETIME,
UNIQUE(household_id, user_id)
```

#### tasks
```sql
id TEXT PRIMARY KEY,
household_id TEXT REFERENCES households(id),
title TEXT,
description TEXT,
recurrence_type TEXT,
recurrence_value TEXT,  -- JSON
assigned_user_id TEXT REFERENCES users(id),
target_count INTEGER,
time_period TEXT,
allow_exceed_target BOOLEAN,
requires_review BOOLEAN,
points_reward INTEGER,
points_penalty INTEGER,
due_time TEXT,
habit_type TEXT,
category_id TEXT REFERENCES task_categories(id),
archived BOOLEAN DEFAULT 0,
created_at DATETIME,
updated_at DATETIME
```

#### task_completions
```sql
id TEXT PRIMARY KEY,
task_id TEXT REFERENCES tasks(id),
user_id TEXT REFERENCES users(id),
completed_at DATETIME,
due_date DATE,
status TEXT DEFAULT 'approved',
UNIQUE(task_id, user_id, due_date)
```

### 7.2 Additional Tables

- `task_categories`: Task categories
- `point_conditions`: Point rules
- `rewards`, `user_rewards`: Rewards
- `punishments`, `user_punishments`: Punishments
- `punishment_options`: Random choice punishment options
- `task_rewards`, `task_punishments`: Linkages
- `invitations`: Invitations
- `chat_messages`: Chat messages
- `notes`: Notes
- `announcements`: Announcements
- `activity_logs`: Activity log
- `household_settings`: Household settings
- `user_settings`: User settings
- `refresh_tokens`: Refresh token storage
- `user_dashboard_tasks`: Dashboard whitelist

---

## 8. API Structure

### 8.1 Authentication

| Method | Path | Purpose |
|--------|------|---------|
| POST | `/auth/register` | Registration |
| POST | `/auth/login` | Login |
| POST | `/auth/refresh` | Token refresh |
| POST | `/auth/logout` | Logout |

### 8.2 Households

| Method | Path | Purpose |
|--------|------|---------|
| GET | `/households` | List own households |
| POST | `/households` | Create household |
| GET | `/households/{id}` | Household details |
| PUT | `/households/{id}` | Update household |
| DELETE | `/households/{id}` | Delete household (Owner) |
| GET | `/households/{id}/members` | Members |
| GET | `/households/{id}/settings` | Settings |
| GET | `/households/{id}/leaderboard` | Leaderboard |

### 8.3 Tasks

| Method | Path | Purpose |
|--------|------|---------|
| GET | `/households/{id}/tasks` | Task list |
| POST | `/households/{id}/tasks` | Create task |
| GET | `/tasks/{id}` | Task details |
| PUT | `/tasks/{id}` | Update task |
| DELETE | `/tasks/{id}` | Delete task |
| POST | `/tasks/{id}/complete` | Complete |
| POST | `/tasks/{id}/uncomplete` | Undo |
| GET | `/households/{id}/tasks/pending-reviews` | Pending reviews |
| POST | `/tasks/{id}/archive` | Archive task |
| POST | `/tasks/{id}/unarchive` | Unarchive task |
| GET | `/households/{id}/tasks/archived` | Archived tasks |

### 8.4 Rewards & Punishments

| Method | Path | Purpose |
|--------|------|---------|
| GET | `/households/{id}/rewards` | Rewards |
| POST | `/households/{id}/rewards` | Create |
| GET | `/households/{id}/punishments` | Punishments |
| POST | `/households/{id}/punishments` | Create |
| POST | `/rewards/{id}/redeem` | Redeem |
| POST | `/punishments/{id}/complete` | Complete |
| GET | `/punishments/{id}/options` | Get punishment options |
| POST | `/punishments/{id}/options/{option_id}` | Add punishment option |
| DELETE | `/punishments/{id}/options/{option_id}` | Remove punishment option |
| POST | `/user-punishments/{id}/pick` | Pick random punishment |

### 8.5 Communication

| Method | Path | Purpose |
|--------|------|---------|
| GET | `/households/{id}/chat` | Chat messages |
| POST | `/households/{id}/chat` | Send message |
| WS | `/ws` | WebSocket (real-time) |
| GET | `/households/{id}/notes` | Notes |
| GET | `/households/{id}/announcements` | Announcements |
| GET | `/households/{id}/activity` | Activity log |

---

## 9. Frontend Structure

### 9.1 Pages

| Path | Page | Description |
|------|------|-------------|
| `/` | Dashboard | Whitelist tasks, invitations |
| `/login` | Login | Sign in |
| `/register` | Register | Registration |
| `/households/:id` | Household | Household overview |
| `/households/:id/tasks` | Tasks | Task management |
| `/households/:id/rewards` | Rewards | Rewards |
| `/households/:id/punishments` | Punishments | Punishments |
| `/households/:id/chat` | Chat | Real-time chat |
| `/households/:id/notes` | Notes | Notes |
| `/households/:id/activity` | Activity | Activity log |
| `/households/:id/settings` | Settings | Settings |
| `/user-settings` | UserSettings | User settings |

### 9.2 Components

- `TaskCard`: Task display with completion UI
- `RewardModal`, `PunishmentModal`: Create/edit
- `TaskModal`, `CategoryModal`: Task forms
- `PendingReviews`: Pending approvals
- `Navbar`, `HouseholdTabs`: Navigation
- `ChatMessage`, `AnnouncementBanner`: Communication

---

## 10. Settings & Feature Toggles

### 10.1 Household Settings

| Setting | Description |
|---------|-------------|
| `dark_mode` | Enable dark mode |
| `role_label_*` | Custom role labels |
| `hierarchy_type` | Equals, Organized, Hierarchy |
| `timezone` | Timezone |
| `rewards_enabled` | Enable rewards |
| `punishments_enabled` | Enable punishments |
| `chat_enabled` | Enable chat |

### 10.2 User Settings

| Setting | Description |
|---------|-------------|
| `language` | Language (en, de) |

---

## 11. Business Rules

### 11.1 Data Integrity

- **Soft Deletes**: Chat messages only
- **Archiving**: Tasks (preserves history, can be unarchived)
- **Hard Deletes**: All other entities
- **Cascade Deletes**: Delete household → delete all data
- **Activity Logs**: Immutable

### 11.2 Invitations

- Email-based
- 7 days validity
- Status: Pending → Accepted/Declined/Expired
- Role assigned at invitation

### 11.3 Chat

- Soft-delete for messages
- WebSocket for real-time
- Edit/delete own messages only

---

## 12. Build & Deployment

### 12.1 Development

```bash
# Development environment
nix develop

# Start backend
cargo run -p backend

# Start frontend
cd frontend && trunk serve

# Tests
cargo test --workspace

# Checks
cargo check --workspace
cargo clippy --workspace
```

### 12.2 Production

```bash
# Build backend
nix build .#backend

# Build frontend
nix build .#frontend
```

### 12.3 Environment Variables

| Variable | Default | Description |
|----------|---------|-------------|
| `DATABASE_URL` | `sqlite://household.db` | Database path |
| `JWT_SECRET` | (required) | JWT signing key |
| `HOST` | `127.0.0.1` | Server address |
| `PORT` | `8080` | Server port |
| `STATIC_FILES_PATH` | `./static` | Frontend files |
| `CORS_ORIGINS` | `http://localhost:3000` | CORS origins |

---

## 13. Quality Requirements

1. **No warnings**: Workspace denies warnings
2. **No clippy warnings**: All clippy rules satisfied
3. **Tests required**: Changes need tests
4. **SQLx offline mode**: Compile-time query verification
5. **Shared types**: No API type duplication
