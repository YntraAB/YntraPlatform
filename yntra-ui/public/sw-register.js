// Client-side Service Worker registration for Yntra Platform PWA
if ('serviceWorker' in navigator) {
  window.addEventListener('load', () => {
    navigator.serviceWorker.register('/public/sw.js', { scope: '/' })
      .then((registration) => {
        console.log('[Service Worker] Registered successfully with scope:', registration.scope);
        
        registration.onupdatefound = () => {
          const installingWorker = registration.installing;
          if (installingWorker) {
            installingWorker.onstatechange = () => {
              if (installingWorker.state === 'installed' && navigator.serviceWorker.controller) {
                console.log('[Service Worker] New content available; please refresh.');
              }
            };
          }
        };
      })
      .catch((error) => {
        console.warn('[Service Worker] Registration failed:', error);
      });
  });
}
