# Requirements: Household Manager

**Defined:** 2026-07-23
**Core Value:** Households can fairly delegate, track, and gamify recurring chores and habits across members with transparent points and streaks.

## Validated Requirements

The v1.0 application is shipped. These capabilities are live and verified by the running app and its test suite. Listed at capability level; the codebase + tests are the living behavioral spec.

| Capability | REQ Range | Count | Summary |
| ---------- | --------- | ----- | ------- |
| authentication | AUTH-01..05 | 5 | Signup/login, refresh-token rotation, persistent sessions, logout |
| households | HSH-01..17 | 17 | Create/manage households, members, roles, settings, vacation mode |
| user-management | USR-01..04 | 4 | Profiles, preferences, language (en/de) |
| tasks | TASK-01..27 | 27 | CRUD, recurrence, completion, review, good/bad habits, archive, bulk edit, suggestions, pause, FAB, context menu |
| task-categories | TCAT-01..05 | 5 | Categorize tasks |
| task-period-tracking | TPT-01..09 | 9 | Per-recurrence period bounds, streak/period results, early completion |
| task-text-filter | TFIL-01 | 1 | Filter household tasks by text |
| rewards | RWD-01..21 | 21 | Point rewards linked to tasks |
| punishments | PUN-01..20 | 20 | Point punishments linked to tasks |
| point-conditions | PCON-01..05 | 5 | Conditional point rules |
| dashboard | DASH-01..13 | 13 | Aggregated dashboard views |
| household-statistics | HSTAT-01..10 | 10 | Household-level statistics |
| announcements | ANN-01..06 | 6 | Household announcements and banner |
| invitations | INV-01..06 | 6 | Invite members to households |
| chat | CHAT-01..05 | 5 | Real-time household chat over WebSocket |
| journal | JRN-01..06 | 6 | Journal entries |
| notes | NOTE-01..05 | 5 | Notes |
| activity-logs | ALOG-01..02 | 2 | Activity logging |
| task-period-tracking (custom fix) | TPT-FIX-01 | 1 | Custom recurrence tracks each date as an independent period (shipped) |

> Detailed BDD scenarios for shipped capabilities are preserved in jj history under the former `openspec/specs/`. The authoritative source for shipped behavior is the code and tests in `backend/` and `frontend/`.

## v1 Requirements

Active milestone **v1.1 Hardening & Connectivity**. Each maps to a roadmap phase.

### Habit Tracker Test Coverage

- [ ] **TEST-01**: Task service has comprehensive tests for creation, completion, uncompleting, and assignment validation
  - Complete assigned/unassigned tasks; reject completion of others' assigned tasks
  - `requires_review` produces Pending; otherwise Approved and points awarded
  - Uncomplete removes the record and reverts points; cannot uncomplete others' completions
- [ ] **TEST-02**: Period results service has tests for period tracking, completion counting, and target validation
  - Period result created/updated when completion reaches target; `target_count` frozen at finalization
  - Deleted when uncomplete drops below target; failed periods finalized for incomplete "yesterday"
  - Skipped periods (paused/vacation) excluded from completion-rate and streak calculations
  - Early completion (`completion_due_date`) and period bounds for every recurrence type
  - Multiple completions honored only when `allow_exceed_target=true`
- [ ] **TEST-03**: Task consequences service has tests for rewards/punishments and good/bad habit logic
  - Good habit: completion awards points/rewards; miss deducts penalty/punishments
  - Bad habit: completion (indulge) deducts; resistance (failed period) awards
  - No points configured → no-op
- [ ] **TEST-04**: Background jobs service has tests for automated punishments, streak updates, and auto-archiving
  - Auto-archive one-time/custom tasks after grace period; never archive incomplete tasks; configurable grace period; `TaskAutoArchived` activity logged
  - Period finalization respects household timezone; handles paused tasks and vacation mode
- [ ] **TEST-05**: Integration tests cover complete habit workflows (daily, weekly, custom recurrence, vacation, good/bad habits)
- [ ] **TEST-06**: Edge case tests cover timezone handling (DST, UTC±), leap years, end-of-month, concurrent completions
- [ ] **TEST-07**: Paused tasks and vacation mode interactions are tested (penalties skipped, manual completion allowed, resume after unpause/vacation)
- [ ] **TEST-08**: Shared test infrastructure exists (in-memory DB pool, migrations, fixture builders, domain assertion helpers)

### Extend Recurrence Types

- [ ] **RECTR-01**: Extend supported recurrence types (scope to be defined — run `/gsd-discuss-phase` for this phase before planning)

### Offline Support

- [ ] **OFFLINE-01**: User can view cached task data while offline (read-only)
- [ ] **OFFLINE-02**: Task data is cached locally using IndexedDB (stores for Tasks, TaskWithStatus, Households)
- [ ] **OFFLINE-03**: An offline indicator is shown when the connection is lost
- [ ] **OFFLINE-04**: Data auto-syncs when the connection is restored (server wins on conflict)
- [ ] **OFFLINE-05**: Interactive actions (complete/edit/create) are disabled while offline

### Daily Task Report

- [ ] **RPT-01**: The logged-in user can see a report of the tasks that are due for them today in the current household
- [ ] **RPT-02**: The logged-in user can see a report of the tasks they missed on the previous day
- [ ] **RPT-03**: Both reports are reachable from the household as a dedicated view, with an empty state when nothing is due/missed
- [ ] **RPT-04**: Report data is served by the backend (service-layer logic, covered by tests) rather than assembled ad-hoc in the frontend

### Task Deletion from Edit Modal

- [x] **TDEL-01**: The task edit modal offers a delete action, available from both the Tasks page and the household Overview page
- [x] **TDEL-02**: Deleting requires an explicit in-modal confirmation step before the task is removed
- [x] **TDEL-03**: After a successful delete the modal closes and the underlying task list reflects the removal
- [x] **TDEL-04**: The delete action respects existing task permissions — it is not offered to users who may not delete the task

## v1.2 Outbound Messaging (Planned)

### Public Cross-Household Report Links

- [x] **PUBREP-01**: A user creates one or more named reports in their user settings, each spanning an explicitly chosen set of the households they belong to
- [x] **PUBREP-02**: Each report exposes a URL carrying an unguessable UUID token that returns the report without any authentication
- [x] **PUBREP-03**: The public response contains nothing but the report text — the same content as the per-household daily report, one block per selected household
- [x] **PUBREP-04**: Each report carries its own output language (de/en); the per-household report endpoint stays English (D-01)
- [x] **PUBREP-05**: A report can be switched off and its token regenerated, which immediately invalidates the previous URL
- [x] **PUBREP-06**: A household whose membership the owner has lost is silently dropped from the output, so a stale link cannot leak data
- [x] **PUBREP-07**: The public endpoint is rate limited per token and marked non-indexable

### Nomi.ai Daily Report Push (deferred)

- [ ] **NOMI-01**: A user configures, per household, their own nomi.ai connection — target, API key, send time and an on/off switch — in one settings section
- [ ] **NOMI-07**: The target may be either a single Nomi or a Room (group chat); both are offered for selection by name, and the delivery path treats them as one abstraction rather than a branch at the call site
- [ ] **NOMI-02**: The API key is stored encrypted at rest and is never returned to the client in plaintext
- [ ] **NOMI-03**: At the configured local time (household timezone) the daily report is delivered to the configured Nomi as an OOC message
- [ ] **NOMI-04**: A report exceeding the nomi.ai message length limit is shortened rather than failing, and the truncation is visible in the message
- [ ] **NOMI-05**: Delivery survives the documented API failure modes — `RoomStillCreating`, `NoReply`, `NomiStillResponding`, `TooManyRequests` (HTTP 429), `MessageLengthLimitExceeded` / `MessageCharacterLimitExceeded` — without aborting the scheduled run for other users or households
- [ ] **NOMI-06**: Content, destination and schedule are separable, so a further content type can be added later without changing the delivery path

## Out of Scope

Explicitly excluded. Documented to prevent scope creep.

| Feature | Reason |
| ------- | ------ |
| Frontend/UI automated tests | Backend service-layer tests are sufficient for habit logic |
| HTTP endpoint integration tests | Mostly boilerplate; service layer is the focus |
| Performance/load testing | Not needed at household scale |
| Automated coverage tooling | Manual verification via code review is sufficient |
| Offline editing/creation | Offline is read-only by design; server wins |
| Migration of old custom-recurrence period records | Old 1970-2100 records are superseded by correct per-date records; harmless |

## Traceability

Which phases cover which requirements. Updated during roadmap creation.

| Requirement | Phase | Status |
| ----------- | ----- | ------ |
| TPT-FIX-01 | Phase 1 | Complete |
| TEST-01 | Phase 2 | In Progress |
| TEST-02 | Phase 2 | In Progress |
| TEST-03 | Phase 2 | Pending |
| TEST-04 | Phase 2 | Pending |
| TEST-05 | Phase 2 | Pending |
| TEST-06 | Phase 2 | Pending |
| TEST-07 | Phase 2 | Pending |
| TEST-08 | Phase 2 | In Progress |
| RECTR-01 | Phase 3 | Pending |
| OFFLINE-01 | Phase 4 | Pending |
| OFFLINE-02 | Phase 4 | Pending |
| OFFLINE-03 | Phase 4 | Pending |
| OFFLINE-04 | Phase 4 | Pending |
| OFFLINE-05 | Phase 4 | Pending |
| RPT-01 | Phase 2.1 | Pending |
| RPT-02 | Phase 2.1 | Pending |
| RPT-03 | Phase 2.1 | Pending |
| RPT-04 | Phase 2.1 | Pending |
| TDEL-01 | Quick task | Complete |
| TDEL-02 | Quick task | Complete |
| TDEL-03 | Quick task | Complete |
| TDEL-04 | Quick task | Complete |
| PUBREP-01 | Phase 6 | Complete |
| PUBREP-02 | Phase 6 | Complete |
| PUBREP-03 | Phase 6 | Complete |
| PUBREP-04 | Phase 6 | Complete |
| PUBREP-05 | Phase 6 | Complete |
| PUBREP-06 | Phase 6 | Complete |
| PUBREP-07 | Phase 6 | Complete |
| NOMI-01 | Phase 5 | Pending |
| NOMI-02 | Phase 5 | Pending |
| NOMI-03 | Phase 5 | Pending |
| NOMI-04 | Phase 5 | Pending |
| NOMI-05 | Phase 5 | Pending |
| NOMI-06 | Phase 5 | Pending |
| NOMI-07 | Phase 5 | Pending |

**Coverage:**
- v1 requirements: 22 total
- Mapped to phases: 22
- Unmapped: 0 ✓

---
*Requirements defined: 2026-07-23*
*Last updated: 2026-07-26 — added Daily Task Report (RPT-01..04) and Task Deletion from Edit Modal (TDEL-01..04)*
