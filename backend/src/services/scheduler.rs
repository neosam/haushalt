use chrono::{Datelike, NaiveDate, NaiveTime, Weekday, DateTime, Utc, TimeZone};
use chrono_tz::Tz;
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
#[allow(dead_code)]
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

/// Get the next due date for a task on or after the given date
/// Returns None for OneTime tasks (they have no schedule)
pub fn get_next_due_date(task: &Task, from_date: NaiveDate) -> Option<NaiveDate> {
    match task.recurrence_type {
        RecurrenceType::OneTime => {
            // OneTime tasks don't have a recurring schedule
            None
        }

        RecurrenceType::Daily => Some(from_date),

        RecurrenceType::Weekly => {
            let target_weekday = match &task.recurrence_value {
                Some(RecurrenceValue::WeekDay(day)) => weekday_from_u8(*day),
                _ => from_date.weekday(), // Default to current weekday
            };

            // Find next occurrence of target weekday
            let current_weekday = from_date.weekday();
            let days_until = (target_weekday.num_days_from_sunday() as i64
                - current_weekday.num_days_from_sunday() as i64
                + 7)
                % 7;

            // If today is the target day, return today
            if days_until == 0 {
                Some(from_date)
            } else {
                Some(from_date + chrono::Duration::days(days_until))
            }
        }

        RecurrenceType::Monthly => {
            let target_day = match &task.recurrence_value {
                Some(RecurrenceValue::MonthDay(day)) => *day as u32,
                _ => task.created_at.day(),
            };

            // Check if we can hit target day this month
            let last_day_this_month = get_last_day_of_month(from_date);
            let effective_day_this_month = target_day.min(last_day_this_month);

            if from_date.day() <= effective_day_this_month {
                // Target day is still ahead this month (or today)
                NaiveDate::from_ymd_opt(from_date.year(), from_date.month(), effective_day_this_month)
            } else {
                // Target day has passed this month, go to next month
                let next_month = if from_date.month() == 12 {
                    NaiveDate::from_ymd_opt(from_date.year() + 1, 1, 1).unwrap()
                } else {
                    NaiveDate::from_ymd_opt(from_date.year(), from_date.month() + 1, 1).unwrap()
                };

                let last_day_next_month = get_last_day_of_month(next_month);
                let effective_day_next_month = target_day.min(last_day_next_month);

                NaiveDate::from_ymd_opt(next_month.year(), next_month.month(), effective_day_next_month)
            }
        }

        RecurrenceType::Weekdays => {
            let weekdays = match &task.recurrence_value {
                Some(RecurrenceValue::Weekdays(days)) => days.clone(),
                _ => vec![1, 2, 3, 4, 5], // Mon-Fri by default
            };

            // Check today and the next 7 days
            for i in 0..7 {
                let check_date = from_date + chrono::Duration::days(i);
                let weekday = weekday_to_u8(check_date.weekday());
                if weekdays.contains(&weekday) {
                    return Some(check_date);
                }
            }
            // Shouldn't happen with valid weekdays, but fallback to from_date
            Some(from_date)
        }

        RecurrenceType::Custom => {
            match &task.recurrence_value {
                Some(RecurrenceValue::CustomDates(dates)) => {
                    // Find the first date >= from_date
                    dates
                        .iter()
                        .filter(|d| **d >= from_date)
                        .min()
                        .copied()
                }
                _ => None,
            }
        }
    }
}

/// Get the period bounds (start, end) for counting completions based on task recurrence
/// This is used for habits that can be completed multiple times per period
pub fn get_period_bounds(task: &Task, date: NaiveDate) -> (NaiveDate, NaiveDate) {
    // Determine period: explicit or inferred from recurrence_type
    let period = task.time_period.unwrap_or(match task.recurrence_type {
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
            // Use a reasonable date range that SQLite/SQLx can handle properly
            // (NaiveDate::MIN/MAX cause serialization issues)
            let min_date = NaiveDate::from_ymd_opt(1970, 1, 1).unwrap();
            let max_date = NaiveDate::from_ymd_opt(2100, 12, 31).unwrap();
            (min_date, max_date)
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

/// Parse a timezone string, defaulting to UTC if invalid
pub fn parse_timezone(tz_str: &str) -> Tz {
    tz_str.parse().unwrap_or(chrono_tz::UTC)
}

/// Get the current date in a specific timezone
pub fn today_in_timezone(tz: Tz) -> NaiveDate {
    Utc::now().with_timezone(&tz).date_naive()
}

/// Parse due_time string "HH:MM" to NaiveTime, defaults to 23:59 if None or invalid
pub fn parse_due_time(due_time: Option<&str>) -> NaiveTime {
    due_time
        .and_then(|t| NaiveTime::parse_from_str(t, "%H:%M").ok())
        .unwrap_or_else(|| NaiveTime::from_hms_opt(23, 59, 0).unwrap())
}

/// Check if a task is overdue at a specific UTC time, considering the household timezone
/// A task is overdue if:
/// 1. The due date in the household timezone has passed
/// 2. The due time on that date has passed
pub fn is_task_overdue(task: &Task, due_date: NaiveDate, timezone: &str, now_utc: DateTime<Utc>) -> bool {
    let tz = parse_timezone(timezone);
    let now_local = now_utc.with_timezone(&tz);
    let today_local = now_local.date_naive();
    let current_time = now_local.time();

    let due_time = parse_due_time(task.due_time.as_deref());

    // If due date is before today, it's overdue
    if due_date < today_local {
        return true;
    }

    // If due date is today, check if due time has passed
    if due_date == today_local && current_time > due_time {
        return true;
    }

    false
}

/// Get the deadline DateTime in UTC for a task on a specific due date
#[allow(dead_code)]
pub fn get_task_deadline_utc(task: &Task, due_date: NaiveDate, timezone: &str) -> Option<DateTime<Utc>> {
    let tz = parse_timezone(timezone);
    let due_time = parse_due_time(task.due_time.as_deref());

    let local_datetime = due_date.and_time(due_time);
    tz.from_local_datetime(&local_datetime)
        .single()
        .map(|dt| dt.with_timezone(&Utc))
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
            time_period: None,
            allow_exceed_target: true,
            requires_review: false,
            points_reward: None,
            points_penalty: None,
            due_time: None,
            habit_type: shared::HabitType::Good,
            category_id: None,
            category_name: None,
            archived: false,
            paused: false,
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
    fn test_last_day_of_month() {
        let jan = NaiveDate::from_ymd_opt(2024, 1, 15).unwrap();
        let feb_leap = NaiveDate::from_ymd_opt(2024, 2, 15).unwrap();
        let feb_normal = NaiveDate::from_ymd_opt(2023, 2, 15).unwrap();

        assert_eq!(get_last_day_of_month(jan), 31);
        assert_eq!(get_last_day_of_month(feb_leap), 29);
        assert_eq!(get_last_day_of_month(feb_normal), 28);
    }

    // Tests for get_next_due_date

    #[test]
    fn test_get_next_due_date_onetime() {
        let task = create_test_task(RecurrenceType::OneTime, None);
        let date = NaiveDate::from_ymd_opt(2024, 1, 15).unwrap();

        assert_eq!(get_next_due_date(&task, date), None);
    }

    #[test]
    fn test_get_next_due_date_daily() {
        let task = create_test_task(RecurrenceType::Daily, None);
        let date = NaiveDate::from_ymd_opt(2024, 1, 15).unwrap();

        assert_eq!(get_next_due_date(&task, date), Some(date));
    }

    #[test]
    fn test_get_next_due_date_weekly_same_day() {
        // Monday task, query on Monday
        let task = create_test_task(RecurrenceType::Weekly, Some(RecurrenceValue::WeekDay(1)));
        let monday = NaiveDate::from_ymd_opt(2024, 1, 15).unwrap(); // Monday

        assert_eq!(get_next_due_date(&task, monday), Some(monday));
    }

    #[test]
    fn test_get_next_due_date_weekly_later_in_week() {
        // Monday task, query on Wednesday
        let task = create_test_task(RecurrenceType::Weekly, Some(RecurrenceValue::WeekDay(1)));
        let wednesday = NaiveDate::from_ymd_opt(2024, 1, 17).unwrap(); // Wednesday
        let next_monday = NaiveDate::from_ymd_opt(2024, 1, 22).unwrap();

        assert_eq!(get_next_due_date(&task, wednesday), Some(next_monday));
    }

    #[test]
    fn test_get_next_due_date_weekly_earlier_in_week() {
        // Friday task, query on Monday
        let task = create_test_task(RecurrenceType::Weekly, Some(RecurrenceValue::WeekDay(5)));
        let monday = NaiveDate::from_ymd_opt(2024, 1, 15).unwrap(); // Monday
        let friday = NaiveDate::from_ymd_opt(2024, 1, 19).unwrap();

        assert_eq!(get_next_due_date(&task, monday), Some(friday));
    }

    #[test]
    fn test_get_next_due_date_monthly_same_day() {
        let task = create_test_task(RecurrenceType::Monthly, Some(RecurrenceValue::MonthDay(15)));
        let jan15 = NaiveDate::from_ymd_opt(2024, 1, 15).unwrap();

        assert_eq!(get_next_due_date(&task, jan15), Some(jan15));
    }

    #[test]
    fn test_get_next_due_date_monthly_later_this_month() {
        let task = create_test_task(RecurrenceType::Monthly, Some(RecurrenceValue::MonthDay(20)));
        let jan15 = NaiveDate::from_ymd_opt(2024, 1, 15).unwrap();
        let jan20 = NaiveDate::from_ymd_opt(2024, 1, 20).unwrap();

        assert_eq!(get_next_due_date(&task, jan15), Some(jan20));
    }

    #[test]
    fn test_get_next_due_date_monthly_next_month() {
        let task = create_test_task(RecurrenceType::Monthly, Some(RecurrenceValue::MonthDay(10)));
        let jan15 = NaiveDate::from_ymd_opt(2024, 1, 15).unwrap();
        let feb10 = NaiveDate::from_ymd_opt(2024, 2, 10).unwrap();

        assert_eq!(get_next_due_date(&task, jan15), Some(feb10));
    }

    #[test]
    fn test_get_next_due_date_monthly_short_month() {
        // Task due on 31st, query in February
        let task = create_test_task(RecurrenceType::Monthly, Some(RecurrenceValue::MonthDay(31)));
        let feb1 = NaiveDate::from_ymd_opt(2024, 2, 1).unwrap();
        let feb29 = NaiveDate::from_ymd_opt(2024, 2, 29).unwrap(); // Leap year

        assert_eq!(get_next_due_date(&task, feb1), Some(feb29));
    }

    #[test]
    fn test_get_next_due_date_weekdays() {
        // Mon, Wed, Fri
        let task = create_test_task(
            RecurrenceType::Weekdays,
            Some(RecurrenceValue::Weekdays(vec![1, 3, 5])),
        );

        let monday = NaiveDate::from_ymd_opt(2024, 1, 15).unwrap();
        let tuesday = NaiveDate::from_ymd_opt(2024, 1, 16).unwrap();
        let wednesday = NaiveDate::from_ymd_opt(2024, 1, 17).unwrap();

        assert_eq!(get_next_due_date(&task, monday), Some(monday));
        assert_eq!(get_next_due_date(&task, tuesday), Some(wednesday));
    }

    #[test]
    fn test_get_next_due_date_custom() {
        let dates = vec![
            NaiveDate::from_ymd_opt(2024, 1, 15).unwrap(),
            NaiveDate::from_ymd_opt(2024, 2, 20).unwrap(),
            NaiveDate::from_ymd_opt(2024, 3, 25).unwrap(),
        ];
        let task = create_test_task(RecurrenceType::Custom, Some(RecurrenceValue::CustomDates(dates)));

        let jan10 = NaiveDate::from_ymd_opt(2024, 1, 10).unwrap();
        let jan15 = NaiveDate::from_ymd_opt(2024, 1, 15).unwrap();
        let jan20 = NaiveDate::from_ymd_opt(2024, 1, 20).unwrap();
        let feb20 = NaiveDate::from_ymd_opt(2024, 2, 20).unwrap();

        assert_eq!(get_next_due_date(&task, jan10), Some(jan15));
        assert_eq!(get_next_due_date(&task, jan15), Some(jan15));
        assert_eq!(get_next_due_date(&task, jan20), Some(feb20));
    }

    #[test]
    fn test_get_next_due_date_custom_all_past() {
        let dates = vec![
            NaiveDate::from_ymd_opt(2024, 1, 15).unwrap(),
        ];
        let task = create_test_task(RecurrenceType::Custom, Some(RecurrenceValue::CustomDates(dates)));

        let jan20 = NaiveDate::from_ymd_opt(2024, 1, 20).unwrap();
        assert_eq!(get_next_due_date(&task, jan20), None);
    }
}
