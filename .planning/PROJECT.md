# Household Manager

## What This Is

Household Manager is a full-stack Rust application for collaborative task and habit tracking for families and shared living situations. Members delegate recurring chores, track completion/streaks, and stay motivated through gamified points, rewards, and punishments with flexible role-based access control.

## Core Value

Households can fairly delegate, track, and gamify recurring chores and habits across members with transparent points and streaks.

## Requirements

### Validated

<!-- Shipped and confirmed valuable. These capabilities are live in the app. -->

- ✓ **authentication** — JWT signup/login, refresh-token rotation, persistent sessions
- ✓ **households** — create/manage households, members, roles, settings, vacation mode
- ✓ **user-management** — user profiles, preferences, language (en/de)
- ✓ **tasks** — CRUD, recurrence, completion, review, good/bad habits, archive, bulk edit, suggestions, pause
- ✓ **task-categories** — categorize tasks
- ✓ **task-period-tracking** — per-recurrence period bounds, streak and period results
- ✓ **task-text-filter** — filter household tasks by text
- ✓ **rewards** — point rewards linked to tasks
- ✓ **punishments** — point punishments linked to tasks
- ✓ **point-conditions** — conditional point rules
- ✓ **dashboard** — aggregated dashboard views
- ✓ **household-statistics** — household-level statistics
- ✓ **announcements** — household announcements and banner
- ✓ **invitations** — invite members to households
- ✓ **chat** — real-time household chat over WebSocket
- ✓ **journal** — journal entries
- ✓ **notes** — notes
- ✓ **activity-logs** — activity logging
- ✓ **task-period-tracking (custom fix)** — custom recurrence tracks each date as an independent period (shipped hotfix)

### Active

<!-- Current scope. Building toward these. -->

- [ ] Habit tracker test coverage (service-layer regression safety net)
- [ ] Extend recurrence types (scope TBD — needs definition)
- [ ] Read-only offline task viewing with local caching and auto-sync

### Out of Scope

<!-- Explicit boundaries. Includes reasoning to prevent re-adding. -->

- Frontend/UI automated testing — backend service-layer tests provide sufficient coverage for habit logic
- HTTP endpoint integration tests — service-layer tests are the focus; handler tests would be mostly boilerplate
- Performance/load testing — not needed for household-scale usage
- Automated coverage metrics tooling — manual verification via code review is sufficient
- Offline editing/creation — offline mode is read-only by design; server wins on conflict

## Context

- **Tech stack**: Rust full-stack — Actix-web + SQLx + SQLite backend, Leptos CSR WASM frontend, shared types crate, Nix + Trunk build.
- **Architecture**: Backend is layered (handlers → services → models) with JWT auth middleware and rate limiting. Frontend is component-based with a central `ApiClient` and i18n (en/de).
- **Domain**: Household task management with gamification (points, rewards, punishments) and role hierarchy (Owner > Admin > Member). `HierarchyType` (Equals/Organized/Hierarchy) controls who can manage and be assigned.
- **Prior work**: The v1.0 app is shipped across 18 capability areas. Active work focuses on hardening (test coverage) and connectivity (offline support). See `docs/constitution.md` for the full domain model and `docs/architecture/` for technical detail.
- **Known issues**: Custom recurrence previously lumped all completions into an all-time period — fixed and shipped. Habit tracker service modules had minimal test coverage — being addressed.

## Constraints

- **Tech stack**: Rust (Actix-web, SQLx, SQLite, Leptos, WASM) + Nix — do not introduce other languages/runtimes
- **Quality**: Workspace denies warnings; `cargo clippy --workspace` must be clean; tests required for changes
- **Build**: SQLx offline mode (`SQLX_OFFLINE=true`) — run `cargo sqlx prepare` after schema changes
- **VCS**: Use jujutsu (`jj`) for commits, not git directly
- **CSS**: Mobile-first (`min-width` media queries); current CSS is desktop-first and needs migration
- **Compatibility**: Frontend imports shared types crate — API types must stay identical on both sides

## Key Decisions

<!-- Decisions that constrain future work. Add throughout project lifecycle. -->

| Decision | Rationale | Outcome |
| -------- | --------- | ------- |
| GSD (Get Shit Done) for spec-driven workflow | Lightweight `.planning/` system with checkable requirements, phased roadmap, and fresh-context execution; replaces OpenSpec | ✓ Good |
| In-memory SQLite per test for habit tracker tests | Fast, perfectly isolated, SQLx supports `:memory:`; migrations applied per test | - Pending |
| Pass `current_date` to functions under test (no global time mocking) | Explicit, predictable, matches existing scheduler signatures | - Pending |
| Custom recurrence uses `TimePeriod::Day` (per-date tracking) | Each custom date tracked independently like Weekdays; `Day` gives `(date, date)` bounds | ✓ Good |
| Offline mode is read-only, server wins on reconnect | Simplicity over conflict resolution; avoids offline mutation races | - Pending |

---
*Last updated: 2026-07-23 after migration from OpenSpec to GSD*
