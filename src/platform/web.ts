import type {
  PlatformAdapter,
  PlatformCapabilities,
  StorageAdapter,
  NotificationAdapter,
  HapticsAdapter,
  NetworkAdapter,
  NetworkStatus,
  AppInfo,
  NotificationOptions,
} from './types'

const webStorage: StorageAdapter = {
  async getItem(key: string): Promise<string | null> {
    try {
      return localStorage.getItem(key)
    } catch {
      return null
    }
  },
  async setItem(key: string, value: string): Promise<void> {
    try {
      localStorage.setItem(key, value)
    } catch {
      // Storage quota or disabled
    }
  },
  async removeItem(key: string): Promise<void> {
    try {
      localStorage.removeItem(key)
    } catch {
      // Ignore
    }
  },
  async clear(): Promise<void> {
    try {
      localStorage.clear()
    } catch {
      // Ignore
    }
  },
}

const webNotifications: NotificationAdapter = {
  isSupported(): boolean {
    return typeof window !== 'undefined' && 'Notification' in window
  },
  async requestPermission(): Promise<NotificationPermission> {
    if (!this.isSupported()) return 'denied'
    try {
      return await Notification.requestPermission()
    } catch {
      return 'denied'
    }
  },
  async showNotification(options: NotificationOptions): Promise<void> {
    if (!this.isSupported() || Notification.permission !== 'granted') return
    try {
      if ('serviceWorker' in navigator) {
        const registration = await navigator.serviceWorker.ready
        if (registration && registration.showNotification) {
          await registration.showNotification(options.title, {
            body: options.body,
            icon: options.icon || '/logo.png',
            badge: options.badge,
            tag: options.tag,
            data: options.data,
          })
          return
        }
      }
      new Notification(options.title, {
        body: options.body,
        icon: options.icon || '/logo.png',
        tag: options.tag,
      })
    } catch {
      // Silent catch
    }
  },
}

const webHaptics: HapticsAdapter = {
  isSupported(): boolean {
    return typeof navigator !== 'undefined' && 'vibrate' in navigator
  },
  async impact(style: 'light' | 'medium' | 'heavy' = 'medium'): Promise<void> {
    if (!this.isSupported()) return
    const durations = { light: 15, medium: 35, heavy: 70 }
    try {
      navigator.vibrate(durations[style] || 35)
    } catch {
      // Silent catch
    }
  },
  async notification(type: 'success' | 'warning' | 'error' = 'success'): Promise<void> {
    if (!this.isSupported()) return
    const patterns = {
      success: [20, 50, 20],
      warning: [30, 40, 30],
      error: [50, 50, 50],
    }
    try {
      navigator.vibrate(patterns[type] || [20, 50, 20])
    } catch {
      // Silent catch
    }
  },
  async selection(): Promise<void> {
    if (!this.isSupported()) return
    try {
      navigator.vibrate(10)
    } catch {
      // Silent catch
    }
  },
}

const webNetwork: NetworkAdapter = {
  getStatus(): NetworkStatus {
    const isOnline = typeof navigator !== 'undefined' ? navigator.onLine : true
    const nav = typeof navigator !== 'undefined' ? (navigator as unknown as Record<string, unknown>) : null
    const connection = nav && typeof nav.connection === 'object' ? (nav.connection as Record<string, unknown>) : null
    return {
      isOnline,
      effectiveType: connection ? String(connection.effectiveType) : undefined,
      downlink: connection ? Number(connection.downlink) : undefined,
      rtt: connection ? Number(connection.rtt) : undefined,
    }
  },
  subscribe(callback: (status: NetworkStatus) => void): () => void {
    if (typeof window === 'undefined') return () => {}
    const handleOnline = () => callback(this.getStatus())
    const handleOffline = () => callback(this.getStatus())

    window.addEventListener('online', handleOnline)
    window.addEventListener('offline', handleOffline)

    return () => {
      window.removeEventListener('online', handleOnline)
      window.removeEventListener('offline', handleOffline)
    }
  },
}

export class WebPlatformAdapter implements PlatformAdapter {
  readonly type = 'web' as const
  readonly isWeb = true
  readonly isDesktop = false
  readonly isMobile = false

  get isPWA(): boolean {
    if (typeof window === 'undefined') return false
    return (
      window.matchMedia('(display-mode: standalone)').matches ||
      (window.navigator as unknown as { standalone?: boolean }).standalone === true
    )
  }

  readonly capabilities: PlatformCapabilities = {
    hasBiometrics: typeof window !== 'undefined' && 'PublicKeyCredential' in window,
    hasNativeFileSystem: typeof window !== 'undefined' && 'showOpenFilePicker' in window,
    hasPushNotifications: typeof window !== 'undefined' && 'Notification' in window,
    hasHaptics: typeof navigator !== 'undefined' && 'vibrate' in navigator,
    hasBackgroundSync: typeof window !== 'undefined' && 'SyncManager' in window,
    hasSystemTray: false,
  }

  readonly storage = webStorage
  readonly notifications = webNotifications
  readonly haptics = webHaptics
  readonly network = webNetwork

  async getAppInfo(): Promise<AppInfo> {
    return {
      name: 'Yntra Platform',
      version: '1.0.0',
      build: 'web-production',
      platform: 'web',
      isNative: false,
      isPWA: this.isPWA,
    }
  }

  async openExternalUrl(url: string): Promise<void> {
    if (typeof window !== 'undefined') {
      window.open(url, '_blank', 'noopener,noreferrer')
    }
  }
}
