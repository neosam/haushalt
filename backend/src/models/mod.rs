use std::sync::Arc;
use sqlx::SqlitePool;

use crate::config::Config;
use crate::middleware::RateLimiter;

pub mod user;
pub mod household;
pub mod household_settings;
pub mod user_settings;
pub mod membership;
pub mod task;
pub mod task_category;
pub mod task_completion;
pub mod task_period_result;
pub mod point_condition;
pub mod reward;
pub mod punishment;
pub mod invitation;
pub mod activity_log;
pub mod chat_message;
pub mod note;
pub mod journal;
pub mod announcement;
pub mod refresh_token;
pub mod statistics;

pub use user::*;
pub use household::*;
pub use household_settings::*;
pub use user_settings::*;
pub use membership::*;
pub use task::*;
pub use task_category::*;
pub use task_completion::*;
pub use task_period_result::*;
pub use point_condition::*;
pub use reward::*;
pub use punishment::*;
pub use invitation::*;
pub use activity_log::*;
pub use chat_message::*;
pub use note::*;
pub use journal::*;
pub use announcement::*;
pub use refresh_token::*;
pub use statistics::*;

/// Application state shared across all handlers
pub struct AppState {
    pub db: SqlitePool,
    pub config: Config,
    pub login_rate_limiter: Arc<RateLimiter>,
}
