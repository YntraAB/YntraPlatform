# End-to-End (E2E) UI Testing with Playwright

Yntra Platform utilizes **Playwright test automation** ([`playwright.config.ts`](file:///c:/Users/hellich/Desktop/YntraPlatform/playwright.config.ts) & [`e2e/specs/user_journey.spec.ts`](file:///c:/Users/hellich/Desktop/YntraPlatform/e2e/specs/user_journey.spec.ts)) for automated end-to-end regression testing across Desktop and Mobile browser viewports.

---

## 1. Test Configuration & Lifecycle

Playwright automatically launches the Dioxus web dev server (`dx serve`) before executing browser test suites:

```typescript
// playwright.config.ts
export default defineConfig({
  testDir: './e2e/specs',
  webServer: {
    command: 'dx serve',
    url: 'http://localhost:8080',
    timeout: 120 * 1000,
  },
});
```

---

## 2. Multi-Browser Matrix

Test suites execute concurrently across 5 browser target profiles:

| Target Name | Engine | Viewport Profile |
| :--- | :--- | :--- |
| `chromium` | Desktop Chromium | $1280 \times 720$ |
| `firefox` | Desktop Gecko | $1280 \times 720$ |
| `webkit` | Desktop WebKit / Safari | $1280 \times 720$ |
| `Mobile Chrome` | Android Chromium | Pixel 5 Emulation |
| `Mobile Safari` | iOS WebKit | iPhone 12 Emulation |

---

## 3. Running E2E Test Suites

```bash
# Install Playwright browser binaries
npx playwright install

# Execute headless E2E test suite
npx playwright test

# Open interactive Playwright UI mode
npx playwright test --ui

# View HTML test execution & failure trace report
npx playwright show-report
```

---

## 4. Failure Artifacts & Tracing

* **Screenshots**: Automatically captured on assertion failures (`screenshot: 'only-on-failure'`).
* **Video Recordings**: Retained for failed test runs (`video: 'retain-on-failure'`).
* **Trace Viewer**: Interactive DOM timeline & network request inspect trace collected on first retry (`trace: 'on-first-retry'`).
