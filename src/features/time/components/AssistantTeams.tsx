import React, { useState, useMemo } from 'react'
import { useTranslation } from 'react-i18next'
import { Users, ChevronLeft, Search, Inbox } from 'lucide-react'
import { Button } from '@/components/ui/button'
import { Input } from '@/components/ui/input'
import { StatusBadge } from './StatusBadge'
import type { TimeReportUI, DevRole } from '../types'

interface AssistantTeamsProps {
  dbTeams: Team[]
  shifts: TimeReportUI[]
  activeRole: DevRole
  workspaceId: string | null
  openTeamShifts: (name: string) => void
  onBack?: () => void
}

interface Team {
  id: string
  name: string
  workspace_id: string
}

export const AssistantTeams: React.FC<AssistantTeamsProps> = ({
  dbTeams,
  shifts,
  activeRole,
  workspaceId,
  openTeamShifts,
  onBack,
}) => {
  const { t } = useTranslation()
  const [searchQuery, setSearchQuery] = useState('')

  const teamsWithStats = useMemo(() => {
    return dbTeams
      .filter((team) => activeRole === 'platform_admin' || team.workspace_id === workspaceId)
      .map((team) => {
        const tSh = shifts.filter((s) => s.teamId === team.id)
        return {
          id: team.id,
          name: team.name,
          totalHours: tSh.reduce((a, b) => a + b.duration, 0),
          status: tSh.some((s) => s.status === 'pending_attest')
            ? 'pending_attest'
            : 'approved',
        }
      })
  }, [dbTeams, shifts, activeRole, workspaceId])

  const filteredTeams = useMemo(() => {
    if (!searchQuery.trim()) return teamsWithStats
    const q = searchQuery.toLowerCase()
    return teamsWithStats.filter((team) => team.name.toLowerCase().includes(q))
  }, [teamsWithStats, searchQuery])

  return (
    <div className="relative flex h-full flex-1 flex-col bg-background selection:bg-primary/20">
      {/* Top Header Controls - exactly matching Inbox / PlatformOverview / TeamOverview */}
      <div className="sticky top-0 z-20 flex h-12 shrink-0 items-center justify-between border-b border-border/50 bg-background/50 px-6 backdrop-blur-md">
        <div className="flex items-center gap-2.5">
          {onBack && (
            <>
              <Button
                variant="ghost"
                size="icon"
                onClick={onBack}
                className="h-8 w-8 rounded-md text-muted-foreground hover:text-foreground hover:bg-secondary"
                aria-label={t('common.back')}
              >
                <ChevronLeft className="h-3.5 w-3.5" />
              </Button>
              <div className="h-3.5 w-px bg-border/40" />
            </>
          )}
          <h2 className="flex items-center gap-1.5 text-xs font-medium text-foreground">
            <Users className="h-3.5 w-3.5 text-muted-foreground" />
            {t('timereports.my_teams_assignments')}
          </h2>
          <div className="rounded border border-border/50 bg-muted/60 px-1.5 py-0.5 text-[10px] font-medium text-muted-foreground">
            {filteredTeams.length} {t('timereports.teams', 'team').toLowerCase()}
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

      {/* Teams List - exactly matching Inbox row height & layout */}
      <div className="scrollbar-dark w-full flex-1 overflow-y-auto scroll-smooth">
        {filteredTeams.length === 0 ? (
          <div className="flex h-full flex-col items-center justify-center text-muted-foreground duration-500 animate-in fade-in zoom-in-95">
            <div className="mb-6 flex h-20 w-20 items-center justify-center rounded-full border border-border/50 bg-muted/30">
              <Inbox className="h-10 w-10 opacity-20" />
            </div>
            <p className="text-sm font-medium">{t('timereports.no_teams')}</p>
            <p className="mt-1 max-w-[250px] text-center text-xs text-muted-foreground/70">
              {t('timereports.no_teams_description')}
            </p>
          </div>
        ) : (
          <div className="flex w-full flex-col text-sm">
            {filteredTeams.map((team, idx) => (
              <div
                key={team.id}
                onClick={() => openTeamShifts(team.name)}
                style={{ animationDelay: `${idx * 30}ms` }}
                className="group flex h-12 cursor-pointer items-center border-b border-border/40 px-6 transition-all duration-300 hover:bg-secondary/40 animate-in fade-in slide-in-from-bottom-2 fill-mode-both"
                tabIndex={0}
                onKeyDown={(e) => {
                  if (e.key === 'Enter') openTeamShifts(team.name)
                }}
              >
                {/* Compact Leading Icon */}
                <div className="mr-3 flex size-6 shrink-0 items-center justify-center text-muted-foreground/70 transition-colors group-hover:text-primary">
                  <Users className="h-4 w-4" />
                </div>

                {/* Name Column - identical to Inbox */}
                <div className="w-36 sm:w-48 md:w-56 shrink-0 truncate pr-3 text-sm font-medium text-foreground transition-colors group-hover:text-primary">
                  {team.name}
                </div>

                {/* Details / ID (Middle Column) - identical to Inbox */}
                <div className="flex min-w-0 flex-1 items-center gap-x-2 truncate pr-6">
                  <span className="truncate text-xs font-mono text-muted-foreground/50">
                    ID: {team.id.substring(0, 8)}...
                  </span>
                </div>

                {/* Status & Hours - identical to Inbox */}
                <div className="flex w-52 shrink-0 items-center justify-end">
                  <StatusBadge status={team.status} />
                  <span className="ml-3 text-[11px] font-medium tracking-tight uppercase tabular-nums text-muted-foreground/60 transition-colors">
                    {team.totalHours} {t('timereports.hours_abbr')}
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

