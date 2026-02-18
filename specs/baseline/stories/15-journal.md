# Journal User Stories

## US-JOURNAL-001: Create Journal Entry

**As a** household member
**I want to** create a journal entry
**So that** I can record my thoughts, activities, or experiences

### Acceptance Criteria
- Entry has a title (optional)
- Entry has content (required)
- Entry has a date (defaults to current date, can be backdated)
- `is_shared` flag determines visibility
- Private entries are only visible to creator
- Shared entries are visible to all household members
- Entry records creation timestamp

---

## US-JOURNAL-002: List Journal Entries

**As a** household member
**I want to** see all journal entries I have access to
**So that** I can browse past entries

### Acceptance Criteria
- Returns my personal (private) entries
- Returns shared entries from any household member
- Sorted by entry date (newest first)
- Can filter by date range
- Can filter by author (for shared entries)

---

## US-JOURNAL-003: View Journal Entry

**As a** household member
**I want to** view a specific journal entry
**So that** I can read its contents

### Acceptance Criteria
- Can view own entries (private and shared)
- Can view shared entries from others
- Cannot view other members' private entries

---

## US-JOURNAL-004: Update Journal Entry

**As the** journal entry author
**I want to** edit my entry
**So that** I can correct or add information

### Acceptance Criteria
- Only the author can edit
- Can change title, content, date, and sharing status
- Records last updated timestamp

---

## US-JOURNAL-005: Delete Journal Entry

**As the** journal entry author
**I want to** delete my entry
**So that** I can remove entries I no longer want

### Acceptance Criteria
- Only the author can delete
- Entry is permanently removed

---

## US-JOURNAL-006: Browse Journal by Date

**As a** household member
**I want to** browse journal entries by date
**So that** I can see what happened on specific days

### Acceptance Criteria
- Can view entries for a specific date
- Shows both my private entries and shared entries for that date
- Calendar or date picker interface for navigation
