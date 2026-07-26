---
task: 260726-vdz
title: Assignee cannot undo a completion (optional per-task flag)
type: quick
status: planned
created: 2026-07-26
extends: 260726-th8
---

# 260726-vdz: Assignee cannot uncomplete

## Objective

Add an optional per-task boolean `assignee_cannot_uncomplete` (default `false`). Real use case:
bad habits that housemates "pin on" the assignee — the assignee must not be able to clear the
completion himself; someone else from the household has to do it.

## Design contract (source of truth)

**Semantics when the flag is ON:**

| Action | Who | Result |
| --- | --- | --- |
| Complete | every household member, **including the assignee** | allowed |
| Uncomplete | every household member **except the assigned user** | allowed |
| Uncomplete | the assigned user | refused (`TaskError::AssigneeCannotUncomplete` → HTTP 403) |
| Counting | all members pool household-wide | like `anyone_can_complete` |

When the flag is OFF, behaviour is entirely unchanged.

**DRY requirement (explicit):** the flag implies the same household-wide behaviour as
`anyone_can_complete`. Do **not** add a second parallel branch at each call site. Introduce ONE
shared predicate on `shared::Task`:

```rust
pub fn is_household_wide(&self) -> bool {
    self.anyone_can_complete || self.assignee_cannot_uncomplete
}
```

and route the two existing helpers in `backend/src/services/tasks.rs`
(`is_completable_by`, `completion_count_filter`, both introduced by 260726-th8) *and* the frontend
predicate `TaskWithStatus::is_completable_by_user()` through it. Putting it on `shared::Task` (rather
than a private fn in the tasks service) is the variant explicitly allowed by the design because the
frontend needs the exact same predicate.

**New error variant:** `TaskError::AssigneeCannotUncomplete` — deliberately *not* `NotAssigned`, the
two mean different things and the frontend must be able to tell them apart. `NotAssigned` currently
falls through the catch-all `400 uncomplete_error` in `handlers/tasks.rs::uncomplete_task`; the new
variant gets an **explicit** arm mapping to `403 assignee_cannot_uncomplete`.

**Keep working:** the `(? IS NULL OR user_id = ?)` DELETE subqueries in `uncomplete_task` (from
260726-th8) — a non-assignee must still find a row to delete.

**Shared UI logic:** add `TaskWithStatus::can_uncomplete()`. Careful: `is_user_assigned` is `true`
for *unassigned* tasks too (documented on the field), so "is the assignee" must be
`task.assigned_user_id.is_some() && is_user_assigned`.

## Tasks

### Task 1 — Thread the field through backend + shared types

Files:

- `backend/migrations/20240149000000_task_assignee_cannot_uncomplete.sql` (new)
  `ALTER TABLE tasks ADD COLUMN assignee_cannot_uncomplete BOOLEAN NOT NULL DEFAULT 0;`
- `shared/src/types.rs`: `Task.assignee_cannot_uncomplete: bool`,
  `CreateTaskRequest`/`UpdateTaskRequest.assignee_cannot_uncomplete: Option<bool>`.
- `backend/src/models/task.rs`: `TaskRow` + `TaskRowWithCategory` field + both `to_shared()`.
- `backend/src/services/tasks.rs`: INSERT/UPDATE column + bind, `create_task` default `false`,
  `update_task` patch, `list_pending_reviews` projection (`t_assignee_cannot_uncomplete`).
- `backend/src/handlers/tasks.rs`: Solo-Mode "no other changes" guard.
- **Test schema parity (mandatory):** `backend/src/test_utils.rs::create_test_schema()` +
  builder field + `with_assignee_cannot_uncomplete(bool)` + INSERT bind + returned `Task`;
  inline test schemas in `backend/src/services/tasks.rs` and
  `backend/src/services/background_jobs.rs`.
- Mechanically extend every `Task` / `CreateTaskRequest` / `UpdateTaskRequest` struct literal in the
  workspace (backend, shared, frontend) — everywhere `anyone_can_complete` appears as a literal.

Done when: `nix develop -c cargo check --workspace` is clean.

### Task 2 — Enforcement + shared predicates + tests

Files: `shared/src/types.rs`, `backend/src/services/tasks.rs`, `backend/src/handlers/tasks.rs`

- `shared::Task::is_household_wide()` as above.
- `is_completable_by` / `completion_count_filter` route through `is_household_wide()`
  (no second branch).
- `TaskWithStatus::is_completable_by_user()` uses `is_household_wide()`.
- `TaskWithStatus::can_uncomplete()` — false for the assignee when the flag is on.
- `TaskError::AssigneeCannotUncomplete` + guard in `uncomplete_task` (after the
  `is_completable_by` guard, so the more specific error wins for the assignee).
- Handler: explicit `403` arm.

Tests (backend `#[tokio::test]`, minimum set from the method section):
1. flag ON → assignee is refused on uncomplete (`AssigneeCannotUncomplete`)
2. flag ON → a different household member CAN uncomplete
3. flag ON → the assignee can still complete
4. flag OFF → uncomplete behaves exactly as before (assignee OK, non-assignee `NotAssigned`)
5. `create_task` default is `false` and round-trips; `update_task` toggles it
6. flag ON → completions pool household-wide toward `target_count`

Tests (shared `#[test]`): `can_uncomplete()` false for assignee with flag on, true for
non-assignee, true for assignee with flag off, and unassigned tasks are unaffected.

Tests (`test_utils`): schema parity + builder round-trip.

### Task 3 — Frontend toggle + gating + i18n

Files: `frontend/src/components/task_fields.rs`, `task_modal.rs`, `task_card.rs`,
`frontend/src/translations/{de,en}.json`, `frontend/src/i18n/mod.rs`

- New `TaskAssigneeCannotUncompleteField` (mirrors `TaskAnyoneCanCompleteField`, same
  `form-group`/`form-hint` markup → no new CSS, mobile-first N/A).
- `task_modal.rs`: signal, rendered next to the `anyone_can_complete` control in **both** places
  (normal edit and bulk edit with its own `apply_*` flag), sent in create + update requests.
- `task_card.rs`: the `−` button gated on `can_uncomplete()` — following the existing idiom in that
  component (`+` is gated via `disabled=`), the `−` button gets
  `disabled=!has_completions || !can_uncomplete` plus a `title` explaining why.
- i18n keys `task_modal.assignee_cannot_uncomplete` / `.assignee_cannot_uncomplete_hint` and
  `task_card.cannot_uncomplete` in de + en; extend the i18n key-presence test.

## Verification

```bash
nix develop -c cargo test --workspace
nix develop -c cargo clippy --workspace
```

No `cargo sqlx prepare` needed (non-macro `sqlx::query()` API throughout).

## Commit

`jj commit <code paths> -m "feat(tasks): let others clear a completion the assignee cannot undo"`
(planning docs stay uncommitted — the orchestrator commits them).
