import React, { useState, useMemo, useRef, useEffect } from 'react'
import { useDroppable } from '@dnd-kit/core'
import { useWorkspace } from '@/contexts/WorkspaceContext'
import {
  cn,
  generateTimeSlots,
  generateWeekDays,
  getStartOfWeek,
  isSameDay,
  isToday,
} from '@/lib/utils'
import type { CalendarEvent, CalendarView, User } from '@/types'
import { EventCard } from './EventCard'

interface WeekViewProps {
  selectedDate: Date
  selectedEndDate?: Date | null
  events: CalendarEvent[]
  users: User[]
  onEventClick: (event: CalendarEvent) => void
  onDateChange?: (date: Date) => void
  onViewChange?: (view: CalendarView) => void
  editMode?: boolean
  onResize?: (event: CalendarEvent, updates: Partial<CalendarEvent>) => void
  hourHeight: number
  startHour?: number
  endHour?: number
}

export const WeekView: React.FC<WeekViewProps> = React.memo(({
  selectedDate,
  selectedEndDate,
  events,
  users,
  onEventClick,
  onDateChange,
  onViewChange,
  editMode,
  onResize,
  hourHeight,
  startHour: propStartHour,
  endHour: propEndHour,
}) => {
  const { settings } = useWorkspace()
  const scrollContainerRef = useRef<HTMLDivElement>(null)

  const startHour = propStartHour ?? settings.business_hours?.start ?? 0
  const endHour = propEndHour ?? settings.business_hours?.end ?? 23
  const timeSlots = useMemo(() => generateTimeSlots(startHour, endHour), [startHour, endHour])
  const HOUR_HEIGHT = hourHeight
  const locale = settings.language === 'sv' ? 'sv-SE' : 'en-US'

  const weekDays = useMemo(() => {
    if (selectedEndDate && selectedEndDate >= selectedDate) {
      const days = []
      const current = new Date(selectedDate)
      while (current <= selectedEndDate) {
        days.push({
          date: new Date(current),
          name: current.toLocaleDateString(locale, { weekday: 'short' }),
          dayOfMonth: current.getDate(),
          isToday: isToday(current),
          isWeekend: current.getDay() === 0 || current.getDay() === 6,
        })
        current.setDate(current.getDate() + 1)
      }
      if (days.length > 0) return days
    }
    const weekStart = getStartOfWeek(selectedDate, settings.week_start)
    return generateWeekDays(weekStart, locale)
  }, [selectedDate, selectedEndDate, settings.week_start, locale])

  // Center or scroll to current time on mount
  const hasScrolledRef = useRef(false)
  useEffect(() => {
    if (!hasScrolledRef.current && scrollContainerRef.current) {
      const now = new Date()
      const currentH = now.getHours()
      const targetHour = Math.max(startHour, Math.min(endHour, currentH - 1))
      const scrollPosition = (targetHour - startHour) * HOUR_HEIGHT
      scrollContainerRef.current.scrollTop = Math.max(0, scrollPosition)
      hasScrolledRef.current = true
    }
  }, [startHour, endHour, HOUR_HEIGHT])

  const getEventsForDay = (date: Date) => {
    const dayStart = new Date(date)
    dayStart.setHours(0, 0, 0, 0)
    const dayEnd = new Date(date)
    dayEnd.setHours(23, 59, 59, 999)

    return events.filter((event) => {
      const eventStart = new Date(event.startTime)
      const eventEnd = new Date(event.endTime)
      return eventStart <= dayEnd && eventEnd >= dayStart
    })
  }

  const [currentTime, setCurrentTime] = React.useState(() => new Date())
  const [isTimeHovered, setIsTimeHovered] = useState(false)
  const [isEventHovered, setIsEventHovered] = useState(false)
  const lineTrackRef = useRef<HTMLDivElement>(null)

  const isHighlighted = isTimeHovered || isEventHovered

  useEffect(() => {
    const timer = setInterval(() => {
      setCurrentTime(new Date())
    }, 30000)
    return () => clearInterval(timer)
  }, [])

  const now = currentTime
  const nowHours = now.getHours()
  const nowMinutes = now.getMinutes()
  const hasTodayInWeek = weekDays.some((d) => d.isToday)
  const isCurrentTimeVisible = hasTodayInWeek && nowHours >= startHour && nowHours <= endHour
  const currentTimeTop = isCurrentTimeVisible
    ? ((nowHours - startHour) + nowMinutes / 60) * HOUR_HEIGHT
    : null

  return (
    <div ref={scrollContainerRef} className="scrollbar-dark flex-1 overflow-auto bg-background">
      <div className="flex min-w-[750px] flex-col min-h-full">
        {/* Sticky Header Row: time corner + day headers in exact lockstep */}
        <div className="sticky top-0 z-30 flex border-b border-border bg-background/95 backdrop-blur-md shadow-xs">
          {/* Time header corner (matches w-16 time column width) */}
          <div className="flex w-16 flex-shrink-0 items-center justify-center border-r border-border bg-muted/30 py-2.5 text-[10px] font-semibold uppercase tracking-wider text-muted-foreground/70">
            {locale.startsWith('sv') ? 'Tid' : 'Time'}
          </div>

          {/* Day column headers */}
          <div
            className="grid flex-1 divide-x divide-border"
            style={{ gridTemplateColumns: `repeat(${weekDays.length}, minmax(0, 1fr))` }}
          >
            {weekDays.map((day, index) => {
              const isSelected = isSameDay(day.date, selectedDate)
              return (
                <button
                  type="button"
                  key={index}
                  onClick={() => {
                    onDateChange?.(day.date)
                    onViewChange?.('day')
                  }}
                  title={locale.startsWith('sv') ? `Visa ${day.name} ${day.dayOfMonth}` : `View ${day.name} ${day.dayOfMonth}`}
                  className={cn(
                    'group px-2 py-2.5 text-center transition-colors cursor-pointer hover:bg-muted/40 focus:outline-none',
                    day.isWeekend && 'bg-muted/15',
                    isSelected && !day.isToday && 'bg-secondary/30',
                    day.isToday && 'bg-primary/[0.04]'
                  )}
                >
                  <div
                    className={cn(
                      'text-[11px] font-medium uppercase tracking-wider transition-colors',
                      day.isToday
                        ? 'text-primary font-semibold'
                        : isSelected
                          ? 'text-foreground font-semibold'
                          : 'text-muted-foreground group-hover:text-foreground'
                    )}
                  >
                    {day.name}
                  </div>
                  <div
                    className={cn(
                      'mt-0.5 text-sm font-medium transition-transform group-hover:scale-105',
                      day.isToday || isSelected
                        ? 'mx-auto flex h-7 w-7 items-center justify-center rounded-md bg-secondary text-white font-semibold shadow-xs'
                        : 'text-foreground'
                    )}
                  >
                    {day.dayOfMonth}
                  </div>
                </button>
              )
            })}
          </div>
        </div>

        {/* Grid Body */}
        <div className="relative flex flex-1">
          {/* Time gutter column */}
          <div className="relative w-16 flex-shrink-0 border-r border-border bg-muted/20 select-none">
            {timeSlots.map((slot, index) => (
              <div
                key={index}
                style={{ height: `${HOUR_HEIGHT}px` }}
                className="relative border-b border-border/60"
              >
                <span className="absolute -top-2.5 right-2 font-mono text-[10px] font-medium text-muted-foreground/80">
                  {slot.label}
                </span>
              </div>
            ))}

            {/* Current time indicator in the gutter (clean, no background, no dots) */}
            {hasTodayInWeek && currentTimeTop !== null && (
              <div
                data-testid="timeline-gutter-badge"
                style={{ top: `${currentTimeTop}px` }}
                className="absolute right-1 z-20 flex items-center -translate-y-1/2 pointer-events-auto cursor-pointer select-none"
                onMouseEnter={() => setIsTimeHovered(true)}
                onMouseLeave={() => setIsTimeHovered(false)}
                title={`${locale.startsWith('sv') ? 'Just nu' : 'Now'}: ${now.toLocaleTimeString(locale, { hour: '2-digit', minute: '2-digit', hour12: false })}`}
              >
                <span
                  className={cn(
                    'font-mono text-[10px] transition-colors duration-150',
                    isHighlighted
                      ? 'font-bold text-red-600 dark:text-red-400'
                      : 'font-medium text-red-500/40 dark:text-red-400/40'
                  )}
                >
                  {now.toLocaleTimeString(locale, { hour: '2-digit', minute: '2-digit', hour12: false })}
                </span>
              </div>
            )}
          </div>

          {/* Day columns */}
          <div
            className="relative grid flex-1 divide-x divide-border"
            style={{ gridTemplateColumns: `repeat(${weekDays.length}, minmax(0, 1fr))` }}
          >
            {/* Full-width current time indicator line spanning across the entire week */}
            {hasTodayInWeek && currentTimeTop !== null && (
              <div
                className="pointer-events-none absolute left-0 right-0 z-20 flex items-center h-4 -my-2"
                style={{ top: `${currentTimeTop}px` }}
              >
                {/* Line track with click-through and hover styling */}
                <div
                  ref={lineTrackRef}
                  data-testid="timeline-track"
                  className="relative flex-1 h-full flex items-center pointer-events-auto cursor-pointer"
                  onMouseEnter={() => setIsTimeHovered(true)}
                  onMouseLeave={() => setIsTimeHovered(false)}
                  onClick={(e) => {
                    const target = e.currentTarget
                    target.style.pointerEvents = 'none'
                    const under = document.elementFromPoint(e.clientX, e.clientY) as HTMLElement | null
                    under?.click()
                    target.style.pointerEvents = 'auto'
                  }}
                >
                  <div
                    className={cn(
                      'w-full transition-all duration-150',
                      isHighlighted
                        ? 'h-[2px] bg-red-500 shadow-xs opacity-100'
                        : 'h-[1.5px] bg-red-500/30 opacity-70'
                    )}
                  />
                </div>
              </div>
            )}

            {/* Grid lines and events for each day */}
            {weekDays.map((day, dayIndex) => {
              const isSelected = isSameDay(day.date, selectedDate)
              return (
                <div
                  key={dayIndex}
                  className={cn(
                    'relative transition-colors',
                    day.isWeekend && 'bg-muted/15',
                    isSelected && !day.isToday && 'bg-primary/[0.015]',
                    day.isToday && 'bg-primary/[0.02]'
                  )}
                >

                {/* Hour grid lines with 15-min sub-lines */}
                {timeSlots.map((_, hourIndex) => (
                  <div
                    key={hourIndex}
                    style={{ height: `${HOUR_HEIGHT}px` }}
                    className="relative border-b border-border/60"
                  >
                    {/* Sub-hour lines (15, 30, 45 mins) */}
                    <div className="pointer-events-none absolute left-0 right-0 top-[25%] border-b border-border/10" />
                    <div className="pointer-events-none absolute left-0 right-0 top-[50%] border-b border-dashed border-border/25" />
                    <div className="pointer-events-none absolute left-0 right-0 top-[75%] border-b border-border/10" />
                  </div>
                ))}

                {/* Events for this day */}
                <DroppableWeekColumn
                  date={day.date}
                  editMode={editMode || false}
                >
                  {getEventsForDay(day.date).map((event) => (
                    <EventCard
                      key={event.id}
                      event={event}
                      onClick={() => onEventClick(event)}
                      hourHeight={HOUR_HEIGHT}
                      startHour={startHour}
                      locale={locale}
                      tooltipPosition="side"
                      currentDate={day.date}
                      users={users}
                      editMode={editMode}
                      onResize={onResize}
                      onMouseEnter={() => setIsEventHovered(true)}
                      onMouseLeave={() => setIsEventHovered(false)}
                    />
                  ))}
                </DroppableWeekColumn>
              </div>
            )
          })}
          </div>
        </div>
      </div>
    </div>
  )
})

const DroppableWeekColumn: React.FC<{ date: Date, editMode: boolean, children: React.ReactNode }> = ({ date, editMode, children }) => {
  const { setNodeRef, isOver } = useDroppable({
    id: `column-${date.toISOString()}`,
    data: { type: 'column', date }
  })

  return (
    <div
      ref={setNodeRef}
      className={cn(
        'absolute inset-0 z-10',
        editMode && isOver && 'bg-primary/5'
      )}
    >
      {children}
    </div>
  )
}

WeekView.displayName = 'WeekView'
