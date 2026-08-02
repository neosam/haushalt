-- Programmatic API access tokens.
--
-- A household member mints a token that lets an external system call the API on their
-- behalf. Each token is bound to exactly one household and one permission level (read, or
-- read+write). A token authenticates as its creator and can never exceed that member's own
-- role — `can_write` is only the coarse read/write gate on top of that.
--
-- Only the SHA-256 hash of the secret is stored (`token_hash`), so a database leak does not
-- expose usable credentials. The plaintext is returned exactly once, at creation. The
-- non-secret `token_prefix` (e.g. 'hht_1a2b3c4d') is kept purely so the owner can tell their
-- tokens apart in a list.
CREATE TABLE api_tokens (
    id TEXT PRIMARY KEY NOT NULL,
    -- The single household this token may reach.
    household_id TEXT NOT NULL REFERENCES households(id) ON DELETE CASCADE,
    -- The member who created it; requests act as this user, bounded by their role.
    user_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    name TEXT NOT NULL,
    -- SHA-256 hex of the secret. UNIQUE so a lookup by hash is an index hit and a
    -- regenerated token can never collide.
    token_hash TEXT NOT NULL UNIQUE,
    -- Non-secret leading characters of the token, for display only.
    token_prefix TEXT NOT NULL,
    -- 0 = read-only (GET/HEAD), 1 = may also write.
    can_write INTEGER NOT NULL DEFAULT 0,
    -- A disabled token is rejected at authentication without losing its configuration.
    enabled INTEGER NOT NULL DEFAULT 1,
    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    -- Stamped on every successful authentication; NULL until first use.
    last_used_at DATETIME
);

-- The token-management UI lists a user's own tokens.
CREATE INDEX idx_api_tokens_user ON api_tokens(user_id);
-- No separate hash index: the UNIQUE constraint above already provides one, and the
-- per-request authentication lookup by hash uses it.
--
-- The REFERENCES clauses document intent but do NOT enforce anything at runtime: this
-- project never enables `PRAGMA foreign_keys`, matching every table that came before. A
-- token pointing at a household the creator has since left is harmless — the handler's
-- membership check rejects the request.
