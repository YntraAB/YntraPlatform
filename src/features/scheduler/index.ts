import { lazy } from 'react'
import type { YntraPlugin } from '@/lib/plugins/types'

const CalendarPage = lazy(() =>
  import('../scheduler/components/CalendarPage').then((module) => ({
    default: module.CalendarPage,
  }))
)

export const schedulingPlugin: YntraPlugin = {
  id: 'scheduling',
  name: 'Scheduling',
  description: 'Calendar and scheduling management',
  icon: 'Calendar',
  routes: [
    {
      path: 'schedule',
      component: CalendarPage,
      allowedRoles: ['platform_admin', 'admin', 'user', 'assistant'],
      breadcrumbKey: 'sidebar.sections.schedule'
    }
  ],
  navigation: [
    {
      id: 'time_group',
      labelKey: 'sidebar.time_management',
      path: '/schedule',
      icon: 'Clock',
      section: 'main',
      children: [
        {
          id: 'schedule',
          labelKey: 'sidebar.sections.schedule',
          path: '/schedule',
          icon: 'Calendar'
        },
        {
          id: 'timereports',
          labelKey: 'sidebar.sections.timereports',
          path: '/timereports',
          icon: 'FileText',
          requiredBlockId: 'time'
        }
      ]
    }
  ]
}
