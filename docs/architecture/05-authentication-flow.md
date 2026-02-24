# Authentication Flow

## Registration Flow

```mermaid
sequenceDiagram
    participant U as User
    participant FE as Frontend
    participant BE as Backend
    participant DB as Database

    U->>FE: Fill registration form
    FE->>FE: Validate input
    FE->>BE: POST /api/auth/register
    BE->>BE: Validate request
    BE->>BE: Hash password (bcrypt)
    BE->>DB: Create user
    DB-->>BE: User created
    BE->>BE: Generate JWT access token
    BE->>BE: Generate refresh token
    BE->>DB: Store refresh token hash
    BE-->>FE: {access_token, refresh_token, user}
    FE->>FE: Store tokens in localStorage
    FE-->>U: Redirect to dashboard
```

## Login Flow

```mermaid
sequenceDiagram
    participant U as User
    participant FE as Frontend
    participant RL as Rate Limiter
    participant BE as Backend
    participant DB as Database

    U->>FE: Enter credentials
    FE->>BE: POST /api/auth/login
    BE->>RL: Check rate limit (IP)
    alt Rate limited
        RL-->>BE: Blocked
        BE-->>FE: 429 Too Many Requests
        FE-->>U: Show error
    else Not rate limited
        RL-->>BE: OK
        BE->>DB: Find user by username
        DB-->>BE: User record
        BE->>BE: Verify password hash
        alt Invalid credentials
            BE->>RL: Increment failure count
            BE-->>FE: 401 Unauthorized
            FE-->>U: Show error
        else Valid credentials
            BE->>BE: Generate JWT access token
            BE->>BE: Generate refresh token
            BE->>DB: Store refresh token hash
            BE-->>FE: {access_token, refresh_token, user}
            FE->>FE: Store tokens in localStorage
            FE-->>U: Redirect to dashboard
        end
    end
```

## Token Refresh Flow

```mermaid
sequenceDiagram
    participant FE as Frontend
    participant BE as Backend
    participant DB as Database

    FE->>BE: API Request with access token
    BE->>BE: Validate JWT
    alt Token expired
        BE-->>FE: 401 Unauthorized
        FE->>BE: POST /api/auth/refresh
        Note right of FE: Send refresh token
        BE->>DB: Find refresh token by hash
        alt Token valid
            DB-->>BE: Token record
            BE->>DB: Update refresh token (rotate)
            BE->>BE: Generate new access token
            BE->>BE: Generate new refresh token hash
            BE->>DB: Update token record with new hash
            BE-->>FE: {access_token, refresh_token}
            FE->>FE: Update stored tokens
            FE->>BE: Retry original request
            BE-->>FE: Success response
        else Token invalid/expired
            DB-->>BE: Not found
            BE-->>FE: 401 Unauthorized
            FE->>FE: Clear tokens
            FE-->>FE: Redirect to login
        end
    else Token valid
        BE-->>FE: Success response
    end
```

## Multi-Device Support

Each device/browser session gets its own independent refresh token. This allows users to be logged in on multiple devices simultaneously without conflicts.

```mermaid
flowchart TB
    subgraph User
        U[User Account]
    end

    subgraph "Device Tokens"
        D1[Phone<br/>Token: abc123]
        D2[Laptop<br/>Token: def456]
        D3[Tablet<br/>Token: ghi789]
    end

    subgraph Database
        DB[(refresh_tokens table)]
    end

    U --> D1 & D2 & D3
    D1 & D2 & D3 --> DB
```

**Key behaviors:**

1. **Login creates new token** - Each login (on any device) creates a new refresh token record
2. **Refresh updates only that token** - Token rotation updates only the device's own token record
3. **Logout removes only that token** - Logging out on one device doesn't affect other devices
4. **Multiple tokens per user** - Database stores multiple refresh tokens per user (one per device)

**Database schema:**
```sql
refresh_tokens (
    id TEXT PRIMARY KEY,
    user_id TEXT NOT NULL,      -- Multiple tokens can have same user_id
    token_hash TEXT NOT NULL,
    expires_at DATETIME NOT NULL,
    created_at DATETIME NOT NULL
)
```

**Token refresh (per-device):**
```
Phone refreshes token "abc123":
  → Only token "abc123" is rotated to new value
  → Laptop token "def456" remains unchanged
  → Tablet token "ghi789" remains unchanged
```

## Logout Flow

```mermaid
sequenceDiagram
    participant U as User
    participant FE as Frontend
    participant BE as Backend
    participant DB as Database

    U->>FE: Click logout
    FE->>BE: POST /api/auth/logout
    Note right of FE: Send refresh token
    BE->>DB: Delete refresh token
    DB-->>BE: Deleted
    BE-->>FE: 200 OK
    FE->>FE: Clear localStorage
    FE-->>U: Redirect to login
```

## JWT Token Structure

```mermaid
flowchart LR
    subgraph JWT["JWT Access Token"]
        Header[Header<br/>alg: HS256<br/>typ: JWT]
        Payload[Payload<br/>sub: user_id<br/>exp: expiry<br/>iat: issued_at]
        Signature[Signature<br/>HMAC-SHA256]
    end

    Header --> Payload --> Signature
```

## Authentication Middleware

```mermaid
flowchart TB
    Req[Incoming Request] --> Extract[Extract Authorization Header]
    Extract --> Check{Has Bearer Token?}
    Check -->|No| Public{Public Route?}
    Public -->|Yes| Handler[Route Handler]
    Public -->|No| Unauth[401 Unauthorized]

    Check -->|Yes| Validate[Validate JWT]
    Validate --> Valid{Token Valid?}
    Valid -->|No| Unauth
    Valid -->|Yes| LoadUser[Load User from DB]
    LoadUser --> Inject[Inject AuthenticatedUser]
    Inject --> Handler
```

## Security Measures

```mermaid
flowchart TB
    subgraph "Password Security"
        PW[Password] --> Bcrypt[Bcrypt Hash]
        Bcrypt --> DB[(Database)]
    end

    subgraph "Token Security"
        RT[Refresh Token] --> SHA256[SHA256 Hash]
        SHA256 --> DB2[(Database)]
        AT[Access Token] --> Short[Short Expiry<br/>15-30 min]
        RT2[Refresh Token] --> Long[Longer Expiry<br/>7 days]
        RT3[Refresh Token] --> Rotate[Rotation on Use]
    end

    subgraph "Rate Limiting"
        IP[IP Address] --> Counter[Failure Counter]
        Counter --> Block{> Threshold?}
        Block -->|Yes| Lockout[Temporary Lockout]
        Block -->|No| Allow[Allow Request]
    end
```

## Frontend Token Refresh Synchronization (Critical)

When the access token expires, multiple API requests may receive 401 errors simultaneously. **This is a critical mechanism** that prevents race conditions during token refresh which would otherwise cause users to be logged out unexpectedly.

### The Race Condition Problem

Without proper synchronization, the following can occur:

```mermaid
sequenceDiagram
    participant R1 as Request 1
    participant R2 as Request 2
    participant Lock as Non-Atomic Check
    participant BE as Backend

    Note over R1,R2: Both receive 401 simultaneously
    R1->>Lock: Load flag (false)
    R2->>Lock: Load flag (false)
    Note over Lock: Gap! Both see "not locked"
    R1->>Lock: Store flag (true)
    R2->>Lock: Store flag (true)
    R1->>BE: POST /auth/refresh (token: abc123)
    R2->>BE: POST /auth/refresh (token: abc123)
    BE-->>R1: Success (rotates to xyz789)
    BE-->>R2: ERROR: Invalid token (abc123 no longer exists!)
    Note over R2: User gets logged out!
```

The problem is that `if (flag) { ... } flag = true` is not atomic - multiple requests can check the flag before any sets it.

### The Solution: Atomic Compare-Exchange

The frontend uses `AtomicBool::compare_exchange` for truly atomic check-and-set:

```mermaid
sequenceDiagram
    participant R1 as Request 1
    participant R2 as Request 2
    participant Lock as AtomicBool
    participant Gen as Generation Counter
    participant BE as Backend

    Note over R1,R2: Both receive 401 simultaneously
    R1->>Lock: compare_exchange(false→true)
    Lock-->>R1: Ok (won the race)
    R2->>Lock: compare_exchange(false→true)
    Lock-->>R2: Err (someone else is refreshing)
    R2->>Gen: Read generation (0)
    R1->>BE: POST /api/auth/refresh
    R2->>R2: Wait loop (check generation)
    BE-->>R1: New tokens
    R1->>R1: Store tokens
    R1->>Gen: Increment generation (→1)
    R1->>Lock: Store false (release)
    R2->>Gen: Read generation (1 > 0)
    R2->>R2: Generation changed! Check tokens
    R2->>R2: Tokens valid, retry original request
```

### Implementation Details

**Static atomics used:**

```rust
static REFRESH_IN_PROGRESS: AtomicBool = AtomicBool::new(false);
static REFRESH_GENERATION: AtomicU64 = AtomicU64::new(0);
```

**Why a generation counter?**

Simply checking if "any token exists" after waiting is insufficient - the old token might still be in localStorage during the refresh. The generation counter increments only after a successful refresh, allowing waiters to detect when new tokens are actually available.

**Key behaviors:**

1. **Atomic lock acquisition** - Uses `compare_exchange` to atomically check and set the lock
2. **Generation-based completion detection** - Waiters check if generation incremented, not just if tokens exist
3. **Timeout protection** - Waiters give up after 5 seconds to prevent infinite blocking
4. **Single refresh guarantee** - Exactly one token refresh request is made regardless of concurrent 401s

**Why this matters:**

- Without this, users can be randomly logged out when the access token expires and multiple requests are in flight
- The backend uses token rotation (old token is deleted when new one is created), so the race condition is particularly severe
- This is especially common on page load when multiple API calls happen simultaneously
