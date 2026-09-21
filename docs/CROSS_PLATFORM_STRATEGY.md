# Yntra Platform: Cross-Platform Strategy & Roadmap

This document outlines how Yntra Platform expands from its primary **Web (PWA)** foundation into **Desktop (Tauri/Electron)** and **Mobile (Capacitor/Native WebView)** without code duplication.

---

## 1. Multi-Target Vision

Unlike legacy approaches that build three separate codebases (e.g. React for web, Swift for iOS, Kotlin for Android), Yntra utilizes a **Unified Core Architecture**:

| Capability | Web (PWA) | Desktop (Tauri) | Mobile (Capacitor) |
| :--- | :--- | :--- | :--- |
| **Runtime** | Modern Browser / Service Worker | Native Rust Shell + Webview | Native iOS/Android Shell |
| **Engine** | React 19 + TypeScript | React 19 + TypeScript | React 19 + TypeScript |
| **Design System** | Tailwind CSS + Radix UI | Tailwind CSS + Radix UI | Tailwind CSS + Radix UI (touch-optimized) |
| **Notifications** | Web Notifications API | System Notification Manager | APNs (iOS) & FCM (Android) |
| **Offline Storage** | IndexedDB via OPFS / idb-keyval | SQLite / Local Filesystem | SQLite / Encrypted Device Storage |
| **Hardware** | Basic Vibration API | System Tray, Global Hotkeys | Haptic Feedback, Camera, Biometrics |

---

## 2. Platform Abstraction Layer (PAL)

All platform differences are encapsulated inside `src/platform/`:
- **`types.ts`**: Defines standard interfaces for storage, notifications, haptics, and network monitoring.
- **`web.ts`**: Implements browser standards (PWA standalone mode, Web Notifications, navigator.vibrate).
- **`desktop.ts`**: Connects to Tauri window APIs, system tray, and native file dialogues.
- **`mobile.ts`**: Connects to Capacitor native plugins for haptic feedback, push notifications, and status bar.

### Usage in Components:
```tsx
import { usePlatform } from '@/platform'

export function ExportReportButton() {
  const { isDesktop, isMobile, capabilities, notify, impact } = usePlatform()

  const handleExport = async () => {
    if (capabilities.hasHaptics) {
      await impact('light')
    }
    // Export logic...
    await notify({ title: 'Export Complete', body: 'Report saved to device' })
  }

  return (
    <button onClick={handleExport} className="btn-primary">
      {isDesktop ? 'Export to File...' : 'Export Report'}
    </button>
  )
}
```

---

## 3. Desktop Deployment Strategy (Tauri 2.0)

Tauri produces lightweight binaries (<15MB) using the host OS webview (WebView2 on Windows, WebKit on macOS):

1. **Prerequisites**: Rust toolchain installed.
2. **Setup**:
   ```bash
   npm install --save-dev @tauri-apps/cli
   npx tauri init
   ```
3. **Configuration (`src-tauri/tauri.conf.json`)**:
   - `build.distDir`: `../dist`
   - `build.devUrl`: `http://localhost:5173`
4. **Build Desktop Installer**:
   ```bash
   npm run build
   npx tauri build
   ```

---

## 4. Mobile Deployment Strategy (Capacitor 6.0)

Capacitor wraps the web application into native Xcode (iOS) and Android Studio projects:

1. **Setup**:
   ```bash
   npm install @capacitor/core @capacitor/cli
   npx cap init "Yntra Platform" "io.yntra.platform" --web-dir dist
   npm install @capacitor/haptics @capacitor/local-notifications @capacitor/status-bar
   ```
2. **Add Native Platforms**:
   ```bash
   npx cap add ios
   npx cap add android
   ```
3. **Sync Web Build**:
   ```bash
   npm run build
   npx cap sync
   ```
4. **Run on Device**:
   ```bash
   npx cap open ios
   npx cap open android
   ```
