import React, { useState, useMemo } from 'react'
import { useTranslation } from 'react-i18next'
import {
  Clock,
  Search,
  Inbox,
} from 'lucide-react'
import { Input } from '@/components/ui/input'
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select'
import { StatusBadge } from './StatusBadge'
import { formatMonthYear } from '../utils/timeFormatters'
import type { TimeReportUI, DevRole } from '../types'
import type { User } from '@/types'

interface TeamItem {
  id: string
  name: string
}

interface TimeHistoryViewProps {
  shifts: TimeReportUI[]
  teams: TeamItem[]
  users: User[]
  activeRole: DevRole
  currentUserId?: string
}

type SortOption = 'date_desc' | 'date_asc' | 'hours_desc' | 'hours_asc'
type StatusFilter = 'all' | 'approved' | 'pending_attest'

export const TimeHistoryView: React.FC<TimeHistoryViewProps> = ({
  shifts,
  teams,
  users,
  activeRole,
}) => {
  const { t } = useTranslation()

  const [statusFilter, setStatusFilter] = useState<StatusFilter>('all')
  const [selectedMonth, setSelectedMonth] = useState<string>('all')
  const [selectedEntityFilter, setSelectedEntityFilter] = useState<string>('all')
  const [sortBy, setSortBy] = useState<SortOption>('date_desc')
  const [searchQuery, setSearchQuery] = useState('')

  const isAdmin = activeRole === 'admin' || activeRole === 'platform_admin'

  // Reported shifts (approved or pending_attest)
  const reportedShifts = useMemo(() => {
    return shifts.filter((s) => s.status !== 'not_submitted')
  }, [shifts])

  // Extract distinct months with reported hours
  const availableMonths = useMemo(() => {
    const map = new Map<string, { key: string; label: string; hours: number; count: number }>()
    reportedShifts.forEach((s) => {
      if (!s.date) return
      const ym = s.date.substring(0, 7)
      const existing = map.get(ym) || {
        key: ym,
        label: formatMonthYear(ym),
        hours: 0,
        count: 0,
      }
      existing.hours = Math.round((existing.hours + s.duration) * 10) / 10
      existing.count += 1
      map.set(ym, existing)
    })
    return Array.from(map.values()).sort((a, b) => b.key.localeCompare(a.key))
  }, [reportedShifts])

  // Filter & sort
  const filteredShifts = useMemo(() => {
    const list = reportedShifts.filter((s) => {
      if (statusFilter !== 'all' && s.status !== statusFilter) {
        return false
      }
      if (selectedMonth !== 'all' && !s.date.startsWith(selectedMonth)) {
        return false
      }
      if (selectedEntityFilter !== 'all') {
        if (isAdmin && s.employeeId !== selectedEntityFilter) {
          return false
        }
        if (!isAdmin && s.teamId !== selectedEntityFilter) {
          return false
        }
      }

      if (searchQuery) {
        const q = searchQuery.toLowerCase()
        const match =
          s.team.toLowerCase().includes(q) ||
          s.employee.toLowerCase().includes(q) ||
          s.note.toLowerCase().includes(q) ||
          s.date.includes(q)
        if (!match) return false
      }

      return true
    })

    return list.sort((a, b) => {
      switch (sortBy) {
        case 'date_asc':
          return a.date.localeCompare(b.date)
        case 'date_desc':
          return b.date.localeCompare(a.date)
        case 'hours_desc':
          return b.duration - a.duration
        case 'hours_asc':
          return a.duration - b.duration
        default:
          return b.date.localeCompare(a.date)
      }
    })
  }, [
    reportedShifts,
    statusFilter,
    selectedMonth,
    selectedEntityFilter,
    isAdmin,
    searchQuery,
    sortBy,
  ])

  // Metric summaries (scoped to filtered view)
  const metrics = useMemo(() => {
    let totalHours = 0
    let approvedHours = 0
    let pendingHours = 0

    let totalCount = 0
    let approvedCount = 0
    let pendingCount = 0

    filteredShifts.forEach((s) => {
      totalHours += s.duration
      totalCount += 1
      if (s.status === 'approved') {
        approvedHours += s.duration
        approvedCount += 1
      } else if (s.status === 'pending_attest') {
        pendingHours += s.duration
        pendingCount += 1
      }
    })

    return {
      totalHours: Math.round(totalHours * 10) / 10,
      approvedHours: Math.round(approvedHours * 10) / 10,
      pendingHours: Math.round(pendingHours * 10) / 10,
      totalCount,
      approvedCount,
      pendingCount,
    }
  }, [filteredShifts])

  return (
    <div className="flex flex-1 flex-col overflow-hidden min-h-0 bg-background">
      {/* Sleek Horizontal Stats Strip - Single Line */}
      <div className="flex h-10 shrink-0 items-center gap-6 border-b border-border/40 bg-secondary/15 px-6 text-xs text-muted-foreground whitespace-nowrap overflow-x-auto">
        <div>
          <span>Totalt rapporterat: </span>
          <strong className="font-mono text-foreground font-medium">{metrics.totalHours}h</strong>
          <span className="text-[11px] text-muted-foreground/70"> ({metrics.totalCount} pass)</span>
        </div>

        <div className="h-3 w-px bg-border/40" />

        <div className="flex items-center gap-1.5">
          <span className="h-1.5 w-1.5 rounded-full bg-emerald-500 shrink-0" />
          <span>Attesterat: </span>
          <strong className="font-mono text-foreground font-medium">{metrics.approvedHours}h</strong>
          <span className="text-[11px] text-muted-foreground/70"> ({metrics.approvedCount} pass)</span>
        </div>

        <div className="h-3 w-px bg-border/40" />

        <div className="flex items-center gap-1.5">
          <span className="h-1.5 w-1.5 rounded-full bg-amber-500 shrink-0" />
          <span>Väntar på attest: </span>
          <strong className="font-mono text-foreground font-medium">{metrics.pendingHours}h</strong>
          <span className="text-[11px] text-muted-foreground/70"> ({metrics.pendingCount} pass)</span>
        </div>
      </div>

      {/* Subheader Controls - Exact h-12 Standard */}
      <div className="flex h-12 shrink-0 items-center justify-between border-b border-border/50 bg-background/50 px-6 backdrop-blur-sm whitespace-nowrap">
        {/* Status Filter Segment */}
        <div className="flex items-center gap-1.5">
          <div className="flex items-center rounded-md border border-border/50 bg-muted/40 p-0.5">
            <button
              onClick={() => setStatusFilter('all')}
              className={`h-7 rounded px-2.5 text-xs font-medium transition-colors ${
                statusFilter === 'all'
                  ? 'bg-background text-foreground shadow-2xs'
                  : 'text-muted-foreground hover:text-foreground'
              }`}
            >
              Alla ({reportedShifts.length})
            </button>
            <button
              onClick={() => setStatusFilter('approved')}
              className={`flex items-center gap-1.5 h-7 rounded px-2.5 text-xs font-medium transition-colors ${
                statusFilter === 'approved'
                  ? 'bg-background text-foreground shadow-2xs'
                  : 'text-muted-foreground hover:text-foreground'
              }`}
            >
              <span className="h-1.5 w-1.5 rounded-full bg-emerald-500 shrink-0" />
              <span>Attesterade</span>
            </button>
            <button
              onClick={() => setStatusFilter('pending_attest')}
              className={`flex items-center gap-1.5 h-7 rounded px-2.5 text-xs font-medium transition-colors ${
                statusFilter === 'pending_attest'
                  ? 'bg-background text-foreground shadow-2xs'
                  : 'text-muted-foreground hover:text-foreground'
              }`}
            >
              <span className="h-1.5 w-1.5 rounded-full bg-amber-500 shrink-0" />
              <span>Väntande</span>
            </button>
          </div>
        </div>

        {/* Right Controls: Exact 3 dropdowns (Period, Medarbetare/Team, Sortering) + Sök */}
        <div className="flex items-center gap-2.5">
          {/* Dropdown 1: Period / Månad */}
          <Select value={selectedMonth} onValueChange={setSelectedMonth}>
            <SelectTrigger className="h-8 w-[160px] border-border/50 bg-muted/40 text-xs font-medium text-foreground">
              <SelectValue placeholder={t('timereports.all_periods', 'Alla perioder')} />
            </SelectTrigger>
            <SelectContent>
              <SelectItem value="all">{t('timereports.all_periods', 'Alla perioder')}</SelectItem>
              {availableMonths.map((m) => (
                <SelectItem key={m.key} value={m.key}>
                  {m.label} ({m.hours}h)
                </SelectItem>
              ))}
            </SelectContent>
          </Select>

          {/* Dropdown 2: Medarbetare (Admin) eller Team (Assistent) */}
          {isAdmin ? (
            users.length > 1 && (
              <Select value={selectedEntityFilter} onValueChange={setSelectedEntityFilter}>
                <SelectTrigger className="h-8 w-[145px] border-border/50 bg-muted/40 text-xs font-medium text-foreground">
                  <SelectValue placeholder="Alla medarbetare" />
                </SelectTrigger>
                <SelectContent>
                  <SelectItem value="all">Alla medarbetare</SelectItem>
                  {users.map((u) => (
                    <SelectItem key={u.id} value={u.id}>
                      {u.full_name || u.email}
                    </SelectItem>
                  ))}
                </SelectContent>
              </Select>
            )
          ) : (
            teams.length > 1 && (
              <Select value={selectedEntityFilter} onValueChange={setSelectedEntityFilter}>
                <SelectTrigger className="h-8 w-[130px] border-border/50 bg-muted/40 text-xs font-medium text-foreground">
                  <SelectValue placeholder="Alla team" />
                </SelectTrigger>
                <SelectContent>
                  <SelectItem value="all">Alla team</SelectItem>
                  {teams.map((tItem) => (
                    <SelectItem key={tItem.id} value={tItem.id}>
                      {tItem.name}
                    </SelectItem>
                  ))}
                </SelectContent>
              </Select>
            )
          )}

          {/* Dropdown 3: Sortering */}
          <Select value={sortBy} onValueChange={(val) => setSortBy(val as SortOption)}>
            <SelectTrigger className="h-8 w-[140px] border-border/50 bg-muted/40 text-xs font-medium text-foreground">
              <SelectValue placeholder={t('timereports.sort_by', 'Sortering')} />
            </SelectTrigger>
            <SelectContent>
              <SelectItem value="date_desc">
                {t('timereports.sort_date_desc', 'Datum (Nyast)')}
              </SelectItem>
              <SelectItem value="date_asc">
                {t('timereports.sort_date_asc', 'Datum (Äldst)')}
              </SelectItem>
              <SelectItem value="hours_desc">
                {t('timereports.sort_hours_desc', 'Timmar (Flest)')}
              </SelectItem>
              <SelectItem value="hours_asc">
                {t('timereports.sort_hours_asc', 'Timmar (Minst)')}
              </SelectItem>
            </SelectContent>
          </Select>

          {/* Sök */}
          <div className="group relative hidden sm:block w-40">
            <Search className="absolute left-2.5 top-1/2 size-3.5 -translate-y-1/2 text-muted-foreground transition-colors group-focus-within:text-primary" />
            <Input
              placeholder={t('timereports.search_placeholder', 'Sök pass...')}
              value={searchQuery}
              onChange={(e) => setSearchQuery(e.target.value)}
              className="h-8 rounded-md border border-border/50 bg-muted/40 pl-8 text-xs font-medium text-foreground transition-all focus-visible:bg-muted"
            />
          </div>
        </div>
      </div>

      {/* Main List - Exact Inbox h-12 Standard Rows with NO text wrapping */}
      <div className="scrollbar-dark w-full flex-1 overflow-y-auto scroll-smooth">
        {filteredShifts.length === 0 ? (
          <div className="flex h-72 flex-col items-center justify-center p-6 text-center text-muted-foreground duration-500 animate-in fade-in zoom-in-95">
            <div className="mb-4 flex size-14 items-center justify-center rounded-md border border-border/40 bg-muted/20">
              <Inbox className="size-7 text-muted-foreground/40" />
            </div>
            <p className="text-sm font-medium text-foreground">Ingen historik hittades</p>
            <p className="mt-1 max-w-[320px] text-xs text-muted-foreground/70">
              Inga rapporterade arbetspass matchar valt filter eller sökkriterium.
            </p>
          </div>
        ) : (
          <div className="flex w-full flex-col text-sm">
            {filteredShifts.map((shift, idx) => {
              const initials = shift.employee
                .split(' ')
                .map((n) => n[0])
                .join('')
                .slice(0, 2)
                .toUpperCase()

              return (
                <div
                  key={shift.id}
                  style={{ animationDelay: `${idx * 15}ms` }}
                  className="group flex h-12 items-center border-b border-border/40 px-6 transition-all duration-150 hover:bg-secondary/40 whitespace-nowrap overflow-hidden animate-in fade-in slide-in-from-bottom-2 fill-mode-both"
                >
                  {/* Left: Employee if Admin, or Team - Strict Single Line */}
                  {isAdmin ? (
                    <div className="flex items-center gap-2.5 w-44 sm:w-52 shrink-0 pr-3 truncate whitespace-nowrap">
                      <div className="flex size-6 shrink-0 items-center justify-center rounded-full bg-secondary text-[10px] font-medium text-foreground border border-border/60">
                        {initials}
                      </div>
                      <span className="truncate text-sm font-medium text-foreground">
                        {shift.employee}
                      </span>
                    </div>
                  ) : (
                    <div className="w-36 sm:w-48 shrink-0 truncate pr-3 text-sm font-medium text-foreground">
                      {shift.team}
                    </div>
                  )}

                  {/* Team for Admin */}
                  {isAdmin && (
                    <div className="w-28 sm:w-36 shrink-0 truncate pr-3 text-xs text-muted-foreground">
                      {shift.team}
                    </div>
                  )}

                  {/* Note / Details - Strict Single Line */}
                  <div className="flex min-w-0 flex-1 items-center gap-x-4 truncate pr-6 text-sm font-light">
                    <span className="truncate text-xs italic text-muted-foreground/60">
                      {shift.note || 'Arbetspass'}
                    </span>
                    <div className="flex shrink-0 items-center gap-1.5 font-mono text-xs text-muted-foreground/80">
                      <Clock className="size-3 opacity-60" />
                      <span>
                        {shift.start} - {shift.end}
                      </span>
                      <span className="opacity-40">·</span>
                      <span className="font-medium text-foreground/80">{shift.duration}h</span>
                    </div>
                  </div>

                  {/* Status & Date - Strict Single Line */}
                  <div className="flex w-52 shrink-0 items-center justify-end gap-3.5 whitespace-nowrap">
                    <StatusBadge status={shift.status} />

                    <span className="font-mono text-xs tabular-nums text-muted-foreground/60 w-20 text-right shrink-0">
                      {shift.date}
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
