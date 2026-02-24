## ADDED Requirements

### Requirement: User Registration
New users SHALL be able to register an account with username, email, and password to access the application.

#### Scenario: Successful registration
- **WHEN** user submits registration with valid username, email, and password (min 8 characters)
- **THEN** account is created
- **THEN** user is automatically logged in
- **THEN** JWT access token and refresh token are issued

#### Scenario: Invalid password
- **WHEN** user submits registration with password less than 8 characters
- **THEN** registration fails with validation error

---

### Requirement: User Login
Registered users SHALL be able to log in with username or email and password.

#### Scenario: Successful login with username
- **WHEN** user submits valid username and password
- **THEN** JWT access token and refresh token are issued

#### Scenario: Successful login with email
- **WHEN** user submits valid email and password
- **THEN** JWT access token and refresh token are issued

#### Scenario: Case insensitive lookup
- **WHEN** user logs in with username/email in different case
- **THEN** login succeeds (lookup is case insensitive)

#### Scenario: Invalid credentials
- **WHEN** user submits incorrect username/email or password
- **THEN** login fails with error message

#### Scenario: Rate limiting
- **WHEN** multiple failed login attempts from same IP address
- **THEN** requests are rate-limited

---

### Requirement: Token Refresh
The system SHALL automatically refresh access tokens to maintain user sessions without re-authentication.

#### Scenario: Automatic token refresh
- **WHEN** access token expires
- **THEN** refresh is triggered automatically
- **THEN** new access token is issued

#### Scenario: Refresh token rotation
- **WHEN** refresh token is used
- **THEN** old refresh token is invalidated
- **THEN** new refresh token is issued

---

### Requirement: User Logout
Authenticated users SHALL be able to log out to terminate their session securely.

#### Scenario: Successful logout
- **WHEN** user logs out
- **THEN** refresh token is invalidated
- **THEN** user is redirected to login page

#### Scenario: Token rejection after logout
- **WHEN** request is made with tokens from logged-out session
- **THEN** request is rejected

---

### Requirement: View Current User Profile
Authenticated users SHALL be able to view their own profile information.

#### Scenario: Retrieve profile
- **WHEN** authenticated user requests their profile
- **THEN** username and email are returned
