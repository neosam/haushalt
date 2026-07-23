# Roadmap: Household Manager

## Overview

The v1.0 MVP is shipped (18 capability areas, full household task/habit management with gamification). The active v1.1 milestone hardens the habit tracker with comprehensive test coverage and adds read-only offline support, with a recurrence-extension placeholder pending definition.

## Milestones

- ✅ **v1.0 MVP** - Phases shipped across 18 capabilities (shipped before GSD migration)
- 🚧 **v1.1 Hardening & Connectivity** - Phases 1-4 (in progress)
- 📋 **v2.0 (future)** - TBD

## Phases

**Phase Numbering:**
- Integer phases (1, 2, 3): Planned milestone work
- Decimal phases (2.1, 2.2): Urgent insertions (marked with INSERTED)

<details>
<summary>✅ v1.0 MVP — SHIPPED (18 capability areas)</summary>

The original application, built before GSD. Shipped capabilities (validated, see `.planning/REQUIREMENTS.md`):
authentication, households, user-management, tasks, task-categories, task-period-tracking,
task-text-filter, rewards, punishments, point-conditions, dashboard, household-statistics,
announcements, invitations, chat, journal, notes, activity-logs.

This work predates GSD tracking; it is represented as Validated requirements rather than phases.

</details>

### 🚧 v1.1 Hardening & Connectivity (In Progress)

**Milestone Goal:** Harden the habit tracker against regressions with comprehensive service-layer tests, and add read-only offline task viewing with local caching and auto-sync.

#### Phase 1: Custom Recurrence Period Fix
**Goal**: Custom recurrence tasks track each custom date as an independent period instead of an all-time window
**Depends on**: v1.0 MVP
**Requirements**: TPT-FIX-01
**Success Criteria** (what must be TRUE):
  1. A custom recurrence task with multiple dates tracks completions per date
  2. Users can complete a task for different custom dates when `allow_exceed_target=false`
  3. Period bounds for Custom are `(date, date)`, not `(1970, 2100)`
**Plans**: 1 plan

Plans:
- [x] 01-01: Change Custom recurrence from `TimePeriod::None` to `TimePeriod::Day` in `get_period_bounds` + tests

#### Phase 2: Habit Tracker Test Coverage
**Goal**: Build comprehensive service-layer tests for habit tracking so regressions are caught, with shared test infrastructure
**Depends on**: Phase 1
**Requirements**: TEST-01, TEST-02, TEST-03, TEST-04, TEST-05, TEST-06, TEST-07, TEST-08
**Success Criteria** (what must be TRUE):
  1. Task, period-results, consequences, and background-jobs services have thorough unit tests
  2. Shared test utilities (in-memory DB, fixtures, builders, assertion helpers) exist and are reused
  3. All recurrence types, period tracking, good/bad habits, and pause/vacation interactions are covered
  4. `cargo test --workspace` and `cargo clippy --workspace` are clean; test count grew from ~131 to 200+
**Plans**: 5 plans

Plans:
- [x] 02-01: Test infrastructure & assertion helpers (`backend/src/test_utils.rs`)
- [x] 02-02: Task service tests — creation & completion (partial: points-service integration pending)
- [ ] 02-03: Period results & task consequences tests
- [ ] 02-04: Background jobs, integration workflows & edge/timezone cases
- [ ] 02-05: Verification & cleanup (run suite, clippy, review coverage)

#### Phase 3: Extend Recurrence Types
**Goal**: Extend the set of supported recurrence types (scope undefined — define via `/gsd-discuss-phase` before planning)
**Depends on**: Phase 2
**Requirements**: RECTR-01
**Success Criteria** (what must be TRUE):
  1. Scope is defined and agreed before implementation
  2. New recurrence types integrate with period tracking, scheduling, and UI consistently
**Plans**: TBD

Plans:
- [ ] 03-01: TBD — define scope, then plan

#### Phase 4: Offline Task Viewing
**Goal**: Users can view their tasks while offline via local IndexedDB caching, with an offline indicator and auto-sync on reconnect
**Depends on**: Phase 2
**Requirements**: OFFLINE-01, OFFLINE-02, OFFLINE-03, OFFLINE-04, OFFLINE-05
**Success Criteria** (what must be TRUE):
  1. User can view cached tasks with no network connection
  2. Offline indicator appears when connection is lost
  3. Data syncs automatically when connection returns (server wins)
  4. Interactive actions are disabled while offline
**Plans**: 3 plans

Plans:
- [ ] 04-01: IndexedDB caching layer (Tasks, TaskWithStatus, Households stores)
- [ ] 04-02: Offline detection, UI indicators, disabled action buttons
- [ ] 04-03: Network-first API client with cache fallback and reconnect sync

## Progress

**Execution Order:**
Phases execute in numeric order: 1 → 2 → 3 → 4

| Phase | Milestone | Plans Complete | Status | Completed |
|-------|-----------|----------------|--------|-----------|
| 1. Custom Recurrence Period Fix | v1.1 | 1/1 | Complete | 2026-07-23 |
| 2. Habit Tracker Test Coverage | v1.1 | 2/5 | In progress | - |
| 3. Extend Recurrence Types | v1.1 | 0/TBD | Not started | - |
| 4. Offline Task Viewing | v1.1 | 0/3 | Not started | - |
