use std::env;

#[derive(Debug, Clone)]
pub struct Config {
    pub host: String,
    pub port: u16,
    pub database_url: String,
    pub jwt_secret: String,
    pub access_token_expiration_minutes: i64,
    pub refresh_token_expiration_days: i64,
    pub static_files_path: Option<String>,
    pub cors_origins: Vec<String>,
    pub legal_dir: Option<String>,
}

impl Config {
    pub fn from_env() -> Result<Self, env::VarError> {
        Ok(Self {
            host: env::var("HOST").unwrap_or_else(|_| "127.0.0.1".to_string()),
            port: env::var("PORT")
                .unwrap_or_else(|_| "8080".to_string())
                .parse()
                .expect("PORT must be a number"),
            database_url: env::var("DATABASE_URL")
                .unwrap_or_else(|_| "sqlite:household.db?mode=rwc".to_string()),
            jwt_secret: env::var("JWT_SECRET")
                .expect("JWT_SECRET environment variable must be set"),
            access_token_expiration_minutes: env::var("ACCESS_TOKEN_EXPIRATION_MINUTES")
                .unwrap_or_else(|_| "15".to_string())
                .parse()
                .expect("ACCESS_TOKEN_EXPIRATION_MINUTES must be a number"),
            refresh_token_expiration_days: env::var("REFRESH_TOKEN_EXPIRATION_DAYS")
                .unwrap_or_else(|_| "30".to_string())
                .parse()
                .expect("REFRESH_TOKEN_EXPIRATION_DAYS must be a number"),
            static_files_path: env::var("STATIC_FILES_PATH").ok(),
            cors_origins: env::var("CORS_ORIGINS")
                .unwrap_or_else(|_| "http://localhost,http://127.0.0.1".to_string())
                .split(',')
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect(),
            legal_dir: env::var("LEGAL_DIR").ok(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    // Mutex to ensure config tests run serially (env vars are global)
    static ENV_MUTEX: Mutex<()> = Mutex::new(());

    fn clear_env() {
        env::remove_var("HOST");
        env::remove_var("PORT");
        env::remove_var("DATABASE_URL");
        env::remove_var("JWT_SECRET");
        env::remove_var("ACCESS_TOKEN_EXPIRATION_MINUTES");
        env::remove_var("REFRESH_TOKEN_EXPIRATION_DAYS");
        env::remove_var("STATIC_FILES_PATH");
        env::remove_var("CORS_ORIGINS");
        env::remove_var("LEGAL_DIR");
    }

    #[test]
    fn test_config_defaults() {
        let _guard = ENV_MUTEX.lock().unwrap();
        clear_env();

        // JWT_SECRET is now required, so we must set it
        env::set_var("JWT_SECRET", "test-secret-for-defaults");

        let config = Config::from_env().unwrap();

        assert_eq!(config.host, "127.0.0.1");
        assert_eq!(config.port, 8080);
        assert_eq!(config.database_url, "sqlite:household.db?mode=rwc");
        assert_eq!(config.access_token_expiration_minutes, 15);
        assert_eq!(config.refresh_token_expiration_days, 30);
        assert!(config.static_files_path.is_none());
        assert_eq!(config.cors_origins, vec!["http://localhost", "http://127.0.0.1"]);

        clear_env();
    }

    #[test]
    fn test_config_from_env() {
        let _guard = ENV_MUTEX.lock().unwrap();
        clear_env();

        env::set_var("HOST", "0.0.0.0");
        env::set_var("PORT", "3000");
        env::set_var("DATABASE_URL", "sqlite:test.db");
        env::set_var("JWT_SECRET", "test-secret");
        env::set_var("ACCESS_TOKEN_EXPIRATION_MINUTES", "30");
        env::set_var("REFRESH_TOKEN_EXPIRATION_DAYS", "7");
        env::set_var("STATIC_FILES_PATH", "./dist");
        env::set_var("CORS_ORIGINS", "https://example.com, https://app.example.com");

        let config = Config::from_env().unwrap();

        assert_eq!(config.host, "0.0.0.0");
        assert_eq!(config.port, 3000);
        assert_eq!(config.database_url, "sqlite:test.db");
        assert_eq!(config.jwt_secret, "test-secret");
        assert_eq!(config.access_token_expiration_minutes, 30);
        assert_eq!(config.refresh_token_expiration_days, 7);
        assert_eq!(config.static_files_path, Some("./dist".to_string()));
        assert_eq!(config.cors_origins, vec!["https://example.com", "https://app.example.com"]);

        // Clean up
        clear_env();
    }
}
