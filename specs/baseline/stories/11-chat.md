# Chat User Stories

## US-CHAT-001: List Chat Messages

**As a** household member
**I want to** view chat history
**So that** I can read past conversations

### Acceptance Criteria
- Returns messages for the household
- Pagination support (limit, before cursor)
- Only available if chat_enabled in household settings

---

## US-CHAT-002: Send Chat Message

**As a** household member
**I want to** send a message to the household chat
**So that** I can communicate with other members

### Acceptance Criteria
- Message is saved to database
- Message is delivered to other members via WebSocket
- Only available if chat_enabled

---

## US-CHAT-003: Edit Chat Message

**As the** message author
**I want to** edit my sent message
**So that** I can correct mistakes

### Acceptance Criteria
- Only the author can edit
- Other members see the updated message
- Edit is broadcast via WebSocket

---

## US-CHAT-004: Delete Chat Message

**As the** message author or Admin
**I want to** delete a message
**So that** inappropriate content can be removed

### Acceptance Criteria
- Author can delete their own messages
- Admin can delete any message
- Soft delete is performed
- Deletion is broadcast via WebSocket

---

## US-CHAT-005: Real-time Message Delivery

**As a** household member
**I want to** receive messages in real-time
**So that** conversations flow naturally

### Acceptance Criteria
- WebSocket connection is established
- User authenticates with JWT
- User joins the chat room
- New messages appear instantly
- Message edits appear instantly
- Message deletions appear instantly
- Ping/Pong for connection keep-alive
