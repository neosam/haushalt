use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

/// Database model for point conditions
#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct PointConditionRow {
    pub id: String,
    pub household_id: String,
    pub name: String,
    pub condition_type: String,
    pub points_value: i64,
    pub streak_threshold: Option<i32>,
    pub multiplier: Option<f64>,
    pub task_id: Option<String>,
    pub created_at: DateTime<Utc>,
}

impl PointConditionRow {
    pub fn to_shared(&self) -> shared::PointCondition {
        shared::PointCondition {
            id: Uuid::parse_str(&self.id).unwrap(),
            household_id: Uuid::parse_str(&self.household_id).unwrap(),
            name: self.name.clone(),
            condition_type: shared::ConditionType::from_str(&self.condition_type)
                .unwrap_or(shared::ConditionType::TaskComplete),
            points_value: self.points_value,
            streak_threshold: self.streak_threshold,
            multiplier: self.multiplier,
            task_id: self.task_id.as_ref().and_then(|id| Uuid::parse_str(id).ok()),
            created_at: self.created_at,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use shared::ConditionType;

    #[test]
    fn test_point_condition_row_to_shared() {
        let now = Utc::now();
        let id = Uuid::new_v4();
        let household_id = Uuid::new_v4();

        let row = PointConditionRow {
            id: id.to_string(),
            household_id: household_id.to_string(),
            name: "Complete Task Bonus".to_string(),
            condition_type: "task_complete".to_string(),
            points_value: 10,
            streak_threshold: None,
            multiplier: None,
            task_id: None,
            created_at: now,
        };

        let shared = row.to_shared();

        assert_eq!(shared.id, id);
        assert_eq!(shared.household_id, household_id);
        assert_eq!(shared.name, "Complete Task Bonus");
        assert_eq!(shared.condition_type, ConditionType::TaskComplete);
        assert_eq!(shared.points_value, 10);
    }

    #[test]
    fn test_point_condition_with_streak() {
        let now = Utc::now();
        let task_id = Uuid::new_v4();

        let row = PointConditionRow {
            id: Uuid::new_v4().to_string(),
            household_id: Uuid::new_v4().to_string(),
            name: "7-Day Streak Bonus".to_string(),
            condition_type: "streak".to_string(),
            points_value: 50,
            streak_threshold: Some(7),
            multiplier: Some(1.5),
            task_id: Some(task_id.to_string()),
            created_at: now,
        };

        let shared = row.to_shared();

        assert_eq!(shared.condition_type, ConditionType::Streak);
        assert_eq!(shared.streak_threshold, Some(7));
        assert_eq!(shared.multiplier, Some(1.5));
        assert_eq!(shared.task_id, Some(task_id));
    }
}
