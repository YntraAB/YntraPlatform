import React, { useState } from 'react'
import { Building2, Users, User, Trash2, Plus, Search, Inbox } from 'lucide-react'
import { useTranslation } from 'react-i18next'
import { Button } from '@/components/ui/button'
import { Input } from '@/components/ui/input'
import { supabase } from '@/lib/supabase'
import type { WorkspaceItem } from '../hooks/useDirectoryData'

interface WorkspacesViewProps {
  workspaces: WorkspaceItem[]
  userRole: string
  onSelectWorkspace: (id: string) => void
  onOpenHub: () => void
  setDbWorkspaces: React.Dispatch<React.SetStateAction<WorkspaceItem[]>>
}

export const WorkspacesView: React.FC<WorkspacesViewProps> = ({
  workspaces,
  userRole,
  onSelectWorkspace,
  onOpenHub,
  setDbWorkspaces,
}) => {
  const { t } = useTranslation()
  const [searchQuery, setSearchQuery] = useState('')

  let filteredWorkspaces = workspaces
  if (searchQuery.trim()) {
    const q = searchQuery.toLowerCase()
    filteredWorkspaces = filteredWorkspaces.filter(
      (ws) =>
        ws.name.toLowerCase().includes(q) ||
        (ws.type && ws.type.toLowerCase().includes(q)),
    )
  }

  const isPlatformAdmin = userRole === 'platform_admin'

  return (
    <div className="relative flex h-full flex-1 flex-col bg-background selection:bg-primary/20">
      {/* Top Header Controls - exactly matching Inbox */}
      <div className="sticky top-0 z-20 flex h-12 shrink-0 items-center justify-between border-b border-border/50 bg-background/50 px-6 backdrop-blur-md">
        <div className="flex items-center gap-3">
          <h2 className="flex items-center gap-1.5 text-xs font-medium text-foreground">
            <Building2 className="h-3.5 w-3.5 text-muted-foreground" />
            {t('directory.levels.workspaces')}
          </h2>
          <div className="rounded border border-border/50 bg-muted/60 px-1.5 py-0.5 text-[10px] font-medium text-muted-foreground">
            {filteredWorkspaces.length} {t('directory.levels.workspaces').toLowerCase()}
          </div>
        </div>

        <div className="flex items-center gap-4">
          <div className="group relative w-64">
            <Search className="absolute left-2.5 top-1/2 h-3.5 w-3.5 -translate-y-1/2 text-muted-foreground transition-colors group-focus-within:text-primary" />
            <Input
              placeholder={t('common.search')}
              value={searchQuery}
              onChange={(e) => setSearchQuery(e.target.value)}
              className="h-8 rounded-md border border-border/50 bg-muted/50 pl-8 text-xs text-foreground transition-all focus-visible:bg-muted focus-visible:ring-1 focus-visible:ring-primary"
            />
          </div>

          {isPlatformAdmin && (
            <Button
              size="sm"
              className="h-8 text-xs shadow-sm"
              onClick={onOpenHub}
            >
              <Plus className="mr-1.5 h-3.5 w-3.5" />
              {t('directory.workspaces.create_button')}
            </Button>
          )}
        </div>
      </div>

      {/* Workspaces List - exactly matching Inbox row height & layout */}
      <div className="scrollbar-dark w-full flex-1 overflow-y-auto scroll-smooth">
        {filteredWorkspaces.length === 0 ? (
          <div className="flex h-full flex-col items-center justify-center text-muted-foreground duration-500 animate-in fade-in zoom-in-95">
            <div className="mb-6 flex h-20 w-20 items-center justify-center rounded-full border border-border/50 bg-muted/30">
              <Inbox className="h-10 w-10 opacity-20" />
            </div>
            <p className="text-sm font-medium">{t('notes.list.empty_state')}</p>
          </div>
        ) : (
          <div className="flex w-full flex-col text-sm">
            {filteredWorkspaces.map((ws, idx) => (
              <div
                key={ws.id}
                onClick={() => onSelectWorkspace(ws.id)}
                style={{ animationDelay: `${idx * 30}ms` }}
                className="group flex h-12 cursor-pointer items-center border-b border-border/40 px-6 transition-all duration-300 hover:bg-secondary/40 animate-in fade-in slide-in-from-bottom-2 fill-mode-both"
                tabIndex={0}
                onKeyDown={(e) => {
                  if (e.key === 'Enter') onSelectWorkspace(ws.id)
                }}
              >
                {/* Leading Workspace Icon */}
                <div className="mr-3 flex size-6 shrink-0 items-center justify-center text-muted-foreground/70 transition-colors group-hover:text-primary">
                  <Building2 className="h-4 w-4" />
                </div>

                {/* Name Column - identical to Inbox */}
                <div className="w-36 sm:w-48 md:w-56 shrink-0 truncate pr-3 text-sm font-medium text-foreground transition-colors">
                  {ws.name}
                </div>

                {/* Subtitle / Type (Middle Column) - identical to Inbox */}
                <div className="flex min-w-0 flex-1 items-center gap-x-2 truncate pr-6">
                  <span className="truncate text-sm font-light italic text-muted-foreground/60">
                    {ws.type || '—'}
                  </span>
                </div>

                {/* Hover Action & Tabular Counts with team/user icons - identical to Inbox */}
                <div className="flex w-52 shrink-0 items-center justify-end">
                  <div className="mr-6 flex translate-x-2 items-center gap-3.5 text-muted-foreground/60 opacity-0 transition-all duration-300 group-hover:translate-x-0 group-hover:opacity-100">
                    {isPlatformAdmin && (
                      <button
                        type="button"
                        onClick={(e) => {
                          e.stopPropagation()
                          if (confirm(t('directory.workspaces.delete_confirm'))) {
                            supabase
                              .rpc('delete_workspace', { target_workspace_id: ws.id })
                              .then(() => {
                                setDbWorkspaces((prev) => prev.filter((w) => w.id !== ws.id))
                              })
                          }
                        }}
                        className="p-0.5 text-muted-foreground/60 transition-colors hover:text-rose-500"
                        title={t('common.delete')}
                      >
                        <Trash2 className="h-[18px] w-[18px]" />
                      </button>
                    )}
                  </div>
                  <span className="flex items-center gap-1.5 text-[11px] font-medium tracking-tight uppercase tabular-nums text-muted-foreground/60 transition-colors">
                    <Users className="inline h-3 w-3 opacity-70" />
                    <span>{ws.teamsCount} {t('directory.workspaces.teams_count')}</span>
                    <span className="opacity-40">·</span>
                    <User className="inline h-3 w-3 opacity-70" />
                    <span>{ws.membersCount} {t('directory.workspaces.users_count')}</span>
                  </span>
                </div>
              </div>
            ))}
          </div>
        )}
      </div>
    </div>
  )
}


