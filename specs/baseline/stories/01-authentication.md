# Authentication User Stories

## US-AUTH-001: User Registration

**As a** new user
**I want to** register an account with username, email, and password
**So that** I can access the application

### Acceptance Criteria
- User provides username, email, and password
- Password must be at least 8 characters
- User is automatically logged in after successful registration
- JWT access token and refresh token are issued

---

## US-AUTH-002: User Login

> **Status:** Implemented
> **Implemented:** 2026-02-20

**As a** registered user
**I want to** log in with my username or email and password
**So that** I can access my account

### Acceptance Criteria
- User provides username or email address, and password
- Username/email lookup is case insensitive
- JWT access token and refresh token are issued on success
- Failed login attempts are rate-limited by IP address
- Invalid credentials return an error message

---

## US-AUTH-003: Token Refresh

**As an** authenticated user
**I want to** automatically refresh my access token
**So that** I can remain logged in without re-entering credentials

### Acceptance Criteria
- Expired access token triggers automatic refresh
- Refresh token is rotated on each use
- New access token is issued on successful refresh

---

## US-AUTH-004: User Logout

**As an** authenticated user
**I want to** log out of the application
**So that** my session is terminated securely

### Acceptance Criteria
- Refresh token is invalidated on logout
- User is redirected to login page
- Subsequent requests with old tokens are rejected

---

## US-AUTH-005: View Current User Profile

**As an** authenticated user
**I want to** view my own profile information
**So that** I can verify my account details

### Acceptance Criteria
- User can retrieve their own username and email
- Endpoint returns current authenticated user's data
