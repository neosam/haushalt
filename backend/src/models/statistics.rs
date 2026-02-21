use chrono::{DateTime, NaiveDate, Utc};
use sqlx::FromRow;
use uuid::Uuid;

/// Weekly statistics database row
#[derive(Debug, Clone, FromRow)]
#[allow(dead_code)]
pub struct WeeklyStatisticsRow {
    pub id: String,
    pub household_id: String,
    pub user_id: String,
    pub week_start: NaiveDate,
    pub week_end: NaiveDate,
    pub total_expected: i32,
    pub total_completed: i32,
    pub completion_rate: f64,
    pub calculated_at: DateTime<Utc>,
}

/// Per-task breakdown for weekly statistics
#[derive(Debug, Clone, FromRow)]
#[allow(dead_code)]
pub struct WeeklyStatisticsTaskRow {
    pub id: String,
    pub weekly_statistics_id: String,
    pub task_id: String,
    pub task_title: String,
    pub expected: i32,
    pub completed: i32,
    pub completion_rate: f64,
}

/// Monthly statistics database row
#[derive(Debug, Clone, FromRow)]
#[allow(dead_code)]
pub struct MonthlyStatisticsRow {
    pub id: String,
    pub household_id: String,
    pub user_id: String,
    pub month: NaiveDate,
    pub total_expected: i32,
    pub total_completed: i32,
    pub completion_rate: f64,
    pub calculated_at: DateTime<Utc>,
}

/// Per-task breakdown for monthly statistics
#[derive(Debug, Clone, FromRow)]
#[allow(dead_code)]
pub struct MonthlyStatisticsTaskRow {
    pub id: String,
    pub monthly_statistics_id: String,
    pub task_id: String,
    pub task_title: String,
    pub expected: i32,
    pub completed: i32,
    pub completion_rate: f64,
}

impl WeeklyStatisticsRow {
    pub fn to_member_statistic(
        &self,
        username: String,
        task_stats: Vec<shared::TaskStatistic>,
    ) -> shared::MemberStatistic {
        shared::MemberStatistic {
            user_id: Uuid::parse_str(&self.user_id).unwrap_or_default(),
            username,
            total_expected: self.total_expected,
            total_completed: self.total_completed,
            completion_rate: self.completion_rate as f32,
            task_stats,
        }
    }
}

impl WeeklyStatisticsTaskRow {
    pub fn to_shared(&self) -> shared::TaskStatistic {
        shared::TaskStatistic {
            task_id: Uuid::parse_str(&self.task_id).unwrap_or_default(),
            task_title: self.task_title.clone(),
            expected: self.expected,
            completed: self.completed,
            completion_rate: self.completion_rate as f32,
        }
    }
}

impl MonthlyStatisticsRow {
    pub fn to_member_statistic(
        &self,
        username: String,
        task_stats: Vec<shared::TaskStatistic>,
    ) -> shared::MemberStatistic {
        shared::MemberStatistic {
            user_id: Uuid::parse_str(&self.user_id).unwrap_or_default(),
            username,
            total_expected: self.total_expected,
            total_completed: self.total_completed,
            completion_rate: self.completion_rate as f32,
            task_stats,
        }
    }
}

impl MonthlyStatisticsTaskRow {
    pub fn to_shared(&self) -> shared::TaskStatistic {
        shared::TaskStatistic {
            task_id: Uuid::parse_str(&self.task_id).unwrap_or_default(),
            task_title: self.task_title.clone(),
            expected: self.expected,
            completed: self.completed,
            completion_rate: self.completion_rate as f32,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_weekly_statistics_task_row_to_shared() {
        let row = WeeklyStatisticsTaskRow {
            id: Uuid::new_v4().to_string(),
            weekly_statistics_id: Uuid::new_v4().to_string(),
            task_id: Uuid::new_v4().to_string(),
            task_title: "Clean room".to_string(),
            expected: 7,
            completed: 5,
            completion_rate: 71.43,
        };

        let shared = row.to_shared();
        assert_eq!(shared.task_title, "Clean room");
        assert_eq!(shared.expected, 7);
        assert_eq!(shared.completed, 5);
        assert!((shared.completion_rate - 71.43).abs() < 0.01);
    }

    #[test]
    fn test_weekly_statistics_row_to_member_statistic() {
        let row = WeeklyStatisticsRow {
            id: Uuid::new_v4().to_string(),
            household_id: Uuid::new_v4().to_string(),
            user_id: Uuid::new_v4().to_string(),
            week_start: NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
            week_end: NaiveDate::from_ymd_opt(2024, 1, 7).unwrap(),
            total_expected: 10,
            total_completed: 8,
            completion_rate: 80.0,
            calculated_at: Utc::now(),
        };

        let member = row.to_member_statistic("testuser".to_string(), vec![]);
        assert_eq!(member.username, "testuser");
        assert_eq!(member.total_expected, 10);
        assert_eq!(member.total_completed, 8);
        assert!((member.completion_rate - 80.0).abs() < 0.01);
    }
}
