# PWA Architecture

## Overview

The application is implemented as a Progressive Web App (PWA), enabling installation on devices, offline access to cached content, and automatic updates.

## Components

### 1. Web App Manifest (`frontend/manifest.json`)

Defines the app's identity and appearance when installed:

```json
{
  "name": "Household Manager",
  "short_name": "Household",
  "description": "Manage your household tasks, rewards, and more",
  "start_url": "/",
  "display": "standalone",
  "background_color": "#f8fafc",
  "theme_color": "#4f46e5",
  "orientation": "any",
  "icons": [
    {
      "src": "/icons/icon-192.png",
      "sizes": "192x192",
      "type": "image/png"
    },
    {
      "src": "/icons/icon-512.png",
      "sizes": "512x512",
      "type": "image/png"
    },
    {
      "src": "/icons/icon.svg",
      "sizes": "any",
      "type": "image/svg+xml"
    },
    {
      "src": "/icons/icon-maskable.svg",
      "sizes": "any",
      "type": "image/svg+xml",
      "purpose": "maskable"
    }
  ]
}
```

**Key settings:**
- `display: standalone` - App runs without browser UI (address bar, etc.)
- `theme_color` - Colors the browser chrome and status bar
- **PNG icons at 192x192 and 512x512 are required** for Chrome/Chromium installability
- SVG icons for scalability (fallback)
- Maskable icon variant for adaptive icon support on Android

### 2. Service Worker (`frontend/sw.js`)

The service worker handles caching and offline functionality.

#### Cache Versioning

The cache name includes the build hash, injected at build time by a Trunk post_build hook:

```javascript
let CACHE_NAME = 'household-34c00ad0a2cae6e8';  // Hash injected at build time
```

**Build-time injection** (Trunk.toml):
```toml
[[hooks]]
stage = "post_build"
command = "sh"
command_arguments = ["-c", """
HASH=$(grep -oP 'frontend-\\K[a-f0-9]+(?=\\.js)' $TRUNK_STAGING_DIR/index.html | head -1)
sed -i "s/__BUILD_HASH__/$HASH/g" $TRUNK_STAGING_DIR/sw.js
"""]
```

The source `sw.js` contains a placeholder `__BUILD_HASH__` which is replaced with the actual WASM bundle hash during each build. This ensures:
1. Each deployment gets a unique cache name automatically
2. The service worker file content changes, triggering browser updates
3. No manual version bumping required

**Runtime fallback**: If the placeholder isn't replaced (e.g., during development), the service worker extracts the hash from `index.html` at install time:

```javascript
const match = html.match(/frontend-([a-f0-9]+)\.js/);
if (match) {
  CACHE_NAME = `household-${match[1]}`;
}
```

#### Lifecycle Events

**Install Event:**
1. Fetches `index.html` to extract bundle hash for cache versioning
2. Precaches the app shell (essential files for offline startup):
   - `/` and `/index.html`
   - `/manifest.json`
   - `/icons/icon.svg` and `/icons/icon-maskable.svg`
3. Calls `skipWaiting()` to activate immediately

**Activate Event:**
1. Deletes all old caches (those not matching current `CACHE_NAME`)
2. Calls `clients.claim()` to take control of all open tabs immediately

#### Caching Strategies

```
┌─────────────────────────────────────────────────────────────────┐
│                        Fetch Request                            │
└─────────────────────────────────────────────────────────────────┘
                              │
                              ▼
              ┌───────────────────────────────┐
              │  Is it an API request?        │
              │  (/api/*)                     │
              └───────────────────────────────┘
                    │                │
                   Yes              No
                    │                │
                    ▼                ▼
              ┌──────────┐   ┌───────────────────────────┐
              │ Bypass   │   │ Is it HTML/JS/WASM/CSS?   │
              │ (no SW)  │   │ (.html, .js, .wasm, .css) │
              └──────────┘   └───────────────────────────┘
                                   │                │
                                  Yes              No
                                   │                │
                                   ▼                ▼
                            ┌────────────┐   ┌────────────┐
                            │ Network    │   │ Cache      │
                            │ First      │   │ First      │
                            └────────────┘   └────────────┘
```

**1. API Requests (`/api/*`):** Bypass service worker entirely - always go to network.

**2. App Files (HTML, JS, WASM, CSS):** Network-first strategy
   - Try network first to get latest version
   - Cache successful responses
   - Fall back to cache if offline
   - Ultimate fallback to cached `/index.html` (SPA routing)
   - CSS included here to ensure style updates deploy immediately

**3. Static Assets (icons, manifest):** Cache-first strategy
   - Check cache first for fast loading
   - Fetch from network if not cached
   - Cache successful network responses
   - Fall back to `/index.html` on complete failure

### 3. Service Worker Registration (`frontend/index.html`)

Registration happens in the HTML to ensure it runs before the WASM app loads:

```javascript
if ('serviceWorker' in navigator) {
    navigator.serviceWorker.register('/sw.js', { updateViaCache: 'none' })
        .then(reg => {
            console.log('SW registered');
            reg.update();  // Check for updates on every page load
        })
        .catch(err => console.error('SW registration failed:', err));

    // Auto-reload when new SW takes control
    let refreshing = false;
    navigator.serviceWorker.addEventListener('controllerchange', () => {
        if (!refreshing) {
            refreshing = true;
            window.location.reload();
        }
    });
}
```

**Key behaviors:**
- `updateViaCache: 'none'` - Always fetch fresh `sw.js` from network
- `reg.update()` - Proactively checks for SW updates on page load
- `controllerchange` listener - Auto-reloads page when new SW activates

### 4. Mobile Viewport Configuration

> **Status:** Implemented

The viewport must be configured to prevent zoom and make the app feel native on mobile devices:

```html
<meta name="viewport" content="width=device-width, initial-scale=1.0, maximum-scale=1.0, user-scalable=no, viewport-fit=cover">
```

**Key settings:**
- `maximum-scale=1.0` - Prevents pinch-to-zoom
- `user-scalable=no` - Disables manual zooming
- `viewport-fit=cover` - Extends content to screen edges (notch support)

This ensures the app behaves like a native mobile application rather than a zoomable website.

### 5. iOS PWA Support

Special meta tags in `index.html` for iOS Safari:

```html
<meta name="apple-mobile-web-app-capable" content="yes">
<meta name="apple-mobile-web-app-status-bar-style" content="default">
<meta name="apple-mobile-web-app-title" content="Household">
<link rel="apple-touch-icon" href="/icons/icon.svg">
```

### 6. Build Integration

**Trunk.toml** watches PWA files for development:
```toml
watch = ["src", "index.html", "styles.css", "manifest.json", "sw.js", "favicon.svg", "icons"]
```

**index.html** copies files to dist:
```html
<link data-trunk rel="copy-file" href="manifest.json" />
<link data-trunk rel="copy-file" href="sw.js" />
<link data-trunk rel="copy-dir" href="icons" />
```

## Update Flow

```
┌─────────────────────────────────────────────────────────────────┐
│  1. User loads page                                             │
└─────────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────────┐
│  2. Browser fetches sw.js (updateViaCache: none)                │
└─────────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────────┐
│  3. If sw.js changed: new SW enters "installing" state          │
└─────────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────────┐
│  4. Install event: fetch index.html, extract new bundle hash,   │
│     create new cache, precache app shell                        │
└─────────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────────┐
│  5. skipWaiting() → SW immediately activates                    │
└─────────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────────┐
│  6. Activate event: delete old caches, claim clients            │
└─────────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────────┐
│  7. controllerchange fires → page auto-reloads with new version │
└─────────────────────────────────────────────────────────────────┘
```

## File Structure

```
frontend/
├── index.html          # SW registration, iOS meta tags
├── manifest.json       # PWA manifest
├── sw.js              # Service worker
├── favicon.svg        # Browser tab icon
└── icons/
    ├── icon-192.png       # Required for Chrome installability
    ├── icon-512.png       # Required for Chrome installability
    ├── apple-touch-icon.png  # iOS home screen icon (180x180)
    ├── icon.svg           # Source SVG icon
    └── icon-maskable.svg  # Adaptive icon for Android
```

## Offline Behavior

| Content Type | Offline Behavior |
|--------------|------------------|
| App shell (HTML, JS, WASM, CSS) | Served from cache |
| Static assets (icons, manifest) | Served from cache |
| API requests | Fail (network required) |
| Navigation to any route | Falls back to cached index.html (SPA handles routing) |

## Browser Support

| Browser | Desktop Install | Mobile Install |
|---------|-----------------|----------------|
| Chrome/Chromium | ✓ | ✓ |
| Vivaldi | ✓ | ✓ |
| Edge | ✓ | ✓ |
| Firefox | ✗ (removed in 2021) | ✓ (Android) |
| Safari | ✗ | ✓ (iOS only, via Share → Add to Home Screen) |

**Note:** On iOS, only Safari can install PWAs. Third-party browsers (Chrome, Firefox, Vivaldi) on iOS cannot install PWAs due to Apple restrictions.

## Limitations

1. **No offline data sync** - API requests require network; no background sync implemented
2. **No push notifications** - Not implemented
3. **Cache size not managed** - No automatic cache eviction beyond version-based cleanup
4. **Firefox desktop** - Does not support PWA installation
