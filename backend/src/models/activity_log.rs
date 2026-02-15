use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use shared::ActivityType;
use sqlx::FromRow;
use uuid::Uuid;

/// Database model for activity logs
#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
#[allow(dead_code)]
pub struct ActivityLogRow {
    pub id: String,
    pub household_id: String,
    pub actor_id: String,
    pub affected_user_id: Option<String>,
    pub activity_type: String,
    pub entity_type: Option<String>,
    pub entity_id: Option<String>,
    pub details: Option<String>,
    pub created_at: DateTime<Utc>,
}

impl ActivityLogRow {
    #[allow(dead_code)]
    pub fn to_shared(&self) -> shared::ActivityLog {
        shared::ActivityLog {
            id: Uuid::parse_str(&self.id).unwrap(),
            household_id: Uuid::parse_str(&self.household_id).unwrap(),
            actor_id: Uuid::parse_str(&self.actor_id).unwrap(),
            affected_user_id: self.affected_user_id.as_ref().map(|s| Uuid::parse_str(s).unwrap()),
            activity_type: self.activity_type.parse().unwrap_or(ActivityType::TaskCreated),
            entity_type: self.entity_type.clone(),
            entity_id: self.entity_id.as_ref().map(|s| Uuid::parse_str(s).unwrap()),
            details: self.details.clone(),
            created_at: self.created_at,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_activity_log_row_to_shared() {
        let now = Utc::now();
        let id = Uuid::new_v4();
        let household_id = Uuid::new_v4();
        let actor_id = Uuid::new_v4();
        let affected_user_id = Uuid::new_v4();
        let entity_id = Uuid::new_v4();

        let row = ActivityLogRow {
            id: id.to_string(),
            household_id: household_id.to_string(),
            actor_id: actor_id.to_string(),
            affected_user_id: Some(affected_user_id.to_string()),
            activity_type: "task_created".to_string(),
            entity_type: Some("task".to_string()),
            entity_id: Some(entity_id.to_string()),
            details: Some(r#"{"task_title":"Clean room"}"#.to_string()),
            created_at: now,
        };

        let shared = row.to_shared();

        assert_eq!(shared.id, id);
        assert_eq!(shared.household_id, household_id);
        assert_eq!(shared.actor_id, actor_id);
        assert_eq!(shared.affected_user_id, Some(affected_user_id));
        assert_eq!(shared.activity_type, ActivityType::TaskCreated);
        assert_eq!(shared.entity_type, Some("task".to_string()));
        assert_eq!(shared.entity_id, Some(entity_id));
    }

    #[test]
    fn test_activity_log_row_to_shared_no_affected_user() {
        let now = Utc::now();
        let id = Uuid::new_v4();
        let household_id = Uuid::new_v4();
        let actor_id = Uuid::new_v4();

        let row = ActivityLogRow {
            id: id.to_string(),
            household_id: household_id.to_string(),
            actor_id: actor_id.to_string(),
            affected_user_id: None,
            activity_type: "settings_changed".to_string(),
            entity_type: None,
            entity_id: None,
            details: None,
            created_at: now,
        };

        let shared = row.to_shared();

        assert_eq!(shared.id, id);
        assert!(shared.affected_user_id.is_none());
        assert_eq!(shared.activity_type, ActivityType::SettingsChanged);
        assert!(shared.entity_type.is_none());
        assert!(shared.entity_id.is_none());
        assert!(shared.details.is_none());
    }
}
