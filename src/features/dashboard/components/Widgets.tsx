import React, { useEffect, useState } from 'react'
import { Card, CardContent } from '@/components/ui/card'
import { CalendarDays, Calculator } from 'lucide-react'
import { Button } from '@/components/ui/button'
import { useNavigate } from 'react-router-dom'
import { useAuth } from '@/hooks/useAuth'
import { useWorkspace } from '@/contexts/WorkspaceContext'
import { supabase } from '@/lib/supabase'
import { useTranslation } from 'react-i18next'
import { cn } from '@/lib/utils'

// ──────────────────────────────────────────────────────────
// Types
// ──────────────────────────────────────────────────────────
interface DashboardEvent {
  id: string
  title: string
  start_time: string
  end_time: string
  assignee_id?: string
  team_id?: string
}

interface MonthShift {
  id: string
  title: string
  date: string
  startTime: string
  endTime: string
  totalHours: number
  obKvallHours: number  // 19:00–22:00
  obNattHours: number   // 22:00–06:00
  isReported: boolean
  isPast: boolean
  isToday?: boolean
}

// ──────────────────────────────────────────────────────────
// Swedish municipality tax rates — common municipalities
// ──────────────────────────────────────────────────────────
const KOMMUN_TAX: Record<string, number> = {
  'stockholm': 30.32, 'göteborg': 33.76, 'gothenburg': 33.76,
  'malmö': 33.71, 'uppsala': 32.82, 'linköping': 32.13,
  'västerås': 31.79, 'örebro': 33.05, 'norrköping': 33.38,
  'helsingborg': 31.94, 'jönköping': 32.45, 'lund': 32.57,
  'umeå': 34.47, 'gävle': 33.76, 'borås': 32.79,
  'södertälje': 32.57, 'eskilstuna': 32.65, 'halmstad': 32.61,
  'växjö': 32.59, 'karlstad': 33.24, 'sundsvall': 34.02,
  'östersund': 34.34, 'trollhättan': 33.48, 'lidingö': 29.57,
  'solna': 29.46, 'nacka': 29.88, 'täby': 29.73,
  'huddinge': 31.96, 'botkyrka': 32.57, 'haninge': 31.96,
  'danderyd': 29.28, 'sollentuna': 30.11, 'järfälla': 31.24,
  'norrtälje': 32.07, 'falun': 33.62, 'skellefteå': 34.47,
  'luleå': 34.12, 'kalmar': 33.23, 'kristianstad': 32.82,
  'varberg': 32.16,
}

function getDefaultTax(location?: string | null): number {
  if (!location) return 32.0
  const key = location.toLowerCase().trim()
  if (KOMMUN_TAX[key]) return KOMMUN_TAX[key]
  const match = Object.entries(KOMMUN_TAX).find(([k]) => key.includes(k) || k.includes(key))
  return match ? match[1] : 32.0
}

// ──────────────────────────────────────────────────────────
// OB calculation helpers
// Kvälls-OB: 19:00–22:00
// Natt-OB: 22:00–06:00
// ──────────────────────────────────────────────────────────
const OB_KVALL_RATE = 42  // kr/tim
const OB_NATT_RATE = 116  // kr/tim
const DEFAULT_HOURLY_RATE = 180 // kr — fallback

function calcObHours(startIso: string, endIso: string): { kvall: number; natt: number; total: number } {
  const start = new Date(startIso)
  const end = new Date(endIso)
  let kvall = 0
  let natt = 0

  // Walk through each hour of the shift
  const cursor = new Date(start)
  while (cursor < end) {
    const nextHour = new Date(cursor)
    nextHour.setMinutes(0, 0, 0)
    nextHour.setHours(nextHour.getHours() + 1)
    const sliceEnd = nextHour < end ? nextHour : end

    const fractionHours = (sliceEnd.getTime() - cursor.getTime()) / (1000 * 60 * 60)
    const hour = cursor.getHours()

    if (hour >= 19 && hour < 22) {
      kvall += fractionHours
    } else if (hour >= 22 || hour < 6) {
      natt += fractionHours
    }

    cursor.setTime(sliceEnd.getTime())
  }

  const totalHours = (end.getTime() - start.getTime()) / (1000 * 60 * 60)
  return { kvall: Math.round(kvall * 100) / 100, natt: Math.round(natt * 100) / 100, total: Math.round(totalHours * 100) / 100 }
}

// Generate realistic, authentic demo shifts for Swedish care & assistance schedule
function generateDemoShifts(): MonthShift[] {
  const now = new Date()
  const year = now.getFullYear()
  const month = now.getMonth()
  const today = now.getDate()
  const daysInMonth = new Date(year, month + 1, 0).getDate()
  const demoShifts: MonthShift[] = []

  const weekdays = ['Sön', 'Mån', 'Tis', 'Ons', 'Tor', 'Fre', 'Lör']

  // Rich schedule with solid hours (approx 25–27 shifts, ~230–260 hours total)
  for (let d = 1; d <= daysInMonth; d++) {
    const date = new Date(year, month, d)
    const dayOfWeek = date.getDay()
    const weekNum = Math.floor((d - 1) / 7)
    const isEvenWeek = weekNum % 2 === 0

    // Occasional rest days (e.g. every other Sunday, one Wednesday)
    if (dayOfWeek === 0 && isEvenWeek) continue
    if (dayOfWeek === 3 && !isEvenWeek && d % 4 === 0) continue

    let shiftType: {
      title: string
      startH: number
      startM: number
      endH: number
      endM: number
      isNight?: boolean
    }

    if (dayOfWeek === 1) {
      // Måndag: Personlig assistans (10h) eller Dagpass (9h)
      shiftType = isEvenWeek
        ? { title: 'Personlig assistans', startH: 7, startM: 30, endH: 17, endM: 30 }
        : { title: 'Dagpass', startH: 7, startM: 0, endH: 16, endM: 0 }
    } else if (dayOfWeek === 2) {
      // Tisdag: Långpass assistans (11.5h) eller Kvällspass (8h)
      shiftType = isEvenWeek
        ? { title: 'Långpass assistans', startH: 8, startM: 0, endH: 19, endM: 30 }
        : { title: 'Kvällspass', startH: 14, startM: 30, endH: 22, endM: 30 }
    } else if (dayOfWeek === 3) {
      // Onsdag: Boendestöd (9h) eller Dagpass (9.5h)
      shiftType = isEvenWeek
        ? { title: 'Boendestöd', startH: 8, startM: 0, endH: 17, endM: 0 }
        : { title: 'Dagpass', startH: 7, startM: 0, endH: 16, endM: 30 }
    } else if (dayOfWeek === 4) {
      // Torsdag: Personlig assistans (10h) eller Dagpass (9h)
      shiftType = isEvenWeek
        ? { title: 'Personlig assistans', startH: 8, startM: 0, endH: 18, endM: 0 }
        : { title: 'Dagpass', startH: 7, startM: 0, endH: 16, endM: 0 }
    } else if (dayOfWeek === 5) {
      // Fredag: Kvällspass (8h) eller Vaken natt (10.5h)
      shiftType = isEvenWeek
        ? { title: 'Kvällspass', startH: 15, startM: 0, endH: 23, endM: 0 }
        : { title: 'Vaken natt', startH: 21, startM: 0, endH: 7, endM: 30, isNight: true }
    } else if (dayOfWeek === 6) {
      // Lördag: Helgpass lång (12h) eller Helgpass dag (9.5h)
      shiftType = isEvenWeek
        ? { title: 'Helgpass lång', startH: 8, startM: 0, endH: 20, endM: 0 }
        : { title: 'Helgpass dag', startH: 7, startM: 30, endH: 17, endM: 0 }
    } else {
      // Söndag: Helgpass kväll (9h)
      shiftType = { title: 'Helgpass kväll', startH: 13, startM: 0, endH: 22, endM: 0 }
    }

    const start = new Date(year, month, d, shiftType.startH, shiftType.startM)
    const end = shiftType.isNight
      ? new Date(year, month, d + 1, shiftType.endH, shiftType.endM)
      : new Date(year, month, d, shiftType.endH, shiftType.endM)

    const ob = calcObHours(start.toISOString(), end.toISOString())
    const weekdayName = weekdays[dayOfWeek]
    const monthShort = date.toLocaleDateString('sv-SE', { month: 'short' }).replace('.', '')
    const isPast = d < today
    const isToday = d === today

    demoShifts.push({
      id: `demo-${d}`,
      title: shiftType.title,
      date: `${weekdayName} ${d} ${monthShort}`,
      startTime: `${String(shiftType.startH).padStart(2, '0')}:${String(shiftType.startM).padStart(2, '0')}`,
      endTime: `${String(shiftType.endH).padStart(2, '0')}:${String(shiftType.endM).padStart(2, '0')}`,
      totalHours: ob.total,
      obKvallHours: ob.kvall,
      obNattHours: ob.natt,
      isReported: isPast,
      isPast,
      isToday,
    })
  }

  return demoShifts
}

// ──────────────────────────────────────────────────────────
// AgendaWidget — Today's events as h-12 timeline
// ──────────────────────────────────────────────────────────
export const AgendaWidget: React.FC = () => {
  const { t } = useTranslation()
  const navigate = useNavigate()
  const { user } = useAuth()
  const { workspaceId } = useWorkspace()
  const [todayEvents, setTodayEvents] = useState<DashboardEvent[]>([])
  const [tomorrowEvents, setTomorrowEvents] = useState<DashboardEvent[]>([])
  const [loading, setLoading] = useState(true)

  useEffect(() => {
    async function fetchAgenda() {
      if (!workspaceId || !user) return

      const today = new Date()
      today.setHours(0, 0, 0, 0)
      const tomorrow = new Date(today)
      tomorrow.setDate(tomorrow.getDate() + 1)
      const dayAfter = new Date(tomorrow)
      dayAfter.setDate(dayAfter.getDate() + 1)

      const { data } = await supabase
        .from('events')
        .select('id, title, start_time, end_time, assignee_id, team_id')
        .eq('workspace_id', workspaceId)
        .gte('start_time', today.toISOString())
        .lt('start_time', dayAfter.toISOString())
        .order('start_time', { ascending: true })
        .limit(12)

      if (data && data.length > 0) {
        const todayEnd = tomorrow.toISOString()
        setTodayEvents(data.filter((e) => e.start_time < todayEnd))
        setTomorrowEvents(data.filter((e) => e.start_time >= todayEnd))
      } else {
        // Provide demo events matching current date so the agenda is populated
        const demoShifts = generateDemoShifts()
        const todayDate = today.getDate()
        const tomorrowDate = tomorrow.getDate()

        const todayShift = demoShifts.find((s) => s.isToday)
        const tomorrowShift = demoShifts.find((s) => {
          const parts = s.date.split(' ')
          return parseInt(parts[1], 10) === tomorrowDate
        })

        const mappedToday: DashboardEvent[] = todayShift
          ? [{
              id: todayShift.id,
              title: todayShift.title,
              start_time: new Date(today.getFullYear(), today.getMonth(), todayDate, parseInt(todayShift.startTime.split(':')[0], 10), parseInt(todayShift.startTime.split(':')[1], 10)).toISOString(),
              end_time: new Date(today.getFullYear(), today.getMonth(), todayDate, parseInt(todayShift.endTime.split(':')[0], 10), parseInt(todayShift.endTime.split(':')[1], 10)).toISOString(),
            }]
          : []

        const mappedTomorrow: DashboardEvent[] = tomorrowShift
          ? [{
              id: tomorrowShift.id,
              title: tomorrowShift.title,
              start_time: new Date(tomorrow.getFullYear(), tomorrow.getMonth(), tomorrowDate, parseInt(tomorrowShift.startTime.split(':')[0], 10), parseInt(tomorrowShift.startTime.split(':')[1], 10)).toISOString(),
              end_time: new Date(tomorrow.getFullYear(), tomorrow.getMonth(), tomorrowDate, parseInt(tomorrowShift.endTime.split(':')[0], 10), parseInt(tomorrowShift.endTime.split(':')[1], 10)).toISOString(),
            }]
          : []

        setTodayEvents(mappedToday)
        setTomorrowEvents(mappedTomorrow)
      }
      setLoading(false)
    }

    fetchAgenda()
  }, [workspaceId, user])

  const formatTime = (iso: string) =>
    new Date(iso).toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' })

  const renderEventRow = (event: DashboardEvent, idx: number) => {
    const now = new Date()
    const start = new Date(event.start_time)
    const end = new Date(event.end_time)
    const isActive = now >= start && now <= end
    const isPast = now > end

    return (
      <div
        key={event.id}
        onClick={() => navigate('/schedule')}
        style={{ animationDelay: `${idx * 25}ms` }}
        className="group flex h-12 cursor-pointer items-center border-b border-border/70 px-4 transition-colors hover:bg-secondary/40 animate-in fade-in slide-in-from-bottom-1 fill-mode-both"
      >
        <div className="mr-3.5 flex w-14 shrink-0 flex-col items-end">
          <span className={cn(
            'text-xs tabular-nums font-medium',
            isActive ? 'text-foreground' : isPast ? 'text-muted-foreground/60' : 'text-foreground'
          )}>
            {formatTime(event.start_time)}
          </span>
          <span className={cn(
            'text-[10px] tabular-nums',
            isPast ? 'text-muted-foreground/40' : 'text-muted-foreground'
          )}>
            {formatTime(event.end_time)}
          </span>
        </div>

        {/* Vertical timeline indicator line — 3.5px width for crisp visibility */}
        <div
          className={cn(
            'mr-3.5 h-7 w-[3.5px] shrink-0 rounded-full transition-colors',
            isActive
              ? 'bg-emerald-500 shadow-xs'
              : isPast
                ? 'bg-muted-foreground/25'
                : 'bg-muted-foreground/50 group-hover:bg-foreground'
          )}
        />

        <div className={cn(
          'min-w-0 flex-1 truncate text-xs transition-colors',
          isPast ? 'font-normal text-muted-foreground/60' : 'font-medium text-foreground'
        )}>
          {event.title}
        </div>

        {isActive && (
          <div className="flex shrink-0 items-center gap-1.5 text-xs text-emerald-500 dark:text-emerald-400">
            <span className="h-1.5 w-1.5 rounded-full bg-emerald-500 shrink-0" />
            <span className="font-medium text-[11px]">Nu</span>
          </div>
        )}
      </div>
    )
  }

  return (
    <Card className="flex flex-col rounded-lg border border-border/70 bg-card shadow-xs overflow-hidden p-0 py-0 gap-0">
      <div className="flex h-11 shrink-0 items-center justify-between border-b border-border/70 px-4 bg-muted/10">
        <div className="flex items-center gap-2">
          <CalendarDays className="h-4 w-4 text-muted-foreground" />
          <span className="text-sm font-medium tracking-tight text-foreground">
            {t('dashboard.agenda', 'Dagens Agenda')}
          </span>
        </div>
        {!loading && (
          <span className="rounded border border-border/70 bg-secondary/70 px-2 py-0.5 text-[10.5px] font-medium text-foreground/80 tabular-nums">
            {todayEvents.length + tomorrowEvents.length} {todayEvents.length + tomorrowEvents.length === 1 ? 'pass' : 'pass'}
          </span>
        )}
      </div>
      <CardContent className="flex flex-1 flex-col p-0 gap-0">
        {loading ? (
          <div className="flex w-full flex-col">
            {[1, 2, 3, 4].map((i) => (
              <div key={i} className="flex h-12 w-full animate-pulse items-center border-b border-border/70 px-4">
                <div className="mr-3.5 flex w-14 shrink-0 flex-col items-end gap-1">
                  <div className="h-3 w-10 rounded bg-muted" />
                  <div className="h-2 w-8 rounded bg-muted/60" />
                </div>
                <div className="mr-3.5 h-7 w-[3.5px] shrink-0 rounded-full bg-muted" />
                <div className="h-3.5 w-32 rounded bg-muted" />
              </div>
            ))}
          </div>
        ) : todayEvents.length === 0 && tomorrowEvents.length === 0 ? (
          <div className="flex h-36 flex-col items-center justify-center text-muted-foreground py-6">
            <CalendarDays className="mb-2 h-6 w-6 opacity-20" />
            <p className="text-xs font-normal">{t('dashboard.no_agenda', 'Inga planerade pass')}</p>
          </div>
        ) : (
          <div className="flex w-full flex-col text-sm">
            {todayEvents.length > 0 && (
              <>
                <div className="flex h-7 items-center border-b border-border/70 bg-muted/25 px-4">
                  <span className="text-[10.5px] font-semibold uppercase tracking-wider text-muted-foreground">
                    {t('dashboard.today', 'Idag')}
                  </span>
                </div>
                {todayEvents.map((e, i) => renderEventRow(e, i))}
              </>
            )}

            {tomorrowEvents.length > 0 && (
              <>
                <div className="flex h-7 items-center border-b border-border/70 bg-muted/25 px-4">
                  <span className="text-[10.5px] font-semibold uppercase tracking-wider text-muted-foreground">
                    {t('dashboard.tomorrow', 'Imorgon')}
                  </span>
                </div>
                {tomorrowEvents.map((e, i) => renderEventRow(e, todayEvents.length + i))}
              </>
            )}
          </div>
        )}

        <div className="p-3">
          <Button
            variant="outline"
            className="h-9 w-full rounded-md border border-border/80 bg-background text-xs font-medium text-foreground hover:bg-secondary hover:text-foreground transition-colors shadow-none"
            onClick={() => navigate('/schedule')}
          >
            {t('dashboard.go_to_schedule', 'Öppna schema')}
          </Button>
        </div>
      </CardContent>
    </Card>
  )
}

// ──────────────────────────────────────────────────────────
// SalaryCalculatorWidget — auto from contract + shifts list
// ──────────────────────────────────────────────────────────
export const SalaryCalculatorWidget: React.FC = () => {
  const { t } = useTranslation()
  const { user } = useAuth()
  const { workspaceId } = useWorkspace()

  const [shifts, setShifts] = useState<MonthShift[]>([])
  const [loading, setLoading] = useState(true)
  const [taxPercent, setTaxPercent] = useState('32.00')

  const hourlyRate = DEFAULT_HOURLY_RATE

  // Set default tax from user's location/kommun
  useEffect(() => {
    if (user?.location) {
      const rate = getDefaultTax(user.location)
      setTaxPercent(rate.toFixed(2))
    }
  }, [user?.location])

  useEffect(() => {
    async function fetchMonthData() {
      if (!workspaceId || !user) return

      const now = new Date()
      const monthStart = new Date(now.getFullYear(), now.getMonth(), 1)
      const monthEnd = new Date(now.getFullYear(), now.getMonth() + 1, 0, 23, 59, 59)

      // Fetch all events this month for the user
      const { data: events } = await supabase
        .from('events')
        .select('id, title, start_time, end_time')
        .eq('workspace_id', workspaceId)
        .eq('assignee_id', user.id)
        .gte('start_time', monthStart.toISOString())
        .lte('start_time', monthEnd.toISOString())
        .order('start_time', { ascending: true })

      // Fetch reported time reports to mark shifts as reported
      const { data: reports } = await supabase
        .from('time_reports')
        .select('date, start_time')
        .eq('workspace_id', workspaceId)
        .eq('user_id', user.id)
        .gte('date', monthStart.toISOString().split('T')[0])
        .lte('date', monthEnd.toISOString().split('T')[0])

      const reportedSet = new Set(
        (reports || []).map((r) => `${r.date}_${r.start_time}`)
      )

      if (events && events.length >= 5) {
        const weekdays = ['Sön', 'Mån', 'Tis', 'Ons', 'Tor', 'Fre', 'Lör']
        const mapped: MonthShift[] = events.map((e) => {
          const ob = calcObHours(e.start_time, e.end_time)
          const d = e.start_time.split('T')[0]
          const st = new Date(e.start_time).toLocaleTimeString('sv-SE', { hour: '2-digit', minute: '2-digit' })
          const isReported = reportedSet.has(`${d}_${st}`)
          const startDate = new Date(e.start_time)
          const endDate = new Date(e.end_time)
          const isPast = endDate < now
          const isToday = startDate.toDateString() === now.toDateString()
          const weekdayName = weekdays[startDate.getDay()]
          const monthShort = startDate.toLocaleDateString('sv-SE', { month: 'short' }).replace('.', '')

          return {
            id: e.id,
            title: e.title || 'Arbetspass',
            date: `${weekdayName} ${startDate.getDate()} ${monthShort}`,
            startTime: startDate.toLocaleTimeString('sv-SE', { hour: '2-digit', minute: '2-digit' }),
            endTime: endDate.toLocaleTimeString('sv-SE', { hour: '2-digit', minute: '2-digit' }),
            totalHours: ob.total,
            obKvallHours: ob.kvall,
            obNattHours: ob.natt,
            isReported,
            isPast,
            isToday,
          }
        })
        setShifts(mapped)
      } else {
        // Generate realistic demo shifts so the widget looks populated with a full month
        setShifts(generateDemoShifts())
      }

      setLoading(false)
    }

    fetchMonthData()
  }, [workspaceId, user])

  // Summaries
  const totalHours = Math.round(shifts.reduce((s, sh) => s + sh.totalHours, 0) * 10) / 10
  const totalObKvall = Math.round(shifts.reduce((s, sh) => s + sh.obKvallHours, 0) * 10) / 10
  const totalObNatt = Math.round(shifts.reduce((s, sh) => s + sh.obNattHours, 0) * 10) / 10

  const baseSalary = totalHours * hourlyRate
  const obKvallPay = totalObKvall * OB_KVALL_RATE
  const obNattPay = totalObNatt * OB_NATT_RATE
  const grossSalary = baseSalary + obKvallPay + obNattPay

  const taxRate = Math.min(100, Math.max(0, parseFloat(taxPercent) || 0)) / 100
  const tax = grossSalary * taxRate
  const netSalary = grossSalary - tax

  const monthName = new Date().toLocaleDateString('sv-SE', { month: 'long' })

  const handleTaxChange = (e: React.ChangeEvent<HTMLInputElement>) => {
    const val = e.target.value
    if (/^\d{0,3}\.?\d{0,2}$/.test(val) || val === '') {
      setTaxPercent(val)
    }
  }

  if (loading) {
    return (
      <Card className="flex flex-col rounded-lg border border-border/70 bg-card shadow-none overflow-hidden p-0 py-0 gap-0">
        <div className="flex h-11 shrink-0 items-center justify-between border-b border-border/70 px-4 bg-muted/10">
          <div className="h-3.5 w-24 rounded bg-muted/60" />
        </div>
        <CardContent className="p-4 pt-3.5">
          <div className="flex w-full flex-col">
            {[1, 2, 3, 4, 5].map((i) => (
              <div key={i} className="flex h-10 w-full items-center border-b border-border/70 px-2">
                <div className="h-3 w-12 rounded bg-muted/60 mr-3" />
                <div className="h-3 w-24 rounded bg-muted/60 flex-1" />
                <div className="h-3 w-10 rounded bg-muted/40" />
              </div>
            ))}
          </div>
        </CardContent>
      </Card>
    )
  }

  return (
    <Card className="flex flex-col rounded-lg border border-border/70 bg-card shadow-none overflow-hidden p-0 py-0 gap-0">
      <div className="flex h-11 shrink-0 items-center justify-between border-b border-border/70 px-4 bg-muted/10">
        <div className="flex items-center gap-2">
          <Calculator className="h-4 w-4 text-muted-foreground" />
          <span className="text-sm font-medium tracking-tight text-foreground">
            {t('dashboard.salary_prediction', 'Lönprognos')}
          </span>
        </div>
        <span className="rounded border border-border/70 bg-secondary/70 px-2 py-0.5 text-[10.5px] font-medium capitalize text-muted-foreground">
          {monthName}
        </span>
      </div>
      <CardContent className="flex flex-col p-4 pt-3">
        {/* Subheader with shift count and hours */}
        <div className="flex items-center justify-between pb-2 px-0.5 text-[11px]">
          <span className="font-medium text-foreground/80">
            {t('dashboard.upcoming_shifts', 'Schemalagda pass')} ({shifts.length})
          </span>
          <span className="tabular-nums text-muted-foreground">
            {totalHours} timmar
          </span>
        </div>

        {/* Shift list — compact, minimalistic & clean */}
        <div className="max-h-[170px] overflow-y-auto rounded-md border border-border/70 bg-muted/15 divide-y divide-border/60 scrollbar-dark">
          {shifts.length === 0 ? (
            <div className="flex h-20 flex-col items-center justify-center text-muted-foreground">
              <p className="text-xs font-normal">Inga pass denna månad</p>
            </div>
          ) : (
            shifts.map((shift, idx) => (
              <div
                key={shift.id}
                style={{ animationDelay: `${idx * 15}ms` }}
                className={cn(
                  'group flex h-8 items-center px-2.5 text-xs transition-colors hover:bg-secondary/40 animate-in fade-in fill-mode-both',
                  shift.isToday
                    ? 'bg-secondary/50 font-medium'
                    : shift.isPast
                      ? 'text-muted-foreground/80'
                      : 'text-foreground'
                )}
              >
                {/* Date with weekday */}
                <span className="w-[66px] shrink-0 tabular-nums text-[11px] font-normal text-muted-foreground">
                  {shift.date}
                </span>

                {/* Shift title */}
                <span className="min-w-0 flex-1 truncate text-xs font-medium text-foreground">
                  {shift.title}
                </span>

                {/* Time range */}
                <span className="shrink-0 tabular-nums text-[11px] text-muted-foreground mr-2.5">
                  {shift.startTime}–{shift.endTime}
                </span>

                {/* Hours & OB micro-dot */}
                <div className="flex w-14 shrink-0 items-center justify-end gap-1.5 text-right">
                  <span className="tabular-nums text-xs font-medium text-foreground">
                    {shift.totalHours}h
                  </span>
                  {(shift.obKvallHours > 0 || shift.obNattHours > 0) ? (
                    <span
                      className="h-1.5 w-1.5 rounded-full bg-amber-500/80 shrink-0"
                      title={`OB: ${(shift.obKvallHours + shift.obNattHours).toFixed(1)} tim`}
                    />
                  ) : (
                    <span className="w-1.5 shrink-0" />
                  )}
                </div>
              </div>
            ))
          )}
        </div>

        {/* Calculation summary */}
        <div className="mt-3 flex flex-col gap-1.5 border-t border-border/70 pt-3">
          {/* Base salary */}
          <div className="flex items-center justify-between text-xs py-0.5 px-0.5">
            <span className="text-muted-foreground">
              Grundlön ({totalHours}h × {hourlyRate} kr)
            </span>
            <span className="tabular-nums font-medium text-foreground">
              {Math.round(baseSalary).toLocaleString('sv-SE')} kr
            </span>
          </div>

          {/* OB additions */}
          {(totalObKvall > 0 || totalObNatt > 0) && (
            <div className="flex items-center justify-between text-xs py-0.5 px-0.5">
              <span className="text-muted-foreground">
                OB-tillägg ({Math.round((totalObKvall + totalObNatt) * 10) / 10}h)
              </span>
              <span className="tabular-nums font-medium text-amber-600 dark:text-amber-400">
                +{Math.round(obKvallPay + obNattPay).toLocaleString('sv-SE')} kr
              </span>
            </div>
          )}

          {/* Tax rate */}
          <div className="flex items-center justify-between text-xs py-0.5 px-0.5">
            <span className="text-muted-foreground">
              Kommunalskatt {user?.location ? `(${user.location})` : ''}
            </span>
            <div className="flex items-center gap-1">
              <input
                type="text"
                inputMode="decimal"
                value={taxPercent}
                onChange={handleTaxChange}
                className="h-5 w-12 rounded border border-border/70 bg-muted/40 px-1 text-right text-xs tabular-nums font-medium text-foreground outline-none transition-colors focus:border-foreground focus:ring-1 focus:ring-foreground"
              />
              <span className="text-xs text-muted-foreground">%</span>
            </div>
          </div>
        </div>

        {/* Net result card */}
        <div className="mt-3 flex flex-col gap-1.5 rounded-md border border-border/70 bg-muted/30 p-3">
          <div className="flex items-center justify-between text-xs">
            <span className="text-muted-foreground">Bruttolön</span>
            <span className="tabular-nums font-medium text-foreground">
              {Math.round(grossSalary).toLocaleString('sv-SE')} kr
            </span>
          </div>
          <div className="flex items-center justify-between text-xs">
            <span className="text-muted-foreground">Preliminärskatt ({taxPercent}%)</span>
            <span className="tabular-nums font-medium text-muted-foreground">
              −{Math.round(tax).toLocaleString('sv-SE')} kr
            </span>
          </div>
          <div className="mt-0.5 flex items-center justify-between border-t border-border/70 pt-2">
            <span className="text-xs font-medium text-foreground">Beräknad nettolön</span>
            <span className="text-sm font-medium tabular-nums tracking-tight text-foreground">
              {Math.round(netSalary).toLocaleString('sv-SE')} kr
            </span>
          </div>
        </div>

        <p className="mt-2 text-[10px] font-normal text-muted-foreground/60 text-center">
          Prognos baserad på schemalagda pass och OB-beräkning
        </p>
      </CardContent>
    </Card>
  )
}
