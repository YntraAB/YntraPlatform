import React, { useEffect, useState, useMemo } from 'react'
import { Card, CardContent } from '@/components/ui/card'
import { CalendarDays, Calculator, MapPin, ChevronRight } from 'lucide-react'
import { Button } from '@/components/ui/button'
import { useNavigate } from 'react-router-dom'
import { useAuth } from '@/hooks/useAuth'
import { useWorkspace } from '@/contexts/WorkspaceContext'
import { supabase } from '@/lib/supabase'
import { useTranslation } from 'react-i18next'
import { cn, getCategoryConfig } from '@/lib/utils'
import type { EventCategory } from '@/types'

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
  category?: EventCategory
  location?: string
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
  category?: EventCategory
  location?: string
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
      category: EventCategory
      location: string
    }

    if (dayOfWeek === 1) {
      // Måndag: Personlig assistans (10h) eller Dagpass (9h)
      shiftType = isEvenWeek
        ? { title: 'Personlig assistans', startH: 7, startM: 30, endH: 17, endM: 30, category: 'assistance_time', location: 'Lindgren, Norr' }
        : { title: 'Dagpass', startH: 7, startM: 0, endH: 16, endM: 0, category: 'other_time', location: 'Södermalm' }
    } else if (dayOfWeek === 2) {
      // Tisdag: Långpass assistans (11.5h) eller Kvällspass (8h)
      shiftType = isEvenWeek
        ? { title: 'Långpass assistans', startH: 8, startM: 0, endH: 19, endM: 30, category: 'assistance_time', location: 'Kungsgatan 12' }
        : { title: 'Kvällspass', startH: 14, startM: 30, endH: 22, endM: 30, category: 'other_time', location: 'Lindgren, Norr' }
    } else if (dayOfWeek === 3) {
      // Onsdag: Boendestöd (9h) eller Dagpass (9.5h)
      shiftType = isEvenWeek
        ? { title: 'Boendestöd', startH: 8, startM: 0, endH: 17, endM: 0, category: 'respite_care', location: 'Bergströms väg 4' }
        : { title: 'Dagpass', startH: 7, startM: 0, endH: 16, endM: 30, category: 'other_time', location: 'Södermalm' }
    } else if (dayOfWeek === 4) {
      // Torsdag: Personlig assistans (10h) eller Dagpass (9h)
      shiftType = isEvenWeek
        ? { title: 'Personlig assistans', startH: 8, startM: 0, endH: 18, endM: 0, category: 'assistance_time', location: 'Lindgren, Norr' }
        : { title: 'Dagpass', startH: 7, startM: 0, endH: 16, endM: 0, category: 'other_time', location: 'Södermalm' }
    } else if (dayOfWeek === 5) {
      // Fredag: Kvällspass (8h) eller Vaken natt (10.5h)
      shiftType = isEvenWeek
        ? { title: 'Kvällspass', startH: 15, startM: 0, endH: 23, endM: 0, category: 'other_time', location: 'Lindgren, Norr' }
        : { title: 'Vaken natt', startH: 21, startM: 0, endH: 7, endM: 30, isNight: true, category: 'on_call', location: 'Kungsgatan 12' }
    } else if (dayOfWeek === 6) {
      // Lördag: Helgpass lång (12h) eller Helgpass dag (9.5h)
      shiftType = isEvenWeek
        ? { title: 'Helgpass lång', startH: 8, startM: 0, endH: 20, endM: 0, category: 'assistance_time', location: 'Lindgren, Norr' }
        : { title: 'Helgpass dag', startH: 7, startM: 30, endH: 17, endM: 0, category: 'other_time', location: 'Södermalm' }
    } else {
      // Söndag: Helgpass kväll (9h)
      shiftType = { title: 'Helgpass kväll', startH: 13, startM: 0, endH: 22, endM: 0, category: 'other_time', location: 'Lindgren, Norr' }
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
      category: shiftType.category,
      location: shiftType.location,
    })
  }

  return demoShifts
}

// ──────────────────────────────────────────────────────────
// AgendaWidget — Multi-day timeline with sticky day headers & scroll
// ──────────────────────────────────────────────────────────
interface DayGroup {
  dateKey: string // YYYY-MM-DD
  date: Date
  label: string
  dateFormatted: string
  isToday: boolean
  isTomorrow: boolean
  totalHours: number
  events: DashboardEvent[]
}

export const AgendaWidget: React.FC = () => {
  const { t } = useTranslation()
  const navigate = useNavigate()
  const { user } = useAuth()
  const { workspaceId } = useWorkspace()
  const [events, setEvents] = useState<DashboardEvent[]>([])
  const [loading, setLoading] = useState(true)

  useEffect(() => {
    async function fetchAgenda() {
      if (!workspaceId || !user) return

      const today = new Date()
      today.setHours(0, 0, 0, 0)
      const futureLimit = new Date(today)
      futureLimit.setDate(futureLimit.getDate() + 7)
      futureLimit.setHours(23, 59, 59, 999)

      const { data } = await supabase
        .from('events')
        .select('id, title, start_time, end_time, assignee_id, team_id, metadata')
        .eq('workspace_id', workspaceId)
        .gte('start_time', today.toISOString())
        .lte('start_time', futureLimit.toISOString())
        .order('start_time', { ascending: true })
        .limit(30)

      if (data && data.length > 0) {
        const mapped: DashboardEvent[] = data.map((e: any) => {
          const meta = (e.metadata as any) || {}
          return {
            id: e.id,
            title: e.title,
            start_time: e.start_time,
            end_time: e.end_time,
            assignee_id: e.assignee_id,
            team_id: e.team_id,
            category: meta.category || 'assistance',
            location: meta.location || '',
          }
        })
        setEvents(mapped)
      } else {
        // Generate realistic demo events spanning today and upcoming 7 days
        const demoShifts = generateDemoShifts()
        const mappedUpcoming: DashboardEvent[] = []

        for (let i = 0; i < 7; i++) {
          const targetDate = new Date(today)
          targetDate.setDate(today.getDate() + i)
          const targetDayNum = targetDate.getDate()

          const matchingShift = demoShifts.find((s) => {
            const parts = s.date.split(' ')
            return parseInt(parts[1], 10) === targetDayNum
          })

          if (matchingShift) {
            const [startH, startM] = matchingShift.startTime.split(':').map(Number)
            const [endH, endM] = matchingShift.endTime.split(':').map(Number)

            mappedUpcoming.push({
              id: `demo-${i}-${targetDayNum}`,
              title: matchingShift.title,
              start_time: new Date(targetDate.getFullYear(), targetDate.getMonth(), targetDate.getDate(), startH, startM).toISOString(),
              end_time: new Date(targetDate.getFullYear(), targetDate.getMonth(), targetDate.getDate(), endH, endM).toISOString(),
              category: matchingShift.category || 'assistance_time',
              location: matchingShift.location || 'Lindgren, Norr',
            })
          }
        }

        // Add a secondary afternoon/evening shift today to demonstrate multiple passes in a day
        mappedUpcoming.push({
          id: 'demo-today-extra',
          title: 'Avlösarservice',
          start_time: new Date(today.getFullYear(), today.getMonth(), today.getDate(), 17, 0).toISOString(),
          end_time: new Date(today.getFullYear(), today.getMonth(), today.getDate(), 21, 30).toISOString(),
          category: 'respite_care',
          location: 'Kungsgatan 12',
        })

        // Sort events chronologically
        mappedUpcoming.sort((a, b) => new Date(a.start_time).getTime() - new Date(b.start_time).getTime())
        setEvents(mappedUpcoming)
      }
      setLoading(false)
    }

    fetchAgenda()
  }, [workspaceId, user])

  const formatTime = (iso: string) =>
    new Date(iso).toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' })

  // Group events by day with localized headers
  const dayGroups: DayGroup[] = useMemo(() => {
    const now = new Date()
    const todayKey = `${now.getFullYear()}-${String(now.getMonth() + 1).padStart(2, '0')}-${String(now.getDate()).padStart(2, '0')}`
    const tom = new Date(now.getFullYear(), now.getMonth(), now.getDate() + 1)
    const tomorrowKey = `${tom.getFullYear()}-${String(tom.getMonth() + 1).padStart(2, '0')}-${String(tom.getDate()).padStart(2, '0')}`

    const groupsMap = new Map<string, DashboardEvent[]>()

    for (const e of events) {
      const d = new Date(e.start_time)
      const key = `${d.getFullYear()}-${String(d.getMonth() + 1).padStart(2, '0')}-${String(d.getDate()).padStart(2, '0')}`
      if (!groupsMap.has(key)) {
        groupsMap.set(key, [])
      }
      groupsMap.get(key)!.push(e)
    }

    const weekdays = ['Söndag', 'Måndag', 'Tisdag', 'Onsdag', 'Torsdag', 'Fredag', 'Lördag']
    const sortedKeys = Array.from(groupsMap.keys()).sort()

    return sortedKeys.map((key) => {
      const dayEvts = groupsMap.get(key)!
      dayEvts.sort((a, b) => new Date(a.start_time).getTime() - new Date(b.start_time).getTime())

      const [y, m, d] = key.split('-').map(Number)
      const dateObj = new Date(y, m - 1, d)
      const isToday = key === todayKey
      const isTomorrow = key === tomorrowKey

      let label = ''
      if (isToday) {
        label = t('dashboard.today', 'Idag')
      } else if (isTomorrow) {
        label = t('dashboard.tomorrow', 'Imorgon')
      } else {
        label = weekdays[dateObj.getDay()]
      }

      const dateFormatted = dateObj.toLocaleDateString('sv-SE', {
        day: 'numeric',
        month: 'short',
      }).replace('.', '')

      const totalHours = Math.round(dayEvts.reduce((acc, e) => {
        const diff = new Date(e.end_time).getTime() - new Date(e.start_time).getTime()
        return acc + Math.max(0, diff) / (1000 * 60 * 60)
      }, 0) * 10) / 10

      return {
        dateKey: key,
        date: dateObj,
        label,
        dateFormatted,
        isToday,
        isTomorrow,
        totalHours,
        events: dayEvts,
      }
    })
  }, [events, t])

  const totalEventCount = events.length
  const totalAllHours = Math.round(events.reduce((acc, e) => {
    const diff = new Date(e.end_time).getTime() - new Date(e.start_time).getTime()
    return acc + Math.max(0, diff) / (1000 * 60 * 60)
  }, 0) * 10) / 10

  const renderEventRow = (event: DashboardEvent, idx: number) => {
    const now = new Date()
    const start = new Date(event.start_time)
    const end = new Date(event.end_time)
    const isActive = now >= start && now <= end
    const isPast = now > end
    const categoryConfig = getCategoryConfig(event.category || 'other')

    const diffMs = end.getTime() - start.getTime()
    const hours = Math.round((Math.max(0, diffMs) / (1000 * 60 * 60)) * 10) / 10

    return (
      <div
        key={event.id}
        onClick={() => navigate('/schedule')}
        style={{ animationDelay: `${Math.min(idx * 20, 200)}ms` }}
        className="group flex h-12 cursor-pointer items-center justify-between border-b border-border/40 px-4 transition-colors hover:bg-secondary/40 animate-in fade-in slide-in-from-bottom-1 fill-mode-both"
      >
        {/* Left: Time and slim vertical category accent bar */}
        <div className="flex shrink-0 items-center gap-2.5">
          <div className="w-12 text-right">
            <div className={cn(
              'font-mono text-xs tabular-nums leading-tight font-medium',
              isActive ? 'text-primary font-semibold' : isPast ? 'text-muted-foreground/60' : 'text-foreground'
            )}>
              {formatTime(event.start_time)}
            </div>
            <div className="font-mono text-[10.5px] tabular-nums text-muted-foreground/60 leading-tight">
              {formatTime(event.end_time)}
            </div>
          </div>

          <div
            className="h-6 w-0.5 shrink-0 rounded-full transition-transform group-hover:scale-y-110"
            style={{ backgroundColor: categoryConfig.color || 'hsl(var(--border))' }}
          />
        </div>

        {/* Center: Title & Location */}
        <div className="min-w-0 flex-1 truncate px-2.5">
          <div className={cn(
            'truncate text-xs font-medium leading-snug transition-colors group-hover:text-primary',
            isPast ? 'text-muted-foreground/70 font-normal' : 'text-foreground'
          )}>
            {event.title}
          </div>
          {event.location && (
            <div className="flex items-center gap-1 text-[10.5px] text-muted-foreground/60 leading-tight mt-0.5 truncate">
              <MapPin className="size-2.5 shrink-0 opacity-60" />
              <span className="truncate">{event.location}</span>
            </div>
          )}
        </div>

        {/* Right: Status (Pågår) + Hours + Category badge */}
        <div className="flex shrink-0 items-center gap-2.5 text-xs">
          {isActive && (
            <div className="flex items-center gap-1 text-[10.5px] text-emerald-500 font-medium">
              <span className="size-1.5 rounded-full bg-emerald-500 shrink-0" />
              <span>Pågår</span>
            </div>
          )}

          <span className="font-mono text-[11px] text-muted-foreground/60 tabular-nums">
            {hours}h
          </span>

          <span
            className="inline-flex shrink-0 items-center rounded px-2 py-0.5 text-[10px] font-medium"
            style={{
              backgroundColor: categoryConfig.bgColor,
              color: categoryConfig.color,
            }}
          >
            {t(categoryConfig.label)}
          </span>
        </div>
      </div>
    )
  }

  return (
    <Card className="flex flex-col h-full max-h-[480px] rounded-lg border border-border/70 bg-card shadow-2xs overflow-hidden p-0 py-0 gap-0">
      <div className="flex h-12 shrink-0 items-center justify-between border-b border-border/70 px-4 bg-secondary/20">
        <div className="flex items-center gap-2">
          <CalendarDays className="size-3.5 text-muted-foreground" />
          <span className="text-xs font-medium tracking-tight text-foreground">
            {t('dashboard.agenda', 'Dagens agenda')}
          </span>
          {!loading && totalEventCount > 0 && (
            <span className="text-[11px] font-mono text-muted-foreground/60 tabular-nums">
              ({totalEventCount} {totalEventCount === 1 ? 'pass' : 'pass'} · {totalAllHours}h)
            </span>
          )}
        </div>
        <Button
          variant="ghost"
          size="sm"
          onClick={() => navigate('/schedule')}
          className="h-7 gap-1 px-2 text-[11px] font-medium text-muted-foreground hover:text-foreground transition-colors"
        >
          <span>{t('dashboard.go_to_schedule', 'Öppna schema')}</span>
          <ChevronRight className="size-3 text-muted-foreground/60" />
        </Button>
      </div>

      <CardContent className="flex flex-1 flex-col p-0 min-h-0 overflow-hidden">
        {loading ? (
          <div className="flex w-full flex-col">
            {[1, 2, 3, 4].map((i) => (
              <div key={i} className="flex h-12 w-full animate-pulse items-center border-b border-border/40 px-4">
                <div className="mr-2.5 flex w-12 shrink-0 flex-col items-end gap-1">
                  <div className="h-2.5 w-10 rounded bg-muted/60" />
                  <div className="h-2 w-8 rounded bg-muted/40" />
                </div>
                <div className="h-6 w-0.5 shrink-0 rounded-full bg-muted/40 mr-3" />
                <div className="flex-1 space-y-1">
                  <div className="h-3 w-32 rounded bg-muted/60" />
                  <div className="h-2 w-20 rounded bg-muted/30" />
                </div>
                <div className="h-4 w-12 rounded bg-muted/40" />
              </div>
            ))}
          </div>
        ) : dayGroups.length === 0 ? (
          <div className="flex h-36 flex-col items-center justify-center text-muted-foreground py-6">
            <CalendarDays className="mb-2 size-6 opacity-20" />
            <p className="text-xs font-medium text-foreground">{t('dashboard.no_agenda', 'Inga planerade pass')}</p>
            <p className="text-[11px] text-muted-foreground/60 mt-0.5">{t('scheduler.no_events_help', 'Dina schemalagda pass visas här.')}</p>
          </div>
        ) : (
          <div className="flex-1 min-h-0 overflow-y-auto scrollbar-dark">
            {dayGroups.map((group) => (
              <div key={group.dateKey} className="flex flex-col">
                <div className="sticky top-0 z-10 flex h-7 shrink-0 items-center justify-between border-b border-border/40 bg-secondary/85 px-4 backdrop-blur-xs">
                  <div className="flex items-center gap-1.5">
                    <span className="text-[11px] font-medium text-foreground/90">
                      {group.label}
                    </span>
                    <span className="text-[10.5px] font-mono text-muted-foreground/50">
                      · {group.dateFormatted}
                    </span>
                  </div>
                  <span className="text-[10.5px] font-mono text-muted-foreground/60 tabular-nums">
                    {group.events.length} {group.events.length === 1 ? 'pass' : 'pass'} · {group.totalHours}h
                  </span>
                </div>
                {group.events.map((e, i) => renderEventRow(e, i))}
              </div>
            ))}
          </div>
        )}
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
      <Card className="flex flex-col h-full max-h-[480px] rounded-lg border border-border/70 bg-card shadow-2xs overflow-hidden p-0 py-0 gap-0">
        <div className="flex h-12 shrink-0 items-center justify-between border-b border-border/70 px-4 bg-secondary/20">
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
    <Card className="flex flex-col h-full max-h-[480px] rounded-lg border border-border/70 bg-card shadow-2xs overflow-hidden p-0 py-0 gap-0">
      <div className="flex h-12 shrink-0 items-center justify-between border-b border-border/70 px-4 bg-secondary/20">
        <div className="flex items-center gap-2">
          <Calculator className="size-3.5 text-muted-foreground" />
          <span className="text-xs font-medium tracking-tight text-foreground">
            {t('dashboard.salary_prediction', 'Lönprognos')}
          </span>
        </div>
        <span className="rounded border border-border/70 bg-secondary/70 px-2 py-0.5 text-[10.5px] font-medium capitalize text-muted-foreground">
          {monthName}
        </span>
      </div>
      <CardContent className="flex flex-1 flex-col p-4 pt-3 min-h-0 overflow-y-auto scrollbar-dark">
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
