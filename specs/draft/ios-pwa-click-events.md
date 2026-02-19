# iOS PWA Click Events Issue (Draft)

> **Status:** Implemented - Ready for testing
> **Created:** 2026-02-19
> **Type:** Bug

## Problem

On iOS in PWA mode, clicking on a task does not open the details view. The same interaction works correctly in:
- Desktop browsers
- Android PWA
- iOS Safari (non-PWA)

## Root Cause

The `.task-title-clickable` CSS class is missing `touch-action: manipulation`. iOS PWA has a 300ms click delay to detect double-tap zoom gestures. This delay can cause click events to be swallowed or not fire reliably.

Current CSS (`frontend/styles.css:335`):
```css
.task-title-clickable {
    cursor: pointer;
    transition: color 0.15s ease;
}
```

## Fix

Add `touch-action: manipulation` to disable double-tap zoom detection on clickable elements:

```css
.task-title-clickable {
    cursor: pointer;
    transition: color 0.15s ease;
    touch-action: manipulation;
    -webkit-tap-highlight-color: transparent;
}
```

### What `touch-action: manipulation` does

- Enables panning and pinch zoom (standard gestures)
- Disables double-tap zoom detection
- Removes the 300ms click delay
- Click events fire immediately on touch

## Affected Files

| File | Line | Element |
|------|------|---------|
| `frontend/styles.css` | 335 | `.task-title-clickable` class |
| `frontend/src/components/task_card.rs` | 174 | `<span class="task-title-clickable">` |
| `frontend/src/pages/tasks.rs` | 401, 496 | `class="task-title task-title-clickable"` |

## Additional Recommendations

Consider adding a global utility class for all clickable non-link elements:

```css
/* Add to frontend/styles.css */
.clickable,
.task-title-clickable,
[on\:click] {
    touch-action: manipulation;
    -webkit-tap-highlight-color: transparent;
    user-select: none;
}
```

## Other Elements to Check

These may have the same issue:
- [ ] Reward/Punishment rows
- [ ] Note/Journal entry rows
- [ ] Navigation menu items
- [ ] Modal close buttons
- [ ] Dropdown toggles

## Implementation

Added `touch-action: manipulation` to the following CSS classes in `frontend/styles.css`:

| Class | Line | Additional Properties |
|-------|------|----------------------|
| `.task-title-clickable` | ~335 | + `-webkit-tap-highlight-color: transparent` |
| `.btn` | ~209 | |
| `.modal-backdrop` | ~582 | |
| `.fab` | ~1661 | |
| `.household-picker-item` | ~1766 | |

## Testing Checklist

- [ ] iOS Safari (non-PWA)
- [ ] iOS PWA (Add to Home Screen)
- [ ] iPadOS Safari
- [ ] iPadOS PWA
- [ ] Android Chrome
- [ ] Android PWA
- [ ] Desktop browsers

## References

- [MDN: touch-action](https://developer.mozilla.org/en-US/docs/Web/CSS/touch-action)
- [300ms tap delay in iOS](https://webkit.org/blog/5610/more-responsive-tapping-on-ios/)
