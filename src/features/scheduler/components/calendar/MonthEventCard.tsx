import React from 'react'
import { useTranslation } from 'react-i18next'
import { Calendar as CalendarIcon, MapPin } from 'lucide-react'
import { formatTimeRange, getCategoryConfig } from '@/lib/utils'
import type { CalendarEvent } from '@/types'

interface MonthEventCardProps {
  event: CalendarEvent
  onClick: (e: React.MouseEvent) => void
  locale?: string
}

export const MonthEventCard: React.FC<MonthEventCardProps> = ({ event, onClick, locale = 'en-US' }) => {
  const { t } = useTranslation()
  const categoryConfig = getCategoryConfig(event.category)

  return (
    <div className="group relative">
      <div
        onClick={onClick}
        className="cursor-pointer truncate rounded px-1.5 py-0.5 text-[10px] transition-all duration-150 hover:brightness-110"
        style={{
          backgroundColor: categoryConfig.bgColor,
          color: categoryConfig.color,
        }}
      >
        {event.startTime.toLocaleTimeString(locale, {
          hour: '2-digit',
          minute: '2-digit',
          hour12: false,
        })}{' '}
        {event.title}
      </div>

      {/* Hover Information Popup */}
      <div className="pointer-events-none absolute bottom-full left-0 z-[100] mb-1 hidden w-64 rounded-lg border border-border bg-background/95 p-4 shadow-sm backdrop-blur-md duration-200 animate-in fade-in slide-in-from-bottom-1 group-hover:block">
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
    </div>
  )
}
