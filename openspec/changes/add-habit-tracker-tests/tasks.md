## 1. Test Infrastructure Setup

- [x] 1.1 Create `backend/src/test_utils.rs` module
- [x] 1.2 Implement `create_test_pool()` function for in-memory database setup
- [x] 1.3 Implement `run_migrations()` helper to apply migrations to test database
- [x] 1.4 Implement `create_test_household()` fixture with sensible defaults
- [x] 1.5 Implement `create_test_user()` fixture with role parameter
- [x] 1.6 Implement `create_test_membership()` fixture linking user to household
- [x] 1.7 Implement `create_test_task()` builder with fluent API for task creation
- [x] 1.8 Add `test_utils` module declaration to `backend/src/lib.rs` with `#[cfg(test)]`

## 2. Assertion Helpers

- [x] 2.1 Implement `assert_completion_exists()` helper for verifying task completions
- [x] 2.2 Implement `assert_completion_not_exists()` helper
- [x] 2.3 Implement `assert_period_result()` helper for verifying period results
- [x] 2.4 Implement `assert_streak()` helper for verifying streak values
- [x] 2.5 Implement `assert_points_balance()` helper for verifying user points
- [x] 2.6 Implement `assert_activity_logged()` helper for verifying activity logs

## 3. Task Service Tests - Task Creation

- [x] 3.1 Test create task with all required fields
- [x] 3.2 Test create task with minimal fields (defaults applied)
- [x] 3.3 Test create Daily task
- [x] 3.4 Test create Weekly task with specific weekday
- [x] 3.5 Test create Monthly task with specific day
- [x] 3.6 Test create Weekdays task with multiple days
- [x] 3.7 Test create Custom recurrence task with date list
- [x] 3.8 Test create OneTime task
- [x] 3.9 Test create task with category assignment
- [x] 3.10 Test create task with assigned user

## 4. Task Service Tests - Task Completion

- [x] 4.1 Test complete assigned task by assigned user
- [x] 4.2 Test complete unassigned task by any user
- [x] 4.3 Test reject completion of task assigned to different user
- [x] 4.4 Test completion with requires_review=true creates Pending status
- [x] 4.5 Test completion with requires_review=false creates Approved status
- [ ] 4.6 Test completion without review awards points immediately (requires points service integration)
- [ ] 4.7 Test completion with review does not award points until approval (requires points service integration)
- [x] 4.8 Test uncomplete removes completion record
- [ ] 4.9 Test uncomplete reverts points (requires points service integration)
- [x] 4.10 Test cannot uncomplete someone else's completion

## 5. Task Service Tests - Streak Logic

- [ ] 5.1 Test streak increments on consecutive daily completions
- [ ] 5.2 Test streak resets to 0 on missed period
- [ ] 5.3 Test best_streak preserved when current_streak resets
- [ ] 5.4 Test best_streak updated when current_streak exceeds it
- [ ] 5.5 Test streak preserved during task pause
- [ ] 5.6 Test streak preserved during vacation mode
- [ ] 5.7 Test streak calculation across multiple recurrence types

## 6. Period Results Service Tests - Period Finalization

- [ ] 6.1 Test period result created when completion reaches target
- [ ] 6.2 Test period result status is 'completed' when target met
- [ ] 6.3 Test target_count frozen at finalization time
- [ ] 6.4 Test period result updated if already exists (failed → completed)
- [ ] 6.5 Test period result deleted when uncomplete drops below target
- [ ] 6.6 Test failed period finalization for incomplete yesterday period
- [ ] 6.7 Test background job marks failed periods correctly
- [ ] 6.8 Test background job skips already finalized periods

## 7. Period Results Service Tests - Skipped Periods

- [ ] 7.1 Test paused task period marked as 'skipped'
- [ ] 7.2 Test vacation mode marks all task periods as 'skipped'
- [ ] 7.3 Test skipped periods excluded from completion rate calculation
- [ ] 7.4 Test skipped periods don't break streak
- [ ] 7.5 Test skipped periods show correct visual indicator

## 8. Period Results Service Tests - Early Completion

- [ ] 8.1 Test Weekdays early completion on non-scheduled day
- [ ] 8.2 Test Weekdays completion on scheduled day
- [ ] 8.3 Test Custom early completion before next date
- [ ] 8.4 Test Custom completion on scheduled date
- [ ] 8.5 Test completion_due_date calculation for all recurrence types
- [ ] 8.6 Test period bounds calculation for early completions

## 9. Period Results Service Tests - Multiple Completions

- [ ] 9.1 Test multiple completions allowed when allow_exceed_target=true
- [ ] 9.2 Test second completion rejected when allow_exceed_target=false
- [ ] 9.3 Test cannot complete same weekday twice with allow_exceed_target=false
- [ ] 9.4 Test can complete different weekdays separately
- [ ] 9.5 Test cannot complete same custom date twice
- [ ] 9.6 Test can complete different custom dates separately

## 10. Period Results Service Tests - Statistics

- [ ] 10.1 Test completion rate calculation excludes skipped periods
- [ ] 10.2 Test completion rate formula: completed / (completed + failed)
- [ ] 10.3 Test current streak counts consecutive completed periods
- [ ] 10.4 Test best streak finds longest completed run
- [ ] 10.5 Test statistics handle zero completions gracefully

## 11. Task Consequences Service Tests - Good Habits

- [ ] 11.1 Test good habit completion awards positive points
- [ ] 11.2 Test good habit completion applies linked rewards
- [ ] 11.3 Test good habit miss (failed period) deducts penalty points
- [ ] 11.4 Test good habit miss applies linked punishments
- [ ] 11.5 Test good habit with no points configured (no-op)

## 12. Task Consequences Service Tests - Bad Habits

- [ ] 12.1 Test bad habit completion (indulgence) deducts penalty points
- [ ] 12.2 Test bad habit completion applies linked punishments
- [ ] 12.3 Test bad habit resistance (failed period) awards positive points
- [ ] 12.4 Test bad habit resistance applies linked rewards
- [ ] 12.5 Test bad habit with no points configured (no-op)

## 13. Task Consequences Service Tests - Pause/Vacation

- [ ] 13.1 Test paused task skips penalty on period end
- [ ] 13.2 Test paused task allows manual completion
- [ ] 13.3 Test vacation mode skips all task penalties
- [ ] 13.4 Test vacation mode allows manual completions
- [ ] 13.5 Test consequences resume after unpause
- [ ] 13.6 Test consequences resume after vacation ends

## 14. Background Jobs Service Tests - Auto-Archiving

- [ ] 14.1 Test auto-archive one-time task after completion and grace period
- [ ] 14.2 Test auto-archive custom task after last date and grace period
- [ ] 14.3 Test never auto-archive incomplete one-time tasks
- [ ] 14.4 Test configurable grace period from household settings
- [ ] 14.5 Test TaskAutoArchived activity logged on auto-archive
- [ ] 14.6 Test background job processes multiple households correctly

## 15. Background Jobs Service Tests - Period Finalization

- [ ] 15.1 Test background job finalizes periods per household timezone
- [ ] 15.2 Test "yesterday" calculation respects household timezone
- [ ] 15.3 Test multiple households in different timezones processed correctly
- [ ] 15.4 Test background job handles paused tasks correctly
- [ ] 15.5 Test background job handles vacation mode correctly

## 16. Integration Tests - Complete Workflows

- [ ] 16.1 Test complete daily habit workflow (create, complete, verify streak)
- [ ] 16.2 Test complete weekly habit workflow with multiple weeks
- [ ] 16.3 Test complete custom recurrence workflow with early completion
- [ ] 16.4 Test complete vacation mode workflow (pause, skip, resume)
- [ ] 16.5 Test complete good habit workflow (complete, miss, points)
- [ ] 16.6 Test complete bad habit workflow (indulge, resist, points)

## 17. Edge Cases and Timezone Tests

- [ ] 17.1 Test timezone handling for period finalization across DST change
- [ ] 17.2 Test timezone handling for households in UTC+/UTC- timezones
- [ ] 17.3 Test leap year handling for monthly tasks on Feb 29
- [ ] 17.4 Test end-of-month handling for monthly tasks (day 31 in Feb)
- [ ] 17.5 Test task created before period and completed in same period
- [ ] 17.6 Test concurrent completions by different users (unassigned task)

## 18. Verification and Cleanup

- [ ] 18.1 Run `cargo test --workspace` and verify all tests pass
- [ ] 18.2 Run `cargo clippy --workspace` and verify no warnings
- [ ] 18.3 Verify test count increased from ~131 to 200+
- [ ] 18.4 Review test coverage - ensure all spec scenarios have corresponding tests
- [ ] 18.5 Update documentation if needed (test README)
