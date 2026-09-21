/**
 * A compact monthly calendar widget for quick date navigation.
 * Displays the current month with day selection and event indicators.
 */

import React, { useMemo } from 'react'
import { ChevronLeft, ChevronRight } from 'lucide-react'
import { useTranslation } from 'react-i18next'
import { cn, formatMonthYear, isSameDay, isToday } from '@/lib/utils'
import { useWorkspace } from '@/contexts/WorkspaceContext'

interface MiniCalendarProps {
  selectedDate: Date
  selectedEndDate?: Date | null
  onSelectDate: (date: Date, isShiftClick?: boolean) => void
  onNewEvent?: () => void
  datesWithEvents?: Date[]
}

/**
 * MiniCalendar Component
 * Compact monthly calendar for sidebar navigation
 */
export const MiniCalendar: React.FC<MiniCalendarProps> = ({
  selectedDate,
  selectedEndDate = null,
  onSelectDate,
  onNewEvent,
  datesWithEvents = [],
}) => {
  const { settings } = useWorkspace()
  const { t } = useTranslation()
  const locale = settings.language === 'sv' ? 'sv-SE' : 'en-US'

  const [displayDate, setDisplayDate] = React.useState<Date>(selectedDate)

  React.useEffect(() => {
    setDisplayDate(selectedDate)
  }, [selectedDate])

  /**
   * Generate the calendar grid data
   * Returns days for the current month view including padding days
   */
  const calendarDays = useMemo(() => {
    const year = displayDate.getFullYear()
    const month = displayDate.getMonth()
    const firstDayOfMonth = new Date(year, month, 1)
    const lastDayOfMonth = new Date(year, month + 1, 0)
    const startDayOfWeek = firstDayOfMonth.getDay()
    const diffToStart =
      (startDayOfWeek < settings.week_start ? 7 : 0) + startDayOfWeek - settings.week_start
    const daysInMonth = lastDayOfMonth.getDate()

    const days: Array<{
      date: Date
      dayOfMonth: number
      isCurrentMonth: boolean
      isToday: boolean
      hasEvents: boolean
    }> = []

    const daysInPrevMonth = new Date(year, month, 0).getDate()
    for (let i = diffToStart - 1; i >= 0; i--) {
      const date = new Date(year, month - 1, daysInPrevMonth - i)
      days.push({
        date,
        dayOfMonth: daysInPrevMonth - i,
        isCurrentMonth: false,
        isToday: isToday(date),
        hasEvents: datesWithEvents.some((d) => isSameDay(d, date)),
      })
    }

    for (let day = 1; day <= daysInMonth; day++) {
      const date = new Date(year, month, day)
      days.push({
        date,
        dayOfMonth: day,
        isCurrentMonth: true,
        isToday: isToday(date),
        hasEvents: datesWithEvents.some((d) => isSameDay(d, date)),
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
        hasEvents: datesWithEvents.some((d) => isSameDay(d, date)),
      })
    }

    return days
  }, [displayDate, datesWithEvents, settings.week_start])

  /**
   * Navigate to the previous month in the mini calendar view
   */
  const goToPreviousMonth = () => {
    setDisplayDate((prev) => {
      const d = new Date(prev)
      d.setDate(1)
      d.setMonth(d.getMonth() - 1)
      return d
    })
  }

  /**
   * Navigate to the next month in the mini calendar view
   */
  const goToNextMonth = () => {
    setDisplayDate((prev) => {
      const d = new Date(prev)
      d.setDate(1)
      d.setMonth(d.getMonth() + 1)
      return d
    })
  }

  const isSv = settings.language === 'sv'
  const dayLabels =
    settings.week_start === 1
      ? isSv
        ? ['M', 'T', 'O', 'T', 'F', 'L', 'S']
        : ['M', 'T', 'W', 'T', 'F', 'S', 'S']
      : isSv
        ? ['S', 'M', 'T', 'O', 'T', 'F', 'L']
        : ['S', 'M', 'T', 'W', 'T', 'F', 'S']

  return (
    <div className="rounded-lg bg-sidebar p-4">
      {/* Header with month/year and navigation */}
      <div className="mb-4 flex items-center justify-between">
        <button
          onClick={goToPreviousMonth}
          className="rounded p-1 transition-colors hover:bg-muted"
        >
          <ChevronLeft className="h-4 w-4 text-muted-foreground" />
        </button>

        <span className="text-sm font-medium text-foreground">
          {formatMonthYear(displayDate, locale)}
        </span>

        <button onClick={goToNextMonth} className="rounded p-1 transition-colors hover:bg-muted">
          <ChevronRight className="h-4 w-4 text-muted-foreground" />
        </button>
      </div>

      {/* Day labels */}
      <div className="mb-2 grid grid-cols-7 gap-1">
        {dayLabels.map((label, index) => (
          <div key={index} className="py-1 text-center text-xs font-medium text-muted-foreground">
            {label}
          </div>
        ))}
      </div>

      {/* Calendar grid */}
      <div className="grid grid-cols-7 gap-1">
        {calendarDays.map((day, index) => (
          <button
            key={index}
            onClick={(e) => onSelectDate(day.date, e.shiftKey)}
            className={cn(
              'relative mx-auto flex h-8 w-8 items-center justify-center rounded-full',
              'text-sm transition-all duration-150',
              day.isCurrentMonth && 'text-foreground hover:bg-muted',
              !day.isCurrentMonth && 'text-muted-foreground/50',
              selectedEndDate &&
                day.date > selectedDate &&
                day.date < selectedEndDate &&
                'bg-primary/20 text-foreground',
              (isSameDay(day.date, selectedDate) ||
                (selectedEndDate && isSameDay(day.date, selectedEndDate))) &&
                'bg-primary text-primary-foreground font-medium hover:bg-primary/90 shadow-xs',
              day.isToday &&
                !isSameDay(day.date, selectedDate) &&
                (!selectedEndDate || !isSameDay(day.date, selectedEndDate)) &&
                'text-primary ring-1 ring-primary font-medium',
            )}
          >
            {day.dayOfMonth}

            {/* Event indicator dot */}
            {day.hasEvents && !isSameDay(day.date, selectedDate) && (
              <span className="absolute bottom-1 left-1/2 h-1 w-1 -translate-x-1/2 rounded-full bg-primary" />
            )}
          </button>
        ))}
      </div>

      <button
        onClick={onNewEvent}
        className="mt-4 flex w-full items-center justify-center gap-2 rounded-md bg-muted px-4 py-2 text-sm font-medium text-foreground transition-colors hover:bg-muted"
      >
        <span className="text-primary">+</span>
        {t('scheduler.new_event')}
      </button>
    </div>
  )
}

export default MiniCalendar
