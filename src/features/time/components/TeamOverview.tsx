import React, { useState, useMemo } from 'react'
import { useTranslation } from 'react-i18next'
import { Users, ChevronLeft, Search, Inbox } from 'lucide-react'
import { Button } from '@/components/ui/button'
import { Input } from '@/components/ui/input'
import { Avatar, AvatarFallback } from '@/components/ui/avatar'
import { StatusBadge } from './StatusBadge'
import type { TimeReportUI } from '../types'

interface TeamUser {
  id: string
  full_name?: string | null
  email?: string
  role?: string
}

interface TeamOverviewProps {
  dbUsers: TeamUser[]
  shifts: TimeReportUI[]
  loading: boolean
  openEmployeeShifts: (id: string) => void
  onBack?: () => void
}

export const TeamOverview: React.FC<TeamOverviewProps> = ({
  dbUsers,
  shifts,
  loading,
  openEmployeeShifts,
  onBack,
}) => {
  const { t } = useTranslation()
  const [searchQuery, setSearchQuery] = useState('')

  const membersWithStats = useMemo(() => {
    return dbUsers.map((u) => {
      const eSh = shifts.filter((s) => s.employeeId === u.id)
      const statusStr =
        eSh.length === 0
          ? 'not_submitted'
          : eSh.some((s) => s.status === 'pending_attest')
            ? 'pending_attest'
            : 'approved'
      return {
        id: u.id,
        name: u.full_name || u.email || t('common.unknown_agent'),
        email: u.email || '',
        role: u.role === 'admin' ? t('directory.roles.admin') : t('directory.roles.assistant'),
        totalHours: eSh.reduce((a, b) => a + b.duration, 0),
        status: statusStr,
      }
    })
  }, [dbUsers, shifts, t])

  const filteredMembers = useMemo(() => {
    if (!searchQuery.trim()) return membersWithStats
    const q = searchQuery.toLowerCase()
    return membersWithStats.filter(
      (m) =>
        m.name.toLowerCase().includes(q) ||
        m.email.toLowerCase().includes(q) ||
        m.role.toLowerCase().includes(q),
    )
  }, [membersWithStats, searchQuery])

  return (
    <div className="relative flex h-full flex-1 flex-col bg-background selection:bg-primary/20">
      {/* Top Header Controls - exactly matching Inbox / Notes */}
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
            {t('timereports.team_members')}
          </h2>
          <div className="rounded border border-border/50 bg-muted/60 px-1.5 py-0.5 text-[10px] font-medium text-muted-foreground">
            {filteredMembers.length} {t('timereports.team_members').toLowerCase()}
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

      {/* Members List - exactly matching Inbox row height & layout */}
      <div className="scrollbar-dark w-full flex-1 overflow-y-auto scroll-smooth">
        {loading ? (
          <div className="flex w-full flex-col text-sm">
            {[1, 2, 3, 4, 5, 6].map((i) => (
              <div
                key={i}
                className="flex h-12 w-full animate-pulse items-center border-b border-border/40 px-6"
              >
                <div className="mr-3 size-6 shrink-0 rounded-full bg-muted" />
                <div className="w-36 sm:w-48 md:w-56 shrink-0 pr-3">
                  <div className="h-3.5 w-28 rounded bg-muted" />
                </div>
                <div className="min-w-0 flex-1 pr-6">
                  <div className="h-3 w-20 rounded bg-muted/60" />
                </div>
                <div className="flex w-52 shrink-0 items-center justify-end gap-3">
                  <div className="h-3 w-16 rounded bg-muted/50" />
                  <div className="h-3 w-10 rounded bg-muted/60" />
                </div>
              </div>
            ))}
          </div>
        ) : filteredMembers.length === 0 ? (
          <div className="flex h-full flex-col items-center justify-center text-muted-foreground duration-500 animate-in fade-in zoom-in-95">
            <div className="mb-6 flex h-20 w-20 items-center justify-center rounded-full border border-border/50 bg-muted/30">
              <Inbox className="h-10 w-10 opacity-20" />
            </div>
            <p className="text-sm font-medium">{t('notes.list.empty_state')}</p>
          </div>
        ) : (
          <div className="flex w-full flex-col text-sm">
            {filteredMembers.map((emp, idx) => (
              <div
                key={emp.id}
                onClick={() => openEmployeeShifts(emp.id)}
                style={{ animationDelay: `${idx * 30}ms` }}
                className="group flex h-12 cursor-pointer items-center border-b border-border/40 px-6 transition-all duration-300 hover:bg-secondary/40 animate-in fade-in slide-in-from-bottom-2 fill-mode-both"
                tabIndex={0}
                onKeyDown={(e) => {
                  if (e.key === 'Enter') openEmployeeShifts(emp.id)
                }}
              >
                {/* Compact Size-6 User Avatar */}
                <div className="mr-3 flex size-6 shrink-0 items-center justify-center">
                  <Avatar className="size-6 border border-border/60">
                    <AvatarFallback className="bg-secondary/80 text-[10px] font-medium text-foreground/80">
                      {emp.name.charAt(0).toUpperCase()}
                    </AvatarFallback>
                  </Avatar>
                </div>

                {/* Name Column - identical to Inbox */}
                <div className="w-36 sm:w-48 md:w-56 shrink-0 truncate pr-3 text-sm font-medium text-foreground transition-colors">
                  {emp.name}
                </div>

                {/* Role / Details (Middle Column) - identical to Inbox */}
                <div className="flex min-w-0 flex-1 items-center gap-x-2 truncate pr-6">
                  <span className="truncate text-sm font-light italic text-muted-foreground/60">
                    {emp.role}
                  </span>
                </div>

                {/* Status & Hours - identical to Inbox */}
                <div className="flex w-52 shrink-0 items-center justify-end">
                  <StatusBadge status={emp.status} />
                  <span className="ml-3 text-[11px] font-medium tracking-tight uppercase tabular-nums text-muted-foreground/60 transition-colors">
                    {emp.totalHours} {t('timereports.hours_abbr')}
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
