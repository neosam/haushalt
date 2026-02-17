# User Management User Stories

## US-USER-001: View User Profile

**As an** authenticated user
**I want to** view another user's basic profile information
**So that** I can see who I'm collaborating with

### Acceptance Criteria
- User can retrieve username and basic info by user ID
- Sensitive information is not exposed

---

## US-USER-002: Update Own Profile

**As an** authenticated user
**I want to** update my username and email
**So that** I can keep my account information current

### Acceptance Criteria
- User can modify their own username
- User can modify their own email
- User cannot modify other users' profiles
- Changes are persisted immediately

---

## US-USER-003: Get User Settings

**As an** authenticated user
**I want to** view my personal settings
**So that** I can see my current preferences

### Acceptance Criteria
- User can retrieve their language preference
- Default language is returned if not set

---

## US-USER-004: Update User Settings

**As an** authenticated user
**I want to** update my personal settings
**So that** I can customize my experience

### Acceptance Criteria
- User can change their language preference
- Supported languages: English (en), German (de)
- Settings are persisted immediately
