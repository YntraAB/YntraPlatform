import React, { useState, useEffect } from 'react'
import { useTranslation } from 'react-i18next'
import { useWorkspace } from '@/contexts/WorkspaceContext'
import { useWorkspaceTeams, useWorkspaceUsers } from '@/hooks/queries/useWorkspaceData'
import { supabase } from '@/lib/supabase'
import { formatDateInput, parseLocalDate } from '@/lib/utils'
import type { CalendarEvent, EventCategory } from '@/types'
import {
  Dialog,
  DialogContent,
  DialogHeader,
  DialogTitle,
  DialogFooter,
} from '@/components/ui/dialog'
import { Button } from '@/components/ui/button'
import { Input } from '@/components/ui/input'
import { Collapsible, CollapsibleContent, CollapsibleTrigger } from '@/components/ui/collapsible'
import { Checkbox } from '@/components/ui/checkbox'
import { Label } from '@/components/ui/label'
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select'
import { ChevronDown, Calendar, Check } from 'lucide-react'

interface AddEventModalProps {
  selectedDate: Date
  event?: CalendarEvent | null
  onClose: () => void
  onSave: (event: Omit<CalendarEvent, 'id'>, eventId?: string) => Promise<void>
}

export const AddEventModal: React.FC<AddEventModalProps> = ({
  selectedDate,
  event,
  onClose,
  onSave,
}) => {
  const { workspaceId } = useWorkspace()
  const { t } = useTranslation()
  const { data: dbTeams = [] } = useWorkspaceTeams(workspaceId)
  const { data: dbUsers = [] } = useWorkspaceUsers(workspaceId)

  const [teamId, setTeamId] = useState(event?.teamId || '')
  const [assigneeId, setAssigneeId] = useState(event?.assigneeId || '')
  const [startDate, setStartDate] = useState(
    formatDateInput(event?.startTime || selectedDate),
  )
  const [endDate, setEndDate] = useState(
    formatDateInput(event?.endTime || selectedDate),
  )
  const [startTime, setStartTime] = useState(
    event?.startTime ? event.startTime.toTimeString().slice(0, 5) : '09:00',
  )
  const [endTime, setEndTime] = useState(
    event?.endTime ? event.endTime.toTimeString().slice(0, 5) : '10:00',
  )
  const [category, setCategory] = useState<EventCategory>(event?.category || 'assistance_time')
  const [description, setDescription] = useState(event?.description || '')
  const [clientId, setClientId] = useState(event?.clientId || 'none')
  const [clients, setClients] = useState<{ id: string; name: string }[]>([])
  const [isSubmitting, setIsSubmitting] = useState(false)
  const [waitingFrom, setWaitingFrom] = useState(event?.waitingTime?.from || '')
  const [waitingTo, setWaitingTo] = useState(event?.waitingTime?.to || '')
  const [active1From, setActive1From] = useState(event?.activeTimes?.[0]?.from || '')
  const [active1To, setActive1To] = useState(event?.activeTimes?.[0]?.to || '')
  const [active2From, setActive2From] = useState(event?.activeTimes?.[1]?.from || '')
  const [active2To, setActive2To] = useState(event?.activeTimes?.[1]?.to || '')
  const [active3From, setActive3From] = useState(event?.activeTimes?.[2]?.from || '')
  const [active3To, setActive3To] = useState(event?.activeTimes?.[2]?.to || '')
  const [breakFrom, setBreakFrom] = useState(event?.break?.from || '')
  const [breakTo, setBreakTo] = useState(event?.break?.to || '')
  const [isBreakPaid, setIsBreakPaid] = useState(event?.break?.isPaid || false)

  useEffect(() => {
    if (dbTeams.length === 1) {
      setTeamId(dbTeams[0].id)
    }
  }, [dbTeams])

  useEffect(() => {
    if (workspaceId) {
      supabase
        .from('clients')
        .select('id, first_name, last_name')
        .eq('workspace_id', workspaceId)
        .then(({ data }) => {
          if (data)
            setClients(data.map((c) => ({ id: c.id, name: `${c.first_name} ${c.last_name}` })))
        })
    }
  }, [workspaceId])

  const handleStartDateChange = (newStartDate: string) => {
    if (startDate === endDate) {
      setEndDate(newStartDate)
    }
    setStartDate(newStartDate)
  }

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault()
    if (!teamId || !assigneeId) return

    setIsSubmitting(true)
    try {
      const start = parseLocalDate(startDate, startTime)
      const end = parseLocalDate(endDate, endTime)

      const eventTitle = t(`scheduler.categories.${category}`, { defaultValue: category })

      await onSave(
        {
          title: eventTitle,
          teamId,
          assigneeId,
          clientId: clientId === 'none' ? undefined : clientId,
          startTime: start,
          endTime: end,
          category,
          description,
          waitingTime: waitingFrom || waitingTo ? { from: waitingFrom, to: waitingTo } : undefined,
          activeTimes: [
            { from: active1From, to: active1To },
            { from: active2From, to: active2To },
            { from: active3From, to: active3To },
          ].filter((t) => t.from || t.to),
          break:
            breakFrom || breakTo
              ? { from: breakFrom, to: breakTo, isPaid: isBreakPaid }
              : undefined,
        },
        event?.id,
      )
      onClose()
    } catch (error) {
      console.error('Failed to save event:', error)
    } finally {
      setIsSubmitting(false)
    }
  }

  return (
    <Dialog open={true} onOpenChange={(open) => !open && onClose()}>
      <DialogContent className="overflow-hidden rounded-lg border border-border bg-card p-0 shadow-2xl sm:max-w-lg">
        <DialogHeader className="flex h-12 flex-row items-center justify-between border-b border-border bg-secondary/40 px-5">
          <DialogTitle className="flex items-center gap-2 text-xs font-medium text-foreground">
            <Calendar className="size-3.5 text-muted-foreground/70 shrink-0" />
            <span>
              {event
                ? t('scheduler.edit_event')
                : t('scheduler.new_event')}
            </span>
          </DialogTitle>
        </DialogHeader>

        <form
          id="add-event-form"
          onSubmit={handleSubmit}
          className="scrollbar-dark max-h-[72vh] space-y-4 overflow-y-auto p-5"
        >
          {/* Category moved to top */}
          <div className="space-y-1.5">
            <label className="text-[11px] font-medium text-muted-foreground">
              {t('common.category')}
            </label>
            <Select value={category} onValueChange={(val) => setCategory(val as EventCategory)}>
              <SelectTrigger className="h-8 w-full rounded-md border border-border bg-background px-2.5 text-xs text-foreground">
                <SelectValue />
              </SelectTrigger>
              <SelectContent className="rounded-md border border-border bg-popover text-xs shadow-lg">
                {(
                  [
                    'assistance_time',
                    'on_call',
                    'travel_time',
                    'introduction',
                    'meeting',
                    'administrative_hours',
                    'training',
                    'escort_service',
                    'respite_care',
                    'unauthorized_absence',
                    'involuntary_leave',
                    'other_time',
                    'customer_staff_note',
                    'severance_pay',
                    'other',
                  ] as const
                ).map((cat) => (
                  <SelectItem key={cat} value={cat} className="text-xs">
                    {t(`scheduler.categories.${cat}`, { defaultValue: cat })}
                  </SelectItem>
                ))}
              </SelectContent>
            </Select>

            <div className="flex flex-wrap gap-1.5 pt-0.5">
              {(
                ['assistance_time', 'on_call', 'administrative_hours', 'meeting', 'other'] as const
              ).map((cat) => (
                <button
                  key={cat}
                  type="button"
                  onClick={() => setCategory(cat)}
                  className={`rounded-md px-2.5 py-1 text-[11px] font-medium transition-colors ${
                    category === cat
                      ? 'border border-primary/40 bg-primary/15 text-primary'
                      : 'border border-border bg-secondary/50 text-muted-foreground hover:bg-secondary hover:text-foreground'
                  }`}
                >
                  {t(`scheduler.categories.${cat}`, { defaultValue: cat })}
                </button>
              ))}
            </div>
          </div>

          <div className="grid grid-cols-2 gap-3">
            <div className="space-y-1">
              <label className="text-[11px] font-medium text-muted-foreground">
                {t('common.team')}
              </label>
              <Select value={teamId} onValueChange={setTeamId}>
                <SelectTrigger className="h-8 w-full rounded-md border border-border bg-background px-2.5 text-xs text-foreground">
                  <SelectValue placeholder={t('scheduler.select_team')} />
                </SelectTrigger>
                <SelectContent className="rounded-md border border-border bg-popover text-xs shadow-lg">
                  {dbTeams.map((t) => (
                    <SelectItem key={t.id} value={t.id} className="text-xs">
                      {t.name}
                    </SelectItem>
                  ))}
                </SelectContent>
              </Select>
            </div>
            <div className="space-y-1">
              <label className="text-[11px] font-medium text-muted-foreground">
                {t('scheduler.staff')}
              </label>
              <Select value={assigneeId} onValueChange={setAssigneeId}>
                <SelectTrigger className="h-8 w-full rounded-md border border-border bg-background px-2.5 text-xs text-foreground">
                  <SelectValue placeholder={t('scheduler.select_staff')} />
                </SelectTrigger>
                <SelectContent className="rounded-md border border-border bg-popover text-xs shadow-lg">
                  {dbUsers.map((u) => (
                    <SelectItem key={u.id} value={u.id} className="text-xs">
                      {u.full_name || u.name}
                    </SelectItem>
                  ))}
                </SelectContent>
              </Select>
            </div>
            {clients.length > 0 && (
              <div className="col-span-2 space-y-1">
                <label className="text-[11px] font-medium text-muted-foreground">
                  {t('scheduler.client_optional')}
                </label>
                <Select value={clientId} onValueChange={setClientId}>
                  <SelectTrigger className="h-8 w-full rounded-md border border-border bg-background px-2.5 text-xs text-foreground">
                    <SelectValue placeholder={t('scheduler.no_specific_client')} />
                  </SelectTrigger>
                  <SelectContent className="rounded-md border border-border bg-popover text-xs shadow-lg">
                    <SelectItem value="none" className="text-xs">{t('scheduler.no_specific_client')}</SelectItem>
                    {clients.map((c) => (
                      <SelectItem key={c.id} value={c.id} className="text-xs">
                        {c.name}
                      </SelectItem>
                    ))}
                  </SelectContent>
                </Select>
              </div>
            )}
          </div>

          <div className="grid grid-cols-2 gap-x-3 gap-y-2.5 rounded-md border border-border bg-secondary/30 p-3">
            <div className="space-y-1">
              <label className="text-[10px] font-medium text-muted-foreground">
                {t('common.start_date')}
              </label>
              <Input
                type="date"
                value={startDate}
                onChange={(e) => handleStartDateChange(e.target.value)}
                className="h-8 rounded-md border border-border bg-background px-2.5 text-xs text-foreground"
              />
            </div>
            <div className="space-y-1">
              <label className="text-[10px] font-medium text-muted-foreground">
                {t('common.start_time')}
              </label>
              <Input
                type="time"
                value={startTime}
                onChange={(e) => setStartTime(e.target.value)}
                className="h-8 rounded-md border border-border bg-background px-2.5 text-xs text-foreground"
              />
            </div>
            <div className="space-y-1">
              <label className="text-[10px] font-medium text-muted-foreground">
                {t('common.end_date')}
              </label>
              <Input
                type="date"
                value={endDate}
                onChange={(e) => setEndDate(e.target.value)}
                className="h-8 rounded-md border border-border bg-background px-2.5 text-xs text-foreground"
              />
            </div>
            <div className="space-y-1">
              <label className="text-[10px] font-medium text-muted-foreground">
                {t('common.end_time')}
              </label>
              <Input
                type="time"
                value={endTime}
                onChange={(e) => setEndTime(e.target.value)}
                className="h-8 rounded-md border border-border bg-background px-2.5 text-xs text-foreground"
              />
            </div>
          </div>

          {/* Overlap, waiting & active time section */}
          <Collapsible className="space-y-1.5">
            <CollapsibleTrigger asChild>
              <Button
                type="button"
                variant="ghost"
                className="flex h-8 w-full items-center justify-between rounded-md border border-border bg-secondary/40 px-3 text-xs font-medium text-foreground hover:bg-secondary/70"
              >
                <span>{t('scheduler.overlap_waiting_active')}</span>
                <ChevronDown className="size-3.5 text-muted-foreground transition-transform duration-200" />
              </Button>
            </CollapsibleTrigger>
            <CollapsibleContent className="space-y-3 rounded-md border border-border bg-secondary/20 p-3 pt-2.5">
              <div className="space-y-1.5">
                <label className="text-[10px] font-medium text-muted-foreground">
                  {t('scheduler.waiting_time')}
                </label>
                <div className="grid grid-cols-2 gap-2.5">
                  <div className="flex items-center gap-1.5">
                    <span className="text-[10px] text-muted-foreground">{t('common.from')}</span>
                    <input
                      type="time"
                      value={waitingFrom}
                      onChange={(e) => setWaitingFrom(e.target.value)}
                      className="h-7 w-full rounded-md border border-border bg-background px-2 text-xs text-foreground outline-none focus:border-ring"
                    />
                  </div>
                  <div className="flex items-center gap-1.5">
                    <span className="text-[10px] text-muted-foreground">{t('common.to')}</span>
                    <input
                      type="time"
                      value={waitingTo}
                      onChange={(e) => setWaitingTo(e.target.value)}
                      className="h-7 w-full rounded-md border border-border bg-background px-2 text-xs text-foreground outline-none focus:border-ring"
                    />
                  </div>
                </div>
              </div>

              {[
                {
                  label: t('scheduler.active_time') + ' 1',
                  from: active1From,
                  setFrom: setActive1From,
                  to: active1To,
                  setTo: setActive1To,
                },
                {
                  label: t('scheduler.active_time') + ' 2',
                  from: active2From,
                  setFrom: setActive2From,
                  to: active2To,
                  setTo: setActive2To,
                },
                {
                  label: t('scheduler.active_time') + ' 3',
                  from: active3From,
                  setFrom: setActive3From,
                  to: active3To,
                  setTo: setActive3To,
                },
              ].map((item, idx) => (
                <div key={idx} className="space-y-1.5">
                  <label className="text-[10px] font-medium text-muted-foreground">
                    {item.label}
                  </label>
                  <div className="grid grid-cols-2 gap-2.5">
                    <div className="flex items-center gap-1.5">
                      <span className="text-[10px] text-muted-foreground">{t('common.from')}</span>
                      <input
                        type="time"
                        value={item.from}
                        onChange={(e) => item.setFrom(e.target.value)}
                        className="h-7 w-full rounded-md border border-border bg-background px-2 text-xs text-foreground outline-none focus:border-ring"
                      />
                    </div>
                    <div className="flex items-center gap-1.5">
                      <span className="text-[10px] text-muted-foreground">{t('common.to')}</span>
                      <input
                        type="time"
                        value={item.to}
                        onChange={(e) => item.setTo(e.target.value)}
                        className="h-7 w-full rounded-md border border-border bg-background px-2 text-xs text-foreground outline-none focus:border-ring"
                      />
                    </div>
                  </div>
                </div>
              ))}
            </CollapsibleContent>
          </Collapsible>

          {/* Breaks section */}
          <Collapsible className="space-y-1.5">
            <CollapsibleTrigger asChild>
              <Button
                type="button"
                variant="ghost"
                className="flex h-8 w-full items-center justify-between rounded-md border border-border bg-secondary/40 px-3 text-xs font-medium text-foreground hover:bg-secondary/70"
              >
                <span>{t('scheduler.breaks')}</span>
                <ChevronDown className="size-3.5 text-muted-foreground transition-transform duration-200" />
              </Button>
            </CollapsibleTrigger>
            <CollapsibleContent className="space-y-3 rounded-md border border-border bg-secondary/20 p-3 pt-2.5">
              <div className="space-y-1.5">
                <label className="text-[10px] font-medium text-muted-foreground">
                  {t('scheduler.break')}
                </label>
                <div className="grid grid-cols-2 gap-2.5">
                  <div className="flex items-center gap-1.5">
                    <span className="text-[10px] text-muted-foreground">{t('common.from')}</span>
                    <input
                      type="time"
                      value={breakFrom}
                      onChange={(e) => setBreakFrom(e.target.value)}
                      className="h-7 w-full rounded-md border border-border bg-background px-2 text-xs text-foreground outline-none focus:border-ring"
                    />
                  </div>
                  <div className="flex items-center gap-1.5">
                    <span className="text-[10px] text-muted-foreground">{t('common.to')}</span>
                    <input
                      type="time"
                      value={breakTo}
                      onChange={(e) => setBreakTo(e.target.value)}
                      className="h-7 w-full rounded-md border border-border bg-background px-2 text-xs text-foreground outline-none focus:border-ring"
                    />
                  </div>
                </div>
              </div>
              <div className="flex items-center space-x-2 pt-0.5">
                <Checkbox
                  id="break-paid"
                  checked={isBreakPaid}
                  onCheckedChange={(checked) => setIsBreakPaid(checked === true)}
                  className="rounded"
                />
                <Label
                  htmlFor="break-paid"
                  className="cursor-pointer text-xs font-normal text-muted-foreground"
                >
                  {t('scheduler.paid')}
                </Label>
              </div>
            </CollapsibleContent>
          </Collapsible>

          <div className="space-y-1">
            <label className="text-[11px] font-medium text-muted-foreground">
              {t('common.description')}
            </label>
            <textarea
              value={description}
              onChange={(e) => setDescription(e.target.value)}
              className="min-h-[80px] w-full resize-none rounded-md border border-border bg-background px-2.5 py-2 text-xs text-foreground outline-none focus:border-ring"
              placeholder={t('scheduler.description_placeholder')}
            />
          </div>
        </form>

        <DialogFooter className="flex items-center justify-end gap-2 border-t border-border bg-secondary/40 px-5 py-2.5">
          <Button
            type="button"
            variant="ghost"
            onClick={onClose}
            className="h-8 rounded-md border border-border bg-background px-3 text-xs font-medium text-foreground hover:bg-secondary"
          >
            {t('common.cancel')}
          </Button>
          <Button
            form="add-event-form"
            type="submit"
            disabled={isSubmitting || !teamId || !assigneeId}
            className="h-8 rounded-md bg-primary px-3 text-xs font-medium text-primary-foreground shadow-xs hover:bg-primary/90"
          >
            {isSubmitting ? (
              t('common.saving')
            ) : (
              <>
                <Check className="mr-1.5 size-3" />
                <span>{t('common.save')}</span>
              </>
            )}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  )
}
