## 1. Dashboard Page Implementation

- [x] 1.1 Add text_filter signal to dashboard.rs state
- [x] 1.2 Implement word-based filtering logic in dashboard.rs memo
- [x] 1.3 Add text input field UI in dashboard.rs view (below household filter)
- [x] 1.4 Ensure filter combines with show_all toggle
- [x] 1.5 Ensure filter combines with household filter
- [x] 1.6 Ensure filter combines with assignment filter
- [x] 1.7 Add CSS styling for text filter input (mobile-first)

## 2. Household Page Implementation

- [x] 2.1 Add text_filter signal to household.rs state
- [x] 2.2 Implement word-based filtering logic in household.rs memo
- [x] 2.3 Add text input field UI in household.rs view (below assignment filter)
- [x] 2.4 Ensure filter combines with assignment filter
- [x] 2.5 Ensure filter applies to both active and archived task sections
- [x] 2.6 Add CSS styling for text filter input (mobile-first)

## 3. Testing

- [ ] 3.1 Manual test: Dashboard filter with single word
- [ ] 3.2 Manual test: Dashboard filter with multiple words
- [ ] 3.3 Manual test: Dashboard filter case-insensitivity
- [ ] 3.4 Manual test: Dashboard filter with show_all toggle
- [ ] 3.5 Manual test: Dashboard filter with household filter
- [ ] 3.6 Manual test: Household page filter with single word
- [ ] 3.7 Manual test: Household page filter with assignment filter
- [ ] 3.8 Manual test: Household page filter on archived tasks
- [ ] 3.9 Verify filter clears on navigation
- [ ] 3.10 Test mobile responsiveness on narrow screens

## 4. Quality Assurance

- [x] 4.1 Run cargo check --workspace (ensure no warnings)
- [x] 4.2 Run cargo clippy --workspace (ensure no warnings)
- [ ] 4.3 Test in different browsers (Chrome, Firefox, Safari)
- [ ] 4.4 Verify no console errors in browser dev tools

## 5. Bug Fixes

- [x] 5.1 Fix input focus loss: Make text input uncontrolled (remove prop:value)
- [x] 5.2 Fix input focus loss properly: Use node_ref to prevent input re-rendering
- [x] 5.3 Fix input focus loss with separate component and stable callbacks

## 6. Critical Fixes (from Code Review)

- [x] 6.1 Fix Household page TextFilterInput placement: Move inside household context block for consistency
- [x] 6.2 Remove inline style (padding: 1rem) from household.rs, use CSS class instead

## 7. Code Quality Improvements (from Code Review)

- [x] 7.1 Add documentation comment explaining untrack usage in TextFilterInput component
- [x] 7.2 Extract duplicate filter logic into shared helper function
- [x] 7.3 Audit filtered_tasks memo redundancy in dashboard.rs (check if double filtering occurs)
- [x] 7.4 Replace CSS magic numbers with CSS variables (--input-padding, --input-max-width)

## 8. Testing & Accessibility (from Code Review)

- [x] 8.1 Add comprehensive WASM tests for TextFilterInput component (focus preservation, callback behavior, untrack usage)
- [x] 8.2 Add unit tests for filter logic (single word, multiple words, case-insensitive) - DONE in 7.2
- [x] 8.3 Add aria-label to text filter input for screen readers
- [x] 8.4 Consider adding id attribute for potential label association

## 9. Critical Bug Fix

- [x] 9.1 Fix TextFilterInput re-creation bug: Move outside reactive household block while maintaining visual position
