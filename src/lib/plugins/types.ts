import { type LazyExoticComponent } from 'react'
import type { UserRole } from '@/types'
import type { IconName } from '@/lib/blocks/icons'

export type { IconName }

export interface PluginRoute {
  path: string
  component: LazyExoticComponent<any>
  allowedRoles: UserRole[]
  breadcrumbKey: string
  isIndex?: boolean
}

export interface PluginNavItem {
  id: string
  labelKey: string
  path: string
  icon: IconName
  allowedRoles?: UserRole[]
  section: 'main' | 'tools' | 'admin'
  children?: (Omit<PluginNavItem, 'section'> & { requiredBlockId?: string })[]
  badgeKey?: 'unread_messages' | 'unread_notes'
}

export interface YntraPlugin {
  id: string
  name: string
  description?: string
  icon: IconName
  dependencies?: string[]
  routes: PluginRoute[]
  navigation: PluginNavItem[]
  enabledByDefault?: boolean
}
