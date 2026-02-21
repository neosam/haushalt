# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Build & Development Commands

```bash
# Enter development shell (provides all tools)
nix develop

# Run backend server
cargo run -p backend

# Run frontend dev server (from frontend/ directory)
cd frontend && trunk serve

# Run all tests
cargo test --workspace

# Run single test
cargo test -p backend test_name

# Check for warnings (must pass - workspace denies warnings)
cargo check --workspace
cargo clippy --workspace

# Production builds
nix build .#backend    # or: nix-build
nix build .#frontend   # or: nix-build frontend/default.nix
```

## Architecture Overview

This is a Rust full-stack application with three workspace members:

### shared/
Defines all API types (requests, responses, domain models) used by both backend and frontend. Key types in `types.rs`:
- `Role` enum (Owner, Admin, Member) with permission methods
- `HierarchyType` enum controlling who can manage tasks/rewards
- All request/response structs for the REST API

### backend/
Actix-web server with layered architecture:
- **handlers/**: HTTP endpoints, extract auth via `AuthenticatedUser` extractor
- **services/**: Business logic, each domain has its own service module
- **models/**: Database row types with `*Row` suffix, convert to shared types
- **middleware/**: JWT auth middleware, rate limiting

Database: SQLite with SQLx. Migrations in `backend/migrations/`. Uses `SQLX_OFFLINE=true` for compile-time query checking.

### frontend/
Leptos CSR (client-side rendered) WASM app:
- **pages/**: Full page components
- **components/**: Reusable UI components
- **api/**: `ApiClient` struct wrapping all backend calls
- **i18n/**: Translation system with JSON files in `translations/`

## Key Patterns

**Authentication flow**: JWT access tokens (short-lived) + refresh tokens (rotation on use). Frontend stores in localStorage, auto-refreshes on 401.

**Role-based access**: `Role::can_*` methods define permissions. Backend enforces in handlers, frontend conditionally renders UI.

**Shared types**: Frontend imports from `shared` crate - API types are identical on both sides.

**SQLx offline mode**: Backend compiles against `sqlx-data.json` snapshots. Run `cargo sqlx prepare` after schema changes.

## Specifications

This project uses speckit structure for managing specifications and requirements.

- **specs/baseline/stories/**: User stories with acceptance criteria (e.g., `05-tasks.md`)
- **specs/baseline/constitution/**: Core project principles
- **specs/architecture/**: Technical architecture documentation

When implementing features, check the relevant spec file for acceptance criteria. When adding new features, update or create spec files first.

## Code Quality Requirements

- Project must build without warnings (workspace denies warnings)
- No clippy warnings allowed
- Always include tests for changes
- Always use jujutsu vcs to create commits.  Basically use jj commit -m "commit message"
- When planning, always think about what needs to be updated in the spec.  Which stories, architecture diagrams and constitution parts must be adjusted to reflect the change and the current state of the change.
- Always think about updating the spec.
- Code changes have this flow: Update spec and show spec change to the user.  Then plan the code change and when user approves, do the code change.  Finally commit with jj.