## Context

The project has an established spec-driven workflow with:
- `specs/baseline/stories/*.md` - 17 user story files with acceptance criteria
- `specs/baseline/constitution/constitution.md` - Domain model, architecture, coding principles
- `specs/architecture/*.md` - Mermaid diagrams for system components
- `specs/draft/offline-support.md` - Work-in-progress feature spec

OpenSpec expects:
- `openspec/specs/<capability>/spec.md` - BDD-style requirements with WHEN/THEN scenarios
- `openspec/changes/<name>/` - Change artifacts (proposal, design, specs, tasks)
- `openspec/config.yaml` - Project context for AI assistance

## Goals / Non-Goals

**Goals:**
- Convert all 17 user story files to OpenSpec BDD format
- Establish `openspec/specs/` as the source of truth for requirements
- Move reference documentation (constitution, architecture) to `docs/`
- Enable the OpenSpec change workflow for future development

**Non-Goals:**
- Changing any application code (this is spec reorganization only)
- Adding new requirements during migration (preserve existing behavior)
- Automated conversion tooling (manual conversion ensures quality)

## Decisions

### 1. File Structure

```
openspec/
├── specs/
│   ├── authentication/spec.md
│   ├── tasks/spec.md
│   └── ... (17 capabilities)
├── changes/
│   ├── archive/
│   └── offline-support/     ← migrated from draft
└── config.yaml

docs/
├── constitution.md          ← moved from specs/baseline/
└── architecture/            ← moved from specs/
    ├── 01-system-overview.md
    └── ...
```

**Rationale:** Clear separation between requirements (openspec/specs), active work (openspec/changes), and reference documentation (docs/).

### 2. Conversion Strategy

| User Story Element | OpenSpec Equivalent |
|-------------------|---------------------|
| `US-XXX-NNN: Title` | `### Requirement: Title` |
| `As a... I want... So that...` | Requirement description prose |
| Acceptance Criteria bullets | `#### Scenario:` with WHEN/THEN |
| Design Decisions | Keep inline under requirement OR move to docs |
| Implementation Notes | Remove (implementation detail, not spec) |

**Rationale:** Direct mapping preserves intent while adopting BDD structure.

### 3. Handling Design Decisions

Some user stories contain "Design Decisions" sections that document architectural choices. Options:
- **A) Keep inline**: Add as notes under the requirement
- **B) Move to docs**: Extract to architecture documentation
- **C) Discard**: If already captured in constitution

**Decision:** Option A for UI/UX decisions that affect requirements, Option B for architectural decisions.

### 4. Normative Language

Convert descriptive language to normative:
- "Can set recurrence type" → "The system SHALL allow setting recurrence type"
- "Shows completion status" → "The system SHALL display completion status"

**Rationale:** Normative language (SHALL/MUST) makes requirements unambiguous and testable.

## Risks / Trade-offs

**[Risk] Loss of context during conversion** → Mitigation: Review each converted spec against original before deleting old files

**[Risk] Broken references in CLAUDE.md** → Mitigation: Update CLAUDE.md to reference new paths after migration

**[Risk] Large PR/commit** → Mitigation: Can split into phases (move files first, convert format second) but adds complexity

**[Trade-off] Manual conversion is time-consuming** → But ensures quality and understanding of each requirement

## Migration Plan

1. **Phase 1: Create OpenSpec spec files** - Convert each story file to BDD format
2. **Phase 2: Move reference docs** - Constitution and architecture to `docs/`
3. **Phase 3: Migrate draft** - Convert `offline-support.md` to a change
4. **Phase 4: Update config** - Add project context to `config.yaml`
5. **Phase 5: Update references** - Fix paths in CLAUDE.md
6. **Phase 6: Remove old specs** - Delete `specs/` directory after verification

**Rollback:** Keep old `specs/` directory until migration is verified working
