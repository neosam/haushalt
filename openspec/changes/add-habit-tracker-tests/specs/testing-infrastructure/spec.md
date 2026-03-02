## ADDED Requirements

### Requirement: Test Database Setup
The testing infrastructure SHALL provide utilities for setting up isolated test databases.

#### Scenario: Create test database pool
- **WHEN** test needs database access
- **THEN** fresh SQLite connection pool is created
- **THEN** all migrations are applied automatically
- **THEN** database is in-memory or temporary file

#### Scenario: Clean database between tests
- **WHEN** test completes
- **THEN** database state can be reset
- **THEN** no data leaks between tests

### Requirement: Fixture Creation Helpers
The testing infrastructure SHALL provide helpers for creating test fixtures.

#### Scenario: Create test household
- **WHEN** test needs household context
- **THEN** helper creates household with default settings
- **THEN** returns household ID and metadata

#### Scenario: Create test user
- **WHEN** test needs user context
- **THEN** helper creates user with specified role
- **THEN** can create multiple users with different roles
- **THEN** returns user ID and credentials

#### Scenario: Create test task with defaults
- **WHEN** test needs task fixture
- **THEN** helper creates task with sensible defaults
- **THEN** allows overriding specific fields
- **THEN** returns complete Task object

#### Scenario: Create test membership
- **WHEN** test needs user-household relationship
- **THEN** helper creates membership with specified role
- **THEN** returns membership details

### Requirement: Time Manipulation Helpers
The testing infrastructure SHALL provide utilities for controlling time in tests.

#### Scenario: Set test date
- **WHEN** test needs specific date context
- **THEN** helper allows setting current date
- **THEN** scheduler functions use test date

#### Scenario: Advance time forward
- **WHEN** testing time-dependent behavior
- **THEN** helper can advance time by days/weeks/months
- **THEN** all date calculations reflect new time

### Requirement: Assertion Helpers
The testing infrastructure SHALL provide custom assertions for habit tracker concepts.

#### Scenario: Assert task completion exists
- **WHEN** verifying task was completed
- **THEN** helper checks completion record in database
- **THEN** optionally validates completion status (Pending/Approved/Rejected)

#### Scenario: Assert streak value
- **WHEN** verifying streak calculation
- **THEN** helper checks streak metadata
- **THEN** validates current and best streaks

#### Scenario: Assert period result
- **WHEN** verifying period tracking
- **THEN** helper checks period_result record
- **THEN** validates completion count and period bounds

#### Scenario: Assert points awarded
- **WHEN** verifying point transactions
- **THEN** helper checks user point balance
- **THEN** validates point change amount

### Requirement: Mock Services
The testing infrastructure SHALL provide mock implementations for external dependencies.

#### Scenario: Mock activity logging
- **WHEN** testing service that logs activities
- **THEN** mock captures log calls without database writes
- **THEN** allows verification of logged activities

#### Scenario: Mock background job scheduler
- **WHEN** testing job scheduling
- **THEN** mock captures scheduled jobs
- **THEN** allows immediate execution for testing
