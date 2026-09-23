import React, { useState, useMemo } from 'react'
import { useTranslation } from 'react-i18next'
import {
  Clock,
  Check,
  Search,
  Inbox,
} from 'lucide-react'
import { Button } from '@/components/ui/button'
import { Input } from '@/components/ui/input'
import { Checkbox } from '@/components/ui/checkbox'
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select'
import { StatusBadge } from './StatusBadge'
import { formatMonthYear } from '../utils/timeFormatters'
import type { TimeReportUI } from '../types'
import type { User } from '@/types'

interface TeamItem {
  id: string
  name: string
}

interface ToAttestViewProps {
  shifts: TimeReportUI[]
  teams: TeamItem[]
  users: User[]
  onApproveSingle: (shiftId: string) => Promise<void>
  onApproveBatch: (shiftIds: string[]) => Promise<void>
}

type SortOption = 'date_desc' | 'date_asc' | 'hours_desc' | 'hours_asc' | 'employee' | 'team'

export const ToAttestView: React.FC<ToAttestViewProps> = ({
  shifts,
  users,
  onApproveSingle,
  onApproveBatch,
}) => {
  const { t } = useTranslation()

  // Selection & simplified filter states (max 3 dropdowns: Period, Medarbetare, Sortering)
  const [selectedIds, setSelectedIds] = useState<string[]>([])
  const [selectedMonth, setSelectedMonth] = useState<string>('all')
  const [selectedUserFilter, setSelectedUserFilter] = useState<string>('all')
  const [sortBy, setSortBy] = useState<SortOption>('date_desc')
  const [searchQuery, setSearchQuery] = useState('')
  const [isApprovingBatch, setIsApprovingBatch] = useState(false)

  // Only pending attest shifts
  const pendingShifts = useMemo(() => {
    return shifts.filter((s) => s.status === 'pending_attest')
  }, [shifts])

  // Extract distinct months with pending hours
  const availableMonths = useMemo(() => {
    const map = new Map<string, { key: string; label: string; hours: number; count: number }>()
    pendingShifts.forEach((s) => {
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
  }, [pendingShifts])

  // Filtered and sorted shifts
  const filteredShifts = useMemo(() => {
    const list = pendingShifts.filter((s) => {
      if (selectedMonth !== 'all' && !s.date.startsWith(selectedMonth)) {
        return false
      }
      if (selectedUserFilter !== 'all' && s.employeeId !== selectedUserFilter) {
        return false
      }

      if (searchQuery) {
        const q = searchQuery.toLowerCase()
        const matchSearch =
          s.team.toLowerCase().includes(q) ||
          s.employee.toLowerCase().includes(q) ||
          s.note.toLowerCase().includes(q) ||
          s.date.includes(q)
        if (!matchSearch) return false
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
        case 'employee':
          return a.employee.localeCompare(b.employee)
        case 'team':
          return a.team.localeCompare(b.team)
        default:
          return b.date.localeCompare(a.date)
      }
    })
  }, [
    pendingShifts,
    selectedMonth,
    selectedUserFilter,
    searchQuery,
    sortBy,
  ])

  // Selection logic
  const isAllSelected =
    filteredShifts.length > 0 && selectedIds.length === filteredShifts.length

  const toggleSelectAll = () => {
    if (isAllSelected) {
      setSelectedIds([])
    } else {
      setSelectedIds(filteredShifts.map((s) => s.id))
    }
  }

  const toggleSelectOne = (id: string) => {
    setSelectedIds((prev) =>
      prev.includes(id) ? prev.filter((item) => item !== id) : [...prev, id]
    )
  }

  const selectedShifts = useMemo(() => {
    return filteredShifts.filter((s) => selectedIds.includes(s.id))
  }, [filteredShifts, selectedIds])

  const totalSelectedHours = useMemo(() => {
    return Math.round(selectedShifts.reduce((acc, s) => acc + s.duration, 0) * 10) / 10
  }, [selectedShifts])

  const totalPendingHours = useMemo(() => {
    return Math.round(filteredShifts.reduce((acc, s) => acc + s.duration, 0) * 10) / 10
  }, [filteredShifts])

  const handleBatchApprove = async () => {
    if (selectedIds.length === 0) return
    setIsApprovingBatch(true)
    try {
      await onApproveBatch(selectedIds)
      setSelectedIds([])
    } finally {
      setIsApprovingBatch(false)
    }
  }

  return (
    <div className="flex flex-1 flex-col overflow-hidden min-h-0 bg-background">
      {/* Subheader Controls - Exact h-12 Standard with 3 clean dropdowns */}
      <div className="flex h-12 shrink-0 items-center justify-between border-b border-border/50 bg-background/50 px-6 backdrop-blur-sm whitespace-nowrap">
        <div className="flex items-center gap-4">
          <div className="flex items-center justify-center">
            <Checkbox
              checked={isAllSelected}
              onCheckedChange={toggleSelectAll}
              aria-label={t('common.select_all', 'Markera alla')}
              className="size-4"
            />
          </div>

          <div className="text-xs text-muted-foreground font-mono">
            {selectedIds.length > 0 ? (
              <span>
                <strong className="text-foreground">{selectedIds.length}</strong> av{' '}
                {filteredShifts.length} valda ({totalSelectedHours}h)
              </span>
            ) : (
              <span>
                <strong className="text-foreground">{filteredShifts.length}</strong> pass att
                attestera ({totalPendingHours}h)
              </span>
            )}
          </div>

          {/* Action button when items are selected */}
          {selectedIds.length > 0 && (
            <div className="flex items-center gap-2 border-l border-border/50 pl-4 duration-200 animate-in fade-in slide-in-from-left-2">
              <Button
                size="sm"
                onClick={handleBatchApprove}
                disabled={isApprovingBatch}
                className="h-8 gap-1.5 px-3 text-xs font-medium bg-primary text-primary-foreground hover:bg-primary/90"
              >
                <Check className="size-3" />
                <span>
                  {selectedIds.length === filteredShifts.length
                    ? `Godkänn & attestera alla (${selectedIds.length})`
                    : `Attestera valda (${selectedIds.length})`}
                </span>
              </Button>
            </div>
          )}
        </div>

        {/* Right controls: Exact 3 dropdowns (Period, Medarbetare, Sortering) + Sök */}
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

          {/* Dropdown 2: Medarbetare */}
          {users.length > 1 && (
            <Select value={selectedUserFilter} onValueChange={setSelectedUserFilter}>
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
          )}

          {/* Dropdown 3: Sortering */}
          <Select value={sortBy} onValueChange={(val) => setSortBy(val as SortOption)}>
            <SelectTrigger className="h-8 w-[145px] border-border/50 bg-muted/40 text-xs font-medium text-foreground">
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
              <SelectItem value="employee">Medarbetare (A-Ö)</SelectItem>
              <SelectItem value="team">Team (A-Ö)</SelectItem>
            </SelectContent>
          </Select>

          {/* Sök */}
          <div className="group relative hidden sm:block w-44">
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
            <p className="text-sm font-medium text-foreground">
              Inga tidsrapporter att attestera
            </p>
            <p className="mt-1 max-w-[320px] text-xs text-muted-foreground/70">
              Alla inlämnade tidsrapporter för valt filter är redan godkända eller inga pass väntar på attest.
            </p>
          </div>
        ) : (
          <div className="flex w-full flex-col text-sm">
            {filteredShifts.map((shift, idx) => {
              const isSelected = selectedIds.includes(shift.id)
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
                  onClick={() => toggleSelectOne(shift.id)}
                  className={`group flex h-12 cursor-pointer items-center border-b border-border/40 px-6 transition-all duration-150 hover:bg-secondary/40 whitespace-nowrap overflow-hidden animate-in fade-in slide-in-from-bottom-2 fill-mode-both ${
                    isSelected ? 'bg-primary/5' : ''
                  }`}
                >
                  {/* Select Checkbox */}
                  <div
                    className="mr-3 flex size-6 shrink-0 cursor-pointer items-center justify-center"
                    onClick={(e) => {
                      e.stopPropagation()
                      toggleSelectOne(shift.id)
                    }}
                  >
                    <Checkbox
                      checked={isSelected}
                      onCheckedChange={() => toggleSelectOne(shift.id)}
                      className="size-4"
                    />
                  </div>

                  {/* Employee Name - Strict Single Line */}
                  <div className="flex items-center gap-2.5 w-44 sm:w-52 shrink-0 pr-3 truncate whitespace-nowrap">
                    <div className="flex size-6 shrink-0 items-center justify-center rounded-full bg-secondary text-[10px] font-medium text-foreground border border-border/60">
                      {initials}
                    </div>
                    <span className="truncate text-sm font-medium text-foreground transition-colors group-hover:text-primary">
                      {shift.employee}
                    </span>
                  </div>

                  {/* Team - Strict Single Line */}
                  <div className="w-28 sm:w-36 shrink-0 truncate pr-3 text-xs text-muted-foreground">
                    {shift.team}
                  </div>

                  {/* Note & Times - Strict Single Line */}
                  <div className="flex min-w-0 flex-1 items-center gap-x-4 truncate pr-6 text-sm font-light">
                    <span className="truncate text-xs italic text-muted-foreground/60">
                      {shift.note || 'Tidsrapport inlämnad'}
                    </span>
                    <div className="flex shrink-0 items-center gap-1.5 font-mono text-xs text-muted-foreground/80">
                      <Clock className="size-3 opacity-60" />
                      <span>
                        {shift.start} - {shift.end}
                      </span>
                      <span className="opacity-40">·</span>
                      <span className="font-medium text-foreground/90">{shift.duration}h</span>
                    </div>
                  </div>

                  {/* Status, Date & Quick Attest Action - Strict Single Line */}
                  <div className="flex w-60 shrink-0 items-center justify-end gap-3.5 whitespace-nowrap">
                    <StatusBadge status={shift.status} />

                    <span className="font-mono text-xs tabular-nums text-muted-foreground/60 w-20 text-right shrink-0">
                      {shift.date}
                    </span>

                    <Button
                      size="sm"
                      variant="secondary"
                      onClick={(e) => {
                        e.stopPropagation()
                        onApproveSingle(shift.id)
                      }}
                      className="h-7 gap-1 px-2.5 text-[11px] font-medium hover:bg-emerald-600 hover:text-white transition-colors shadow-2xs shrink-0"
                    >
                      <Check className="size-3" />
                      <span>Attestera</span>
                    </Button>
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
