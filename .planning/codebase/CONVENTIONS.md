# Coding Conventions

**Analysis Date:** 2026-07-23

## Naming Patterns

**Files:**
- `snake_case.rs` for all Rust source files
- One file per domain in `handlers/`, `services/`, `models/`
- One file per page/component in `frontend/src/pages/` and `components/`

**Functions:**
- `snake_case` for all functions
- Async functions: no special prefix (most service fns are `async`)
- Handlers: named after the action (e.g., `create_task`, `complete_task`)

**Variables:**
- `snake_case` for variables
- `SCREAMING_SNAKE_CASE` for constants

**Types:**
- `PascalCase` for structs and enums (no `I` prefix)
- Enums: `PascalCase` name, `PascalCase` variants (e.g., `RecurrenceType::Daily`, `Role::Owner`)
- DB row types: `*Row` suffix (`TaskRow`, `UserRow`), converted to shared types via `From`/`into`

## Code Style

**Formatting:**
- Rust standard formatting (`rustfmt` defaults)
- Warnings are denied at the workspace level — code must compile clean

**Linting:**
- `cargo clippy --workspace` must pass with no warnings
- Run: `cargo clippy --workspace`

## Import Organization

**Order:**
1. External crates (`actix_web`, `sqlx`, `serde`, `chrono`, ...)
2. `shared` crate (workspace dependency)
3. Crate-internal modules (`crate::services::...`, `crate::models::...`)

**Grouping:**
- Blank line between groups
- Workspace deps come from `Cargo.toml` `[workspace.dependencies]`

## Error Handling

**Patterns:**
- Services return `Result<T, AppError>` (or domain error types)
- Errors are mapped to HTTP responses at the handler boundary
- Custom error type(s) in backend with `Into<HttpResponse>` / `ResponseError`

**Error Types:**
- Throw/return errors on invalid input, unauthorized access, not-found, invariant violations
- Handlers translate service errors into appropriate HTTP status codes

## Logging

**Framework:**
- `log` + `env_logger` (backend)
- Levels: `error`, `warn`, `info`, `debug`, `trace`

**Patterns:**
- Log at service boundaries and on external/side-effecting operations
- Avoid noisy logging inside hot paths

## Comments

**When to Comment:**
- Explain *why*, not *what*
- Document business rules and non-obvious recurrence/period logic
- Avoid obvious comments

**Doc comments:**
- Use `///` doc comments for public APIs where helpful

**TODO Comments:**
- Format: `// TODO: description`
- Prefer linking to a `.planning/` todo via `/gsd-add-todo`

## Function Design

**Size:**
- Keep functions small and focused (single responsibility)
- Extract helpers for complex logic (e.g., period-bound calculation, streak updates)

**Parameters:**
- Prefer passing `current_date` / time explicitly to functions under test (no global time mocking)

**Return Values:**
- Explicit `Result` returns; early-return guard clauses for authorization checks

## Module Design

**Exports:**
- `pub` functions and types per module; `mod.rs` re-exports the public API
- Each domain is self-contained: handler + service + model triplet

**Shared types:**
- All API request/response types live in `shared/src/types.rs` and are used identically by backend and frontend

## Testing

- Tests live in `#[cfg(test)] mod tests` blocks next to the code (Rust convention)
- Use `backend/src/test_utils.rs` for shared fixtures: `create_test_pool()` (in-memory SQLite), `run_migrations()`, fixture builders (`create_test_task().with_*(...).build()`), and domain assertion helpers (`assert_completion_exists`, `assert_streak`, `assert_period_result`, ...)
- In-memory SQLite per test for isolation; apply migrations per test
- Tests are required for changes (enforced by project quality rules)

---
*Convention analysis: 2026-07-23*
*Update when patterns change*
