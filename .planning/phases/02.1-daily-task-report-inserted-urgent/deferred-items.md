# Deferred Items — Phase 02.1

Out-of-scope discoveries found during plan execution. Not fixed as part of this phase per the
executor's scope-boundary rule (pre-existing, unrelated to the current task's changes).

## 1. ~~Pre-existing backend clippy failures in `backend/src/services/tasks.rs`~~ — RESOLVED

**Resolved:** 2026-07-27 in quick task `260727-t5x` (commit `4d5324f9`). That task modified
`tasks.rs` itself, so the scope-boundary argument below no longer applied. All 6 findings are gone —
2 disappeared with the tautological Weekdays/Custom tests that were replaced, the remaining 4 were
fixed mechanically. `nix develop -c cargo clippy -p backend --all-targets` now exits 0. Item 2
(frontend) is still open.

<details>
<summary>Original entry</summary>


**Found during:** 02.1-01, Task 1 verification (`nix develop -c cargo clippy -p backend --all-targets`)

**Issue:** 6 clippy errors, all inside `backend/src/services/tasks.rs`, all pre-dating this plan
(confirmed present verbatim at commit `cfaad3f`, before any 02.1-01 edits):

- `clippy::assign_op_pattern` at lines 3173, 3240, 3366, 3437 (`x = x + Duration::days(1)` instead
  of `x += Duration::days(1)`)
- `clippy::bool_assert_comparison` at lines 3555, 3556 (`assert_eq!(x, true)` /
  `assert_eq!(x, false)` instead of `assert!(x)` / `assert!(!x)`)

**Why out of scope:** `backend/src/services/tasks.rs` was not touched by any 02.1-01 task (Task 1
and Task 2 only modify `backend/src/test_utils.rs`; Task 3 only modifies root `Cargo.toml`). Per
the executor scope boundary, pre-existing issues in unrelated files are not auto-fixed. This is the
backend-crate analogue of the plan's already-documented pre-existing frontend clippy failure
(`frontend/src/components/solo_mode_banner.rs:66`, `clippy::type_complexity`) — the plan's
`<verification>` step 2 (`cargo clippy -p backend --all-targets` exits 0) cannot currently be
satisfied for reasons unrelated to this plan's changes.

**Impact on this plan:** None of the 02.1-01 code changes (`backend/src/test_utils.rs`,
`Cargo.toml`) introduce any new clippy findings — verified by grepping clippy output for
`test_utils` (zero hits) and confirming all 6 reported errors resolve to `tasks.rs` line numbers.

**Recommended follow-up:** A `/gsd-quick` task to fix the 6 lints in `tasks.rs` (mechanical,
low-risk `+=` and `assert!`/`assert!(!...)` rewrites), analogous to how `solo_mode_banner.rs` should
eventually be addressed outside this phase.

</details>

## 2. `nix develop -c cargo clippy -p frontend --all-targets` reports 61 pre-existing errors, not the documented 1

**Found during:** 02.1-01, Task 3 verification / plan-level `<verification>` step 4

**Issue:** The plan's `<known_out_of_scope_failure>` and `<verification>` step 4 state that
`cargo clippy -p frontend --all-targets` is expected to report EXACTLY ONE error
(`clippy::type_complexity` at `frontend/src/components/solo_mode_banner.rs:66`). Running that exact
command instead reports **61** pre-existing errors across ~20 files, none of them
`solo_mode_banner.rs`:

- `clippy::eq_op` ("identical args used in this `assert_eq!` macro call") — ~50 hits across
  component/page `#[cfg(test)] mod tests` blocks (e.g. `loading.rs`, `modal.rs`, `card.rs`,
  `context_menu.rs`, `login.rs`, `settings.rs`, …) — pre-existing placeholder assertions of the
  form `assert_eq!("card", "card")`.
- `clippy::useless_vec` — 7 hits (`task_fields.rs`, `calendar_picker.rs`, `quick_task_fab.rs` x2,
  `dashboard.rs`, `tasks.rs`, `punishments.rs`).
- `clippy::search_is_some` — 2 hits in `pages/tasks.rs` (`position(...).is_none()`).
- `clippy::assertions_on_constants` — 1 hit in `components/accordion.rs` (`assert!(true)`).

**Why the documented single error doesn't appear:** `solo_mode_banner.rs:66` sits inside a
`#[cfg(target_arch = "wasm32")]` block. Plain `cargo clippy -p frontend --all-targets` (no
`--target wasm32-unknown-unknown`) builds for the host target, so that block — and its
`type_complexity` finding — is compiled out entirely and cannot appear in this invocation. The
plan's documented expectation appears to have been produced by a different invocation (e.g. an
actual wasm32 target build) than the literal command in `<verification>` step 4.

**Why out of scope:** None of the 61 errors are in `Cargo.toml` or any file this plan touches
(Task 3 only edits the root `Cargo.toml` `web-sys` feature array). All 61 predate this plan —
confirmed structurally impossible to be caused by a feature-flag-only Cargo.toml change (feature
flags gate which `web-sys` API bindings are compiled in; they cannot affect `assert_eq!` literal
duplication, `vec!` usage, or `position().is_none()` patterns in unrelated component/page files).
Per the executor scope boundary, pre-existing issues in unrelated files are not auto-fixed here.

**Impact on this plan:** Zero — Task 3's own `<acceptance_criteria>` do not include this clippy
command (only `cargo check -p frontend`, which exits 0). The plan-level `<verification>` step 4
cannot be satisfied as literally written, for reasons unrelated to this plan's changes.

**Recommended follow-up:** A dedicated `/gsd-quick` or hardening-phase task to either (a) run
`cargo clippy -p frontend --all-targets --target wasm32-unknown-unknown` in CI/verification going
forward so `solo_mode_banner.rs` is the one real signal, and (b) clean up the 61 pre-existing
lints (mechanical: replace placeholder `assert_eq!(x, x)` with `assert_eq!(actual, x)` or `assert!`,
`vec![...]` with array literals, `position().is_none()` with `!...any(...)`).
