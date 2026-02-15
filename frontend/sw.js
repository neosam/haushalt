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
  // Force immediate activation
  self.skipWaiting();

  event.waitUntil(
    // Fetch index.html to extract the JS bundle hash as version
    fetch('/index.html')
      .then(response => response.text())
      .then(html => {
        // Extract hash from script src like: frontend-abc123_bg.js
        const match = html.match(/frontend-([a-f0-9]+)_bg\.js/);
        if (match) {
          CACHE_NAME = `household-${match[1]}`;
        }
        return caches.open(CACHE_NAME);
      })
      .then(cache => cache.addAll(SHELL_URLS))
  );
});

// Activate: clean old caches
self.addEventListener('activate', event => {
  event.waitUntil(
    caches.keys().then(keys =>
      Promise.all(keys.filter(k => k !== CACHE_NAME).map(k => caches.delete(k)))
    ).then(() => self.clients.claim())
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
