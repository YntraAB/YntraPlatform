import React from 'react'
import { getCategoryConfig } from '@/lib/utils'
import type { CalendarEvent } from '@/types'

interface MonthEventCardProps {
  event: CalendarEvent
  onClick: (e: React.MouseEvent) => void
  locale?: string
}

export const MonthEventCard: React.FC<MonthEventCardProps> = ({ event, onClick, locale = 'en-US' }) => {
  const categoryConfig = getCategoryConfig(event.category)
  const timeStr = event.startTime.toLocaleTimeString(locale, {
    hour: '2-digit',
    minute: '2-digit',
    hour12: false,
  })

  return (
    <div
      onClick={onClick}
      title={`${timeStr} - ${event.title}${event.location ? ` (${event.location})` : ''}`}
      className="cursor-pointer truncate rounded px-1.5 py-0.5 text-[10px] font-medium transition-all duration-150 hover:brightness-110 shadow-2xs"
      style={{
        backgroundColor: categoryConfig.bgColor,
        color: categoryConfig.color,
      }}
    >
      <span className="font-mono opacity-80 mr-1">{timeStr}</span>
      <span>{event.title}</span>
    </div>
  )
}
