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

export function isMobileEnvironment(): boolean {
  if (typeof window === 'undefined') return false
  const w = window as unknown as Record<string, unknown>
  const capacitorObj = w.Capacitor as { isNativePlatform?: () => boolean } | undefined
  const isCapacitor = Boolean(
    capacitorObj &&
      typeof capacitorObj.isNativePlatform === 'function' &&
      capacitorObj.isNativePlatform(),
  )
  const isUserAgentMobile = /Android|webOS|iPhone|iPad|iPod|BlackBerry|IEMobile|Opera Mini/i.test(
    navigator.userAgent,
  )
  return isCapacitor || isUserAgentMobile
}

export class MobilePlatformAdapter implements PlatformAdapter {
  readonly type = 'mobile' as const
  readonly isWeb = false
  readonly isDesktop = false
  readonly isMobile = true

  private fallback = new WebPlatformAdapter()

  get isPWA(): boolean {
    return this.fallback.isPWA
  }

  readonly capabilities: PlatformCapabilities = {
    hasBiometrics: true,
    hasNativeFileSystem: false,
    hasPushNotifications: true,
    hasHaptics: true,
    hasBackgroundSync: true,
    hasSystemTray: false,
  }

  readonly storage: StorageAdapter = this.fallback.storage

  readonly notifications: NotificationAdapter = {
    isSupported: () => true,
    requestPermission: () => this.fallback.notifications.requestPermission(),
    showNotification: async (opts: NotificationOptions) => {
      const w = window as unknown as Record<string, unknown>
      const cap = w.Capacitor as Record<string, unknown> | undefined
      if (cap?.Plugins && (cap.Plugins as Record<string, unknown>).LocalNotifications) {
        try {
          const localNotif = (cap.Plugins as Record<string, unknown>).LocalNotifications as {
            schedule: (arg: unknown) => Promise<void>
          }
          await localNotif.schedule({
            notifications: [
              {
                id: Math.floor(Math.random() * 100000),
                title: opts.title,
                body: opts.body,
              },
            ],
          })
          return
        } catch {
          // Fallback
        }
      }
      return this.fallback.notifications.showNotification(opts)
    },
  }

  readonly haptics: HapticsAdapter = {
    isSupported: () => true,
    impact: async (style = 'medium') => {
      const w = window as unknown as Record<string, unknown>
      const cap = w.Capacitor as Record<string, unknown> | undefined
      if (cap?.Plugins && (cap.Plugins as Record<string, unknown>).Haptics) {
        try {
          const hapticsPlugin = (cap.Plugins as Record<string, unknown>).Haptics as {
            impact: (opt: { style: string }) => Promise<void>
          }
          await hapticsPlugin.impact({ style: style.toUpperCase() })
          return
        } catch {
          // Fallback
        }
      }
      return this.fallback.haptics.impact(style)
    },
    notification: async (type = 'success') => {
      const w = window as unknown as Record<string, unknown>
      const cap = w.Capacitor as Record<string, unknown> | undefined
      if (cap?.Plugins && (cap.Plugins as Record<string, unknown>).Haptics) {
        try {
          const hapticsPlugin = (cap.Plugins as Record<string, unknown>).Haptics as {
            notification: (opt: { type: string }) => Promise<void>
          }
          await hapticsPlugin.notification({ type: type.toUpperCase() })
          return
        } catch {
          // Fallback
        }
      }
      return this.fallback.haptics.notification(type)
    },
    selection: async () => {
      const w = window as unknown as Record<string, unknown>
      const cap = w.Capacitor as Record<string, unknown> | undefined
      if (cap?.Plugins && (cap.Plugins as Record<string, unknown>).Haptics) {
        try {
          const hapticsPlugin = (cap.Plugins as Record<string, unknown>).Haptics as {
            selectionChanged: () => Promise<void>
          }
          await hapticsPlugin.selectionChanged()
          return
        } catch {
          // Fallback
        }
      }
      return this.fallback.haptics.selection()
    },
  }

  readonly network: NetworkAdapter = this.fallback.network

  async getAppInfo(): Promise<AppInfo> {
    return {
      name: 'Yntra Mobile',
      version: '1.0.0',
      build: 'mobile-release',
      platform: 'mobile',
      isNative: !this.fallback.isWeb,
      isPWA: this.fallback.isPWA,
    }
  }

  async openExternalUrl(url: string): Promise<void> {
    return this.fallback.openExternalUrl(url)
  }
}
