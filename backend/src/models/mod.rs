use sqlx::SqlitePool;

use crate::config::Config;

pub mod user;
pub mod household;
pub mod membership;
pub mod task;
pub mod task_completion;
pub mod point_condition;
pub mod reward;
pub mod punishment;
pub mod invitation;

pub use user::*;
pub use household::*;
pub use membership::*;
pub use task::*;
pub use task_completion::*;
pub use point_condition::*;
pub use reward::*;
pub use punishment::*;
pub use invitation::*;

/// Application state shared across all handlers
pub struct AppState {
    pub db: SqlitePool,
    pub config: Config,
}
