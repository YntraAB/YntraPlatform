// Sentry Browser SDK initialization script for Yntra Platform Dioxus UI
(function () {
  const sentryDsn = window.__YNTRA_SENTRY_DSN__ || "https://public@sentry.io/1234567";
  const environment = window.__YNTRA_ENV__ || "production";
  const release = window.__YNTRA_RELEASE__ || "yntra-ui@0.1.0";

  if (window.Sentry) {
    window.Sentry.init({
      dsn: sentryDsn,
      environment: environment,
      release: release,
      tracesSampleRate: 0.1,
      replaysSessionSampleRate: 0.05,
      replaysOnErrorSampleRate: 1.0,
      integrations: [
        new window.Sentry.BrowserTracing(),
        new window.Sentry.Replay()
      ],
      beforeSend(event, hint) {
        // Intercept WASM panic stack traces
        if (hint && hint.originalException && hint.originalException.message) {
          if (hint.originalException.message.includes('unreachable') || hint.originalException.message.includes('panicked')) {
            event.tags = event.tags || {};
            event.tags['crash.type'] = 'wasm_panic';
          }
        }
        return event;
      }
    });

    console.log('[Sentry] Initialized Browser Crash & Error Reporter for environment:', environment);
  }
})();
