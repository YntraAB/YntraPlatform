import { lazy } from 'react'
import type { YntraPlugin } from '@/lib/plugins/types'

const MessagesPage = lazy(() =>
  import('./components/MessagesPage').then((module) => ({
    default: module.MessagesPage,
  }))
)

export const messagingPlugin: YntraPlugin = {
  id: 'messaging',
  name: 'Messaging',
  description: 'Internal messaging system',
  icon: 'MessageSquare',
  routes: [
    {
      path: 'inbox',
      component: MessagesPage,
      allowedRoles: ['platform_admin', 'admin', 'user', 'assistant', 'client'],
      breadcrumbKey: 'sidebar.sections.inbox'
    }
  ],
  navigation: [
    {
      id: 'inbox',
      labelKey: 'sidebar.sections.inbox',
      path: '/inbox',
      icon: 'MessageSquare',
      section: 'main',
      badgeKey: 'unread_messages'
    }
  ]
}
