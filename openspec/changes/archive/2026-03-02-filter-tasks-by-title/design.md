## Context

Both the Dashboard (`frontend/src/pages/dashboard.rs`) and Household page (`frontend/src/pages/household.rs`) display task lists. Currently, users must visually scan through all tasks to find specific ones.

Both pages already have filter mechanisms:
- Dashboard: `show_all` toggle, household filter (`enabled_households`), assignment filter (`show_only_assigned`)
- Household page: Assignment filter (`show_only_assigned`)

The task lists are rendered using the `GroupedTaskList` component, which receives filtered task vectors.

## Goals / Non-Goals

**Goals:**
- Add client-side text filtering to both Dashboard and Household task lists
- Filter by task title using case-insensitive, word-based matching
- Combine seamlessly with existing filters using AND logic
- Reset filter on page navigation (session-scoped only)
- Provide real-time filtering as user types

**Non-Goals:**
- Backend changes or database queries (purely client-side)
- Filtering by task description, category, or other fields
- Persistent filter state across page reloads
- Full-text search or fuzzy matching

## Decisions

### Decision 1: Client-Side Filtering Only

**Choice:** Implement filtering purely in the frontend, applying to already-fetched task lists.

**Rationale:**
- Task lists are already loaded into memory
- Typical household task counts are low (tens, not thousands)
- No latency from API round-trips
- Simpler implementation with no backend changes

**Alternatives Considered:**
- Backend filtering via API query parameters: Adds complexity, requires migration, doesn't improve performance for small datasets

### Decision 2: Word-Based Case-Insensitive Matching

**Choice:** Split search input by whitespace, check if ALL words appear in task title (case-insensitive, word order independent).

**Rationale:**
- Matches user expectations ("clean kitchen" finds "Kitchen Cleaning")
- More flexible than exact substring match
- Simple to implement with `.to_lowercase()` and `.contains()`

**Example:**
- Search: "clean kitchen"
- Matches: "Clean Kitchen", "Kitchen Cleaning", "kitchen deep clean"
- No match: "cleaning bedroom"

**Alternatives Considered:**
- Exact substring match: Too rigid
- Fuzzy matching (Levenshtein distance): Overkill for this use case

### Decision 3: Signal-Based Reactive Filtering

**Choice:** Add `text_filter` as a `RwSignal<String>`, create a `Memo` that filters tasks reactively.

**Rationale:**
- Leptos reactive system automatically re-renders on signal changes
- Consistent with existing filter patterns in both pages
- Real-time updates as user types

**Implementation Pattern:**
```rust
let text_filter = create_rw_signal(String::new());

let filtered_tasks = create_memo(move |_| {
    let filter_text = text_filter.get().to_lowercase();
    let words: Vec<&str> = filter_text.split_whitespace().collect();

    tasks.get()
        .into_iter()
        .filter(|task| {
            if words.is_empty() {
                return true;
            }
            let title_lower = task.title.to_lowercase();
            words.iter().all(|word| title_lower.contains(word))
        })
        .collect::<Vec<_>>()
});
```

**Alternatives Considered:**
- Callback-based filtering: Less idiomatic in Leptos, more boilerplate

### Decision 4: Placement Below Existing Filters

**Choice:** Place text filter input below existing toggles/filters, above task list.

**Rationale:**
- Maintains visual hierarchy: toggles → filters → content
- Consistent position across both pages
- Doesn't disrupt existing UI layout

**Dashboard Order:**
1. "Show All" toggle
2. Household filter
3. Text filter (new)
4. Task list

**Household Order:**
1. Assignment filter toggle
2. Text filter (new)
3. Task list

### Decision 5: Standard Text Input Styling

**Choice:** Use existing form input styles from the project, ensure mobile-first responsive design.

**Rationale:**
- Visual consistency with other inputs (modals, forms)
- Leverages existing CSS
- Mobile-first ensures touch-friendly on small screens

**CSS Approach:**
```css
/* Base mobile styles */
.text-filter-input {
    width: 100%;
    padding: 0.75rem;
    font-size: 1rem;
}

/* Desktop enhancement */
@media (min-width: 768px) {
    .text-filter-input {
        max-width: 400px;
    }
}
```

## Risks / Trade-offs

### [Performance with large task lists]
→ **Mitigation:** Acceptable for typical use (< 100 tasks). Re-filtering on every keystroke is fast enough for this scale. If households exceed 200+ tasks, consider debouncing input.

### [No filter state persistence]
→ **Trade-off:** Filter resets on navigation, consistent with existing filters. Users may need to re-enter text after page reload. This simplicity is preferred over localStorage persistence.

### [Single-language search only]
→ **Trade-off:** Filtering happens on task titles in their stored language. No translation-aware search. This is acceptable given the i18n system translates UI strings, not user content.

### [Archived tasks section behavior]
→ **Mitigation:** On Household page, filter applies to both active and archived sections. Empty sections should be hidden when no matches.
