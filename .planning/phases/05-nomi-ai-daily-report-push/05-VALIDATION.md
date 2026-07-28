---
phase: 05
slug: nomi-ai-daily-report-push
status: draft
nyquist_compliant: false
wave_0_complete: false
created: 2026-07-28
---

# Phase 05 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution.

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | Rust built-in `#[test]` / `#[tokio::test]` / `#[actix_web::test]`; tests in `#[cfg(test)] mod tests` next to the code |
| **Config file** | none — workspace `Cargo.toml`; in-memory SQLite via `backend/src/test_utils.rs` |
| **Quick run command** | `nix develop -c cargo test -p backend nomi` |
| **Full suite command** | `nix develop -c cargo test --workspace` |
| **Estimated runtime** | measure on the first green run and record here |

**Toolchain note:** `cargo` is not on the bare system PATH. Every command runs through the nix
devShell (`nix develop -c …`).

**Baseline measured 2026-07-28:** backend 303 passed, frontend 166, shared 67;
`cargo clippy -p backend --all-targets` exits 0.

---

## Sampling Rate

- **After every task commit:** `nix develop -c cargo test -p backend nomi` and
  `nix develop -c cargo clippy -p backend --all-targets`
- **After every plan wave:** `nix develop -c cargo test -p backend`
- **Before `/gsd-verify-phase`:** `nix develop -c cargo test --workspace` green and
  `nix develop -c cargo check --workspace` clean (the workspace denies warnings)
- **Max feedback latency:** to be measured

**Clippy scope:** gate on `-p backend` with **no file-level exception** — the backend crate is
clean as of 2026-07-28. Frontend work gates on `cargo test -p frontend --lib`, which includes the
translation-key-presence tests and the `css_contract` test that fails the build on undefined
`form-*` / `modal-*` classes. The ~61 pre-existing frontend clippy findings stay out of scope.

---

## Per-Task Verification Map

| Task ID | Plan | Wave | Requirement | Threat Ref | Secure Behavior | Test Type | Automated Command | File Exists | Status |
|---------|------|------|-------------|------------|-----------------|-----------|-------------------|-------------|--------|
| *filled in by the planner — one row per task, covering NOMI-01..NOMI-07* | | | | | | | | | ⬜ pending |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

---

## Wave 0 Requirements

Per RESEARCH.md, two infrastructure gaps block everything else and must land first:

- [ ] `backend/src/test_utils.rs` — `run_migrations()` does **not** run `backend/migrations/`; it
      hand-builds a duplicate schema. Any new table must be written twice, and the two must be
      guarded against drift. (Evidence that this is a live risk: the test `user_settings` table
      carries three columns the real migration does not know.)
- [ ] `start_scheduler(pool: Arc<SqlitePool>, config: JobConfig)` (`background_jobs.rs:77`)
      receives **no** `Config`, so the encryption key cannot reach the job. The signature and the
      call site in `main.rs:49-55` must change before any sending task can work.
- [ ] Test seam for outbound HTTP — RESEARCH.md recommends a `NomiTransport` trait boundary over
      adding a wiremock dependency. It is also the only way to assert D-10 (raw `Authorization`
      header with no `Bearer` prefix) at all.

---

## Manual-Only Verifications

| Behavior | Requirement | Why Manual | Test Instructions |
|----------|-------------|------------|-------------------|
| A real report actually arrives in a real Nomi/Room chat | NOMI-03, NOMI-07 | Requires a live nomi.ai account, a real API key and a real target | Configure a connection, set the send time a few minutes out, wait for the tick, confirm the OOC message appears |

---

## Validation Sign-Off

- [ ] All tasks have `<automated>` verify or Wave 0 dependencies
- [ ] Sampling continuity: no 3 consecutive tasks without automated verify
- [ ] Wave 0 covers all MISSING references
- [ ] No watch-mode flags
- [ ] `nyquist_compliant: true` set in frontmatter

**Approval:** pending
