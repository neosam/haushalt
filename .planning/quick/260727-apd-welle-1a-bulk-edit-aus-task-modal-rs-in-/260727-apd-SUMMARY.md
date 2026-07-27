---
task: 260727-apd
title: Wave 1a — extract bulk edit from task_modal.rs into BulkEditModal
type: quick
status: complete
created: 2026-07-27
completed: 2026-07-27
commits: [0a6fbe10, 45098246, 7ea7397a]
tests_added: 27
tests_removed: 1
---

# 260727-apd: Bulk edit extracted into its own component — Summary

Pure refactor. The bulk-edit mode moved out of `task_modal.rs` (which carried five modes in one
component) into a new `frontend/src/components/bulk_edit_modal.rs`. The request-building logic is
now a signal-free function `build_bulk_update_request` with 27 host-runnable `#[test]`s — the first
real automated coverage this code path has ever had.

**Behaviour, UI, texts, CSS classes and DOM structure are unchanged.** Every oddity found along the
way was carried over verbatim (see below); none were "fixed" in passing.

## Commits

| Commit | Message |
| --- | --- |
| `0a6fbe10` | `refactor(frontend): extract bulk edit request builder as testable function` |
| `45098246` | `refactor(frontend): move bulk edit into its own BulkEditModal component` |
| `7ea7397a` | `refactor(frontend): drop bulk edit branch from TaskModal` |

## Numbers

| Metric | Before | After |
| --- | --- | --- |
| `task_modal.rs` | 1953 lines | **1570 lines** (−383, budget was < 1650) |
| `bulk_edit_modal.rs` | — | **897 lines** (budget was > 350) |
| `cargo test -p frontend --lib` | 31 passed | **57 passed**, 0 failed |
| `cargo clippy -p frontend` | green, exit 0 | **green, exit 0** |
| `cargo clippy -p frontend --all-targets` | 61 findings | **61 findings — no delta** |
| `grep -ci bulk task_modal.rs` | 100 | **0** |

Test arithmetic: 31 baseline − 1 removed (`delete_hidden_during_bulk_edit`, D-02) + 27 new = 57.

Clippy delta was compared against the histogram captured before any code change, using the exact
same command variant. `diff` of before/after is empty: still 51× *identical args in `assert_eq!`*,
7× *useless use of `vec!`*, 2× *`is_none()` after `position`*, 1× *assertion is always true* — all
in pre-existing `#[cfg(test)]` code, all left untouched as agreed.

## What was built

### `frontend/src/components/bulk_edit_modal.rs` (new)

- **`BulkEditForm`** — signal-free snapshot of the form (which boxes are ticked, plus the raw
  strings as typed). Exists so the request building can be tested without a browser.
  `Default` is derived for test ergonomics only; the component never uses `..Default::default()`
  because `weekday`/`month_day` would then be 0 instead of 1.
- **`build_bulk_update_request(&BulkEditForm) -> UpdateTaskRequest`** — 1:1 translation of the old
  inline logic from `task_modal.rs:513-626`. Same semantics, same order, same fallbacks.
- **`BulkEditModal`** — the component. Six props (`bulk_task_ids`, `household_id`, `members`,
  `categories`, `on_close`, `on_bulk_save`). Per D-04 it deliberately does *not* take
  `household_rewards`, `household_punishments`, `linked_rewards`, `linked_punishments`, `task`,
  `on_save`, `default_*`, `is_suggestion` or `on_delete` — the bulk branch never read them.
  Per D-03 `on_bulk_save` is a required `Callback<usize>` rather than an `Option`.
  It contains no delete path at all, which makes "no deleting during bulk edit" structural rather
  than a runtime flag.
- 27 `#[test]`s (not `#[wasm_bindgen_test]`, per D-01 — those only run in a browser and would be
  dead weight in the normal build).

### `frontend/src/pages/tasks.rs`

The bulk branch now renders `<BulkEditModal>`. The `on_bulk_save` body (close modal, leave
multi-select, clear selection, reload tasks) is unchanged line for line. The three other
`TaskModal` call sites in this file are untouched.

### `frontend/src/components/task_modal.rs`

All bulk traces removed: both props, `is_bulk_edit`/`bulk_task_count`, the 14 `apply_*` signals,
the bulk recurrence signals, the `paused` signal (verified bulk-only), progress/error state,
`on_bulk_submit`, the progress bar, the error list, the `on:submit` switch, all four `<Show>`
branches and the entire bulk view. `grep -ci bulk` is now 0.

`delete_action_available` lost its first parameter (D-02):
`fn delete_action_available(has_task: bool, has_callback: bool) -> bool`. It is trivial now, but it
still carries the documented invariant "no callback = no permission = no delete" and its three
tests are the only host-runnable tests left in the file.

## Deliberately preserved quirks

These look like bugs. They were carried over **unchanged and on purpose**, each marked with a
comment in the new code, and are pinned by tests where testable. They belong to the follow-up
cleanup round of the big task-form rework, not to this refactor.

| # | Quirk | Where it now lives |
| --- | --- | --- |
| 1 | Button reads **"Erstelle…"** (create) while a bulk save runs — the old cascade treated bulk as neither edit nor suggestion and fell through to the create branch | `saving_text` in `BulkEditModal`, commented |
| 2 | **`assigned_user_id` cannot be cleared** via bulk edit — `Some(None).flatten()` is `None`, so ticking "Assigned to" with an empty selection does nothing | `build_bulk_update_request`, test `empty_assignment_cannot_clear_the_assignee` |
| 3 | **`due_time` cannot be cleared** via bulk edit — same flattening | test `empty_due_time_cannot_clear_the_due_time` |
| 4 | **`category_id` *can* be cleared** (`Some(None)`, not flattened) — and an invalid UUID clears it too | tests `empty_category_clears_the_category`, `unparsable_category_clears_the_category` |
| 5 | **Auto-select with exactly one assignable member** — inherited from the create path, visible in the bulk dropdown | `initial_assigned_user` in `BulkEditModal`, commented |
| 6 | **`target_count` clamps to `.max(0)`**, not `.max(1)` — `"-5"` becomes `Some(0)` | test `negative_target_count_is_clamped_to_zero` |
| 7 | Bulk start values: `recurrence_type="daily"`, `target_count="1"`, `allow_exceed_target=true`, `habit_type="good"`, `bulk_selected_weekdays` **empty**, all 14 `apply_*` false | signal initialisation in `BulkEditModal` |

## Deviation from the plan

**[Rule 3 — blocking] `habit_type` had to be written differently than the plan's snippet.**

The plan's literal code used `form.apply_habit_type.then(|| match ... )`. That trips
`clippy::unnecessary_lazy_evaluations`, which the workspace's `-D warnings` turns into a hard
error — so `cargo clippy -p frontend` (the plan's own gate: green, exit 0, no `#[allow]`) failed.

Resolved by using the exact shape the original code in `task_modal.rs:611-618` had:

```rust
habit_type: if form.apply_habit_type {
    Some(match form.habit_type_raw.as_str() {
        "bad" => HabitType::Bad,
        _ => HabitType::Good,
    })
} else {
    None
},
```

Semantically identical (a pure match, no side effects, eagerly evaluated either way), closer to the
source it replaces, and already proven clippy-clean in this repo. No `#[allow]` was added and no
test was weakened. Covered by `bad_habit_type_is_recognised` and `any_other_habit_type_means_good`.

## Verification

```
cargo check --workspace                      Finished, no errors
cargo clippy -p frontend                     Finished, exit 0, no findings
cargo test -p frontend --lib                 57 passed; 0 failed
cargo clippy -p frontend --all-targets       61 findings — diff vs baseline empty
grep -ci bulk task_modal.rs                  0
grep -rn "bulk_task_ids\|on_bulk_save"       only bulk_edit_modal.rs and tasks.rs
wc -l task_modal.rs                          1570 (was 1953)
```

The six non-bulk `TaskModal` call sites (`dashboard.rs:694`, `quick_task_fab.rs:198`,
`tasks.rs:629/657/685`, `household.rs:1339`) are unchanged and compile — `cargo check --workspace`
covers this.

**Not verified:** the visual check in the browser (`trunk serve` → task list → multi-select → two
tasks → "edit selected"). Title with count, hint text, field order, progress bar and success
behaviour should look exactly as before. Non-blocking, but worth a glance before the next wave.

## Known stubs

None.
