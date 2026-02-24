## ADDED Requirements

### Requirement: List Chat Messages
Household members SHALL be able to view chat history.

#### Scenario: View messages
- **WHEN** member requests chat messages
- **THEN** messages for the household are returned

#### Scenario: Pagination
- **WHEN** messages are requested with limit and cursor
- **THEN** paginated results are returned

#### Scenario: Feature disabled
- **WHEN** chat_enabled is false in household settings
- **THEN** chat is not available

---

### Requirement: Send Chat Message
Household members SHALL be able to send messages to household chat.

#### Scenario: Send message
- **WHEN** member sends chat message
- **THEN** message is saved to database
- **THEN** message is delivered via WebSocket

#### Scenario: Feature disabled
- **WHEN** chat_enabled is false
- **THEN** sending is not available

---

### Requirement: Edit Chat Message
Message authors SHALL be able to edit their messages.

#### Scenario: Edit own message
- **WHEN** author edits their message
- **THEN** message is updated
- **THEN** edit is broadcast via WebSocket

#### Scenario: Cannot edit others' messages
- **WHEN** user attempts to edit another's message
- **THEN** request is rejected

---

### Requirement: Delete Chat Message
Message authors and Admins SHALL be able to delete messages.

#### Scenario: Author deletes own message
- **WHEN** author deletes their message
- **THEN** soft delete is performed
- **THEN** deletion is broadcast via WebSocket

#### Scenario: Admin deletes any message
- **WHEN** Admin deletes any message
- **THEN** soft delete is performed
- **THEN** deletion is broadcast via WebSocket

---

### Requirement: Real-time Message Delivery
Household members SHALL receive messages in real-time.

#### Scenario: WebSocket connection
- **WHEN** member connects via WebSocket
- **THEN** authenticates with JWT
- **THEN** joins chat room

#### Scenario: Real-time updates
- **WHEN** new message is sent
- **THEN** appears instantly for all members

#### Scenario: Real-time edits
- **WHEN** message is edited
- **THEN** edit appears instantly for all members

#### Scenario: Real-time deletions
- **WHEN** message is deleted
- **THEN** deletion appears instantly for all members

#### Scenario: Keep-alive
- **WHEN** connection is idle
- **THEN** Ping/Pong keeps connection alive
