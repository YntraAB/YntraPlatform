import { lazy } from 'react'
import type { YntraPlugin } from '@/lib/plugins/types'

const ReportingPage = lazy(() =>
  import('./components/ReportingPage').then((module) => ({
    default: module.ReportingPage,
  }))
)

export const reportingPlugin: YntraPlugin = {
  id: 'reporting',
  name: 'Reporting',
  description: 'Incident and deviation reporting',
  icon: 'AlertTriangle',
  routes: [
    {
      path: 'reporting',
      component: ReportingPage,
      allowedRoles: ['platform_admin', 'admin', 'user', 'assistant'],
      breadcrumbKey: 'sidebar.sections.reporting'
    }
  ],
  navigation: [
    {
      id: 'reporting',
      labelKey: 'sidebar.sections.reporting',
      path: '/reporting',
      icon: 'AlertTriangle',
      section: 'main'
    }
  ]
}
