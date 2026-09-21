/**
 * Yntra Platform - Platform Abstraction Layer (PAL)
 * Standardized interfaces for cross-platform capabilities across Web, Desktop, and Mobile.
 */

export type PlatformType = 'web' | 'desktop' | 'mobile'

export interface StorageAdapter {
  getItem(key: string): Promise<string | null>
  setItem(key: string, value: string): Promise<void>
  removeItem(key: string): Promise<void>
  clear(): Promise<void>
}

export interface NotificationOptions {
  title: string
  body: string
  icon?: string
  badge?: string
  data?: unknown
  tag?: string
}

export interface NotificationAdapter {
  isSupported(): boolean
  requestPermission(): Promise<NotificationPermission | 'granted' | 'denied' | 'default'>
  showNotification(options: NotificationOptions): Promise<void>
}

export interface HapticsAdapter {
  isSupported(): boolean
  impact(style?: 'light' | 'medium' | 'heavy'): Promise<void>
  notification(type?: 'success' | 'warning' | 'error'): Promise<void>
  selection(): Promise<void>
}

export interface NetworkStatus {
  isOnline: boolean
  effectiveType?: string
  downlink?: number
  rtt?: number
}

export interface NetworkAdapter {
  getStatus(): NetworkStatus
  subscribe(callback: (status: NetworkStatus) => void): () => void
}

export interface AppInfo {
  name: string
  version: string
  build: string
  platform: PlatformType
  isNative: boolean
  isPWA: boolean
}

export interface PlatformCapabilities {
  hasBiometrics: boolean
  hasNativeFileSystem: boolean
  hasPushNotifications: boolean
  hasHaptics: boolean
  hasBackgroundSync: boolean
  hasSystemTray: boolean
}

export interface PlatformAdapter {
  readonly type: PlatformType
  readonly isWeb: boolean
  readonly isDesktop: boolean
  readonly isMobile: boolean
  readonly isPWA: boolean
  readonly capabilities: PlatformCapabilities
  readonly storage: StorageAdapter
  readonly notifications: NotificationAdapter
  readonly haptics: HapticsAdapter
  readonly network: NetworkAdapter
  getAppInfo(): Promise<AppInfo>
  openExternalUrl(url: string): Promise<void>
}
