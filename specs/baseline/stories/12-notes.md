# Notes User Stories

## US-NOTE-001: Create Note

**As a** household member
**I want to** create a note
**So that** I can save information for later

### Acceptance Criteria
- Note has a title
- Note has content
- is_shared flag determines visibility
- Personal notes are only visible to creator
- Shared notes are visible to all household members

---

## US-NOTE-002: List Notes

**As a** household member
**I want to** see all notes I have access to
**So that** I can find saved information

### Acceptance Criteria
- Returns personal notes (my own)
- Returns shared notes (from any member)

---

## US-NOTE-003: View Note

**As a** household member
**I want to** view a specific note
**So that** I can read its contents

### Acceptance Criteria
- Can view own notes
- Can view shared notes from others
- Cannot view other members' personal notes

---

## US-NOTE-004: Update Note

**As the** note author
**I want to** edit my note
**So that** I can update information

### Acceptance Criteria
- Only the author can edit
- Can change title, content, and sharing status

---

## US-NOTE-005: Delete Note

**As the** note author
**I want to** delete my note
**So that** I can remove outdated information

### Acceptance Criteria
- Only the author can delete
- Note is permanently removed
