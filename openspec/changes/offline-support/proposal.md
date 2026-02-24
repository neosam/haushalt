## Why

Users need to view their tasks even when offline (e.g., in areas with poor connectivity, subway, airplane mode). Currently, the app requires an active internet connection to display any data.

## What Changes

- Add read-only offline support for task viewing
- Cache task data locally using IndexedDB
- Display offline indicator when connection is lost
- Auto-sync when connection is restored (server wins)
- Disable interactive actions (complete/edit/create) while offline

## Capabilities

### New Capabilities

- `offline-support`: Read-only offline task viewing with local caching, sync indicator, and automatic reconnection sync

### Modified Capabilities

(None - this adds new functionality without changing existing requirements)

## Impact

- **Frontend**: New IndexedDB caching layer, offline detection, UI indicators
- **API Client**: Network-first strategy with cache fallback
- **UX**: Users see "offline" indicator, disabled action buttons when offline
- **Storage**: IndexedDB stores for Tasks, TaskWithStatus, Households
