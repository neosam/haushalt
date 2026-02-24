## 1. Fix Period Type Mapping

- [x] 1.1 Change `get_period_bounds()` in `backend/src/services/scheduler.rs` line 227 to separate Weekdays from Weekly: `RecurrenceType::Weekdays => TimePeriod::Day`

## 2. Add Tests

- [x] 2.1 Add test verifying Weekdays task completion uses daily period bounds (period_start = completion date, not week's Monday)
