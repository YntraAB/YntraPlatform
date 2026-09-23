import React, { useState, useMemo } from 'react'
import { useTranslation } from 'react-i18next'
import {
  Clock,
  Check,
  Plus,
  X,
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
import { toast } from 'sonner'
import type { TimeReportUI } from '../types'

interface TeamItem {
  id: string
  name: string
}

interface ToReportViewProps {
  shifts: TimeReportUI[]
  teams: TeamItem[]
  userId?: string
  onReportSingle: (shiftId: string) => Promise<void>
  onReportBatch: (shiftIds: string[]) => Promise<void>
  onManualReport: (data: {
    teamId?: string | null
    date: string
    start: string
    end: string
    hours: number
    note?: string
  }) => Promise<boolean>
}

type SortOption = 'date_desc' | 'date_asc' | 'hours_desc' | 'hours_asc' | 'team'

export const ToReportView: React.FC<ToReportViewProps> = ({
  shifts,
  teams,
  onReportSingle,
  onReportBatch,
  onManualReport,
}) => {
  const { t } = useTranslation()

  // Selection & filter states (2 dropdowns: Period, Sortering)
  const [selectedIds, setSelectedIds] = useState<string[]>([])
  const [selectedMonth, setSelectedMonth] = useState<string>('all')
  const [sortBy, setSortBy] = useState<SortOption>('date_desc')
  const [searchQuery, setSearchQuery] = useState('')
  const [isManualOpen, setIsManualOpen] = useState(false)

  // Manual form state
  const [manualTeamId, setManualTeamId] = useState<string>(teams[0]?.id || '')
  const [manualDate, setManualDate] = useState<string>(
    new Date().toLocaleDateString('sv-SE')
  )
  const [manualStart, setManualStart] = useState('08:00')
  const [manualEnd, setManualEnd] = useState('16:00')
  const [manualBreak, setManualBreak] = useState('0')
  const [manualNote, setManualNote] = useState('')
  const [isSubmittingManual, setIsSubmittingManual] = useState(false)

  // Base list: only unsubmitted shifts
  const unsubmittedShifts = useMemo(() => {
    return shifts.filter((s) => s.status === 'not_submitted')
  }, [shifts])

  // Extract distinct months with worked hours
  const availableMonths = useMemo(() => {
    const map = new Map<string, { key: string; label: string; hours: number; count: number }>()
    unsubmittedShifts.forEach((s) => {
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
  }, [unsubmittedShifts])

  // Live calculation of manual hours
  const calculatedManualHours = useMemo(() => {
    try {
      const [sh, sm] = manualStart.split(':').map(Number)
      const [eh, em] = manualEnd.split(':').map(Number)
      let diffMinutes = eh * 60 + em - (sh * 60 + sm)
      if (diffMinutes < 0) diffMinutes += 24 * 60
      const breakMinutes = Math.max(0, parseInt(manualBreak, 10) || 0)
      const workMinutes = Math.max(0, diffMinutes - breakMinutes)
      return Math.round((workMinutes / 60) * 100) / 100
    } catch {
      return 0
    }
  }, [manualStart, manualEnd, manualBreak])

  // Filter & Sortering
  const filteredShifts = useMemo(() => {
    const list = unsubmittedShifts.filter((s) => {
      if (selectedMonth !== 'all' && !s.date.startsWith(selectedMonth)) {
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
        case 'team':
          return a.team.localeCompare(b.team)
        default:
          return b.date.localeCompare(a.date)
      }
    })
  }, [unsubmittedShifts, selectedMonth, searchQuery, sortBy])

  // Select all logic
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
    return (
      Math.round(selectedShifts.reduce((acc, s) => acc + s.duration, 0) * 10) / 10
    )
  }, [selectedShifts])

  const totalUnreportedHours = useMemo(() => {
    return Math.round(filteredShifts.reduce((acc, s) => acc + s.duration, 0) * 10) / 10
  }, [filteredShifts])

  // Submit manual
  const handleSaveManual = async (e: React.FormEvent) => {
    e.preventDefault()
    if (calculatedManualHours <= 0) {
      toast.error('Arbetstiden måste vara större än 0 timmar.')
      return
    }

    setIsSubmittingManual(true)
    try {
      const ok = await onManualReport({
        teamId: manualTeamId || null,
        date: manualDate,
        start: manualStart,
        end: manualEnd,
        hours: calculatedManualHours,
        note: manualNote,
      })
      if (ok) {
        setIsManualOpen(false)
        setManualNote('')
      }
    } finally {
      setIsSubmittingManual(false)
    }
  }

  return (
    <div className="flex flex-1 flex-col overflow-hidden min-h-0 bg-background">
      {/* Inline Manual Entry Panel */}
      {isManualOpen && (
        <div className="border-b border-border bg-secondary/30 px-6 py-4 duration-200 animate-in fade-in slide-in-from-top-2">
          <div className="mb-3 flex items-center justify-between">
            <div className="flex items-center gap-2">
              <Plus className="size-3.5 text-primary" />
              <h3 className="text-xs font-medium text-foreground">
                Registrera manuell arbetstid
              </h3>
            </div>
            <button
              onClick={() => setIsManualOpen(false)}
              className="text-muted-foreground hover:text-foreground rounded p-1"
            >
              <X className="size-3.5" />
            </button>
          </div>

          <form onSubmit={handleSaveManual} className="flex flex-col gap-3">
            <div className="grid grid-cols-1 sm:grid-cols-2 md:grid-cols-6 gap-3">
              <div className="flex flex-col gap-1 md:col-span-2">
                <label className="text-[11px] font-medium text-muted-foreground">
                  {t('timereports.client_or_team', 'Brukare / Team')}
                </label>
                <select
                  value={manualTeamId}
                  onChange={(e) => setManualTeamId(e.target.value)}
                  className="h-8 rounded-md border border-border bg-background px-2.5 text-xs text-foreground outline-none focus:ring-1 focus:ring-ring"
                >
                  {teams.map((tItem) => (
                    <option key={tItem.id} value={tItem.id}>
                      {tItem.name}
                    </option>
                  ))}
                  {teams.length === 0 && (
                    <option value="">{t('timereports.unassigned_team', 'Odelat team')}</option>
                  )}
                </select>
              </div>

              <div className="flex flex-col gap-1">
                <label className="text-[11px] font-medium text-muted-foreground">
                  {t('timereports.date_label', 'Datum')}
                </label>
                <Input
                  type="date"
                  value={manualDate}
                  onChange={(e) => setManualDate(e.target.value)}
                  required
                  className="h-8 text-xs bg-background"
                />
              </div>

              <div className="flex flex-col gap-1">
                <label className="text-[11px] font-medium text-muted-foreground">
                  {t('timereports.start_time_label', 'Start')}
                </label>
                <Input
                  type="time"
                  value={manualStart}
                  onChange={(e) => setManualStart(e.target.value)}
                  required
                  className="h-8 text-xs bg-background"
                />
              </div>

              <div className="flex flex-col gap-1">
                <label className="text-[11px] font-medium text-muted-foreground">
                  {t('timereports.end_time_label', 'Slut')}
                </label>
                <Input
                  type="time"
                  value={manualEnd}
                  onChange={(e) => setManualEnd(e.target.value)}
                  required
                  className="h-8 text-xs bg-background"
                />
              </div>

              <div className="flex flex-col gap-1">
                <label className="text-[11px] font-medium text-muted-foreground">
                  {t('timereports.break_minutes', 'Rast (min)')}
                </label>
                <Input
                  type="number"
                  min="0"
                  step="5"
                  value={manualBreak}
                  onChange={(e) => setManualBreak(e.target.value)}
                  className="h-8 text-xs bg-background"
                />
              </div>
            </div>

            <div className="flex flex-col sm:flex-row items-center justify-between gap-3 pt-1">
              <div className="flex-1 w-full sm:max-w-md">
                <Input
                  type="text"
                  placeholder={t(
                    'timereports.note_placeholder',
                    'Valfri anteckning (t.ex. extrapass, övertid)...'
                  )}
                  value={manualNote}
                  onChange={(e) => setManualNote(e.target.value)}
                  className="h-8 text-xs bg-background"
                />
              </div>

              <div className="flex items-center gap-3 shrink-0">
                <span className="font-mono text-xs text-muted-foreground">
                  Arbetstid:{' '}
                  <strong className="text-foreground">{calculatedManualHours}h</strong>
                </span>
                <Button
                  type="button"
                  variant="outline"
                  size="sm"
                  onClick={() => setIsManualOpen(false)}
                  className="h-8 px-3 text-xs"
                >
                  {t('common.cancel', 'Avbryt')}
                </Button>
                <Button
                  type="submit"
                  size="sm"
                  disabled={isSubmittingManual || calculatedManualHours <= 0}
                  className="h-8 px-3 text-xs font-medium"
                >
                  <Check className="mr-1.5 size-3" />
                  <span>Spara & skicka för attest</span>
                </Button>
              </div>
            </div>
          </form>
        </div>
      )}

      {/* Subheader Controls - Exact h-12 Standard */}
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
                rapportera ({totalUnreportedHours}h)
              </span>
            )}
          </div>

          {/* Action buttons when items are selected */}
          {selectedIds.length > 0 && (
            <div className="flex items-center gap-2 border-l border-border/50 pl-4 duration-200 animate-in fade-in slide-in-from-left-2">
              <Button
                size="sm"
                onClick={() => onReportBatch(selectedIds)}
                className="h-8 gap-1.5 px-3 text-xs font-medium bg-primary text-primary-foreground hover:bg-primary/90"
              >
                <Check className="size-3" />
                <span>
                  {selectedIds.length === filteredShifts.length
                    ? 'Godkänn och rapportera alla'
                    : `Rapportera valda (${selectedIds.length})`}
                </span>
              </Button>
            </div>
          )}
        </div>

        {/* Right controls: Exact 2 dropdowns (Period, Sortering) + Sök + Ny manuell tid */}
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

          {/* Dropdown 2: Sortering */}
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
              <SelectItem value="team">
                {t('timereports.sort_team', 'Team / Brukare')}
              </SelectItem>
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

          {!isManualOpen && (
            <Button
              size="sm"
              variant="outline"
              onClick={() => setIsManualOpen(true)}
              className="h-8 gap-1.5 px-3 text-xs font-medium border-border/60 bg-background hover:bg-secondary shrink-0"
            >
              <Plus className="size-3" />
              <span>{t('timereports.add_manual_shift', '+ Ny manuell tid')}</span>
            </Button>
          )}
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
              {t('timereports.no_unreported_shifts', 'Inga osparade pass')}
            </p>
            <p className="mt-1 max-w-[320px] text-xs text-muted-foreground/70">
              {t(
                'timereports.no_unreported_shifts_desc',
                'Alla dina schemalagda pass är redan rapporterade.'
              )}
            </p>
            {!isManualOpen && (
              <Button
                variant="outline"
                size="sm"
                onClick={() => setIsManualOpen(true)}
                className="mt-4 h-8 gap-1.5 px-3 text-xs font-medium border-border/60"
              >
                <Plus className="size-3" />
                <span>{t('timereports.add_manual_shift', '+ Ny manuell tid')}</span>
              </Button>
            )}
          </div>
        ) : (
          <div className="flex w-full flex-col text-sm">
            {filteredShifts.map((shift, idx) => {
              const isSelected = selectedIds.includes(shift.id)
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

                  {/* Target Column (Team / Brukare) - Strict Single Line */}
                  <div className="w-36 sm:w-48 md:w-56 shrink-0 truncate pr-3 text-sm font-medium text-foreground transition-colors group-hover:text-primary">
                    {shift.team}
                  </div>

                  {/* Middle Column - Details / Note / Hours - Strict Single Line */}
                  <div className="flex min-w-0 flex-1 items-center gap-x-4 truncate pr-6 text-sm font-light">
                    <span className="truncate text-xs italic text-muted-foreground/60">
                      {shift.note || 'Schemalagt arbetspass'}
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

                  {/* Status, Date & Quick Report Action - Strict Single Line */}
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
                        onReportSingle(shift.id)
                      }}
                      className="h-7 gap-1 px-2.5 text-[11px] font-medium hover:bg-primary hover:text-primary-foreground transition-colors shadow-2xs shrink-0"
                    >
                      <Check className="size-3" />
                      <span>Rapportera</span>
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
