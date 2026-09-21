import React from 'react'
import { useTranslation } from 'react-i18next'
import { useWorkspace } from '@/contexts/WorkspaceContext'
import { useWorkspaceTeams, useWorkspaceUsers } from '@/hooks/queries/useWorkspaceData'
import { getCategoryConfig } from '@/lib/utils'
import type { CalendarEvent } from '@/types'
import { Calendar, Clock, Users, User, Trash2, Edit2 } from 'lucide-react'
import {
  Dialog,
  DialogContent,
  DialogHeader,
  DialogTitle,
  DialogFooter,
} from '@/components/ui/dialog'
import { Button } from '@/components/ui/button'

interface EventModalProps {
  event: CalendarEvent | null
  onClose: () => void
  onEdit?: (event: CalendarEvent) => void
  onDelete?: (eventId: string) => void
}

export const EventModal: React.FC<EventModalProps> = ({ event, onClose, onEdit, onDelete }) => {
  const { settings, workspaceId } = useWorkspace()
  const { t } = useTranslation()

  const { data: dbTeams = [] } = useWorkspaceTeams(workspaceId)
  const { data: dbUsers = [] } = useWorkspaceUsers(workspaceId)

  const locale = settings.language === 'sv' ? 'sv-SE' : 'en-US'

  if (!event) return null

  const eventTeam = dbTeams.find((t) => t.id === event.teamId)
  const eventAssignee = dbUsers.find((u) => u.id === event.assigneeId)
  const category = getCategoryConfig(event.category)

  return (
    <Dialog open={!!event} onOpenChange={(open) => !open && onClose()}>
      <DialogContent className="overflow-hidden rounded-lg border border-border bg-card p-0 shadow-2xl sm:max-w-md">
        {/* Header - Delicate micro-icon + title */}
        <DialogHeader className="flex h-12 flex-row items-center justify-between border-b border-border bg-secondary/40 px-5 py-0">
          <DialogTitle className="flex items-center gap-2 truncate text-xs font-medium tracking-tight text-foreground">
            <Calendar className="size-3.5 shrink-0 text-muted-foreground/70" />
            <span className="truncate">{event.title}</span>
          </DialogTitle>
        </DialogHeader>

        {/* Content Body - Clean, light typography with subtle micro-icons */}
        <div className="space-y-4 p-5">
          {/* Time & Date */}
          <div>
            <p className="text-xs font-medium text-foreground capitalize">
              {event.startTime.toLocaleDateString(locale, {
                weekday: 'long',
                month: 'long',
                day: 'numeric',
              })}
            </p>
            <p className="mt-1 flex items-center gap-1.5 text-xs font-normal text-muted-foreground tabular-nums">
              <Clock className="size-3 shrink-0 text-muted-foreground/60" />
              <span>
                {event.startTime.toLocaleTimeString(locale, { hour: '2-digit', minute: '2-digit' })}{' '}
                – {event.endTime.toLocaleTimeString(locale, { hour: '2-digit', minute: '2-digit' })}
              </span>
            </p>
          </div>

          {/* Description */}
          {event.description && (
            <p className="text-xs font-normal leading-relaxed text-foreground/90">
              {event.description}
            </p>
          )}

          {/* Metadata Section - Clean key-value hierarchy */}
          <div className="space-y-2 border-t border-border pt-3.5 text-xs">
            {/* Category */}
            <div className="flex items-center justify-between">
              <span className="text-muted-foreground">{t('common.category', 'Kategori')}</span>
              <span className="font-medium text-foreground">{t(category.label)}</span>
            </div>

            {/* Team */}
            {eventTeam && (
              <div className="flex items-center justify-between">
                <span className="flex items-center gap-1.5 text-muted-foreground">
                  <Users className="size-3 text-muted-foreground/60" />
                  <span>{t('common.team', 'Team')}</span>
                </span>
                <span className="font-medium text-foreground">{eventTeam.name}</span>
              </div>
            )}

            {/* Assistant / Assignee */}
            {eventAssignee && (
              <div className="flex items-center justify-between">
                <span className="flex items-center gap-1.5 text-muted-foreground">
                  <User className="size-3 text-muted-foreground/60" />
                  <span>{t('scheduler.assigned_to', 'Tilldelad')}</span>
                </span>
                <span className="font-medium text-foreground">
                  {eventAssignee.full_name || eventAssignee.name}
                </span>
              </div>
            )}
          </div>
        </div>

        {/* Footer - Calm, restrained text buttons with micro-icons */}
        <DialogFooter className="flex flex-row items-center justify-between border-t border-border bg-secondary/40 px-5 py-2.5">
          <Button
            variant="ghost"
            size="sm"
            onClick={() => onDelete?.(event.id)}
            className="h-8 rounded-md px-2.5 text-xs font-medium text-muted-foreground hover:bg-destructive/15 hover:text-destructive"
          >
            <Trash2 className="mr-1.5 size-3" />
            <span>{t('common.delete')}</span>
          </Button>

          <div className="flex items-center gap-2">
            <Button
              variant="outline"
              size="sm"
              onClick={onClose}
              className="h-8 rounded-md border border-border bg-background px-3 text-xs font-medium text-foreground hover:bg-secondary"
            >
              {t('common.close', 'Stäng')}
            </Button>
            <Button
              size="sm"
              onClick={() => onEdit?.(event)}
              className="h-8 rounded-md bg-primary px-3 text-xs font-medium text-primary-foreground shadow-xs hover:bg-primary/90"
            >
              <Edit2 className="mr-1.5 size-3" />
              <span>{t('common.edit')}</span>
            </Button>
          </div>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  )
}
