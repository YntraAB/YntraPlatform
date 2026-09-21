import React from 'react'
import { useTranslation } from 'react-i18next'
import { MapPin, GripVertical, Calendar as CalendarIcon } from 'lucide-react'
import { useDraggable } from '@dnd-kit/core'
import { cn } from '@/lib/utils'
import {
  formatTimeRange,
  getCategoryConfig,
  calculateEventTop,
  calculateEventHeight,
} from '@/lib/utils'
import type { CalendarEvent, User } from '@/types'

interface EventCardProps {
  event: CalendarEvent
  onClick: () => void
  hourHeight?: number
  startHour?: number
  locale?: string
  tooltipPosition?: 'side' | 'top'
  currentDate: Date
  users?: User[]
  editMode?: boolean
  onResize?: (event: CalendarEvent, updates: Partial<CalendarEvent>) => void
  onMouseEnter?: () => void
  onMouseLeave?: () => void
}

export const EventCard: React.FC<EventCardProps> = React.memo(({
  event,
  onClick,
  hourHeight = 60,
  startHour = 0,
  locale = 'en-US',
  tooltipPosition = 'side',
  currentDate,
  users = [],
  editMode = false,
  onResize,
  onMouseEnter,
  onMouseLeave,
}) => {
  const { attributes, listeners, setNodeRef, transform, isDragging } = useDraggable({
    id: `event-${event.id}`,
    data: { type: 'event', event },
    disabled: !editMode,
  })
  const { t } = useTranslation()
  const categoryConfig = getCategoryConfig(event.category)

  // Helper to format assignee name
  const getDisplayName = () => {
    if (!event.assigneeId) return event.title
    const user = users.find((u) => u.id === event.assigneeId)
    if (!user) return event.title

    const fullName = user.name || (user as unknown as { full_name?: string }).full_name || ''
    if (!fullName) return event.title

    const parts = fullName.trim().split(/\s+/)
    const firstName = parts[0]
    if (parts.length === 1) return firstName

    const lastName = parts[parts.length - 1]
    // If "First Last" is longer than 12 chars, use "First L."
    if (`${firstName} ${lastName}`.length > 12) {
      return `${firstName} ${lastName.charAt(0)}.`
    }
    return `${firstName} ${lastName}`
  }

  const displayName = getDisplayName()

  const [tempStartTime, setTempStartTime] = React.useState<Date | null>(null)
  const [tempEndTime, setTempEndTime] = React.useState<Date | null>(null)

  const dayStart = new Date(currentDate)
  dayStart.setHours(0, 0, 0, 0)
  const dayEnd = new Date(currentDate)
  dayEnd.setHours(23, 59, 59, 999)

  const displayStart = tempStartTime || event.startTime
  const displayEnd = tempEndTime || event.endTime

  const effectiveStart = displayStart < dayStart ? dayStart : displayStart
  const effectiveEnd = displayEnd > dayEnd ? dayEnd : displayEnd

  const top = calculateEventTop(effectiveStart, hourHeight, startHour)
  const height = calculateEventHeight(effectiveStart, effectiveEnd, hourHeight)

  return (
    <div
      ref={setNodeRef}
      {...attributes}
      onClick={editMode ? undefined : onClick}
      onMouseEnter={onMouseEnter}
      onMouseLeave={onMouseLeave}
      className={cn(
        "group absolute left-1 right-1 z-10 cursor-pointer rounded-md text-xs transition-all duration-200 hover:z-50 hover:shadow-sm hover:brightness-105",
        isDragging && "opacity-50 grayscale",
        editMode && "ring-1 ring-primary/30"
      )}
      style={{
        top: `${top}px`,
        height: `${Math.max(height - 2, 24)}px`,
        backgroundColor: categoryConfig.bgColor,
        borderLeft: `3px solid ${categoryConfig.color}`,
        transform: transform ? `translate3d(${transform.x}px, ${transform.y}px, 0)` : undefined,
      }}
    >
      {/* Drag Area - Covers the whole card but has lower z-index than handles */}
      <div
        {...(editMode ? listeners : {})}
        className="absolute inset-0 z-10 px-2 py-1.5"
      >
        {/* Event title */}
        <div style={{ color: categoryConfig.color }} className="font-semibold">{displayName}</div>

        {/* Event time (only show if height allows) */}
        {height > 35 && (
          <div className="mt-0.5 text-[10px] text-muted-foreground">
            {formatTimeRange(displayStart, displayEnd, locale)}
          </div>
        )}

        {/* Event location (only show if height allows) */}
        {height > 50 && event.location && (
          <div className="mt-0.5 flex items-center gap-1 text-[10px] text-muted-foreground">
            <MapPin className="h-3 w-3" />
            <span className="truncate">{event.location}</span>
          </div>
        )}
      </div>

      {/* Resize Handle - Top */}
      {editMode && (
        <div
          className="absolute -top-1 left-0 right-0 h-2 cursor-ns-resize z-50 group/top"
          onMouseDown={(e) => {
            e.stopPropagation();
            e.preventDefault();
            const startY = e.clientY;
            const originalStartTime = event.startTime.getTime();

            const handleMouseMove = (moveEvent: MouseEvent) => {
              const deltaY = moveEvent.clientY - startY;
              const snappedMinutesDelta = Math.round((deltaY / hourHeight) * 60 / 15) * 15;
              const newStartTime = new Date(originalStartTime + snappedMinutesDelta * 60 * 1000);

              if (newStartTime.getTime() >= event.endTime.getTime() - 15 * 60 * 1000) return;
              setTempStartTime(newStartTime);
            };

            const handleMouseUp = () => {
              window.removeEventListener('mousemove', handleMouseMove);
              window.removeEventListener('mouseup', handleMouseUp);

              setTempStartTime(prev => {
                if (prev) onResize?.(event, { startTime: prev });
                return null;
              });
            };

            window.addEventListener('mousemove', handleMouseMove);
            window.addEventListener('mouseup', handleMouseUp);
          }}
        >
          <div className="mx-auto w-8 h-1 bg-primary/40 rounded-full opacity-0 group-hover/top:opacity-100 transition-opacity" />
        </div>
      )}

      {editMode && (
        <div className="absolute -left-1 top-0 bottom-0 flex items-center z-50 opacity-0 group-hover:opacity-100 transition-opacity">
          <GripVertical className="h-3 w-3 text-muted-foreground" />
        </div>
      )}

      {/* Hover Information Popup */}
      <div
        className={cn(
          'absolute hidden w-64 p-4 group-hover:block',
          'z-[100] rounded-lg border border-border bg-background/95 shadow-sm backdrop-blur-md',
          'pointer-events-none duration-200 animate-in fade-in',
          tooltipPosition === 'top'
            ? 'bottom-full left-1/2 mb-2 -translate-x-1/2 slide-in-from-bottom-2'
            : 'left-full top-0 ml-2 zoom-in-95',
        )}
      >
        <div className="mb-2 flex items-center justify-between">
          <span
            className="rounded px-2 py-0.5 text-[10px] font-bold uppercase tracking-wider"
            style={{ backgroundColor: categoryConfig.bgColor, color: categoryConfig.color }}
          >
            {t(categoryConfig.label)}
          </span>
          <span className="flex items-center gap-1 text-[10px] text-muted-foreground">
            <CalendarIcon className="h-3 w-3" />
            {event.startTime.toLocaleDateString(locale, { month: 'short', day: 'numeric' })}
          </span>
        </div>

        <h4 className="mb-1 text-sm font-bold leading-tight text-foreground">{event.title}</h4>

        <div className="mb-3 flex flex-col gap-1 text-xs text-muted-foreground">
          <div className="flex items-center gap-1.5">
            <span
              className="h-1.5 w-1.5 rounded-full"
              style={{ backgroundColor: categoryConfig.color }}
            />
            {formatTimeRange(event.startTime, event.endTime, locale)}
          </div>
          {event.location && (
            <div className="flex items-center gap-1.5">
              <MapPin className="h-3 w-3 text-primary" />
              <span>{event.location}</span>
            </div>
          )}
        </div>

        {event.description && (
          <div className="mt-2 border-t border-border pt-2">
            <p className="text-[11px] italic leading-relaxed text-muted-foreground">
              {event.description}
            </p>
          </div>
        )}
      </div>

      {/* Resize Handle - Bottom */}
      {editMode && (
        <div
          className="absolute -bottom-1 left-0 right-0 h-2 cursor-ns-resize z-50 group/bottom"
          onMouseDown={(e) => {
            e.stopPropagation();
            e.preventDefault();
            const startY = e.clientY;
            const originalEndTime = event.endTime.getTime();

            const handleMouseMove = (moveEvent: MouseEvent) => {
              const deltaY = moveEvent.clientY - startY;
              const snappedMinutesDelta = Math.round((deltaY / hourHeight) * 60 / 15) * 15;
              const newEndTime = new Date(originalEndTime + snappedMinutesDelta * 60 * 1000);

              if (newEndTime.getTime() <= event.startTime.getTime() + 15 * 60 * 1000) return;
              setTempEndTime(newEndTime);
            };

            const handleMouseUp = () => {
              window.removeEventListener('mousemove', handleMouseMove);
              window.removeEventListener('mouseup', handleMouseUp);

              setTempEndTime(prev => {
                if (prev) onResize?.(event, { endTime: prev });
                return null;
              });
            };

            window.addEventListener('mousemove', handleMouseMove);
            window.addEventListener('mouseup', handleMouseUp);
          }}
        >
          <div className="mx-auto w-8 h-1 bg-primary/40 rounded-full opacity-0 group-hover/bottom:opacity-100 transition-opacity mt-1" />
        </div>
      )}
    </div>
  )
})

EventCard.displayName = 'EventCard'
