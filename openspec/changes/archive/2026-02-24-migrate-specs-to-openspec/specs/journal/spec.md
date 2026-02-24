## ADDED Requirements

### Requirement: Create Journal Entry
Household members SHALL be able to create journal entries.

#### Scenario: Create private entry
- **WHEN** member creates entry with is_shared = false
- **THEN** entry is only visible to creator

#### Scenario: Create shared entry
- **WHEN** member creates entry with is_shared = true
- **THEN** entry is visible to all household members

#### Scenario: Entry properties
- **WHEN** entry is created
- **THEN** title (optional), content (required), date are stored
- **THEN** creation timestamp is recorded

#### Scenario: Backdate entry
- **WHEN** entry is created with past date
- **THEN** entry date is set to provided date

---

### Requirement: List Journal Entries
Household members SHALL be able to see journal entries they have access to.

#### Scenario: View accessible entries
- **WHEN** member requests entries
- **THEN** personal (private) entries are returned
- **THEN** shared entries from any member are returned
- **THEN** sorted by entry date (newest first)

#### Scenario: Filter by date range
- **WHEN** date range filter is applied
- **THEN** only entries within range are returned

#### Scenario: Filter by author
- **WHEN** author filter is applied
- **THEN** only entries by that author are returned

---

### Requirement: View Journal Entry
Household members SHALL be able to view specific journal entries.

#### Scenario: View own entry
- **WHEN** member views their own entry
- **THEN** entry content is shown

#### Scenario: View shared entry
- **WHEN** member views shared entry from another member
- **THEN** entry content is shown

#### Scenario: Cannot view others' private entries
- **WHEN** member attempts to view another's private entry
- **THEN** access is denied

---

### Requirement: Update Journal Entry
Journal entry authors SHALL be able to edit their entries.

#### Scenario: Update entry
- **WHEN** author updates entry
- **THEN** title, content, date, and sharing status can be changed
- **THEN** last updated timestamp is recorded

#### Scenario: Cannot edit others' entries
- **WHEN** user attempts to edit another's entry
- **THEN** request is rejected

---

### Requirement: Delete Journal Entry
Journal entry authors SHALL be able to delete their entries.

#### Scenario: Delete entry
- **WHEN** author deletes their entry
- **THEN** entry is permanently removed

#### Scenario: Cannot delete others' entries
- **WHEN** user attempts to delete another's entry
- **THEN** request is rejected

---

### Requirement: Browse Journal by Date
Household members SHALL be able to browse entries by date.

#### Scenario: View by date
- **WHEN** member selects specific date
- **THEN** entries for that date are shown
- **THEN** includes private and shared entries

#### Scenario: Calendar navigation
- **WHEN** member uses date picker
- **THEN** can navigate to specific days
