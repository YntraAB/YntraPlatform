import type {
  PlatformAdapter,
  PlatformCapabilities,
  StorageAdapter,
  NotificationAdapter,
  HapticsAdapter,
  NetworkAdapter,
  AppInfo,
  NotificationOptions,
} from './types'
import { WebPlatformAdapter } from './web'

// Check if running in a Tauri or Electron container
export function isDesktopEnvironment(): boolean {
  if (typeof window === 'undefined') return false
  const w = window as unknown as Record<string, unknown>
  return Boolean(w.__TAURI__ || w.__TAURI_INTERNALS__ || (w.process && (w.process as Record<string, unknown>).type === 'renderer'))
}

export class DesktopPlatformAdapter implements PlatformAdapter {
  readonly type = 'desktop' as const
  readonly isWeb = false
  readonly isDesktop = true
  readonly isMobile = false
  readonly isPWA = false

  private fallback = new WebPlatformAdapter()

  readonly capabilities: PlatformCapabilities = {
    hasBiometrics: false,
    hasNativeFileSystem: true,
    hasPushNotifications: true,
    hasHaptics: false,
    hasBackgroundSync: true,
    hasSystemTray: true,
  }

  readonly storage: StorageAdapter = {
    getItem: (key) => this.fallback.storage.getItem(key),
    setItem: (key, val) => this.fallback.storage.setItem(key, val),
    removeItem: (key) => this.fallback.storage.removeItem(key),
    clear: () => this.fallback.storage.clear(),
  }

  readonly notifications: NotificationAdapter = {
    isSupported: () => true,
    requestPermission: async () => 'granted',
    showNotification: async (opts: NotificationOptions) => {
      // In Tauri: invoke native notification plugin
      const w = window as unknown as Record<string, unknown>
      if (w.__TAURI__) {
        try {
          const tauriNotification = (w.__TAURI__ as Record<string, unknown>).notification as
            | { sendNotification: (arg: unknown) => Promise<void> }
            | undefined
          if (tauriNotification) {
            await tauriNotification.sendNotification({ title: opts.title, body: opts.body })
            return
          }
        } catch {
          // Fallback to web notification
        }
      }
      return this.fallback.notifications.showNotification(opts)
    },
  }

  readonly haptics: HapticsAdapter = {
    isSupported: () => false,
    impact: async () => {},
    notification: async () => {},
    selection: async () => {},
  }

  readonly network: NetworkAdapter = this.fallback.network

  async getAppInfo(): Promise<AppInfo> {
    return {
      name: 'Yntra Desktop',
      version: '1.0.0',
      build: 'desktop-release',
      platform: 'desktop',
      isNative: true,
      isPWA: false,
    }
  }

  async openExternalUrl(url: string): Promise<void> {
    const w = window as unknown as Record<string, unknown>
    if (w.__TAURI__) {
      try {
        const shell = (w.__TAURI__ as Record<string, unknown>).shell as
          | { open: (target: string) => Promise<void> }
          | undefined
        if (shell) {
          await shell.open(url)
          return
        }
      } catch {
        // Fallback
      }
    }
    return this.fallback.openExternalUrl(url)
  }
}
