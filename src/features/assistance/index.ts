import { lazy } from 'react'
import type { YntraPlugin } from '@/lib/plugins/types'

const ClientOverviewPage = lazy(() =>
  import('./components/ClientOverviewPage').then((module) => ({
    default: module.ClientOverviewPage,
  }))
)

export const assistancePlugin: YntraPlugin = {
  id: 'assistance',
  name: 'Assistance',
  description: 'Client assistance and overview',
  icon: 'Heart',
  routes: [
    {
      path: 'medication',
      component: ClientOverviewPage,
      allowedRoles: ['platform_admin', 'admin', 'user', 'assistant'],
      breadcrumbKey: 'sidebar.sections.assistance'
    }
  ],
  navigation: [
    {
      id: 'medication',
      labelKey: 'sidebar.sections.assistance',
      path: '/medication',
      icon: 'Heart',
      section: 'main'
    }
  ]
}
