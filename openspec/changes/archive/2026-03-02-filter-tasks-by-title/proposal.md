## Why

Users currently have to manually scan through all tasks to find specific ones by name. When a household has many tasks, this becomes inefficient. A text-based filter allows users to quickly narrow down the task list by entering search terms, improving usability and productivity.

## What Changes

- Add a text input field above task lists on the Dashboard and Household Overview pages
- Implement client-side filtering that shows only tasks whose titles contain the search terms (case-insensitive, word-based matching)
- Filter persists during the session but resets on page reload
- The filter works alongside existing filters (assignment filter, household filter, "show all" toggle)

## Capabilities

### New Capabilities
- `task-text-filter`: Filter tasks by title using a text search field

### Modified Capabilities
- `dashboard`: Add text filter field to dashboard task display
- `tasks`: Add text filter field to household task list view

## Impact

**Frontend changes:**
- `frontend/src/pages/dashboard.rs`: Add text filter input and filtering logic
- `frontend/src/pages/households_overview.rs`: Add text filter input and filtering logic
- Frontend CSS for the text input field styling (mobile-first)

**No backend changes needed** - filtering is implemented client-side on already-fetched task lists.

**No database changes needed** - existing task data structure is sufficient.
