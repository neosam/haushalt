---
phase: 05
slug: nomi-ai-daily-report-push
status: planned
nyquist_compliant: true
wave_0_complete: false
created: 2026-07-28
updated: 2026-07-28
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
| **Estimated runtime** | backend `--lib` measured at 2.67 s, frontend `--lib` at 0.01 s (2026-07-28); re-measure after the phase |

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
- **Max feedback latency:** ~3 s for the backend lib suite

**Clippy scope:** gate on `-p backend` with **no file-level exception** — the backend crate is
clean as of 2026-07-28. Frontend work gates on `cargo test -p frontend --lib`, which includes the
translation-key-presence tests and the `css_contract` test that fails the build on undefined
`form-*` / `modal-*` classes. The ~61 pre-existing frontend clippy findings stay out of scope.

---

## Per-Task Verification Map

| Task ID | Plan | Wave | Requirement | Threat Ref | Secure Behavior | Test Type | Automated Command | File Exists | Status |
|---------|------|------|-------------|------------|-----------------|-----------|-------------------|-------------|--------|
| 05-01-T1 | 05-01 | 1 | NOMI-02 | T-02, T-04 | Neither secret appears in `{:?}`; a missing key never panics `from_env` | unit | `nix develop -c cargo test -p backend --lib config::` | ✅ `backend/src/config.rs` | ⬜ pending |
| 05-01-T2 | 05-01 | 1 | NOMI-02 | T-01, T-03, T-05 | Fresh nonce per encryption; wrong key fails closed; no key in any error | unit | `nix develop -c cargo test -p backend --lib crypto::` | ❌ new `backend/src/services/crypto.rs` | ⬜ pending |
| 05-01-T3 | 05-01 | 1 | NOMI-01, NOMI-02 | T-03, T-06 | The credential column is a BLOB named `api_key_encrypted`; test and real schema provably identical | unit (db) | `nix develop -c cargo test -p backend --lib test_utils::` | ❌ new migration + `test_utils.rs` edit | ⬜ pending |
| 05-01-T4 | 05-01 | 1 | NOMI-03 | T-02, T-04 | The job can reach the key; a malformed key disables rather than crashes | unit | `nix develop -c cargo test -p backend --lib background_jobs::tests::nomi_settings` | ✅ `background_jobs.rs`, `main.rs` | ⬜ pending |
| 05-02-T1 | 05-02 | 2 | NOMI-01, NOMI-02, NOMI-07 | T-08 | The read DTO structurally cannot carry the key | unit | `nix develop -c cargo test -p shared --lib` | ✅ `shared/src/types.rs` | ⬜ pending |
| 05-02-T2 | 05-02 | 2 | NOMI-01 | T-08, T-09 | The row is not serializable and its `Debug` hides the ciphertext | unit | `nix develop -c cargo test -p backend --lib models::nomi_connection` | ❌ new `backend/src/models/nomi_connection.rs` | ⬜ pending |
| 05-02-T3 | 05-02 | 2 | NOMI-01, NOMI-02 | T-10, T-11, T-12, T-13 | Per-(household,user) isolation; ciphertext in the column; fresh nonce on update; error text truncated in chars | unit (db) | `nix develop -c cargo test -p backend --lib nomi_connections::` | ❌ new `backend/src/services/nomi_connections.rs` | ⬜ pending |
| 05-03-T1 | 05-03 | 3 | NOMI-03, NOMI-05, NOMI-07 | T-13, T-14, T-15 | Host is a const HTTPS literal; no error carries the key; `is_due` latches per local day | unit (pure) | `nix develop -c cargo test -p backend --lib nomi::tests` | ❌ new `backend/src/services/nomi.rs` | ⬜ pending |
| 05-03-T2 | 05-03 | 3 | NOMI-03, NOMI-06, NOMI-07 | T-16, T-18 | Raw `Authorization` header asserted on a recorded request; 25 s timeout | unit (fake transport) | `nix develop -c cargo test -p backend --lib nomi::` | ❌ same file | ⬜ pending |
| 05-03-T3 | 05-03 | 3 | NOMI-04 | T-17 | Characters not bytes; never exceeds the limit; degenerate header cannot panic | unit (pure) | `nix develop -c cargo test -p backend --lib report::` | ✅ `backend/src/services/report.rs` | ⬜ pending |
| 05-04-T1 | 05-04 | 4 | NOMI-03, NOMI-05 | T-20, T-22, T-24, T-25, T-27 | One failure does not abort the run; a removed member is disabled; `last_error` never holds the key | unit (db + fake transport) | `nix develop -c cargo test -p backend --lib background_jobs::` | ✅ `background_jobs.rs` | ⬜ pending |
| 05-04-T2 | 05-04 | 4 | NOMI-01, NOMI-02, NOMI-07 | T-23, T-28 | User identity comes only from the JWT; membership checked before every nomi call | unit (pure mappers) | `nix develop -c cargo test -p backend --lib handlers::nomi` | ❌ new `backend/src/handlers/nomi.rs` | ⬜ pending |
| 05-04-T3 | 05-04 | 4 | NOMI-02 | T-21, T-26 | Both secret files coexist; the plain-string option carries the Nix-store warning | eval | `nix develop -c nix eval --file ./module.nix --apply builtins.isFunction` | ✅ `module.nix` | ⬜ pending |
| 05-05-T1 | 05-05 | 5 | NOMI-01 | T-32 | Every label exists in de and en; both JSON files still parse | unit | `nix develop -c cargo test -p frontend --lib i18n::` | ✅ `frontend/src/api/mod.rs`, `translations/*.json`, `i18n/mod.rs` | ⬜ pending |
| 05-05-T2 | 05-05 | 5 | NOMI-01, NOMI-07 | T-29, T-31, T-33 | The key field is never populated from the server; no `inner_html`; no undefined `form-*` class | unit | `nix develop -c cargo test -p frontend --lib` | ✅ `frontend/src/pages/household_settings.rs` | ⬜ pending |
| 05-05-T3 | 05-05 | 5 | NOMI-03, NOMI-07 | T-29, T-21 | A real send reaches a real chat; the deployment secret arrives without dropping the JWT secret | **manual** | — (checkpoint, see § Manual-Only Verifications) | n/a | ⬜ pending |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

**Nyquist note:** every task except 05-05-T3 carries an `<automated>` command. 05-05-T3 is the
phase's human checkpoint and is manual by nature — it verifies the live third-party service and the
deployment host, neither of which is reachable from the test suite. No three consecutive tasks lack
an automated verify.

---

## Wave 0 Requirements

Per RESEARCH.md, two infrastructure gaps block everything else and must land first. **All three are
closed by plan 05-01 (wave 1), which nothing else depends on being skipped:**

- [ ] `backend/src/test_utils.rs` — `run_migrations()` does **not** run `backend/migrations/`; it
      hand-builds a duplicate schema. → 05-01 Task 3 adds the table twice **and** adds a
      `PRAGMA table_info` / `PRAGMA index_list` drift guard. (Evidence that this is a live risk: the
      test `user_settings` table carries three columns the real migration does not have.)
- [ ] `start_scheduler(pool: Arc<SqlitePool>, config: JobConfig)` (`background_jobs.rs:77`)
      receives **no** `Config`, so the encryption key cannot reach the job. → 05-01 Task 4 changes the
      signature to take `NomiJobSettings` and rewires `main.rs:49-55`.
- [ ] Test seam for outbound HTTP — a `NomiTransport` trait boundary rather than a wiremock
      dependency. → 05-03 Task 1 defines the trait and the recording `FakeTransport`; it is the only
      way to assert D-10 (raw `Authorization` header, no `Bearer` prefix) at all.

---

## Manual-Only Verifications

| Behavior | Requirement | Why Manual | Test Instructions |
|----------|-------------|------------|-------------------|
| A real report arrives in a real Nomi chat and a real Room chat | NOMI-03, NOMI-07 | Requires a live nomi.ai account, a real API key and a real target. Only a live call can confirm that the raw `Authorization` header is accepted and that 800 is still the current limit. | 05-05 Task 3, steps 1-7 |
| An induced failure surfaces as `last_error` without echoing the key | NOMI-05, NOMI-02 | The end-to-end path through a real 4xx body cannot be reproduced by the fake transport | 05-05 Task 3, step 8 |
| The NixOS unit receives both the JWT secret file and the nomi key file | NOMI-02 | Only reproducible on the deployment host; systemd `EnvironmentFile` semantics and `ProtectSystem = "strict"` are not testable from the crate | 05-05 Task 3, step 10 |
| A non-admin member can configure their own connection | NOMI-01 | The role gating is markup-level; the repo has no rendering test harness | 05-05 Task 3, step 2 |
| No untranslated `nomi.*` key is visible in the German UI | NOMI-01 | The key-presence test proves the strings exist, not that the right key is referenced in the markup | 05-05 Task 3, step 11 |

---

## Validation Sign-Off

- [x] All tasks have `<automated>` verify or are the declared manual checkpoint (05-05-T3)
- [x] Sampling continuity: no 3 consecutive tasks without automated verify
- [x] Wave 0 covers all MISSING references (test schema, scheduler signature, transport seam)
- [x] No watch-mode flags
- [x] `nyquist_compliant: true` set in frontmatter

**Approval:** planned 2026-07-28 — re-check after execution and flip the per-task Status column.
