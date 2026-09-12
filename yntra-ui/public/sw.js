// Service Worker for Yntra Platform PWA
// Handles offline caching of WASM modules, JS bridges, stylesheets, and fallback assets.

const CACHE_NAME = 'yntra-pwa-v2';
const IMMUTABLE_WASM_CACHE = 'yntra-wasm-v2';

const ASSETS_TO_CACHE = [
  '/',
  '/index.html',
  '/manifest.json',
  '/db-bridge.js',
  '/db-worker.js',
  '/sqlite3.js',
  '/dx-components-theme.css',
  '/tailwind.css',
  '/global.css',
  '/sw-register.js'
];

const WASM_ASSETS = [
  '/sqlite3.wasm'
];

// Install Event - Pre-cache core assets safely
self.addEventListener('install', (event) => {
  event.waitUntil(
    (async () => {
      try {
        const cache = await caches.open(CACHE_NAME);
        for (const url of ASSETS_TO_CACHE) {
          try {
            const res = await fetch(url);
            if (res.ok) await cache.put(url, res);
          } catch (_) {}
        }
        const wasmCache = await caches.open(IMMUTABLE_WASM_CACHE);
        for (const url of WASM_ASSETS) {
          try {
            const res = await fetch(url);
            const ct = res.headers.get('content-type') || '';
            if (res.ok && ct.includes('application/wasm')) {
              await wasmCache.put(url, res);
            }
          } catch (_) {}
        }
      } catch (_) {}
      await self.skipWaiting();
    })()
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

  // 1. WASM Binary Modules - Cache First (with MIME validation)
  if (url.pathname.endsWith('.wasm')) {
    event.respondWith(
      caches.open(IMMUTABLE_WASM_CACHE).then((cache) => {
        return cache.match(event.request).then((cachedResponse) => {
          if (cachedResponse) return cachedResponse;
          return fetch(event.request).then((networkResponse) => {
            const ct = networkResponse.headers.get('content-type') || '';
            if (networkResponse.ok && ct.includes('application/wasm')) {
              cache.put(event.request, networkResponse.clone());
            }
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
