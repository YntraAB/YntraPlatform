import React, { useState } from 'react'
import { Building2, Search, Clock, Inbox } from 'lucide-react'
import { useTranslation } from 'react-i18next'
import { Input } from '@/components/ui/input'
import type { TimeReportUI } from '../types'

interface Workspace {
  id: string
  name: string
  type?: string
}

interface PlatformOverviewProps {
  dbWorkspaces: Workspace[]
  shifts: TimeReportUI[]
  onSelectWorkspace?: (id: string) => void
}

export const PlatformOverview: React.FC<PlatformOverviewProps> = ({
  dbWorkspaces,
  shifts,
  onSelectWorkspace,
}) => {
  const { t } = useTranslation()
  const [searchQuery, setSearchQuery] = useState('')

  let filteredWorkspaces = dbWorkspaces
  if (searchQuery.trim()) {
    const q = searchQuery.toLowerCase()
    filteredWorkspaces = filteredWorkspaces.filter(
      (ws) =>
        ws.name.toLowerCase().includes(q) ||
        (ws.type && ws.type.toLowerCase().includes(q)),
    )
  }

  return (
    <div className="relative flex h-full flex-1 flex-col bg-background selection:bg-primary/20">
      {/* Top Header Controls - exactly matching Inbox / Directory */}
      <div className="sticky top-0 z-20 flex h-12 shrink-0 items-center justify-between border-b border-border/50 bg-background/50 px-6 backdrop-blur-md">
        <div className="flex items-center gap-3">
          <h2 className="flex items-center gap-1.5 text-xs font-medium text-foreground">
            <Building2 className="h-3.5 w-3.5 text-muted-foreground" />
            {t('directory.levels.workspaces', 'Organisationer')}
          </h2>
          <div className="rounded border border-border/50 bg-muted/60 px-1.5 py-0.5 text-[10px] font-medium text-muted-foreground">
            {filteredWorkspaces.length} {t('directory.levels.workspaces', 'Organisationer').toLowerCase()}
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
            {filteredWorkspaces.map((ws, idx) => {
              const wsShifts = shifts.filter((s) => s.workspaceId === ws.id)
              const totalHours = wsShifts.reduce((acc, s) => acc + s.duration, 0)
              const pendingAttest = wsShifts.filter((s) => s.status === 'pending_attest').length

              return (
                <div
                  key={ws.id}
                  onClick={() => onSelectWorkspace?.(ws.id)}
                  style={{ animationDelay: `${idx * 30}ms` }}
                  className="group flex h-12 cursor-pointer items-center border-b border-border/40 px-6 transition-all duration-300 hover:bg-secondary/40 animate-in fade-in slide-in-from-bottom-2 fill-mode-both"
                  tabIndex={0}
                  onKeyDown={(e) => {
                    if (e.key === 'Enter') onSelectWorkspace?.(ws.id)
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
                      {ws.type || 'Företag'}
                    </span>
                  </div>

                  {/* Status & Hours - identical to Inbox */}
                  <div className="flex w-52 shrink-0 items-center justify-end">
                    <span className="flex items-center gap-1.5 text-[11px] font-medium tracking-tight uppercase tabular-nums text-muted-foreground/60 transition-colors">
                      {pendingAttest > 0 ? (
                        <>
                          <span className="h-1.5 w-1.5 rounded-full bg-amber-500 shrink-0" />
                          <span className="text-amber-500/90">{pendingAttest} {t('timereports.pending')}</span>
                        </>
                      ) : (
                        <>
                          <span className="h-1.5 w-1.5 rounded-full bg-emerald-500 shrink-0" />
                          <span>{t('timereports.status.approved')}</span>
                        </>
                      )}
                      <span className="opacity-40">·</span>
                      <Clock className="inline h-3 w-3 opacity-70" />
                      <span>{totalHours} h</span>
                    </span>
                  </div>
                </div>
              )
            })}
          </div>
        )}
      </div>
    </div>
  )
}
