import React, { useMemo } from 'react'
import { useTranslation } from 'react-i18next'
import { Calendar as CalendarIcon, MapPin } from 'lucide-react'
import { useWorkspace } from '@/contexts/WorkspaceContext'
import { cn, getCategoryConfig, isSameDay, isToday } from '@/lib/utils'
import type { CalendarEvent, User } from '@/types'

interface AgendaViewProps {
  events: CalendarEvent[]
  users: User[]
  onEventClick: (event: CalendarEvent) => void
}

export const AgendaView: React.FC<AgendaViewProps> = ({ events, onEventClick }) => {
  const { settings } = useWorkspace()
  const { t } = useTranslation()
  const locale = settings.language === 'sv' ? 'sv-SE' : 'en-US'

  const groupedEvents = useMemo(() => {
    const sortedEvents = [...events].sort((a, b) => a.startTime.getTime() - b.startTime.getTime())

    const groups: Array<{
      date: Date
      dateLabel: string
      isToday: boolean
      events: CalendarEvent[]
    }> = []

    sortedEvents.forEach((event) => {
      const existingGroup = groups.find((g) => isSameDay(g.date, event.startTime))
      if (existingGroup) {
        existingGroup.events.push(event)
      } else {
        const isEventToday = isToday(event.startTime)
        groups.push({
          date: event.startTime,
          dateLabel: isEventToday
            ? t('common.today')
            : event.startTime.toLocaleDateString(locale, {
              weekday: 'long',
              month: 'long',
              day: 'numeric',
            }),
          isToday: isEventToday,
          events: [event],
        })
      }
    })

    return groups
  }, [events, locale, t])

  return (
    <div className="scrollbar-dark flex-1 overflow-y-auto p-4">
      {groupedEvents.length === 0 ? (
        <div className="flex h-full flex-col items-center justify-center text-muted-foreground">
          <CalendarIcon className="mb-4 h-12 w-12 opacity-50" />
          <p className="text-lg font-medium">{t('scheduler.no_upcoming_events', 'Inga kommande händelser')}</p>
          <p className="text-sm">{t('scheduler.no_events_help', 'Dina schemalagda händelser kommer att visas här')}</p>
        </div>
      ) : (
        <div className="mx-auto max-w-3xl space-y-6">
          {groupedEvents.map((group, groupIndex) => (
            <div key={groupIndex}>
              {/* Date header */}
              <div className="mb-3 flex items-center gap-3">
                <div
                  className={cn(
                    'flex h-10 w-10 flex-col items-center justify-center rounded-lg text-xs',
                    group.isToday ? 'bg-primary text-primary-foreground font-medium' : 'bg-muted text-muted-foreground',
                  )}
                >
                  <span className="text-[10px] uppercase">
                    {group.date.toLocaleDateString(locale, { month: 'short' })}
                  </span>
                  <span className="font-medium">{group.date.getDate()}</span>
                </div>
                <h3
                  className={cn(
                    'text-lg font-medium',
                    group.isToday ? 'text-foreground' : 'text-muted-foreground',
                  )}
                >
                  {group.dateLabel}
                </h3>
              </div>

              {/* Events for this date */}
              <div className="ml-12 space-y-2">
                {group.events.map((event) => {
                  const categoryConfig = getCategoryConfig(event.category)
                  return (
                    <div
                      key={event.id}
                      onClick={() => onEventClick(event)}
                      className="flex cursor-pointer items-center gap-4 rounded-lg border border-border bg-muted p-3 transition-all duration-150 hover:border-primary/50"
                    >
                      {/* Time */}
                      <div className="w-20 flex-shrink-0 text-right">
                        <div className="text-sm font-medium text-foreground">
                          {event.startTime.toLocaleTimeString(locale, {
                            hour: '2-digit',
                            minute: '2-digit',
                            hour12: false,
                          })}
                        </div>
                        <div className="text-xs text-muted-foreground">
                          {event.endTime.toLocaleTimeString(locale, {
                            hour: '2-digit',
                            minute: '2-digit',
                            hour12: false,
                          })}
                        </div>
                      </div>

                      {/* Category indicator */}
                      <div
                        className="h-10 w-1 flex-shrink-0 rounded-full"
                        style={{ backgroundColor: categoryConfig.color }}
                      />

                      {/* Event details */}
                      <div className="min-w-0 flex-1">
                        <div className="truncate font-medium text-foreground">{event.title}</div>
                        {event.description && (
                          <div className="truncate text-sm text-muted-foreground">
                            {event.description}
                          </div>
                        )}
                      </div>

                      {/* Location */}
                      {event.location && (
                        <div className="flex flex-shrink-0 items-center gap-1 text-sm text-muted-foreground">
                          <MapPin className="h-4 w-4" />
                          <span>{event.location}</span>
                        </div>
                      )}

                      {/* Category badge */}
                      <span
                        className="flex-shrink-0 rounded px-2 py-1 text-xs"
                        style={{
                          backgroundColor: categoryConfig.bgColor,
                          color: categoryConfig.color,
                        }}
                      >
                        {t(categoryConfig.label)}
                      </span>
                    </div>
                  )
                })}
              </div>
            </div>
          ))}
        </div>
      )}
    </div>
  )
}
