## Why

The project has a mature spec-driven development workflow using a manual structure (`specs/baseline/stories/`, `specs/architecture/`). Migrating to OpenSpec will:
- Enable AI-assisted spec updates through the change workflow (proposal → specs → design → tasks)
- Standardize on BDD-style requirements with WHEN/THEN scenarios for better testability
- Provide tooling for tracking changes and archiving completed work

## What Changes

- **Move** 17 user story files from `specs/baseline/stories/` to `openspec/specs/<capability>/spec.md`
- **Convert** user story format (As a/I want/So that + Acceptance Criteria) to OpenSpec BDD format (Requirements + Scenarios)
- **Move** constitution and architecture docs to `docs/` (outside OpenSpec, reference documentation)
- **Move** draft spec (`offline-support.md`) to `openspec/changes/offline-support/` as a pending change
- **Update** `openspec/config.yaml` with project context
- **Remove** old `specs/` directory after migration is verified

## Capabilities

### New Capabilities

- `authentication`: User registration, login, JWT token management, refresh token rotation
- `user-management`: User profiles, settings, language preferences
- `households`: Household creation, membership, settings, hierarchy types
- `invitations`: Email-based household invitations with role assignment
- `tasks`: Task creation, completion, recurrence, assignment, archiving, pausing, suggestions
- `task-categories`: Categorization of tasks within households
- `rewards`: Points-based rewards, purchasing, random choice rewards
- `punishments`: Consequence system, random choice punishments
- `point-conditions`: Automated point rules for streaks, completions, misses
- `activity-logs`: Immutable activity tracking for household events
- `chat`: Real-time messaging with WebSocket, soft-delete messages
- `notes`: Shared household notes
- `announcements`: Household-wide announcements
- `dashboard`: Personal dashboard with whitelisted tasks across households
- `journal`: Personal journal entries within household context
- `task-period-tracking`: Completion rates, streaks, period calculations
- `household-statistics`: Leaderboards, household-level metrics

### Modified Capabilities

(None - this is initial migration, no existing OpenSpec specs to modify)

## Impact

- **File structure**: New `openspec/specs/` with 17 capability directories; old `specs/` removed
- **Documentation**: `docs/constitution.md` and `docs/architecture/` for reference material
- **Workflow**: Future changes use `/opsx:propose` → `/opsx:apply` → `/opsx:archive`
- **No code changes**: This is a spec reorganization only
