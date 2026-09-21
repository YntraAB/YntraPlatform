import { lazy } from 'react'
import type { YntraPlugin } from '@/lib/plugins/types'

const NotesPage = lazy(() =>
  import('./components/NotesPage').then((module) => ({
    default: module.NotesPage,
  }))
)

export const notesPlugin: YntraPlugin = {
  id: 'notes',
  name: 'Notes',
  description: 'Team notes and documentation',
  icon: 'FileText',
  routes: [
    {
      path: 'notes',
      component: NotesPage,
      allowedRoles: ['platform_admin', 'admin', 'user', 'assistant'],
      breadcrumbKey: 'sidebar.sections.notes'
    }
  ],
  navigation: [
    {
      id: 'notes',
      labelKey: 'sidebar.sections.notes',
      path: '/notes',
      icon: 'FileText',
      section: 'main',
      badgeKey: 'unread_notes'
    }
  ]
}
