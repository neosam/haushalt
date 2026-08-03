# Roadmap: Household Manager

## Overview

The v1.0 MVP is shipped (18 capability areas, full household task/habit management with gamification). The active v1.1 milestone hardens the habit tracker with comprehensive test coverage, adds read-only offline support, a daily/missed task report, and task deletion from the edit modal, with a recurrence-extension placeholder pending definition.

## Milestones

- ✅ **v1.0 MVP** - Phases shipped across 18 capabilities (shipped before GSD migration)
- 🚧 **v1.1 Hardening & Connectivity** - Phases 1-4 incl. inserted 2.1 (in progress)
- 🚧 **v1.2 Outbound Messaging** - Phase 6 (in progress); Phase 5 deferred 2026-07-31
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

#### Phase 2.1: Daily Task Report (INSERTED — urgent)

**Goal**: The logged-in user can see, per household, which tasks are due today and which they missed yesterday
**Depends on**: v1.0 MVP
**Requirements**: RPT-01, RPT-02, RPT-03, RPT-04
**Success Criteria** (what must be TRUE):

  1. A user sees the tasks due for them today in the current household
  2. A user sees the tasks they missed on the previous day
  3. Both reports are reachable from the household and show a clear empty state when nothing applies
  4. Report data comes from a backend service covered by tests

**Plans**: 1/6 plans executed

Scope confirmed in `/gsd-discuss-phase 2.1` (see `02.1-CONTEXT.md`, decisions D-01..D-27):
a single English plain-text report generated in the backend, rendered read-only at
`/households/:id/report` with a copy-to-clipboard button. "Today"/"yesterday" resolve in the
household timezone; "missed yesterday" merges `missed_task_penalties` rows with indulged bad
habits into one section. No schema change, no `?date=` parameter, no LLM integration.

Plans:

- [x] 02.1-01 (wave 0): Test-harness and web-sys build gaps — 22-column `household_settings`, `missed_task_penalties` + junction tables, `insert_missed_task_penalty` fixture, `Clipboard`/`Navigator` features
- [x] 02.1-02 (wave 1): `shared::DailyReportResponse` + report service core — timezone date resolution, "Due today" section, shared line formatter, empty states
- [x] 02.1-03 (wave 2): "Missed yesterday" section — penalized good habits plus indulged bad habits, one list, vacation-suppressed
- [x] 02.1-04 (wave 2): Thin handler `GET /api/households/{id}/report` + route registration
- [x] 02.1-05 (wave 3): Frontend — `ReportPage`, Report tab after Tasks, copy button, de/en strings, mobile-first CSS
- [ ] 02.1-06 (wave 4): Human verification checkpoint — tab placement, verbatim rendering, clipboard, empty states, XSS probe

> **Doku-Nachtrag 2026-07-28:** Die Pläne 02.1-02..05 waren bereits ausgeführt, ohne dass ROADMAP,
> STATE oder SUMMARYs nachgezogen wurden (bewusst: Code zuerst, GSD-Doku später). Nachgewiesen am
> 2026-07-28: `services/report.rs` (1438 Zeilen, `generate_daily_report`), `handlers/report.rs`,
> `frontend/src/pages/report.rs` mit registrierter Route, `shared::DailyReportResponse`, und
> `nix develop -c cargo test -p backend report` → **48 passed, 0 failed**. Es fehlen weiterhin die
> SUMMARY-Dateien für 02.1-02..05 und die menschliche Abnahme 02.1-06.

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

### 📋 v1.2 Outbound Messaging (Planned)

**Milestone Goal:** The household app delivers content of its own accord to external services the user
has connected, starting with the daily report to a nomi.ai companion.

**Scope decisions** (from the design discussion, 2026-07-28):

- **Push, not pull.** The app sends; nothing external queries it. An earlier MCP-server design
  (inbound read-only tokens) was explored and discarded on 2026-07-28 — it solves the opposite
  problem. Note the consequence for credentials: an outgoing API key must be **encrypted at rest**
  and recoverable in plaintext to be used, not hashed like an inbound token.

- **Per user, per household.** Each member configures their own connection for each household —
  target Nomi, API key, send time, on/off — in one settings section. Not a household-wide setting.

- **The report already exists.** Phase 2.1 produces the plain-text daily report; this milestone only
  transports it. Phase 2.1 must be executed first.

- **Built to extend.** Content, destination and schedule stay separable so further content types
  (individual completions, weekly summaries) can be added later without touching the delivery path.
  Only one content type ships in v1.2.

**API facts** (researched 2026-07-28, see sources in the phase RESEARCH.md):

- Two possible targets, **both using the same `{"messageText": "..."}` body**:
  - `POST /v1/nomis/{uuid}/chat` — a single Nomi. Listed via `GET /v1/nomis`.
  - `POST /v1/rooms/{uuid}/chat` — a Room (group chat). Listed via `GET /v1/rooms`.
- Auth header carries the **raw key, without a `Bearer ` prefix**: `Authorization: <uuid>`.
  Several secondary sources claim Bearer-style; the official docs do not.

- **Rooms are the better fit for a scheduled job** and should be the reference path:
  the room endpoint returns only `sentMessage` and does **not** wait for a reply, so the
  `NoReply` (30 s) and `NomiStillResponding` failure modes of the direct chat simply do not
  arise. Its errors are `RoomNotFound`, `InsufficientPlan`, `MessageCharacterLimitExceeded`,
  `RoomStillCreating`, `InvalidBody`, `InvalidContentType`.

- The direct-Nomi endpoint is **synchronous**: it waits up to 30 s for the reply, then returns
  `NoReply`. Extra failure modes there: `NomiStillResponding`, `LimitExceeded`.

- Message length: 800 for rooms; 400 free / 800 with a subscription for direct chats (the user
  has a subscription). Treat the limit as a runtime constraint, not a hard-coded constant —
  Nomi has changed it before.

- HTTP 429 with a `Retry-After` header applies to both.
- Optional, not required for this phase: `POST /v1/rooms/{id}/chat/request` with `{nomiUuid}`
  asks a specific Nomi in the room to reply. Synchronous, 15 s timeout.

#### Phase 6: Public Cross-Household Report Links

**Goal**: A user configures named, cross-household reports in their user settings and shares each one as an unauthenticated URL that returns nothing but the report text
**Depends on**: Phase 2.1 — satisfied. `services::report::generate_daily_report` is built and covered by 48 passing tests; this phase makes it language-aware and calls it once per selected household.
**Requirements**: PUBREP-01..07
**Success Criteria** (what must be TRUE):

  1. A user can create, rename, and delete several reports in their user settings
  2. Each report has an explicit household selection, restricted to households the user belongs to
  3. Opening the generated URL in a logged-out browser returns the report as `text/plain` and nothing else
  4. The output contains one daily-report block per selected household, each resolved in that household's own timezone
  5. A report renders in the language configured on that report; the existing `GET /api/households/{id}/report` stays English
  6. Switching a report off returns 404 on its URL; regenerating the token invalidates the old URL
  7. Losing membership in a household removes it from the output without breaking the rest

**Decisions** (2026-07-31, with the user):

- **D-01**: Household selection is explicit, never "all my households" — a newly joined household must not silently
  appear in an already distributed URL.
- **D-02**: One `generate_daily_report` block per household, concatenated. The existing block already carries the
  household name in its header, so no new formatting layer is needed and the per-household code path stays untouched.
- **D-03**: The public response is `text/plain; charset=utf-8`. No HTML wrapper, no JSON envelope.
- **D-04**: Each household block resolves "today"/"yesterday" in its own timezone. No new user-level timezone setting.
- **D-05**: The token is a UUID v4, regenerable, and the report has an on/off switch. No expiry date.
- **D-06**: Output language is a per-report setting (`de`/`en`). This narrows Phase 2.1's D-01 ("the report is always
  English") to the per-household endpoint only — that endpoint keeps emitting English so its 48 tests and any later
  LLM consumer are unaffected.
- **D-07**: Blocks are ordered alphabetically by household name, so the output is deterministic without storing a
  sort position.
- **D-08**: Households the owner is no longer a member of are skipped silently. `generate_daily_report` already
  returns `NotAMember`; the aggregator swallows exactly that variant and no other.
- **D-09**: The public endpoint is rate limited per token via the existing in-memory `RateLimiter` and answers with
  `X-Robots-Tag: noindex, nofollow`.
- **D-10**: A disabled report, an unknown token and a malformed token all answer `404` with the same body, so the
  endpoint leaks no information about which tokens exist.

**Plans**: executed in one bundled session on 2026-07-31 (user preference: code first, GSD docs after),
landing as two commits — backend, then frontend.

Plans:

- [x] 06-01: Migration `public_reports` + `public_report_households`, mirrored in `test_utils::create_test_schema`
- [x] 06-02: Shared contract — `PublicReport`, `CreatePublicReportRequest`, `UpdatePublicReportRequest`
- [x] 06-03: `report.rs` becomes language-aware (`ReportLanguage`, `ReportStrings`); household endpoint pinned to English
- [x] 06-04: `services/public_reports.rs` — CRUD, token regeneration, membership-validated household selection, aggregation
- [x] 06-05: `handlers/public_reports.rs` — authenticated CRUD under `/api/users/me/reports`, unauthenticated `/api/public/reports/{token}`
- [x] 06-06: Frontend — `ApiClient` methods, report section on `UserSettingsPage`, de/en strings, mobile-first CSS

**Follow-up 2026-08-01** — two changes from the acceptance pass:

- The output opens with the report's own name, underlined with `=`. Without it the text named
  the households but not which of the user's reports it was.
- `include_missed` per report decides whether "Missed yesterday" is rendered. Off means the
  header is absent rather than sitting above an empty body, and the section is not computed
  at all. Migration `20240151000000` defaults it to 1, so existing reports are unchanged.
- `generate_daily_report_localized` became `generate_daily_report_with(…, ReportOptions)`;
  `ReportOptions::default()` is where the per-household endpoint's behaviour now lives.

**Follow-up 2026-08-02** — progress counter for multi-completion tasks:

- A task whose `target_count > 1` now renders `(X/N)` in "Due today", e.g. `- Drink water (5/8)`,
  so the report shows how far along an "N times" habit is. The count is clamped to the target
  (`8/8`, never `10/8`) and reads the same in every language, so no new translations are needed.
- The counter *replaces* the `(done)`/`(erledigt)` marker for these tasks — `8/8` already says done.
  Plain single-completion tasks (`target_count = 1`) and free-form tasks (`0`) are untouched.
- No schema or query change: `is_completed_for_today` became `completions_this_period`, returning
  the raw count the report already computed. It flows through the same builder, so the public
  cross-household reports get the counter for free. Only "Due today" carries it — the "Missed
  yesterday" section is about what went wrong, not progress.

**Follow-up 2026-08-03** — optional "No fixed date" section for undated tasks:

- OneTime / free-form tasks (`recurrence_type` onetime/none) are "always due", so they showed up
  under "Due today" every day. A new per-report switch (`separate_undated`) pulls them into their
  own section, "No fixed date" / "Ohne festen Termin", between "Due today" and "Missed yesterday".
- Off by default — `ReportOptions::default()` and migration `20240153000000` (DEFAULT 0) keep the
  per-household endpoint and existing reports unchanged (undated tasks stay mixed in). The section
  renders only when the switch is on (`None` omits it, exactly like `include_missed`).
- `build_due_today_section` now returns `(due_today, undated)`; `format_report` gained the optional
  middle section. Configured via a checkbox in the report settings, alongside "Missed yesterday".

**Verification 2026-07-31** — workspace suite green (345 backend / 168 frontend / 69 shared), clippy clean in
`backend` and `shared`, and an end-to-end run against a fresh SQLite database with the server up: the migration
applies, an unauthenticated `GET` returns `200 text/plain` with one block per household in the configured
language, and switching off (404), rotating the token (old link 404), selecting a foreign household (403),
unknown and malformed tokens (404) and the rate limit (429) all behave as designed.

**Still open**: the human UI check — the settings section has not been looked at in a browser, only built.

**Incidental fix**: `test_utils::create_test_household_with_name` now generates a per-call owner email. It used to
hardcode `owner@test.com`, so a second household in one test database hit the UNIQUE constraint; `report.rs` had
worked around that with a local duplicate of the fixture, which this replaces.

#### Phase 7: API Access Tokens

**Goal**: A household member issues a token so an external system can call the API on their behalf, bound to one
household and a read-only-or-read+write permission.

> **Context**: this revives the "inbound token" idea the v1.2 discussion (2026-07-28, above) explored and discarded
> as out of scope for *outbound* messaging. The user requested it directly on 2026-08-02, broadened from read-only
> to read+write. Unlike an outbound key (encrypted at rest, recoverable), an inbound token is a **hashed** credential
> (SHA-256), shown once — the same asymmetry that note predicted.

**Built (2026-08-02)**:

- `api_tokens` table + migration `20240152000000`: one household, one creator, `can_write`, `enabled`,
  `token_hash` (SHA-256, UNIQUE), a non-secret `token_prefix`, `last_used_at`. The secret (`hht_` + two v4 UUIDs =
  244 bits) is shown once at creation and never stored in plaintext.
- **Auth via a `from_fn` middleware on the `/households` scope** (`middleware::api_token`). A request bearing
  `Authorization: Bearer hht_...` is validated, checked against the household in the URL and the read/write gate
  (anything but GET/HEAD/OPTIONS needs `can_write`), then handed to the existing handlers AS its creator by minting
  a 5-minute JWT and rewriting the header. Every existing `extract_user_id`/`is_member`/role check therefore applies
  unchanged — **no handler was touched**. JWT requests pass through untouched.
- **Key property**: a token can never exceed its creator's own role; `can_write` is only the coarse gate on top.
  That is why *any* member may create one — no privilege escalation (decided with the user, 2026-08-02).
- Owner-scoped CRUD at `/api/users/me/api-tokens` (mirrors the reports surface). Being under `/users/me`, a token
  cannot reach it, so a token can never mint further tokens.
- Frontend `ApiTokensSection` in the user settings: create (name + household + write toggle), the one-time secret
  box (copy + warning), per-token enable/write/delete. 17 `api_tokens.*` translation keys, `.api-token*` CSS
  (mobile-first), `api-token` added to the css-contract allowlist.

**Design decisions**: one household per token (the user framed it as "which household", singular); read/write is a
single flag, with the creator's role supplying the finer authorization; reachability is the whole household-scoped
API of the bound household, not a subset.

**Verification (2026-08-02)**: workspace green (backend 389 / frontend 175 native / shared 69), clippy clean, WASM
build succeeds. Middleware end-to-end tests cover read-as-creator, read-only blocked on writes, write token writes,
foreign-household 403, non-household-path 403, unknown/disabled 401, and JWT passthrough.

**Still open**: the human UI check — the settings section has not been looked at in a browser. No per-token rate
limiting in v1: revocation (disable/delete) is the mitigation for a leaked token, and can be tightened later.

#### Phase 5: Nomi.ai Daily Report Push (DEFERRED)

> **Deferred 2026-07-31** at the user's request in favour of Phase 6. The plans, context and research
> under `.planning/phases/05-nomi-ai-daily-report-push/` remain valid and untouched; nothing was executed,
> so there is no code to unwind. Pick it up again with `/gsd-execute-phase 5`.

**Goal**: A member configures a nomi.ai connection per household and receives the daily report there as an OOC message at a time of their choosing
**Depends on**: Phase 2.1 — **already satisfied.** `services::report::generate_daily_report` is built and covered by 48 passing tests; call it directly rather than going through `GET /api/households/{id}/report`.
**Requirements**: NOMI-01, NOMI-02, NOMI-03, NOMI-04, NOMI-05, NOMI-06, NOMI-07
**Success Criteria** (what must be TRUE):

  1. A member can set target, API key, send time and on/off per household; the key is stored encrypted and never returned in plaintext
  2. The target may be a single Nomi **or** a Room, both selectable by name from the account
  3. At the configured local time the report arrives in that chat as `(OOC: Household App (…))`
  4. A report longer than the limit is shortened with a counter (`… and N more`, English per D-16's 2026-07-28 correction) instead of failing
  5. `RoomStillCreating`, `NoReply`, `NomiStillResponding` and 429 are handled without aborting the run for other users
  6. The settings show when the last send happened and what the last error was
  7. Adding a second content type later requires no change to the delivery path

**Plans**: 5 plans in 5 waves (planned 2026-07-28; context in `05-CONTEXT.md` D-01..D-22,
research in `05-RESEARCH.md`, validation contract in `05-VALIDATION.md`).
The scheduler question is settled: `services::background_jobs` already ticks every minute
(`check_interval_minutes: 1`), so a minute-precise send time needs no new scheduling machinery —
but the tick drifts, so the send is a level trigger (`now_local >= send_time`) latched on
`last_attempt_date`, not an equality check. Two new dependencies are unavoidable: `aes-gcm 0.10`
and `reqwest 0.12` (rustls/webpki; `awc` is not in the tree and is `!Send`). New deployment secret:
`NOMI_ENCRYPTION_KEY` (D-08).

Plans:

- [ ] 05-01 (wave 1): Wave-0 blockers & credential foundation — the three dependencies, `Config.nomi_encryption_key` + `nomi_message_limit` with a redacting `Debug`, `services/crypto.rs` (AES-256-GCM), the `nomi_connections` migration plus its guarded test twin, and `start_scheduler(pool, JobConfig, NomiJobSettings)`
- [ ] 05-02 (wave 2): Shared contract & encrypted storage — `NomiTargetKind`/`NomiTarget`/`NomiConnection`/`UpdateNomiConnectionRequest`/`NomiTargetsResponse`, `NomiConnectionRow`, and `services/nomi_connections.rs` CRUD with the enable guard and the D-19 feedback writers
- [ ] 05-03 (wave 3): Delivery core — `services/nomi.rs` (one target abstraction, the `NomiTransport` seam, the full error taxonomy including both length-error spellings, the OOC wrapper, `is_due`) plus `generate_daily_report_capped` with the English `… and N more` counter
- [ ] 05-04 (wave 4): Scheduled sender & HTTP API — `process_nomi_sends` in the minute tick with per-connection failure isolation, `GET`/`PUT /api/households/{id}/nomi` and `GET .../nomi/targets`, and the `module.nix` key options with a list-valued `EnvironmentFile`
- [ ] 05-05 (wave 5): Frontend & acceptance — three `ApiClient` methods, 25 de/en strings, the ungated per-member section on `HouseholdSettingsPage`, and the human checkpoint (real send to a Nomi and to a Room, deployment secret, non-admin path)

### Completed Outside Phases

**Delete Task from Edit Modal** (TDEL-01..04) — done 2026-07-26 as a quick task.
Backend already provided `DELETE /households/{id}/tasks/{task_id}`; the change was
frontend-only: an opt-in `on_delete` prop on `TaskModal` with an in-modal confirmation
step, wired into `pages/tasks.rs` and `pages/household.rs` behind the existing
manage-tasks permission.

## Progress

**Execution Order:**
Phases execute in numeric order: 1 → 2.1 → 2 → 3 → 4
Phase 2.1 is an urgent insertion and runs before the remainder of Phase 2.

Phase 5 (v1.2) is the only phase with a cross-milestone dependency: it needs the report text from
Phase 2.1 and cannot start before 2.1 is executed. It is independent of phases 2, 3 and 4.

| Phase | Milestone | Plans Complete | Status | Completed |
|-------|-----------|----------------|--------|-----------|
| 1. Custom Recurrence Period Fix | v1.1 | 1/1 | Complete | 2026-07-23 |
| 2. Habit Tracker Test Coverage | v1.1 | 2/5 | In progress | - |
| 2.1 Daily Task Report (INSERTED) | v1.1 | 5/6 | Code fertig, Abnahme offen | - |
| 3. Extend Recurrence Types | v1.1 | 0/TBD | Not started | - |
| 4. Offline Task Viewing | v1.1 | 0/3 | Not started | - |
| 5. Nomi.ai Daily Report Push | v1.2 | 0/5 | Deferred (2026-07-31) | - |
| 6. Public Cross-Household Report Links | v1.2 | 0/6 | In progress | - |
