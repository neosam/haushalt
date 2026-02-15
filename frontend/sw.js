// Dynamic cache name based on app version - extracted from index.html
let CACHE_NAME = 'household-v1';

// Assets to always try to cache
const SHELL_URLS = [
  '/',
  '/index.html',
  '/manifest.json',
  '/icons/icon.svg',
  '/icons/icon-maskable.svg'
];

// Install: extract version from index.html and precache app shell
self.addEventListener('install', event => {
  console.log('SW: Installing new version');

  event.waitUntil(
    // Fetch index.html to extract the JS bundle hash as version
    fetch('/index.html', { cache: 'no-store' })
      .then(response => response.text())
      .then(html => {
        // Extract hash from script src like: frontend-abc123_bg.js
        const match = html.match(/frontend-([a-f0-9]+)_bg\.js/);
        if (match) {
          CACHE_NAME = `household-${match[1]}`;
          console.log('SW: Cache version:', CACHE_NAME);
        }
        return caches.open(CACHE_NAME);
      })
      .then(cache => cache.addAll(SHELL_URLS))
      .then(() => {
        // Force immediate activation
        console.log('SW: Skip waiting');
        return self.skipWaiting();
      })
  );
});

// Activate: clean old caches and take control immediately
self.addEventListener('activate', event => {
  console.log('SW: Activating, taking control');
  event.waitUntil(
    caches.keys().then(keys =>
      Promise.all(keys.filter(k => k !== CACHE_NAME).map(k => {
        console.log('SW: Deleting old cache:', k);
        return caches.delete(k);
      }))
    ).then(() => {
      console.log('SW: Claiming clients');
      return self.clients.claim();
    })
  );
});

// Fetch: network-first for API, cache-first for assets
self.addEventListener('fetch', event => {
  const url = new URL(event.request.url);

  // Skip API requests - always go to network
  if (url.pathname.startsWith('/api/')) {
    return;
  }

  event.respondWith(
    caches.match(event.request)
      .then(cached => cached || fetch(event.request)
        .then(response => {
          // Cache successful responses
          if (response.ok) {
            const clone = response.clone();
            caches.open(CACHE_NAME).then(cache => cache.put(event.request, clone));
          }
          return response;
        })
      )
      .catch(() => caches.match('/index.html')) // Fallback to app shell
  );
});
