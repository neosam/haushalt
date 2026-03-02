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

This project uses OpenSpec for managing specifications and changes.

- **openspec/specs/**: Capability specs with BDD-style requirements (e.g., `tasks/spec.md`)
- **openspec/changes/**: Active change proposals (proposal → specs → design → tasks)
- **docs/constitution.md**: Core project principles and domain model
- **docs/architecture/**: Technical architecture documentation

When implementing features:
1. Use `/opsx:propose` to create a change proposal
2. Use `/opsx:apply` to implement tasks from the change
3. Use `/opsx:archive` when implementation is complete

## Coding Principles

### DRY (Don't Repeat Yourself)
- Extract common patterns into reusable functions, components, or utilities
- When you see similar code in multiple places, consider refactoring
- Key areas to watch:
  - API call + refresh patterns in page components
  - Modal state management and form submissions
  - Callback handlers that follow the same async pattern

### Clean Code
- Meaningful names for functions, variables, and types
- Small, focused functions (single responsibility)
- Keep components manageable in size - extract sub-components when they grow
- Self-documenting code preferred over comments

### Spec-Driven Development (SDD)
- Use OpenSpec workflow for changes:
  1. `/opsx:propose` - Create change with proposal, specs, design, tasks
  2. `/opsx:apply` - Implement tasks from the change
  3. `/opsx:archive` - Archive completed change
  4. Commit with jj

### Design-Driven Implementation (CRITICAL)

**The design document is the source of truth.** Always follow the design document exactly when implementing tasks.

#### Before Each Task:
1. **Read design.md** - Re-read the relevant section for the task you're implementing
2. **Identify the pattern** - What specific code pattern or architecture does the design describe?
3. **Document your understanding** - Mentally note: "Design says: [pattern/approach]"

#### During Implementation:
4. **Follow the design pattern exactly** - Do NOT adapt to existing code if it contradicts the design
5. **Stop on contradictions** - If existing code differs from the design document:
   - PAUSE implementation
   - Ask the user: "Existing code uses [X], but design.md specifies [Y]. How should I proceed?"
6. **No assumptions** - Do not assume existing patterns are correct

#### After Implementation:
7. **Verify against design** - Before marking a task as complete:
   - Does my code match the design pattern described?
   - Does it use the exact approach specified (e.g., `create_memo` not inline logic)?
8. **Document verification** - Add a comment in your response: "✓ Verified: Uses [pattern] as specified in design.md"

#### When Design is Missing or Unclear:
- STOP and ask the user before implementing
- Do NOT fill in gaps with assumptions
- Say: "Design doesn't specify [X]. Should I [option A] or [option B]?"

#### Example (Correct):
```
Task: "Implement filtering logic"
Design says: "create a Memo that filters tasks reactively"
Implementation: ✓ Uses create_memo(move |_| { ... })
Verification: ✓ Matches design pattern exactly
```

#### Example (Incorrect - Do NOT do this):
```
Task: "Implement filtering logic"
Design says: "create a Memo"
Implementation: ✗ Used inline filtering in view because existing code did it that way
Problem: Ignored design, followed existing code instead
```

**Remember**: Design documents exist to prevent mistakes. Following them strictly prevents bugs and rework.

### Mobile First (CSS)
- Write base styles for mobile screens first
- Use `min-width` media queries to add desktop enhancements
- Example:
  ```css
  /* Base mobile styles */
  .grid { grid-template-columns: 1fr; }

  /* Desktop enhancement */
  @media (min-width: 768px) {
    .grid { grid-template-columns: repeat(3, 1fr); }
  }
  ```
- **Note**: Current CSS uses Desktop First (`max-width`) - migration needed

## Code Quality Requirements

- Project must build without warnings (workspace denies warnings)
- No clippy warnings allowed
- Always include tests for changes
- Always use jujutsu vcs to create commits.  Basically use jj commit -m "commit message"
- Use OpenSpec workflow for all changes: `/opsx:propose` → `/opsx:apply` → `/opsx:archive`
- Check `openspec/specs/` for existing capability requirements before implementing
- If openspec is not in the path, try using nix develop because it is mentioned in the flake.