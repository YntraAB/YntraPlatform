import React, { useState, useMemo, useEffect } from 'react'
import { useTranslation } from 'react-i18next'
import {
  Clock,
  Check,
  History,
  Plus,
  Inbox,
} from 'lucide-react'
import {
  Dialog,
  DialogContent,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog'
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
import { toast } from 'sonner'
import { supabase } from '@/lib/supabase'
import { useWorkspace } from '@/contexts/WorkspaceContext'
import { useAuth } from '@/hooks/useAuth'

interface ReportTimeDialogProps {
  open: boolean
  onOpenChange: (open: boolean) => void
  onSuccess?: () => void
}

interface TeamItem {
  id: string
  name: string
}

interface ShiftItem {
  id: string
  eventId?: string
  date: string
  start: string
  end: string
  duration: number
  teamId: string | null
  teamName: string
  note: string
  status: 'not_submitted' | 'pending_attest' | 'approved'
}

type TabType = 'unreported' | 'manual' | 'history'
type SortOption = 'date_desc' | 'date_asc' | 'hours_desc' | 'hours_asc' | 'team'

export const ReportTimeDialog: React.FC<ReportTimeDialogProps> = ({
  open,
  onOpenChange,
  onSuccess,
}) => {
  const { t } = useTranslation()
  const { workspaceId } = useWorkspace()
  const { user } = useAuth()

  const [activeTab, setActiveTab] = useState<TabType>('unreported')
  const [loading, setLoading] = useState(false)
  const [teams, setTeams] = useState<TeamItem[]>([])
  const [unreportedShifts, setUnreportedShifts] = useState<ShiftItem[]>([])
  const [reportedHistory, setReportedHistory] = useState<ShiftItem[]>([])
  const [selectedIds, setSelectedIds] = useState<string[]>([])
  const [sortBy, setSortBy] = useState<SortOption>('date_desc')

  // Manual entry form state
  const [manualTeamId, setManualTeamId] = useState<string>('')
  const [manualDate, setManualDate] = useState<string>(
    new Date().toLocaleDateString('sv-SE')
  )
  const [manualStart, setManualStart] = useState<string>('08:00')
  const [manualEnd, setManualEnd] = useState<string>('16:00')
  const [manualBreak, setManualBreak] = useState<string>('0')
  const [manualNote, setManualNote] = useState<string>('')
  const [submittingManual, setSubmittingManual] = useState(false)

  const normalizeDate = (dStr: string) => {
    if (!dStr) return ''
    try {
      const d = new Date(dStr)
      if (isNaN(d.getTime())) return dStr.split('T')[0]
      return d.toLocaleDateString('sv-SE')
    } catch {
      return dStr.split('T')[0]
    }
  }

  const normalizeTime = (tStr: string) => {
    if (!tStr) return '00:00'
    if (tStr.includes('T')) {
      try {
        const d = new Date(tStr)
        if (!isNaN(d.getTime())) {
          return d.toLocaleTimeString('sv-SE', { hour: '2-digit', minute: '2-digit' })
        }
      } catch {
        return tStr.substring(11, 16)
      }
    }
    return tStr.substring(0, 5)
  }

  // Load shifts and teams
  const loadData = async () => {
    if (!workspaceId || !user?.id) return
    setLoading(true)

    try {
      // 1. Teams
      const { data: dbTeams } = await supabase
        .from('teams')
        .select('id, name')
        .eq('workspace_id', workspaceId)
      const loadedTeams: TeamItem[] = dbTeams || []
      setTeams(loadedTeams)
      if (loadedTeams.length > 0 && !manualTeamId) {
        setManualTeamId(loadedTeams[0].id)
      }

      // 2. Existing time reports for current user
      const { data: dbReports } = await supabase
        .from('time_reports')
        .select('*')
        .eq('workspace_id', workspaceId)
        .eq('user_id', user.id)
        .order('date', { ascending: false })

      const rawReports = dbReports || []

      // Map history reports
      const mappedHistory: ShiftItem[] = rawReports.map((r: any) => {
        const teamObj = loadedTeams.find((item) => item.id === r.team_id)
        return {
          id: r.id,
          date: normalizeDate(r.date),
          start: normalizeTime(r.start_time || '08:00'),
          end: normalizeTime(r.end_time || '16:00'),
          duration: r.hours || 0,
          teamId: r.team_id,
          teamName: teamObj ? teamObj.name : t('timereports.unassigned_team', 'Odelat team'),
          note: r.note || '',
          status: (r.status as ShiftItem['status']) || 'pending_attest',
        }
      })
      setReportedHistory(mappedHistory)

      // Set of reported slots
      const reportedKeys = new Set(
        rawReports.map(
          (r: any) =>
            `${r.user_id}_${normalizeDate(r.date)}_${normalizeTime(r.start_time)}`
        )
      )

      // 3. Past completed events for current user
      const nowIso = new Date().toISOString()
      const { data: dbEvents } = await supabase
        .from('events')
        .select('*')
        .eq('workspace_id', workspaceId)
        .lt('end_time', nowIso)
        .order('start_time', { ascending: false })
        .limit(100)

      const rawEvents = dbEvents || []
      const userPastEvents = rawEvents.filter(
        (e: any) => e.assignee_id === user.id || (!e.assignee_id && e.user_id === user.id)
      )

      const unsubmitted: ShiftItem[] = []
      for (const e of userPastEvents) {
        const d = normalizeDate(e.start_time)
        const st = normalizeTime(e.start_time)
        const et = normalizeTime(e.end_time)
        const key = `${user.id}_${d}_${st}`

        if (!reportedKeys.has(key)) {
          const startObj = new Date(e.start_time)
          const endObj = new Date(e.end_time)
          let diffMs = endObj.getTime() - startObj.getTime()
          if (diffMs < 0) diffMs += 24 * 60 * 60 * 1000
          const hours = Math.round((diffMs / (1000 * 60 * 60)) * 100) / 100

          const teamObj = loadedTeams.find((item) => item.id === e.team_id)

          unsubmitted.push({
            id: e.id,
            eventId: e.id,
            date: d,
            start: st,
            end: et,
            duration: hours,
            teamId: e.team_id || null,
            teamName: teamObj ? teamObj.name : t('timereports.unassigned_team', 'Odelat team'),
            note: e.title || '',
            status: 'not_submitted',
          })
        }
      }

      setUnreportedShifts(unsubmitted)
      // By default select all unsubmitted shifts
      setSelectedIds(unsubmitted.map((s) => s.id))
    } catch (err) {
      console.error('Error loading time dialog data:', err)
    } finally {
      setLoading(false)
    }
  }

  useEffect(() => {
    if (open) {
      loadData()
    }
  }, [open, workspaceId, user?.id])

  // Sorting
  const sortedUnreported = useMemo(() => {
    return [...unreportedShifts].sort((a, b) => {
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
          return a.teamName.localeCompare(b.teamName)
        default:
          return b.date.localeCompare(a.date)
      }
    })
  }, [unreportedShifts, sortBy])

  // Selected totals
  const selectedShifts = useMemo(() => {
    return sortedUnreported.filter((s) => selectedIds.includes(s.id))
  }, [sortedUnreported, selectedIds])

  const totalSelectedHours = useMemo(() => {
    return Math.round(selectedShifts.reduce((acc, s) => acc + s.duration, 0) * 100) / 100
  }, [selectedShifts])

  const isAllSelected =
    sortedUnreported.length > 0 && selectedIds.length === sortedUnreported.length

  const toggleSelectAll = () => {
    if (isAllSelected) {
      setSelectedIds([])
    } else {
      setSelectedIds(sortedUnreported.map((s) => s.id))
    }
  }

  const toggleSelectOne = (id: string) => {
    setSelectedIds((prev) =>
      prev.includes(id) ? prev.filter((item) => item !== id) : [...prev, id]
    )
  }

  // Submit batch or single shifts
  const submitShifts = async (shiftsToSubmit: ShiftItem[]) => {
    if (shiftsToSubmit.length === 0 || !workspaceId || !user?.id) return

    setLoading(true)
    try {
      const inserts = shiftsToSubmit.map((s) => ({
        workspace_id: workspaceId,
        user_id: user.id,
        team_id: s.teamId || null,
        date: s.date,
        start_time: s.start,
        end_time: s.end,
        hours: s.duration,
        status: 'pending_attest',
        note: s.note ? `Schemalagt pass: ${s.note}` : 'Schemalagt pass',
      }))

      const { error } = await supabase.from('time_reports').insert(inserts)
      if (error) throw error

      toast.success(t('timereports.success_reported', 'Tidsrapporten har skickats in för attest!'))
      await loadData()
      if (onSuccess) onSuccess()
    } catch (err: any) {
      console.error('Error submitting shifts:', err)
      toast.error(
        t('timereports.error_reporting', 'Kunde inte spara tidsrapporten') +
          ': ' +
          (err?.message || '')
      )
    } finally {
      setLoading(false)
    }
  }

  // Calculate live manual hours
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

  // Submit manual shift
  const handleSaveManual = async (e: React.FormEvent) => {
    e.preventDefault()
    if (!workspaceId || !user?.id) return

    if (calculatedManualHours <= 0) {
      toast.error('Arbetstiden måste vara större än 0 timmar.')
      return
    }

    setSubmittingManual(true)
    try {
      const insertData = {
        workspace_id: workspaceId,
        user_id: user.id,
        team_id: manualTeamId || null,
        date: manualDate,
        start_time: manualStart,
        end_time: manualEnd,
        hours: calculatedManualHours,
        status: 'pending_attest',
        note: manualNote ? `Manuell registrering: ${manualNote}` : 'Manuell registrering',
      }

      const { error } = await supabase.from('time_reports').insert([insertData])
      if (error) throw error

      toast.success(t('timereports.success_reported', 'Tidsrapporten har skickats in för attest!'))
      setManualNote('')
      await loadData()
      setActiveTab('unreported')
      if (onSuccess) onSuccess()
    } catch (err: any) {
      console.error('Error saving manual report:', err)
      toast.error(
        t('timereports.error_reporting', 'Kunde inte spara tidsrapporten') +
          ': ' +
          (err?.message || '')
      )
    } finally {
      setSubmittingManual(false)
    }
  }

  // History totals
  const historyStats = useMemo(() => {
    const total = reportedHistory.reduce((acc, r) => acc + r.duration, 0)
    const approved = reportedHistory
      .filter((r) => r.status === 'approved')
      .reduce((acc, r) => acc + r.duration, 0)
    const pending = reportedHistory
      .filter((r) => r.status === 'pending_attest')
      .reduce((acc, r) => acc + r.duration, 0)

    return {
      total: Math.round(total * 10) / 10,
      approved: Math.round(approved * 10) / 10,
      pending: Math.round(pending * 10) / 10,
    }
  }, [reportedHistory])

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="sm:max-w-[620px] max-h-[85vh] flex flex-col overflow-hidden rounded-lg border border-border bg-card p-0 shadow-2xl gap-0">
        {/* Header Standard h-12 */}
        <DialogHeader className="flex h-12 shrink-0 flex-row items-center justify-between border-b border-border bg-secondary/40 px-5 py-0 space-y-0">
          <DialogTitle className="flex items-center gap-2 text-xs font-medium text-foreground">
            <Clock className="size-3.5 text-muted-foreground/70 shrink-0" />
            <span>{t('timereports.report_modal_title', 'Rapportera tid')}</span>
          </DialogTitle>

          {/* Subheader tabs */}
          <div className="flex items-center gap-1 border border-border/50 bg-background/60 p-0.5 rounded-md">
            <button
              type="button"
              onClick={() => setActiveTab('unreported')}
              className={`flex items-center gap-1.5 px-2.5 py-1 text-[11px] font-medium rounded transition-colors ${
                activeTab === 'unreported'
                  ? 'bg-secondary text-foreground shadow-xs'
                  : 'text-muted-foreground hover:text-foreground'
              }`}
            >
              <Clock className="size-3" />
              <span>{t('timereports.worked_hours', 'Arbetade timmar')}</span>
              {unreportedShifts.length > 0 && (
                <span className="ml-0.5 rounded-full bg-primary/20 text-primary px-1.5 py-0.2 text-[10px] tabular-nums font-mono font-medium">
                  {unreportedShifts.length}
                </span>
              )}
            </button>

            <button
              type="button"
              onClick={() => setActiveTab('manual')}
              className={`flex items-center gap-1 px-2.5 py-1 text-[11px] font-medium rounded transition-colors ${
                activeTab === 'manual'
                  ? 'bg-secondary text-foreground shadow-xs'
                  : 'text-muted-foreground hover:text-foreground'
              }`}
            >
              <Plus className="size-3" />
              <span>{t('timereports.manual_entry', 'Manuell tid')}</span>
            </button>

            <button
              type="button"
              onClick={() => setActiveTab('history')}
              className={`flex items-center gap-1 px-2.5 py-1 text-[11px] font-medium rounded transition-colors ${
                activeTab === 'history'
                  ? 'bg-secondary text-foreground shadow-xs'
                  : 'text-muted-foreground hover:text-foreground'
              }`}
            >
              <History className="size-3" />
              <span>{t('timereports.my_history', 'Historik')}</span>
            </button>
          </div>
        </DialogHeader>

        {/* Tab 1: Unreported Shifts */}
        {activeTab === 'unreported' && (
          <div className="flex flex-1 flex-col overflow-hidden min-h-0">
            {/* Controls Bar */}
            <div className="flex h-10 shrink-0 items-center justify-between border-b border-border/50 bg-background/50 px-5">
              <div className="flex items-center gap-3">
                <Checkbox
                  checked={isAllSelected}
                  onCheckedChange={toggleSelectAll}
                  aria-label={t('common.select_all', 'Markera alla')}
                  className="h-3.5 w-3.5"
                />
                <span className="text-xs text-muted-foreground">
                  {selectedIds.length > 0
                    ? `${selectedIds.length} av ${sortedUnreported.length} valda`
                    : `${sortedUnreported.length} pass att rapportera`}
                </span>
              </div>

              <div className="flex items-center gap-2">
                <span className="text-[11px] text-muted-foreground/70 hidden sm:inline">
                  {t('timereports.sort_by', 'Sortering')}:
                </span>
                <Select
                  value={sortBy}
                  onValueChange={(val) => setSortBy(val as SortOption)}
                >
                  <SelectTrigger className="h-7 w-[150px] border-border/50 bg-background text-[11px] font-medium text-foreground">
                    <SelectValue />
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
              </div>
            </div>

            {/* List of Shifts (h-12 Standard Rows) */}
            <div className="scrollbar-dark flex-1 overflow-y-auto min-h-[240px] max-h-[380px]">
              {sortedUnreported.length === 0 ? (
                <div className="flex h-56 flex-col items-center justify-center p-6 text-center text-muted-foreground">
                  <div className="mb-3 flex size-12 items-center justify-center rounded-md border border-border/40 bg-muted/20">
                    <Inbox className="size-6 text-muted-foreground/40" />
                  </div>
                  <p className="text-xs font-medium text-foreground">
                    {t('timereports.no_unreported_shifts', 'Inga osparade pass')}
                  </p>
                  <p className="mt-1 max-w-[280px] text-[11px] text-muted-foreground/70">
                    {t(
                      'timereports.no_unreported_shifts_desc',
                      'Alla dina schemalagda pass är redan rapporterade.'
                    )}
                  </p>
                  <Button
                    variant="outline"
                    size="sm"
                    onClick={() => setActiveTab('manual')}
                    className="mt-4 h-7 gap-1.5 px-3 text-xs"
                  >
                    <Plus className="size-3" />
                    <span>{t('timereports.add_manual_shift', 'Lägg till manuell tid')}</span>
                  </Button>
                </div>
              ) : (
                <div className="flex flex-col">
                  {sortedUnreported.map((shift) => {
                    const isSelected = selectedIds.includes(shift.id)
                    return (
                      <div
                        key={shift.id}
                        onClick={() => toggleSelectOne(shift.id)}
                        className={`group flex h-12 cursor-pointer items-center justify-between border-b border-border/40 px-5 transition-colors hover:bg-secondary/40 ${
                          isSelected ? 'bg-primary/5' : ''
                        }`}
                      >
                        {/* Checkbox and Target Info */}
                        <div className="flex items-center gap-3 min-w-0 flex-1">
                          <Checkbox
                            checked={isSelected}
                            onCheckedChange={() => toggleSelectOne(shift.id)}
                            onClick={(e) => e.stopPropagation()}
                            className="h-3.5 w-3.5"
                          />
                          <div className="min-w-0 flex-1 truncate">
                            <div className="flex items-center gap-2">
                              <span className="truncate text-xs font-medium text-foreground">
                                {shift.teamName}
                              </span>
                              {shift.note && (
                                <span className="truncate text-[11px] text-muted-foreground/60">
                                  · {shift.note}
                                </span>
                              )}
                            </div>
                            <div className="flex items-center gap-2 text-[11px] text-muted-foreground font-mono">
                              <span>{shift.date}</span>
                              <span className="opacity-40">·</span>
                              <span>
                                {shift.start} - {shift.end}
                              </span>
                            </div>
                          </div>
                        </div>

                        {/* Hours & Quick Report Action */}
                        <div className="flex items-center gap-3 shrink-0 ml-3">
                          <span className="font-mono text-xs font-medium tabular-nums text-foreground/90">
                            {shift.duration}h
                          </span>

                          <Button
                            size="sm"
                            variant="secondary"
                            onClick={(e) => {
                              e.stopPropagation()
                              submitShifts([shift])
                            }}
                            className="h-7 gap-1 rounded-md px-2.5 text-[11px] font-medium hover:bg-primary hover:text-primary-foreground transition-colors"
                          >
                            <Check className="size-3" />
                            <span>{t('timereports.report_direct', 'Rapportera')}</span>
                          </Button>
                        </div>
                      </div>
                    )
                  })}
                </div>
              )}
            </div>

            {/* Footer standard shelf */}
            <div className="flex h-12 shrink-0 items-center justify-between border-t border-border bg-secondary/40 px-5">
              <div className="text-xs text-muted-foreground">
                {selectedShifts.length > 0 && (
                  <span className="font-mono">
                    <strong className="text-foreground">{selectedShifts.length}</strong> valda (
                    <strong className="text-foreground">{totalSelectedHours}h</strong>)
                  </span>
                )}
              </div>

              <div className="flex items-center gap-2">
                <Button
                  variant="outline"
                  size="sm"
                  onClick={() => onOpenChange(false)}
                  className="h-8 rounded-md border border-border bg-background px-3 text-xs font-medium text-foreground hover:bg-secondary"
                >
                  {t('common.close', 'Stäng')}
                </Button>

                {sortedUnreported.length > 0 && (
                  <Button
                    size="sm"
                    disabled={selectedShifts.length === 0 || loading}
                    onClick={() => submitShifts(selectedShifts)}
                    className="h-8 rounded-md bg-primary px-3 text-xs font-medium text-primary-foreground shadow-xs hover:bg-primary/90"
                  >
                    <Check className="mr-1.5 size-3" />
                    <span>
                      {selectedShifts.length === sortedUnreported.length
                        ? t('timereports.report_all_direct', 'Godkänn och rapportera alla')
                        : `${t('timereports.report_selected', 'Rapportera valda')} (${selectedShifts.length})`}
                    </span>
                  </Button>
                )}
              </div>
            </div>
          </div>
        )}

        {/* Tab 2: Manual Time Reporting */}
        {activeTab === 'manual' && (
          <form onSubmit={handleSaveManual} className="flex flex-1 flex-col overflow-hidden min-h-0">
            <div className="flex flex-1 flex-col gap-3.5 p-5 overflow-y-auto">
              <div className="flex flex-col gap-1.5">
                <label className="text-xs font-medium text-muted-foreground">
                  {t('timereports.client_or_team', 'Brukare / Team')}
                </label>
                <select
                  value={manualTeamId}
                  onChange={(e) => setManualTeamId(e.target.value)}
                  className="h-8 w-full rounded-md border border-border bg-background px-2.5 text-xs text-foreground outline-none transition-colors focus:border-border focus:ring-1 focus:ring-ring"
                >
                  {teams.map((team) => (
                    <option key={team.id} value={team.id}>
                      {team.name}
                    </option>
                  ))}
                  {teams.length === 0 && (
                    <option value="">{t('timereports.unassigned_team', 'Odelat team')}</option>
                  )}
                </select>
              </div>

              <div className="flex flex-col gap-1.5">
                <label className="text-xs font-medium text-muted-foreground">
                  {t('timereports.date_label', 'Datum')}
                </label>
                <Input
                  type="date"
                  value={manualDate}
                  onChange={(e) => setManualDate(e.target.value)}
                  required
                  className="h-8 rounded-md border border-border bg-background px-2.5 text-xs text-foreground"
                />
              </div>

              <div className="grid grid-cols-2 gap-3">
                <div className="flex flex-col gap-1.5">
                  <label className="text-xs font-medium text-muted-foreground">
                    {t('timereports.start_time_label', 'Starttid')}
                  </label>
                  <Input
                    type="time"
                    value={manualStart}
                    onChange={(e) => setManualStart(e.target.value)}
                    required
                    className="h-8 rounded-md border border-border bg-background px-2.5 text-xs text-foreground"
                  />
                </div>
                <div className="flex flex-col gap-1.5">
                  <label className="text-xs font-medium text-muted-foreground">
                    {t('timereports.end_time_label', 'Sluttid')}
                  </label>
                  <Input
                    type="time"
                    value={manualEnd}
                    onChange={(e) => setManualEnd(e.target.value)}
                    required
                    className="h-8 rounded-md border border-border bg-background px-2.5 text-xs text-foreground"
                  />
                </div>
              </div>

              <div className="grid grid-cols-2 gap-3 items-center">
                <div className="flex flex-col gap-1.5">
                  <label className="text-xs font-medium text-muted-foreground">
                    {t('timereports.break_minutes', 'Rast (minuter)')}
                  </label>
                  <Input
                    type="number"
                    min="0"
                    step="5"
                    value={manualBreak}
                    onChange={(e) => setManualBreak(e.target.value)}
                    className="h-8 rounded-md border border-border bg-background px-2.5 text-xs text-foreground"
                  />
                </div>
                <div className="flex flex-col gap-1.5">
                  <span className="text-xs font-medium text-muted-foreground">
                    {t('timereports.calculated_hours', 'Beräknade timmar')}
                  </span>
                  <div className="flex h-8 items-center rounded-md border border-border/50 bg-muted/40 px-2.5 font-mono text-xs font-medium text-foreground">
                    {calculatedManualHours}h
                  </div>
                </div>
              </div>

              <div className="flex flex-col gap-1.5">
                <label className="text-xs font-medium text-muted-foreground">
                  {t('timereports.note_label', 'Kommentar (valfritt)')}
                </label>
                <Input
                  type="text"
                  value={manualNote}
                  onChange={(e) => setManualNote(e.target.value)}
                  placeholder={t(
                    'timereports.note_placeholder',
                    'T.ex. extrapass, förlängning...'
                  )}
                  className="h-8 rounded-md border border-border bg-background px-2.5 text-xs text-foreground"
                />
              </div>
            </div>

            {/* Footer */}
            <div className="flex h-12 shrink-0 items-center justify-end gap-2 border-t border-border bg-secondary/40 px-5">
              <Button
                type="button"
                variant="outline"
                size="sm"
                onClick={() => setActiveTab('unreported')}
                className="h-8 rounded-md border border-border bg-background px-3 text-xs font-medium text-foreground hover:bg-secondary"
              >
                {t('common.cancel', 'Avbryt')}
              </Button>
              <Button
                type="submit"
                size="sm"
                disabled={submittingManual || calculatedManualHours <= 0}
                className="h-8 rounded-md bg-primary px-3 text-xs font-medium text-primary-foreground shadow-xs hover:bg-primary/90"
              >
                <Check className="mr-1.5 size-3" />
                <span>{t('timereports.save_report', 'Skicka in rapport')}</span>
              </Button>
            </div>
          </form>
        )}

        {/* Tab 3: History */}
        {activeTab === 'history' && (
          <div className="flex flex-1 flex-col overflow-hidden min-h-0">
            {/* Quick summary strip */}
            <div className="grid grid-cols-3 border-b border-border/50 bg-secondary/20 divide-x divide-border/40 py-2.5 px-5">
              <div className="flex flex-col">
                <span className="text-[10px] text-muted-foreground uppercase tracking-wider">
                  {t('timereports.total_reported_hours', 'Totalt')}
                </span>
                <span className="font-mono text-sm font-medium text-foreground">
                  {historyStats.total}h
                </span>
              </div>
              <div className="flex flex-col pl-4">
                <span className="flex items-center gap-1 text-[10px] text-muted-foreground uppercase tracking-wider">
                  <span className="size-1.5 rounded-full bg-emerald-500" />
                  {t('timereports.approved_hours', 'Attesterat')}
                </span>
                <span className="font-mono text-sm font-medium text-foreground">
                  {historyStats.approved}h
                </span>
              </div>
              <div className="flex flex-col pl-4">
                <span className="flex items-center gap-1 text-[10px] text-muted-foreground uppercase tracking-wider">
                  <span className="size-1.5 rounded-full bg-amber-500" />
                  {t('timereports.pending_attest_hours', 'Väntar')}
                </span>
                <span className="font-mono text-sm font-medium text-foreground">
                  {historyStats.pending}h
                </span>
              </div>
            </div>

            {/* History shift list */}
            <div className="scrollbar-dark flex-1 overflow-y-auto min-h-[220px] max-h-[360px]">
              {reportedHistory.length === 0 ? (
                <div className="flex h-48 flex-col items-center justify-center p-6 text-center text-muted-foreground">
                  <Inbox className="size-6 text-muted-foreground/40 mb-2" />
                  <p className="text-xs font-medium text-foreground">
                    {t('timereports.no_history_title', 'Ingen historik')}
                  </p>
                  <p className="mt-1 text-[11px] text-muted-foreground/70">
                    {t(
                      'timereports.no_history_desc',
                      'Det finns inga tidigare rapporter för detta urval.'
                    )}
                  </p>
                </div>
              ) : (
                <div className="flex flex-col">
                  {reportedHistory.map((report) => (
                    <div
                      key={report.id}
                      className="flex h-12 items-center justify-between border-b border-border/40 px-5 text-xs hover:bg-secondary/20 transition-colors"
                    >
                      <div className="min-w-0 flex-1 truncate pr-3">
                        <div className="flex items-center gap-2">
                          <span className="truncate font-medium text-foreground">
                            {report.teamName}
                          </span>
                          {report.note && (
                            <span className="truncate text-[11px] text-muted-foreground/60 italic">
                              · {report.note}
                            </span>
                          )}
                        </div>
                        <div className="flex items-center gap-2 text-[11px] text-muted-foreground font-mono">
                          <span>{report.date}</span>
                          <span className="opacity-40">·</span>
                          <span>
                            {report.start} - {report.end}
                          </span>
                        </div>
                      </div>

                      <div className="flex items-center gap-4 shrink-0">
                        <span className="font-mono text-xs font-medium tabular-nums text-foreground">
                          {report.duration}h
                        </span>

                        <div className="flex items-center gap-1.5 w-24 justify-end">
                          <span
                            className={`size-1.5 rounded-full ${
                              report.status === 'approved' ? 'bg-emerald-500' : 'bg-amber-500'
                            }`}
                          />
                          <span className="text-[11px] text-muted-foreground/80 font-medium">
                            {report.status === 'approved'
                              ? t('timereports.status.approved', 'Godkänd')
                              : t('timereports.status.pending_attest', 'Väntar')}
                          </span>
                        </div>
                      </div>
                    </div>
                  ))}
                </div>
              )}
            </div>

            {/* Footer */}
            <div className="flex h-12 shrink-0 items-center justify-end border-t border-border bg-secondary/40 px-5">
              <Button
                variant="outline"
                size="sm"
                onClick={() => onOpenChange(false)}
                className="h-8 rounded-md border border-border bg-background px-3 text-xs font-medium text-foreground hover:bg-secondary"
              >
                {t('common.close', 'Stäng')}
              </Button>
            </div>
          </div>
        )}
      </DialogContent>
    </Dialog>
  )
}

export default ReportTimeDialog

