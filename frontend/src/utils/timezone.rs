use chrono::{DateTime, Datelike, NaiveDate, Utc};
use chrono_tz::Tz;

use chrono::TimeZone as _;

/// Convert a local time string to UTC
/// Used when parsing datetime-local input values
pub fn local_string_to_utc(s: &str, tz_str: &str) -> Option<DateTime<Utc>> {
    if s.is_empty() {
        return None;
    }
    let tz: Tz = tz_str.parse().unwrap_or(chrono_tz::UTC);
    chrono::NaiveDateTime::parse_from_str(s, "%Y-%m-%dT%H:%M")
        .ok()
        .and_then(|naive| {
            // Convert naive local time to the timezone, then to UTC
            tz.from_local_datetime(&naive)
                .single()
                .map(|local_dt: DateTime<Tz>| local_dt.with_timezone(&Utc))
        })
}

/// Convert UTC datetime to local time string for datetime-local input
pub fn utc_to_local_string(dt: DateTime<Utc>, tz_str: &str) -> String {
    let tz: Tz = tz_str.parse().unwrap_or(chrono_tz::UTC);
    let local = dt.with_timezone(&tz);
    local.format("%Y-%m-%dT%H:%M").to_string()
}

/// List of common timezones for the dropdown
pub const COMMON_TIMEZONES: &[(&str, &str)] = &[
    ("UTC", "UTC"),
    ("America/New_York", "Eastern Time (US)"),
    ("America/Chicago", "Central Time (US)"),
    ("America/Denver", "Mountain Time (US)"),
    ("America/Los_Angeles", "Pacific Time (US)"),
    ("Europe/London", "London"),
    ("Europe/Paris", "Paris"),
    ("Europe/Berlin", "Berlin"),
    ("Europe/Moscow", "Moscow"),
    ("Asia/Tokyo", "Tokyo"),
    ("Asia/Shanghai", "Shanghai"),
    ("Asia/Singapore", "Singapore"),
    ("Asia/Dubai", "Dubai"),
    ("Australia/Sydney", "Sydney"),
    ("Australia/Melbourne", "Melbourne"),
    ("Pacific/Auckland", "Auckland"),
];

/// Convert UTC datetime to household timezone for display (date and time)
pub fn format_datetime(dt: DateTime<Utc>, tz_str: &str) -> String {
    let tz: Tz = tz_str.parse().unwrap_or(chrono_tz::UTC);
    let local = dt.with_timezone(&tz);
    local.format("%b %d, %Y %H:%M").to_string()
}

/// Format time only (for chat messages, activity)
pub fn format_time(dt: DateTime<Utc>, tz_str: &str) -> String {
    let tz: Tz = tz_str.parse().unwrap_or(chrono_tz::UTC);
    let local = dt.with_timezone(&tz);
    local.format("%H:%M").to_string()
}

/// Format date only
pub fn format_date(dt: DateTime<Utc>, tz_str: &str) -> String {
    let tz: Tz = tz_str.parse().unwrap_or(chrono_tz::UTC);
    let local = dt.with_timezone(&tz);
    local.format("%b %d, %Y").to_string()
}

/// Format date as short format (e.g., "Jan 25")
pub fn format_date_short(dt: DateTime<Utc>, tz_str: &str) -> String {
    let tz: Tz = tz_str.parse().unwrap_or(chrono_tz::UTC);
    let local = dt.with_timezone(&tz);
    local.format("%b %d").to_string()
}

/// Get "today" in household timezone (for task due date comparisons)
pub fn today_in_tz(tz_str: &str) -> NaiveDate {
    let tz: Tz = tz_str.parse().unwrap_or(chrono_tz::UTC);
    Utc::now().with_timezone(&tz).date_naive()
}

/// Format a NaiveDate relative to today in the given timezone
/// Returns "Today", "Tomorrow", weekday name, or formatted date
pub fn format_relative_date(date: NaiveDate, tz_str: &str) -> String {
    use chrono::Weekday;

    let today = today_in_tz(tz_str);
    let days_until = (date - today).num_days();

    match days_until {
        0 => "Today".to_string(),
        1 => "Tomorrow".to_string(),
        2..=6 => match date.weekday() {
            Weekday::Mon => "Monday",
            Weekday::Tue => "Tuesday",
            Weekday::Wed => "Wednesday",
            Weekday::Thu => "Thursday",
            Weekday::Fri => "Friday",
            Weekday::Sat => "Saturday",
            Weekday::Sun => "Sunday",
        }
        .to_string(),
        _ => date.format("%b %d").to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{TimeZone, Timelike};

    #[test]
    fn test_format_datetime() {
        let dt = Utc.with_ymd_and_hms(2024, 1, 15, 12, 30, 0).unwrap();

        // UTC should stay the same
        assert_eq!(format_datetime(dt, "UTC"), "Jan 15, 2024 12:30");

        // New York is UTC-5 in January
        assert_eq!(format_datetime(dt, "America/New_York"), "Jan 15, 2024 07:30");
    }

    #[test]
    fn test_format_time() {
        let dt = Utc.with_ymd_and_hms(2024, 1, 15, 12, 30, 0).unwrap();
        assert_eq!(format_time(dt, "UTC"), "12:30");
        assert_eq!(format_time(dt, "America/New_York"), "07:30");
    }

    #[test]
    fn test_invalid_timezone_defaults_to_utc() {
        let dt = Utc.with_ymd_and_hms(2024, 1, 15, 12, 30, 0).unwrap();
        assert_eq!(format_time(dt, "Invalid/Timezone"), "12:30");
    }

    #[test]
    fn test_berlin_timezone_conversion() {
        // In February, Berlin is UTC+1
        let dt = Utc.with_ymd_and_hms(2024, 2, 13, 21, 39, 0).unwrap();

        // 21:39 UTC should display as 22:39 Berlin time
        assert_eq!(format_time(dt, "Europe/Berlin"), "22:39");
    }

    #[test]
    fn test_local_string_to_utc_berlin() {
        // 22:39 Berlin time should convert to 21:39 UTC (Berlin is UTC+1 in February)
        let result = local_string_to_utc("2024-02-13T22:39", "Europe/Berlin");
        assert!(result.is_some());
        let utc = result.unwrap();
        assert_eq!(utc.hour(), 21);
        assert_eq!(utc.minute(), 39);
    }

    #[test]
    fn test_utc_to_local_string_berlin() {
        // 21:39 UTC should display as 22:39 Berlin time
        let dt = Utc.with_ymd_and_hms(2024, 2, 13, 21, 39, 0).unwrap();
        let result = utc_to_local_string(dt, "Europe/Berlin");
        assert_eq!(result, "2024-02-13T22:39");
    }

    #[test]
    fn test_roundtrip_berlin() {
        // Start with a UTC time
        let original_utc = Utc.with_ymd_and_hms(2024, 2, 13, 21, 39, 0).unwrap();

        // Convert to local string
        let local_str = utc_to_local_string(original_utc, "Europe/Berlin");

        // Convert back to UTC
        let back_to_utc = local_string_to_utc(&local_str, "Europe/Berlin").unwrap();

        // Should match the original
        assert_eq!(original_utc, back_to_utc);
    }
}
