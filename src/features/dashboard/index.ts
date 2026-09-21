import { lazy } from 'react'
import type { YntraPlugin } from '@/lib/plugins/types'

const DashboardPage = lazy(() =>
  import('./components/DashboardPage').then((module) => ({
    default: module.DashboardPage,
  }))
)

export const dashboardPlugin: YntraPlugin = {
  id: 'dashboard',
  name: 'Dashboard',
  description: 'Central overview and command center',
  icon: 'LayoutGrid',
  routes: [
    {
      path: 'dashboard',
      component: DashboardPage,
      allowedRoles: ['platform_admin', 'admin', 'user', 'assistant', 'client'],
      breadcrumbKey: 'sidebar.sections.dashboard'
    }
  ],
  navigation: [
    {
      id: 'dashboard',
      labelKey: 'sidebar.sections.dashboard',
      path: '/dashboard',
      icon: 'LayoutGrid',
      section: 'main'
    }
  ]
}
