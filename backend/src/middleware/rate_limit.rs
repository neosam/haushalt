use std::collections::HashMap;
use std::sync::Mutex;
use std::time::{Duration, Instant};

/// In-memory rate limiter for protecting against brute force attacks
pub struct RateLimiter {
    /// Maps keys (e.g., IP address or username) to list of attempt timestamps
    attempts: Mutex<HashMap<String, Vec<Instant>>>,
    /// Maximum number of attempts allowed within the time window
    max_attempts: usize,
    /// Time window for rate limiting
    window: Duration,
}

impl RateLimiter {
    /// Create a new rate limiter
    ///
    /// # Arguments
    /// * `max_attempts` - Maximum attempts allowed within the window
    /// * `window_secs` - Time window in seconds
    pub fn new(max_attempts: usize, window_secs: u64) -> Self {
        Self {
            attempts: Mutex::new(HashMap::new()),
            max_attempts,
            window: Duration::from_secs(window_secs),
        }
    }

    /// Check if a request is allowed (returns true if allowed, false if rate limited)
    pub fn check(&self, key: &str) -> bool {
        let mut attempts = self.attempts.lock().unwrap();
        let now = Instant::now();

        // Get or create entry for this key
        let entry = attempts.entry(key.to_string()).or_default();

        // Remove old attempts outside the window
        entry.retain(|&time| now.duration_since(time) < self.window);

        // Check if under limit
        entry.len() < self.max_attempts
    }

    /// Record an attempt for a key (call after failed login)
    pub fn record(&self, key: &str) {
        let mut attempts = self.attempts.lock().unwrap();
        let now = Instant::now();

        let entry = attempts.entry(key.to_string()).or_default();

        // Clean up old entries while we're at it
        entry.retain(|&time| now.duration_since(time) < self.window);

        // Add new attempt
        entry.push(now);
    }

    /// Clear all attempts for a key (e.g., after successful login)
    #[allow(dead_code)]
    pub fn clear(&self, key: &str) {
        let mut attempts = self.attempts.lock().unwrap();
        attempts.remove(key);
    }

    /// Get remaining attempts for a key
    #[allow(dead_code)]
    pub fn remaining(&self, key: &str) -> usize {
        let attempts = self.attempts.lock().unwrap();
        let now = Instant::now();

        if let Some(entry) = attempts.get(key) {
            let valid_attempts = entry
                .iter()
                .filter(|&&time| now.duration_since(time) < self.window)
                .count();
            self.max_attempts.saturating_sub(valid_attempts)
        } else {
            self.max_attempts
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread::sleep;

    #[test]
    fn test_rate_limiter_allows_under_limit() {
        let limiter = RateLimiter::new(3, 60);

        assert!(limiter.check("test_key"));
        limiter.record("test_key");
        assert!(limiter.check("test_key"));
        limiter.record("test_key");
        assert!(limiter.check("test_key"));
    }

    #[test]
    fn test_rate_limiter_blocks_over_limit() {
        let limiter = RateLimiter::new(2, 60);

        limiter.record("test_key");
        limiter.record("test_key");
        assert!(!limiter.check("test_key"));
    }

    #[test]
    fn test_rate_limiter_window_expires() {
        let limiter = RateLimiter::new(2, 1); // 1 second window

        limiter.record("test_key");
        limiter.record("test_key");
        assert!(!limiter.check("test_key"));

        // Wait for window to expire
        sleep(Duration::from_secs(2));

        assert!(limiter.check("test_key"));
    }

    #[test]
    fn test_rate_limiter_different_keys() {
        let limiter = RateLimiter::new(1, 60);

        limiter.record("key1");
        assert!(!limiter.check("key1"));
        assert!(limiter.check("key2")); // Different key should still be allowed
    }

    #[test]
    fn test_rate_limiter_clear() {
        let limiter = RateLimiter::new(2, 60);

        limiter.record("test_key");
        limiter.record("test_key");
        assert!(!limiter.check("test_key"));

        limiter.clear("test_key");
        assert!(limiter.check("test_key"));
    }

    #[test]
    fn test_rate_limiter_remaining() {
        let limiter = RateLimiter::new(3, 60);

        assert_eq!(limiter.remaining("test_key"), 3);
        limiter.record("test_key");
        assert_eq!(limiter.remaining("test_key"), 2);
        limiter.record("test_key");
        assert_eq!(limiter.remaining("test_key"), 1);
    }
}
