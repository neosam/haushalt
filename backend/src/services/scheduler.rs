use chrono::{Datelike, NaiveDate, Weekday};
use shared::{RecurrenceType, RecurrenceValue, Task, TimePeriod};

/// Check if a task is due on a specific date based on its recurrence settings
pub fn is_task_due_on_date(task: &Task, date: NaiveDate) -> bool {
    match task.recurrence_type {
        RecurrenceType::OneTime => {
            // Free-form/one-time tasks are always "due" (can be completed anytime)
            true
        }

        RecurrenceType::Daily => true,

        RecurrenceType::Weekly => {
            // Default to the day the task was created if no specific day set
            let target_weekday = match &task.recurrence_value {
                Some(RecurrenceValue::WeekDay(day)) => weekday_from_u8(*day),
                _ => date.weekday(),
            };
            date.weekday() == target_weekday
        }

        RecurrenceType::Monthly => {
            // Default to the day of month the task was created
            let target_day = match &task.recurrence_value {
                Some(RecurrenceValue::MonthDay(day)) => *day as u32,
                _ => task.created_at.day(),
            };
            // Handle months with fewer days
            let last_day_of_month = get_last_day_of_month(date);
            let effective_day = target_day.min(last_day_of_month);
            date.day() == effective_day
        }

        RecurrenceType::Weekdays => {
            let weekdays = match &task.recurrence_value {
                Some(RecurrenceValue::Weekdays(days)) => days.clone(),
                _ => vec![1, 2, 3, 4, 5], // Mon-Fri by default
            };
            let current_weekday = weekday_to_u8(date.weekday());
            weekdays.contains(&current_weekday)
        }

        RecurrenceType::Custom => {
            match &task.recurrence_value {
                Some(RecurrenceValue::CustomDates(dates)) => dates.contains(&date),
                _ => false,
            }
        }
    }
}

/// Get the previous due date for a task before the given date
pub fn get_previous_due_date(task: &Task, current_date: NaiveDate) -> NaiveDate {
    match task.recurrence_type {
        RecurrenceType::OneTime => {
            // Free-form/one-time tasks don't have a schedule, return yesterday
            current_date - chrono::Duration::days(1)
        }

        RecurrenceType::Daily => current_date - chrono::Duration::days(1),

        RecurrenceType::Weekly => current_date - chrono::Duration::days(7),

        RecurrenceType::Monthly => {
            let target_day = match &task.recurrence_value {
                Some(RecurrenceValue::MonthDay(day)) => *day as u32,
                _ => task.created_at.day(),
            };

            // Go to previous month
            let prev_month = if current_date.month() == 1 {
                NaiveDate::from_ymd_opt(current_date.year() - 1, 12, 1).unwrap()
            } else {
                NaiveDate::from_ymd_opt(current_date.year(), current_date.month() - 1, 1).unwrap()
            };

            let last_day = get_last_day_of_month(prev_month);
            let effective_day = target_day.min(last_day);

            NaiveDate::from_ymd_opt(prev_month.year(), prev_month.month(), effective_day)
                .unwrap_or(prev_month)
        }

        RecurrenceType::Weekdays => {
            let weekdays = match &task.recurrence_value {
                Some(RecurrenceValue::Weekdays(days)) => days.clone(),
                _ => vec![1, 2, 3, 4, 5],
            };

            let mut check_date = current_date - chrono::Duration::days(1);
            for _ in 0..7 {
                let weekday = weekday_to_u8(check_date.weekday());
                if weekdays.contains(&weekday) {
                    return check_date;
                }
                check_date -= chrono::Duration::days(1);
            }
            current_date - chrono::Duration::days(1)
        }

        RecurrenceType::Custom => {
            match &task.recurrence_value {
                Some(RecurrenceValue::CustomDates(dates)) => {
                    dates
                        .iter()
                        .filter(|d| **d < current_date)
                        .max()
                        .copied()
                        .unwrap_or(current_date - chrono::Duration::days(1))
                }
                _ => current_date - chrono::Duration::days(1),
            }
        }
    }
}

/// Get the period bounds (start, end) for counting completions based on task recurrence
/// This is used for habits that can be completed multiple times per period
pub fn get_period_bounds(task: &Task, date: NaiveDate) -> (NaiveDate, NaiveDate) {
    // Determine period: explicit or inferred from recurrence_type
    let period = task.time_period.unwrap_or_else(|| match task.recurrence_type {
        RecurrenceType::Daily => TimePeriod::Day,
        RecurrenceType::Weekly | RecurrenceType::Weekdays => TimePeriod::Week,
        RecurrenceType::Monthly => TimePeriod::Month,
        RecurrenceType::Custom | RecurrenceType::OneTime => TimePeriod::None,
    });

    match period {
        TimePeriod::Day => (date, date),

        TimePeriod::Week => {
            // Get start of week (Monday) and end of week (Sunday)
            let days_from_monday = date.weekday().num_days_from_monday();
            let week_start = date - chrono::Duration::days(days_from_monday as i64);
            let week_end = week_start + chrono::Duration::days(6);
            (week_start, week_end)
        }

        TimePeriod::Month => {
            let month_start = NaiveDate::from_ymd_opt(date.year(), date.month(), 1)
                .unwrap_or(date);
            let last_day = get_last_day_of_month(date);
            let month_end = NaiveDate::from_ymd_opt(date.year(), date.month(), last_day)
                .unwrap_or(date);
            (month_start, month_end)
        }

        TimePeriod::Year => {
            // January 1st to December 31st of the current year
            let year_start = NaiveDate::from_ymd_opt(date.year(), 1, 1)
                .unwrap_or(date);
            let year_end = NaiveDate::from_ymd_opt(date.year(), 12, 31)
                .unwrap_or(date);
            (year_start, year_end)
        }

        TimePeriod::None => {
            // All-time for free-form/one-time tasks
            // Use the full date range supported by NaiveDate
            (NaiveDate::MIN, NaiveDate::MAX)
        }
    }
}

/// Get the next due date for a task after the given date
pub fn get_next_due_date(task: &Task, current_date: NaiveDate) -> Option<NaiveDate> {
    match task.recurrence_type {
        RecurrenceType::OneTime => {
            // Free-form/one-time tasks don't have a schedule, return None
            None
        }

        RecurrenceType::Daily => Some(current_date + chrono::Duration::days(1)),

        RecurrenceType::Weekly => Some(current_date + chrono::Duration::days(7)),

        RecurrenceType::Monthly => {
            let target_day = match &task.recurrence_value {
                Some(RecurrenceValue::MonthDay(day)) => *day as u32,
                _ => task.created_at.day(),
            };

            // Go to next month
            let next_month = if current_date.month() == 12 {
                NaiveDate::from_ymd_opt(current_date.year() + 1, 1, 1)?
            } else {
                NaiveDate::from_ymd_opt(current_date.year(), current_date.month() + 1, 1)?
            };

            let last_day = get_last_day_of_month(next_month);
            let effective_day = target_day.min(last_day);

            NaiveDate::from_ymd_opt(next_month.year(), next_month.month(), effective_day)
        }

        RecurrenceType::Weekdays => {
            let weekdays = match &task.recurrence_value {
                Some(RecurrenceValue::Weekdays(days)) => days.clone(),
                _ => vec![1, 2, 3, 4, 5],
            };

            let mut check_date = current_date + chrono::Duration::days(1);
            for _ in 0..7 {
                let weekday = weekday_to_u8(check_date.weekday());
                if weekdays.contains(&weekday) {
                    return Some(check_date);
                }
                check_date += chrono::Duration::days(1);
            }
            None
        }

        RecurrenceType::Custom => {
            match &task.recurrence_value {
                Some(RecurrenceValue::CustomDates(dates)) => {
                    dates.iter().filter(|d| **d > current_date).min().copied()
                }
                _ => None,
            }
        }
    }
}

fn weekday_from_u8(day: u8) -> Weekday {
    match day {
        0 => Weekday::Sun,
        1 => Weekday::Mon,
        2 => Weekday::Tue,
        3 => Weekday::Wed,
        4 => Weekday::Thu,
        5 => Weekday::Fri,
        6 => Weekday::Sat,
        _ => Weekday::Mon,
    }
}

fn weekday_to_u8(weekday: Weekday) -> u8 {
    match weekday {
        Weekday::Sun => 0,
        Weekday::Mon => 1,
        Weekday::Tue => 2,
        Weekday::Wed => 3,
        Weekday::Thu => 4,
        Weekday::Fri => 5,
        Weekday::Sat => 6,
    }
}

fn get_last_day_of_month(date: NaiveDate) -> u32 {
    let (year, month) = (date.year(), date.month());
    let next_month = if month == 12 {
        NaiveDate::from_ymd_opt(year + 1, 1, 1)
    } else {
        NaiveDate::from_ymd_opt(year, month + 1, 1)
    };

    next_month
        .map(|d| d.pred_opt().map(|p| p.day()).unwrap_or(28))
        .unwrap_or(28)
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    fn create_test_task(recurrence_type: RecurrenceType, recurrence_value: Option<RecurrenceValue>) -> Task {
        Task {
            id: uuid::Uuid::new_v4(),
            household_id: uuid::Uuid::new_v4(),
            title: "Test Task".to_string(),
            description: "".to_string(),
            recurrence_type,
            recurrence_value,
            assigned_user_id: None,
            target_count: 1,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }

    #[test]
    fn test_daily_task_always_due() {
        let task = create_test_task(RecurrenceType::Daily, None);

        let date1 = NaiveDate::from_ymd_opt(2024, 1, 15).unwrap();
        let date2 = NaiveDate::from_ymd_opt(2024, 1, 16).unwrap();
        let date3 = NaiveDate::from_ymd_opt(2024, 6, 1).unwrap();

        assert!(is_task_due_on_date(&task, date1));
        assert!(is_task_due_on_date(&task, date2));
        assert!(is_task_due_on_date(&task, date3));
    }

    #[test]
    fn test_weekly_task() {
        let task = create_test_task(RecurrenceType::Weekly, Some(RecurrenceValue::WeekDay(1))); // Monday

        let monday = NaiveDate::from_ymd_opt(2024, 1, 15).unwrap(); // Monday
        let tuesday = NaiveDate::from_ymd_opt(2024, 1, 16).unwrap();
        let next_monday = NaiveDate::from_ymd_opt(2024, 1, 22).unwrap();

        assert!(is_task_due_on_date(&task, monday));
        assert!(!is_task_due_on_date(&task, tuesday));
        assert!(is_task_due_on_date(&task, next_monday));
    }

    #[test]
    fn test_monthly_task() {
        let task = create_test_task(RecurrenceType::Monthly, Some(RecurrenceValue::MonthDay(15)));

        let jan15 = NaiveDate::from_ymd_opt(2024, 1, 15).unwrap();
        let jan16 = NaiveDate::from_ymd_opt(2024, 1, 16).unwrap();
        let feb15 = NaiveDate::from_ymd_opt(2024, 2, 15).unwrap();

        assert!(is_task_due_on_date(&task, jan15));
        assert!(!is_task_due_on_date(&task, jan16));
        assert!(is_task_due_on_date(&task, feb15));
    }

    #[test]
    fn test_monthly_task_short_month() {
        // Task due on 31st should be due on 29th in February (leap year)
        let task = create_test_task(RecurrenceType::Monthly, Some(RecurrenceValue::MonthDay(31)));

        let feb29_2024 = NaiveDate::from_ymd_opt(2024, 2, 29).unwrap();
        let feb28_2024 = NaiveDate::from_ymd_opt(2024, 2, 28).unwrap();

        assert!(is_task_due_on_date(&task, feb29_2024)); // Last day of Feb
        assert!(!is_task_due_on_date(&task, feb28_2024));
    }

    #[test]
    fn test_weekdays_task() {
        let task = create_test_task(
            RecurrenceType::Weekdays,
            Some(RecurrenceValue::Weekdays(vec![1, 3, 5])), // Mon, Wed, Fri
        );

        let monday = NaiveDate::from_ymd_opt(2024, 1, 15).unwrap();
        let tuesday = NaiveDate::from_ymd_opt(2024, 1, 16).unwrap();
        let wednesday = NaiveDate::from_ymd_opt(2024, 1, 17).unwrap();
        let thursday = NaiveDate::from_ymd_opt(2024, 1, 18).unwrap();
        let friday = NaiveDate::from_ymd_opt(2024, 1, 19).unwrap();

        assert!(is_task_due_on_date(&task, monday));
        assert!(!is_task_due_on_date(&task, tuesday));
        assert!(is_task_due_on_date(&task, wednesday));
        assert!(!is_task_due_on_date(&task, thursday));
        assert!(is_task_due_on_date(&task, friday));
    }

    #[test]
    fn test_custom_dates_task() {
        let dates = vec![
            NaiveDate::from_ymd_opt(2024, 1, 15).unwrap(),
            NaiveDate::from_ymd_opt(2024, 2, 20).unwrap(),
            NaiveDate::from_ymd_opt(2024, 3, 25).unwrap(),
        ];
        let task = create_test_task(RecurrenceType::Custom, Some(RecurrenceValue::CustomDates(dates)));

        let jan15 = NaiveDate::from_ymd_opt(2024, 1, 15).unwrap();
        let jan16 = NaiveDate::from_ymd_opt(2024, 1, 16).unwrap();
        let feb20 = NaiveDate::from_ymd_opt(2024, 2, 20).unwrap();

        assert!(is_task_due_on_date(&task, jan15));
        assert!(!is_task_due_on_date(&task, jan16));
        assert!(is_task_due_on_date(&task, feb20));
    }

    #[test]
    fn test_get_previous_due_date_daily() {
        let task = create_test_task(RecurrenceType::Daily, None);
        let date = NaiveDate::from_ymd_opt(2024, 1, 15).unwrap();

        let prev = get_previous_due_date(&task, date);
        assert_eq!(prev, NaiveDate::from_ymd_opt(2024, 1, 14).unwrap());
    }

    #[test]
    fn test_get_next_due_date_weekly() {
        let task = create_test_task(RecurrenceType::Weekly, Some(RecurrenceValue::WeekDay(1)));
        let monday = NaiveDate::from_ymd_opt(2024, 1, 15).unwrap();

        let next = get_next_due_date(&task, monday);
        assert_eq!(next, Some(NaiveDate::from_ymd_opt(2024, 1, 22).unwrap()));
    }

    #[test]
    fn test_last_day_of_month() {
        let jan = NaiveDate::from_ymd_opt(2024, 1, 15).unwrap();
        let feb_leap = NaiveDate::from_ymd_opt(2024, 2, 15).unwrap();
        let feb_normal = NaiveDate::from_ymd_opt(2023, 2, 15).unwrap();

        assert_eq!(get_last_day_of_month(jan), 31);
        assert_eq!(get_last_day_of_month(feb_leap), 29);
        assert_eq!(get_last_day_of_month(feb_normal), 28);
    }
}
