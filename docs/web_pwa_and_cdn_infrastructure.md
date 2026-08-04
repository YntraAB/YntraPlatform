# Web PWA & Edge CDN Infrastructure

Comprehensive documentation for Yntra Platform's Progressive Web App (PWA) offline capabilities, Service Worker caching strategies, origin headers, and Cloudflare Pages Edge CDN deployment.

---

## 1. Architecture Overview

Yntra Platform web applications run as local-first WASM modules using SQLite in browser Origin Private File System (OPFS) storage. To achieve sub-millisecond launch times and full offline operation, the platform utilizes:

- **Service Worker (`sw.js`)**: Implements dual caching Tiers:
  - **Immutable WASM Cache (`yntra-wasm-v1`)**: Cache-First strategy for heavy compiled binaries (`sqlite3.wasm`, `yntra-ui.wasm`).
  - **App Shell Cache (`yntra-pwa-v1`)**: Stale-While-Revalidate strategy for static JS bridges, CSS themes, and HTML entrypoints.
- **Web App Manifest (`manifest.json`)**: Declares app identity, dark theme (`#0f172a`), standalone window mode, and icon masks.
- **Origin Security Headers (`_headers`)**: Enforces COOP/COEP isolation headers required by browser high-performance `SharedArrayBuffer` and OPFS multithreading.

---

## 2. PWA & Caching Tiers

### Service Worker Caching Strategies

| Asset Category | Target Files | Caching Strategy | Rationale |
| :--- | :--- | :--- | :--- |
| **WASM Binaries** | `sqlite3.wasm`, `yntra-ui.wasm` | **Cache-First** | WASM binaries are content-hashed; caching prevents multi-megabyte re-downloads. |
| **App Shell & Theme** | `index.html`, `tailwind.css`, `global.css` | **Stale-While-Revalidate** | Instant boot from cache while silently fetching updates in background. |
| **JS Bridges** | `db-bridge.js`, `db-worker.js` | **Stale-While-Revalidate** | Guarantees OPFS worker scripts remain in sync with database migrations. |
| **Service Worker** | `sw.js` | **No-Cache (Always Revalidate)** | Ensures browser checks for updated Service Worker logic on every navigation. |

---

## 3. COOP & COEP Security Headers

Browser OPFS SQLite multi-threading requires `SharedArrayBuffer`, which browsers restrict unless Cross-Origin Isolation is active.

### Required HTTP Headers (`packaging/web/_headers`)

```http
Cross-Origin-Opener-Policy: same-origin
Cross-Origin-Embedder-Policy: require-corp
Cross-Origin-Resource-Policy: same-site
Strict-Transport-Security: max-age=31536000; includeSubDomains; preload
```

---

## 4. Edge CDN Deployment & Custom Domain

Cloudflare Pages provides global Edge CDN distribution, automated SSL termination, and HSTS enforcement.

### Cloudflare Wrangler Configuration (`packaging/web/wrangler.toml`)

```toml
name = "yntra-platform-web"
pages_build_output_dir = "target/dx/yntra-ui/release/web/public"
compatibility_date = "2026-08-04"

[env.production]
routes = [
  { pattern = "app.yntra.se", custom_domain = true },
  { pattern = "yntra.app", custom_domain = true }
]
```

### Manual CLI Deployment Command

```bash
# Build production web bundle
cd yntra-ui
dx build --release

# Copy Cloudflare security headers
cp ../packaging/web/_headers target/dx/yntra-ui/release/web/public/

# Deploy using Wrangler CLI
npx wrangler pages deploy target/dx/yntra-ui/release/web/public --project-name yntra-platform-web
```

---

## 5. CI/CD Pipeline

The `.github/workflows/web-release.yml` GitHub Actions pipeline automatically builds WASM targets and deploys to Cloudflare Pages Edge CDN on every push to `main`.
