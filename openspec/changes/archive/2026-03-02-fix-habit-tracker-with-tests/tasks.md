## 1. Review und Korrektur bestehender Tests

- [x] 1.1 Identifiziere falsche Tests in `backend/src/services/scheduler.rs` (Zeile 694: weekdays, Zeile 713: custom)
- [x] 1.2 Korrigiere `test_get_next_due_date_weekdays`: Wenn today=monday, erwarte **next** monday, nicht today
- [x] 1.3 Korrigiere `test_get_next_due_date_custom`: Wenn today=jan15, erwarte **next** custom date, nicht jan15
- [x] 1.4 Run tests und verifiziere dass sie jetzt **failing** sind (weil Implementierung noch falsch ist)

## 2. Neue Tests für Early Completion (scheduler.rs)

- [x] 2.1 Schreibe `test_get_next_due_date_weekdays_early_completion`: Complete Montag task am Sonntag → expect Monday
- [x] 2.2 Schreibe `test_get_next_due_date_weekdays_on_scheduled_day`: Complete Montag task am Montag → expect next Monday (not today)
- [x] 2.3 Schreibe `test_get_next_due_date_custom_early_completion`: Complete Feb25 task am Feb24 → expect Feb25
- [x] 2.4 Schreibe `test_get_next_due_date_custom_on_scheduled_date`: Complete Feb25 task am Feb25 → expect next custom date after Feb25
- [x] 2.5 Run tests und verifiziere alle early completion tests **fail** (Implementierung noch nicht gefixt)

## 3. Neue Tests für Edge Cases (scheduler.rs)

- [x] 3.1 Schreibe `test_get_next_due_date_weekdays_no_match_in_week`: Weekdays=[Mon], from_date=Tuesday → expect next Monday (7 days later)
- [x] 3.2 Schreibe `test_get_next_due_date_custom_last_date_passed`: All custom dates in past → expect None
- [x] 3.3 Schreibe `test_get_period_bounds_consistency`: Verifiziere period_bounds für alle recurrence types mit verschiedenen dates
- [x] 3.4 Run tests und verifiziere edge case tests (manche sollten passen, manche failen)

## 4. Integration Tests für complete_task (tasks.rs)

- [x] 4.1 Schreibe `test_complete_weekday_task_early`: Complete Mon/Wed/Fri task am Tuesday → completion_due_date=Wednesday, period=(Wed, Wed)
- [x] 4.2 Schreibe `test_complete_weekday_task_on_scheduled_day`: Complete Mon/Wed/Fri task am Monday → completion_due_date=next Monday
- [x] 4.3 Schreibe `test_complete_custom_task_early`: Complete custom task before date → completion_due_date=next custom date
- [x] 4.4 Schreibe `test_allow_exceed_target_weekdays`: Verify can't complete same weekday twice with allow_exceed_target=false
- [x] 4.5 Schreibe `test_allow_exceed_target_different_weekdays`: Verify can complete different weekdays separately
- [x] 4.6 Run integration tests und verifiziere sie **grün** (mit Implementierung fix)

## 5. Fix Implementierung - get_next_due_date

- [x] 5.1 Fix Weekdays case in `get_next_due_date`: Change loop von `0..7` zu `1..=7` (start at from_date+1)
- [x] 5.2 Fix Custom case in `get_next_due_date`: Change filter von `>= from_date` zu `> from_date`
- [x] 5.3 Run unit tests für `get_next_due_date` und verifiziere alle Tests **grün** sind
- [x] 5.4 Verify other recurrence types (Daily, Weekly, Monthly, OneTime) still work correctly

## 6. Verification und Full Test Suite

- [x] 6.1 Run `cargo test -p backend test_get_next_due_date` und verifiziere alle tests grün
- [x] 6.2 Run `cargo test -p backend test_get_period_bounds` und verifiziere alle tests grün
- [x] 6.3 Run `cargo test -p backend test_complete` (integration tests) und verifiziere alle tests grün
- [x] 6.4 Run `cargo test --workspace` und verifiziere keine Regressions (248 tests, alle grün!)
- [x] 6.5 Run `cargo clippy --workspace` und verifiziere keine warnings
- [x] 6.6 Tests stellen sicher dass Habit Tracker korrekt funktioniert (18 Unit Tests + 5 Integration Tests)
