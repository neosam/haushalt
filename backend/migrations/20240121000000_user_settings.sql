-- User settings for per-user preferences like language
CREATE TABLE IF NOT EXISTS user_settings (
    user_id TEXT PRIMARY KEY NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    language TEXT NOT NULL DEFAULT 'en',
    updated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
);
