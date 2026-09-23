import React, { useMemo } from 'react'
import { useWorkspace } from '@/contexts/WorkspaceContext'
import { cn, isSameDay, isToday } from '@/lib/utils'
import type { CalendarEvent, CalendarView as ViewType, User } from '@/types'
import { MonthEventCard } from './MonthEventCard'

interface MonthViewProps {
  selectedDate: Date
  events: CalendarEvent[]
  users: User[]
  onEventClick: (event: CalendarEvent) => void
  onDateChange: (date: Date) => void
  onViewChange?: (view: ViewType) => void
}

export const MonthView: React.FC<MonthViewProps> = React.memo(({
  selectedDate,
  events,
  onEventClick,
  onDateChange,
  onViewChange,
}) => {
  const { settings } = useWorkspace()
  const locale = settings.language === 'sv' ? 'sv-SE' : 'en-US'

  const calendarDays = useMemo(() => {
    const year = selectedDate.getFullYear()
    const month = selectedDate.getMonth()

    const firstDayOfMonth = new Date(year, month, 1)
    const lastDayOfMonth = new Date(year, month + 1, 0)

    const startDayOfWeek = firstDayOfMonth.getDay()
    const diffToStart =
      (startDayOfWeek < settings.week_start ? 7 : 0) + startDayOfWeek - settings.week_start

    const daysInMonth = lastDayOfMonth.getDate()
    const daysInPrevMonth = new Date(year, month, 0).getDate()

    const days: Array<{
      date: Date
      dayOfMonth: number
      isCurrentMonth: boolean
      isToday: boolean
      events: CalendarEvent[]
    }> = []

    for (let i = diffToStart - 1; i >= 0; i--) {
      const date = new Date(year, month - 1, daysInPrevMonth - i)
      days.push({
        date,
        dayOfMonth: daysInPrevMonth - i,
        isCurrentMonth: false,
        isToday: isToday(date),
        events: events.filter((e) => isSameDay(e.startTime, date)),
      })
    }

    for (let day = 1; day <= daysInMonth; day++) {
      const date = new Date(year, month, day)
      days.push({
        date,
        dayOfMonth: day,
        isCurrentMonth: true,
        isToday: isToday(date),
        events: events.filter((e) => isSameDay(e.startTime, date)),
      })
    }

    const remainingCells = 42 - days.length
    for (let day = 1; day <= remainingCells; day++) {
      const date = new Date(year, month + 1, day)
      days.push({
        date,
        dayOfMonth: day,
        isCurrentMonth: false,
        isToday: isToday(date),
        events: events.filter((e) => isSameDay(e.startTime, date)),
      })
    }

    return days
  }, [selectedDate, events, settings.week_start])

  const dayLabels = useMemo(() => {
    const labels: string[] = []
    const base = new Date(2026, 0, settings.week_start === 1 ? 5 : 4)
    for (let i = 0; i < 7; i++) {
      const d = new Date(base)
      d.setDate(base.getDate() + i)
      labels.push(d.toLocaleDateString(locale, { weekday: 'short' }))
    }
    return labels
  }, [settings.week_start, locale])

  return (
    <div className="scrollbar-dark flex-1 overflow-y-auto px-6 py-4 min-h-0 bg-background">
      {/* Day labels */}
      <div className="mb-2 grid grid-cols-7 gap-1">
        {dayLabels.map((label, index) => (
          <div key={index} className="py-1.5 text-center text-[11px] font-medium text-muted-foreground/70 uppercase tracking-wider">
            {label}
          </div>
        ))}
      </div>

      {/* Calendar grid */}
      <div className="grid grid-cols-7 gap-1">
        {calendarDays.map((day, index) => {
          const isSelected = isSameDay(day.date, selectedDate)
          return (
            <div
              key={index}
              onClick={() => {
                onDateChange(day.date)
                onViewChange?.('day')
              }}
              title={locale.startsWith('sv') ? 'Klicka för att öppna dagsöversikt' : 'Click to view day overview'}
              className={cn(
                'min-h-[100px] cursor-pointer rounded-md border p-2 transition-all duration-150 flex flex-col',
                day.isCurrentMonth
                  ? 'bg-card/60'
                  : 'bg-muted/15 opacity-55',
                isSelected
                  ? 'border-white/20 bg-secondary/35 shadow-2xs'
                  : 'border-border/35 hover:border-border/60 hover:bg-secondary/20',
              )}
            >
              {/* Day number header */}
              <div className="mb-1.5 flex items-center justify-between">
                {day.isToday || isSelected ? (
                  <div
                    className={cn(
                      'flex h-6 w-6 items-center justify-center rounded-md bg-secondary text-xs font-semibold shadow-xs',
                      day.isToday
                        ? 'text-white'
                        : 'text-white',
                    )}
                  >
                    {day.dayOfMonth}
                  </div>
                ) : (
                  <span
                    className={cn(
                      'flex h-6 w-6 items-center text-xs px-1',
                      day.isCurrentMonth
                        ? 'text-foreground/80 font-normal'
                        : 'text-muted-foreground/45 font-light',
                    )}
                  >
                    {day.dayOfMonth}
                  </span>
                )}
              </div>

              {/* Events for this day */}
              <div className="flex-1 space-y-1 overflow-hidden">
                {day.events.slice(0, 3).map((event) => (
                  <MonthEventCard
                    key={event.id}
                    event={event}
                    onClick={(e: React.MouseEvent) => {
                      e.stopPropagation()
                      onEventClick(event)
                    }}
                    locale={locale}
                  />
                ))}
                {day.events.length > 3 && (
                  <div className="pl-1 text-[10px] font-mono text-muted-foreground/70">
                    +{day.events.length - 3} {locale.startsWith('sv') ? 'fler' : 'more'}
                  </div>
                )}
              </div>
            </div>
          )
        })}
      </div>
    </div>
  )
})

MonthView.displayName = 'MonthView'
