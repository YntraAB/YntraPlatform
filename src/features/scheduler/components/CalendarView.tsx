/**
 * This is the main calendar/scheduler view component with support for
 * multiple view modes: Day, Week, Month, and Agenda.
 *
 * Each view mode provides a different perspective on the scheduled events,
 * allowing users to choose the most appropriate view for their needs.
 */

import React, { useMemo } from 'react'
import { useTranslation } from 'react-i18next'
import { useWorkspace } from '@/contexts/WorkspaceContext'
import {
  ChevronLeft,
  ChevronRight,
  Calendar as CalendarIcon,
  Pencil,
  PanelLeftClose,
  PanelLeftOpen,
  Clock,
} from 'lucide-react'
import { Button } from '@/components/ui/button'
import { cn } from '@/lib/utils'
import {
  DndContext,
  DragOverlay,
  PointerSensor,
  useSensor,
  useSensors,
  type DragEndEvent,
  type DragStartEvent,
  defaultDropAnimationSideEffects
} from '@dnd-kit/core'
import { restrictToFirstScrollableAncestor } from '@dnd-kit/modifiers'
import {
  formatTimeRange,
  generateWeekDays,
  getStartOfWeek,
  isToday,
  getCategoryConfig,
  formatMonthYear,
} from '@/lib/utils'
import type { CalendarEvent, CalendarView as ViewType, User } from '@/types'
import { DayView } from './calendar/DayView'
import { WeekView } from './calendar/WeekView'
import { MonthView } from './calendar/MonthView'
import { AgendaView } from './calendar/AgendaView'
import { UnscheduledBucket } from './calendar/UnscheduledBucket'

const VIEW_MODES: ViewType[] = ['day', 'week', 'month', 'agenda']

/**
 * Main CalendarView Component
 * Renders the appropriate view based on the current view mode
 */
interface CalendarViewProps {
  selectedDate: Date
  selectedEndDate?: Date | null
  view: ViewType
  events: CalendarEvent[]
  users: User[]
  onDateChange: (date: Date) => void
  onViewChange: (view: ViewType) => void
  onEventClick: (event: CalendarEvent) => void
  onEventUpdate: (eventId: string, updates: Partial<CalendarEvent>) => void
  onNext: () => void
  onPrevious: () => void
  onToday: () => void
  isLeftPanelOpen?: boolean
  onToggleLeftPanel?: () => void
}

export const CalendarView: React.FC<CalendarViewProps> = ({
  selectedDate,
  selectedEndDate,
  view,
  events,
  users,
  onDateChange,
  onViewChange,
  onEventClick,
  onEventUpdate,
  onNext,
  onPrevious,
  onToday,
  isLeftPanelOpen,
  onToggleLeftPanel,
}) => {
  const { settings, preferences } = useWorkspace()
  const [editMode, setEditMode] = React.useState(false)
  const [is24hMode, setIs24hMode] = React.useState(true)
  const [activeEvent, setActiveEvent] = React.useState<CalendarEvent | null>(null)
  const [currentDragTime, setCurrentDragTime] = React.useState<Date | null>(null)
  const [zoomLevel, setZoomLevel] = React.useState(preferences.calendar_density === 'compact' ? 40 : 60)

  const sensors = useSensors(
    useSensor(PointerSensor, {
      activationConstraint: {
        distance: 8,
      },
    })
  )

  // Hantera CTRL + Scroll & CTRL + +/- för zoom
  React.useEffect(() => {
    const handleWheel = (e: WheelEvent) => {
      if (e.ctrlKey) {
        e.preventDefault()
        setZoomLevel(prev => {
          const delta = e.deltaY > 0 ? -10 : 10
          return Math.max(40, Math.min(160, prev + delta))
        })
      }
    }

    const handleKeyDown = (e: KeyboardEvent) => {
      if (e.ctrlKey && (e.key === '+' || e.key === '=' || e.key === '-')) {
        e.preventDefault()
        setZoomLevel(prev => {
          const delta = e.key === '-' ? -10 : 10
          return Math.max(40, Math.min(160, prev + delta))
        })
      }
    }

    document.addEventListener('wheel', handleWheel, { passive: false })
    document.addEventListener('keydown', handleKeyDown)

    return () => {
      document.removeEventListener('wheel', handleWheel)
      document.removeEventListener('keydown', handleKeyDown)
    }
  }, [])

  const { t } = useTranslation()
  const locale = settings.language === 'sv' ? 'sv-SE' : 'en-US'
  const startHour = is24hMode ? 0 : (settings.business_hours?.start ?? 0)
  const endHour = is24hMode ? 23 : (settings.business_hours?.end ?? 23)
  const HOUR_HEIGHT = zoomLevel

  const [currentHeaderTime, setCurrentHeaderTime] = React.useState(() => new Date())
  React.useEffect(() => {
    const timer = setInterval(() => setCurrentHeaderTime(new Date()), 30000)
    return () => clearInterval(timer)
  }, [])

  const unscheduledEvents = useMemo(() => {
    return events.filter(e => e.isUnscheduled)
  }, [events])

  const gridEvents = useMemo(() => {
    return events.filter(e => !e.isUnscheduled)
  }, [events])

  const handleDragStart = (event: DragStartEvent) => {
    const { active } = event
    const draggedEvent = active.data.current?.event as CalendarEvent
    setActiveEvent(draggedEvent)
    setCurrentDragTime(draggedEvent.startTime)
  }

  const handleDragMove = (event: any) => {
    const { delta, over, active } = event
    if (!over || !activeEvent) return

    const dropData = over.data.current
    if (dropData?.type === 'column') {
      const dropDate = dropData.date as Date
      let newStartTime = new Date(dropDate)

      if (active.data.current?.isFromBucket) {
        newStartTime.setHours(startHour, 0, 0, 0)
      } else {
        const minutesDelta = (delta.y / HOUR_HEIGHT) * 60
        const snappedMinutesDelta = Math.round(minutesDelta / 15) * 15
        newStartTime = new Date(activeEvent.startTime.getTime() + snappedMinutesDelta * 60 * 1000)
        newStartTime.setFullYear(dropDate.getFullYear(), dropDate.getMonth(), dropDate.getDate())
      }
      setCurrentDragTime(newStartTime)
    }
  }

  const handleDragEnd = (event: DragEndEvent) => {
    setActiveEvent(null)
    setCurrentDragTime(null)
    const { active, over } = event

    if (!over) return

    const draggedEvent = active.data.current?.event as CalendarEvent
    const dropData = over.data.current

    if (dropData?.type === 'column') {
      const dropDate = dropData.date as Date
      const { y } = event.delta

      let newStartTime = new Date(dropDate)
      let duration = draggedEvent.endTime.getTime() - draggedEvent.startTime.getTime()

      if (active.data.current?.isFromBucket) {
        newStartTime.setHours(startHour, 0, 0, 0)
      } else {
        const minutesDelta = (y / HOUR_HEIGHT) * 60
        const snappedMinutesDelta = Math.round(minutesDelta / 15) * 15
        newStartTime = new Date(draggedEvent.startTime.getTime() + snappedMinutesDelta * 60 * 1000)
        newStartTime.setFullYear(dropDate.getFullYear(), dropDate.getMonth(), dropDate.getDate())
      }

      const newEndTime = new Date(newStartTime.getTime() + duration)

      onEventUpdate(draggedEvent.id, {
        startTime: newStartTime,
        endTime: newEndTime,
        isUnscheduled: false
      })
    }
  }

  const handleResize = (event: CalendarEvent, updates: Partial<CalendarEvent>) => {
    onEventUpdate(event.id, updates)
  }

  const weekDays = useMemo(() => {
    if (selectedEndDate && selectedEndDate >= selectedDate && view === 'week') {
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
  }, [selectedDate, selectedEndDate, view, settings.week_start, locale])

  /**
   * Format the date range for the header based on current view
   */
  const formatDateRange = () => {
    switch (view) {
      case 'day':
        return selectedDate.toLocaleDateString(locale, {
          weekday: 'long',
          month: 'long',
          day: 'numeric',
          year: 'numeric',
        })

      case 'week': {
        if (!weekDays || weekDays.length === 0) {
          return selectedDate.toLocaleDateString(locale, { month: 'long', year: 'numeric' })
        }
        const start = weekDays[0].date
        const end = weekDays[weekDays.length - 1].date
        if (start.getMonth() === end.getMonth()) {
          return `${start.toLocaleDateString(locale, { month: 'long' })} ${start.getDate()} - ${end.getDate()}, ${start.getFullYear()}`
        } else if (start.getFullYear() === end.getFullYear()) {
          return `${start.toLocaleDateString(locale, { month: 'short', day: 'numeric' })} - ${end.toLocaleDateString(locale, { month: 'short', day: 'numeric' })}, ${start.getFullYear()}`
        } else {
          return `${start.toLocaleDateString(locale, { month: 'short', day: 'numeric', year: 'numeric' })} - ${end.toLocaleDateString(locale, { month: 'short', day: 'numeric', year: 'numeric' })}`
        }
      }

      case 'month':
        return formatMonthYear(selectedDate, locale)

      case 'agenda':
        return t('scheduler.upcoming_events')

      default:
        return ''
    }
  }

  /**
   * Render the appropriate view based on current view mode
   */
  const renderView = () => {
    switch (view) {
      case 'day':
        return (
          <DayView
            selectedDate={selectedDate}
            events={gridEvents}
            users={users}
            onEventClick={onEventClick}
            editMode={editMode}
            onResize={handleResize}
            hourHeight={HOUR_HEIGHT}
            startHour={startHour}
            endHour={endHour}
          />
        )

      case 'week':
        return (
          <WeekView
            selectedDate={selectedDate}
            selectedEndDate={selectedEndDate}
            events={gridEvents}
            users={users}
            onEventClick={onEventClick}
            onDateChange={onDateChange}
            onViewChange={onViewChange}
            editMode={editMode}
            onResize={handleResize}
            hourHeight={HOUR_HEIGHT}
            startHour={startHour}
            endHour={endHour}
          />
        )

      case 'month':
        return (
          <MonthView
            selectedDate={selectedDate}
            events={gridEvents}
            users={users}
            onEventClick={onEventClick}
            onDateChange={onDateChange}
            onViewChange={onViewChange}
          />
        )

      case 'agenda':
        return <AgendaView events={gridEvents} users={users} onEventClick={onEventClick} />

      default:
        return null
    }
  }

  return (
    <DndContext
      sensors={sensors}
      onDragStart={handleDragStart}
      onDragMove={handleDragMove}
      onDragEnd={handleDragEnd}
      modifiers={[restrictToFirstScrollableAncestor]}
    >
      <div className="flex h-full flex-1 overflow-hidden bg-background">
        <UnscheduledBucket events={unscheduledEvents} editMode={editMode} />

        <div className="flex flex-col flex-1 min-w-0">
          {/* Header Toolbar */}
          <div className="flex h-12 flex-wrap items-center justify-between gap-3 border-b border-border/50 px-6 bg-background/95 backdrop-blur-sm z-50 sticky top-0">
            {/* Left side - Navigation, panel toggle and date display */}
            <div className="flex flex-wrap items-center gap-2 sm:gap-3">
              {onToggleLeftPanel && (
                <Button
                  variant="ghost"
                  size="sm"
                  className="h-8 w-8 p-0 text-muted-foreground hover:text-foreground"
                  onClick={onToggleLeftPanel}
                  title={isLeftPanelOpen ? t('scheduler.hide_panel', 'Dölj panel') : t('scheduler.show_panel', 'Visa panel')}
                >
                  {isLeftPanelOpen ? (
                    <PanelLeftClose className="h-3.5 w-3.5" />
                  ) : (
                    <PanelLeftOpen className="h-3.5 w-3.5" />
                  )}
                </Button>
              )}

              {/* Navigation buttons */}
              <div className="flex items-center gap-0.5">
                <Button
                  variant="ghost"
                  size="icon"
                  onClick={onPrevious}
                  className="h-8 w-8 rounded-md"
                >
                  <ChevronLeft className="h-3.5 w-3.5 text-muted-foreground" />
                </Button>
                <Button
                  variant="ghost"
                  size="sm"
                  onClick={onToday}
                  className="h-8 px-2.5 text-xs font-medium text-foreground"
                  title={`${t('common.today')} • ${t('common.current_time', 'Just nu')}: ${currentHeaderTime.toLocaleTimeString(locale, { hour: '2-digit', minute: '2-digit', hour12: false })}`}
                >
                  {t('common.today')}
                </Button>
                <Button
                  variant="ghost"
                  size="icon"
                  onClick={onNext}
                  className="h-8 w-8 rounded-md"
                >
                  <ChevronRight className="h-3.5 w-3.5 text-muted-foreground" />
                </Button>
              </div>

              {/* Date display */}
              <div className="flex items-center gap-1.5 pl-1">
                <CalendarIcon className="h-3.5 w-3.5 text-muted-foreground" />
                <span className="text-xs font-medium text-foreground">{formatDateRange()}</span>
              </div>
            </div>

            {/* Right side - Edit Mode & View mode selector */}
            <div className="flex items-center gap-2 sm:gap-3">
              {/* 24h / Business hours toggle for day and week views */}
              {(view === 'day' || view === 'week') && (
                <Button
                  variant="outline"
                  size="sm"
                  onClick={() => setIs24hMode(!is24hMode)}
                  className={cn(
                    'h-8 gap-1.5 px-2.5 text-xs font-medium transition-colors',
                    is24hMode
                      ? 'border-primary/40 bg-primary/10 text-primary hover:bg-primary/15'
                      : 'text-muted-foreground hover:text-foreground'
                  )}
                  title={is24hMode ? t('scheduler.switch_to_business_hours', 'Växla till arbetstid') : t('scheduler.switch_to_24h', 'Växla till heldygn (24h)')}
                >
                  <Clock className="h-3.5 w-3.5" />
                  <span>{is24hMode ? '24h' : t('scheduler.business_hours', 'Arbetstid')}</span>
                </Button>
              )}

              {/* Edit Mode Toggle */}
              <Button
                variant={editMode ? 'default' : 'secondary'}
                size="sm"
                onClick={() => setEditMode(!editMode)}
                className="h-8 gap-1.5 text-xs font-medium"
              >
                <Pencil className={cn('h-3.5 w-3.5', editMode && 'animate-pulse')} />
                <span className="hidden sm:inline">
                  {editMode ? t('scheduler.exit_edit_mode') : t('scheduler.edit_mode')}
                </span>
              </Button>

              <div className="flex items-center rounded-md bg-secondary p-0.5">
                {VIEW_MODES.map((modeId) => (
                  <button
                    key={modeId}
                    onClick={() => onViewChange(modeId)}
                    className={cn(
                      'rounded-sm px-2.5 py-1 text-xs font-medium transition-all duration-150',
                      view === modeId
                        ? 'bg-background text-foreground shadow-xs'
                        : 'text-muted-foreground hover:text-foreground',
                    )}
                  >
                    {t(`scheduler.views.${modeId}`)}
                  </button>
                ))}
              </div>
            </div>
          </div>

          {/* Main content - renders the active view with integrated sticky header */}
          <div className="flex-1 overflow-hidden flex flex-col">
            {renderView()}
          </div>
        </div>
      </div>

      <DragOverlay dropAnimation={{
        sideEffects: defaultDropAnimationSideEffects({
          styles: {
            active: {
              opacity: '0.5',
            },
          },
        }),
      }}>
        {activeEvent ? (
          <div
            className="rounded-md px-3 py-2 text-xs shadow-sm brightness-110 pointer-events-none scale-105 transition-transform bg-background/80 backdrop-blur-md border border-primary/50 flex flex-col gap-1"
            style={{
              width: '220px',
              borderLeft: `4px solid ${getCategoryConfig(activeEvent.category).color}`,
            }}
          >
            <div className="font-bold text-foreground flex items-center justify-between">
              <span className="truncate">{activeEvent.title}</span>
              {currentDragTime && (
                <span className="bg-primary/20 text-primary px-1.5 py-0.5 rounded text-[10px]">
                  {currentDragTime.toLocaleTimeString(locale, { hour: '2-digit', minute: '2-digit', hour12: false })}
                </span>
              )}
            </div>
            <div className="text-[10px] text-muted-foreground">
              {currentDragTime && formatTimeRange(currentDragTime, new Date(currentDragTime.getTime() + (activeEvent.endTime.getTime() - activeEvent.startTime.getTime())), locale)}
            </div>
          </div>
        ) : null}
      </DragOverlay>
    </DndContext>
  )
}

export default CalendarView
