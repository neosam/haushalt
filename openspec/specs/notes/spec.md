## ADDED Requirements

### Requirement: Create Note
Household members SHALL be able to create notes.

#### Scenario: Create personal note
- **WHEN** member creates note with is_shared = false
- **THEN** note is only visible to creator

#### Scenario: Create shared note
- **WHEN** member creates note with is_shared = true
- **THEN** note is visible to all household members

---

### Requirement: List Notes
Household members SHALL be able to see notes they have access to.

#### Scenario: View accessible notes
- **WHEN** member requests notes
- **THEN** personal notes (own) are returned
- **THEN** shared notes (from any member) are returned

---

### Requirement: View Note
Household members SHALL be able to view specific notes.

#### Scenario: View own note
- **WHEN** member views their own note
- **THEN** note content is shown

#### Scenario: View shared note
- **WHEN** member views shared note from another member
- **THEN** note content is shown

#### Scenario: Cannot view others' personal notes
- **WHEN** member attempts to view another's personal note
- **THEN** access is denied

---

### Requirement: Update Note
Note authors SHALL be able to edit their notes.

#### Scenario: Update note
- **WHEN** author updates note
- **THEN** title, content, and sharing status can be changed

#### Scenario: Cannot edit others' notes
- **WHEN** user attempts to edit another's note
- **THEN** request is rejected

---

### Requirement: Delete Note
Note authors SHALL be able to delete their notes.

#### Scenario: Delete note
- **WHEN** author deletes their note
- **THEN** note is permanently removed

#### Scenario: Cannot delete others' notes
- **WHEN** user attempts to delete another's note
- **THEN** request is rejected
