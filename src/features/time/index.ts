import { lazy } from 'react'
import type { YntraPlugin } from '@/lib/plugins/types'

const TimeManagerPage = lazy(() =>
  import('./components/TimeManagerPage').then((module) => ({
    default: module.TimeManagerPage,
  }))
)

export const timePlugin: YntraPlugin = {
  id: 'time',
  name: 'Time',
  description: 'Time management and reporting',
  icon: 'Clock',
  routes: [
    {
      path: 'time',
      component: TimeManagerPage,
      allowedRoles: ['platform_admin', 'admin', 'user', 'assistant'],
      breadcrumbKey: 'sidebar.sections.time'
    },
    {
      path: 'timereports',
      component: TimeManagerPage,
      allowedRoles: ['platform_admin', 'admin', 'user', 'assistant'],
      breadcrumbKey: 'sidebar.sections.timereports'
    }
  ],
  navigation: []
}
