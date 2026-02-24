## ADDED Requirements

### Requirement: Create Announcement
Household Owners and Admins SHALL be able to create announcements.

#### Scenario: Create announcement
- **WHEN** Owner/Admin creates announcement with title and content
- **THEN** announcement is created

#### Scenario: Schedule announcement
- **WHEN** start_date and end_date are set
- **THEN** announcement is visible only during that period

---

### Requirement: List All Announcements
Household Owners and Admins SHALL be able to see all announcements.

#### Scenario: View all announcements
- **WHEN** Owner/Admin requests all announcements
- **THEN** all announcements are returned (including inactive)
- **THEN** start/end dates are shown

---

### Requirement: List Active Announcements
Household members SHALL be able to see currently active announcements.

#### Scenario: View active announcements
- **WHEN** member requests active announcements
- **THEN** only current announcements are returned
- **THEN** where start_date is null or in the past
- **THEN** where end_date is null or in the future

#### Scenario: Sort order
- **WHEN** active announcements are returned
- **THEN** sorted by creation date

---

### Requirement: View Announcement
Household members SHALL be able to view specific announcements.

#### Scenario: View announcement
- **WHEN** member views announcement
- **THEN** title, content, and dates are shown

---

### Requirement: Update Announcement
Household Owners and Admins SHALL be able to edit announcements.

#### Scenario: Update announcement
- **WHEN** Owner/Admin updates announcement
- **THEN** title and content can be modified
- **THEN** start/end dates can be changed

---

### Requirement: Delete Announcement
Household Owners and Admins SHALL be able to delete announcements.

#### Scenario: Delete announcement
- **WHEN** Owner/Admin deletes announcement
- **THEN** announcement is permanently removed
