import React, { useState, useMemo, useRef, useEffect } from 'react'
import { useDroppable } from '@dnd-kit/core'
import { useWorkspace } from '@/contexts/WorkspaceContext'
import { cn, generateTimeSlots, isToday } from '@/lib/utils'
import type { CalendarEvent, User } from '@/types'
import { EventCard } from './EventCard'

interface DayViewProps {
  selectedDate: Date
  events: CalendarEvent[]
  users: User[]
  onEventClick: (event: CalendarEvent) => void
  editMode?: boolean
  onResize?: (event: CalendarEvent, updates: Partial<CalendarEvent>) => void
  hourHeight: number
  startHour?: number
  endHour?: number
}

export const DayView: React.FC<DayViewProps> = React.memo(({
  selectedDate,
  events,
  users,
  onEventClick,
  editMode,
  onResize,
  hourHeight,
  startHour: propStartHour,
  endHour: propEndHour,
}) => {
  const { settings } = useWorkspace()
  const scrollContainerRef = useRef<HTMLDivElement>(null)

  const { setNodeRef, isOver } = useDroppable({
    id: `column-${selectedDate.toISOString()}`,
    data: { type: 'column', date: selectedDate }
  })

  const startHour = propStartHour ?? settings.business_hours?.start ?? 0
  const endHour = propEndHour ?? settings.business_hours?.end ?? 23
  const timeSlots = useMemo(() => generateTimeSlots(startHour, endHour), [startHour, endHour])
  const HOUR_HEIGHT = hourHeight
  const locale = settings.language === 'sv' ? 'sv-SE' : 'en-US'

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

  const dayEvents = useMemo(() => {
    const dayStart = new Date(selectedDate)
    dayStart.setHours(0, 0, 0, 0)
    const dayEnd = new Date(selectedDate)
    dayEnd.setHours(23, 59, 59, 999)

    return events.filter((event) => {
      const eventStart = new Date(event.startTime)
      const eventEnd = new Date(event.endTime)
      return eventStart <= dayEnd && eventEnd >= dayStart
    })
  }, [events, selectedDate])

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
  const isCurrentTimeVisible = isToday(selectedDate) && nowHours >= startHour && nowHours <= endHour
  const currentTimeTop = isCurrentTimeVisible
    ? ((nowHours - startHour) + nowMinutes / 60) * HOUR_HEIGHT
    : null

  return (
    <div ref={scrollContainerRef} className="scrollbar-dark flex-1 overflow-auto bg-background">
      <div className="flex flex-col min-h-full min-w-full">
        {/* Sticky Header Row */}
        <div className="sticky top-0 z-30 flex border-b border-border bg-background/95 backdrop-blur-md shadow-xs">
          {/* Time header corner (matches w-16 time column width) */}
          <div className="flex w-16 flex-shrink-0 items-center justify-center border-r border-border bg-muted/30 py-2.5 text-[10px] font-semibold uppercase tracking-wider text-muted-foreground/70">
            {locale.startsWith('sv') ? 'Tid' : 'Time'}
          </div>

          {/* Day Header Info */}
          <div className="flex-1 px-4 py-2.5 text-center">
            <div
              className={cn(
                'text-[11px] font-semibold uppercase tracking-wider',
                isToday(selectedDate) ? 'font-bold text-primary' : 'text-muted-foreground'
              )}
            >
              {selectedDate.toLocaleDateString(locale, { weekday: 'long' })}
            </div>
            <div
              className="mt-0.5 mx-auto flex h-7 w-7 items-center justify-center rounded-md bg-primary text-primary-foreground font-medium shadow-xs text-base"
            >
              {selectedDate.getDate()}
            </div>
          </div>
        </div>

        {/* Grid Body */}
        <div className="relative flex flex-1">
          {/* Time column */}
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
            {currentTimeTop !== null && (
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

          {/* Day column with events */}
          <div
            ref={setNodeRef}
            className={cn(
              'relative flex-1 transition-colors',
              isToday(selectedDate) && 'bg-primary/[0.02]',
              editMode && isOver && 'bg-primary/5'
            )}
          >
            {/* Current time indicator line */}
            {currentTimeTop !== null && (
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

            {/* Hour grid lines with 15-min sub-lines */}
            {timeSlots.map((_, index) => (
              <div
                key={index}
                style={{ height: `${HOUR_HEIGHT}px` }}
                className="relative border-b border-border/60"
              >
                {/* Sub-hour lines (15, 30, 45 mins) */}
                <div className="pointer-events-none absolute left-0 right-0 top-[25%] border-b border-border/10" />
                <div className="pointer-events-none absolute left-0 right-0 top-[50%] border-b border-dashed border-border/25" />
                <div className="pointer-events-none absolute left-0 right-0 top-[75%] border-b border-border/10" />
              </div>
            ))}

            {/* Events container (flush top=0) */}
            <div className="absolute inset-0 z-10">
              {dayEvents.map((event) => (
                <EventCard
                  key={event.id}
                  event={event}
                  onClick={() => onEventClick(event)}
                  hourHeight={HOUR_HEIGHT}
                  startHour={startHour}
                  locale={locale}
                  tooltipPosition="top"
                  currentDate={selectedDate}
                  users={users}
                  editMode={editMode}
                  onResize={onResize}
                  onMouseEnter={() => setIsEventHovered(true)}
                  onMouseLeave={() => setIsEventHovered(false)}
                />
              ))}
            </div>
          </div>
        </div>
      </div>
    </div>
  )
})

DayView.displayName = 'DayView'
