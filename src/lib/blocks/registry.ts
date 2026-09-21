import type { UserRole } from '@/types'
import { type IconName } from './icons'
import { messagingPlugin } from '@/features/messages'
import { schedulingPlugin } from '@/features/scheduler'
import { notesPlugin } from '@/features/notes'
import { directoryPlugin } from '@/features/directory'
import { assistancePlugin } from '@/features/assistance'
import { timePlugin } from '@/features/time'
import { reportingPlugin } from '@/features/reporting'
import { dashboardPlugin } from '@/features/dashboard'

export type { UserRole, IconName }

export interface BlockRoute {
  path: string
  component: any
  allowedRoles: UserRole[]
  breadcrumbKey: string
  isIndex?: boolean
}

export interface BlockNavItem {
  id: string
  labelKey: string
  path: string
  icon: IconName
  allowedRoles?: UserRole[]
  section: 'main' | 'tools' | 'admin'
  children?: (Omit<BlockNavItem, 'section'> & { requiredBlockId?: string })[]
  badgeKey?: 'unread_messages' | 'unread_notes'
}

export interface BlockDefinition {
  id: string
  dependencies?: string[]
  routes: BlockRoute[]
  navigation: BlockNavItem[]
}

export const BLOCK_REGISTRY: Record<string, BlockDefinition> = {
  dashboard: dashboardPlugin,
  messaging: messagingPlugin,
  scheduling: schedulingPlugin,
  notes: notesPlugin,
  directory: directoryPlugin,
  assistance: assistancePlugin,
  time: timePlugin,
  reporting: reportingPlugin
}
