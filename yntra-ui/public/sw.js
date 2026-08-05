// Service Worker for Yntra Platform PWA
// Handles offline caching of WASM modules, JS bridges, stylesheets, and fallback assets.

const CACHE_NAME = 'yntra-pwa-v1';
const IMMUTABLE_WASM_CACHE = 'yntra-wasm-v1';

const ASSETS_TO_CACHE = [
  '/',
  '/index.html',
  '/public/manifest.json',
  '/public/db-bridge.js',
  '/public/db-worker.js',
  '/public/sqlite3.js',
  '/public/dx-components-theme.css',
  '/public/tailwind.css',
  '/public/global.css',
  '/public/sw-register.js',
  '/db-bridge.js',
  '/db-worker.js',
  '/sqlite3.js',
  '/sw-register.js'
];

const WASM_ASSETS = [
  '/public/sqlite3.wasm',
  '/sqlite3.wasm',
  '/yntra-ui.wasm'
];

// Install Event - Pre-cache core assets
self.addEventListener('install', (event) => {
  event.waitUntil(
    Promise.all([
      caches.open(CACHE_NAME).then((cache) => cache.addAll(ASSETS_TO_CACHE)),
      caches.open(IMMUTABLE_WASM_CACHE).then((cache) => cache.addAll(WASM_ASSETS.filter(a => fetch(a, { method: 'HEAD' }).then(r => r.ok).catch(() => false))))
    ]).then(() => self.skipWaiting())
  );
});

// Activate Event - Clean up stale cache versions
self.addEventListener('activate', (event) => {
  event.waitUntil(
    caches.keys().then((cacheNames) => {
      return Promise.all(
        cacheNames.map((cacheName) => {
          if (cacheName !== CACHE_NAME && cacheName !== IMMUTABLE_WASM_CACHE) {
            console.log('[Service Worker] Deleting obsolete cache:', cacheName);
            return caches.delete(cacheName);
          }
        })
      );
    }).then(() => self.clients.claim())
  );
});

// Fetch Event - Hybrid Cache Strategy (Cache First for WASM binaries, Stale-While-Revalidate for static assets)
self.addEventListener('fetch', (event) => {
  const url = new URL(event.request.url);

  // 1. WASM Binary Modules - Cache First
  if (url.pathname.endsWith('.wasm')) {
    event.respondWith(
      caches.open(IMMUTABLE_WASM_CACHE).then((cache) => {
        return cache.match(event.request).then((cachedResponse) => {
          if (cachedResponse) return cachedResponse;
          return fetch(event.request).then((networkResponse) => {
            cache.put(event.request, networkResponse.clone());
            return networkResponse;
          });
        });
      })
    );
    return;
  }

  // 2. Static Assets & Pages - Stale-While-Revalidate
  if (event.request.method === 'GET') {
    event.respondWith(
      caches.open(CACHE_NAME).then((cache) => {
        return cache.match(event.request).then((cachedResponse) => {
          const fetchPromise = fetch(event.request)
            .then((networkResponse) => {
              if (networkResponse.status === 200) {
                cache.put(event.request, networkResponse.clone());
              }
              return networkResponse;
            })
            .catch(() => {
              // Return cached response or fallback index.html for navigation
              if (event.request.mode === 'navigate') {
                return cache.match('/index.html') || cache.match('/');
              }
            });

          return cachedResponse || fetchPromise;
        });
      })
    );
  }
});
