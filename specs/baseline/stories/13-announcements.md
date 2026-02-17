# Announcements User Stories

## US-ANN-001: Create Announcement

**As a** household Owner or Admin
**I want to** create an announcement
**So that** I can inform household members

### Acceptance Criteria
- Announcement has a title
- Announcement has content
- Optional start date (when it becomes visible)
- Optional end date (when it stops being visible)

---

## US-ANN-002: List All Announcements

**As a** household Owner or Admin
**I want to** see all announcements (including inactive)
**So that** I can manage them

### Acceptance Criteria
- Returns all announcements regardless of dates
- Shows start/end dates

---

## US-ANN-003: List Active Announcements

**As a** household member
**I want to** see currently active announcements
**So that** I'm informed of important information

### Acceptance Criteria
- Returns only announcements where:
  - start_date is null or in the past
  - end_date is null or in the future
- Sorted by creation date

---

## US-ANN-004: View Announcement

**As a** household member
**I want to** view a specific announcement
**So that** I can read its details

### Acceptance Criteria
- Shows title, content, and dates

---

## US-ANN-005: Update Announcement

**As a** household Owner or Admin
**I want to** edit an announcement
**So that** I can update information

### Acceptance Criteria
- Can modify title and content
- Can change start/end dates

---

## US-ANN-006: Delete Announcement

**As a** household Owner or Admin
**I want to** delete an announcement
**So that** I can remove outdated announcements

### Acceptance Criteria
- Announcement is permanently removed
