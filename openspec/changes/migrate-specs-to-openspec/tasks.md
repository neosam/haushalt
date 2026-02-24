## 1. Create OpenSpec Spec Directories

- [x] 1.1 Create `openspec/specs/` directories for all 17 capabilities
- [x] 1.2 Copy converted spec files from `openspec/changes/migrate-specs-to-openspec/specs/` to `openspec/specs/`

## 2. Move Reference Documentation

- [x] 2.1 Create `docs/` directory at project root
- [x] 2.2 Move `specs/baseline/constitution/constitution.md` to `docs/constitution.md`
- [x] 2.3 Move `specs/architecture/` to `docs/architecture/`

## 3. Migrate Draft to Change

- [x] 3.1 Create `openspec/changes/offline-support/` directory
- [x] 3.2 Convert `specs/draft/offline-support.md` to OpenSpec change format (proposal.md)

## 4. Update OpenSpec Config

- [x] 4.1 Add project context to `openspec/config.yaml` (tech stack, patterns, conventions)

## 5. Update Project References

- [x] 5.1 Update `CLAUDE.md` to reference new spec paths (`openspec/specs/` instead of `specs/baseline/stories/`)
- [x] 5.2 Update any other files referencing old spec paths

## 6. Cleanup

- [x] 6.1 Verify all specs are correctly migrated by comparing content
- [x] 6.2 Remove old `specs/` directory after verification
- [x] 6.3 Commit migration with jj
