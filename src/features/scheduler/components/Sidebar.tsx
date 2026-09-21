/**
 * Main navigation sidebar for Yntra Platform.
 * Follows the clean, minimalist Linear/Apple aesthetic with relaxed sizing.
 */

import React, { useState, useEffect } from 'react'
import { ChevronRight, Search } from 'lucide-react'
import { cn } from '@/lib/utils'
import { useWorkspace } from '@/contexts/WorkspaceContext'
import { useAuth } from '@/hooks/useAuth'
import { supabase } from '@/lib/supabase'
import { BLOCK_REGISTRY } from '@/lib/blocks/registry'
import { ICON_MAP } from '@/lib/blocks/icons'
import type { UserRole } from '@/types'
import { useUnreadNotes } from '@/hooks/useUnreadNotes'
import { useTranslation } from 'react-i18next'
import { TeamSwitcher } from './TeamSwitcher'
import { openGlobalSearch } from '@/components/GlobalSearch'

interface NavItem {
  id: string
  label: string
  icon: React.ElementType
  badge?: number
  children?: NavItem[]
}

interface SidebarProps {
  activeSection: string
  onSectionChange: (sectionId: string) => void
  className?: string
  onNavigate?: () => void
}

interface SidebarNavItemProps {
  item: NavItem
  activeSection: string
  isExpanded: boolean
  onToggle: () => void
  onSelect: (id: string) => void
  depth?: number
}

const SidebarNavItem: React.FC<SidebarNavItemProps> = ({
  item,
  activeSection,
  isExpanded,
  onSelect,
  onToggle,
  depth = 0,
}) => {
  const hasChildren = item.children && item.children.length > 0
  const isDirectActive = activeSection === item.id
  const isChildActive = item.children?.some((c) => activeSection === c.id)
  const isActive = isDirectActive || isChildActive
  const Icon = item.icon

  return (
    <div>
      {/* Main item button */}
      <div
        onClick={hasChildren ? onToggle : () => onSelect(item.id)}
        className={cn(
          'group flex h-[38px] cursor-pointer items-center gap-2.5 rounded-lg px-3 text-[13.5px] transition-colors duration-150',
          isActive
            ? 'bg-secondary/80 font-medium text-foreground'
            : 'text-muted-foreground hover:bg-secondary/40 hover:text-foreground',
          depth > 0 && 'ml-3',
        )}
      >
        {/* Icon — 15px */}
        <Icon
          className={cn(
            'h-[15px] w-[15px] shrink-0 transition-colors',
            isActive ? 'text-foreground' : 'text-muted-foreground/80 group-hover:text-foreground',
          )}
        />

        {/* Label — 13.5px */}
        <span className="flex-1 truncate tracking-tight text-[13.5px] font-normal" data-testid={`sidebar-item-${item.id}`}>
          {item.label}
        </span>

        {/* Badge for unread counts */}
        {item.badge !== undefined && item.badge > 0 && (
          <span className="flex h-4.5 min-w-[18px] items-center justify-center rounded-full bg-primary/15 px-1 text-[10px] font-medium tabular-nums text-primary">
            {item.badge}
          </span>
        )}

        {/* Expand/collapse chevron */}
        {hasChildren && (
          <ChevronRight
            className={cn(
              'h-3 w-3 shrink-0 text-muted-foreground/45 transition-transform duration-200',
              isExpanded && 'rotate-90 text-foreground',
            )}
          />
        )}
      </div>

      {/* Child items — 12px clean text, comfortable h-[34px] buttons */}
      {hasChildren && isExpanded && (
        <div className="mt-1 space-y-1 border-l border-border/40 pl-3 ml-4.5">
          {item.children!.map((child) => {
            const isSubActive = activeSection === child.id
            return (
              <div
                key={child.id}
                onClick={(e) => {
                  e.stopPropagation()
                  onSelect(child.id)
                }}
                className={cn(
                  'group flex h-[34px] cursor-pointer items-center rounded-md px-2.5 text-xs transition-colors duration-150',
                  isSubActive
                    ? 'bg-secondary/70 font-medium text-foreground'
                    : 'text-muted-foreground hover:bg-secondary/40 hover:text-foreground',
                )}
              >
                <span className="truncate">{child.label}</span>
              </div>
            )
          })}
        </div>
      )}
    </div>
  )
}

export const Sidebar: React.FC<SidebarProps> = ({
  activeSection,
  onSectionChange,
  className,
  onNavigate,
}) => {
  const { modules, workspaceId, workspaceName, workspaceLogo, setAdminWorkspace } = useWorkspace()
  const { t } = useTranslation()
  const { user } = useAuth()
  const unreadNotes = useUnreadNotes()

  const handleSelect = (id: string) => {
    onSectionChange(id)
    onNavigate?.()
  }

  const isAdmin = user?.role === 'admin' || user?.role === 'platform_admin'

  const [adminWorkspaces, setAdminWorkspaces] = useState<{ id: string; name: string }[]>([])
  useEffect(() => {
    if (user?.role === 'platform_admin' && adminWorkspaces.length === 0) {
      supabase
        .from('workspaces')
        .select('id, name')
        .then(({ data }) => {
          if (data && data.length > 0) setAdminWorkspaces(data)
        })
    }
  }, [user?.role, adminWorkspaces.length])

  const [unreadMessages, setUnreadMessages] = useState<number>(0)

  useEffect(() => {
    async function fetchUnreadCounts() {
      if (!workspaceId || !user) return
      const { count } = await supabase
        .from('messages')
        .select('*', { count: 'exact', head: true })
        .eq('workspace_id', workspaceId)
        .eq('is_read', false)
        .neq('sender_id', user.id)

      if (count !== null) setUnreadMessages(count)
    }
    fetchUnreadCounts()
  }, [workspaceId, user])

  const [expandedSections, setExpandedSections] = useState<Set<string>>(new Set(['time_group']))

  const NAVIGATION_ITEMS: NavItem[] = Object.values(BLOCK_REGISTRY).flatMap((block) => {
    if (block.id !== 'dashboard' && !(modules as any)[block.id]) return []

    return block.navigation
      .filter((item) => {
        if (!item.allowedRoles) return true
        return item.allowedRoles.includes(user?.role as UserRole)
      })
      .map((item) => {
        let badge: number | undefined = undefined
        if (item.badgeKey === 'unread_messages') badge = unreadMessages > 0 ? unreadMessages : undefined
        if (item.badgeKey === 'unread_notes') badge = unreadNotes.total > 0 ? unreadNotes.total : undefined

        return {
          id: item.id,
          label: t(item.labelKey),
          icon: ICON_MAP[item.icon],
          badge,
          children: item.children
            ?.filter((child) => !child.requiredBlockId || (modules as any)[child.requiredBlockId])
            .map((child) => ({
              id: child.id,
              label: t(child.labelKey),
              icon: ICON_MAP[child.icon],
            })),
        }
      })
  })

  const toggleSection = (sectionId: string) => {
    setExpandedSections((prev) => {
      const next = new Set(prev)
      if (next.has(sectionId)) {
        next.delete(sectionId)
      } else {
        next.add(sectionId)
      }
      return next
    })
  }

  return (
    <aside
      className={cn(
        'flex h-full w-64 shrink-0 flex-col border-r border-border/50 bg-sidebar select-none',
        className,
      )}
    >
      {/* Header / Workspace Identity */}
      <div className="flex h-12 items-center border-b border-border/40 px-4">
        <div className="flex min-w-0 flex-1 items-center gap-2.5">
          {workspaceLogo ? (
            <img
              src={workspaceLogo}
              alt={workspaceName}
              className="h-7 w-7 rounded-md border border-border/50 object-contain"
            />
          ) : (
            <div className="flex h-7 w-7 shrink-0 items-center justify-center rounded-md border border-primary/20 bg-primary/10 text-primary">
              <span className="text-xs font-semibold">
                {(workspaceName || 'Yntra').charAt(0).toUpperCase()}
              </span>
            </div>
          )}
          <div className="min-w-0 flex-1">
            <h2 className="truncate text-[13.5px] font-medium tracking-tight text-foreground">
              {workspaceName || 'Yntra Platform'}
            </h2>
            {user?.role === 'platform_admin' ? (
              <select
                className="pointer-events-auto mt-0.5 w-full max-w-[130px] cursor-pointer appearance-none truncate border-none bg-transparent p-0 text-[11px] text-muted-foreground/70 underline decoration-dashed underline-offset-2 outline-none"
                value={workspaceId || ''}
                onChange={(e) => setAdminWorkspace(e.target.value)}
              >
                {adminWorkspaces.map((w) => (
                  <option key={w.id} value={w.id}>
                    {w.name}
                  </option>
                ))}
              </select>
            ) : (
              <p className="truncate text-xs font-normal text-muted-foreground/60 capitalize">
                {user?.role ? t(`auth.role_simulator.${user.role}`, user.role) : 'Workspace'}
              </p>
            )}
          </div>
        </div>
      </div>

      {/* Controls: Search Trigger & Team Switcher */}
      <div className="flex flex-col gap-2.5 px-3 pt-2.5 pb-2">
        <button
          type="button"
          onClick={() => openGlobalSearch()}
          className="flex h-8 w-full cursor-pointer items-center gap-2 rounded-md border border-border/40 bg-secondary/20 px-2.5 text-xs text-muted-foreground/70 transition-colors hover:bg-secondary/40 hover:text-foreground"
        >
          <Search className="h-3.5 w-3.5 shrink-0 text-muted-foreground/60" />
          <span className="flex-1 text-left text-xs font-normal">{t('common.search')}...</span>
          <kbd className="hidden rounded border border-border/40 bg-background/50 px-1 py-0.2 font-mono text-[9px] text-muted-foreground/50 sm:inline-block">
            ⌘K
          </kbd>
        </button>

        {isAdmin && <TeamSwitcher />}
      </div>

      <div className="mx-3 h-px bg-border/40" />

      {/* Navigation items */}
      <nav className="scrollbar-dark flex-1 space-y-1 overflow-y-auto px-3 py-2">
        {NAVIGATION_ITEMS.map((item) => (
          <SidebarNavItem
            key={item.id}
            item={item}
            activeSection={activeSection}
            isExpanded={expandedSections.has(item.id)}
            onToggle={() => toggleSection(item.id)}
            onSelect={handleSelect}
          />
        ))}
      </nav>

      {/* Bottom Status Panel (clean & minimal) */}
      <div className="border-t border-border/40 px-4 py-3">
        <div className="flex items-center justify-between text-xs text-muted-foreground/60">
          <div className="flex items-center gap-2">
            <span className="h-1.5 w-1.5 rounded-full bg-emerald-500/80 shrink-0" />
            <span className="text-[11px] font-normal text-muted-foreground/70">Yntra OS</span>
          </div>
          <span className="font-mono text-[10px] text-muted-foreground/40">v1.2</span>
        </div>
      </div>
    </aside>
  )
}

export default Sidebar
