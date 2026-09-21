import { useState, useEffect, useCallback } from 'react'
import { platform } from './index'
import type { NetworkStatus, NotificationOptions } from './types'

export function usePlatform() {
  const [networkStatus, setNetworkStatus] = useState<NetworkStatus>(() => platform.network.getStatus())

  useEffect(() => {
    const unsubscribe = platform.network.subscribe((status) => {
      setNetworkStatus(status)
    })
    return unsubscribe
  }, [])

  const notify = useCallback(async (options: NotificationOptions) => {
    await platform.notifications.showNotification(options)
  }, [])

  const impact = useCallback(async (style?: 'light' | 'medium' | 'heavy') => {
    await platform.haptics.impact(style)
  }, [])

  const hapticNotification = useCallback(async (type?: 'success' | 'warning' | 'error') => {
    await platform.haptics.notification(type)
  }, [])

  return {
    platform: platform.type,
    isWeb: platform.isWeb,
    isDesktop: platform.isDesktop,
    isMobile: platform.isMobile,
    isPWA: platform.isPWA,
    capabilities: platform.capabilities,
    isOnline: networkStatus.isOnline,
    networkStatus,
    storage: platform.storage,
    notify,
    impact,
    hapticNotification,
    openExternalUrl: platform.openExternalUrl.bind(platform),
    getAppInfo: platform.getAppInfo.bind(platform),
  }
}
