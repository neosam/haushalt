## ADDED Requirements

### Requirement: View User Profile
Authenticated users SHALL be able to view another user's basic profile information.

#### Scenario: View other user's profile
- **WHEN** authenticated user requests profile by user ID
- **THEN** username and basic info are returned
- **THEN** sensitive information is NOT exposed

---

### Requirement: Update Own Profile
Authenticated users SHALL be able to update their own username and email.

#### Scenario: Update username
- **WHEN** user submits new username
- **THEN** username is updated
- **THEN** change is persisted immediately

#### Scenario: Update email
- **WHEN** user submits new email
- **THEN** email is updated
- **THEN** change is persisted immediately

#### Scenario: Cannot modify other users
- **WHEN** user attempts to modify another user's profile
- **THEN** request is rejected

---

### Requirement: Get User Settings
Authenticated users SHALL be able to view their personal settings.

#### Scenario: Retrieve settings
- **WHEN** user requests their settings
- **THEN** language preference is returned

#### Scenario: Default language
- **WHEN** user has not set language preference
- **THEN** default language is returned

---

### Requirement: Update User Settings
Authenticated users SHALL be able to update their personal settings.

#### Scenario: Change language
- **WHEN** user sets language to supported value (en, de)
- **THEN** language preference is updated
- **THEN** change is persisted immediately

#### Scenario: Unsupported language
- **WHEN** user attempts to set unsupported language
- **THEN** request is rejected
