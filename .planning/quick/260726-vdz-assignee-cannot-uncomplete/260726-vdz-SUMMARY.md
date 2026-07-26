---
task: 260726-vdz
title: Assignee cannot undo a completion (optional per-task flag)
type: quick
status: complete
created: 2026-07-26
completed: 2026-07-26
commit: a7e0260a
extends: 260726-th8
tests_added: 18
---

# 260726-vdz: Assignee cannot uncomplete — Summary

Optional per-task flag `assignee_cannot_uncomplete` (default `false`). When enabled, **every**
household member may check the task off — **including** the assigned user — and every member's
completions pool toward the target, but the **assigned user may not undo a completion**. Somebody
else from the household has to clear it. Real use case: bad habits that housemates pin on the
assignee.

With the flag off, behaviour is entirely unchanged.

## Commit

`a7e0260a` — `feat(tasks): let others clear a completion the assignee cannot undo`

## What was built

### Schema

- `backend/migrations/20240149000000_task_assignee_cannot_uncomplete.sql`:
  `ALTER TABLE tasks ADD COLUMN assignee_cannot_uncomplete BOOLEAN NOT NULL DEFAULT 0;`
  Verified by replaying the full migration chain into a fresh SQLite DB — column lands at index 23,
  `NOT NULL`, default `0`.
- Test schema parity (all three, as mandated): `backend/src/test_utils.rs::create_test_schema()`,
  the inline schema in `backend/src/services/tasks.rs`, and the one in
  `backend/src/services/background_jobs.rs`.

### Types (threaded exactly like `anyone_can_complete`)

`shared::Task`, `shared::CreateTaskRequest`, `shared::UpdateTaskRequest`, `backend::TaskRow`,
`backend::TaskRowWithCategory` (+ both `to_shared()`), `create_task` default `false`,
`update_task` patch, INSERT/UPDATE SQL + binds, `list_pending_reviews` projection
(`t.assignee_cannot_uncomplete as t_assignee_cannot_uncomplete`), the Solo-Mode "no other changes
allowed" guard in `handlers/tasks.rs`, `TestTaskBuilder` field +
`with_assignee_cannot_uncomplete(bool)` + bind + returned `Task`, and 74 struct literals across
backend/shared/frontend (extended mechanically by script, then hand-corrected where the script
guessed wrong — see Deviations).

### The single shared predicate (DRY requirement)

Placed on `shared::Task` — the variant the design explicitly allowed, chosen because the frontend
needs the exact same predicate:

```rust
pub fn is_household_wide(&self) -> bool {
    self.anyone_can_complete || self.assignee_cannot_uncomplete
}
```

Both existing helpers from 260726-th8 now route through it — no second parallel branch was added
anywhere:

- `backend/src/services/tasks.rs::is_completable_by` → `task.is_household_wide() || ...`
- `backend/src/services/tasks.rs::completion_count_filter` → `if task.is_household_wide() { None }`
- `shared::TaskWithStatus::is_completable_by_user` → `self.is_user_assigned || self.task.is_household_wide()`

That single change gives the new flag household-wide completing **and** household-wide completion
counting for free, exactly as specified.

### Enforcement

New error variant (deliberately not `NotAssigned` — different meaning, and the frontend must be able
to tell them apart):

```rust
#[error("The assigned user may not undo a completion of this task")]
AssigneeCannotUncomplete,
```

`uncomplete_task` gained the guard *after* the `is_completable_by` guard, so the more specific error
wins for the assignee:

```rust
if task.assignee_cannot_uncomplete && is_assignee(&task, user_id) {
    return Err(TaskError::AssigneeCannotUncomplete);
}
```

New private helper `is_assignee(task, user_id)` — `task.assigned_user_id == Some(*user_id)`.

The `(? IS NULL OR user_id = ?)` DELETE subqueries from 260726-th8 keep working: the new flag makes
`completion_count_filter` return `None`, so a non-assignee finds the row to delete.

`handlers/tasks.rs::uncomplete_task` — `NotAssigned` falls through the catch-all `400
uncomplete_error`, so per the design the new variant got an **explicit** arm:

```rust
Err(e @ task_service::TaskError::AssigneeCannotUncomplete) => {
    Ok(HttpResponse::Forbidden().json(ApiError {
        error: "assignee_cannot_uncomplete".to_string(),
        message: e.to_string(),
    }))
}
```

### Shared UI logic

```rust
/// `is_user_assigned` alone is not enough: it is also true for *unassigned* tasks.
pub fn is_assignee(&self) -> bool {
    self.task.assigned_user_id.is_some() && self.is_user_assigned
}

pub fn can_uncomplete(&self) -> bool {
    if self.task.assignee_cannot_uncomplete && self.is_assignee() {
        return false;
    }
    self.is_completable_by_user()
}
```

The `is_assignee()` distinction matters: `TaskWithStatus::is_user_assigned` is documented as "true if
the task has **no** assigned user OR the current user is the assigned user", so a naive
`is_user_assigned` check would have wrongly refused every member on unassigned tasks. Covered by
`test_can_uncomplete_unassigned_task_is_unaffected` and, backend-side, by
`test_assignee_cannot_uncomplete_unassigned_task_refuses_nobody`.

### Frontend

- `TaskAssigneeCannotUncompleteField` in `task_fields.rs` (mirrors `TaskAnyoneCanCompleteField`;
  same `form-group`/`form-hint` markup, no new CSS → mobile-first N/A).
- `task_modal.rs`: `assignee_cannot_uncomplete` signal, rendered directly below the
  `anyone_can_complete` control, sent in create + update requests, plus the bulk-edit
  `BulkEditField` with its own `apply_assignee_cannot_uncomplete` flag (both controls covered, as
  required).
- `task_card.rs`: `can_uncomplete()` now gates the `−` button (`disabled=`, plus a guard in the
  click handler and a `title` tooltip explaining why). The `+` button and the visibility of the
  control block still use `can_complete()` / `is_completable_by_user()`, so the assignee keeps
  seeing progress and can still check the task off.
- i18n: `task_modal.assignee_cannot_uncomplete`, `.assignee_cannot_uncomplete_hint`,
  `task_card.cannot_uncomplete` in `de.json` + `en.json`.

## Tests (18 added, all passing)

Backend `services::tasks::tests`:

1. `test_create_task_assignee_cannot_uncomplete_defaults_to_false` — default `false`, round-trips
2. `test_update_task_toggles_assignee_cannot_uncomplete`
3. `test_assignee_cannot_uncomplete_refuses_the_assignee` — `TaskError::AssigneeCannotUncomplete`,
   and asserts the completion row is still there
4. `test_assignee_cannot_uncomplete_allows_other_member` — row actually deleted
5. `test_assignee_cannot_uncomplete_still_lets_assignee_complete` — assignee *and* other members
   may complete
6. `test_uncomplete_without_restriction_still_allows_assignee` — flag off: assignee OK,
   non-assignee still `NotAssigned` (unchanged behaviour)
7. `test_assignee_cannot_uncomplete_unassigned_task_refuses_nobody`
8. `test_assignee_cannot_uncomplete_counts_all_members_toward_target` — target 2 reached by two
   members → period finalized `completed` with `completions_count = 2`, third rejected
9. `test_list_pending_reviews_projects_completion_flags` — see below

Backend `test_utils::tests`:

10. `test_task_assignee_cannot_uncomplete_insertable_and_defaults_off` — schema parity + builder

Shared `types::tests`:

11. `test_task_is_household_wide` — neither / `anyone_can_complete` / `assignee_cannot_uncomplete`
12. `test_can_uncomplete_false_for_assignee_when_restricted` (also asserts `can_complete()` stays true)
13. `test_can_uncomplete_true_for_other_member_when_restricted`
14. `test_can_uncomplete_true_for_assignee_without_restriction` (+ non-assignee still blocked)
15. `test_can_uncomplete_unassigned_task_is_unaffected`

Frontend `i18n::tests`:

16. `test_assignee_cannot_uncomplete_keys_present_in_both_languages`

Frontend `task_card` (`#[wasm_bindgen_test]`, browser-only module — type-checked but not executed
natively, same caveat as 260726-th8; the predicate itself is covered by the shared tests):

17. `test_assignee_cannot_uncomplete_hides_minus_for_assignee`
18. `test_assignee_cannot_uncomplete_keeps_minus_for_other_member`

### Extra coverage added on purpose

`list_pending_reviews` had **zero** test coverage in the whole repo, and this change edits its
hand-written SQL projection + `FromRow` struct — a column-alias mismatch there would only blow up at
runtime, never at compile time (non-macro `sqlx::query_as`). `test_list_pending_reviews_projects_completion_flags`
now exercises the query end to end and asserts the new flag is projected correctly.

## Verification (real output)

`nix develop -c cargo test --workspace`:

```
     Running unittests src/lib.rs (target/debug/deps/backend-2b996d5f9f09de53)
test result: ok. 288 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 2.04s
     Running unittests src/main.rs (target/debug/deps/backend-2def2fded6a9d465)
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
     Running unittests src/lib.rs (target/debug/deps/frontend-d8c9007b0bc8a695)
test result: ok. 31 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
     Running unittests src/lib.rs (target/debug/deps/shared-5ad9ea1943a34edf)
test result: ok. 47 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
   Doc-tests frontend
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.43s
```

(backend 278 → 288, shared 42 → 47, frontend 30 → 31)

`nix develop -c cargo clippy --workspace`:

```
    Checking shared v0.1.0
    Checking backend v0.1.0
    Checking frontend v0.1.0
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 8.30s
```

Clean — no new warnings. The known pre-existing `--all-targets` clippy noise was not touched.

Migration chain replay (fresh SQLite DB, all `backend/migrations/*.sql` in order):

```
--- tasks table columns ---
22|anyone_can_complete|BOOLEAN|1|0|0
23|assignee_cannot_uncomplete|BOOLEAN|1|0|0
```

No `cargo sqlx prepare` needed — non-macro `sqlx::query()` API throughout.

## Deviations from the design

1. **The shared predicate lives on `shared::Task`, not as a private fn in the tasks service.**
   The design offered this explicitly ("or the equivalent as a method on `shared::Task` if that
   reads better and is reusable by the frontend"). It *is* reusable: `is_completable_by_user()` in
   `shared` needed the identical expression, so a service-private fn would have forced a duplicate.
2. **The `−` button is gated by `disabled=` rather than hidden.** The design said "gated on
   `can_uncomplete()`". Disabling follows the existing idiom in that exact component (`+` is gated
   via `disabled=move || !can_complete`), preserves the `[-] 2/3 [+]` layout, and — with the added
   `title` tooltip — actually communicates the restriction instead of silently making the control
   vanish. The click handler also guards on `can_uncomplete`, and the backend refuses regardless, so
   this is defence in depth, not the only gate.
3. **Extra i18n key `task_card.cannot_uncomplete`** (not in the design) — needed for the tooltip in
   deviation 2. Added in de + en and covered by the key-presence test.
4. **Extra helper `TaskWithStatus::is_assignee()`** (not named in the design) — required because
   `is_user_assigned` is also `true` for unassigned tasks, so "is the assignee" is not expressible
   without it. Mirrored backend-side by the private `is_assignee(task, user_id)`.
5. **Extra test for `list_pending_reviews`** — out of the required minimum set, added because the
   change touches its untested hand-written SQL projection (rationale above).

Nothing else deviates. No blockers, no failures, nothing deferred.

## Known Stubs

None.

## Self-Check

- `backend/migrations/20240149000000_task_assignee_cannot_uncomplete.sql` — FOUND
- `.planning/quick/260726-vdz-assignee-cannot-uncomplete/260726-vdz-PLAN.md` — FOUND
- Commit `a7e0260a` — FOUND (`jj log -r @-`), contains code only; `.planning/` left uncommitted
- `nix develop -c cargo test --workspace` — PASSED (288/31/47, 0 failed)
- `nix develop -c cargo clippy --workspace` — PASSED (clean)

**Self-Check: PASSED**
