use std::env;

#[derive(Debug, Clone)]
pub struct Config {
    pub host: String,
    pub port: u16,
    pub database_url: String,
    pub jwt_secret: String,
    pub jwt_expiration_hours: i64,
    pub static_files_path: Option<String>,
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
                .unwrap_or_else(|_| "development-secret-key-change-in-production".to_string()),
            jwt_expiration_hours: env::var("JWT_EXPIRATION_HOURS")
                .unwrap_or_else(|_| "24".to_string())
                .parse()
                .expect("JWT_EXPIRATION_HOURS must be a number"),
            static_files_path: env::var("STATIC_FILES_PATH").ok(),
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
        env::remove_var("JWT_EXPIRATION_HOURS");
        env::remove_var("STATIC_FILES_PATH");
    }

    #[test]
    fn test_config_defaults() {
        let _guard = ENV_MUTEX.lock().unwrap();
        clear_env();

        let config = Config::from_env().unwrap();

        assert_eq!(config.host, "127.0.0.1");
        assert_eq!(config.port, 8080);
        assert_eq!(config.database_url, "sqlite:household.db?mode=rwc");
        assert_eq!(config.jwt_expiration_hours, 24);
        assert!(config.static_files_path.is_none());
    }

    #[test]
    fn test_config_from_env() {
        let _guard = ENV_MUTEX.lock().unwrap();
        clear_env();

        env::set_var("HOST", "0.0.0.0");
        env::set_var("PORT", "3000");
        env::set_var("DATABASE_URL", "sqlite:test.db");
        env::set_var("JWT_SECRET", "test-secret");
        env::set_var("JWT_EXPIRATION_HOURS", "48");
        env::set_var("STATIC_FILES_PATH", "./dist");

        let config = Config::from_env().unwrap();

        assert_eq!(config.host, "0.0.0.0");
        assert_eq!(config.port, 3000);
        assert_eq!(config.database_url, "sqlite:test.db");
        assert_eq!(config.jwt_secret, "test-secret");
        assert_eq!(config.jwt_expiration_hours, 48);
        assert_eq!(config.static_files_path, Some("./dist".to_string()));

        // Clean up
        clear_env();
    }
}
