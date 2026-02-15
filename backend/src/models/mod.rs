use sqlx::SqlitePool;

use crate::config::Config;

pub mod user;
pub mod household;
pub mod household_settings;
pub mod user_settings;
pub mod membership;
pub mod task;
pub mod task_completion;
pub mod point_condition;
pub mod reward;
pub mod punishment;
pub mod invitation;
pub mod activity_log;
pub mod chat_message;
pub mod note;
pub mod announcement;

pub use user::*;
pub use household::*;
pub use household_settings::*;
pub use user_settings::*;
pub use membership::*;
pub use task::*;
pub use task_completion::*;
pub use point_condition::*;
pub use reward::*;
pub use punishment::*;
pub use invitation::*;
pub use activity_log::*;
pub use chat_message::*;
pub use note::*;
pub use announcement::*;

/// Application state shared across all handlers
pub struct AppState {
    pub db: SqlitePool,
    pub config: Config,
}
