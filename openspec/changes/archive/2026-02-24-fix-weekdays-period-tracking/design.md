## Context

The `get_period_bounds()` function in `scheduler.rs` determines how task completions are grouped into periods. Currently, Weekdays recurrence is mapped to `TimePeriod::Week`, causing:

1. All completions in a Mon-Sun week to share one period
2. Sunday completions to be assigned to the **previous** week (due to `num_days_from_monday()` returning 6 for Sunday)

The buggy code (line 227):
```rust
RecurrenceType::Weekly | RecurrenceType::Weekdays => TimePeriod::Week,
```

## Goals / Non-Goals

**Goals:**
- Fix period boundary calculation for Weekdays tasks
- Each scheduled day tracks as its own period (daily granularity)
- New completions show correct dates in period tracker

**Non-Goals:**
- Migrating/fixing existing incorrect period results in database
- Changing behavior for Weekly or other recurrence types

## Decisions

### 1. Use Daily Periods for Weekdays Tasks

**Change:**
```rust
RecurrenceType::Weekly => TimePeriod::Week,
RecurrenceType::Weekdays => TimePeriod::Day,
```

**Rationale:** Weekdays tasks specify individual days (e.g., Sun, Mon, Tue). Each day is a distinct completion opportunity. Users expect to see "completed on Sunday" not "completed in the week of Monday."

**Alternatives considered:**
- Fix the week boundary calculation → Still groups multiple days together, doesn't match user expectation
- Make period type configurable per task → Over-engineering for this use case

### 2. No Data Migration

Existing period results with wrong dates will remain in the database. This is acceptable because:
- Old data shows historical state (even if visually confusing)
- New completions will be correct going forward
- Migration would be complex and risk data integrity

## Risks / Trade-offs

**[Risk] Visual discontinuity in period tracker** → Users may see old weekly periods alongside new daily periods. Mitigation: This is cosmetic and self-corrects as new data accumulates.

**[Risk] Streak calculation may be affected** → Streak logic counts consecutive completed periods. Daily periods may change streak numbers. Mitigation: Verify streak calculation handles mixed period types.
