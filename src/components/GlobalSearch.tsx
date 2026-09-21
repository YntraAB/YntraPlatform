import React from 'react'
import { useNavigate } from 'react-router-dom'
import { useTranslation } from 'react-i18next'
import { useWorkspace } from '@/contexts/WorkspaceContext'
import {
  useWorkspaceUsers,
  useWorkspaceTeams,
  useWorkspaceNotes,
} from '@/hooks/queries/useWorkspaceData'
import { useMessages } from '@/hooks/queries/useMessages'
import {
  CommandDialog,
  CommandInput,
  CommandList,
  CommandEmpty,
  CommandGroup,
  CommandItem,
  CommandSeparator,
} from '@/components/ui/command'
import {
  Users,
  User,
  Calendar,
  Mail,
  FileText,
  Settings,
  Clock,
  LayoutGrid,
  Globe,
  Palette,
  BellRing,
  AlertTriangle,
  AlertCircle,
  FileWarning,
  UserX,
  Stethoscope,
  CalendarClock,
} from 'lucide-react'

import { useAuth } from '@/hooks/useAuth'
import { useTheme } from '@/components/theme-provider'
import { Sun, Moon, Laptop } from 'lucide-react'

export const openGlobalSearch = () => {
  document.dispatchEvent(new CustomEvent('open-global-search'))
}

export const GlobalSearch: React.FC = () => {
  const [open, setOpen] = React.useState(false)
  const navigate = useNavigate()
  const { t } = useTranslation()
  const { user } = useAuth()
  const { workspaceId } = useWorkspace()
  const { setTheme } = useTheme()

  const { data: users = [] } = useWorkspaceUsers(workspaceId)
  const { data: teams = [] } = useWorkspaceTeams(workspaceId)
  const { data: notes = [] } = useWorkspaceNotes(workspaceId)
  const { data: rawMessages = [] } = useMessages(workspaceId || undefined)

  React.useEffect(() => {
    const down = (e: KeyboardEvent) => {
      if (e.key === 'k' && (e.metaKey || e.ctrlKey)) {
        e.preventDefault()
        setOpen((open) => !open)
      }
      if (e.key === '/') {
        const target = e.target as HTMLElement
        const isInput =
          target.tagName === 'INPUT' ||
          target.tagName === 'TEXTAREA' ||
          target.isContentEditable

        if (!isInput) {
          e.preventDefault()
          setOpen((open) => !open)
        }
      }
    }

    const openListener = () => setOpen(true)

    document.addEventListener('keydown', down)
    document.addEventListener('open-global-search', openListener)
    
    return () => {
      document.removeEventListener('keydown', down)
      document.removeEventListener('open-global-search', openListener)
    }
  }, [])

  const messages = React.useMemo(() => {
    return rawMessages.map((m) => {
      const sndrData = (Array.isArray(m.sender) ? m.sender[0] : m.sender) as {
        full_name?: string
        email?: string
      } | null
      const senderName = sndrData
        ? sndrData.full_name || sndrData.email || t('messages.system')
        : t('messages.system')
      const isSent = m.sender_id === user?.id

      return {
        id: m.id,
        subject: m.subject || t('messages.no_header'),
        senderName,
        isSent,
        date: new Date(m.created_at).toLocaleDateString(),
      }
    })
  }, [rawMessages, user?.id, t])

  const navigationItems = [
    { id: 'dashboard', label: t('sidebar.sections.dashboard', 'Overview'), path: '/dashboard', icon: LayoutGrid },
    { id: 'schedule', label: t('sidebar.schedule'), path: '/schedule', icon: Calendar },
    { id: 'inbox', label: t('sidebar.inbox'), path: '/inbox', icon: Mail },
    { id: 'notes', label: t('sidebar.notes'), path: '/notes', icon: FileText },
    { id: 'directory', label: t('sidebar.teams'), path: '/directory', icon: Users },
    { id: 'time', label: t('sidebar.time_management'), path: '/time', icon: Clock },
    { id: 'settings', label: t('common.settings'), path: '/settings', icon: Settings },
  ]

  const settingsItems = [
    {
      id: 'set-gen',
      label: t('settings.localization'),
      path: '/settings?tab=general',
      icon: Globe,
    },
    {
      id: 'set-app',
      label: t('settings.theme'),
      path: '/settings?tab=general',
      icon: Palette,
    },
    {
      id: 'set-not',
      label: t('settings.notifications.title'),
      path: '/settings?tab=notifications',
      icon: BellRing,
    },
    {
      id: 'set-acc',
      label: t('settings.tabs.account'),
      path: '/settings?tab=account',
      icon: User,
    },
    ...(user?.role === 'admin' || user?.role === 'platform_admin' ? [
      {
        id: 'set-sch',
        label: t('settings.tabs.scheduler'),
        path: '/settings?tab=scheduler',
        icon: Calendar,
      },
      {
        id: 'set-blk',
        label: t('settings.tabs.blocks') || 'Feature Blocks',
        path: '/settings?tab=blocks',
        icon: LayoutGrid,
      },
    ] : []),
  ]

  const actionItems = [
    {
      id: 'action-leave',
      label: t('reporting.types.leave_request'),
      path: '/schedule?action=leave_request',
      icon: CalendarClock,
    },
    {
      id: 'report-complaint',
      label: `${t('reporting.tabs.send')}: ${t('reporting.types.complaint')}`,
      path: '/reporting?tab=send&type=complaint',
      icon: AlertCircle,
    },
    {
      id: 'report-work-injury',
      label: `${t('reporting.tabs.send')}: ${t('reporting.types.work_injury')}`,
      path: '/reporting?tab=send&type=work_injury',
      icon: Stethoscope,
    },
    {
      id: 'report-incident',
      label: `${t('reporting.tabs.send')}: ${t('reporting.types.incident')}`,
      path: '/reporting?tab=send&type=incident',
      icon: FileWarning,
    },
    {
      id: 'report-deviation',
      label: `${t('reporting.tabs.send')}: ${t('reporting.types.deviation')}`,
      path: '/reporting?tab=send&type=deviation',
      icon: AlertTriangle,
    },
    {
      id: 'report-whistleblower',
      label: `${t('reporting.tabs.send')}: ${t('reporting.types.whistleblower')}`,
      path: '/reporting?tab=send&type=whistleblower',
      icon: UserX,
    },
  ]



  const onSelect = (path: string) => {
    navigate(path)
    setOpen(false)
  }

  return (
    <CommandDialog open={open} onOpenChange={setOpen}>
      <CommandInput placeholder={t('common.search') + '...'} />
      <CommandList className="scrollbar-dark">
        <CommandEmpty>{t('messages.empty_state')}</CommandEmpty>

        <CommandGroup heading={t('sidebar.show')}>
          {navigationItems.map((item) => (
            <CommandItem key={item.id} onSelect={() => onSelect(item.path)}>
              <item.icon className="mr-2 h-4 w-4" />
              <span>{item.label}</span>
            </CommandItem>
          ))}
        </CommandGroup>

        <CommandSeparator />

        <CommandGroup heading={t('common.actions')}>
          {actionItems.map((item) => (
            <CommandItem key={item.id} onSelect={() => onSelect(item.path)}>
              <item.icon className="mr-2 h-4 w-4 text-primary" />
              <span>{item.label}</span>
            </CommandItem>
          ))}
        </CommandGroup>

        {messages.length > 0 && (
          <>
            <CommandSeparator />
            <CommandGroup heading={t('sidebar.inbox')}>
              {messages.map(
                (msg: {
                  id: string
                  subject: string
                  isSent: boolean
                  senderName: string
                  date: string
                }) => (
                  <CommandItem key={msg.id} onSelect={() => onSelect(`/inbox?messageId=${msg.id}`)}>
                    <Mail className="mr-2 h-4 w-4" />
                    <div className="flex flex-col">
                      <span className="font-medium">{msg.subject}</span>
                      <span className="max-w-[400px] truncate text-[10px] text-muted-foreground">
                        {msg.isSent ? t('messages.sent') : msg.senderName} • {msg.date}
                      </span>
                    </div>
                  </CommandItem>
                ),
              )}
            </CommandGroup>
          </>
        )}

        {notes.length > 0 && (
          <>
            <CommandSeparator />
            <CommandGroup heading={t('sidebar.notes')}>
              {notes.map((note) => (
                <CommandItem
                  key={note.id}
                  onSelect={() => onSelect(`/notes?team=${note.team_id}&note=${note.id}`)}
                >
                  <FileText className="mr-2 h-4 w-4" />
                  <div className="flex flex-col">
                    <span className="font-medium">{note.subject}</span>
                    <span className="max-w-[400px] truncate text-[10px] text-muted-foreground">
                      {note.team?.name} • {note.author?.full_name || note.author?.email}
                    </span>
                  </div>
                </CommandItem>
              ))}
            </CommandGroup>
          </>
        )}

        <CommandSeparator />

        <CommandGroup heading={t('common.settings')}>
          {settingsItems.map((item) => (
            <CommandItem key={item.id} onSelect={() => onSelect(item.path)}>
              <item.icon className="mr-2 h-4 w-4" />
              <span>{item.label}</span>
            </CommandItem>
          ))}
        </CommandGroup>

        <CommandSeparator />

        <CommandGroup heading={t('command_menu.groups.theme', 'Theme')}>
          <CommandItem onSelect={() => { setTheme('light'); setOpen(false) }}>
            <Sun className="mr-2 h-4 w-4" />
            <span>{t('command_menu.actions.light_theme', 'Light mode')}</span>
          </CommandItem>
          <CommandItem onSelect={() => { setTheme('dark'); setOpen(false) }}>
            <Moon className="mr-2 h-4 w-4" />
            <span>{t('command_menu.actions.dark_theme', 'Dark mode')}</span>
          </CommandItem>
          <CommandItem onSelect={() => { setTheme('system'); setOpen(false) }}>
            <Laptop className="mr-2 h-4 w-4" />
            <span>{t('command_menu.actions.system_theme', 'System theme')}</span>
          </CommandItem>
        </CommandGroup>

        {teams.length > 0 && (
          <>
            <CommandSeparator />
            <CommandGroup heading={t('directory.levels.teams')}>
              {teams.map((team) => (
                <CommandItem key={team.id} onSelect={() => onSelect(`/directory?team=${team.id}`)}>
                  <LayoutGrid className="mr-2 h-4 w-4" />
                  <span>{team.name}</span>
                </CommandItem>
              ))}
            </CommandGroup>
          </>
        )}

        {users.length > 0 && (
          <>
            <CommandSeparator />
            <CommandGroup heading={t('directory.roles.assistant')}>
              {users.map((user) => (
                <CommandItem key={user.id} onSelect={() => onSelect(`/directory?user=${user.id}`)}>
                  <User className="mr-2 h-4 w-4" />
                  <div className="flex flex-col">
                    <span>{user.name}</span>
                    <span className="text-[10px] text-muted-foreground">{user.email}</span>
                  </div>
                </CommandItem>
              ))}
            </CommandGroup>
          </>
        )}
      </CommandList>
    </CommandDialog>
  )
}
