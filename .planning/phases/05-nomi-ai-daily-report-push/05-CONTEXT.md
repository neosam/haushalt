# Phase 5: Nomi.ai Daily Report Push - Context

**Gathered:** 2026-07-28
**Status:** Ready for planning
**Source:** Direct design conversation (no separate discuss-phase run — the user answered the
open questions inline; API facts were researched during the same session)

<domain>
## Phase Boundary

The household app delivers the existing daily report to a nomi.ai companion of the user's
choosing, at a time of their choosing, without anyone asking for it.

**In scope:** per-user-per-household connection settings, encrypted API key storage, a scheduled
sender riding the existing background-jobs tick, delivery to either a single Nomi or a Room,
length handling, failure handling, and last-send/last-error feedback in the UI.

**Out of scope:** any content other than the daily report; inbound access of any kind; an OAuth
flow; rate limiting beyond honouring `Retry-After`.

**Direction matters.** This is *push*: the app sends. An inbound MCP-server design was fully
planned on 2026-07-28 and discarded — it solves the opposite problem. The consequence that keeps
biting: an outgoing API key must be recoverable in plaintext to be *used*, so it is encrypted,
never hashed. Do not carry over reasoning from token-validation code.

</domain>

<decisions>
## Implementation Decisions

### Direction and scope
- **D-01:** Push only. The app initiates; nothing external queries it.
- **D-02:** Exactly one content type ships in this phase: the daily report. The user has said
  more will follow, so content, destination and schedule stay separable — but do not build
  speculative content types now.

### Settings and ownership
- **D-03:** The connection is configured **per user and per household**, not household-wide.
  Each member has their own target, their own key and their own send time for each household
  they belong to.
- **D-04:** Target, API key, send time and an on/off switch live in **one** settings section.
- **D-05:** The target may be a **single Nomi or a Room** (group chat). Both are offered for
  selection **by name**, fetched from the account via `GET /v1/nomis` and `GET /v1/rooms` — the
  user must not have to paste a UUID.
- **D-06:** Nomi and Room are one abstraction in the delivery path, not an `if` at the call site.
  Both take the identical `{"messageText": "..."}` body; they differ only in URL and in whether a
  reply comes back.

### Credential handling
- **D-07:** The API key is stored **encrypted at rest with AES-256-GCM**, using a **dedicated new
  server config value** — deliberately *not* derived from `jwt_secret`, so that rotating the JWT
  secret does not silently destroy every stored nomi key.
- **D-08:** This adds a new option to `module.nix` and a new value on the deployment host. That
  deployment consequence is accepted and must be documented in the phase output.
- **D-09:** The key is **never** returned to the client in plaintext, not even to its owner. The
  settings UI shows presence/absence, not the value.
- **D-10:** The `Authorization` header carries the **raw key with no `Bearer ` prefix** —
  `Authorization: 7c38494b-…`. This is what the official docs specify; several secondary sources
  claim Bearer-style and are wrong. Getting this wrong produces a 401 that looks like a bad key.

### Scheduling
- **D-11:** Ride the **existing** `services::background_jobs` loop, which already ticks every
  minute (`check_interval_minutes: 1`). No new scheduling machinery, no cron dependency.
- **D-12:** The send time is interpreted in the **household's timezone**, consistent with how the
  report itself resolves "today" and "yesterday".
- **D-13:** A failure for one user must **not** abort the run for other users or households.

### Content
- **D-14:** The report text comes from calling `services::report::generate_daily_report`
  **directly**. Do not route through `GET /api/households/{id}/report` — this is server-side code
  calling server-side code.
- **D-15:** The message is wrapped as an OOC aside: `(OOC: Household App (<report text>))`.
  Define the wrapper in exactly one place.
- **D-16:** When the wrapped message exceeds the length limit, **shorten the task list and append
  a counter** — in **English**, e.g. `… and 7 more`. Do not truncate blindly from the end; that
  would drop "Missed yesterday", the more interesting half.
  **Corrected 2026-07-28:** an earlier draft of this decision wrote the counter in German. That
  was wrong. `backend/src/services/report.rs:3` states the report text is *always* English by
  design, bypassing frontend i18n because a later phase feeds it to an LLM (phase 2.1, D-01).
  The counter is part of that text and follows the same rule. Only the **settings UI** strings
  are localized de/en.
- **D-17:** Treat the character limit as a **runtime constraint, not a hard-coded constant**.
  It is 800 for rooms, and 400/800 for direct chats depending on subscription; Nomi has changed
  these values before.

### Failure handling and feedback
- **D-18:** Handle the documented failure modes without crashing the run: `RoomStillCreating`,
  `InsufficientPlan`, `RoomNotFound` / `NomiNotFound`, HTTP 429 `TooManyRequests`, and — for
  direct chats only — `NoReply` (30 s) and `NomiStillResponding`.
  **Two corrections, 2026-07-28:**
  (a) An earlier draft said to honour a `Retry-After` header. The official docs do **not**
  document one — only 429 / `TooManyRequests`. Back off on 429 using our own policy and read
  `Retry-After` opportunistically if present, but never depend on it.
  (b) The length error has **different names per endpoint**: `MessageLengthLimitExceeded` for
  `/v1/nomis/:id/chat`, `MessageCharacterLimitExceeded` for `/v1/rooms/:id/chat`. A `match` on
  only one spelling falls through silently. Handle both.
- **D-19:** Record **last send time and last error** per connection and surface both in the
  settings UI. Without this a stale API key fails silently for days.

### Where the settings live
- **D-20:** The connection is configured in a **new section on `HouseholdSettingsPage`**
  (`frontend/src/pages/household_settings.rs`, route `/households/:id/settings`), labelled clearly
  as the member's *personal* settings for this household — as opposed to the household-wide
  administrative sections above it. It becomes the natural home for further per-member,
  per-household settings later.
- **D-21:** No route guard has to change. Verified 2026-07-28: the page has **no** admin guard;
  it is reachable by every member, and individual sections are gated inline with
  `<Show when=move || current_role.get().map(|r| r.can_manage_tasks())…>` (see lines 265-266,
  and the existing non-admin fallback at line 295). The new section simply carries **no** such
  `Show` wrapper, so every member sees their own.
- **D-22:** Target selection (D-05) is proxied through our backend, never called from the browser.
  The frontend must never hold or see the nomi.ai API key — it asks our backend for the list of
  Nomis and Rooms, and the backend talks to nomi.ai using the stored, decrypted key.

### Claude's Discretion
- Table and module naming, and whether the connection lives in a new table or extends an existing one
- Retry counts and backoff shape, beyond honouring `Retry-After`
- The exact HTTP client (whatever the backend already depends on, if anything suitable exists)
- How the report is shortened internally, as long as D-16's visible outcome holds
- Whether target selection is cached or fetched live when the settings screen opens

</decisions>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### The report being sent
- `backend/src/services/report.rs` — `generate_daily_report`, 1438 lines, 48 passing tests.
  The producer of the text. Already built (phase 2.1).
- `shared/src/types.rs` — `DailyReportResponse` at ~line 429.
- `.planning/phases/02.1-daily-task-report-inserted-urgent/02.1-CONTEXT.md` — the report's
  agreed text shape and empty-state wording, decisions D-01..D-27 of that phase.

### Where the sender hooks in
- `backend/src/services/background_jobs.rs` — the minute tick (`check_interval_minutes: 1`,
  the loop at ~line 76-86).
- `backend/src/services/scheduler.rs`
- `backend/src/services/household_settings.rs` — the existing per-household settings pattern,
  and the household timezone.

### Conventions to match
- `.planning/codebase/CONVENTIONS.md` and `.planning/codebase/STRUCTURE.md`
- `backend/src/services/auth.rs` — the existing credential-handling module. **Read it for style,
  not for approach**: it hashes, this phase encrypts.
- `backend/migrations/` — migration naming series
- `CLAUDE.md` — workspace denies warnings; tests mandatory; jj for commits

### Deployment
- `module.nix` — must gain an option for the new encryption secret

### External API
- `https://api.nomi.ai/docs/` — endpoints, auth format, error types. The researched facts are
  summarised in ROADMAP.md under "v1.2 Outbound Messaging"; re-verify anything load-bearing.

</canonical_refs>

<specifics>
## Specific Ideas

- The user's phrasing for the message: `(OOC: Household App (berichtdaten))` — "berichtdaten"
  being the placeholder for the report text.
- The user has a **nomi.ai subscription**, so the 800-character limit applies, and
  `InsufficientPlan` should not occur in practice — handle it anyway.
- Rooms are the more robust path for a scheduled job: `POST /v1/rooms/{id}/chat` returns only
  `sentMessage` and does not wait for a reply, so the 30-second `NoReply` window and
  `NomiStillResponding` do not arise at all. Prefer it as the reference implementation and treat
  the direct-Nomi path as the variant with extra failure modes.
- `POST /v1/rooms/{id}/chat/request` (ask a specific Nomi in the room to reply) exists and is
  explicitly **not** required here — noted only so nobody rediscovers it as a gap.

</specifics>

<deferred>
## Deferred Ideas

- Further content types — individual completions, weekly summaries. The user wants them later;
  the architecture must not block them, but none ship in this phase.
- Asking a Nomi to reply after the report is posted (`/chat/request`).
- Any inbound/read access to household data (the discarded MCP design). Recoverable via
  `jj op restore fbeab6da1e65` if it ever becomes relevant again.
- Rate limiting of outbound sends beyond honouring `Retry-After`.

</deferred>

---

*Phase: 05-nomi-ai-daily-report-push*
*Context gathered: 2026-07-28 via direct design conversation*
