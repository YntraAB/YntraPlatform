import React from 'react'
import { useTranslation } from 'react-i18next'
import { Plus, Calendar as CalendarIcon } from 'lucide-react'
import { useDraggable } from '@dnd-kit/core'
import { cn, getCategoryConfig } from '@/lib/utils'
import { Badge } from '@/components/ui/badge'
import type { CalendarEvent } from '@/types'

interface UnscheduledBucketProps {
  events: CalendarEvent[]
  editMode: boolean
}

export const UnscheduledBucket: React.FC<UnscheduledBucketProps> = ({ events, editMode }) => {
  const { t } = useTranslation()
  return (
    <div className={cn(
      "w-72 border-r border-border bg-sidebar flex flex-col h-full transition-all duration-300",
      !editMode && "w-0 opacity-0 overflow-hidden border-none"
    )}>
      <div className="p-4 border-b border-border flex items-center justify-between bg-background/50">
        <h3 className="font-semibold text-xs uppercase tracking-widest text-muted-foreground flex items-center gap-2">
          <Plus className="h-4 w-4" />
          {t('scheduler.unscheduled')}
        </h3>
        <Badge variant="secondary" className="bg-primary/10 text-primary border-none">
          {events.length}
        </Badge>
      </div>
      <div className="flex-1 overflow-y-auto p-3 space-y-2 scrollbar-dark">
        {events.map(event => (
          <UnscheduledEventCard key={event.id} event={event} editMode={editMode} />
        ))}
        {events.length === 0 && (
          <div className="text-center py-12 px-4">
            <CalendarIcon className="h-8 w-8 mx-auto text-muted-foreground/30 mb-2" />
            <div className="text-muted-foreground text-xs italic">
              {t('scheduler.no_unscheduled_events')}
            </div>
          </div>
        )}
      </div>
    </div>
  )
}

const UnscheduledEventCard: React.FC<{ event: CalendarEvent, editMode: boolean }> = ({ event, editMode }) => {
  const { attributes, listeners, setNodeRef, transform, isDragging } = useDraggable({
    id: `unscheduled-${event.id}`,
    data: { type: 'event', event, isFromBucket: true },
    disabled: !editMode,
  })

  const { t } = useTranslation()
  const categoryConfig = getCategoryConfig(event.category)

  return (
    <div
      ref={setNodeRef}
      {...attributes}
      {...listeners}
      className={cn(
        "p-3 rounded-lg border border-border bg-muted cursor-grab active:cursor-grabbing transition-all duration-200 shadow-sm",
        isDragging && "opacity-50 scale-95",
        "hover:border-primary/50 hover:shadow-sm hover:bg-background"
      )}
      style={{
        transform: transform ? `translate3d(${transform.x}px, ${transform.y}px, 0)` : undefined,
        borderLeft: `4px solid ${categoryConfig.color}`
      }}
    >
      <div className="text-sm font-semibold text-foreground truncate">{event.title}</div>
      <div className="text-[10px] font-bold text-muted-foreground uppercase mt-1 tracking-wider">
        {t(categoryConfig.label)}
      </div>
    </div>
  )
}
