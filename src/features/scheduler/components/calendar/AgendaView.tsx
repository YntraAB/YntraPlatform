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
      totalHours: number
      events: CalendarEvent[]
    }> = []

    sortedEvents.forEach((event) => {
      const existingGroup = groups.find((g) => isSameDay(g.date, event.startTime))
      const diffMs = event.endTime.getTime() - event.startTime.getTime()
      const hours = Math.round((Math.max(0, diffMs) / (1000 * 60 * 60)) * 10) / 10

      if (existingGroup) {
        existingGroup.events.push(event)
        existingGroup.totalHours = Math.round((existingGroup.totalHours + hours) * 10) / 10
      } else {
        const isEventToday = isToday(event.startTime)
        const rawDate = isEventToday
          ? t('common.today', 'Idag')
          : event.startTime.toLocaleDateString(locale, {
              weekday: 'long',
              day: 'numeric',
              month: 'long',
            })
        const capitalized = rawDate.charAt(0).toUpperCase() + rawDate.slice(1)

        groups.push({
          date: event.startTime,
          dateLabel: capitalized,
          isToday: isEventToday,
          totalHours: hours,
          events: [event],
        })
      }
    })

    return groups
  }, [events, locale, t])

  return (
    <div className="scrollbar-dark flex-1 overflow-y-auto px-6 py-4 min-h-0 bg-background">
      {groupedEvents.length === 0 ? (
        <div className="flex h-72 flex-col items-center justify-center text-muted-foreground duration-500 animate-in fade-in zoom-in-95">
          <CalendarIcon className="mb-3 size-8 opacity-40" />
          <p className="text-xs font-medium text-foreground">
            {t('scheduler.no_upcoming_events', 'Inga kommande händelser')}
          </p>
          <p className="mt-1 text-[11px] text-muted-foreground/70">
            {t('scheduler.no_events_help', 'Dina schemalagda händelser kommer att visas här.')}
          </p>
        </div>
      ) : (
        <div className="mx-auto max-w-3xl space-y-4">
          {groupedEvents.map((group, groupIndex) => (
            <div key={groupIndex}>
              {/* Date Header: Compact size-8 date badge and text-xs title */}
              <div className="mb-2 flex items-center gap-2.5">
                <div
                  className={cn(
                    'flex h-8 w-8 shrink-0 flex-col items-center justify-center rounded-md border text-xs',
                    group.isToday
                      ? 'border-primary/40 bg-primary/10 text-primary'
                      : 'border-border/50 bg-secondary/50 text-muted-foreground',
                  )}
                >
                  <span className="text-[8.5px] uppercase font-mono leading-none tracking-wider opacity-80">
                    {group.date.toLocaleDateString(locale, { month: 'short' }).replace('.', '')}
                  </span>
                  <span className="font-mono text-xs font-semibold leading-none mt-0.5">
                    {group.date.getDate()}
                  </span>
                </div>

                <div className="flex items-center gap-2">
                  <h3
                    className={cn(
                      'text-xs font-medium tracking-tight',
                      group.isToday ? 'text-foreground' : 'text-muted-foreground',
                    )}
                  >
                    {group.dateLabel}
                  </h3>
                  <span className="text-[11px] font-mono text-muted-foreground/60 tabular-nums">
                    ({group.events.length} {group.events.length === 1 ? 'pass' : 'pass'} · {group.totalHours}h)
                  </span>
                </div>
              </div>

              {/* Event cards: exact original structure, compact refined dimensions */}
              <div className="ml-[42px] space-y-1.5">
                {group.events.map((event) => {
                  const categoryConfig = getCategoryConfig(event.category)
                  const startStr = event.startTime.toLocaleTimeString(locale, {
                    hour: '2-digit',
                    minute: '2-digit',
                    hour12: false,
                  })
                  const endStr = event.endTime.toLocaleTimeString(locale, {
                    hour: '2-digit',
                    minute: '2-digit',
                    hour12: false,
                  })

                  return (
                    <div
                      key={event.id}
                      onClick={() => onEventClick(event)}
                      className="group flex cursor-pointer items-center justify-between gap-3.5 rounded-md border border-border/50 bg-card/60 px-3.5 py-2 transition-all duration-150 hover:border-border hover:bg-secondary/40 shadow-2xs"
                    >
                      {/* Left: Time block + vertical colored accent bar */}
                      <div className="flex shrink-0 items-center gap-2.5">
                        <div className="w-12 text-right">
                          <div className="font-mono text-xs font-medium text-foreground leading-tight">
                            {startStr}
                          </div>
                          <div className="font-mono text-[10.5px] text-muted-foreground/60 leading-tight">
                            {endStr}
                          </div>
                        </div>
                        <div
                          className="h-6 w-0.5 rounded-full shrink-0"
                          style={{ backgroundColor: categoryConfig.color || 'hsl(var(--border))' }}
                        />
                      </div>

                      {/* Middle: Title & Subtitle */}
                      <div className="min-w-0 flex-1 truncate pr-2">
                        <div className="truncate text-xs font-medium text-foreground transition-colors group-hover:text-primary leading-snug">
                          {event.title}
                        </div>
                        {event.description && (
                          <div className="truncate text-[11px] font-light text-muted-foreground/70 mt-0.5 leading-snug">
                            {event.description}
                          </div>
                        )}
                      </div>

                      {/* Right: Location & Category Pill */}
                      <div className="flex shrink-0 items-center gap-2.5 text-xs">
                        {event.location && (
                          <div className="hidden sm:flex items-center gap-1 text-[11px] text-muted-foreground/70">
                            <MapPin className="size-3 shrink-0 opacity-60" />
                            <span className="truncate max-w-[140px]">{event.location}</span>
                          </div>
                        )}

                        <span
                          className="inline-flex shrink-0 items-center rounded px-2 py-0.5 text-[10.5px] font-medium"
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
                })}
              </div>
            </div>
          ))}
        </div>
      )}
    </div>
  )
}

export default AgendaView
