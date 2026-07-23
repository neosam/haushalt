# Codebase Structure

**Analysis Date:** 2026-07-23

## Directory Layout

```
haushalt/
├── backend/             # Actix-web REST API + WebSocket server
│   ├── src/
│   │   ├── db/          # Database connection & pool
│   │   ├── handlers/    # HTTP endpoints (one file per domain)
│   │   ├── middleware/  # JWT auth, rate limiting
│   │   ├── models/      # DB row types (*Row suffix), convert to shared types
│   │   ├── services/    # Business logic (one file per domain)
│   │   ├── config.rs    # App configuration
│   │   ├── test_utils.rs# Shared test fixtures & assertion helpers
│   │   ├── lib.rs
│   │   └── main.rs      # Server entry point
│   └── migrations/      # SQLx migrations
├── frontend/            # Leptos CSR WASM app
│   ├── src/
│   │   ├── api/         # ApiClient wrapping all backend calls
│   │   ├── components/  # Reusable UI components (one file each)
│   │   ├── i18n/        # Translation system
│   │   ├── pages/       # Full page components (one file per route)
│   │   ├── translations/# JSON translation files (en, de)
│   │   ├── utils/       # Frontend utilities
│   │   ├── app.rs       # Root app component / routing
│   │   └── lib.rs
│   └── index.html
├── shared/              # Shared API types (frontend + backend)
│   └── src/
│       ├── types.rs     # All API types, Role, HierarchyType, request/response structs
│       └── lib.rs
├── docs/                # Architecture docs + constitution
│   ├── architecture/    # 10 architecture docs + README
│   └── constitution.md  # Core domain model & principles
├── .planning/           # GSD planning workspace (source of truth for specs/roadmap)
├── Cargo.toml           # Workspace manifest
├── flake.nix            # Nix build + dev shell
├── module.nix           # NixOS module
└── default.nix          # Backend package
```

## Directory Purposes

**backend/src/handlers/**
- Purpose: HTTP endpoints, extract auth via `AuthenticatedUser` extractor
- Contains: One `.rs` file per domain (auth, tasks, households, chat, websocket, ...)
- Key files: `tasks.rs`, `auth.rs`, `households.rs`, `websocket.rs`

**backend/src/services/**
- Purpose: Business logic; each domain has its own service module
- Contains: One `.rs` file per domain plus cross-cutting (`scheduler.rs`, `period_results.rs`, `task_consequences.rs`, `background_jobs.rs`, `points.rs`, `solo_mode.rs`)
- Key files: `tasks.rs`, `scheduler.rs`, `period_results.rs`, `task_consequences.rs`

**backend/src/models/**
- Purpose: Database row types with `*Row` suffix; convert rows to shared types
- Contains: One `.rs` per entity (`task.rs`, `user.rs`, `household.rs`, ...)

**backend/src/middleware/**
- Purpose: JWT auth middleware and rate limiting
- Key files: `auth.rs`, `rate_limit.rs`

**frontend/src/pages/**
- Purpose: Full page components (one per route)
- Key files: `tasks.rs`, `dashboard.rs`, `household.rs`, `login.rs`, `register.rs`

**frontend/src/components/**
- Purpose: Reusable UI components (one file each, PascalCase component names)
- Contains: `task_card.rs`, `task_modal.rs`, `modal.rs`, `navbar.rs`, `quick_task_fab.rs`, `period_tracker.rs`, ...

**frontend/src/api/**
- Purpose: `ApiClient` struct wrapping all backend REST + WebSocket calls
- Key files: `mod.rs`, `websocket.rs`

**shared/src/**
- Purpose: API types shared identically by backend and frontend
- Key files: `types.rs` (`Role`, `HierarchyType`, all request/response structs)

## Key File Locations

**Entry Points:**
- `backend/src/main.rs`: Server startup
- `frontend/src/lib.rs` / `frontend/src/app.rs`: WASM app entry + routing

**Configuration:**
- `Cargo.toml`: Workspace manifest (backend, frontend, shared members)
- `flake.nix`: Nix build + dev shell
- `.env` / `.env.example`: Runtime env
- `backend/sqlx-data.json`: SQLx offline query data (regenerate with `cargo sqlx prepare`)

**Core Logic:**
- `backend/src/services/`: All business logic
- `backend/src/handlers/`: HTTP layer
- `backend/migrations/`: SQLx migrations

**Testing:**
- `backend/src/test_utils.rs`: Shared test fixtures, builders, assertion helpers
- `#[cfg(test)] mod tests` blocks inside each service file

**Documentation:**
- `docs/constitution.md`: Domain model & principles
- `docs/architecture/`: Technical architecture (01-10)

## Naming Conventions

**Files:** `snake_case.rs` for all Rust source
**Components:** Component structs are `PascalCase`; files are `snake_case.rs`
**Test files:** Inline `#[cfg(test)] mod tests` in the same file as code
**Row types:** `*Row` suffix (e.g., `TaskRow`), converted to shared types

## Where to Add New Code

**New backend domain (e.g., `widgets`):**
- Handler: `backend/src/handlers/widgets.rs` (+ register in `mod.rs`)
- Service: `backend/src/services/widgets.rs` (+ register in `mod.rs`)
- Model: `backend/src/models/widget.rs` (+ register in `mod.rs`)
- Migration: `backend/migrations/NNNN_widget.sql`
- Shared types: add to `shared/src/types.rs`
- Tests: inline `mod tests` in the service file; fixtures via `test_utils.rs`

**New frontend page:**
- Page: `frontend/src/pages/widgets.rs` (+ register in `mod.rs`, wire route in `app.rs`)
- Components: `frontend/src/components/widget_card.rs`
- API calls: extend `frontend/src/api/mod.rs` `ApiClient`

**New reusable component:**
- `frontend/src/components/<name>.rs` (+ register in `mod.rs`)

## Special Directories

**.planning/**
- Purpose: GSD planning workspace (PROJECT, REQUIREMENTS, ROADMAP, STATE, config, codebase map)
- Committed: Yes (`commit_docs: true`)

**docs/architecture/**
- Purpose: Technical architecture documentation (read-only reference)
- Committed: Yes

---
*Structure analysis: 2026-07-23*
*Update when directory structure changes*
