## Context

The `get_period_bounds` function in `backend/src/services/scheduler.rs` determines how completions are grouped for counting against targets. Currently, Custom recurrence uses `TimePeriod::None`, which returns an all-time period (1970-2100). This was likely unintentional - Custom should behave like Weekdays where each scheduled date is tracked independently.

The recent fix for Weekdays (commit c772ad8) changed it from `TimePeriod::Week` to `TimePeriod::Day` for the same reason. Custom needs the same treatment.

## Goals / Non-Goals

**Goals:**
- Fix Custom recurrence to track each custom date as a separate period
- Maintain backward compatibility with existing Custom tasks
- Add test coverage for Custom period bounds

**Non-Goals:**
- Migrating existing incorrect period results (1970-2100 records) - these will be superseded by new correct records
- Changing OneTime recurrence behavior (it correctly uses all-time periods)

## Decisions

### 1. Use `TimePeriod::Day` for Custom recurrence

**Decision:** Change Custom from `TimePeriod::None` to `TimePeriod::Day`

**Rationale:**
- Custom tasks have specific dates (e.g., [Feb 25, Feb 28, Mar 5])
- Each date should be tracked independently, like Weekdays
- `TimePeriod::Day` gives period bounds of (date, date) for each custom date
- The `completion_due_date` (from `get_next_due_date`) determines which date the completion counts for

**Alternatives considered:**
- Keep `TimePeriod::None`: Would require special-case logic throughout the codebase
- Create `TimePeriod::Custom`: Over-engineering; `Day` works perfectly since each custom date is a single day

### 2. Separate Custom from OneTime in the match arm

**Decision:** Split the combined match arm into separate cases

**Current code:**
```rust
RecurrenceType::Custom | RecurrenceType::OneTime => TimePeriod::None,
```

**New code:**
```rust
RecurrenceType::Custom => TimePeriod::Day,
RecurrenceType::OneTime => TimePeriod::None,
```

**Rationale:**
- OneTime correctly uses all-time periods (no schedule, count all completions ever)
- Custom has a schedule and needs per-date tracking

## Risks / Trade-offs

**[Risk] Existing period results with 1970-2100 dates** → These records remain in the database but will be superseded by new per-date records. The upsert logic in `finalize_period` uses `task_id + period_start` as the key, so new completions create new records with correct dates. Old records become orphaned but harmless.

**[Risk] Users with `allow_exceed_target=false` may see behavior change** → This is the intended fix. Previously they couldn't complete multiple custom dates; now they can complete once per custom date.

**[Trade-off] No migration for old data** → Acceptable because:
1. The fix creates correct records going forward
2. Old period results don't affect new completion logic
3. Period tracker shows most recent periods, so old 1970 records scroll off
