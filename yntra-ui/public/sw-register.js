// Client-side Service Worker registration for Yntra Platform PWA
if ('serviceWorker' in navigator) {
  window.addEventListener('load', () => {
    navigator.serviceWorker.register('/sw.js', { scope: '/' })
      .catch(() => navigator.serviceWorker.register('/public/sw.js', { scope: '/' }))
      .then((registration) => {
        if (!registration) return;
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
        const errMsg = (error && error.message) ? String(error.message) : String(error);
        console.warn(`[Service Worker] Registration note: ${errMsg}`);
      });
  });
}
