---
task: 260726-th8
title: Anyone can complete task (optional per-task flag)
type: quick
status: complete
created: 2026-07-26
completed: 2026-07-26
commit: 1f3e965d
tests_added: 14
---

# 260726-th8: Anyone can complete task — Summary

Optional per-task flag `anyone_can_complete` (default `false`). When enabled, **any** household
member may check the task off — and uncheck it again — not just the assignee. With the flag off,
behaviour is unchanged.

## Commit

`1f3e965d` — `feat(tasks): allow anyone to complete a task when enabled`

## What was built

### Schema

- `backend/migrations/20240148000000_task_anyone_can_complete.sql`:
  `ALTER TABLE tasks ADD COLUMN anyone_can_complete BOOLEAN NOT NULL DEFAULT 0;`
  Verified against a copy of the real `household.db` — applies cleanly, column lands at index 22.
- Same column added to the three test/dev schemas that mirror production:
  `backend/src/test_utils.rs::create_test_schema()`, the inline schema in
  `backend/src/services/tasks.rs`, and the one in `backend/src/services/background_jobs.rs`.

### Types (threaded exactly like `allow_exceed_target`)

- `shared::Task.anyone_can_complete: bool`
- `shared::CreateTaskRequest/UpdateTaskRequest.anyone_can_complete: Option<bool>`
- `backend::TaskRow` + `TaskRowWithCategory` + both `to_shared()` mappings
- `list_pending_reviews` projection gained `t.anyone_can_complete as t_anyone_can_complete`
- `create_task` defaults to `false`, `update_task` patches it, INSERT/UPDATE SQL carry the column
- Solo-Mode "no other changes allowed" guard in `handlers/tasks.rs` includes the new field
- 78 struct literals across backend/shared/frontend extended mechanically

### Enforcement

`backend/src/services/tasks.rs`, new private helpers:

- `is_completable_by(task, user_id)` — replaces the duplicated `NotAssigned` guard in both
  `complete_task` and `uncomplete_task`; passes when the user is the assignee, when the task has
  no assignee, or when `anyone_can_complete` is set.
- `completion_count_filter(task, user_id) -> Option<&Uuid>` — `None` (i.e. household-wide) for
  `anyone_can_complete` tasks, otherwise the current user.
- `count_completions()` / `count_completions_in_period()` — replace four near-identical inline
  COUNT queries (DRY), taking the optional user filter.

`uncomplete_task` also applies the filter to the DELETE subqueries (`(? IS NULL OR user_id = ?)`),
so a task anyone may check off can be unchecked by anyone. `uncomplete_task` performs no point
reversal today (pre-existing, unchanged), so this has no cross-member point side effects.

### Shared UI logic

`shared/src/types.rs`:

```rust
pub fn is_completable_by_user(&self) -> bool {
    self.is_user_assigned || self.task.anyone_can_complete
}

pub fn can_complete(&self) -> bool {
    self.is_completable_by_user() && (self.task.allow_exceed_target || !self.is_target_met())
}
```

`is_user_assigned` keeps its literal meaning (it is still `false` for a non-assignee) and is still
used for the "assigned to you" badge.

### Frontend

- `TaskAnyoneCanCompleteField` in `task_fields.rs` (mirrors `TaskAllowExceedField`; no new CSS, so
  mobile-first is N/A).
- `task_modal.rs`: `anyone_can_complete` signal, control rendered directly below the
  `allow_exceed_target` control, sent in create + update requests, plus a bulk-edit `BulkEditField`
  with its own `apply_anyone_can_complete` flag.
- `task_card.rs`: the `+/-` controls were gated on `is_user_assigned` alone — now on
  `is_completable_by_user()`.
- i18n `task_modal.anyone_can_complete` / `.anyone_can_complete_hint` in `de.json` + `en.json`.

## Finding: per-period completion counting

**The design's suspicion was correct, and it was worse than described.** Four counting sites in
`complete_task` filtered by `user_id`:

| Site | Old behaviour | New behaviour |
| --- | --- | --- |
| OneTime `AlreadyCompleted` guard | always per-user | household-wide when `anyone_can_complete` |
| Period `AlreadyCompleted` guard | always per-user | household-wide when `anyone_can_complete` |
| OneTime period finalization (~752) | per-user **iff** `assigned_user_id.is_some()` | additionally household-wide when `anyone_can_complete` |
| Recurring period finalization (~788) | per-user **iff** `assigned_user_id.is_some()` | additionally household-wide when `anyone_can_complete` |

The two guards were stricter than the design's note suggested: they filtered per-user even for
*unassigned* tasks, while finalization for the same task counted household-wide. Without changing
the guards, an `anyone_can_complete` task with `allow_exceed_target = false` and `target_count = 1`
would have let every member complete it once (N completions for a target of 1), and the backend
would have been more permissive than the UI, which computes `is_target_met()` from the
household-wide `completions_today`. Both guards now pool completions when the flag is set, so
backend and UI agree.

The pre-existing per-user guard for *unassigned* tasks (flag off) was deliberately left untouched —
out of scope, and changing it would alter existing behaviour.

Covered by `test_anyone_can_complete_counts_all_members_toward_target` (target 2 reached by two
different members → period finalized `completed` with `completions_count = 2`, third completion
rejected as `AlreadyCompleted`) and by
`test_assigned_task_without_flag_counts_only_assignee` (flag off → unchanged).

## Tests (14 added, all passing)

Backend `services::tasks::tests`:
1. `test_create_task_anyone_can_complete_defaults_to_false` — default preserved, round-trips through DB
2. `test_complete_task_anyone_can_complete_allows_non_assignee` — plus asserts the completion is
   recorded for the acting user (points go to whoever checks it off)
3. `test_complete_task_anyone_can_complete_off_forbids_non_assignee` — `TaskError::NotAssigned`
4. `test_uncomplete_task_anyone_can_complete_allows_non_assignee`
5. `test_uncomplete_task_anyone_can_complete_off_forbids_non_assignee`
6. `test_update_task_toggles_anyone_can_complete`
7. `test_anyone_can_complete_counts_all_members_toward_target`
8. `test_assigned_task_without_flag_counts_only_assignee`

Backend `test_utils::tests`:
9. `test_task_anyone_can_complete_insertable_and_defaults_off` — schema parity + builder

Shared `types::tests`:
10. `test_task_with_status_can_complete_not_assigned_but_anyone_can_complete`
11. `test_task_with_status_anyone_can_complete_defaults_off`
12. `test_task_with_status_anyone_can_complete_still_respects_target`
13. `test_task_with_status_is_completable_by_user_when_assigned`

Frontend `i18n::tests`:
14. `test_anyone_can_complete_keys_present_in_both_languages`

Two further `#[wasm_bindgen_test]` cases were added in `task_card.rs`
(`test_non_assignee_can_complete_when_anyone_can_complete`, `test_non_assignee_cannot_complete_by_default`).
They type-check under `cargo test` but the whole `task_card` test module is browser-only
(`wasm_bindgen_test_configure!(run_in_browser)`), so they do **not** execute natively — which is why
the natively-running i18n test was added and why the gating predicate itself lives in `shared`
where it *is* covered by running tests. Not counted in the 14.

## Verification (real output)

`nix develop -c cargo test --workspace`:

```
test result: ok. 278 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 5.65s   (backend)
test result: ok. 0 passed; 0 failed; ...                                                          (backend bin)
test result: ok. 30 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s    (frontend)
test result: ok. 42 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s    (shared)
test result: ok. 1 passed; 0 failed; ...                                                          (frontend doc-tests)
```

`nix develop -c cargo clippy --workspace`:

```
    Checking shared v0.1.0
    Checking backend v0.1.0
    Checking frontend v0.1.0
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 32.75s
```

Clean — no new warnings. The known pre-existing `--all-targets` clippy noise was not touched.

No `cargo sqlx prepare` was needed: the backend has zero `sqlx::query!` macro invocations and there
is no `.sqlx/` directory or `sqlx-data.json` in the repo (verified before relying on it).

## Deviations from the design

1. **`can_complete()` body extracted into `is_completable_by_user()`.** The design specified
   `(self.is_user_assigned || self.task.anyone_can_complete) && (...)`. The expression is identical,
   but it is now behind a named method because `task_card.rs` needs the same predicate (see 2).
   `is_user_assigned` was *not* forced to `true`, as required.
2. **`task_card.rs` +/- gating changed (not mentioned in the design).** The design only named
   `can_complete()`, but the `+`/`-` buttons are rendered behind `if is_user_assigned` — without
   also changing that, a non-assignee would never see the button and the feature would be invisible.
   Changed to `is_completable_by_user()`. The "assigned to you" badge still uses `is_user_assigned`.
3. **Guard-side counting changed too.** The design asked only about the finalization counting at
   ~752/~788; the two `AlreadyCompleted` guards had the same problem and were changed as well, for
   backend/UI consistency. Rationale in the finding section above.
4. **Bulk edit also got the toggle.** The design said "next to the existing `allow_exceed_target`
   control" — there are two such controls in `task_modal.rs` (single edit and bulk edit), so both
   got it.
5. **`uncomplete_task` DELETE scope.** The design required that a task anyone may check off can be
   unchecked again; that needed the DELETE subqueries to drop the `user_id` filter for such tasks,
   not just the guard. Implemented with `(? IS NULL OR user_id = ?)`.

No blockers, no failures, nothing deferred.

## Known Stubs

None.

## Self-Check

- `backend/migrations/20240148000000_task_anyone_can_complete.sql` — FOUND
- `.planning/quick/260726-th8-anyone-can-complete-task/260726-th8-PLAN.md` — FOUND
- Commit `1f3e965d` — FOUND (`jj log -r @-`)
- `cargo test --workspace` — PASSED
- `cargo clippy --workspace` — PASSED

**Self-Check: PASSED**
