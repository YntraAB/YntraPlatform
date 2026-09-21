import { lazy } from 'react'
import type { YntraPlugin } from '@/lib/plugins/types'

const DirectoryPage = lazy(() =>
  import('./components/DirectoryPage').then((module) => ({
    default: module.DirectoryPage,
  }))
)

export const directoryPlugin: YntraPlugin = {
  id: 'directory',
  name: 'Directory',
  description: 'Team and user directory',
  icon: 'Users',
  routes: [
    {
      path: 'directory',
      component: DirectoryPage,
      allowedRoles: ['platform_admin', 'admin', 'user', 'assistant', 'client'],
      breadcrumbKey: 'sidebar.sections.directory'
    }
  ],
  navigation: [
    {
      id: 'directory',
      labelKey: 'sidebar.sections.directory',
      path: '/directory',
      icon: 'Users',
      section: 'main'
    }
  ]
}
