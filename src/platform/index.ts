import type { PlatformAdapter } from './types'
import { WebPlatformAdapter } from './web'
import { DesktopPlatformAdapter, isDesktopEnvironment } from './desktop'
import { MobilePlatformAdapter, isMobileEnvironment } from './mobile'

export * from './types'
export { usePlatform } from './usePlatform'

function resolvePlatformAdapter(): PlatformAdapter {
  if (typeof window === 'undefined') {
    return new WebPlatformAdapter()
  }

  if (isDesktopEnvironment()) {
    return new DesktopPlatformAdapter()
  }

  if (isMobileEnvironment()) {
    return new MobilePlatformAdapter()
  }

  return new WebPlatformAdapter()
}

export const platform: PlatformAdapter = resolvePlatformAdapter()
