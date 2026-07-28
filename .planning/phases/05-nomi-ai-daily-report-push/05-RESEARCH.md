# Phase 5: Nomi.ai Daily Report Push — Research

**Researched:** 2026-07-28
**Domain:** Encryption at rest (AES-256-GCM), outbound HTTPS from an actix-web backend, scheduled per-user delivery on an existing minute tick, plain-text length shaping
**Confidence:** HIGH for everything read out of this repository and out of `api.nomi.ai/docs`; MEDIUM for the dependency recommendations (verified against crates.io and the lockfile, but not yet compiled in this workspace)

---

<user_constraints>

## User Constraints (from CONTEXT.md)

### Locked Decisions

**Direction and scope**

- **D-01:** Push only. The app initiates; nothing external queries it.
- **D-02:** Exactly one content type ships in this phase: the daily report. The user has said
  more will follow, so content, destination and schedule stay separable — but do not build
  speculative content types now.

**Settings and ownership**

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

**Credential handling**

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

**Scheduling**

- **D-11:** Ride the **existing** `services::background_jobs` loop, which already ticks every
  minute (`check_interval_minutes: 1`). No new scheduling machinery, no cron dependency.
- **D-12:** The send time is interpreted in the **household's timezone**, consistent with how the
  report itself resolves "today" and "yesterday".
- **D-13:** A failure for one user must **not** abort the run for other users or households.

**Content**

- **D-14:** The report text comes from calling `services::report::generate_daily_report`
  **directly**. Do not route through `GET /api/households/{id}/report` — this is server-side code
  calling server-side code.
- **D-15:** The message is wrapped as an OOC aside: `(OOC: Household App (<report text>))`.
  Define the wrapper in exactly one place.
- **D-16:** When the wrapped message exceeds the length limit, **shorten the task list and append
  a counter** (`… und N weitere`). Do not truncate blindly from the end — that would drop
  "Missed yesterday", which is the more interesting half.
- **D-17:** Treat the character limit as a **runtime constraint, not a hard-coded constant**.
  It is 800 for rooms, and 400/800 for direct chats depending on subscription; Nomi has changed
  these values before.

**Failure handling and feedback**

- **D-18:** Handle the documented failure modes without crashing the run: `RoomStillCreating`,
  `InsufficientPlan`, `MessageCharacterLimitExceeded`, `RoomNotFound` / `NomiNotFound`, and —
  for direct chats only — `NoReply` (30 s) and `NomiStillResponding`. Honour HTTP 429's
  `Retry-After`.
- **D-19:** Record **last send time and last error** per connection and surface both in the
  settings UI. Without this a stale API key fails silently for days.

### Claude's Discretion

- Table and module naming, and whether the connection lives in a new table or extends an existing one
- Retry counts and backoff shape, beyond honouring `Retry-After`
- The exact HTTP client (whatever the backend already depends on, if anything suitable exists)
- How the report is shortened internally, as long as D-16's visible outcome holds
- Whether target selection is cached or fetched live when the settings screen opens

### Deferred Ideas (OUT OF SCOPE)

- Further content types — individual completions, weekly summaries. The user wants them later;
  the architecture must not block them, but none ship in this phase.
- Asking a Nomi to reply after the report is posted (`/chat/request`).
- Any inbound/read access to household data (the discarded MCP design). Recoverable via
  `jj op restore fbeab6da1e65` if it ever becomes relevant again.
- Rate limiting of outbound sends beyond honouring `Retry-After`.

</user_constraints>

---

<phase_requirements>

## Phase Requirements

| ID          | Description                                                                                                                                                  | Research Support                                                                                                                            |
| ----------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------ | ------------------------------------------------------------------------------------------------------------------------------------------- |
| **NOMI-01** | A user configures, per household, their own nomi.ai connection — target, API key, send time and an on/off switch — in one settings section                    | § Per-user-per-household storage; § Frontend settings surface. Concrete schema, migration number, `test_utils` duplication warning, UI slot. |
| **NOMI-02** | The API key is stored encrypted at rest and is never returned to the client in plaintext                                                                     | § Encryption at rest. Crate choice, key material path from `module.nix` → env → `Config`, nonce handling, column format, DTO shape.          |
| **NOMI-03** | At the configured local time (household timezone) the daily report is delivered to the configured Nomi as an OOC message                                     | § Where the scheduled sender hooks in. Tick drift, idempotence pattern copied from `missed_task_penalties`, timezone helpers.                |
| **NOMI-04** | A report exceeding the nomi.ai message length limit is shortened rather than failing, and the truncation is visible in the message                            | § Length handling. Where the shortener lives, chars-vs-bytes, the 23-char OOC overhead, degenerate cases. **Contradiction C-1 flagged.**     |
| **NOMI-05** | Delivery survives the documented API failure modes — `NomiStillResponding`, `NoReply`, HTTP 429 with `Retry-After` — without aborting the run for other users | § Nomi.ai API facts (verified error taxonomy); § Failure isolation. **Contradictions C-2 and C-3 flagged.**                                  |
| **NOMI-06** | Content, destination and schedule are separable, so a further content type can be added later without changing the delivery path                              | § Architecture Patterns, Pattern 2 (transport seam) and Pattern 3 (content producer seam).                                                   |
| **NOMI-07** | The target may be either a single Nomi or a Room; both offered by name, one abstraction in the delivery path                                                  | § Nomi.ai API facts (list response shapes); § Architecture Patterns, Pattern 1 (`NomiTarget`).                                               |

</phase_requirements>

---

## Summary

Almost everything this phase needs is already in the repository — except the two things it needs
most. The report producer, the minute tick, the household timezone resolution, the per-day
idempotence pattern, the settings-page scaffolding and the de/en translation mechanism are all
present and battle-tested. What is genuinely absent is (a) any symmetric-encryption facility and
(b) any outbound HTTP client. Both require new direct dependencies; neither has a "free" option
hiding in the tree, and the briefing's hypothesis that `awc` might come along with actix-web is
**wrong** — `awc` does not appear in `Cargo.lock` at all, and `awc::Client` is `!Send`, which
would not compile inside the `tokio::spawn`ed background task anyway.

The lowest-risk dependency set is `aes-gcm = "0.10"` plus
`reqwest = { version = "0.12", default-features = false, features = ["json", "rustls-tls"] }`.
`aes-gcm 0.10.3` slots into the RustCrypto generation already in the lock (`generic-array 0.14.7`,
`crypto-common 0.1.7`, `subtle 2.6.1`, `zeroize 1.8.2`) and pairs with the `rand_core::OsRng` that
`services/auth.rs:57` already uses. `reqwest 0.12`'s `rustls-tls` feature resolves to
webpki-roots + the **`ring 0.17.14` that jsonwebtoken already pulls in**, so there is no OpenSSL,
no `aws-lc-rs`, no `cmake`, and no dependency on the host trust store — which matters because the
systemd unit runs with `ProtectSystem = "strict"`. Both Nix derivations consume `./Cargo.lock`
directly (`default.nix:27-29`, `flake.nix:40-42`), so a new dependency needs no hash update.

Three planning decisions carry more risk than the dependencies. First, `start_scheduler` at
`background_jobs.rs:77` takes only an `Arc<SqlitePool>` — it has no access to `Config` and
therefore no access to the encryption key, so its signature and the `main.rs:49-55` call site
must change. Second, `test_utils::run_migrations` does **not** run the real migrations; it
executes a hand-maintained duplicate schema (`test_utils.rs:26-30, 32+`), so every new table must
be written twice or every test touching it fails. Third, the report text is English-only by
explicit design (`report.rs:3-5`), which puts D-16's German `… und N weitere` in direct conflict
with phase 2.1's D-01 — flagged below as **C-1**, not resolved here.

**Primary recommendation:** add `aes-gcm 0.10` + `reqwest 0.12` (rustls/webpki), put the
connection in a new `nomi_connections` table keyed `PRIMARY KEY (household_id, user_id)`, drive
the send off a `last_attempt_date` column using the exact "has this subject been processed for
this local date" pattern that `missed_task_penalties` already uses, and split the sender into
pure functions (OOC wrap, shorten, due-check, error classify, target URL) behind one thin
`NomiTransport` trait so that everything except the literal reqwest call is unit-testable without
a network.

---

## Project Constraints (from CLAUDE.md)

Directives extracted from `./CLAUDE.md`. The planner must not produce a plan that violates any of
these.

| Directive                                                                         | Source                | Consequence for this phase                                                                                                        |
| --------------------------------------------------------------------------------- | --------------------- | --------------------------------------------------------------------------------------------------------------------------------- |
| "Project must build without warnings (workspace denies warnings)"                 | CLAUDE.md § Code Quality | `Cargo.toml:52-53` sets `[workspace.lints.rust] warnings = "deny"`. An unused import in a new module is a build failure, not a nit. |
| "No clippy warnings allowed"                                                      | CLAUDE.md § Code Quality | Enforceable only for `-p backend` and `-p shared` today; see § Environment Availability for the pre-existing frontend debt.        |
| "Always include tests for changes"                                                | CLAUDE.md + global     | Every task in the plan needs a test. See § Validation Architecture.                                                               |
| "Always use jujutsu vcs to create commits… `jj commit -m`"                        | CLAUDE.md             | No `git commit`.                                                                                                                  |
| Tests live in `#[cfg(test)] mod tests` next to the code                            | CONVENTIONS.md:105    | No `tests/` directory. `#[tokio::test]` is the async attribute (verified: 164 uses across `backend/src`).                          |
| In-memory SQLite per test; pass `current_date`/time explicitly, no global mocking | CONVENTIONS.md:89,107 | The sender must take `now_utc: DateTime<Utc>` as a parameter, exactly like `generate_daily_report` does.                           |
| DRY — "when you see similar code in multiple places, consider refactoring"        | CLAUDE.md § DRY       | D-15's OOC wrapper and D-17's limit must each exist in exactly one place.                                                         |
| Mobile First CSS (`min-width` media queries)                                      | CLAUDE.md § Mobile First | New settings markup writes mobile base styles first.                                                                              |
| Design-Driven Implementation: "Stop on contradictions" — ask, do not assume       | CLAUDE.md § DDI       | The three contradictions below must be escalated, not silently resolved by the executor.                                          |
| Shared API types live in `shared/src/types.rs` and are used identically both sides | CONVENTIONS.md:101    | `NomiConnection`, `UpdateNomiConnectionRequest`, `NomiTarget` go in `shared`.                                                     |

**Stale documentation — do not trust:**

- CLAUDE.md:44 and STRUCTURE.md:49 describe an `AuthenticatedUser` extractor. It does not exist.
  The real entry point is the free function `middleware::auth::extract_user_id(&HttpRequest, &str) -> Result<Uuid, AuthMiddlewareError>` (`backend/src/middleware/auth.rs:20`), called as
  `crate::middleware::auth::extract_user_id(&req, &state.config.jwt_secret)` (`handlers/report.rs:23`).
- CLAUDE.md:49,66 and STRUCTURE.md:92 mention `SQLX_OFFLINE`, `sqlx-data.json` and
  `cargo sqlx prepare`. Verified false: `grep -rn "query!\|query_as!\|query_scalar!" backend/src`
  returns nothing, and neither `backend/sqlx-data.json` nor `.sqlx/` exists. All queries are
  runtime-checked `sqlx::query`/`query_as`/`query_scalar`. **No `cargo sqlx prepare` step is
  needed after adding the migration.**
- STRUCTURE.md:56 lists `frontend/src/translations/` as JSON — that part is correct (see C-5).

---

## Nomi.ai API Facts (re-verified 2026-07-28 against the official docs)

The briefing said not to redo this. These are the spot-checks of the load-bearing claims, plus
three corrections.

| Fact                     | Verified value                                                                                                                                    | Source                                            |
| ------------------------ | ------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------- |
| Auth header              | `Authorization: xxxxxxxx-xxxx-xxxx-xxxx-xxxxxxxxxxxx` — **raw key, no `Bearer`**. Confirms D-10.                                                    | `api.nomi.ai/docs/` § Authorization               |
| Content-Type             | **Mandatory**: "it is also necessary to explicitly specify the `Content-Type` header as `application/json` when sending a request with a body"      | `api.nomi.ai/docs/reference/general`              |
| Base URL / versioning    | `https://api.nomi.ai/v1`                                                                                                                          | `…/reference/general` § Versioning                |
| Error envelope           | `{ "error": { "type": "SomeErrorNameHere" } }` — always present, may carry extra context fields                                                   | `…/reference/general` § Error Response Format     |
| Room send                | `POST https://api.nomi.ai/v1/rooms/:id/chat`, body `{ messageText: string }` — **800 characters max**; response `{ sentMessage: {uuid, text, sent} }` (no reply) | `…/reference/post-v1-rooms-id-chat/`              |
| Room errors              | `RoomNotFound`, `InvalidRouteParams`, `InvalidContentType`, `InsufficientPlan`, `MessageCharacterLimitExceeded`, `RoomStillCreating`, `InvalidBody` | `…/reference/post-v1-rooms-id-chat/`              |
| Nomi send                | `POST https://api.nomi.ai/v1/nomis/:id/chat`, body `{ messageText: string }` — "Maximum message length is **400 for free accounts and 800 for users with a subscription**"; response `{ sentMessage, replyMessage }` | `…/reference/post-v1-nomis-id-chat/`              |
| Nomi errors              | `NomiNotFound`, `InvalidRouteParams`, `InvalidContentType`, `NoReply` (30 s), `NomiStillResponding`, `NomiNotReady`, `OngoingVoiceCallDetected`, **`MessageLengthLimitExceeded`**, `LimitExceeded`, `InvalidBody` | `…/reference/post-v1-nomis-id-chat/`              |
| List nomis               | `GET /v1/nomis` → `{ nomis: [{ uuid, gender, name, created, relationshipType }] }`                                                                 | `api.nomi.ai/docs/`, `…/reference/get-v1-nomis/`  |
| List rooms               | `GET /v1/rooms` → `{ rooms: [{ uuid, name, created, updated, status, backchannelingEnabled, nomis: [{uuid, gender, name, created, relationshipType}], note }] }` where `status ∈ "Creating" \| "Default" \| "Waiting" \| "Typing" \| "Error" \| "InitialNoteError" \| "Manual" \| "TranscriptionError"` | `…/reference/get-v1-rooms/`                       |
| Rate limit               | `429 Too Many Requests`, body `{ "error": { "type": "TooManyRequests" } }`. "Repeatedly hitting the rate limit may result in API access being revoked." | `…/reference/general` § Rate Limits               |
| Missing-key / bad-key    | `401` + `{"error":{"type":"Unauthorized"}}`; malformed (non-UUID) key → `400` + `{"error":{"type":"InvalidAPIKey"}}`                               | `api.nomi.ai/docs/`                               |

**Corrections to what ROADMAP.md / CONTEXT.md record:**

1. **The direct-Nomi length error is named `MessageLengthLimitExceeded`, not
   `MessageCharacterLimitExceeded`.** Only the Room endpoint uses
   `MessageCharacterLimitExceeded`. D-18 lists just the one name. A `match` on the string will
   silently fall through for direct chats if only one spelling is handled. → **C-2**.
2. **The docs do not document a `Retry-After` header.** The general rate-limit section documents
   only the 429 status and the `TooManyRequests` body. D-18 and NOMI-05 require honouring
   `Retry-After`. → **C-3**.
3. Two failure modes are documented for direct chats that D-18 omits: **`NomiNotReady`** (the
   direct-chat analogue of `RoomStillCreating`) and **`OngoingVoiceCallDetected`**. Both are
   transient-ish and cheap to include in the same non-fatal bucket.
4. A 2024 tweet (and search snippets derived from it) claims the subscriber limit is **600**. The
   current official reference says **800** for both paths. ROADMAP.md's 800 is correct; the 600
   figure is stale. This is precisely why D-17 says the limit is a runtime constraint.

**Confidence:** HIGH — all from `api.nomi.ai/docs`, fetched 2026-07-28.

---

## Standard Stack

### Core (new direct dependencies)

| Library     | Version                                                          | Purpose                              | Why standard                                                                                                                                                                                                                                                    |
| ----------- | ---------------------------------------------------------------- | ------------------------------------ | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `aes-gcm`   | `0.10` (0.10.3)                                                  | AES-256-GCM for the stored API key   | RustCrypto's reference AEAD. Same family as the `argon2 0.5` / `sha2 0.10` already used in `services/auth.rs`. Its transitive `aead 0.5`/`cipher 0.4` generation reuses `generic-array 0.14.7`, `crypto-common 0.1.7`, `subtle 2.6.1`, `zeroize 1.8.2` from the lock. |
| `reqwest`   | `0.12`, `default-features = false`, `features = ["json", "rustls-tls"]` | Outbound HTTPS to `api.nomi.ai`      | The de-facto async HTTP client. `rustls-tls` → `rustls-tls-webpki-roots` + `__rustls-ring`, i.e. **bundled Mozilla roots and the `ring 0.17.14` already in the lock**. No OpenSSL, no `aws-lc-rs`, no host trust store, no new native build inputs.                    |
| `base64`    | `0.22`                                                           | Decode the 32-byte key from the env var | Already at `0.22.1` in `Cargo.lock` (transitively via `actix-http`, `jsonwebtoken`, `sqlx-core`). Promoting it to a direct backend dep costs zero additional compilation. Matches `module.nix:144`'s existing `openssl rand -base64` idiom.                        |

**Installation:**

```toml
# Cargo.toml  [workspace.dependencies]
aes-gcm = "0.10"
reqwest = { version = "0.12", default-features = false, features = ["json", "rustls-tls"] }
base64  = "0.22"
```

```toml
# backend/Cargo.toml  [dependencies]
aes-gcm = { workspace = true }
reqwest = { workspace = true }
base64  = { workspace = true }
```

Nothing changes in `frontend/Cargo.toml` — none of these are WASM-compatible and none are needed
there.

**Version verification** (crates.io, 2026-07-28):

| Package   | Latest stable   | Published    | Recommended here | Why not latest                                                                                                                                                                                      |
| --------- | --------------- | ------------ | ---------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `aes-gcm` | `0.11.0`        | 2026-06-28   | `0.10.3`         | 0.11 is one month old, `edition = "2024"`, MSRV 1.85, and a breaking API rewrite (new `Generate` trait; `Key::<Aes256Gcm>::generate()` / `Nonce::generate()`). It also moves to `crypto-common 0.2`, adding a *second* copy of the crypto base crates alongside the `0.1.7` the workspace already has. 0.10.3 is the long-stable line. |
| `reqwest` | `0.13.4`        | 2026-05-25   | `0.12.28`        | In 0.13 `default-tls = ['rustls']` and `rustls` now means `aws-lc-rs` + `rustls-platform-verifier` — a C/assembly build and a host-trust-store read. `default.nix:11-12` provides only `curl pkg-config openssl sqlite`, and the unit runs `ProtectSystem = "strict"`. 0.12's `rustls-tls` reuses the `ring` already in the tree. |
| `base64`  | `0.22.x`        | —            | `0.22`           | Already the locked version; no delta.                                                                                                                                                               |

The toolchain is not a blocker either way: `nix develop -c cargo --version` → **cargo 1.93.1
(083ac5135 2025-12-15)**, comfortably above both crates' MSRV of 1.85.

### Supporting (optional, dev only)

| Library       | Version  | Purpose                                | When to use                                                                                                                                                       |
| ------------- | -------- | -------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| `async-trait` | `0.1.89` | `dyn`-safe async trait for the transport seam | Only if the plan wants `Box<dyn NomiTransport>`. Already in `Cargo.lock` (via `wasm-bindgen-test`), so it is a proc-macro that is already fetched. A generic `<T: NomiTransport>` bound avoids it entirely — prefer that. |
| `wiremock`    | `0.6.5`  | Local mock HTTP server for integration tests | Only if the plan wants to exercise the real reqwest code path. **Not recommended** — the trait seam covers the same behaviour with zero extra crates. See § Validation Architecture. |

### Alternatives Considered

| Instead of                | Could use                          | Tradeoff                                                                                                                                                                                                                                             |
| ------------------------- | ---------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| `aes-gcm`                 | `ring = "0.17"`                    | **Genuinely zero new crates** — `ring 0.17.14` is already in `Cargo.lock:2561` as a transitive dep of `jsonwebtoken` (a *direct* backend dep), so it is already compiled for this target. Downside: `ring::aead` is a lower-level, easier-to-misuse API (`UnboundKey`/`LessSafeKey`, `Nonce::assume_unique_for_key`, in-place `seal_in_place_append_tag` on a `Vec`). If build size or build time is the deciding factor, take `ring`. Otherwise `aes-gcm` is clearer and matches the RustCrypto style already in `services/auth.rs`. |
| `aes-gcm`                 | `chacha20poly1305`                 | Equally fine cryptographically and often faster without AES-NI. But D-07 says AES-256-GCM explicitly. Do not substitute.                                                                                                                              |
| `reqwest`                 | `awc = "3.8"`                      | **Disqualified twice over.** (1) It is *not* in `Cargo.lock` — actix-web does not pull it; it would be just as new a dependency as reqwest. (2) `awc::Client` is `impl !Send` and `impl !Sync` (docs.rs/awc/3.8.2). `main.rs:49` uses `tokio::spawn`, which requires `Send + 'static`, so an `awc::Client` in the background job **would not compile**. |
| `reqwest`                 | `ureq = "3.3"`                     | Blocking, tiny, rustls-based. Would need `tokio::task::spawn_blocking` around every call to avoid stalling the runtime — for a once-a-day send that is defensible, but it adds a foot-gun for no gain over reqwest.                                    |
| `reqwest`                 | hand-rolled over `tokio` + `rustls` | Do not. TLS, redirects, chunked bodies, timeouts, connection reuse.                                                                                                                                                                                   |
| `reqwest` `rustls-tls`    | `reqwest` `native-tls`             | Would work — `default.nix:11-12` already supplies `pkg-config` + `openssl`. But it makes the binary depend on the host's OpenSSL and its cert store at runtime, under `ProtectSystem = "strict"`. webpki-roots has no filesystem dependency at all. Prefer rustls. |

---

## Architecture Patterns

### Recommended module layout

```
backend/src/
├── services/
│   ├── crypto.rs             # NEW — AES-256-GCM seal/open. No nomi knowledge.
│   ├── nomi.rs               # NEW — NomiTarget, NomiError, NomiTransport trait,
│   │                         #       ReqwestTransport, OOC wrapper, shortener, due-check
│   ├── nomi_connections.rs   # NEW — CRUD over the nomi_connections table
│   ├── background_jobs.rs    # EDIT — new process_nomi_sends() + call in the loop
│   ├── report.rs             # EDIT — new pub fn that emits a length-capped report
│   └── mod.rs                # EDIT — register the three new modules
├── handlers/
│   └── nomi.rs               # NEW — GET/PUT connection, GET targets
├── models/
│   └── nomi_connection.rs    # NEW — NomiConnectionRow + to_shared()
├── config.rs                 # EDIT — nomi_encryption_key: Option<String>
├── test_utils.rs             # EDIT — new table in create_test_schema + fixture
└── migrations/
    └── 20240150000000_nomi_connections.sql   # NEW
shared/src/types.rs           # EDIT — NomiConnection, UpdateNomiConnectionRequest,
                              #        NomiTarget, NomiTargetKind, NomiTargetsResponse
frontend/src/
├── pages/household_settings.rs   # EDIT — one new Card section
├── api/mod.rs                    # EDIT — three ApiClient methods
├── translations/{en,de}.json     # EDIT — new nomi.* keys
├── i18n/mod.rs                   # EDIT — key-presence test
└── styles.css                    # EDIT only if new form-*/modal-* classes appear
module.nix                        # EDIT — nomiEncryptionKeyFile / nomiEncryptionKey option
```

### Pattern 1: One target abstraction, not a branch at the call site (D-06, NOMI-07)

The two endpoints differ in exactly two ways: the URL path segment, and whether the response
carries a `replyMessage`. Model that as data, not control flow.

```rust
// shared/src/types.rs
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum NomiTargetKind { Nomi, Room }

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NomiTarget {
    pub kind: NomiTargetKind,
    pub uuid: Uuid,
    pub name: String,   // for display only; the uuid is what is stored and sent
}
```

```rust
// backend/src/services/nomi.rs
const API_BASE: &str = "https://api.nomi.ai/v1";

/// The ONLY place that knows the two URL shapes differ.
pub fn chat_url(kind: NomiTargetKind, uuid: &Uuid) -> String {
    match kind {
        NomiTargetKind::Nomi => format!("{API_BASE}/nomis/{uuid}/chat"),
        NomiTargetKind::Room => format!("{API_BASE}/rooms/{uuid}/chat"),
    }
}
```

Everything downstream takes a `&str` URL. The body is identical (`{"messageText": ...}`) for both,
per the official docs, so there is no second branch.

The `replyMessage` asymmetry is handled by *not deserialising it*: define the success response as
`struct ChatResponse { sent_message: SentMessage }` with `#[serde(rename_all = "camelCase")]` and
let serde ignore the extra field. Both endpoints then parse with one type.

**Anti-pattern:** `if target.is_room() { post_room(...) } else { post_nomi(...) }` with two
near-identical functions. D-06 forbids it and it doubles the error-handling surface.

### Pattern 2: One transport seam (NOMI-05, NOMI-06, testability)

Put exactly one trait between the delivery logic and reqwest. Everything above the seam becomes
unit-testable with no network; everything below it is ~20 lines that need manual verification once.

```rust
// backend/src/services/nomi.rs
pub struct RawResponse {
    pub status: u16,
    pub retry_after: Option<u64>,   // seconds, parsed from the header IF present (see C-3)
    pub body: String,
}

/// The seam. One method. Nothing nomi-specific above the wire level.
pub trait NomiTransport {
    fn post_json(
        &self,
        url: &str,
        api_key: &str,
        body: &str,
    ) -> impl std::future::Future<Output = Result<RawResponse, TransportError>> + Send;

    fn get_json(
        &self,
        url: &str,
        api_key: &str,
    ) -> impl std::future::Future<Output = Result<RawResponse, TransportError>> + Send;
}
```

Use a generic bound (`async fn send<T: NomiTransport>(t: &T, ...)`) rather than
`Box<dyn NomiTransport>` — native `async fn` in traits is stable and generic dispatch avoids both
`async-trait` and the `async_fn_in_trait` lint that fires on public `dyn`-intended traits. With
`-D warnings` in force this matters.

Tests then define a `FakeTransport { responses: RefCell<VecDeque<RawResponse>>, calls: RefCell<Vec<(String, String, String)>> }` and assert on the recorded calls — which is also how you
*prove* D-10 (that the `Authorization` value has no `Bearer ` prefix) without touching the network.

### Pattern 3: The content producer is a parameter, not a hardcoded call (D-02, NOMI-06)

D-02 says one content type ships; NOMI-06 says a second must not require touching the delivery
path. Satisfy both by making the delivery function take the already-produced text:

```rust
/// Delivery path. Knows nothing about reports.
pub async fn deliver<T: NomiTransport>(
    transport: &T,
    target_kind: NomiTargetKind,
    target_uuid: &Uuid,
    api_key: &str,
    message_text: &str,       // <- already wrapped and already within the limit
) -> Result<(), NomiError>
```

and keep report-specific assembly (OOC wrapping, shortening) in a small function that the
background job calls before `deliver`. Adding "weekly summary" later then means one new producer
function and zero edits to `deliver`.

### Pattern 4: Pure functions for everything decidable without I/O

These five are the whole behavioural risk of the phase and every one of them is a pure function:

| Function                                                          | Covers            |
| ----------------------------------------------------------------- | ----------------- |
| `fn wrap_ooc(text: &str) -> String`                               | D-15              |
| `fn shorten(...) -> String`                                       | D-16, D-17, NOMI-04 |
| `fn is_due(send_time: NaiveTime, last_attempt: Option<NaiveDate>, now_local: NaiveDateTime) -> bool` | D-11, D-12, NOMI-03 |
| `fn classify_error(status: u16, body: &str) -> NomiError`         | D-18, NOMI-05     |
| `fn chat_url(kind, uuid) -> String`                               | D-06, NOMI-07     |

Write these first; they need no pool, no runtime, no fixtures.

### Anti-Patterns to Avoid

- **Comparing the current minute to the send time for equality.** See Pitfall 3.
- **Reusing a stored nonce when re-encrypting.** See Pitfall 6.
- **Putting the plaintext key into an error type or a log line.** See Pitfall 7.
- **Adding the connection columns to `household_settings`.** That table is keyed on
  `household_id` alone (`PRIMARY KEY` in migration `20240109000000_household_settings.sql`, shape
  reproduced at `test_utils.rs:77-100`), so it structurally cannot hold per-user data. Its shared
  type also has 22 columns that must stay "column-for-column identical to `HouseholdSettingsRow`"
  (`test_utils.rs:73-74`) — widening it ripples into `household_settings.rs:88-97` and `:196-223`,
  the `UpdateHouseholdSettingsRequest` DTO, and the frontend settings form.
- **Hanging the connection off `household_memberships`.** It is the only existing (user, household)
  table (`migrations/20240101000000_initial.sql:31-39`, `UNIQUE(household_id, user_id)`), but it is
  core domain data carrying `role` and `points`; every `SELECT *` into `MembershipRow` would have
  to change.

---

## Encryption at Rest (D-07, D-08, NOMI-02)

### What is already there

| Fact                                                                                     | Evidence                                        |
| ---------------------------------------------------------------------------------------- | ----------------------------------------------- |
| No AES crate anywhere. No `aes`, `aes-gcm`, `chacha20poly1305`, `ctr`, `ghash`, `aead`.  | full `grep '^name = '` of `Cargo.lock`          |
| `ring 0.17.14` IS present — transitively, via `jsonwebtoken` (a direct backend dep).      | `Cargo.lock:2561-2572`, consumer `jsonwebtoken` |
| `rand_core 0.6` with `getrandom` is a **direct** backend dependency; `OsRng` already used | `Cargo.toml:32`, `backend/Cargo.toml:36`, `services/auth.rs:5` and `:57` |
| `subtle 2.6.1`, `zeroize 1.8.2`, `generic-array 0.14.7`, `crypto-common 0.1.7` in the lock | `Cargo.lock:3191, 4144, 1140, 778`              |
| `base64 0.22.1` in the lock (transitive only)                                             | `Cargo.lock:451`; consumers: actix-http, jsonwebtoken, sqlx-core, leptos_reactive, pem |
| The project's only credential handling today is **hashing** — argon2 for passwords, sha2 for refresh tokens | `services/auth.rs:1-11, 57-63`                  |

So: nothing to reuse for encryption, and the existing `auth.rs` is a style reference only, exactly
as CONTEXT.md warns.

### Key material: format and path into `Config`

Mirror `jwt_secret` structurally, but **make it optional**. `jwt_secret` uses
`.expect("JWT_SECRET environment variable must be set")` (`config.rs:26-27`). Copying that for the
new key would (a) break `Config::from_env()` for every existing deployment on upgrade and (b) fail
the existing `test_config_defaults` at `config.rs:69-87`, which only sets `JWT_SECRET`.

```rust
// backend/src/config.rs
pub struct Config {
    // … existing 9 fields unchanged …
    /// Base64-encoded 32-byte AES-256-GCM key. `None` disables the nomi.ai integration
    /// entirely: the endpoints return 503 and the background job skips its pass.
    pub nomi_encryption_key: Option<String>,
}

// in from_env():
nomi_encryption_key: env::var("NOMI_ENCRYPTION_KEY").ok(),
```

Validation (length = 32 bytes after base64 decode) belongs in `services::crypto`, not in
`Config::from_env` — `from_env` returns `Result<Self, env::VarError>` and has no channel for a
"wrong length" error. Decode once at startup or lazily in the crypto service; either way, surface
a clear `log::error!` and treat a malformed key as "feature disabled", not as a panic that takes
the whole household app down.

> **Note for the planner:** `Config` derives `Debug` (`config.rs:3`) and is cloned into
> `AppState` (`models/mod.rs:50-54`) and into `web::Data` (`main.rs:74`). `jwt_secret` is already
> exposed to `{:?}` the same way, so this is not a regression — but if the plan wants to fix it,
> a hand-written `impl Debug for Config` that prints `nomi_encryption_key: "<redacted>"` is a
> two-line addition. Worth doing while touching the struct.

### Stored format

Recommend **one BLOB column**, self-describing, no separate nonce column:

```
api_key_encrypted BLOB          -- layout: [0x01][nonce: 12 bytes][ciphertext || tag: 16 bytes]
```

- Byte 0 is a scheme version. It costs one byte and makes key rotation or an algorithm change a
  migration rather than an archaeology exercise.
- The 96-bit nonce is the AES-GCM standard size and is what `aes_gcm::Nonce` expects.
- The 128-bit GCM tag is appended by `Aead::encrypt` automatically; do not manage it separately.
- SQLite stores `BLOB` natively and sqlx binds `Vec<u8>` / decodes `Vec<u8>` without ceremony,
  so **no base64 is needed for storage** — base64 is only for getting the *key* in from the env var.

Nonce generation: a **fresh random 12 bytes on every encryption**, from
`rand_core::OsRng` (already a direct dep). GCM nonce reuse under the same key is a total break, and
the realistic way to hit it here is re-encrypting on an update while keeping the old nonce.

### `module.nix` (D-08)

The existing `jwtSecretFile` / `jwtSecret` / auto-generate triad is the template
(`module.nix:61-86`, `:139-150`, `:174-176`, `:192-194`). Mirror it:

```nix
nomiEncryptionKeyFile = lib.mkOption {
  type = lib.types.nullOr lib.types.path;
  default = null;
  example = "/run/secrets/haushalt-nomi-key";
  description = ''
    Path to a systemd EnvironmentFile containing a single line
    `NOMI_ENCRYPTION_KEY=<base64 of 32 random bytes>`, e.g. from `openssl rand -base64 32`.

    Optional. If neither this nor `nomiEncryptionKey` is set, the service generates one into
    its state directory on first start and reuses it afterwards.

    WARNING: this key is not derivable from anything else. If it is lost or changed, every
    stored nomi.ai API key becomes undecryptable and every member must re-enter theirs.
  '';
};

nomiEncryptionKey = lib.mkOption { … };   # plain string, same Nix-store warning as jwtSecret
```

and in `startScript` (`module.nix:139-150`), alongside the existing JWT block:

```bash
if [ -z "''${NOMI_ENCRYPTION_KEY:-}" ]; then
  if [ ! -s "${stateDir}/nomi-encryption-key" ]; then
    ( umask 077; ${pkgs.openssl}/bin/openssl rand -base64 32 | tr -d '\n' > "${stateDir}/nomi-encryption-key" )
  fi
  NOMI_ENCRYPTION_KEY=$(cat "${stateDir}/nomi-encryption-key")
  export NOMI_ENCRYPTION_KEY
fi
```

**Two deployment gotchas the planner must not miss:**

1. `serviceConfig` currently sets `EnvironmentFile` as a **single value** inside one
   `lib.optionalAttrs` (`module.nix:192-194`). systemd accepts a list; with two independent secret
   files the attribute has to become a list built from both options, or the second one silently
   wins. This is a real edit, not a copy-paste.
2. Auto-generating into `${stateDir}` is safe *because the SQLite database lives in the same
   directory* (`DATABASE_URL = "sqlite:${stateDir}/haushalt.db?mode=rwc"`, `module.nix:164`).
   Losing the state dir already loses the data, so the key is no more fragile than the DB. Say so
   in the phase output; it is the argument that makes auto-generation acceptable.

Outbound network egress is already permitted: the unit sets `DynamicUser`, `NoNewPrivileges`,
`PrivateTmp`, `ProtectSystem = "strict"`, `ProtectHome` (`module.nix:187-191`) but **no
`IPAddressDeny`, no `PrivateNetwork`, no `RestrictAddressFamilies`**. With webpki-roots there is
also no `/etc/ssl` read to worry about under `ProtectSystem = "strict"`.

---

## Per-User-Per-Household Storage (D-03, D-04, NOMI-01)

### Survey of existing tables

| Table                     | Key                                | Verdict                                                                 |
| ------------------------- | ---------------------------------- | ----------------------------------------------------------------------- |
| `household_settings`      | `PRIMARY KEY (household_id)`       | Household-wide only. Cannot express per-user. `test_utils.rs:77-78`.     |
| `user_settings`           | `PRIMARY KEY (user_id)`            | User-wide only. `migrations/20240121000000_user_settings.sql:3`.         |
| `household_memberships`   | `id` PK + `UNIQUE(household_id, user_id)` | The **only** (user, household) table. Core domain data — do not extend. |
| `user_dashboard_tasks`    | `PRIMARY KEY (user_id, task_id)`   | Precedent for a composite-PK per-user join table with no surrogate id.   |

Conclusion: **a new table.** There is no per-user-per-household settings concept in the schema yet;
this phase introduces it.

### Proposed schema

`backend/migrations/20240150000000_nomi_connections.sql` — the series runs
`20240101000000` … `20240149000000_task_assignee_cannot_uncomplete.sql`, so `20240150000000` is
next. Naming is `NNNNNNNNNNNNNN_snake_case_subject.sql`.

```sql
-- Per-user, per-household outbound nomi.ai connection (D-03).
-- The API key is stored ENCRYPTED, never hashed: it must be recoverable to be used.
CREATE TABLE IF NOT EXISTS nomi_connections (
    household_id       TEXT NOT NULL REFERENCES households(id) ON DELETE CASCADE,
    user_id            TEXT NOT NULL REFERENCES users(id)      ON DELETE CASCADE,

    enabled            BOOLEAN NOT NULL DEFAULT 0,

    -- Credential (D-07): [version:1][nonce:12][ciphertext||tag]. NULL = not configured.
    api_key_encrypted  BLOB,

    -- Target (D-05/D-06). NULL until the user has picked one.
    target_kind        TEXT CHECK(target_kind IN ('nomi', 'room')),
    target_uuid        TEXT,
    target_name        TEXT,          -- cached display name, refreshed when targets are listed

    -- Schedule (D-11/D-12): "HH:MM" in the HOUSEHOLD timezone, same shape as tasks.due_time.
    send_time          TEXT NOT NULL DEFAULT '08:00',

    -- Idempotence + feedback (D-19). last_attempt_date is the local date in the household tz.
    last_attempt_date  DATE,
    last_sent_at       DATETIME,
    last_error         TEXT,

    created_at         DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at         DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,

    PRIMARY KEY (household_id, user_id)
);

-- The background job's hot query: "all enabled connections", then filter in Rust per household tz.
CREATE INDEX IF NOT EXISTS idx_nomi_connections_enabled
    ON nomi_connections(enabled);
```

Notes on the choices:

- **`PRIMARY KEY (household_id, user_id)`** rather than a surrogate `id` — matches
  `user_dashboard_tasks` and `missed_task_penalties`, and makes the uniqueness invariant of D-03
  structural.
- **`send_time TEXT` in "HH:MM"** matches `tasks.due_time` exactly
  (`migrations/20240124000000_task_due_time.sql`), so `scheduler::parse_due_time(Option<&str>)`
  (`scheduler.rs:332-336`) can be reused verbatim for parsing. Note its default is `23:59` on a
  parse failure — for a send time you probably want an explicit `NaiveTime::parse_from_str(t, "%H:%M")`
  and to treat failure as "misconfigured, skip + record error", not "send at 23:59". Decide explicitly.
- **`ON DELETE CASCADE`** on both FKs so removing a household or a user does not leave an
  orphaned encrypted key behind. Note: SQLite only enforces this with `PRAGMA foreign_keys = ON`,
  which sqlx's SQLite driver enables by default — but the existing schema is inconsistent about
  this (`missed_task_penalties` uses CASCADE, `household_memberships` does not), so do not rely on
  it as the only cleanup path.
- **`last_attempt_date` vs `last_sent_at`**: two different jobs. `last_attempt_date` is the
  idempotence key ("this local date has been attempted"); `last_sent_at` is the D-19 feedback
  ("when did it last actually arrive"). Conflating them means either no feedback on failures or a
  retry storm. See Pitfall 4.

### The test-schema duplication — a mandatory second write

**`test_utils::run_migrations` does not run the migrations.** Verbatim, `test_utils.rs:26-30`:

```rust
pub async fn run_migrations(pool: &SqlitePool) {
    // Note: Using sqlx::migrate!() here would require the migrations to be in the right path
    // For tests, we'll create the tables manually based on the actual schema
    create_test_schema(pool).await;
}
```

`create_test_schema` (`test_utils.rs:32`) is ~440 lines of hand-written `CREATE TABLE IF NOT EXISTS`.
Any new table must be added there too, or every `#[tokio::test]` that touches it fails with
`no such table`.

The duplication has already drifted, which is the proof that this is a live hazard: the test
`user_settings` table (`test_utils.rs:110-118`) declares `theme`, `notifications_enabled` and
`created_at`, none of which exist in `migrations/20240121000000_user_settings.sql`.

→ **This belongs in Wave 0 of the plan, not as an afterthought inside a later task.**

---

## Where the Scheduled Sender Hooks In (D-11, D-12, D-13, NOMI-03)

### The loop as it exists

`backend/src/services/background_jobs.rs:77-176`:

```rust
pub async fn start_scheduler(pool: Arc<SqlitePool>, config: JobConfig) {
    log::info!("Background job scheduler started. Missed task check every {} minutes", config.check_interval_minutes);
    let interval = std::time::Duration::from_secs((config.check_interval_minutes * 60) as u64);

    loop {
        time::sleep(interval).await;          // :86 — sleeps FIRST

        match process_missed_tasks(&pool).await     { Ok(r) => …, Err(e) => log::error!(…) }   // :89-111
        match process_auto_archive(&pool).await     { Ok(r) => …, Err(e) => log::error!(…) }   // :114-132
        match process_period_finalization(&pool).await { … }                                    // :135-157
        match solo_mode::check_and_deactivate_expired_solo_modes(&pool).await { … }             // :160-174
    }
}
```

`JobConfig::default()` is `check_interval_minutes: 1` (`:67-73`). Call site: `main.rs:48-56`.

**Adding a job is a four-line edit** — a `match process_nomi_sends(...).await { Ok(r) => log::info!/debug!, Err(e) => log::error!("…") }` block appended inside the loop, in the same
shape as the four that precede it. That shape *is* the D-13 isolation at the job level: an `Err`
is logged and the next job still runs.

### The signature problem (blocking, must be planned)

`start_scheduler` takes only `Arc<SqlitePool>`. **It has no access to `Config`, therefore no access
to the encryption key, therefore it cannot decrypt anything.** The plan must change the signature
and the `main.rs:49-55` call site. Two shapes, both fine:

```rust
// A: pass the whole Config (simplest, mirrors AppState)
pub async fn start_scheduler(pool: Arc<SqlitePool>, config: JobConfig, app_config: Config)

// B: pass only what is needed (narrower, easier to construct in tests)
pub async fn start_scheduler(pool: Arc<SqlitePool>, config: JobConfig, nomi_key: Option<Vec<u8>>)
```

B is preferable: `process_nomi_sends` then takes `Option<&[u8]>` and a test can pass a fixed
32-byte key without building a whole `Config`.

### Idempotence — copy `missed_task_penalties`, do not invent

The existing pattern is "has this (subject, local date) already been processed?", stored as a row:

- Table: `migrations/20240125000000_missed_task_tracking.sql`, `PRIMARY KEY (task_id, due_date)`,
  with the comment "This prevents duplicate penalties when the background job runs multiple times per day".
- Check before acting: `background_jobs.rs:288-299`
  (`SELECT COUNT(*) FROM missed_task_penalties WHERE task_id = ? AND due_date = ?` → `continue`).
- Record after acting: `background_jobs.rs:368-374`.

For nomi the subject is the connection, and the natural home for the marker is a column on the
connection row itself (there is exactly one per (household, user), so a separate table buys
nothing except history that D-19 does not ask for):

```
send if:  enabled
      AND api_key_encrypted IS NOT NULL
      AND target_uuid IS NOT NULL
      AND (last_attempt_date IS NULL OR last_attempt_date < today_local)
      AND now_local.time() >= send_time
```

`last_attempt_date < today_local` rather than `!=` so that a clock/timezone change backwards
cannot resurrect an old date and cause a re-send.

Set `last_attempt_date = today_local` **on every attempt, success or failure** — with `last_error`
set on failure and `last_sent_at` set on success. That is one attempt per local day, which is what
D-18's "handle without crashing the run" plus D-19's "surface the last error" imply. See Pitfall 4
for the alternative and its cost.

### Timezone resolution — reuse, with one caveat

```rust
let settings = household_settings::get_or_create_settings(pool, &household_id).await?;  // :60
let tz       = scheduler::parse_timezone(&settings.timezone);                            // scheduler.rs:322
let now_local = now_utc.with_timezone(&tz);
let today_local = now_local.date_naive();
```

Note `scheduler::today_in_timezone(tz)` (`scheduler.rs:327-329`) hardcodes `Utc::now()` internally.
`report.rs:69-74` deliberately does **not** use it, with the comment "the scheduler's convenience
helper for the local date hardcodes `Utc::now()` internally, which would make this function
untestable with a pinned moment". **The sender must make the same choice**: take
`now_utc: DateTime<Utc>` as a parameter and derive the local time from it. This is also
CONVENTIONS.md:89 ("prefer passing `current_date` / time explicitly to functions under test").

`process_missed_tasks` also caches settings per household in a `HashMap<Uuid, HouseholdSettings>`
(`background_jobs.rs:199-230`) to avoid re-querying. With one connection per (household, user) the
same caching applies and is worth copying.

### D-13 at the right granularity

The existing per-job `match … Err(e) => log::error!` isolates *jobs* from each other. D-13 demands
isolation between *connections*. So the inner loop must be:

```rust
for conn in connections {
    match send_one(&pool, transport, key, &conn, now_utc).await {
        Ok(_)  => sent += 1,
        Err(e) => {                       // never `?`
            log::warn!("nomi send failed for household {} user {}: {}", conn.household_id, conn.user_id, e);
            record_failure(&pool, &conn, &e).await.ok();   // best effort; do not propagate
            failed += 1;
        }
    }
}
```

`?` inside that loop is the bug D-13 exists to prevent.

One more free safety property: `generate_daily_report` re-checks membership itself
(`report.rs:63-65`, returns `ReportError::NotAMember`). A connection left behind for a member who
was removed from the household therefore fails closed rather than leaking data. Treat `NotAMember`
as "disable this connection", not as a transient error to retry tomorrow.

---

## Calling the Report (D-14)

Exact signature, `backend/src/services/report.rs:54-59`:

```rust
pub async fn generate_daily_report(
    pool: &SqlitePool,
    household_id: &Uuid,
    user_id: &Uuid,
    now_utc: DateTime<Utc>,
) -> Result<String, ReportError>
```

- Needs only the pool, the two ids and an injected instant. It resolves the household timezone
  itself (`report.rs:67-76`) and fetches the household name itself (`:78-80`).
- Returns the finished plain-text report as a `String`. No structure.
- `ReportError` variants (`report.rs:25-39`): `Database`, `Task`, `Settings`, `Household`,
  `HouseholdNotFound`, `NotAMember`.
- Current call site for reference: `handlers/report.rs:49-55`, which passes `chrono::Utc::now()`
  "at the edge, precisely so the service stays testable with a pinned date".

The sender calls this directly — same crate, same pool, no HTTP. `handlers/report.rs` stays
untouched.

**Text shape** (`report.rs:303-343`):

```
Daily report — {household name} — {%a, %Y-%m-%d}
                                                   <- blank line
Due today:
- {title} (by {HH:MM}) (done)
- …
                                                   <- blank line
Missed yesterday:
- {title} (by {HH:MM})
- …
```

with `DUE_TODAY_EMPTY = "No tasks scheduled for today"` and
`MISSED_YESTERDAY_EMPTY = "All tasks completed yesterday"` (`report.rs:18-23`). Both sections
always render; there is no combined empty variant (`:301-302`). No trailing newline.

---

## Length Handling (D-16, D-17, NOMI-04)

### Where the shortener can live without duplicating the formatter

Everything needed is private in `report.rs`:

| Item                 | Line          | Visibility |
| -------------------- | ------------- | ---------- |
| `struct ReportLine`  | `report.rs:44` | private    |
| `fn format_report`   | `report.rs:303` | private    |
| `fn format_section`  | `report.rs:321` | private    |
| `fn format_report_line` | `report.rs:334` | private |
| `fn sort_report_lines`  | `report.rs:347` | private |

Three options, in order of preference:

1. **Add a second public entry point to `report.rs`** that builds the same two `Vec<ReportLine>`s,
   trims them, then calls the *existing* `format_report`:

   ```rust
   /// Same report as `generate_daily_report`, shortened to fit `max_chars` (D-16/D-17).
   /// `max_chars` counts CHARACTERS, not bytes, and excludes any external wrapper.
   pub async fn generate_daily_report_capped(
       pool: &SqlitePool,
       household_id: &Uuid,
       user_id: &Uuid,
       now_utc: DateTime<Utc>,
       max_chars: usize,
   ) -> Result<String, ReportError>
   ```

   `ReportLine` stays private, `format_report_line` stays the single line formatter (its DRY
   comment at `:332` is explicit about that), the 48 existing report tests stay green, and the
   shortening logic sits next to the formatter it has to agree with. **Recommended.**

2. Refactor `generate_daily_report` into `build_sections` + `format_report` and make
   `generate_daily_report` a thin wrapper. Cleaner in the abstract, but it moves code the 48
   existing tests exercise, for no functional gain.

3. Parse the produced string back into lines in `nomi.rs`. Do not — it couples the sender to the
   report's text format and silently breaks the next time the report wording changes.

### The arithmetic

The OOC wrapper (D-15) is `"(OOC: Household App (" + text + "))"` — **21 + 2 = 23 characters** of
fixed overhead. So `max_chars` passed to the report = `limit - 23`. Define both the wrapper string
and the subtraction in one place (`services::nomi`), per D-15 and CLAUDE.md § DRY.

The limit itself (D-17) must be a runtime value, not a `const`. Concretely: a field on the
connection (or a per-target-kind default of 800 with an override) that the plan can raise without
a recompile. A `const DEFAULT_LIMIT: usize = 800;` used as the *default value of a configurable
field* satisfies D-17; a `const` consulted at the call site does not.

### Chars, not bytes

`String::len()` returns **bytes**. Nomi's limit is **characters**. German task titles routinely
contain `ü`/`ö`/`ä`/`ß` (2 bytes each in UTF-8), and the report header itself uses an em dash `—`
(`report.rs:310`, 3 bytes, 1 char). A byte-based check would truncate a valid message and, worse,
a byte-based *slice* would panic on a non-boundary index.

Use `.chars().count()` for measurement and `.chars().take(n).collect::<String>()` for any
character-level trimming. Never `&s[..n]`.

### Degenerate cases the shortener must survive

1. **Zero task lines still does not fit.** `Daily report — {household.name} — Mon, 2027-01-04`
   embeds a user-controlled household name of unbounded length. If the header + headers + empty
   states already exceed the budget, the shortener must produce *something* rather than loop or
   panic. Decide: truncate the household name with `…`, or fall back to a minimal message.
2. **Which section gives way first.** D-16 says "do not truncate blindly from the end — that would
   drop 'Missed yesterday'". The natural reading is: shorten "Due today" first, and only touch
   "Missed yesterday" if it still does not fit. Make that ordering explicit in the plan rather than
   leaving it to the executor.
3. **The counter itself costs characters.** `… und N weitere` is ~15 chars and N is variable.
   Removing one line to make room can push the counter from `N=9` to `N=10`, adding a character.
   Compute the budget including the counter, or iterate to a fixed point.
4. **Empty-state substitution.** If shortening removes *every* line from a section, the section
   should show its empty state — except that would be a lie ("No tasks scheduled for today" when
   there were 40). Prefer keeping at least one line plus the counter.

### C-1: the language contradiction

`report.rs:1-5`, verbatim:

```
//! D-01: the report text is generated here, in the backend, and is ALWAYS English —
//! it deliberately bypasses the frontend i18n system because a later phase will feed
//! this text to an LLM.
```

Every string in the report is English: `"Due today:"`, `"Missed yesterday:"`,
`"No tasks scheduled for today"`, `"All tasks completed yesterday"`, `"- {title} (by …) (done)"`,
and the date is formatted `%a` which "is always English" (`report.rs:312`).

D-16 specifies the German `… und N weitere`. Inserting it produces a mixed-language artefact and
contradicts phase 2.1's locked D-01, which report.rs treats as binding.

**Flagged, not resolved.** Options for the user:
- (a) `… and N more` — consistent with the report, consistent with 2.1 D-01, breaks the literal
  wording of 05 D-16.
- (b) `… und N weitere` verbatim — honours 05 D-16 literally, breaks 2.1 D-01.
- (c) Make it the one localized fragment, keyed off the user's `user_settings.language` — most
  work, and it makes the report no longer language-stable for the future LLM consumer that 2.1
  D-01 was designed for.

The planner must escalate this rather than pick.

---

## Frontend Settings Surface (NOMI-01, D-04, D-05, D-09, D-19)

### C-5: the briefing's i18n claim is wrong

The brief states "strings are in Rust, not JSON". They are in JSON.
`frontend/src/i18n/mod.rs:48-55`:

```rust
fn load_translations(lang: &str) -> Translations {
    let json = match lang {
        "de" => include_str!("../translations/de.json"),
        _ => include_str!("../translations/en.json"),
    };
    serde_json::from_str(json).unwrap_or_default()
}
```

`Translations = HashMap<String, String>` with flat dotted keys. `frontend/src/translations/en.json`
has **726 keys**, 58 of them under `settings.`. The Rust module is only the loader, the
`I18nContext { language, translations }` signal wrapper with `t(&self, key) -> String`
(`mod.rs:26-32`), and the tests.

**Adding strings:** edit both `en.json` and `de.json`, then add a key-presence test to
`i18n/mod.rs` copying `test_report_keys_present_in_both_languages` (`mod.rs:102-116`):

```rust
#[test]
fn test_nomi_keys_present_in_both_languages() {
    let en = load_translations("en");
    let de = load_translations("de");
    for key in ["nomi.section_title", "nomi.api_key", "nomi.api_key_set", /* … */] {
        assert!(en.contains_key(key), "missing english key: {}", key);
        assert!(de.contains_key(key), "missing german key: {}", key);
    }
}
```

This runs natively — verified: `nix develop -c cargo test -p frontend --lib` → **166 passed**.
(`#[wasm_bindgen_test]` blocks also run natively under `cargo test`, despite the comment at
`components/css_contract.rs:9-11` claiming otherwise. The 166 count includes them.)

### Where the section goes

| Candidate                        | Route                        | Has household context? | Problem                                                                                                              |
| -------------------------------- | ---------------------------- | ---------------------- | ---------------------------------------------------------------------------------------------------------------------- |
| `pages/settings.rs` (74 lines)   | `/settings` (`app.rs:54`)    | **No**                 | Only shows username/email and an About card. Would need a household picker just to host this.                        |
| `pages/user_settings.rs` (132)   | `/user-settings` (`app.rs:55`) | **No**                 | Same problem. It is the language picker.                                                                             |
| `pages/household_settings.rs` (1067) | `/households/:id/settings` (`app.rs:52`) | **Yes**                | Parts are role-gated (`current_role.get().map(|r| r.can_manage_tasks())` at `:266`, `is_owner` at `:139`). The route itself is not gated. |

**Recommendation:** a new `Card` section on `pages/household_settings.rs`, placed **outside** every
`<Show when=…role…>` guard, since the connection is per-user (D-03) and every member must be able
to configure their own. This is the smallest change satisfying D-04's "one settings section" and
NOMI-01's "per household". The alternative — a dedicated `/households/:id/nomi` route plus a tab in
`components/household_tabs.rs` — is more code and arguably worse, since D-04 asks for one section,
not one page. Flag the choice for the planner; it is within "table and module naming" discretion
but has a visible UX consequence.

### Components already available (reuse, do not build)

`frontend/src/components/mod.rs:43-63` exports a full form kit. The relevant ones:

- **`TimeInput`** (`components/time_input.rs`) — `type="time"`, prop
  `value: RwSignal<Option<String>>`, emits `"HH:MM"`, `None` when cleared. Exactly the send-time
  input, no new component needed.
- `SelectInput`, `Checkbox`, `TextInput`, `FormGroup`, `Card`, `SectionHeader`, `Button`,
  `Alert`, `Divider`, `Loading`.

`pages/household_settings.rs:9-13` shows the import style; `:23-27` the
`loading`/`saving`/`error`/`success` signal quartet; `:76-` the `create_effect` load pattern.

### The CSS contract is enforced by a test

`frontend/src/components/css_contract.rs` is a **native** test (`#![cfg(all(test, not(target_arch = "wasm32")))]`, `:13`) that scans every `class="…"` in `src/pages/` and
`src/components/`, and fails the build if any class starting with `form-` or `modal-`
(`CHECKED_PREFIXES`, `:31`) has no `.class` selector in `frontend/styles.css`.

Classes that exist today: `.form-field-error .form-group .form-hint .form-input .form-label
.form-row .form-select .form-textarea .modal-backdrop .modal-close .modal-footer .modal-header
.modal-large .modal-task .modal-title`. Exempt: `modal-body`, `modal-sm` (`UNSTYLED_CLASSES`, `:26`).

→ Do not invent new `form-*` / `modal-*` class names without adding the rule to `styles.css`.
This test caught exactly this class of bug on 2026-07-28 (quick task `260728-dah`).

### The API surface (D-09, D-05, D-19)

`ApiClient` is a set of associated functions on a unit struct going through one
`Self::request::<T>(method, path, body, auth)` helper (`frontend/src/api/mod.rs`, pattern at
`:414-422` and `:436-447`). Three new methods:

```rust
ApiClient::get_nomi_connection(household_id)                  -> NomiConnection
ApiClient::update_nomi_connection(household_id, request)      -> NomiConnection
ApiClient::list_nomi_targets(household_id)                    -> NomiTargetsResponse
```

Backend routes registered in a new `handlers/nomi.rs::configure` added to the
`web::scope("/{household_id}")` block at `handlers/households.rs:30-44` (alongside
`report::configure` at `:43`), giving:

```
GET  /api/households/{household_id}/nomi          -> connection (NEVER the key)
PUT  /api/households/{household_id}/nomi          -> update (key optional; absent = unchanged)
GET  /api/households/{household_id}/nomi/targets  -> proxied GET /v1/nomis + GET /v1/rooms
```

Handler shape: copy `handlers/report.rs:18-45` — `extract_user_id`, then
`household_service::is_member(&state.db, &household_id, &user_id)` (`services/households.rs:182`),
then one service call, then map the `Result`. Thin.

**D-09 enforcement is a type-level property, and should be tested as one.** The shared DTO must
have no field that could carry the key:

```rust
pub struct NomiConnection {
    pub household_id: Uuid,
    pub user_id: Uuid,
    pub enabled: bool,
    pub api_key_set: bool,          // presence only — never the value
    pub target: Option<NomiTarget>,
    pub send_time: String,          // "HH:MM"
    pub last_sent_at: Option<DateTime<Utc>>,   // D-19
    pub last_error: Option<String>,            // D-19
    pub updated_at: DateTime<Utc>,
}
```

A `serde_json::to_string(&conn)` test asserting the output does not contain a known key string is
cheap and catches accidental field additions later.

### The target-selection ordering problem (D-05)

To list targets by name you need a valid API key; to configure the connection you need a target.
The frontend must never see the key (D-09), so the listing must be server-side. That leaves an
ordering question the plan has to answer:

- **Two-step (recommended):** save the key → "Load targets" button → the backend uses the *stored*
  key to call `GET /v1/nomis` and `GET /v1/rooms` → user picks a name → save the target. Simple,
  the key only ever travels once, and `GET …/nomi/targets` needs no body.
- **One-step:** `POST …/nomi/targets` with the key in the body, before it is persisted. Fewer
  clicks, but it puts the plaintext key in a second request path (and therefore in a second set of
  logs to audit).

D-05's "Whether target selection is cached or fetched live" is explicitly Claude's discretion; the
*ordering* is not addressed and should be decided in the plan.

Rooms carry a `status` field; `"Creating"` is exactly the state that produces `RoomStillCreating`
on send. Showing it in the picker (greyed, or with a hint) is a cheap way to prevent a
predictable failure.

---

## Don't Hand-Roll

| Problem                             | Don't build                                      | Use instead                                                      | Why                                                                                                                                       |
| ----------------------------------- | ------------------------------------------------ | ---------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------- |
| Symmetric encryption                | AES rounds, GCM/GHASH, tag comparison            | `aes-gcm` (or `ring::aead`)                                       | Constant-time tag comparison, correct GHASH, correct counter handling. A hand-rolled tag check leaks via timing.                            |
| Nonce generation                    | A counter, a timestamp, a hash of the user id    | `rand_core::OsRng` (already a direct dep, `services/auth.rs:5`)   | GCM nonce reuse under one key is a catastrophic break, and every deterministic scheme eventually collides after a restore-from-backup.      |
| HTTPS / TLS                         | Raw `tokio::net::TcpStream` + a TLS handshake    | `reqwest` + rustls                                                | Certificate validation, SNI, ALPN, redirects, chunked encoding, timeouts, connection pooling.                                              |
| Base64                              | A 40-line encoder                                | `base64 = "0.22"` (already in the lock)                           | Padding, URL-safe vs standard alphabets, whitespace tolerance.                                                                            |
| Timezone / DST arithmetic           | Offset tables, "add 1 hour in summer"            | `chrono-tz` via `scheduler::parse_timezone` (`scheduler.rs:322`)  | Already a workspace dep (`Cargo.toml:20`) and already the project's answer for exactly this.                                               |
| "Today in the household's timezone" | A fresh `Utc::now().date_naive()` + manual shift | `now_utc.with_timezone(&tz).date_naive()`, as `report.rs:73-74` does | One line, already the established idiom, and testable with a pinned instant.                                                               |
| Per-day idempotence                 | An in-memory `HashSet` of "already sent today"   | A persisted date column, as `missed_task_penalties` does           | An in-memory set is lost on restart — and the service restarts on failure (`module.nix:184-185`, `Restart = "on-failure"`).                |
| A cron scheduler                    | `cron`, `tokio-cron-scheduler`, a second task    | The existing `background_jobs` loop (D-11)                        | It already ticks every 60 s. A second scheduler means a second set of drift, restart and overlap bugs.                                     |
| JSON parsing                        | String scanning of `{"error":{"type":"…"}}`      | `serde_json` (already a direct backend dep)                       | The error envelope carries "additional fields depending on the error" per the docs — a scanner breaks on the first one that does.          |
| An HTTP mock server for tests       | —                                                | A `FakeTransport` implementing the one-method trait               | Faster, deterministic, no ports, no new crates. See § Validation Architecture.                                                            |

**Key insight:** the only genuinely novel code in this phase is ~5 pure functions and one
`nomi_connections` CRUD module. Everything with sharp edges — crypto, TLS, timezone maths,
scheduling, idempotence — already has an owner, either in a crate or in this repository. The
failure mode for this phase is not "we could not build it", it is "we reimplemented something that
was already three lines away".

---

## Common Pitfalls

### Pitfall 1: Copying the inbound `Bearer ` convention outbound

**What goes wrong:** `Authorization: Bearer <key>` → nomi returns `401 Unauthorized`, which looks
exactly like a wrong API key. The user re-enters a perfectly good key and it fails again.
**Why it happens:** this very codebase enforces the Bearer prefix on the *inbound* side —
`middleware/auth.rs:30-34`:
```rust
if !auth_str.starts_with("Bearer ") { return Err(AuthMiddlewareError::InvalidToken); }
let token = &auth_str[7..];
```
It is the nearest example and it is the wrong one to copy.
**How to avoid:** `.header("Authorization", api_key)` with the raw value. Assert it in a test using
the `FakeTransport`'s recorded calls — that is the only way this stays fixed.
**Warning signs:** a 401 with `{"error":{"type":"Unauthorized"}}` for a key that works in `curl`.

### Pitfall 2: Byte length instead of character length

**What goes wrong:** `text.len() <= 800` passes for a message with 800 characters that is 860
bytes; nomi rejects it with `MessageCharacterLimitExceeded` / `MessageLengthLimitExceeded`. Or
worse, `&text[..800]` panics with "byte index is not a char boundary".
**Why it happens:** `String::len()` is bytes. German task titles (`Küche`, `Müll`, `Wäsche`) are
routinely multi-byte, and the report header contains an em dash (`report.rs:310`).
**How to avoid:** `.chars().count()` everywhere the limit is involved; `.chars().take(n)` for any
trimming.
**Warning signs:** the shortener behaves differently for English and German task titles.

### Pitfall 3: Treating the minute tick as precise

**What goes wrong:** `if now_local.time().format("%H:%M") == conn.send_time` misses sends, sometimes
for days.
**Why it happens:** `background_jobs.rs:85-86` does `time::sleep(interval).await` at the *top* of
the loop and then runs four jobs. The period is 60 s **plus the work time**, so ticks drift
forward continuously; the minute 08:00 can simply be skipped. And the first tick is 60 s after
startup, not immediately.
**How to avoid:** the condition is `now_local.time() >= send_time`, gated by
`last_attempt_date < today_local`. That is a level trigger with a persisted latch, not an edge
trigger. It self-heals across restarts, long GC pauses and slow report generation.
**Warning signs:** "it usually sends but sometimes doesn't"; sends stop after a day with many tasks.

### Pitfall 4: An idempotence marker that produces either silence or a storm

**What goes wrong:** two symmetric mistakes. (a) Set the marker only on success → a permanently
bad key retries every 60 s for the rest of the day, hammering nomi's rate limit ("repeatedly
hitting the rate limit may result in API access being revoked"). (b) Never record failures → D-19
has nothing to show and the user sees no error at all.
**How to avoid:** set `last_attempt_date = today_local` on **every** attempt, and record
`last_error` on failure / `last_sent_at` on success. One attempt per local day.
**Consequence to accept:** a transient `RoomStillCreating` or a network blip costs that day's
report. If the plan wants in-run retry (this is inside "retry counts and backoff shape",
explicitly Claude's discretion), do it as N attempts with a short backoff **within the same tick**,
then latch — not by leaving the latch open.
**Warning signs:** `last_error` is always `NULL` even though nothing arrives.

### Pitfall 5: The new table exists in the migration but not in the test schema

**What goes wrong:** every new `#[tokio::test]` fails with `no such table: nomi_connections`.
**Why it happens:** `test_utils::run_migrations` does not run the migrations
(`test_utils.rs:26-30`); it runs `create_test_schema`, a hand-written duplicate.
**How to avoid:** add the `CREATE TABLE` to `create_test_schema` **in the same task** as the
migration, and add a `create_test_nomi_connection(...)` fixture next to
`insert_missed_task_penalty` (`test_utils.rs:1110`).
**Warning signs:** the drift is already visible — test `user_settings` (`test_utils.rs:110-118`)
has three columns the real migration does not.

### Pitfall 6: Reusing the nonce on update

**What goes wrong:** the user changes their API key; the code decrypts with the stored nonce,
encrypts the new value with the same nonce, writes back. Two ciphertexts under one key+nonce pair
in AES-GCM leaks the XOR of the plaintexts and, worse, allows tag forgery.
**Why it happens:** it reads as an optimisation ("we already have the nonce right there").
**How to avoid:** the encryption function generates its own nonce internally and returns the whole
`[version][nonce][ct||tag]` blob. There is no API to pass a nonce in.
**Warning signs:** a `nonce` parameter on the encrypt function.

### Pitfall 7: The key in a log line or an error type

**What goes wrong:** the plaintext nomi key lands in the journal, where `DynamicUser` does not
protect it and log aggregation might ship it off-host.
**Why it happens:** the project's error-logging convention is `log::error!("…: {:?}", e)`
(`handlers/report.rs:71`). If `NomiError` carries the request that failed, and the request carries
the header, `{:?}` prints it.
**How to avoid:** `NomiError` variants carry a status code and the `error.type` string, never the
key or the full request. Do not `#[derive(Debug)]` on a struct that holds the key without a manual
redacting impl. Same for `Config` (`config.rs:3`).
**Warning signs:** any `{:?}` on a value transitively reachable from the decrypted key.

### Pitfall 8: Making the new config value required

**What goes wrong:** `Config::from_env()` panics on every existing deployment after the upgrade,
and `test_config_defaults` (`config.rs:69-87`, which sets only `JWT_SECRET`) fails.
**How to avoid:** `Option<String>`; absent means the feature is off. The endpoints return a clear
error, the background job logs once at startup and skips.
**Warning signs:** `.expect("NOMI_ENCRYPTION_KEY must be set")` in `from_env`.

### Pitfall 9: `?` inside the per-connection loop

**What goes wrong:** the first user with a stale key aborts the run; nobody else gets a report.
Directly violates D-13 / NOMI-05.
**How to avoid:** `match` + `log::warn!` + `continue`, mirroring `background_jobs.rs:108-111`.
**Warning signs:** `process_nomi_sends` returns `Result<_, E>` and its inner loop uses `?`.

### Pitfall 10: A 30-second synchronous call holding a DB connection

**What goes wrong:** the direct-Nomi endpoint blocks up to 30 s waiting for a reply
(`NoReply` after that). With `SqlitePoolOptions::new().max_connections(5)` (`main.rs:33-34`) shared
between the HTTP server and the background job, a held connection during that wait starves request
handling.
**How to avoid:** finish all DB work (load the connection, generate the report) *before* the HTTP
call; take a fresh connection afterwards to write `last_sent_at` / `last_error`. Do not hold a
transaction across the network call.
**Also note:** with N direct-Nomi connections the serial loop can take N×30 s, which exceeds the
60 s tick. The loop is serial so runs cannot overlap, but the *other* four jobs get delayed by
that much. Prefer Rooms (which return immediately, per the docs and CONTEXT.md § Specific Ideas)
and consider a per-call timeout below 30 s.

### Pitfall 11: Forgetting `Content-Type: application/json`

**What goes wrong:** `InvalidContentType` on both endpoints. The docs call this out explicitly:
"it is also necessary to explicitly specify the `Content-Type` header as `application/json`".
**How to avoid:** `reqwest`'s `.json(&body)` sets it. If the plan uses `.body(String)` for any
reason, set the header by hand.

### Pitfall 12: An unused import failing the build

`Cargo.toml:52-53` sets `[workspace.lints.rust] warnings = "deny"`. In a new module with several
`#[cfg(test)]`-only imports this bites immediately. Run `nix develop -c cargo check --workspace`
early and often, not only at the end.

---

## Code Examples

### Encrypt / decrypt with `aes-gcm 0.10` and the existing `OsRng`

```rust
// backend/src/services/crypto.rs
// Sources: https://docs.rs/aes-gcm/0.10.3/aes_gcm/  (RustCrypto AEADs)
//          existing OsRng usage: backend/src/services/auth.rs:5, :57
use aes_gcm::{
    aead::{Aead, KeyInit},
    Aes256Gcm, Key, Nonce,
};
use rand_core::{OsRng, RngCore};

const SCHEME_V1: u8 = 0x01;
const NONCE_LEN: usize = 12;

#[derive(Debug, thiserror::Error)]
pub enum CryptoError {
    #[error("Encryption key is not configured")]
    NoKey,
    #[error("Encryption key must decode to exactly 32 bytes")]
    BadKeyLength,
    #[error("Encryption key is not valid base64")]
    BadKeyEncoding,
    #[error("Stored ciphertext is malformed")]
    Malformed,
    #[error("Decryption failed — wrong key or tampered data")]
    DecryptFailed,
}

/// Decode NOMI_ENCRYPTION_KEY once. Base64 so it survives an env var and a
/// systemd EnvironmentFile intact — same idiom as module.nix:144's `openssl rand -base64`.
pub fn decode_key(b64: &str) -> Result<[u8; 32], CryptoError> {
    use base64::{engine::general_purpose::STANDARD, Engine as _};
    let raw = STANDARD.decode(b64.trim()).map_err(|_| CryptoError::BadKeyEncoding)?;
    raw.as_slice().try_into().map_err(|_| CryptoError::BadKeyLength)
}

/// [version:1][nonce:12][ciphertext || tag:16]. A FRESH nonce every call — never reuse.
pub fn seal(key: &[u8; 32], plaintext: &str) -> Result<Vec<u8>, CryptoError> {
    let cipher = Aes256Gcm::new(Key::<Aes256Gcm>::from_slice(key));

    let mut nonce_bytes = [0u8; NONCE_LEN];
    OsRng.fill_bytes(&mut nonce_bytes);
    let nonce = Nonce::from_slice(&nonce_bytes);

    let ct = cipher
        .encrypt(nonce, plaintext.as_bytes())
        .map_err(|_| CryptoError::DecryptFailed)?;

    let mut out = Vec::with_capacity(1 + NONCE_LEN + ct.len());
    out.push(SCHEME_V1);
    out.extend_from_slice(&nonce_bytes);
    out.extend_from_slice(&ct);
    Ok(out)
}

pub fn open(key: &[u8; 32], stored: &[u8]) -> Result<String, CryptoError> {
    if stored.len() < 1 + NONCE_LEN + 16 || stored[0] != SCHEME_V1 {
        return Err(CryptoError::Malformed);
    }
    let cipher = Aes256Gcm::new(Key::<Aes256Gcm>::from_slice(key));
    let nonce = Nonce::from_slice(&stored[1..1 + NONCE_LEN]);

    let plain = cipher
        .decrypt(nonce, &stored[1 + NONCE_LEN..])
        .map_err(|_| CryptoError::DecryptFailed)?;

    String::from_utf8(plain).map_err(|_| CryptoError::Malformed)
}
```

> If the plan takes the `ring` route instead (zero new crates), the equivalent is
> `UnboundKey::new(&ring::aead::AES_256_GCM, key)` → `LessSafeKey::new(..)` →
> `seal_in_place_append_tag(Nonce::assume_unique_for_key(nonce), Aad::empty(), &mut in_out)`, with
> `ring::rand::SystemRandom` for the nonce. Same stored layout.

### The reqwest transport (the only untestable-without-network part)

```rust
// backend/src/services/nomi.rs
// D-10: raw key, NO "Bearer " prefix — https://api.nomi.ai/docs/ § Authorization
// Content-Type is mandatory — https://api.nomi.ai/docs/reference/general
pub struct ReqwestTransport {
    client: reqwest::Client,
}

impl ReqwestTransport {
    pub fn new() -> Result<Self, reqwest::Error> {
        Ok(Self {
            client: reqwest::Client::builder()
                // Below the direct-chat 30 s NoReply window, so we fail before nomi does.
                .timeout(std::time::Duration::from_secs(25))
                .build()?,
        })
    }
}

impl NomiTransport for ReqwestTransport {
    async fn post_json(&self, url: &str, api_key: &str, body: &str) -> Result<RawResponse, TransportError> {
        let resp = self
            .client
            .post(url)
            .header("Authorization", api_key)          // <- raw, no Bearer
            .header("Content-Type", "application/json")
            .body(body.to_owned())
            .send()
            .await
            .map_err(TransportError::from)?;

        let status = resp.status().as_u16();
        // C-3: the docs do not promise this header. Read it if present, otherwise fall back.
        let retry_after = resp
            .headers()
            .get(reqwest::header::RETRY_AFTER)
            .and_then(|v| v.to_str().ok())
            .and_then(|v| v.parse::<u64>().ok());
        let body = resp.text().await.map_err(TransportError::from)?;

        Ok(RawResponse { status, retry_after, body })
    }
    // get_json is the same minus the body and Content-Type
}
```

### Error classification — a pure function, fully testable

```rust
// Covers every type documented at
//   https://api.nomi.ai/docs/reference/post-v1-rooms-id-chat/
//   https://api.nomi.ai/docs/reference/post-v1-nomis-id-chat/
//   https://api.nomi.ai/docs/reference/general
#[derive(Debug, serde::Deserialize)]
struct ErrorEnvelope { error: ErrorBody }
#[derive(Debug, serde::Deserialize)]
struct ErrorBody { #[serde(rename = "type")] kind: String }

pub fn classify(status: u16, body: &str, retry_after: Option<u64>) -> NomiError {
    if (200..300).contains(&status) { return NomiError::None; }

    let kind = serde_json::from_str::<ErrorEnvelope>(body)
        .map(|e| e.error.kind)
        .unwrap_or_else(|_| format!("Http{status}"));

    match kind.as_str() {
        // Not our fault, will pass: retry next tick / next day.
        "RoomStillCreating" | "NomiNotReady" | "NomiStillResponding"
        | "OngoingVoiceCallDetected" | "NoReply"       => NomiError::Transient(kind),
        "TooManyRequests"                              => NomiError::RateLimited { retry_after },
        // Our fault: the message was too long. Both spellings — see C-2.
        "MessageCharacterLimitExceeded"
        | "MessageLengthLimitExceeded"                 => NomiError::TooLong,
        // User must act: bad key, wrong target, plan too small, quota gone.
        "Unauthorized" | "InvalidAPIKey"
        | "NomiNotFound" | "RoomNotFound"
        | "InsufficientPlan" | "LimitExceeded"         => NomiError::Configuration(kind),
        // Our bug: malformed request.
        "InvalidBody" | "InvalidContentType"
        | "InvalidRouteParams"                         => NomiError::Client(kind),
        _ if status >= 500                             => NomiError::Transient(kind),
        _                                              => NomiError::Unknown(kind),
    }
}
```

### The due check — a pure function, no clock, no pool

```rust
/// D-11/D-12/NOMI-03. `now_local` is derived by the caller from an INJECTED `now_utc`
/// and the household timezone — never from `Utc::now()` inside (see report.rs:69-74).
pub fn is_due(
    send_time: chrono::NaiveTime,
    last_attempt_date: Option<chrono::NaiveDate>,
    now_local: chrono::NaiveDateTime,
) -> bool {
    let today = now_local.date();
    let already_today = last_attempt_date.is_some_and(|d| d >= today);
    // `>=`, not `==`: the tick drifts (background_jobs.rs:85-86) and the exact minute is missable.
    !already_today && now_local.time() >= send_time
}
```

---

## State of the Art

| Old approach                                   | Current approach                                                    | When changed             | Impact here                                                                                   |
| ---------------------------------------------- | ------------------------------------------------------------------- | ------------------------ | ----------------------------------------------------------------------------------------------- |
| `reqwest` `default-tls` = native-tls (OpenSSL) | `reqwest 0.13` `default-tls` = rustls + aws-lc-rs + platform-verifier | reqwest 0.13.0 (2026)    | Pin 0.12 and select `rustls-tls` explicitly, or you inherit an aws-lc-rs native build.        |
| `Aes256Gcm::generate_key(OsRng)` (aes-gcm 0.10) | `Key::<Aes256Gcm>::generate()` via the new `Generate` trait (0.11)   | aes-gcm 0.11.0, 2026-06-28 | Any 0.10-era snippet found online will not compile against 0.11 and vice versa. Pin one.      |
| `async-trait` for every async trait            | Native `async fn` in traits (stable since Rust 1.75)                | Rust 1.75, 2023-12       | The transport seam needs no `async-trait` if it is used with a generic bound instead of `dyn`. |
| nomi direct-chat limit 400 free / 600 sub      | 400 free / **800** sub; rooms flat 800                              | some time after Mar 2024 | The 600 figure survives in tweets and search snippets. D-17's "runtime constraint" is right.   |

**Deprecated / not applicable here:**

- `cargo sqlx prepare` and `SQLX_OFFLINE` — documented in CLAUDE.md:49,66 but the backend has zero
  `query!` macros and no `.sqlx/`. No prepare step is needed for the new migration.
- `AuthenticatedUser` extractor — documented in CLAUDE.md:44 / STRUCTURE.md:49, does not exist.

---

## Contradictions to Escalate

Per CLAUDE.md § Design-Driven Implementation ("Stop on contradictions… PAUSE… ask the user"),
these are surfaced rather than resolved.

| #       | Contradiction                                                                                                                                                                     | Evidence                                                                                                                          | Suggested escalation                                                                                                              |
| ------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------- |
| **C-1** | D-16 specifies the German counter `… und N weitere`, but the report is English-only by phase 2.1's locked D-01, which `report.rs` treats as binding.                              | `report.rs:1-5`, `:18-23`, `:312`; 05-CONTEXT.md D-16                                                                             | Ask the user: `… and N more` (consistent), `… und N weitere` (literal), or localized per user language (breaks LLM-stable text)?  |
| **C-2** | D-18 lists `MessageCharacterLimitExceeded` as the length error. The official docs use that name **only for rooms**; the direct-Nomi endpoint returns `MessageLengthLimitExceeded`. | `…/reference/post-v1-rooms-id-chat/` vs `…/reference/post-v1-nomis-id-chat/`                                                      | No user decision needed — handle both spellings. Recorded so the discrepancy with D-18 is not read as an implementation deviation. |
| **C-3** | D-18 and NOMI-05 require honouring `Retry-After`. The official rate-limit documentation specifies only `429` + `{"error":{"type":"TooManyRequests"}}`; no `Retry-After` header.    | `…/reference/general` § Rate Limits                                                                                               | Implement "read `Retry-After` if present, else a fixed backoff". Flag that NOMI-05's acceptance criterion cannot be met literally. |
| **C-4** | D-04 wants "one settings section", but no per-user-per-household settings surface exists. Household settings are the admin page; user settings have no household context.          | `pages/settings.rs` (74 lines, no household), `pages/user_settings.rs` (language only), `pages/household_settings.rs:266` role gate | Recommend an ungated `Card` on `household_settings.rs`. Confirm the placement before building.                                     |
| **C-5** | The research brief states frontend i18n strings are "in Rust, not JSON". They are in JSON.                                                                                        | `i18n/mod.rs:48-55` `include_str!("../translations/{de,en}.json")`; 726 keys in `en.json`                                          | No decision needed; correcting the record so the planner budgets JSON edits + a key-presence test.                                 |
| **C-6** | STRUCTURE.md and CLAUDE.md describe `AuthenticatedUser` and `cargo sqlx prepare`; neither exists.                                                                                 | `middleware/auth.rs:20`; no `query!` macros; no `.sqlx/`                                                                          | Worth a one-line doc fix in a later quick task; do not let it drive plan tasks.                                                    |

---

## Open Questions

1. **Attempt-per-day vs in-run retry.**
   - What we know: D-18 says handle failures without crashing; retry shape is Claude's discretion.
   - What's unclear: whether a transient `RoomStillCreating` should cost the whole day's report.
   - Recommendation: 2 attempts with a 30 s gap inside the same tick, then latch `last_attempt_date`.
     Cheap, bounded, and it covers the documented "several seconds" transient states.

2. **What happens when decryption fails.**
   - What we know: the key can change (host rebuilt, secret rotated, state dir wiped).
   - What's unclear: whether the connection should be auto-disabled or just error every day.
   - Recommendation: set `last_error = "encryption key changed — please re-enter your API key"`,
     leave `enabled` alone, and show it in the UI (D-19 already provides the surface).

3. **Whether `enabled` and "configured" are the same thing.**
   - The schema above lets a connection be `enabled = 1` with `api_key_encrypted IS NULL`.
   - Recommendation: the service refuses to enable without a key and a target; the check lives in
     one place and is unit-tested.

4. **Target-list caching (explicitly Claude's discretion, D-05).**
   - Recommendation: fetch live when the user presses "Load targets", cache `target_name` in the
     row for display. No background refresh; a renamed Nomi shows a stale name until the next load,
     which is acceptable and avoids a second scheduled job.

---

## Environment Availability

Probed 2026-07-28 on this machine.

| Dependency               | Required by                        | Available                | Version                    | Fallback                                                        |
| ------------------------ | ---------------------------------- | ------------------------ | -------------------------- | ----------------------------------------------------------------- |
| `cargo` / rustc          | everything                         | ✓ (devShell only)        | cargo 1.93.1 (2025-12-15)  | —                                                                |
| `node`                   | GSD tooling                        | ✓ (devShell only)        | present                    | —                                                                |
| `sqlite3`                | manual DB inspection               | ✓ (devShell only)        | present                    | `sqlx-cli`                                                       |
| `sqlx-cli`               | migration scaffolding (optional)   | ✓ (devShell only)        | present                    | write the `.sql` by hand — the backend runs `sqlx::migrate!` at `main.rs:40` |
| `jj`                     | commits                            | ✓ (bare PATH too)        | jj 0.41.0                  | —                                                                |
| `openssl` CLI            | generating the new key on the host | ✗ on bare PATH, ✓ in the Nix build inputs and in `module.nix` (`${pkgs.openssl}/bin/openssl`, `:144`) | — | `head -c 32 /dev/urandom \| base64` |
| `curl`                   | manual API probing                 | ✓ (bare PATH)            | curl 8.21.0                | —                                                                |
| Network egress to `api.nomi.ai` | the whole feature           | ✓ (docs fetched during this research) | —                          | —                                                                |
| A live nomi.ai API key   | end-to-end verification            | ✗ (user has a subscription; the key is not in the repo or CI) | — | manual verification step, see § Validation Architecture           |

**Missing dependencies with no fallback:** none — nothing blocks execution.

**Missing dependencies with fallback:**
- `openssl` on the bare PATH: only needed to mint the deployment secret; the Nix module already
  provides it, and `/dev/urandom` + `base64` works anywhere.

**Toolchain constraint (repeat, because it bites):** `cargo`, `node`, `sqlite3`, `sqlx` and
`gsd-tools` are **not** on the bare system PATH. Every command in the plan must be written as
`nix develop -c <command>`. A bare `cargo test` failing is a PATH problem, not a code problem.

**Pre-existing clippy debt (out of scope, do not "fix"):** `nix develop -c cargo clippy -p backend
--all-targets` exits 0 as of 2026-07-28. `cargo clippy --workspace` does **not**, because of ~61
pre-existing frontend findings across ~20 files, including
`frontend/src/components/solo_mode_banner.rs:66` (`clippy::type_complexity`). Note the distinction:
the `-D warnings` in `Cargo.toml:52-53` is `[workspace.lints.rust]`, i.e. **rustc** warnings — so
`cargo check --workspace` is clean and is a valid gate; only `clippy --workspace` is not.

---

## Validation Architecture

### Test Framework

| Property           | Value                                                                                                              |
| ------------------ | -------------------------------------------------------------------------------------------------------------------- |
| Framework          | Rust built-in `#[test]` + `#[tokio::test]` (tokio 1 with `features = ["full"]`, `Cargo.toml:30`). No external harness. |
| Config file        | none — `#[cfg(test)] mod tests` blocks inline, per CONVENTIONS.md:105                                              |
| Shared fixtures    | `backend/src/test_utils.rs` — `create_test_pool()` (in-memory SQLite, `:19`), `create_test_household` (`:471`), `create_test_user` (`:508`), `create_test_membership` (`:530`), `create_test_task().with_*().build()` (`:764`), `set_household_timezone` (`:975`), `insert_missed_task_penalty` (`:1110`) |
| Quick run command  | `nix develop -c cargo test -p backend --lib`  (measured: 303 passed, 1 ignored, **2.67 s**)                         |
| Full suite command | `nix develop -c cargo test --workspace`  (backend 303 + frontend 166 + shared 67)                                   |
| Frontend command   | `nix develop -c cargo test -p frontend --lib` (measured: **166 passed**, 0.01 s — includes the i18n key-presence tests and the CSS-contract tests; `#[wasm_bindgen_test]` blocks also execute natively) |
| Lint gate          | `nix develop -c cargo clippy -p backend --all-targets` (exits 0 today) + `nix develop -c cargo check --workspace`   |

### Phase Requirements → Test Map

| Req ID      | Behavior                                                    | Test type   | Automated command                                                                     | File exists?                       |
| ----------- | ------------------------------------------------------------- | ----------- | --------------------------------------------------------------------------------------- | ---------------------------------- |
| NOMI-01     | Create/read/update a connection round-trips per (hh, user)  | unit (db)   | `nix develop -c cargo test -p backend --lib nomi_connections::`                        | ❌ Wave 0 (`services/nomi_connections.rs`) |
| NOMI-01     | Two users in the same household hold independent connections | unit (db)   | `… --lib nomi_connections::tests::two_users_are_independent`                            | ❌ Wave 0                           |
| NOMI-01     | de/en strings exist for every new UI key                    | unit        | `nix develop -c cargo test -p frontend --lib i18n::tests::test_nomi_keys`               | ✅ pattern at `i18n/mod.rs:102-116` |
| NOMI-01     | No undefined `form-*` / `modal-*` class in the new markup   | unit        | `nix develop -c cargo test -p frontend --lib css_contract::`                            | ✅ `components/css_contract.rs`     |
| NOMI-02     | `open(seal(k, s)) == s`, and `seal` output ≠ plaintext      | unit        | `… --lib crypto::tests::seal_open_roundtrip`                                             | ❌ Wave 0 (`services/crypto.rs`)    |
| NOMI-02     | Two seals of the same plaintext differ (fresh nonce)        | unit        | `… --lib crypto::tests::nonce_is_fresh_per_call`                                         | ❌ Wave 0                           |
| NOMI-02     | A wrong key fails to decrypt rather than returning garbage  | unit        | `… --lib crypto::tests::wrong_key_fails`                                                 | ❌ Wave 0                           |
| NOMI-02     | The stored BLOB does not contain the plaintext as a substring | unit (db)   | `… --lib nomi_connections::tests::stored_blob_has_no_plaintext`                          | ❌ Wave 0                           |
| NOMI-02     | The serialized `NomiConnection` DTO never carries the key   | unit        | `… -p shared --lib types::tests::nomi_connection_json_has_no_key`                        | ❌ Wave 0 (`shared/src/types.rs`)   |
| NOMI-03     | `is_due` is false before the send time, true at/after it    | unit (pure) | `… --lib nomi::tests::is_due_*`                                                          | ❌ Wave 0 (`services/nomi.rs`)      |
| NOMI-03     | `is_due` is false once `last_attempt_date` is today         | unit (pure) | `… --lib nomi::tests::is_due_latches_for_the_day`                                        | ❌ Wave 0                           |
| NOMI-03     | The send time resolves in the household tz, not UTC (Berlin vs UTC vs a negative-offset zone, plus a DST boundary) | unit (db) | `… --lib background_jobs::tests::nomi_send_uses_household_timezone`                      | ❌ Wave 0; helper `set_household_timezone` at `test_utils.rs:975` ✅ |
| NOMI-03     | A pinned `now_utc` past the send time produces exactly one POST to the right URL | unit (fake transport) | `… --lib background_jobs::tests::nomi_send_posts_once`                                   | ❌ Wave 0                           |
| NOMI-03     | The `Authorization` header is the raw key, no `Bearer ` (D-10) | unit (fake transport) | `… --lib nomi::tests::authorization_header_is_raw`                                       | ❌ Wave 0                           |
| NOMI-04     | Under the limit → text unchanged                            | unit (pure) | `… --lib report::tests::capped_report_under_limit_is_identical`                          | ❌ Wave 0 (extends `report.rs` tests) |
| NOMI-04     | Over the limit → shortened, counter present, "Missed yesterday" retained | unit (pure) | `… --lib report::tests::capped_report_keeps_missed_section`                              | ❌ Wave 0                           |
| NOMI-04     | Multi-byte titles are counted as characters, not bytes      | unit (pure) | `… --lib report::tests::capped_report_counts_chars_not_bytes`                            | ❌ Wave 0                           |
| NOMI-04     | A household name alone exceeding the limit does not panic   | unit (pure) | `… --lib report::tests::capped_report_degenerate_header`                                 | ❌ Wave 0                           |
| NOMI-04     | The OOC-wrapped message never exceeds the limit             | unit (pure) | `… --lib nomi::tests::wrapped_message_respects_limit`                                    | ❌ Wave 0                           |
| NOMI-05     | Each documented error string maps to the right `NomiError`  | unit (pure) | `… --lib nomi::tests::classify_*` (one case per type in the § API Facts table)           | ❌ Wave 0                           |
| NOMI-05     | Both length-error spellings map to `TooLong` (C-2)          | unit (pure) | `… --lib nomi::tests::classify_both_length_error_spellings`                              | ❌ Wave 0                           |
| NOMI-05     | 429 with and without `Retry-After` both classify (C-3)      | unit (pure) | `… --lib nomi::tests::classify_rate_limited_without_header`                              | ❌ Wave 0                           |
| NOMI-05     | Connection A failing still lets connection B send (D-13)    | unit (db + fake transport) | `… --lib background_jobs::tests::one_failure_does_not_abort_the_run`                     | ❌ Wave 0                           |
| NOMI-05     | A failure writes `last_error` and still latches the day (D-19) | unit (db)   | `… --lib background_jobs::tests::failure_records_last_error`                             | ❌ Wave 0                           |
| NOMI-06     | `deliver` takes the message text and knows nothing of reports | unit (fake transport) | `… --lib nomi::tests::deliver_is_content_agnostic`                                       | ❌ Wave 0                           |
| NOMI-07     | `chat_url` produces `/v1/nomis/{u}/chat` and `/v1/rooms/{u}/chat` | unit (pure) | `… --lib nomi::tests::chat_url_*`                                                        | ❌ Wave 0                           |
| NOMI-07     | Both list responses deserialize (fixtures copied from the docs) | unit (pure) | `… --lib nomi::tests::parse_nomis_list`, `parse_rooms_list`                              | ❌ Wave 0                           |
| NOMI-07     | The delivery path has one POST call site, not two           | unit (fake transport) | `… --lib nomi::tests::room_and_nomi_share_one_call_site`                                 | ❌ Wave 0                           |

### How the outbound HTTP gets tested without hitting nomi.ai

**There is no existing precedent in this codebase** — `grep -rn "reqwest\|awc::\|ureq\|hyper::"
backend/src` returns nothing, and no mocking crate (`wiremock`, `mockito`, `httpmock`, `mockall`)
appears anywhere in `Cargo.lock`. So this is a fresh decision.

**Recommended: the `NomiTransport` trait seam with an in-test fake.** Zero new crates, no ports,
deterministic, and it is the only way to assert on the *request* (the raw `Authorization` value,
the `Content-Type`, the exact URL, the exact JSON body) rather than merely on the response.

```rust
#[cfg(test)]
struct FakeTransport {
    responses: std::cell::RefCell<std::collections::VecDeque<RawResponse>>,
    calls: std::cell::RefCell<Vec<(String, String, String)>>,   // (url, api_key, body)
}
```

(`RefCell` needs `Mutex` instead if the future must be `Send`; with `#[tokio::test]` on a
current-thread runtime and a generic — not `dyn` — bound, `RefCell` is usually fine. If the
compiler disagrees, swap to `std::sync::Mutex`; do not fight it.)

**Explicitly not recommended: `wiremock 0.6.5`.** It would work and it would exercise the real
reqwest code path, but it adds ~10 dev crates to test ~20 lines whose only real failure modes
(wrong header, wrong content type, wrong URL) the fake already covers — and covers *better*,
because the fake records the request.

**What genuinely needs manual verification:**

1. **One real send to a real Nomi and to a real Room**, using the user's actual subscription key.
   Nothing automated can confirm that the raw-key header is accepted by the live service, that the
   OOC wrapper renders as intended in the Nomi chat, or that 800 is still the current limit. This
   is the phase's human checkpoint.
2. **The NixOS deployment path** — that `nomiEncryptionKeyFile` (or the auto-generated
   `${stateDir}/nomi-encryption-key`) actually reaches the process, that both `EnvironmentFile`
   entries coexist, and that outbound TLS works under `ProtectSystem = "strict"` + `DynamicUser`.
   Only reproducible on the deployment host.
3. **Visual placement of the settings section** on `/households/:id/settings`, mobile-first
   layout, and that the API key field never shows a value (D-09).
4. **A member who is not an admin can configure their own connection** — the role-gating question
   from C-4.

### Sampling Rate

- **Per task commit:** `nix develop -c cargo test -p backend --lib` (2.67 s) — or
  `cargo test -p frontend --lib` (0.01 s) for frontend tasks.
- **Per wave merge:** `nix develop -c cargo test --workspace` **and**
  `nix develop -c cargo clippy -p backend --all-targets` **and**
  `nix develop -c cargo check --workspace`.
- **Phase gate:** full suite green, backend clippy at 0, workspace check clean, then the four
  manual items above, then `/gsd-verify-work`.

### Wave 0 Gaps

- [ ] `backend/migrations/20240150000000_nomi_connections.sql` — the new table
- [ ] `backend/src/test_utils.rs::create_test_schema` — **the same `CREATE TABLE`, written a second
      time** (`run_migrations` does not run migrations, `test_utils.rs:26-30`)
- [ ] `backend/src/test_utils.rs` — `create_test_nomi_connection(...)` fixture, in the style of
      `insert_missed_task_penalty` (`:1110`)
- [ ] `backend/src/services/crypto.rs` — new module + `mod.rs` registration; covers NOMI-02
- [ ] `backend/src/services/nomi.rs` — `NomiTarget`, `NomiError`, `NomiTransport`, `FakeTransport`
      (in `#[cfg(test)]`), the five pure functions; covers NOMI-03..07
- [ ] `backend/src/services/nomi_connections.rs` — CRUD; covers NOMI-01
- [ ] `backend/src/models/nomi_connection.rs` — `NomiConnectionRow` + `to_shared()` + `mod.rs`
- [ ] `backend/src/handlers/nomi.rs` — three routes + registration in `handlers/households.rs:30-44`
- [ ] `backend/src/config.rs` — `nomi_encryption_key: Option<String>` + a case in the existing
      `clear_env()` helper (`:56-66`), or `test_config_defaults` will see a leaked env var
- [ ] `shared/src/types.rs` — `NomiConnection`, `UpdateNomiConnectionRequest`, `NomiTarget`,
      `NomiTargetKind`, `NomiTargetsResponse`
- [ ] `frontend/src/translations/{en,de}.json` + a `test_nomi_keys_present_in_both_languages` in
      `frontend/src/i18n/mod.rs`
- [ ] `frontend/styles.css` — only if any new `form-*` / `modal-*` class is introduced (the
      css_contract test will fail loudly otherwise)
- [ ] Dependency additions: `aes-gcm`, `reqwest`, `base64` in `Cargo.toml` +
      `backend/Cargo.toml`, then `nix develop -c cargo check --workspace` to refresh `Cargo.lock`
      (no Nix hash update needed — `default.nix:27-29` and `flake.nix:40-42` read the lockfile)
- [ ] No test framework install needed.

---

## Sources

### Primary (HIGH confidence)

**This repository** — read directly, 2026-07-28:

- `Cargo.toml`, `backend/Cargo.toml`, `Cargo.lock` — full dependency inventory
- `backend/src/config.rs`, `main.rs`, `models/mod.rs`, `middleware/auth.rs`
- `backend/src/services/{background_jobs,scheduler,report,household_settings,user_settings,auth,households}.rs`
- `backend/src/handlers/{mod,households,report,users}.rs`
- `backend/src/models/{user_settings,mod}.rs`, `backend/src/test_utils.rs`
- `backend/migrations/` (46 files; `20240101000000_initial.sql`, `20240121000000_user_settings.sql`,
  `20240125000000_missed_task_tracking.sql`, `20240128000000_user_dashboard_tasks.sql` read in full)
- `frontend/src/{app.rs, i18n/mod.rs, api/mod.rs}`,
  `frontend/src/pages/{settings,user_settings,household_settings,mod}.rs`,
  `frontend/src/components/{mod,css_contract,time_input,household_tabs}.rs`,
  `frontend/src/translations/en.json`
- `flake.nix`, `default.nix`, `module.nix`, `.planning/{ROADMAP,REQUIREMENTS,STATE,config.json}`,
  `.planning/codebase/{STRUCTURE,CONVENTIONS}.md`, `CLAUDE.md`

**Measured in the devShell**, 2026-07-28:

- `nix develop -c cargo --version` → cargo 1.93.1 (083ac5135 2025-12-15)
- `nix develop -c cargo test -p backend --lib` → 303 passed, 0 failed, 1 ignored, 2.67 s
- `nix develop -c cargo test -p frontend --lib` → 166 passed, 0 failed, 0.01 s
- Tool probes for `sqlite3`, `jj`, `node`, `cargo`, `sqlx`, `openssl`, `curl` (bare PATH vs devShell)

**Official nomi.ai documentation** — fetched 2026-07-28:

- <https://api.nomi.ai/docs/> — auth header format (raw key), `GET /v1/nomis` example + response,
  `POST /v1/nomis/:id/chat` example, 401/400 error bodies
- <https://api.nomi.ai/docs/reference/general> — versioning, rate limits (429 + `TooManyRequests`,
  **no `Retry-After`**), response codes, mandatory `Content-Type: application/json`, error envelope
- <https://api.nomi.ai/docs/reference/post-v1-rooms-id-chat/> — 800-char limit, response shape,
  full 7-item error list
- <https://api.nomi.ai/docs/reference/post-v1-nomis-id-chat/> — 400/800 limits, `sentMessage` +
  `replyMessage`, full 10-item error list including `MessageLengthLimitExceeded`, `NomiNotReady`,
  `OngoingVoiceCallDetected`
- <https://api.nomi.ai/docs/reference/get-v1-nomis/> — nomi object schema
- <https://api.nomi.ai/docs/reference/get-v1-rooms/> — room object schema incl. the `status` enum

**crates.io registry API** — queried 2026-07-28:

- `aes-gcm` versions + 0.10.3 dependency list + 0.11.0 metadata (edition 2024, MSRV 1.85, published 2026-06-28)
- `reqwest` versions + full feature tables for 0.12.28 and 0.13.4
- `ureq`, `awc`, `wiremock` latest versions

**docs.rs:**

- <https://docs.rs/awc/3.8.2/awc/struct.Client.html> — `impl !Send` / `impl !Sync` for `awc::Client`
- <https://docs.rs/aes-gcm/0.11.0/aes_gcm/> — the 0.11 API (used only to establish the 0.10→0.11 break)

### Secondary (MEDIUM confidence)

- reqwest 0.12 vs 0.13 TLS defaults: derived from the crates.io feature tables (authoritative for
  what the features resolve to) rather than from a changelog. The *consequence* for the Nix build
  (`aws-lc-rs` needing native build inputs `default.nix` does not provide) is an inference and
  should be confirmed by the first `nix build .#backend` after the dependency lands.
- `awc` not being compilable in the `tokio::spawn`ed task: `awc::Client` is confirmed `!Send` and
  `tokio::spawn` confirmed requires `Send`; the composition is an inference, not a compile test.

### Tertiary (LOW confidence — flagged, not relied upon)

- The "600 character" subscriber limit from an X/Twitter post and derived search snippets.
  Contradicted by the current official reference (800). Recorded only to explain why the number
  circulates.

---

## Metadata

**Confidence breakdown:**

| Area                        | Level  | Reason                                                                                                                                       |
| --------------------------- | ------ | ---------------------------------------------------------------------------------------------------------------------------------------------- |
| Codebase facts              | HIGH   | Every signature, line number and schema quoted from a file read in this session; test counts measured, not assumed.                            |
| nomi.ai API                 | HIGH   | Every claim from `api.nomi.ai/docs` fetched today, including the three corrections to ROADMAP.md/CONTEXT.md.                                   |
| Standard stack (versions)   | HIGH   | Versions and feature tables from the crates.io API today; MSRV cross-checked against the measured cargo 1.93.1.                                |
| Standard stack (fit)        | MEDIUM | The `aes-gcm 0.10` + `reqwest 0.12 rustls-tls` combination has not yet been compiled in this workspace. Lockfile analysis says it fits cleanly; the first `cargo check` is the proof. |
| Nix build impact            | MEDIUM | `importCargoLock` / `cargoLock.lockFile` mean no hash churn — that is certain. That `rustls-tls` needs no new native build inputs is a strong inference, verifiable only by `nix build .#backend`. |
| Architecture patterns       | HIGH   | Every pattern is either already used in this repository (idempotence, timezone injection, thin handlers, per-job error isolation) or is a direct consequence of a verified API fact. |
| Pitfalls                    | HIGH   | Each one is grounded in a specific line of this codebase or a specific line of the nomi docs, not in general lore.                             |
| Frontend placement (C-4)    | MEDIUM | The routes, role gates and component inventory are verified; which surface the user wants is a product decision, not a research finding.       |

**Research date:** 2026-07-28
**Valid until:** ~2026-08-27 for the dependency recommendations (reqwest and aes-gcm are both
mid-transition between major lines; re-check before a long delay). The codebase findings are valid
until the files change. The nomi.ai limits are explicitly volatile — D-17 exists for that reason.

---

*Phase: 05-nomi-ai-daily-report-push*
*Research conducted: 2026-07-28*
